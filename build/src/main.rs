//! Experimental build tool for cargo

extern crate glob;
extern crate wasm_utils;
extern crate clap;
extern crate parity_wasm;

use std::{fs, io};
use std::path::PathBuf;

use clap::{App, Arg};

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

pub fn wasm_path(target_dir: &str, bin_name: &str) -> String {
	let mut path = PathBuf::from(target_dir);
	path.push(format!("{}.wasm", bin_name));
	path.to_string_lossy().to_string()
}

pub fn process_output(target_dir: &str, bin_name: &str) -> Result<(), Error> {
	let mut path = PathBuf::from(target_dir);
	let wasm_name = bin_name.to_string().replace("-", "_");
	path.push("wasm32-unknown-emscripten");
	path.push("release");
	path.push("deps");
	path.push(format!("{}-*.wasm", wasm_name));

	let mut files = glob::glob(path.to_string_lossy().as_ref()).expect("glob err")
		.collect::<Vec<Result<PathBuf, glob::GlobError>>>();

	if files.len() == 0 {
		return Err(Error::NoSuitableFile(path.to_string_lossy().to_string()));
	} else if files.len() > 1 {
		return Err(Error::TooManyFiles(
			files.into_iter().map(|f| f.expect("glob err").to_string_lossy().to_string())
				.fold(String::new(), |mut a, b| { a.push_str(", "); a.push_str(&b); a })
		))
	} else {
		let file = files.drain(..).nth(0).expect("0th element exists").expect("glob err");
		let mut path = PathBuf::from(target_dir);
		path.push(format!("{}.wasm", bin_name));
		fs::copy(file, path)?;
	}

	Ok(())
}

fn main() {
	wasm_utils::init_log();

	let matches = App::new("wasm-opt")
		.arg(Arg::with_name("target")
			.index(1)
			.required(true)
			.help("Cargo target directory"))
		.arg(Arg::with_name("wasm")
			.index(2)
			.required(true)
			.help("Wasm binary name"))
		.get_matches();

    let target_dir = matches.value_of("target").expect("is required; qed");
    let wasm_binary = matches.value_of("wasm").expect("is required; qed");

	process_output(target_dir, wasm_binary).expect("Failed to process cargo target directory");

	let path = wasm_path(target_dir, wasm_binary);

	let mut module = wasm_utils::externalize(
		parity_wasm::deserialize_file(&path).unwrap(),
		vec!["_free", "_malloc"],
	);

	wasm_utils::optimize(&mut module, vec!["_call"]).expect("Optimizer to finish without errors");

    parity_wasm::serialize_to_file(&path, module).unwrap();
}