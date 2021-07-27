use clap::{App, Arg};
use pwasm_utils::{self as utils, logger};

fn main() {
	logger::init();

	let target_runtime = utils::TargetRuntime::pwasm();

	let matches = App::new("wasm-pack")
		.arg(Arg::with_name("input").index(1).required(true).help("Input WASM file"))
		.arg(Arg::with_name("output").index(2).required(true).help("Output WASM file"))
		.get_matches();

	let input = matches.value_of("input").expect("is required; qed");
	let output = matches.value_of("output").expect("is required; qed");

	let module =
		parity_wasm::deserialize_file(&input).expect("Input module deserialization failed");
	let ctor_module = module.clone();
	let raw_module = parity_wasm::serialize(module).expect("Serialization failed");

	// Invoke packer
	let mut result_module =
		utils::pack_instance(raw_module, ctor_module, &utils::TargetRuntime::pwasm())
			.expect("Packing failed");
	// Optimize constructor, since it does not need everything
	utils::optimize(&mut result_module, vec![target_runtime.symbols().call])
		.expect("Optimization failed");

	parity_wasm::serialize_to_file(&output, result_module).expect("Serialization failed");
}
