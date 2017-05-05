extern crate parity_wasm;
extern crate wasm_utils;

use std::env;

fn main() {

    wasm_utils::init_log();

    let args = env::args().collect::<Vec<_>>();
    if args.len() < 3 {
        println!("Usage: {} input_file.wasm output_file.wasm", args[0]);
        return;
    }

    let mut module = parity_wasm::deserialize_file(&args[1]).unwrap();

    // Invoke optimizer
    //   Contract is supposed to have only these functions as public api
    //   All other symbols not usable by this list is optimized away
    wasm_utils::optimize(&mut module, vec!["_call"]);    

    parity_wasm::serialize_to_file(&args[2], module).unwrap();    
}
