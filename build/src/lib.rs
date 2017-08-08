//! Tools library for building contracts via cargo

extern crate glob;

use std::{env, fs, io};
use std::path::PathBuf;

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
	let out_dir = env::var("OUT").map_err(|_| Error::NoEnvVar)?;
	let mut path = PathBuf::from(out_dir.clone());
	path.push("deps");
	path.push(format!("{}-*.wasm", bin_name));

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
		fs::copy(file, out_dir)?;
	}

	Ok(())
}