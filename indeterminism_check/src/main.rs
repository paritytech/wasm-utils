extern crate parity_wasm;
extern crate wasm_utils;

use std::env;


fn main() {

	wasm_utils::init_log();

	let args = env::args().collect::<Vec<_>>();
	if args.len() != 2 {
		println!("Usage: {} input_file.wasm", args[0]);
		return;
	}

	// Loading module
	let module = parity_wasm::deserialize_file(&args[1]).expect("Module deserialization to succeed");

	if wasm_utils::have_indeterminism(module) {
		println!("Non-determinism found");
	} else {
		println!("Non-determinism not found");
	}

}
