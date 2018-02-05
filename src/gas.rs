use parity_wasm::{elements, builder};
use rules;

pub fn update_call_index(opcodes: &mut elements::Opcodes, inserted_index: u32) {
	use parity_wasm::elements::Opcode::*;
	for opcode in opcodes.elements_mut().iter_mut() {
		match opcode {
			&mut Call(ref mut call_index) => {
				if *call_index >= inserted_index { *call_index += 1}
			},
			_ => { },
		}
	}
}

enum InjectAction {
	Spawn(u32),
	Continue(u32),
	Increment,
	IncrementSpawn,
}

fn inject_grow_counter(opcodes: &mut elements::Opcodes, grow_counter_func: u32) -> usize {
	use parity_wasm::elements::Opcode::*;
	let mut counter = 0;
	for opcode in opcodes.elements_mut() {
		match *opcode {
			GrowMemory(_) => {
				*opcode = Call(grow_counter_func);
				counter += 1;
			},
			_ => {}
		}
	}
	counter
}

fn add_grow_counter(module: elements::Module, rules: &rules::Set, gas_func: u32) -> elements::Module {
	use parity_wasm::elements::Opcode::*;

	let mut b = builder::from_module(module);
	b.push_function(
		builder::function()
			.signature().params().i32().build().with_return_type(Some(elements::ValueType::I32)).build()
			.body()
				.with_opcodes(elements::Opcodes::new(vec![
					GetLocal(0),
					GetLocal(0),
					I32Const(rules.grow_cost() as i32),
					I32Mul,
					// todo: there should be strong guarantee that it does not return anything on stack?
					Call(gas_func),
					GrowMemory(0),
					GetLocal(0),
					End,
				]))
				.build()
			.build()
	);

	b.build()
}

pub fn inject_counter(
	opcodes: &mut elements::Opcodes,
	rules: &rules::Set,
	gas_func: u32,
) {
	use parity_wasm::elements::Opcode::*;

	let mut stack: Vec<(usize, usize)> = Vec::new();
	let mut cursor = 0;
	stack.push((0, 1));

	loop {
		if cursor >= opcodes.elements().len() {
			break;
		}

		let last_entry = stack.pop().expect("There should be at least one entry on stack");

		let action = {
			let opcode = &opcodes.elements()[cursor];
			match *opcode {
				Block(_) | If(_) | Loop(_) => {
					InjectAction::Spawn(rules.process(opcode))
				},
				Else => {
					InjectAction::IncrementSpawn
				},
				End => {
					InjectAction::Increment
				},
				_ => {
					InjectAction::Continue(rules.process(opcode))
				}
			}
		};

		match action {
			InjectAction::Increment => {
				let (pos, ops) = last_entry;
				opcodes.elements_mut().insert(pos, I32Const(ops as i32));
				opcodes.elements_mut().insert(pos+1, Call(gas_func));
				cursor += 3;
			},
			InjectAction::IncrementSpawn => {
				let (pos, ops) = last_entry;
				opcodes.elements_mut().insert(pos, I32Const(ops as i32));
				opcodes.elements_mut().insert(pos+1, Call(gas_func));
				cursor += 3;
				stack.push((cursor, 1));
			},
			InjectAction::Continue(val) => {
				cursor += 1;
				let (pos, ops) = last_entry;
				stack.push((pos, ops + val as usize));
			},
			InjectAction::Spawn(val) => {
				let (pos, ops) = last_entry;
				stack.push((pos, ops + val as usize));

				cursor += 1;
				stack.push((cursor, 1));
			},
		}
	}
}

pub fn inject_gas_counter(module: elements::Module, rules: &rules::Set) -> elements::Module {
	// Injecting gas counting external
	let mut mbuilder = builder::from_module(module);
	let import_sig = mbuilder.push_signature(
		builder::signature()
			.param().i32()
			.build_sig()
		);

	mbuilder.push_import(
		builder::import()
			.module("env")
			.field("gas")
			.external().func(import_sig)
			.build()
		);

	// back to plain module
	let mut module = mbuilder.build();

	// calculate actual function index of the imported definition
	//    (substract all imports that are NOT functions)

	let gas_func = module.import_count(elements::ImportCountType::Function) as u32 - 1;
	let total_func = module.functions_space() as u32;
	let mut need_grow_counter = false;

	// Updating calling addresses (all calls to function index >= `gas_func` should be incremented)
	for section in module.sections_mut() {
		match section {
			&mut elements::Section::Code(ref mut code_section) => {
				for ref mut func_body in code_section.bodies_mut() {
					update_call_index(func_body.code_mut(), gas_func);
					inject_counter(func_body.code_mut(), rules, gas_func);
					if rules.grow_cost() > 0 {
						if inject_grow_counter(func_body.code_mut(), total_func) > 0 {
							need_grow_counter = true;
						}
					}
				}
			},
			&mut elements::Section::Export(ref mut export_section) => {
				for ref mut export in export_section.entries_mut() {
					match export.internal_mut() {
						&mut elements::Internal::Function(ref mut func_index) => {
							if *func_index >= gas_func { *func_index += 1}
						},
						_ => {}
					}
				}
			},
			&mut elements::Section::Element(ref mut elements_section) => {
				for ref mut segment in elements_section.entries_mut() {
					// update all indirect call addresses initial values
					for func_index in segment.members_mut() {
						if *func_index >= gas_func { *func_index += 1}
					}
				}
			},
			_ => { }
		}
	}

	if need_grow_counter { add_grow_counter(module, rules, gas_func) } else { module }
}

#[cfg(test)]
mod tests {

	extern crate wabt;

	use parity_wasm::{serialize, builder, elements};
	use super::*;
	use rules;

	#[test]
	fn simple_grow() {
		use parity_wasm::elements::Opcode::*;

		let module = builder::module()
			.global()
				.value_type().i32()
				.build()
			.function()
				.signature().param().i32().build()
				.body()
					.with_opcodes(elements::Opcodes::new(
						vec![
							GetGlobal(0),
							GrowMemory(0),
							End
						]
					))
					.build()
				.build()
			.build();

		let injected_module = inject_gas_counter(module, &rules::Set::default().with_grow_cost(10000));

		assert_eq!(
			&vec![
				I32Const(3),
				Call(0),
				GetGlobal(0),
				Call(2),
				End
			][..],
			injected_module
				.code_section().expect("function section should exist").bodies()[0]
				.code().elements()
		);
		assert_eq!(
			&vec![
				GetLocal(0),
				GetLocal(0),
				I32Const(10000),
				I32Mul,
				Call(0),
				GrowMemory(0),
				GetLocal(0),
				End,
			][..],
			injected_module
				.code_section().expect("function section should exist").bodies()[1]
				.code().elements()
		);

		let binary = serialize(injected_module).expect("serialization failed");
		self::wabt::wasm2wat(&binary).unwrap();
	}

	#[test]
	fn grow_no_gas_no_track() {
		use parity_wasm::elements::Opcode::*;

		let module = builder::module()
			.global()
				.value_type().i32()
				.build()
			.function()
				.signature().param().i32().build()
				.body()
					.with_opcodes(elements::Opcodes::new(
						vec![
							GetGlobal(0),
							GrowMemory(0),
							End
						]
					))
					.build()
				.build()
			.build();

		let injected_module = inject_gas_counter(module, &rules::Set::default());

		assert_eq!(
			&vec![
				I32Const(3),
				Call(0),
				GetGlobal(0),
				GrowMemory(0),
				End
			][..],
			injected_module
				.code_section().expect("function section should exist").bodies()[0]
				.code().elements()
		);

		assert_eq!(injected_module.functions_space(), 2);

		let binary = serialize(injected_module).expect("serialization failed");
		self::wabt::wasm2wat(&binary).unwrap();
	}

	#[test]
	fn simple() {
		use parity_wasm::elements::Opcode::*;

		let module = builder::module()
			.global()
				.value_type().i32()
				.build()
			.function()
				.signature().param().i32().build()
				.body()
					.with_opcodes(elements::Opcodes::new(
						vec![
							GetGlobal(0),
							End
						]
					))
					.build()
				.build()
			.build();

		let injected_module = inject_gas_counter(module, &Default::default());

		assert_eq!(
			&vec![
				I32Const(2),
				Call(0),
				GetGlobal(0),
				End
			][..],
			injected_module
				.code_section().expect("function section should exist").bodies()[0]
				.code().elements()
		);
	}

	#[test]
	fn nested() {
		use parity_wasm::elements::Opcode::*;

		let module = builder::module()
			.global()
				.value_type().i32()
				.build()
			.function()
				.signature().param().i32().build()
				.body()
					.with_opcodes(elements::Opcodes::new(
						vec![
							GetGlobal(0),
							Block(elements::BlockType::NoResult),
								GetGlobal(0),
								GetGlobal(0),
								GetGlobal(0),
							End,
							GetGlobal(0),
							End
						]
					))
					.build()
				.build()
			.build();

		let injected_module = inject_gas_counter(module, &Default::default());

		assert_eq!(
			&vec![
				I32Const(4),
				Call(0),
				GetGlobal(0),
				Block(elements::BlockType::NoResult),
					I32Const(4),
					Call(0),
					GetGlobal(0),
					GetGlobal(0),
					GetGlobal(0),
				End,
				GetGlobal(0),
				End
			][..],
			injected_module
				.code_section().expect("function section should exist").bodies()[0]
				.code().elements()
		);
	}

	#[test]
	fn ifelse() {
		use parity_wasm::elements::Opcode::*;

		let module = builder::module()
			.global()
				.value_type().i32()
				.build()
			.function()
				.signature().param().i32().build()
				.body()
					.with_opcodes(elements::Opcodes::new(
						vec![
							GetGlobal(0),
							If(elements::BlockType::NoResult),
								GetGlobal(0),
								GetGlobal(0),
								GetGlobal(0),
							Else,
								GetGlobal(0),
								GetGlobal(0),
							End,
							GetGlobal(0),
							End
						]
					))
					.build()
				.build()
			.build();

		let injected_module = inject_gas_counter(module, &Default::default());

		assert_eq!(
			&vec![
				I32Const(4),
				Call(0),
				GetGlobal(0),
				If(elements::BlockType::NoResult),
					I32Const(4),
					Call(0),
					GetGlobal(0),
					GetGlobal(0),
					GetGlobal(0),
				Else,
					I32Const(3),
					Call(0),
					GetGlobal(0),
					GetGlobal(0),
				End,
				GetGlobal(0),
				End
			][..],
			injected_module
				.code_section().expect("function section should exist").bodies()[0]
				.code().elements()
		);
	}

	#[test]
	fn call_index() {
		use parity_wasm::elements::Opcode::*;

		let module = builder::module()
			.global()
				.value_type().i32()
				.build()
			.function()
				.signature().param().i32().build()
				.body().build()
				.build()
			.function()
				.signature().param().i32().build()
				.body()
					.with_opcodes(elements::Opcodes::new(
						vec![
							Call(0),
							If(elements::BlockType::NoResult),
								Call(0),
								Call(0),
								Call(0),
							Else,
								Call(0),
								Call(0),
							End,
							Call(0),
							End
						]
					))
					.build()
				.build()
			.build();

		let injected_module = inject_gas_counter(module, &Default::default());

		assert_eq!(
			&vec![
				I32Const(4),
				Call(0),
				Call(1),
				If(elements::BlockType::NoResult),
					I32Const(4),
					Call(0),
					Call(1),
					Call(1),
					Call(1),
				Else,
					I32Const(3),
					Call(0),
					Call(1),
					Call(1),
				End,
				Call(1),
				End
			][..],
			injected_module
				.code_section().expect("function section should exist").bodies()[1]
				.code().elements()
		);
	}

}