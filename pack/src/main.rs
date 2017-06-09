extern crate parity_wasm;
extern crate wasm_utils;
extern crate clap;

use clap::{App, Arg};

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
                        .get_matches();

    let input = matches.value_of("input").expect("is required; qed");
    let output = matches.value_of("output").expect("is required; qed");

    // doing serialization roundtrip to make sure the input is a valid wasm module
    let module = parity_wasm::deserialize_file(&input).expect("Failed to load wasm module from file");
    let bytes = parity_wasm::serialize(module).expect("Failed to serialize wasm module");

    // Wrap contract code into the wasm module that returns it
    let packed_module = wasm_utils::pack_instance(bytes);

    parity_wasm::serialize_to_file(&output, packed_module).unwrap();
}
