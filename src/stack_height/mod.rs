//! The pass that tries to make stack overflows deterministic, by introducing
//! an upper bound of the stack size.
//!
//! This pass process introduces a global mutable variable to track stack height,
//! and wraps each function with prolog and epilog.
//!
//! The prolog is inserted before the original function code. It increments
//! the global stack height variable with statically determined "stack cost"
//! of the current function. If after increment the stack height exceeds
//! the limit (specified by the `rules`) execution traps.
//! Otherwise, control flow proceeds to the original function body.
//!
//! The epilog is inserted at the each return point of the function, namely:
//!
//! - explicit `return` instruction,
//! - implicit return execution function scope `end` instruction.
//!
//! The purpose of the epilog is to decrease the stack height by the "stack cost"
//! of the current function.
//!
//! As an optimization, we can wrap the whole body of the original function in
//! block, put the single epilog after the block
//! and replace all explicit `return` with unconditional branches to the end of that block.
//!
//! Note, that we can't instrument all possible ways to return from the function. The simplest
//! example would be trap issued by the host function.
//! That means stack height global won't be equal to zero upon the next execution after such trap.
//!
//! # Stack cost
//!
//! Stack cost of the function is calculated as a sum of it's locals
//! and the maximal height of the value stack.
//!
//! All values are treated equally, as they have the same size.
//!
//! The rationale for this it makes it possible to use this very naive wasm executor, that is:
//!
//! - values are implemented by a union, so each value takes a size equal to
//!   the size of the largest possible value type this union can hold. (In MVP it is 8 bytes)
//! - each local variable and function argument is placed on the stack.
//! - arguments pushed by the caller are copied into callee stack rather than shared
//!   between the frames.

use parity_wasm::elements::{self, FunctionType, Internal, Type};
use parity_wasm::builder;
use rules;

use std::collections::HashMap;

mod max_height;

#[derive(Debug)]
pub struct Error(elements::Module);

impl Error {
	pub fn into_module(self) -> elements::Module {
		self.0
	}
}

macro_rules! instrument_call {
	($callee_idx: expr, $stack_height_global_idx: expr, $callee_stack_cost: expr, $stack_limit: expr) => {{
		use $crate::parity_wasm::elements::Opcode::*;
		[
			// stack_height += stack_cost(F)
			GetGlobal($stack_height_global_idx),
			I32Const($callee_stack_cost),
			I32Add,
			SetGlobal($stack_height_global_idx),
			// if stack_counter > LIMIT: unreachable
			GetGlobal($stack_height_global_idx),
			I32Const($stack_limit),
			I32GtU,
			If(elements::BlockType::NoResult),
			Unreachable,
			End,
			// Original call
			Call($callee_idx),
			// stack_height -= stack_cost(F)
			GetGlobal($stack_height_global_idx),
			I32Const($callee_stack_cost),
			I32Sub,
			SetGlobal($stack_height_global_idx),
		]
	}};
}

#[allow(unused)]
pub fn inject_stack_counter(
	module: elements::Module,
	rules: &rules::Set,
) -> Result<elements::Module, Error> {
	let mut mbuilder = builder::from_module(module);
	mbuilder = mbuilder
		.global()
			.value_type().i32()
			.mutable()
			.init_expr(elements::Opcode::I32Const(0))
			.build();

	let mut module = mbuilder.build();

	// Save index of `stack_height` global variable.
	let stack_height_global_idx = (module.globals_space() as u32) - 1;

	// Calculate stack costs for all original functions.
	let funcs_stack_costs = {
		let mut funcs_stack_costs = vec![0; module.functions_space()];
		for (func_idx, func_stack_cost) in funcs_stack_costs.iter_mut().enumerate() {
			*func_stack_cost = stack_cost(func_idx as u32, &module);
		}
		funcs_stack_costs
	};

	// Instrument functions.
	for section in module.sections_mut() {
		match *section {
			elements::Section::Code(ref mut code_section) => {
				for func_body in code_section.bodies_mut() {
					let mut opcodes = func_body.code_mut();
					instrument_function(
						opcodes,
						stack_height_global_idx,
						&funcs_stack_costs,
						rules.stack_limit(),
					);
				}
			}
			_ => {}
		}
	}

	//
	// Generate thunks for exports and tables.
	//

	struct Thunk {
		signature: FunctionType,
		// Index in function space of this thunk.
		idx: Option<u32>,
	}

	// First, we need to collect all function indicies that should be replaced by thunks.
	let mut replacement_map: HashMap<u32, Thunk> = {
		let exports = module.export_section().map(|es| es.entries()).unwrap_or(&[]);
		let elem_segments = module.elements_section().map(|es| es.entries()).unwrap_or(&[]);
		let functions = module.function_section().map(|fs| fs.entries()).unwrap_or(&[]);
		let types = module.type_section().map(|ts| ts.types()).unwrap_or(&[]);

		let func_type = |idx: u32| -> FunctionType {
			let type_idx = functions[idx as usize].type_ref();
			let Type::Function(ref ty) = types[type_idx as usize];
			ty.clone()
		};

		// Replacement map is atleast export_section size.
		let mut replacement_map: HashMap<u32, Thunk> =
			HashMap::with_capacity(exports.len());

		for entry in exports {
			match *entry.internal() {
				Internal::Function(ref function_idx) => {
					replacement_map.insert(
						*function_idx,
						Thunk {
							signature: func_type(*function_idx),
							idx: None,
						},
					);
				}
				_ => {}
			}
		}

		for segment in elem_segments {
			for function_idx in segment.members() {
				replacement_map.insert(
					*function_idx,
					Thunk {
						signature: func_type(*function_idx),
						idx: None,
					},
				);
			}
		}

		replacement_map
	};

	// Then, we create a thunk for each original function.
	let mut func_idx = module.functions_space() as u32;

	let mut mbuilder = builder::from_module(module);
	for (orig_func_idx, thunk) in &mut replacement_map {
		let callee_stack_cost = funcs_stack_costs[*orig_func_idx as usize];

		let mut thunk_body = instrument_call!(
			*orig_func_idx,
			stack_height_global_idx,
			callee_stack_cost as i32,
			rules.stack_limit() as i32
		).to_vec();
		thunk_body.push(elements::Opcode::End);

		mbuilder = mbuilder.function()
				// Signature of the thunk should match the original function signature.
				.signature()
					.with_params(thunk.signature.params().to_vec())
					.with_return_type(thunk.signature.return_type().clone())
					.build()
				.body()
					.with_opcodes(elements::Opcodes::new(
						thunk_body
					))
					.build()
				.build();
		thunk.idx = Some(func_idx);
	}
	let mut module = mbuilder.build();

	// And finally, fixup thunks in export and table sections.

	// Fixup original function index to a index of a thunk generated earlier.
	let fixup = |function_idx: &mut u32| {
		let thunk = replacement_map
			.get(function_idx)
			.expect("Replacement map should contain all functions from export section");
		*function_idx = thunk
			.idx
			.expect("At this point an index must be assigned to each thunk");
	};

	for section in module.sections_mut() {
		match *section {
			elements::Section::Export(ref mut export_section) => {
				for entry in export_section.entries_mut() {
					match *entry.internal_mut() {
						Internal::Function(ref mut function_idx) => fixup(function_idx),
						_ => {}
					}
				}
			}
			elements::Section::Element(ref mut elem_section) => {
				for segment in elem_section.entries_mut() {
					for function_idx in segment.members_mut() {
						fixup(function_idx)
					}
				}
			}
			_ => {}
		}
	}

	Ok(module)
}

fn instrument_function(
	opcodes: &mut elements::Opcodes,
	stack_height_global_idx: u32,
	funcs_stack_costs: &[u32],
	stack_limit: u32,
) {
	use parity_wasm::elements::Opcode::*;

	let mut cursor = 0;
	loop {
		if cursor >= opcodes.elements().len() {
			break;
		}

		enum Action {
			InstrumentCall(u32),
			Nop,
		}

		let action: Action = {
			let opcode = &opcodes.elements()[cursor];
			match *opcode {
				Call(ref idx) => Action::InstrumentCall(*idx),
				_ => Action::Nop,
			}
		};

		match action {
			// We need to wrap a `call idx` instruction
			// with a code that adjusts stack height counter
			// and then restores it.
			Action::InstrumentCall(callee_idx) => {
				let callee_stack_cost = funcs_stack_costs[callee_idx as usize];

				let new_seq = instrument_call!(
					callee_idx,
					stack_height_global_idx,
					callee_stack_cost as i32,
					stack_limit as i32
				);

				// Replace the original `call idx` instruction with
				// a wrapped call sequence.
				//
				// To splice actually take a place, we need to consume iterator
				// splice returns. So we just `count()` it.
				let seq_len = opcodes
					.elements_mut()
					.splice(cursor..(cursor + 1), new_seq.iter().cloned())
					.count();

				// Advance cursor to be after the inserted sequence.
				cursor += seq_len;
			}
			// Do nothing for other instructions.
			_ => {
				cursor += 1;
			}
		}
	}
}

/// Stack cost of the given function is the sum of it's locals count (that is,
/// number of arguments plus number of local variables) and the maximal stack
/// height.
fn stack_cost(func_idx: u32, module: &elements::Module) -> u32 {
	let code_section = module
		.code_section()
		.expect("Due to validation code section should exists");
	let body = &code_section.bodies()[func_idx as usize];

	let locals_count = body.locals().len() as u32;
	let max_stack_height = max_height::max_stack_height(func_idx, module);

	locals_count + max_stack_height
}

#[cfg(test)]
mod tests {
	extern crate wabt;
	use parity_wasm::elements;
	use super::*;

	fn parse_wat(source: &str) -> elements::Module {
		elements::deserialize_buffer(&wabt::wat2wasm(source).expect("Failed to wat2wasm"))
			.expect("Failed to deserialize the module")
	}

	#[test]
	fn simple_test() {
		let module = parse_wat(
			r#"
(module
	(func (export "simple")
		i32.const 123
		drop
	)
)
"#,
		);

		let module = inject_stack_counter(module, &Default::default()).unwrap();
		elements::serialize_to_file("test.wasm", module).unwrap();

		// let binary = elements::serialize(module).unwrap();
		// let wat = wabt::wasm2wat(binary).unwrap();

		// println!("{}", wat);
		// panic!()
	}
}

