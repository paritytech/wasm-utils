/*

    Rust contract demo runner

*/

extern crate parity_wasm;
extern crate wasm_utils;

mod alloc;
mod storage;

use std::env;
use parity_wasm::interpreter::ModuleInstanceInterface;

fn main() {
    // First, load wasm contract as a module
    wasm_utils::init_log();

    let args = env::args().collect::<Vec<_>>();
    if args.len() != 2 {
        println!("Usage: {} contract.wasm", args[0]);
        return;
    }

    let module = parity_wasm::deserialize_file(&args[1]).expect("Module deserialization to succeed");

    // Second, create program instance
    let program = parity_wasm::interpreter::ProgramInstance::new().expect("Program instance to be created");

    // Add module to the programm
    let module_instance = program.add_module("contract", module).expect("Module to be added successfully");

    // Create allocator
    let mut allocator = alloc::Arena::new(5*1024*1024);

    // Invoke _call method of the module
    module_instance.execute_export("_call", vec![]).expect("_call to execute successfully");

    // ???
}