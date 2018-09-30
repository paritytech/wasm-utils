use std;
use super::{
	optimize,
	pack_instance,
	ununderscore_funcs,
	externalize_mem,
	shrink_unknown_stack,
	inject_runtime_type,
	PackingError,
	OptimizerError,
	TargetRuntime,
};
use parity_wasm;
use parity_wasm::elements;

#[derive(Debug)]
pub enum Error {
	Encoding(elements::Error),
	Packing(PackingError),
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
		}
	}
}

fn has_ctor(module: &elements::Module, target_runtime: &TargetRuntime) -> bool {
	if let Some(ref section) = module.export_section() {
		section.entries().iter().any(|e| target_runtime.create_symbol == e.field())
	} else {
		false
	}
}

pub fn build(
	mut module: elements::Module,
	source_target: SourceTarget,
	runtime_type_version: Option<([u8; 4], u32)>,
	public_api_entries: &[&str],
	enforce_stack_adjustment: bool,
	stack_size: u32,
	skip_optimization: bool,
	target_runtime: &TargetRuntime,
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
	public_api_entries.push(target_runtime.call_symbol);
	if !skip_optimization {
		optimize(
			&mut module,
			public_api_entries,
		)?;
	}

	if has_ctor(&ctor_module, target_runtime) {
		if !skip_optimization {
			optimize(&mut ctor_module, vec![target_runtime.create_symbol])?;
		}
		let ctor_module = pack_instance(
			parity_wasm::serialize(module.clone()).map_err(Error::Encoding)?,
			ctor_module.clone(),
			target_runtime,
		)?;
		Ok((module, Some(ctor_module)))
	} else {
		Ok((module, None))
	}
}
