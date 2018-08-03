use std;
use super::{
	CREATE_SYMBOL,
	CALL_SYMBOL,
	optimize,
	pack_instance,
	ununderscore_funcs,
	externalize_mem,
	shrink_unknown_stack,
	inject_runtime_type,
	PackingError,
	OptimizerError,
};
use parity_wasm;
use parity_wasm::elements;

#[derive(Debug)]
pub enum Error {
	Encoding(elements::Error),
	Packing(PackingError),
	NoCreateSymbolFound,
	Optimizer,
}

impl From<OptimizerError> for Error {
	fn from(_err: OptimizerError) -> Self {
		Error::Optimizer
	}
}

impl From<PackingError> for Error {
	fn from(err: PackingError) -> Self {
		Error::Packing(err)
	}
}

#[derive(Debug, Clone, Copy)]
pub enum SourceTarget {
	Emscripten,
	Unknown,
}

impl std::fmt::Display for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
		use self::Error::*;
		match *self {
			Encoding(ref err) => write!(f, "Encoding error ({})", err),
			Optimizer => write!(f, "Optimization error due to missing export section. Pointed wrong file?"),
			Packing(ref e) => write!(f, "Packing failed due to module structure error: {}. Sure used correct libraries for building contracts?", e),
			NoCreateSymbolFound => write!(f, "Packing failed: no \"{}\" symbol found?", CREATE_SYMBOL),
		}
	}
}

fn has_ctor(module: &elements::Module) -> bool {
	if let Some(ref section) = module.export_section() {
		section.entries().iter().any(|e| CREATE_SYMBOL == e.field())
	} else {
		false
	}
}

pub fn build_with_constructor(
	module: elements::Module,
	source_target: SourceTarget,
	runtime_type_version: Option<([u8; 4], u32)>,
	public_api_entries: &[&str],
	enforce_stack_adjustment: bool,
	stack_size: u32,
	skip_optimization: bool,
) -> Result<(elements::Module, elements::Module), Error> {
	let (module, module_ctor) = build(
		module,
		true,
		source_target,
		runtime_type_version,
		public_api_entries,
		enforce_stack_adjustment,
		stack_size,
		skip_optimization,
	)?;

	Ok((
		module,
		module_ctor.expect(
			"ctor_module can't be None, because \
			'constructor' argument is set to true in build")
	))
}

pub fn build_raw(
	module: elements::Module,
	source_target: SourceTarget,
	runtime_type_version: Option<([u8; 4], u32)>,
	public_api_entries: &[&str],
	enforce_stack_adjustment: bool,
	stack_size: u32,
	skip_optimization: bool,
) -> Result<elements::Module, Error> {
	let (module, _) = build(
		module,
		false,
		source_target,
		runtime_type_version,
		public_api_entries,
		enforce_stack_adjustment,
		stack_size,
		skip_optimization,
	)?;

	Ok(module)
}

pub fn build(
	mut module: elements::Module,
	constructor: bool,
	source_target: SourceTarget,
	runtime_type_version: Option<([u8; 4], u32)>,
	public_api_entries: &[&str],
	enforce_stack_adjustment: bool,
	stack_size: u32,
	skip_optimization: bool,
) -> Result<(elements::Module, Option<elements::Module>), Error> {

	if let SourceTarget::Emscripten = source_target {
		module = ununderscore_funcs(module);
	}

	if let SourceTarget::Unknown = source_target {
		// 49152 is 48kb!
		if enforce_stack_adjustment {
			assert!(stack_size <= 1024*1024);
			let (new_module, new_stack_top) = shrink_unknown_stack(module, 1024 * 1024 - stack_size);
			module = new_module;
			let mut stack_top_page = new_stack_top / 65536;
			if new_stack_top % 65536 > 0 { stack_top_page += 1 };
			module = externalize_mem(module, Some(stack_top_page), 16);
		} else {
			module = externalize_mem(module, None, 16);
		}
	}

	if let Some(runtime_type_version) = runtime_type_version {
		let (runtime_type, runtime_version) = runtime_type_version;
		module = inject_runtime_type(module, runtime_type, runtime_version);
	}

	let mut ctor_module = module.clone();

	let mut public_api_entries = public_api_entries.to_vec();
	public_api_entries.push(CALL_SYMBOL);
	if !skip_optimization {
		optimize(
			&mut module,
			public_api_entries,
		)?;
	}

	if constructor {
		if !has_ctor(&ctor_module) {
			Err(Error::NoCreateSymbolFound)?
		}
		if !skip_optimization {
			optimize(&mut ctor_module, vec![CREATE_SYMBOL])?;
		}
		let ctor_module = pack_instance(
			parity_wasm::serialize(module.clone()).map_err(Error::Encoding)?,
			ctor_module.clone(),
		)?;
		Ok((module, Some(ctor_module)))
	} else {
		Ok((module, None))
	}
}
