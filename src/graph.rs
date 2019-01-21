//! Wasm binary graph format

use parity_wasm::elements;
use std::cell::RefCell;
use std::rc::Rc;

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

type TypeRef = Rc<RefCell<elements::Type>>;
type FuncRef = Rc<RefCell<Func>>;
type GlobalRef = Rc<RefCell<Global>>;
type MemoryRef = Rc<RefCell<Memory>>;
type TableRef = Rc<RefCell<Table>>;

struct Func {
	type_ref: TypeRef,
	origin: FuncOrigin,
}

impl Func {
	fn into_ref(self) -> Rc<RefCell<Self>> {
		Rc::from(RefCell::from(self))
	}
}

struct Global {
	content: elements::ValueType,
	is_mut: bool,
	origin: GlobalOrigin,
}

impl Global {
	fn into_ref(self) -> Rc<RefCell<Self>> {
		Rc::from(RefCell::from(self))
	}
}

enum Instruction {
	Plain(elements::Instruction),
	Call(FuncRef),
}

struct Memory {
	limits: elements::ResizableLimits,
	origin: MemoryOrigin,
}

impl Memory {
	fn into_ref(self) -> Rc<RefCell<Self>> {
		Rc::from(RefCell::from(self))
	}
}

struct Table {
	origin: TableOrigin,
	limits: elements::ResizableLimits,
}

impl Table {
	fn into_ref(self) -> Rc<RefCell<Self>> {
		Rc::from(RefCell::from(self))
	}
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
	Func(FuncRef),
	Global(GlobalRef),
	Table(TableRef),
	Memory(MemoryRef),
}

#[derive(Default)]
struct Module {
	types: Vec<TypeRef>,
	funcs: Vec<FuncRef>,
	tables: Vec<TableRef>,
	memory: Vec<MemoryRef>,
	globals: Vec<GlobalRef>,
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
					res.types = type_section
						.types()
						.iter()
						.cloned()
						.map(RefCell::<_>::from)
						.map(Rc::<_>::from)
						.collect();
				},
				elements::Section::Import(import_section) => {
					for entry in import_section.entries() {
						match *entry.external() {
							elements::External::Function(f) => {
								res.funcs.push(Func {
									type_ref: res.types[f as usize].clone(),
									origin: entry.into(),
								}.into_ref())
							},
							elements::External::Memory(m) => {
								res.memory.push(Memory {
									limits: m.limits().clone(),
									origin: entry.into(),
								}.into_ref())
							},
							elements::External::Global(g) => {
								res.globals.push(Global {
									content: g.content_type(),
									is_mut: g.is_mutable(),
									origin: entry.into(),
								}.into_ref())
							},
							elements::External::Table(t) => {
								res.tables.push(Table {
									limits: t.limits().clone(),
									origin: entry.into(),
								}.into_ref())
							},
						}
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