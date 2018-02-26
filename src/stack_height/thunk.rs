use parity_wasm::elements::{self, FunctionType, Internal};
use parity_wasm::builder;

use std::collections::HashMap;

use super::{resolve_func_type, Context, Error};

struct Thunk {
	signature: FunctionType,
	// Index in function space of this thunk.
	idx: Option<u32>,
	original_func_idx: u32,
	callee_stack_cost: u32,
}

pub(crate) fn generate_thunks(
	ctx: &mut Context,
	module: elements::Module,
) -> Result<elements::Module, Error> {
	// First, we need to collect all function indicies that should be replaced by thunks

	// Function indicies which needs to generate thunks.
	let mut need_thunks: Vec<u32> = Vec::new();

	let mut replacement_map: HashMap<u32, Thunk> = {
		let exports = module
			.export_section()
			.map(|es| es.entries())
			.unwrap_or(&[]);
		let elem_segments = module
			.elements_section()
			.map(|es| es.entries())
			.unwrap_or(&[]);

		let exported_func_indicies = exports.iter().filter_map(|entry| match *entry.internal() {
			Internal::Function(ref function_idx) => Some(*function_idx),
			_ => None,
		});
		let table_func_indicies = elem_segments
			.iter()
			.flat_map(|segment| segment.members())
			.cloned();

		// Replacement map is at least export section size.
		let mut replacement_map: HashMap<u32, Thunk> = HashMap::new();

		for func_idx in exported_func_indicies.chain(table_func_indicies) {
			let callee_stack_cost = ctx.stack_cost(func_idx).ok_or_else(|| {
				Error(format!("function with idx {} isn't found", func_idx))
			})?;

			// Don't generate a thunk if stack_cost of a callee is zero.
			if callee_stack_cost != 0 {
				need_thunks.push(func_idx);
				replacement_map.insert(func_idx, Thunk {
					signature: resolve_func_type(func_idx, &module).clone(),
					idx: None,
					callee_stack_cost,
					original_func_idx: func_idx,
				});
			}
		}

		replacement_map
	};

	// Then, we generate a thunk for each original function.

	// Save current func_idx
	let mut next_func_idx = module.functions_space() as u32;

	let mut mbuilder = builder::from_module(module);
	for func_idx in need_thunks {
		let mut thunk = replacement_map
			.get_mut(&func_idx)
			.expect(
				"`func_idx` should come from `need_thunks`;
				`need_thunks` is populated with the same items that in `replacement_map`;
				qed"
			);

		let instrumented_call = instrument_call!(
			thunk.original_func_idx as u32,
			thunk.callee_stack_cost as i32,
			ctx.stack_height_global_idx(),
			ctx.stack_limit()
		);
		// Thunk body consist of:
		//  - argument pushing
		//  - instrumented call
		//  - end
		let mut thunk_body: Vec<elements::Opcode> = Vec::with_capacity(
			thunk.signature.params().len() +
			instrumented_call.len() +
			1
		);

		for (arg_idx, _) in thunk.signature.params().iter().enumerate() {
			thunk_body.push(elements::Opcode::GetLocal(arg_idx as u32));
		}
		thunk_body.extend(instrumented_call.iter().cloned());
		thunk_body.push(elements::Opcode::End);

		// TODO: Don't generate a signature, but find an existing one.

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
		if let Some(ref thunk) = replacement_map.get(function_idx) {
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
