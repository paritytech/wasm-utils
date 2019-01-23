use std::vec::Vec;

use parity_wasm::{elements, builder};
use rules;

pub fn update_call_index(instructions: &mut elements::Instructions, inserted_index: u32) {
	use parity_wasm::elements::Instruction::*;
	for instruction in instructions.elements_mut().iter_mut() {
		if let &mut Call(ref mut call_index) = instruction {
			if *call_index >= inserted_index { *call_index += 1}
		}
	}
}

/// A block of code represented by it's start position and cost.
///
/// The block typically starts with instructions such as `loop`, `block`, `if`, etc.
///
/// An example of block:
///
/// ```ignore
/// loop
///   i32.const 1
///   get_local 0
///   i32.sub
///   tee_local 0
///   br_if 0
/// end
/// ```
///
/// The start of the block is `i32.const 1`.
///
#[derive(Debug)]
struct BlockEntry {
	/// Index of the first instruction (aka `Opcode`) in the block.
	start_pos: usize,
	/// Sum of costs of all instructions until end of the block.
	cost: u32,
}

struct Counter {
	/// All blocks in the order of theirs start position.
	blocks: Vec<BlockEntry>,

	// Stack of blocks. Each element is an index to a `self.blocks` vector.
	stack: Vec<usize>,
}

impl Counter {
	fn new() -> Counter {
		Counter {
			stack: Vec::new(),
			blocks: Vec::new(),
		}
	}

	/// Begin a new block.
	fn begin(&mut self, cursor: usize) {
		let block_idx = self.blocks.len();
		self.blocks.push(BlockEntry {
			start_pos: cursor,
			cost: 1,
		});
		self.stack.push(block_idx);
	}

	/// Finalize the current block.
	///
	/// Finalized blocks have final cost which will not change later.
	fn finalize(&mut self) -> Result<(), ()> {
		self.stack.pop().ok_or_else(|| ())?;
		Ok(())
	}

	/// Increment the cost of the current block by the specified value.
	fn increment(&mut self, val: u32) -> Result<(), ()> {
		let stack_top = self.stack.last_mut().ok_or_else(|| ())?;
		let top_block = self.blocks.get_mut(*stack_top).ok_or_else(|| ())?;

		top_block.cost = top_block.cost.checked_add(val).ok_or_else(|| ())?;

		Ok(())
	}
}

fn inject_grow_counter(instructions: &mut elements::Instructions, grow_counter_func: u32) -> usize {
	use parity_wasm::elements::Instruction::*;
	let mut counter = 0;
	for instruction in instructions.elements_mut() {
		if let GrowMemory(_) = *instruction {
			*instruction = Call(grow_counter_func);
			counter += 1;
		}
	}
	counter
}

fn add_grow_counter(module: elements::Module, rules: &rules::Set, gas_func: u32) -> elements::Module {
	use parity_wasm::elements::Instruction::*;

	let mut b = builder::from_module(module);
	b.push_function(
		builder::function()
			.signature().params().i32().build().with_return_type(Some(elements::ValueType::I32)).build()
			.body()
				.with_instructions(elements::Instructions::new(vec![
					GetLocal(0),
					GetLocal(0),
					I32Const(rules.grow_cost() as i32),
					I32Mul,
					// todo: there should be strong guarantee that it does not return anything on stack?
					Call(gas_func),
					GrowMemory(0),
					End,
				]))
				.build()
			.build()
	);

	b.build()
}

pub fn inject_counter(
	instructions: &mut elements::Instructions,
	rules: &rules::Set,
	gas_func: u32,
) -> Result<(), ()> {
	use parity_wasm::elements::Instruction::*;

	let mut counter = Counter::new();

	// Begin an implicit function (i.e. `func...end`) block.
	counter.begin(0);

	for cursor in 0..instructions.elements().len() {
		let instruction = &instructions.elements()[cursor];
		match *instruction {
			Block(_) | If(_) | Loop(_) => {
				// Increment previous block with the cost of the current opcode.
				let instruction_cost = rules.process(instruction)?;
				counter.increment(instruction_cost)?;

				// Begin new block. The cost of the following opcodes until `End` or `Else` will
				// be included into this block.
				counter.begin(cursor + 1);
			}
			End => {
				// Just finalize current block.
				counter.finalize()?;
			},
			Else => {
				// `Else` opcode is being encountered. So the case we are looking at:
				//
				// if
				//   ...
				// else <-- cursor
				//   ...
				// end
				//
				// Finalize the current block ('then' part of the if statement),
				// and begin another one for the 'else' part.
				counter.finalize()?;
				counter.begin(cursor + 1);
			}
			_ => {
				// An ordinal non control flow instruction. Just increment the cost of the current block.
				let instruction_cost = rules.process(instruction)?;
				counter.increment(instruction_cost)?;
			}
		}
	}

	// Then insert metering calls.
	let mut cumulative_offset = 0;
	for block in counter.blocks {
		let effective_pos = block.start_pos + cumulative_offset;

		instructions.elements_mut().insert(effective_pos, I32Const(block.cost as i32));
		instructions.elements_mut().insert(effective_pos+1, Call(gas_func));

		// Take into account these two inserted instructions.
		cumulative_offset += 2;
	}

	Ok(())
}

/// Injects gas counter.
///
/// Can only fail if encounters operation forbidden by gas rules,
/// in this case it returns error with the original module.
pub fn inject_gas_counter(module: elements::Module, rules: &rules::Set)
	-> Result<elements::Module, elements::Module>
{
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
	let module = mbuilder.build();

	// calculate actual function index of the imported definition
	//    (substract all imports that are NOT functions)

	let gas_func = module.import_count(elements::ImportCountType::Function) as u32 - 1;
	inject_gas_counter_func(module, rules, gas_func)
}

/// Injects calls to counter function identified by index.
/// Assumes gas counter function is already present in import section.
///
/// Can only fail if encounters operation forbidden by gas rules,
/// in this case it returns error with the original module.
pub fn inject_gas_counter_func(mut module: elements::Module, rules: &rules::Set, gas_counter_func: u32)
	-> Result<elements::Module, elements::Module>
{
	let total_func = module.functions_space() as u32;
	let mut need_grow_counter = false;
	let mut error = false;

	// Updating calling addresses (all calls to function index >= `gas_counter_func` should be incremented)
	for section in module.sections_mut() {
		match section {
			&mut elements::Section::Code(ref mut code_section) => {
				for ref mut func_body in code_section.bodies_mut() {
					update_call_index(func_body.code_mut(), gas_counter_func);
					if let Err(_) = inject_counter(func_body.code_mut(), rules, gas_counter_func) {
						error = true;
						break;
					}
					if rules.grow_cost() > 0 {
						if inject_grow_counter(func_body.code_mut(), total_func) > 0 {
							need_grow_counter = true;
						}
					}
				}
			},
			&mut elements::Section::Export(ref mut export_section) => {
				for ref mut export in export_section.entries_mut() {
					if let &mut elements::Internal::Function(ref mut func_index) = export.internal_mut() {
						if *func_index >= gas_counter_func { *func_index += 1}
					}
				}
			},
			&mut elements::Section::Element(ref mut elements_section) => {
				for ref mut segment in elements_section.entries_mut() {
					// update all indirect call addresses initial values
					for func_index in segment.members_mut() {
						if *func_index >= gas_counter_func { *func_index += 1}
					}
				}
			},
			&mut elements::Section::Start(ref mut start_idx) => {
				if *start_idx >= gas_counter_func { *start_idx += 1}
			},
			_ => { }
		}
	}

	if error { return Err(module); }

	if need_grow_counter {
		Ok(add_grow_counter(module, rules, gas_counter_func))
	} else {
		Ok(module)
	}
}

#[cfg(test)]
mod tests {

	extern crate wabt;

	use parity_wasm::{serialize, builder, elements};
	use super::*;
	use rules;

	#[test]
	fn simple_grow() {
		use parity_wasm::elements::Instruction::*;

		let module = builder::module()
			.global()
				.value_type().i32()
				.build()
			.function()
				.signature().param().i32().build()
				.body()
					.with_instructions(elements::Instructions::new(
						vec![
							GetGlobal(0),
							GrowMemory(0),
							End
						]
					))
					.build()
				.build()
			.build();

		let injected_module = inject_gas_counter(module, &rules::Set::default().with_grow_cost(10000)).unwrap();

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
		use parity_wasm::elements::Instruction::*;

		let module = builder::module()
			.global()
				.value_type().i32()
				.build()
			.function()
				.signature().param().i32().build()
				.body()
					.with_instructions(elements::Instructions::new(
						vec![
							GetGlobal(0),
							GrowMemory(0),
							End
						]
					))
					.build()
				.build()
			.build();

		let injected_module = inject_gas_counter(module, &rules::Set::default()).unwrap();

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
		use parity_wasm::elements::Instruction::*;

		let module = builder::module()
			.global()
				.value_type().i32()
				.build()
			.function()
				.signature().param().i32().build()
				.body()
					.with_instructions(elements::Instructions::new(
						vec![
							GetGlobal(0),
							End
						]
					))
					.build()
				.build()
			.build();

		let injected_module = inject_gas_counter(module, &Default::default()).unwrap();

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
		use parity_wasm::elements::Instruction::*;

		let module = builder::module()
			.global()
				.value_type().i32()
				.build()
			.function()
				.signature().param().i32().build()
				.body()
					.with_instructions(elements::Instructions::new(
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

		let injected_module = inject_gas_counter(module, &Default::default()).unwrap();

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
		use parity_wasm::elements::Instruction::*;

		let module = builder::module()
			.global()
				.value_type().i32()
				.build()
			.function()
				.signature().param().i32().build()
				.body()
					.with_instructions(elements::Instructions::new(
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

		let injected_module = inject_gas_counter(module, &Default::default()).unwrap();

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
		use parity_wasm::elements::Instruction::*;

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
					.with_instructions(elements::Instructions::new(
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

		let injected_module = inject_gas_counter(module, &Default::default()).unwrap();

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


	#[test]
	fn forbidden() {
		use parity_wasm::elements::Instruction::*;

		let module = builder::module()
			.global()
				.value_type().i32()
				.build()
			.function()
				.signature().param().i32().build()
				.body()
					.with_instructions(elements::Instructions::new(
						vec![
							F32Const(555555),
							End
						]
					))
					.build()
				.build()
			.build();

		let rules = rules::Set::default().with_forbidden_floats();

		if let Err(_) = inject_gas_counter(module, &rules) { }
		else { panic!("Should be error because of the forbidden operation")}

	}

}
