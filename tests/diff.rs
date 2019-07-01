extern crate diff;
extern crate pwasm_utils as utils;
extern crate wabt;
extern crate parity_wasm;

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use parity_wasm::elements;

fn slurp<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
	let mut f = fs::File::open(path)?;
	let mut buf = vec![];
	f.read_to_end(&mut buf)?;
	Ok(buf)
}

fn dump<P: AsRef<Path>>(path: P, buf: &[u8]) -> io::Result<()> {
	let mut f = fs::File::create(path)?;
	f.write_all(buf)?;
	Ok(())
}

fn validate_wasm(binary: &[u8]) -> Result<(), wabt::Error> {
	wabt::Module::read_binary(
		&binary,
		&Default::default()
	)?.validate()?;
	Ok(())
}

fn run_diff_test<F: FnOnce(&[u8]) -> Vec<u8>>(test_dir: &str, name: &str, test: F) {
	// FIXME: not going to work on windows?
	let mut fixture_path = PathBuf::from(concat!(
		env!("CARGO_MANIFEST_DIR"),
		"/tests/fixtures/",
	));
	fixture_path.push(test_dir);
	fixture_path.push(name);

	// FIXME: not going to work on windows?
	let mut expected_path = PathBuf::from(concat!(
		env!("CARGO_MANIFEST_DIR"),
		"/tests/expectations/"
	));
	expected_path.push(test_dir);
	expected_path.push(name);

	let fixture_wat = slurp(&fixture_path).expect("Failed to read fixture");
	let fixture_wasm = wabt::wat2wasm(fixture_wat).expect("Failed to read fixture");
	validate_wasm(&fixture_wasm).expect("Fixture is invalid");

	let expected_wat = slurp(&expected_path).unwrap_or_default();
	let expected_wat = String::from_utf8_lossy(&expected_wat);

	let actual_wasm = test(fixture_wasm.as_ref());
	validate_wasm(&actual_wasm).expect("Result module is invalid");

	let actual_wat = wabt::wasm2wat(&actual_wasm).expect("Failed to convert result wasm to wat");

	if actual_wat != expected_wat {
		println!("difference!");
		println!("--- {}", expected_path.display());
		println!("+++ {} test {}", test_dir, name);
		for diff in diff::lines(&expected_wat, &actual_wat) {
			match diff {
				diff::Result::Left(l) => println!("-{}", l),
				diff::Result::Both(l, _) => println!(" {}", l),
				diff::Result::Right(r) => println!("+{}", r),
			}
		}

		dump(&expected_path, actual_wat.as_bytes()).expect("Failed to write to expected");

		panic!();
	}
}

mod stack_height {
	use super::*;

	macro_rules! def_stack_height_test {
		( $name:ident ) => {
			#[test]
			fn $name() {
				run_diff_test("stack-height", concat!(stringify!($name), ".wat"), |input| {
					let module = elements::deserialize_buffer(input).expect("Failed to deserialize");
					let instrumented = utils::stack_height::inject_limiter(module, 1024).expect("Failed to instrument with stack counter");
					elements::serialize(instrumented).expect("Failed to serialize")
				});
			}
		};
	}

	def_stack_height_test!(simple);
	def_stack_height_test!(start);
	def_stack_height_test!(table);
	def_stack_height_test!(global);
	def_stack_height_test!(imports);
}

mod gas {
	use super::*;

	macro_rules! def_gas_test {
		( $name:ident ) => {
			#[test]
			fn $name() {
				run_diff_test("gas", concat!(stringify!($name), ".wat"), |input| {
					let rules = utils::rules::Set::default();

					let module = elements::deserialize_buffer(input).expect("Failed to deserialize");
					let instrumented = utils::inject_gas_counter(module, &rules).expect("Failed to instrument with gas metering");
					elements::serialize(instrumented).expect("Failed to serialize")
				});
			}
		};
	}


	def_gas_test!(ifs);
	def_gas_test!(simple);
	def_gas_test!(start);
	def_gas_test!(call);
	def_gas_test!(branch);
}
