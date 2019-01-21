//! Wasm binary graph format

use parity_wasm::elements;
use std::cell::RefCell;
use std::rc::Rc;

enum ImportedOrDeclared<T=()> {
	Imported(String, String),
	Declared(T),
}

type FuncOrigin = ImportedOrDeclared<Vec<Instruction>>;
type GlobalOrigin = ImportedOrDeclared<Vec<Instruction>>;
type MemoryOrigin = ImportedOrDeclared;
type TableOrigin = ImportedOrDeclared;

type TypeRef = Rc<RefCell<elements::Type>>;
type FuncRef = Rc<RefCell<Func>>;
type GlobalRef = Rc<RefCell<Global>>;

struct Func {
	type_ref: TypeRef,
	origin: FuncOrigin,
}

struct Global {
	content: elements::ValueType,
	is_mut: bool,
	origin: GlobalOrigin,
}

enum Instruction {
	Plain(elements::Instruction),
	Call(FuncRef),
}

struct Memory {
	limits: elements::ResizableLimits,
	origin: MemoryOrigin,
}

struct Table {
	origin: TableOrigin,
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
}

#[derive(Default)]
struct Module {
	types: Vec<TypeRef>,
	funcs: Vec<FuncRef>,
	tables: Vec<Table>,
	memories: Vec<Memory>,
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
					res.types = type_section.types().iter().cloned().map(|t| Rc::new(RefCell::new(t))).collect();
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