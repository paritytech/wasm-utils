use clap::{App, Arg};
use pwasm_utils::{self as utils, logger};

fn main() {
	logger::init();

	let target_runtime = utils::TargetRuntime::pwasm();

	let matches = App::new("wasm-prune")
		.arg(Arg::with_name("input").index(1).required(true).help("Input WASM file"))
		.arg(Arg::with_name("output").index(2).required(true).help("Output WASM file"))
		.arg(
			Arg::with_name("exports")
				.long("exports")
				.short("e")
				.takes_value(true)
				.value_name("functions")
				.help(&format!(
					"Comma-separated list of exported functions to keep. Default: '{}'",
					target_runtime.symbols().call
				)),
		)
		.get_matches();

	let exports = matches
		.value_of("exports")
		.unwrap_or(target_runtime.symbols().call)
		.split(',')
		.collect();

	let input = matches.value_of("input").expect("is required; qed");
	let output = matches.value_of("output").expect("is required; qed");

	let mut module = parity_wasm::deserialize_file(&input).unwrap();

	// Invoke optimizer
	//   Contract is supposed to have only these functions as public api
	//   All other symbols not usable by this list is optimized away
	utils::optimize(&mut module, exports).expect("Optimizer failed");

	parity_wasm::serialize_to_file(&output, module).expect("Serialization failed");
}
