//! Experimental build tool for cargo

extern crate glob;
extern crate wasm_utils;
extern crate clap;

use std::{env, fs, io};
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

pub fn process_output(bin_name: &str) -> Result<(), Error> {
	let out_dir = env::var("OUT_DIR").map_err(|_| Error::NoEnvVar)?;
	let mut path = PathBuf::from(out_dir.clone());
	let wasm_name = bin_name.to_string().replace("-", "_");
	path.push("..");
	path.push("..");
	path.push("..");
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
		let mut path = PathBuf::from(out_dir.clone());
		path.push(format!("{}.wasm", bin_name));
		fs::copy(file, path)?;
	}

	Ok(())
}

fn main() {
	wasm_utils::init_log();

	let matches = App::new("wasm-opt")
		.arg(Arg::with_name("input")
			.index(1)
			.required(true)
			.help("Input WASM file"))
		.arg(Arg::with_name("output")
			.index(2)
			.required(true)
			.help("Output WASM file"))
		.arg(Arg::with_name("exports")
			.long("exports")
			.short("e")
			.takes_value(true)
			.value_name("functions")
			.help("Comma-separated list of exported functions to keep. Default: _call"))
		.get_matches();



}