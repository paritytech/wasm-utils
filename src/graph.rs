//! Wasm binary graph format

use parity_wasm::elements;
use super::ref_list::{RefList, EntryRef};

enum ImportedOrDeclared<T=()> {
	Imported(String, String),
	Declared(T),
}

impl<T> ImportedOrDeclared<T> {
	fn imported(module: String, name: String) -> Self {
		ImportedOrDeclared::Imported(module, name)
	}
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

struct DataSegment {
	offset_expr: Vec<Instruction>,
	data: Vec<u8>,
}

struct ElementSegment {
	offset_expr: Vec<Instruction>,
	data: Vec<u32>,
}

enum Export {
	Func(EntryRef<Func>),
	Global(EntryRef<Global>),
	Table(EntryRef<Table>),
	Memory(EntryRef<Memory>),
}

#[derive(Default)]
struct Module {
	types: RefList<elements::Type>,
	funcs: RefList<Func>,
	tables: RefList<Table>,
	memory: RefList<Memory>,
	globals: RefList<Global>,
	elements: Vec<ElementSegment>,
	data: Vec<DataSegment>,
	exports: Vec<Export>,
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
				_ => continue,
			}
		}

		res
	}

}

#[cfg(test)]
mod tests {

}