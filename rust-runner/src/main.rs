/*

    Rust contract demo runner

*/

extern crate parity_wasm;
extern crate wasm_utils;

mod alloc;
mod storage;
mod call_args;

use std::env;
use parity_wasm::interpreter::{self, ModuleInstanceInterface, RuntimeValue};

pub const DEFAULT_MEMORY_INDEX: interpreter::ItemIndex = interpreter::ItemIndex::Internal(0);
pub type WasmMemoryPtr = i32;

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

    // Initialize call descriptor
    let descriptor = call_args::init(
        &*program.module("env").expect("env module to exist"), 
        &mut allocator, 
        &[], 
        &[0u8; 128],
    ).expect("call descriptor initialization to succeed");

    // Invoke _call method of the module
    module_instance.execute_export("_call", vec![descriptor.into()]).expect("_call to execute successfully");

    // ???
}