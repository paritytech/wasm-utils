extern crate parity_wasm;

use std::env;
use std::collections::HashSet;
use parity_wasm::elements;

#[derive(PartialEq, Eq, Hash, Copy, Clone)]
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

fn expand_symbols(module: &elements::Module, set: &mut HashSet<Symbol>) {
    use Symbol::*;

    // symbols that were already processed
    let mut stop: HashSet<Symbol> = HashSet::new();
    let mut fringe = set.iter().cloned().collect::<Vec<Symbol>>();
    loop {
        let next = match fringe.pop() {
            Some(s) => s,
            _ => { break; }
        };

        if stop.contains(&next) {
            continue;
        }

        match next {
            Export(idx) => {
                let entry = &module.export_section().expect("Export section to exist").entries()[idx];
                match entry.internal() {
                    &elements::Internal::Function(func_idx) => { fringe.push(resolve_function(module, func_idx)); },
                    &elements::Internal::Global(global_idx) => { fringe.push(resolve_global(module, global_idx)); },
                    _ => {}
                }
            },
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

    // Finally, delete all items one by one, updating reference indices in the process
    //   (todo: initial naive impementation can be optimized to avoid multiple passes)

    parity_wasm::serialize_to_file(&args[2], module).unwrap();    
}
