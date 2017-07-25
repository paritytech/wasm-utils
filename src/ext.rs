use parity_wasm::{elements, builder};

type Insertion = (usize, u32, u32, String);

pub fn update_call_index(opcodes: &mut elements::Opcodes, original_imports: usize, inserts: &[Insertion]) {
	use parity_wasm::elements::Opcode::*;
	for opcode in opcodes.elements_mut().iter_mut() {
		match opcode {
			&mut Call(ref mut call_index) => {
				if let Some(pos) = inserts.iter().position(|x| x.1 == *call_index) {
					*call_index = (original_imports + pos) as u32;
				} else if *call_index as usize > original_imports {
					*call_index += inserts.len() as u32;
				}
			},
			_ => { }
		}
	}
}

pub fn externalize(
	module: elements::Module,
	replaced_funcs: Vec<&str>,
) -> elements::Module {
   // Save import functions number for later
	let import_funcs_total = module
		.import_section().expect("Import section to exist")
		.entries()
		.iter()
		.filter(|e| if let &elements::External::Function(_) = e.external() { true } else { false })
		.count();

	// First, we find functions indices that are to be rewired to externals
	//   Triple is (function_index (callable), type_index, function_name)
	let mut replaces: Vec<Insertion> = replaced_funcs
		.into_iter()
		.filter_map(|f| {
			let export = module
				.export_section().expect("Export section to exist")
				.entries().iter().enumerate()
				.find(|&(_, entry)| entry.field() == f)
				.expect("All functions of interest to exist");

			if let &elements::Internal::Function(func_idx) = export.1.internal() {
				let type_ref = module
					.function_section().expect("Functions section to exist")
					.entries()[func_idx as usize - import_funcs_total]
					.type_ref();

				Some((export.0, func_idx, type_ref, export.1.field().to_owned()))
			} else {
				None
			}
		})
		.collect();

	replaces.sort_by_key(|e| e.0);

	// Second, we duplicate them as import definitions
	let mut mbuilder = builder::from_module(module);
	for &(_, _, type_ref, ref field) in replaces.iter() {
		mbuilder.push_import(
			builder::import()
				.module("env")
				.field(field)
				.external().func(type_ref)
				.build()
		);
	}

	// Back to mutable access
	let mut module = mbuilder.build();

	// Third, rewire all calls to imported functions and update all other calls indices
	for section in module.sections_mut() {
		match section {
			&mut elements::Section::Code(ref mut code_section) => {
				for ref mut func_body in code_section.bodies_mut() {
					update_call_index(func_body.code_mut(), import_funcs_total, &replaces);
				}
			},
			&mut elements::Section::Export(ref mut export_section) => {
				for ref mut export in export_section.entries_mut() {
					match export.internal_mut() {
						&mut elements::Internal::Function(ref mut func_index) => {
							if *func_index >= import_funcs_total as u32 { *func_index += replaces.len() as u32; }
						},
						_ => {}
					}
				}
			},
			&mut elements::Section::Element(ref mut elements_section) => {
				for ref mut segment in elements_section.entries_mut() {
					// update all indirect call addresses initial values
					for func_index in segment.members_mut() {
						if *func_index >= import_funcs_total as u32 { *func_index += replaces.len() as u32; }
					}
				}
			},
			_ => { }
		}
	}

	module

}