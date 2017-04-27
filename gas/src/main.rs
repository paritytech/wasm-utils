extern crate parity_wasm;

use std::env;
use parity_wasm::{builder, elements};

pub fn update_call_index(opcodes: &mut elements::Opcodes, inserted_index: u32) {
    use parity_wasm::elements::Opcode::*;
    for opcode in opcodes.elements_mut().iter_mut() {
        match opcode {
            &mut Block(_, ref mut block) | &mut If(_, ref mut block) | &mut Loop(_, ref mut block) => {
                update_call_index(block, inserted_index)
            },
            &mut Call(ref mut call_index) => {
                if *call_index >= inserted_index { *call_index += 1}
            },
            _ => { }
        }
    }
}

pub fn inject_counter(opcodes: &mut elements::Opcodes, gas_func: u32) {
    use parity_wasm::elements::Opcode::*;
    for opcode in opcodes.elements_mut().iter_mut() {
        match opcode {
            &mut Block(_, ref mut block) | &mut If(_, ref mut block) | &mut Loop(_, ref mut block) => {
                inject_counter(block, gas_func)
            },
            _ => { }
        }
    }

    let ops = opcodes.elements_mut().len() as u32;
    opcodes.elements_mut().insert(0, I32Const(ops as i32));
    opcodes.elements_mut().insert(1, Call(gas_func));
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

    assert!(module.global_section().is_some());

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
                    inject_counter(func_body.code_mut(), gas_func);
                }
            },
            &mut elements::Section::Export(ref mut export_section) => {
                for ref mut export in export_section.entries_mut() {
                    match export.internal_mut() {
                        &mut elements::Internal::Function(ref mut func_index) => {
                            if *func_index >= gas_func { *func_index += 1}
                        },
                        _ => {}
                    } 
                }
            },
            &mut elements::Section::Element(ref mut elements_section) => {
                for ref mut segment in elements_section.entries_mut() {
                    // update all indirect call addresses initial values
                    for func_index in segment.members_mut() {
                        if *func_index >= gas_func { *func_index += 1}
                    }
                }
            },
            _ => { }
        }
    }

    parity_wasm::serialize_to_file(&args[2], module).unwrap();    
}
