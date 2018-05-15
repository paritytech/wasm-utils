extern crate pwasm_utils as utils;
extern crate parity_wasm;
extern crate pwasm_utils_cli as logger;

use std::env;
use utils::stack_height;

fn main() {
	logger::init_log();

	let args = env::args().collect::<Vec<_>>();
	if args.len() != 3 {
		println!("Usage: {} input_file.wasm output_file.wasm", args[0]);
		return;
	}

	let input_file = &args[1];
	let output_file = &args[2];

	// Loading module
	let module = parity_wasm::deserialize_file(&input_file).expect("Module deserialization to succeed");

	let result = stack_height::inject_limiter(
		module, 1024
	).expect("Failed to inject stack height counter");

	parity_wasm::serialize_to_file(&output_file, result).expect("Module serialization to succeed")
}
