extern crate pwasm_utils as utils;

use std::env;

fn main() {
	let args = env::args().collect::<Vec<_>>();
	if args.len() != 3 {
		println!("Usage: {} input_file.wasm output_file.wasm", args[0]);
		return;
	}

	// Loading module
	let mut module = utils::Module::from_elements(
		&parity_wasm::deserialize_file(&args[1]).expect("Module deserialization to succeed")
	).expect("Failed to parse parity-wasm format");

	let mut delete_types = Vec::new();
	for type_ in module.types.iter() {
		if type_.link_count() == 0 {
			delete_types.push(type_.order().expect("type in list should have index"));
		}
	}
	module.types.delete(&delete_types[..]);

	parity_wasm::serialize_to_file(&args[2],
		module.generate().expect("Failed to generate valid format")
	).expect("Module serialization to succeed")
}
