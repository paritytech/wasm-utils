//! Experimental build tool for cargo

extern crate glob;
extern crate wasm_utils;
extern crate clap;
extern crate parity_wasm;

mod source;

use std::{fs, io};
use std::io::Write;
use std::path::PathBuf;

use clap::{App, Arg};
use parity_wasm::elements;

use wasm_utils::{CREATE_SYMBOL, CALL_SYMBOL, MEMORY_SYMBOL, ununderscore_funcs};

#[derive(Debug)]
pub enum Error {
	Io(io::Error),
	NoSuitableFile(String),
	TooManyFiles(String),
	NoEnvVar,
}

impl From<io::Error> for Error {
	fn from(err: io::Error) -> Self {
		Error::Io(err)
	}
}

pub fn wasm_path(input: &source::SourceInput) -> String {
	let mut path = PathBuf::from(input.target_dir());
	path.push(format!("{}.wasm", input.final_name()));
	path.to_string_lossy().to_string()
}

pub fn process_output(input: &source::SourceInput) -> Result<(), Error> {
	let mut cargo_path = PathBuf::from(input.target_dir());
	let wasm_name = input.bin_name().to_string().replace("-", "_");
	cargo_path.push(
		match input.target() {
			source::SourceTarget::Emscripten => source::EMSCRIPTEN_TRIPLET,
			source::SourceTarget::Unknown => source::UNKNOWN_TRIPLET,
		}
	);
	cargo_path.push("release");
	cargo_path.push(format!("{}.wasm", wasm_name));

	let mut target_path = PathBuf::from(input.target_dir());
	target_path.push(format!("{}.wasm", input.final_name()));
	fs::copy(cargo_path, target_path)?;

	Ok(())
}

fn has_ctor(module: &elements::Module) -> bool {
	if let Some(ref section) = module.export_section() {
		section.entries().iter().any(|e| CREATE_SYMBOL == e.field())
	} else {
		false
	}
}

fn main() {
	wasm_utils::init_log();

	let matches = App::new("wasm-build")
		.arg(Arg::with_name("target")
			.index(1)
			.required(true)
			.help("Cargo target directory"))
		.arg(Arg::with_name("wasm")
			.index(2)
			.required(true)
			.help("Wasm binary name"))
		.arg(Arg::with_name("skip_optimization")
			.help("Skip symbol optimization step producing final wasm")
			.long("skip-optimization"))
		.arg(Arg::with_name("runtime_type")
			.help("Injects RUNTIME_TYPE global export")
			.takes_value(true)
			.long("runtime-type"))
		.arg(Arg::with_name("runtime_version")
			.help("Injects RUNTIME_VERSION global export")
			.takes_value(true)
			.long("runtime-version"))
		.arg(Arg::with_name("source_target")
			.help("Cargo target type kind ('wasm32-unknown-unknown' or 'wasm32-unknown-emscripten'")
			.takes_value(true)
			.long("target"))
		.arg(Arg::with_name("final_name")
			.help("Final wasm binary name")
			.takes_value(true)
			.long("final"))
		.arg(Arg::with_name("save_raw")
			.help("Save intermediate raw bytecode to path")
			.takes_value(true)
			.long("save-raw"))
		.get_matches();

    let target_dir = matches.value_of("target").expect("is required; qed");
    let wasm_binary = matches.value_of("wasm").expect("is required; qed");
	let mut source_input = source::SourceInput::new(target_dir, wasm_binary);

	let source_target_val = matches.value_of("source_target").unwrap_or_else(|| source::EMSCRIPTEN_TRIPLET);
	if source_target_val == source::UNKNOWN_TRIPLET {
		source_input = source_input.unknown()
	} else if source_target_val == source::EMSCRIPTEN_TRIPLET {
		source_input = source_input.emscripten()
	} else {
		println!("--target can be: '{}' or '{}'", source::EMSCRIPTEN_TRIPLET, source::UNKNOWN_TRIPLET);
		::std::process::exit(1);
	}

	if let Some(final_name) = matches.value_of("final_name") {
		source_input = source_input.with_final(final_name);
	}

	process_output(&source_input).expect("Failed to process cargo target directory");

	let path = wasm_path(&source_input);

	let mut module = parity_wasm::deserialize_file(&path).unwrap();

	if let source::SourceTarget::Unknown = source_input.target() {
		module = ununderscore_funcs(module);
	}

	if let Some(runtime_type) = matches.value_of("runtime_type") {
		let runtime_type: &[u8] = runtime_type.as_bytes();
		if runtime_type.len() != 4 {
			panic!("--runtime-type should be equal to 4 bytes");
		}
		let runtime_version: u32 = matches.value_of("runtime_version").unwrap_or("1").parse()
			.expect("--runtime-version should be a positive integer");
		module = wasm_utils::inject_runtime_type(module, &runtime_type, runtime_version);
	}

	let mut ctor_module = module.clone();

	if !matches.is_present("skip_optimization") {
		wasm_utils::optimize(
			&mut module,
			vec![CALL_SYMBOL, MEMORY_SYMBOL]
		).expect("Optimizer to finish without errors");
	}

	if let Some(save_raw_path) = matches.value_of("save_raw") {
		parity_wasm::serialize_to_file(save_raw_path, module.clone())
			.expect("Failed to write intermediate module");
	}

	let raw_module = parity_wasm::serialize(module).expect("Failed to serialize module");

	// If module has an exported function with name=CREATE_SYMBOL
	// build will pack the module (raw_module) into this funciton and export as CALL_SYMBOL.
	// Otherwise it will just save an optimised raw_module
	if has_ctor(&ctor_module) {
		if !matches.is_present("skip_optimization") {
			wasm_utils::optimize(&mut ctor_module, vec![CREATE_SYMBOL]).expect("Optimizer to finish without errors");
		}
		let ctor_module = wasm_utils::pack_instance(raw_module, ctor_module).expect("Packing failed");
		parity_wasm::serialize_to_file(&path, ctor_module).expect("Failed to serialize to file");
	} else {
		let mut file = fs::File::create(&path).expect("Failed to create file");
		file.write_all(&raw_module).expect("Failed to write module to file");
	}
}

#[cfg(test)]
mod tests {
	extern crate tempdir;

	use self::tempdir::TempDir;
	use std::fs;

	use super::process_output;
	use super::source::SourceInput;

	#[test]
	fn processes_cargo_output() {
		let tmp_dir = TempDir::new("target").expect("temp dir failed");

		let target_path = tmp_dir.path().join("wasm32-unknown-emscripten").join("release");
		fs::create_dir_all(target_path.clone()).expect("create dir failed");

		{
			use std::io::Write;

			let wasm_path = target_path.join("example_wasm.wasm");
			let mut f = fs::File::create(wasm_path).expect("create fail failed");
			f.write(b"\0asm").expect("write file failed");
		}

		let path = tmp_dir.path().to_string_lossy();
		let input = SourceInput::new(&path, "example-wasm");

		process_output(&input).expect("process output failed");

		assert!(
			fs::metadata(tmp_dir.path().join("example-wasm.wasm")).expect("metadata failed").is_file()
		)
	}
}
