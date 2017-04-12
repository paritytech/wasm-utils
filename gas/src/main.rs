extern crate parity_wasm;

use std::env;
use parity_wasm::{builder, elements};

pub fn update_call_index(opcodes: &mut elements::Opcodes, inserted_index: u32) {
    use parity_wasm::elements::Opcode::*;
    for opcode in opcodes.elements_mut().iter_mut() {
        match opcode {
            &mut Block(_, ref mut block) | &mut If(_, ref mut block) => {
                update_call_index(block, inserted_index)
            },
            &mut Call(ref mut call_index) | &mut CallIndirect(ref mut call_index, _) => {
                if *call_index >= inserted_index { *call_index += 1}
            },
            _ => { }
        }
    }
}

fn main() {

    let args = env::args().collect::<Vec<_>>();
    if args.len() != 3 {
        println!("Usage: {} input_file.wasm output_file.wasm", args[0]);
        return;
    }

    // Loading module
    let module = parity_wasm::deserialize_file(&args[1]).unwrap();

    // Injecting gas counting external
    let mut mbuilder = builder::from_module(module);
    let import_sig = mbuilder.push_signature(
        builder::signature()
            .param().i32()
            .build_sig()
        );

    let mut gas_func = mbuilder.push_import(
        builder::import()
            .module("env")
            .field("gas")
            .external().func(import_sig)
            .build()
        );

    // back to plain module
    let mut module = mbuilder.build();

    // calculate actual function index of the imported definition
    //    (substract all imports that are NOT functions)

    for import_entry in module.import_section().expect("Builder should have insert the import section").entries() {
        match *import_entry.external() {
            elements::External::Function(_) => {},
            _ => { gas_func -= 1; }
        }
    }

    // Updating calling addresses (all calls to function index >= `gas_func` should be incremented)
    for section in module.sections_mut() {
        match section {
            &mut elements::Section::Code(ref mut code_section) => {
                for ref mut func_body in code_section.bodies_mut() {
                    update_call_index(func_body.code_mut(), gas_func);
                }
            },
            _ => { }
        }
    }

    parity_wasm::serialize_to_file(&args[2], module).unwrap();    
}
