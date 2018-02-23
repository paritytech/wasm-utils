//! The pass that tries to make stack overflows deterministic, by introducing
//! an upper bound of the stack size.
//!
//! This pass introduces a global mutable variable to track stack height,
//! and instruments all calls with preamble and postamble.
//!
//! Stack height is increased prior the call. Otherwise, the check would
//! be made after the stack frame is allocated.
//!
//! The preamble is inserted before the call. It increments
//! the global stack height variable with statically determined "stack cost"
//! of the callee. If after the increment the stack height exceeds
//! the limit (specified by the `rules`) then execution traps.
//! Otherwise, the call is executed.
//!
//! The postamble is inserted after the call. The purpose of the postamble is to decrease
//! the stack height by the "stack cost" of the callee function.
//!
//! Note, that we can't instrument all possible ways to return from the function. The simplest
//! example would be a trap issued by the host function.
//! That means stack height global won't be equal to zero upon the next execution after such trap.
//!
//! # Thunks
//!
//! Because stack height is increased prior the call few problems arises:
//!
//! - Stack height isn't increased upon an entry to the first function, i.e. exported function.
//! - It is statically unknown what function will be invoked in an indirect call.
//!
//! The solution for this problems is to generate a intermediate functions, called 'thunks', which
//! will increase before and decrease the stack height after the call to original function, and
//! then make exported function and table entries to point to a corresponding thunks.
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
//! - each value from the value stack is placed on the native stack.
//! - each local variable and function argument is placed on the native stack.
//! - arguments pushed by the caller are copied into callee stack rather than shared
//!   between the frames.
//! - upon entry into the function entire stack frame is allocated.

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
		.value_type()
		.i32()
		.mutable()
		.init_expr(elements::Opcode::I32Const(0))
		.build();

	let mut module = mbuilder.build();

	// Save index of `stack_height` global variable.
	let stack_height_global_idx = (module.globals_space() as u32) - 1;

	// Calculate stack costs for all original functions.
	let funcs_stack_costs = {
		let func_imports = module.import_count(elements::ImportCountType::Function);
		let mut funcs_stack_costs = vec![0; module.functions_space()];
		// TODO: optimize!
		for (func_idx, func_stack_cost) in funcs_stack_costs.iter_mut().enumerate() {
			// We can't calculate stack_cost of the import functions.
			if func_idx >= func_imports {
				*func_stack_cost = stack_cost(func_idx as u32, &module);
			}
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
		callee_stack_cost: u32,
	}

	// First, we need to collect all function indicies that should be replaced by thunks.
	let mut replacement_map: HashMap<u32, Thunk> = {
		let func_imports = module.import_count(elements::ImportCountType::Function);
		let exports = module
			.export_section()
			.map(|es| es.entries())
			.unwrap_or(&[]);
		let elem_segments = module
			.elements_section()
			.map(|es| es.entries())
			.unwrap_or(&[]);
		let functions = module
			.function_section()
			.map(|fs| fs.entries())
			.unwrap_or(&[]);
		let types = module.type_section().map(|ts| ts.types()).unwrap_or(&[]);

		// Replacement map is at least export_section size.
		let mut replacement_map: HashMap<u32, Thunk> = HashMap::with_capacity(exports.len());

		{
			// This function will check if the function needs a thunk,
			// add into a replacement_map if one is needed.
			let mut add_candidate_thunk = |func_idx: u32| {
				let callee_stack_cost = funcs_stack_costs[func_idx as usize];
				if callee_stack_cost == 0 {
					return;
				}

				replacement_map.insert(
					func_idx,
					Thunk {
						signature: resolve_func_type(func_idx, &module).clone(),
						idx: None,
						callee_stack_cost,
					},
				);
			};

			for entry in exports {
				match *entry.internal() {
					Internal::Function(ref function_idx) => add_candidate_thunk(*function_idx),
					_ => {}
				}
			}

			for segment in elem_segments {
				for function_idx in segment.members() {
					add_candidate_thunk(*function_idx)
				}
			}
		}

		replacement_map
	};

	// Then, we create a thunk for each original function.

	// Save current func_idx
	let mut next_func_idx = module.functions_space() as u32;

	let mut mbuilder = builder::from_module(module);
	for (orig_func_idx, thunk) in &mut replacement_map {
		// Thunk body consist of:
		//  - argument pushing
		//  - instrumented call
		//  - end
		let instrumented_call = instrument_call!(
			*orig_func_idx,
			stack_height_global_idx,
			thunk.callee_stack_cost as i32,
			rules.stack_limit() as i32
		);
		let mut thunk_body: Vec<elements::Opcode> = Vec::with_capacity(instrumented_call.len() + 1);

		for (arg_idx, _) in thunk.signature.params().iter().enumerate() {
			thunk_body.push(elements::Opcode::GetLocal(arg_idx as u32));
		}
		thunk_body.extend(instrumented_call.iter().cloned());
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

		thunk.idx = Some(next_func_idx);
		next_func_idx += 1;
	}
	let mut module = mbuilder.build();

	// And finally, fixup thunks in export and table sections.

	// Fixup original function index to a index of a thunk generated earlier.
	let fixup = |function_idx: &mut u32| {
		// Check whether this function is in replacement_map, since
		// we can skip thunk generation (e.g. if stack_cost of function is 0).
		if let Some(thunk) = replacement_map.get(function_idx) {
			*function_idx = thunk
				.idx
				.expect("At this point an index must be assigned to each thunk");
		}
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
				let _ = opcodes
					.elements_mut()
					.splice(cursor..(cursor + 1), new_seq.iter().cloned())
					.count();

				// Advance cursor to be after the inserted sequence.
				cursor += new_seq.len();
			}
			// Do nothing for other instructions.
			_ => {
				cursor += 1;
			}
		}
	}
}

/// Stack cost of the given *defined* function is the sum of it's locals count (that is,
/// number of arguments plus number of local variables) and the maximal stack
/// height.
fn stack_cost(func_idx: u32, module: &elements::Module) -> u32 {
	// To calculate the cost of a function we need to convert index from
	// function index space to defined function spaces.
	let func_imports = module.import_count(elements::ImportCountType::Function) as u32;
	let defined_func_idx = func_idx
		.checked_sub(func_imports)
		.expect("This should be a index of a defined function");

	let code_section = module
		.code_section()
		.expect("Due to validation code section should exists");
	let body = &code_section.bodies()[defined_func_idx as usize];

	let locals_count = body.locals().len() as u32;
	let max_stack_height = max_height::max_stack_height(defined_func_idx, module);

	locals_count + max_stack_height
}

fn resolve_func_type(func_idx: u32, module: &elements::Module) -> &elements::FunctionType {
	let func_section = module.function_section().unwrap();
	let type_section = module.type_section().unwrap();

	let func_imports = module.import_count(elements::ImportCountType::Function);
	let sig_idx = if func_idx < func_imports as u32 {
		module
			.import_section()
			.expect("function import count is not zero; function section must exists; qed")
			.entries()
			.iter()
			.filter_map(|entry| match *entry.external() {
				elements::External::Function(ref idx) => Some(*idx),
				_ => None,
			})
			.nth(func_idx as usize)
			.unwrap()
	} else {
		func_section.entries()[func_idx as usize - func_imports].type_ref()
	};
	let Type::Function(ref ty) = type_section.types()[sig_idx as usize];
	ty
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

	fn validate_module(module: elements::Module) {
		let binary = elements::serialize(module).expect("Failed to serialize");
		wabt::Module::read_binary(&binary, &Default::default())
			.expect("Wabt failed to read final binary")
			.validate()
			.expect("Invalid module");
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
	}

	#[test]
	fn test_with_params_and_result() {
		let module = parse_wat(
			r#"
(module
  (func (export "i32.add") (param i32 i32) (result i32)
    get_local 0
	get_local 1
	i32.add
  )
)
"#,
		);

		let module = inject_stack_counter(module, &Default::default())
			.expect("Failed to inject stack counter");
		validate_module(module);
	}

	#[test]
	fn simple_with_imports() {
		let module = parse_wat(
			r#"
(module
  (import "env" "foo" (func $foo))
  (import "env" "boo" (func $boo))
  (func (export "i32.add") (param i32 i32) (result i32)
    call $foo
	call $boo
    get_local 0
	get_local 1
	i32.add
  )
)
"#,
		);

		let module = inject_stack_counter(module, &Default::default()).unwrap();
		validate_module(module);
	}

	#[test]
	fn simple_with_global() {
		let module = parse_wat(
			r#"
(module
  (import "env" "foo" (func $foo))
  (global (mut i32) (i32.const 1))
  (func $i32.add (export "i32.add") (param i32 i32) (result i32)
    get_local 0
	get_local 1
	i32.add
  )
  (func (param i32)
     get_local 0
     i32.const 0
     call $i32.add
     drop
  )
)
"#,
		);

		let module = inject_stack_counter(module, &Default::default()).unwrap();
		validate_module(module);
	}

	#[test]
	fn simple_with_table() {
		let module = parse_wat(
			r#"
(module
  (import "env" "foo" (func $foo))
  (global (mut i32) (i32.const 1))
  (func $i32.add (export "i32.add") (param i32 i32) (result i32)
    get_local 0
	get_local 1
	i32.add
  )
  (func (param i32)
     get_local 0
     i32.const 0
     call $i32.add
     drop
  )
  (table 10 anyfunc)
  (elem (i32.const 0) 0 1 2)
)
"#,
		);

		let module = inject_stack_counter(module, &Default::default()).unwrap();
		validate_module(module.clone());
	}
}
