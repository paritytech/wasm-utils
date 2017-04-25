extern crate parity_wasm;

use std::env;
use std::collections::HashSet;
use parity_wasm::elements;

#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug)]
enum Symbol {
    Import(usize),
    Global(usize),
    Function(usize),
    Export(usize),
}

fn resolve_function(module: &elements::Module, index: u32) -> Symbol {
    let imports_len = module
        .import_section()
        .expect("Functions section to exist")
        .entries()
        .iter()
        .map(|e| match e.external() {
            &elements::External::Function(_) => 1,
            _ => 0,
        })
        .sum();

    if index < imports_len {
        Symbol::Import(index as usize)
    } else {
        Symbol::Function(index as usize - imports_len as usize)
    }
}

fn resolve_global(module: &elements::Module, index: u32) -> Symbol {
    let imports_len = module
        .import_section()
        .expect("Functions section to exist")
        .entries()
        .iter()
        .map(|e| match e.external() {
            &elements::External::Global(_) => 1,
            _ => 0,
        })
        .sum();

    if index < imports_len {
        Symbol::Import(index as usize)
    } else {
        Symbol::Global(index as usize - imports_len as usize)
    }
}

fn push_code_symbols(module: &elements::Module, opcodes: &[elements::Opcode], dest: &mut Vec<Symbol>) {
    use parity_wasm::elements::Opcode::*;

    for opcode in opcodes {
        match opcode {
            &Call(idx) | &CallIndirect(idx, _) => {
                dest.push(resolve_function(module, idx));
            },
            &GetGlobal(idx) | &SetGlobal(idx) => {
                dest.push(resolve_global(module, idx))
            },
            &If(_, ref block) | &Loop(_, ref block) | &Block(_, ref block) => {
                push_code_symbols(module, block.elements(), dest);
            },
            _ => { },
        } 
    }
}

fn expand_symbols(module: &elements::Module, set: &mut HashSet<Symbol>) {
    use Symbol::*;

    // symbols that were already processed
    let mut stop: HashSet<Symbol> = HashSet::new();
    let mut fringe = set.iter().cloned().collect::<Vec<Symbol>>();
    loop {
        let next = match fringe.pop() {
            Some(s) if stop.contains(&s) => { continue; } 
            Some(s) => s,
            _ => { break; }
        };
        println!("Processing symbol {:?}", next);

        match next {
            Export(idx) => {
                let entry = &module.export_section().expect("Export section to exist").entries()[idx];
                match entry.internal() {
                    &elements::Internal::Function(func_idx) => {
                        let symbol = resolve_function(module, func_idx); 
                        if !stop.contains(&symbol) {
                            fringe.push(symbol);
                        }
                        set.insert(symbol);
                    },
                    &elements::Internal::Global(global_idx) => {
                        let symbol = resolve_global(module, global_idx);
                        if !stop.contains(&symbol) {
                            fringe.push(symbol);
                        }
                        set.insert(symbol); 
                    },
                    _ => {}
                }
            },
            Function(idx) => {
                let body = &module.code_section().expect("Code section to exist").bodies()[idx];
                let mut code_symbols = Vec::new();
                push_code_symbols(module, body.code().elements(), &mut code_symbols);
                for symbol in code_symbols.drain(..) {
                    if !stop.contains(&symbol) {
                        fringe.push(symbol);
                    }
                    set.insert(symbol);
                }
            },
            Global(idx) => {
                let entry = &module.global_section().expect("Global section to exist").entries()[idx];
                let mut code_symbols = Vec::new();
                push_code_symbols(module, entry.init_expr().code(), &mut code_symbols);
                for symbol in code_symbols.drain(..) {
                    if !stop.contains(&symbol) {
                        fringe.push(symbol);
                    }
                    set.insert(symbol);
                }                
            }
            _ => {}
        }

        stop.insert(next);
    }
}

fn main() {

    let args = env::args().collect::<Vec<_>>();
    if args.len() != 3 {
        println!("Usage: {} input_file.wasm output_file.wasm", args[0]);
        return;
    }

    // Loading module
    let mut module = parity_wasm::deserialize_file(&args[1]).unwrap();

    // WebAssembly exports optimizer
    // Motivation: emscripten compiler backend compiles in many unused exports
    //   which in turn compile in unused imports and leaves unused functions

    // List of exports that are actually used in the managed code
    let used_exports = vec!["_call", "_malloc", "_free"];

    // Algo starts from the top, listing all items that should stay
    let mut stay = HashSet::new();
    for (index, entry) in module.export_section().expect("Export section to exist").entries().iter().enumerate() {
        if used_exports.iter().find(|e| **e == entry.field()).is_some() {
            stay.insert(Symbol::Export(index));
        } 
    }

    // Call function which will traverse the list recursively, filling stay with all symbols
    // that are already used by those which already there
    expand_symbols(&mut module, &mut stay);

    for symbol in stay.iter() {
        println!("symbol to stay: {:?}", symbol);
    }

    // Finally, delete all items one by one, updating reference indices in the process
    //   (todo: initial naive impementation can be optimized to avoid multiple passes)

    parity_wasm::serialize_to_file(&args[2], module).unwrap();    
}
