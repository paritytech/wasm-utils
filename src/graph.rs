//! Wasm binary graph format

use parity_wasm::elements;
use super::ref_list::{RefList, EntryRef};
use std::vec::Vec;
use std::borrow::ToOwned;
use std::string::String;
use std::collections::BTreeMap;

/// Imported or declared variant of the same thing.
///
/// In WebAssembly, function/global/memory/table instances can either be
/// imported or declared internally, forming united index space.
#[derive(Debug)]
pub enum ImportedOrDeclared<T=()> {
	/// Variant for imported instances.
	Imported(String, String),
	/// Variant for instances declared internally in the module.
	Declared(T),
}

impl<T> From<&elements::ImportEntry> for ImportedOrDeclared<T> {
	fn from(v: &elements::ImportEntry) -> Self {
		ImportedOrDeclared::Imported(v.module().to_owned(), v.field().to_owned())
	}
}

/// Function origin (imported or internal).
pub type FuncOrigin = ImportedOrDeclared<FuncBody>;
/// Global origin (imported or internal).
pub type GlobalOrigin = ImportedOrDeclared<Vec<Instruction>>;
/// Memory origin (imported or internal).
pub type MemoryOrigin = ImportedOrDeclared;
/// Table origin (imported or internal).
pub type TableOrigin = ImportedOrDeclared;

/// Function body.
///
/// Function consist of declaration (signature, i.e. type reference)
/// and the actual code. This part is the actual code.
#[derive(Debug)]
pub struct FuncBody {
	pub locals: Vec<elements::Local>,
	pub code: Vec<Instruction>,
}

/// Function declaration.
///
/// As with other instances, functions can be either imported or declared
/// within the module - `origin` field is handling this.
#[derive(Debug)]
pub struct Func {
	/// Function signature/type reference.
	pub type_ref: EntryRef<elements::Type>,
	/// Where this function comes from (imported or declared).
	pub origin: FuncOrigin,
}

/// Global declaration.
///
/// As with other instances, globals can be either imported or declared
/// within the module - `origin` field is handling this.
#[derive(Debug)]
pub struct Global {
	pub content: elements::ValueType,
	pub is_mut: bool,
	pub origin: GlobalOrigin,
}

/// Instruction.
///
/// Some instructions don't reference any entities within the WebAssembly module,
/// while others do. This enum is for tracking references when required.
#[derive(Debug)]
pub enum Instruction {
	/// WebAssembly instruction that does not reference any module entities.
	Plain(elements::Instruction),
	/// Call instruction which references the function.
	Call(EntryRef<Func>),
	/// Indirect call instruction which references function type (function signature).
	CallIndirect(EntryRef<elements::Type>, u8),
	/// get_global instruction which references the global.
	GetGlobal(EntryRef<Global>),
	/// set_global instruction which references the global.
	SetGlobal(EntryRef<Global>),
}

/// Memory instance decriptor.
///
/// As with other similar instances, memory instances can be either imported
/// or declared within the module - `origin` field is handling this.
#[derive(Debug)]
pub struct Memory {
	/// Declared limits of the table instance.
	pub limits: elements::ResizableLimits,
	/// Origin of the memory instance (internal or imported).
	pub origin: MemoryOrigin,
}

/// Memory instance decriptor.
///
/// As with other similar instances, memory instances can be either imported
/// or declared within the module - `origin` field is handling this.
#[derive(Debug)]
pub struct Table {
	/// Declared limits of the table instance.
	pub limits: elements::ResizableLimits,
	/// Origin of the table instance (internal or imported).
	pub origin: TableOrigin,
}

/// Segment location.
///
/// Reserved for future use. Currenty only `Default` variant is supported.
#[derive(Debug)]
pub enum SegmentLocation {
	/// Not used currently.
	Passive,
	/// Default segment location with index `0`.
	Default(Vec<Instruction>),
	/// Not used currently.
	WithIndex(u32, Vec<Instruction>),
}

/// Data segment of data section.
#[derive(Debug)]
pub struct DataSegment {
	/// Location of the segment in the linear memory.
	pub location: SegmentLocation,
	/// Raw value of the data segment.
	pub value: Vec<u8>,
}

/// Element segment of element section.
#[derive(Debug)]
pub struct ElementSegment {
	/// Location of the segment in the table space.
	pub location: SegmentLocation,
	/// Raw value (function indices) of the element segment.
	pub value: Vec<u32>,
}

/// Export entry reference.
///
/// Module can export function, global, table or memory instance
/// under specific name (field).
#[derive(Debug)]
pub enum ExportLocal {
	/// Function reference.
	Func(EntryRef<Func>),
	/// Global reference.
	Global(EntryRef<Global>),
	/// Table reference.
	Table(EntryRef<Table>),
	/// Memory reference.
	Memory(EntryRef<Memory>),
}

/// Export entry description.
#[derive(Debug)]
pub struct Export {
	/// Name (field) of the export entry.
	pub name: String,
	/// What entity is exported.
	pub local: ExportLocal,
}

/// Module
#[derive(Debug, Default)]
pub struct Module {
	pub types: RefList<elements::Type>,
	pub funcs: RefList<Func>,
	pub memory: RefList<Memory>,
	pub tables: RefList<Table>,
	pub globals: RefList<Global>,
	pub start: Option<EntryRef<Func>>,
	pub exports: Vec<Export>,
	pub elements: Vec<ElementSegment>,
	pub data: Vec<DataSegment>,
	pub other: BTreeMap<usize, elements::Section>,
}

impl Module {

	fn map_instructions(&self, instructions: &[elements::Instruction]) -> Vec<Instruction> {
		use parity_wasm::elements::Instruction::*;
		instructions.iter().map(|instruction|  match instruction {
			Call(func_idx) => Instruction::Call(self.funcs.clone_ref(*func_idx as usize)),
			CallIndirect(type_idx, arg2) =>
				Instruction::CallIndirect(
					self.types.clone_ref(*type_idx as usize),
					*arg2,
				),
			SetGlobal(global_idx) =>
				Instruction::SetGlobal(self.globals.clone_ref(*global_idx as usize)),
			GetGlobal(global_idx) =>
				Instruction::GetGlobal(self.globals.clone_ref(*global_idx as usize)),
			other_instruction => Instruction::Plain(other_instruction.clone()),
		}).collect()
	}

	fn generate_instructions(&self, instructions: &[Instruction]) -> Vec<elements::Instruction> {
		use parity_wasm::elements::Instruction::*;
		instructions.iter().map(|instruction| match instruction {
			Instruction::Call(func_ref) => Call(func_ref.order().expect("detached instruction!") as u32),
			Instruction::CallIndirect(type_ref, arg2) => CallIndirect(type_ref.order().expect("detached instruction!") as u32, *arg2),
			Instruction::SetGlobal(global_ref) => SetGlobal(global_ref.order().expect("detached instruction!") as u32),
			Instruction::GetGlobal(global_ref) => GetGlobal(global_ref.order().expect("detached instruction!") as u32),
			Instruction::Plain(plain) => plain.clone(),
		}).collect()
	}

	pub fn from_elements(module: &elements::Module) -> Self {

		let mut idx = 0;
		let mut res = Module::default();

		let mut imported_functions = 0;

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
								imported_functions += 1;
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
							origin: ImportedOrDeclared::Declared(FuncBody {
								locals: Vec::new(),
								// code will be populated later
								code: Vec::new(),
							}),
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
						let init_code = res.map_instructions(g.init_expr().code());
						res.globals.push(Global {
							content: g.global_type().content_type(),
							is_mut: g.global_type().is_mutable(),
							origin: ImportedOrDeclared::Declared(init_code),
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
						// 	SegmentLocation::Default(Vec::new())
						// } else {
						// 	SegmentLocation::WithIndex(element_segment.index(), Vec::new())
						// };

						// TODO: update parity-wasm and uncomment the above instead
						let location = SegmentLocation::Default(
							res.map_instructions(element_segment.offset().code())
						);

						res.elements.push(ElementSegment {
							value: element_segment.members().to_vec(),
							location: location,
						});
					}
				},
				elements::Section::Code(code_section) => {
					let mut idx = 0;
					for func_body in code_section.bodies() {
						let code = res.map_instructions(func_body.code().elements());

						let mut func = res.funcs.get_ref(imported_functions + idx).write();
						match func.origin {
							ImportedOrDeclared::Declared(ref mut body) => {
								body.code = code;
								body.locals = func_body.locals().iter().cloned().collect();
							},
							_ => unreachable!("All declared functions added after imported; qed"),
						}
					}
				},
				elements::Section::Data(data_section) => {
					for data_segment in data_section.entries() {
						// TODO: update parity-wasm and use the same logic as in
						// commented element segment branch
						let location = SegmentLocation::Default(
							res.map_instructions(data_segment.offset().code())
						);

						res.data.push(DataSegment {
							value: data_segment.value().to_vec(),
							location: location,
						});
					}
				},
				_ => {
					res.other.insert(idx, section.clone());
				}
			}
			idx += 1;
		}

		res
	}

	fn generate(&self) -> elements::Module {
		use self::ImportedOrDeclared::*;

		let mut idx = 0;
		let mut sections = Vec::new();

		custom_round(&self.other, &mut idx, &mut sections);

		if self.types.len() > 0 {
			// TYPE SECTION (1)
			let mut type_section = elements::TypeSection::default();
			{
				let types = type_section.types_mut();

				for type_entry in self.types.iter() {
					types.push(type_entry.read().clone())
				}
			}
			sections.push(elements::Section::Type(type_section));
			idx += 1;

			custom_round(&self.other, &mut idx, &mut sections);
		}

		// IMPORT SECTION (2)
		let mut import_section = elements::ImportSection::default();

		let add = {
			let imports = import_section.entries_mut();
			for func in self.funcs.iter() {
				match func.read().origin {
					Imported(ref module, ref field) => {
						imports.push(
							elements::ImportEntry::new(
								module.to_owned(),
								field.to_owned(),
								elements::External::Function(
									func.read().type_ref.order()
										.expect("detached func encountered somehow!") as u32
								),
							)
						)
					},
					_ => continue,
				}
			}

			for global in self.globals.iter() {
				match global.read().origin {
					Imported(ref module, ref field) => {
						imports.push(
							elements::ImportEntry::new(
								module.to_owned(),
								field.to_owned(),
								elements::External::Global(
									elements::GlobalType::new(
										global.read().content,
										global.read().is_mut,
									)
								),
							)
						)
					},
					_ => continue,
				}
			}

			for memory in self.memory.iter() {
				match memory.read().origin {
					Imported(ref module, ref field) => {
						imports.push(
							elements::ImportEntry::new(
								module.to_owned(),
								field.to_owned(),
								elements::External::Memory(
									elements::MemoryType::new(
										memory.read().limits.initial(),
										memory.read().limits.maximum(),
									)
								),
							)
						)
					},
					_ => continue,
				}
			}

			for table in self.tables.iter() {
				match table.read().origin {
					Imported(ref module, ref field) => {
						imports.push(
							elements::ImportEntry::new(
								module.to_owned(),
								field.to_owned(),
								elements::External::Table(
									elements::TableType::new(
										table.read().limits.initial(),
										table.read().limits.maximum(),
									)
								),
							)
						)
					},
					_ => continue,
				}
			}
			imports.len() > 0
		};

		if add {
			sections.push(elements::Section::Import(import_section));
			idx += 1;
			custom_round(&self.other, &mut idx, &mut sections);
		}

		if self.funcs.len() > 0 {
			// FUNC SECTION (3)
			let mut func_section = elements::FunctionSection::default();
			{
				let funcs = func_section.entries_mut();

				for func in self.funcs.iter() {
					match func.read().origin {
						Declared(_) => {
							funcs.push(elements::Func::new(
								func.read().type_ref.order()
									.expect("detached func encountered somehow!") as u32
							));
						},
						_ => continue,
					}
				}
			}
			sections.push(elements::Section::Function(func_section));
			idx += 1;

			custom_round(&self.other, &mut idx, &mut sections);
		}

		if self.tables.len() > 0 {
			// TABLE SECTION (4)
			let mut table_section = elements::TableSection::default();
			{
				let tables = table_section.entries_mut();

				for table in self.tables.iter() {
					match table.read().origin {
						Declared(_) => {
							tables.push(elements::TableType::new(
								table.read().limits.initial(),
								table.read().limits.maximum(),
							));
						},
						_ => continue,
					}
				}
			}
			sections.push(elements::Section::Table(table_section));
			idx += 1;

			custom_round(&self.other, &mut idx, &mut sections);
		}

		if self.memory.len() > 0 {
			// MEMORY SECTION (5)
			let mut memory_section = elements::MemorySection::default();
			{
				let memories = memory_section.entries_mut();

				for memory in self.memory.iter() {
					match memory.read().origin {
						Declared(_) => {
							memories.push(elements::MemoryType::new(
								memory.read().limits.initial(),
								memory.read().limits.maximum(),
							));
						},
						_ => continue,
					}
				}
			}
			sections.push(elements::Section::Memory(memory_section));
			idx += 1;

			custom_round(&self.other, &mut idx, &mut sections);
		}

		if self.globals.len() > 0 {
			// GLOBAL SECTION (6)
			let mut global_section = elements::GlobalSection::default();
			{
				let globals = global_section.entries_mut();

				for global in self.globals.iter() {
					match global.read().origin {
						Declared(ref init_code) => {
							globals.push(elements::GlobalEntry::new(
								elements::GlobalType::new(global.read().content, global.read().is_mut),
								elements::InitExpr::new(self.generate_instructions(&init_code[..])),
							));
						},
						_ => continue,
					}
				}
			}
			sections.push(elements::Section::Global(global_section));
			idx += 1;

			custom_round(&self.other, &mut idx, &mut sections);
		}

		if self.exports.len() > 0 {
			// EXPORT SECTION (7)
			let mut export_section = elements::ExportSection::default();
			{
				let exports = export_section.entries_mut();

				for export in self.exports.iter() {
					let internal = match export.local {
						ExportLocal::Func(ref func_ref) => {
							elements::Internal::Function(func_ref.order().expect("detached func ref") as u32)
						},
						ExportLocal::Global(ref global_ref) => {
							elements::Internal::Global(global_ref.order().expect("detached global ref") as u32)
						},
						ExportLocal::Table(ref table_ref) => {
							elements::Internal::Table(table_ref.order().expect("detached table ref") as u32)
						},
						ExportLocal::Memory(ref memory_ref) => {
							elements::Internal::Memory(memory_ref.order().expect("detached memory ref") as u32)
						},
					};

					exports.push(elements::ExportEntry::new(export.name.to_owned(), internal));
				}
			}
			sections.push(elements::Section::Export(export_section));
			idx += 1;

			custom_round(&self.other, &mut idx, &mut sections);
		}

		if let Some(ref func_ref) = self.start {
			// START SECTION (8)
			sections.push(elements::Section::Start(
				func_ref.order().expect("detached start func") as u32
			));
		}

		if self.elements.len() > 0 {
			// START SECTION (9)
			let mut element_section = elements::ElementSection::default();
			{
				let element_segments = element_section.entries_mut();

				for element in self.elements.iter() {
					match element.location {
						SegmentLocation::Default(ref offset_expr) => {
							element_segments.push(
								elements::ElementSegment::new(
									0,
									elements::InitExpr::new(self.generate_instructions(&offset_expr[..])),
									element.value.clone(),
								)
							);
						},
						_ => unreachable!("Other segment location types are never added"),
					}
				}
			}

			sections.push(elements::Section::Element(element_section));
			idx += 1;

			custom_round(&self.other, &mut idx, &mut sections);
		}

		if self.funcs.len() > 0 {
			// CODE SECTION (10)
			let mut code_section = elements::CodeSection::default();
			{
				let funcs = code_section.bodies_mut();

				for func in self.funcs.iter() {
					match func.read().origin {
						Declared(ref body) => {
							funcs.push(elements::FuncBody::new(
								body.locals.clone(),
								elements::Instructions::new(self.generate_instructions(&body.code[..])),
							));
						},
						_ => continue,
					}
				}
			}
			sections.push(elements::Section::Code(code_section));
			idx += 1;

			custom_round(&self.other, &mut idx, &mut sections);
		}


		if self.data.len() > 0 {
			// DATA SECTION (11)
			let mut data_section = elements::DataSection::default();
			{
				let data_segments = data_section.entries_mut();

				for data_entry in self.data.iter() {
					match data_entry.location {
						SegmentLocation::Default(ref offset_expr) => {
							data_segments.push(
								elements::DataSegment::new(
									0,
									elements::InitExpr::new(self.generate_instructions(&offset_expr[..])),
									data_entry.value.clone(),
								)
							);
						},
						_ => unreachable!("Other segment location types are never added"),
					}
				}
			}

			sections.push(elements::Section::Data(data_section));
			idx += 1;

			custom_round(&self.other, &mut idx, &mut sections);
		}

		elements::Module::new(sections)
	}
}

fn custom_round(
	map: &BTreeMap<usize, elements::Section>,
	idx: &mut usize,
	sections: &mut Vec<elements::Section>,
) {
	while let Some(other_section) = map.get(&idx) {
		sections.push(other_section.clone());
		*idx += 1;
	}
}

pub fn parse(wasm: &[u8]) -> Module {
	Module::from_elements(&::parity_wasm::deserialize_buffer(wasm).expect("failed to parse wasm"))
}

pub fn generate(f: &Module) -> Vec<u8> {
	let pm = f.generate();
	::parity_wasm::serialize(pm).expect("failed to generate wasm")
}

#[cfg(test)]
mod tests {

	extern crate wabt;

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

	#[test]
	#[ignore]
	fn simple_round_trip() {
		let wat = r#"
			(module
				(type (func))
				(import "env" "f1" (func (type 0)))
				(memory 0 1)
				(export "simple" (func 0))
			)
		"#;
		let wasm = wabt::wat2wasm(wat).expect("Failed to read fixture");

		let f = super::parse(&wasm[..]);
		let wasm_new = super::generate(&f);

		let wat_new = wabt::wasm2wat(&wasm_new).expect("Failed to generate expectation");

		if &wasm_new[..] != &wasm[..] {
			panic!(
				"{}\n != \n{}", wat, wat_new
			);
		}
	}

}