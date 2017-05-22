/*

	Rust contract demo runner

*/

extern crate parity_wasm;
extern crate wasm_utils;

mod call_args;
mod runtime;

use std::env;
use std::sync::Arc;
use parity_wasm::interpreter::{self, ModuleInstanceInterface};
use parity_wasm::elements;

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

	let program = parity_wasm::interpreter::ProgramInstance::new()
		.expect("Program instance to be created");

	// Add module to the programm
	let module_instance = program.add_module("contract", module).expect("Module to be added successfully");

	{
		let env_instance = program.module("env").expect("env module to exist");
		let env_memory = env_instance.memory(interpreter::ItemIndex::Internal(0))
			.expect("liner memory to exist");

		// Second, create runtime and program instance
		let mut runtime = runtime::Runtime::with_params(
			env_memory.clone(),  // memory shared ptr
			5*1024*1024,         // default stack space 
			65536,               // runner arbitrary gas limit
		);

		// Initialize call descriptor
		let descriptor = call_args::init(
			&*env_memory,
			&mut runtime, 
			&[3u8; 128],
		).expect("call descriptor initialization to succeed");                

		// create native env module with native add && sub implementations
		let functions = interpreter::UserFunctions {
			executor: &mut runtime,
			functions: vec![
				interpreter::UserFunction {
					name: "_storage_read".to_owned(),
					params: vec![elements::ValueType::I32, elements::ValueType::I32],
					result: Some(elements::ValueType::I32),
				},
				interpreter::UserFunction {
					name: "_storage_write".to_owned(),
					params: vec![elements::ValueType::I32, elements::ValueType::I32],
					result: Some(elements::ValueType::I32),
				},
				interpreter::UserFunction {
					name: "_malloc".to_owned(),
					params: vec![elements::ValueType::I32],
					result: Some(elements::ValueType::I32),
				},
				interpreter::UserFunction {
					name: "gas".to_owned(),
					params: vec![elements::ValueType::I32],
					result: None,
				},
				interpreter::UserFunction {
					name: "_free".to_owned(),
					params: vec![elements::ValueType::I32],
					result: None,
				},
			],
		};
		let native_env_instance = Arc::new(interpreter::env_native_module(env_instance, functions).unwrap());

		// Form ExecutionParams (payload + env link)
		let params = interpreter::ExecutionParams::with_external("env".into(), native_env_instance)
			.add_argument(interpreter::RuntimeValue::I32(descriptor));

		module_instance.execute_export("_call", params)
			.expect("_call to execute successfully")
			.expect("_call function to return result ptr");        
	}
}