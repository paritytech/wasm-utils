//! Wasm binary graph format

use parity_wasm::elements;
use super::ref_list::{RefList, EntryRef};
use std::vec::Vec;
use std::borrow::ToOwned;
use std::string::String;

enum ImportedOrDeclared<T=()> {
	Imported(String, String),
	Declared(T),
}

impl<T> From<&elements::ImportEntry> for ImportedOrDeclared<T> {
	fn from(v: &elements::ImportEntry) -> Self {
		ImportedOrDeclared::Imported(v.module().to_owned(), v.field().to_owned())
	}
}

type FuncOrigin = ImportedOrDeclared<Vec<Instruction>>;
type GlobalOrigin = ImportedOrDeclared<Vec<Instruction>>;
type MemoryOrigin = ImportedOrDeclared;
type TableOrigin = ImportedOrDeclared;

struct Func {
	type_ref: EntryRef<elements::Type>,
	origin: FuncOrigin,
}

struct Global {
	content: elements::ValueType,
	is_mut: bool,
	origin: GlobalOrigin,
}

enum Instruction {
	Plain(elements::Instruction),
	Call(EntryRef<Func>),
}

struct Memory {
	limits: elements::ResizableLimits,
	origin: MemoryOrigin,
}

struct Table {
	origin: TableOrigin,
	limits: elements::ResizableLimits,
}

enum SegmentLocation {
	Passive,
	Default(Vec<Instruction>),
	WithIndex(u32, Vec<Instruction>),
}

struct DataSegment {
	location: SegmentLocation,
	value: Vec<u8>,
}

struct ElementSegment {
	location: SegmentLocation,
	value: Vec<u32>,
}

enum ExportLocal {
	Func(EntryRef<Func>),
	Global(EntryRef<Global>),
	Table(EntryRef<Table>),
	Memory(EntryRef<Memory>),
}

struct Export {
	name: String,
	local: ExportLocal,
}

#[derive(Default)]
struct Module {
	types: RefList<elements::Type>,
	funcs: RefList<Func>,
	memory: RefList<Memory>,
	tables: RefList<Table>,
	globals: RefList<Global>,
	start: Option<EntryRef<Func>>,
	exports: Vec<Export>,
	elements: Vec<ElementSegment>,
	data: Vec<DataSegment>,
}

impl Module {

	fn from_elements(module: &elements::Module) -> Self {

		let mut res = Module::default();

		for section in module.sections() {
			match section {
				elements::Section::Type(type_section) => {
					res.types = RefList::from_slice(type_section.types());
				},
				elements::Section::Import(import_section) => {
					for entry in import_section.entries() {
						match *entry.external() {
							elements::External::Function(f) => {
								res.funcs.push(Func {
									type_ref: res.types.get(f as usize).expect("validated; qed").clone(),
									origin: entry.into(),
								});
							},
							elements::External::Memory(m) => {
								res.memory.push(Memory {
									limits: m.limits().clone(),
									origin: entry.into(),
								});
							},
							elements::External::Global(g) => {
								res.globals.push(Global {
									content: g.content_type(),
									is_mut: g.is_mutable(),
									origin: entry.into(),
								});
							},
							elements::External::Table(t) => {
								res.tables.push(Table {
									limits: t.limits().clone(),
									origin: entry.into(),
								});
							},
						};
					}
				},
				elements::Section::Function(function_section) => {
					for f in function_section.entries() {
						res.funcs.push(Func {
							type_ref: res.types.get(f.type_ref() as usize).expect("validated; qed").clone(),
							// code will be populated later
							origin: ImportedOrDeclared::Declared(Vec::new()),
						});
					};
				},
				elements::Section::Table(table_section) => {
					for t in table_section.entries() {
						res.tables.push(Table {
							limits: t.limits().clone(),
							origin: ImportedOrDeclared::Declared(()),
						});
					}
				},
				elements::Section::Memory(table_section) => {
					for t in table_section.entries() {
						res.memory.push(Memory {
							limits: t.limits().clone(),
							origin: ImportedOrDeclared::Declared(()),
						});
					}
				},
				elements::Section::Global(global_section) => {
					for g in global_section.entries() {
						res.globals.push(Global {
							content: g.global_type().content_type(),
							is_mut: g.global_type().is_mutable(),
							// TODO: init expr
							origin: ImportedOrDeclared::Declared(Vec::new()),
						});
					}
				},
				elements::Section::Export(export_section) => {
					for e in export_section.entries() {
						let local = match e.internal() {
							&elements::Internal::Function(func_idx) => {
								ExportLocal::Func(res.funcs.clone_ref(func_idx as usize))
							},
							&elements::Internal::Global(global_idx) => {
								ExportLocal::Global(res.globals.clone_ref(global_idx as usize))
							},
							&elements::Internal::Memory(mem_idx) => {
								ExportLocal::Memory(res.memory.clone_ref(mem_idx as usize))
							},
							&elements::Internal::Table(table_idx) => {
								ExportLocal::Table(res.tables.clone_ref(table_idx as usize))
							},
						};

						res.exports.push(Export { local: local, name: e.field().to_owned() })
					}
				},
				elements::Section::Start(start_func) => {
					res.start = Some(res.funcs.clone_ref(*start_func as usize));
				},
				elements::Section::Element(element_section) => {
					for element_segment in element_section.entries() {

						// let location = if element_segment.passive() {
						// 	SegmentLocation::Passive
						// } else if element_segment.index() == 0 {
						// 	// TODO: transform instructions
						// 	SegmentLocation::Default(Vec::new())
						// } else {
						// 	// TODO: transform instructions
						// 	SegmentLocation::WithIndex(element_segment.index(), Vec::new())
						// };

						// TODO: transform instructions
						// TODO: update parity-wasm and uncomment the above
						let location = SegmentLocation::Default(Vec::new());

						res.elements.push(ElementSegment {
							value: element_segment.members().to_vec(),
							location: location,
						});
					}
				},
				elements::Section::Data(data_section) => {
					for data_segment in data_section.entries() {
						// TODO: transform instructions
						// TODO: update parity-wasm and uncomment the above
						let location = SegmentLocation::Default(Vec::new());

						res.data.push(DataSegment {
							value: data_segment.value().to_vec(),
							location: location,
						});
					}
				}
				_ => continue,
			}
		}

		res
	}

}

fn parse(wasm: &[u8]) -> Module {
	Module::from_elements(&::parity_wasm::deserialize_buffer(wasm).expect("failed to parse wasm"))
}

#[cfg(test)]
mod tests {

	extern crate wabt;
	use parity_wasm;

	#[test]
	fn smoky() {
		let wasm = wabt::wat2wasm(r#"
			(module
				(type (func))
				(func (type 0))
				(memory 0 1)
				(export "simple" (func 0))
			)
		"#).expect("Failed to read fixture");

		let f = super::parse(&wasm[..]);

		assert_eq!(f.types.len(), 1);
		assert_eq!(f.funcs.len(), 1);
		assert_eq!(f.tables.len(), 0);
		assert_eq!(f.memory.len(), 1);
		assert_eq!(f.exports.len(), 1);

		assert_eq!(f.types.get_ref(0).link_count(), 1);
		assert_eq!(f.funcs.get_ref(0).link_count(), 1);
	}
}