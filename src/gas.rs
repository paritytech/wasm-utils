//! This module is used to instrument a Wasm module with gas metering code.
//!
//! The primary public interface is the `inject_gas_counter` function which transforms a given
//! module into one that charges gas for code to be executed. See function documentation for usage
//! and details.

use std::mem;
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

	insert_metering_calls(instructions, counter.blocks, gas_func)
}

// Then insert metering calls into a sequence of instructions given the block locations and costs.
fn insert_metering_calls(
	instructions: &mut elements::Instructions,
	blocks: Vec<BlockEntry>,
	gas_func: u32,
)
	-> Result<(), ()>
{
	use parity_wasm::elements::Instruction::*;

	// To do this in linear time, construct a new vector of instructions, copying over old
	// instructions one by one and injecting new ones as required.
	let new_instrs_len = instructions.elements().len() + 2 * blocks.len();
	let original_instrs = mem::replace(
		instructions.elements_mut(), Vec::with_capacity(new_instrs_len)
	);
	let new_instrs = instructions.elements_mut();

	let mut original_pos = 0;
	let mut block_iter = blocks.into_iter().peekable();
	for instr in original_instrs.into_iter() {
		// If there the next block starts at this position, inject metering instructions.
		let used_block = if let Some(ref block) = block_iter.peek() {
			if block.start_pos == original_pos {
				new_instrs.push(I32Const(block.cost as i32));
				new_instrs.push(Call(gas_func));
				true
			} else { false }
		} else { false };

		if used_block {
			block_iter.next();
		}

		// Copy over the original instruction.
		new_instrs.push(instr);
		original_pos += 1;
	}

	if block_iter.next().is_some() {
		return Err(());
	}

	Ok(())
}

/// Transforms a given module into one that charges gas for code to be executed by proxy of an
/// imported gas metering function.
///
/// The output module imports a function "gas" from the module "env" with type signature
/// [i32] -> []. The argument is the amount of gas required to continue execution. The external
/// function is meant to keep track of the total amount of gas used and trap or otherwise halt
/// execution of the runtime if the gas usage exceeds some allowed limit.
///
/// The calls to charge gas are inserted at the beginning of every block of code. A block is
/// defined by `block`, `if`, `else`, `loop`, and `end` boundaries. Blocks form a nested hierarchy
/// where `block`, `if`, `else`, and `loop` begin a new nested block, and `end` and `else` mark the
/// end of a block. The gas cost of a block is determined statically as 1 plus the gas cost of all
/// instructions directly in that block. Each instruction is only counted in the most deeply
/// nested block containing it (ie. a block's cost does not include the cost of instructions in any
/// blocks nested within it). The cost of the `begin`, `if`, and `loop` instructions is counted
/// towards the block containing them, not the nested block that they open. There is no gas cost
/// added for `end`/`else`, as they are pseudo-instructions. The gas cost of each instruction is
/// determined by a `rules::Set` parameter. At the beginning of each block, this procedure injects
/// new instructions to call the "gas" function with the gas cost of the block as an argument.
///
/// Additionally, each `memory.grow` instruction found in the module is instrumented to first make
/// a call to charge gas for the additional pages requested. This cannot be done as part of the
/// block level gas charges as the gas cost is not static and depends on the stack argument to
/// `memory.grow`.
///
/// The above transformations are performed for every function body defined in the module. This
/// function also rewrites all function indices references by code, table elements, etc., since
/// the addition of an imported functions changes the indices of module-defined functions.
///
/// The function fails if the module contains any operation forbidden by gas rule set, returning
/// the original module as an Err.
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
	let mut module = mbuilder.build();

	// calculate actual function index of the imported definition
	//    (subtract all imports that are NOT functions)

	let gas_func = module.import_count(elements::ImportCountType::Function) as u32 - 1;
	let total_func = module.functions_space() as u32;
	let mut need_grow_counter = false;
	let mut error = false;

	// Updating calling addresses (all calls to function index >= `gas_func` should be incremented)
	for section in module.sections_mut() {
		match section {
			&mut elements::Section::Code(ref mut code_section) => {
				for ref mut func_body in code_section.bodies_mut() {
					update_call_index(func_body.code_mut(), gas_func);
					if let Err(_) = inject_counter(func_body.code_mut(), rules, gas_func) {
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
						if *func_index >= gas_func { *func_index += 1}
					}
				}
			},
			&mut elements::Section::Element(ref mut elements_section) => {
				// Note that we do not need to check the element type referenced because in the
				// WebAssembly 1.0 spec, the only allowed element type is funcref.
				for ref mut segment in elements_section.entries_mut() {
					// update all indirect call addresses initial values
					for func_index in segment.members_mut() {
						if *func_index >= gas_func { *func_index += 1}
					}
				}
			},
			&mut elements::Section::Start(ref mut start_idx) => {
				if *start_idx >= gas_func { *start_idx += 1}
			},
			_ => { }
		}
	}

	if error { return Err(module); }

	if need_grow_counter { Ok(add_grow_counter(module, rules, gas_func)) } else { Ok(module) }
}

#[cfg(test)]
mod tests {

	extern crate wabt;

	use parity_wasm::{serialize, builder, elements};
	use super::*;
	use rules;

	fn get_function_body(module: &elements::Module, index: usize)
		-> Option<&[elements::Instruction]>
	{
		module.code_section()
			.and_then(|code_section| code_section.bodies().get(index))
			.map(|func_body| func_body.code().elements())
	}

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
			get_function_body(&injected_module, 0).unwrap(),
			&vec![
				I32Const(3),
				Call(0),
				GetGlobal(0),
				Call(2),
				End
			][..]
		);
		assert_eq!(
			get_function_body(&injected_module, 1).unwrap(),
			&vec![
				GetLocal(0),
				GetLocal(0),
				I32Const(10000),
				I32Mul,
				Call(0),
				GrowMemory(0),
				End,
			][..]
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
			get_function_body(&injected_module, 0).unwrap(),
			&vec![
				I32Const(3),
				Call(0),
				GetGlobal(0),
				GrowMemory(0),
				End
			][..]
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
			get_function_body(&injected_module, 0).unwrap(),
			&vec![
				I32Const(2),
				Call(0),
				GetGlobal(0),
				End
			][..]
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
			get_function_body(&injected_module, 0).unwrap(),
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
			][..]
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
			get_function_body(&injected_module, 0).unwrap(),
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
			][..]
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
			get_function_body(&injected_module, 1).unwrap(),
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
			][..]
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
