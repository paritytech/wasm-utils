use std::string::String;
use std::vec::Vec;
use std::borrow::ToOwned;

use parity_wasm::{elements, builder};
use optimizer::{import_section, export_section};
use byteorder::{LittleEndian, ByteOrder};

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

pub fn memory_section<'a>(module: &'a mut elements::Module) -> Option<&'a mut elements::MemorySection> {
   for section in module.sections_mut() {
		match section {
			&mut elements::Section::Memory(ref mut sect) => {
				return Some(sect);
			},
			_ => { }
		}
	}
	None
}

pub fn externalize_mem(mut module: elements::Module, adjust_pages: Option<u32>, max_pages: u32) -> elements::Module {
	let mut entry = memory_section(&mut module)
		.expect("Memory section to exist")
		.entries_mut()
		.pop()
		.expect("Own memory entry to exist in memory section");

	if let Some(adjust_pages) = adjust_pages {
		assert!(adjust_pages <= max_pages);
		entry = elements::MemoryType::new(adjust_pages, Some(max_pages));
	}

	if entry.limits().maximum().is_none() {
		entry = elements::MemoryType::new(entry.limits().initial(), Some(max_pages));
	}

	let mut builder = builder::from_module(module);
	builder.push_import(
		elements::ImportEntry::new(
			"env".to_owned(),
			"memory".to_owned(),
			elements::External::Memory(entry),
		)
	);

	builder.build()
}

fn foreach_public_func_name<F>(mut module: elements::Module, f: F) -> elements::Module
where F: Fn(&mut String)
{
	import_section(&mut module).map(|is| {
		for entry in is.entries_mut() {
			if let elements::External::Function(_) = *entry.external() {
				f(entry.field_mut())
			}
		}
	});

	export_section(&mut module).map(|es| {
		for entry in es.entries_mut() {
			if let elements::Internal::Function(_) = *entry.internal() {
				f(entry.field_mut())
			}
		}
	});

	module
}

pub fn underscore_funcs(module: elements::Module) -> elements::Module {
	foreach_public_func_name(module, |n| n.insert(0, '_'))
}

pub fn ununderscore_funcs(module: elements::Module) -> elements::Module {
	foreach_public_func_name(module, |n| { n.remove(0); })
}

pub fn shrink_unknown_stack(
	mut module: elements::Module,
	// for example, `shrink_amount = (1MB - 64KB)` will limit stack to 64KB
	shrink_amount: u32,
) -> (elements::Module, u32) {
	let mut new_stack_top = 0;
	for section in module.sections_mut() {
		match section {
			&mut elements::Section::Data(ref mut data_section) => {
				for ref mut data_segment in data_section.entries_mut() {
					if data_segment.offset().code() == &[elements::Opcode::I32Const(4), elements::Opcode::End] {
						assert_eq!(data_segment.value().len(), 4);
						let current_val = LittleEndian::read_u32(data_segment.value());
						let new_val = current_val - shrink_amount;
						LittleEndian::write_u32(data_segment.value_mut(), new_val);
						new_stack_top = new_val;
					}
				}
			},
			_ => continue
		}
	}
	(module, new_stack_top)
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
