//! Experimental build tool for cargo

extern crate glob;
extern crate pwasm_utils as utils;
extern crate clap;
extern crate parity_wasm;
extern crate pwasm_utils_cli as logger;

mod source;

use std::{fs, io};
use std::path::PathBuf;

use clap::{App, Arg};
use parity_wasm::elements;
use utils::{build, BuildError, SourceTarget};

#[derive(Debug)]
pub enum Error {
	Io(io::Error),
	FailedToCopy(String),
	Decoding(elements::Error, String),
	Encoding(elements::Error),
	Build(BuildError),
}

impl std::fmt::Display for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
		use self::Error::*;
		match *self {
			Io(ref io) => write!(f, "Generic i/o error: {}", io),
			FailedToCopy(ref msg) => write!(f, "{}. Have you tried to run \"cargo build\"?", msg),
			Decoding(ref err, ref file) => write!(f, "Decoding error ({}). Must be a valid wasm file {}. Pointed wrong file?", err, file),
			Encoding(ref err) => write!(f, "Encoding error ({}). Almost impossible to happen, no free disk space?", err),
			Build(ref err) => write!(f, "Build error: {}", err)
		}
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
			SourceTarget::Emscripten => source::EMSCRIPTEN_TRIPLET,
			SourceTarget::Unknown => source::UNKNOWN_TRIPLET,
		}
	);
	cargo_path.push("release");
	cargo_path.push(format!("{}.wasm", wasm_name));

	let mut target_path = PathBuf::from(input.target_dir());
	target_path.push(format!("{}.wasm", input.final_name()));
	fs::copy(cargo_path.as_path(), target_path.as_path())
		.map_err(|io| Error::FailedToCopy(
			format!("Failed to copy '{}' to '{}': {}", cargo_path.display(), target_path.display(), io)
		))?;

	Ok(())
}

fn do_main() -> Result<(), Error> {
	logger::init_log();

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
		.arg(Arg::with_name("enforce_stack_adjustment")
			.help("Enforce stack size adjustment (used for old wasm32-unknown-unknown)")
			.long("enforce-stack-adjustment"))
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
		.arg(Arg::with_name("shrink_stack")
			.help("Shrinks the new stack size for wasm32-unknown-unknown")
			.takes_value(true)
			.long("shrink-stack"))
		.arg(Arg::with_name("public_api")
			.help("Preserves specific imports in the library")
			.takes_value(true)
			.long("public-api"))

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
		eprintln!("--target can be: '{}' or '{}'", source::EMSCRIPTEN_TRIPLET, source::UNKNOWN_TRIPLET);
		::std::process::exit(1);
	}

	if let Some(final_name) = matches.value_of("final_name") {
		source_input = source_input.with_final(final_name);
	}

	process_output(&source_input)?;

	let path = wasm_path(&source_input);

	let module = parity_wasm::deserialize_file(&path)
		.map_err(|e| Error::Decoding(e, path.to_string()))?;

	let runtime_type_version = if let (Some(runtime_type), Some(runtime_version))
		 = (matches.value_of("runtime_type"), matches.value_of("runtime_version")) {
		let mut ty: [u8; 4] = Default::default();
		let runtime_bytes = runtime_type.as_bytes();
		if runtime_bytes.len() != 4 {
			panic!("--runtime-type should be equal to 4 bytes");
		}
		ty.copy_from_slice(runtime_bytes);
		let version: u32 = runtime_version.parse()
			.expect("--runtime-version should be a positive integer");
		Some((ty, version))
	} else {
		None
	};

	let public_api_entries = matches.value_of("public_api")
		.map(|val| val.split(",").collect())
		.unwrap_or(Vec::new());

	let (module, ctor_module) = build(
		module,
		true,
		source_input.target(),
		runtime_type_version,
		&public_api_entries,
		matches.is_present("enforce_stack_adjustment"),
		matches.value_of("shrink_stack").unwrap_or_else(|| "49152").parse()
			.expect("New stack size is not valid u32"),
		matches.is_present("skip_optimization"),
	).map_err(Error::Build)?;

	if let Some(save_raw_path) = matches.value_of("save_raw") {
		parity_wasm::serialize_to_file(save_raw_path, module.clone()).map_err(Error::Encoding)?;
	}

	parity_wasm::serialize_to_file(
		&path,
		ctor_module.expect("ctor_module can't be None, because 'constructor' argument is set to true in build"),
	).map_err(Error::Encoding)?;
	Ok(())
}

fn main() {
	if let Err(e) = do_main() {
		eprintln!("{}", e);
		std::process::exit(1)
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
