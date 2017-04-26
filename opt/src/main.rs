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
    let mut functions = 0;
    for (item_index, item) in module.import_section().expect("Functions section to exist").entries().iter().enumerate() {
        match item.external() {
            &elements::External::Function(_) => {
                if functions == index {
                    return Symbol::Import(item_index as usize);
                }
                functions += 1;
            },
            _ => {}
        }
    }

    Symbol::Function(index as usize - functions as usize)
}

fn resolve_global(module: &elements::Module, index: u32) -> Symbol {
    let mut globals = 0;
    for (item_index, item) in module.import_section().expect("Functions section to exist").entries().iter().enumerate() {
        match item.external() {
            &elements::External::Global(_) => {
                if globals == index {
                    return Symbol::Import(item_index as usize);
                }
                globals += 1;
            },
            _ => {}
        }
    }

    Symbol::Global(index as usize - globals as usize)
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

pub fn update_call_index(opcodes: &mut elements::Opcodes, eliminated_index: u32) {
    use parity_wasm::elements::Opcode::*;
    for opcode in opcodes.elements_mut().iter_mut() {
        match opcode {
            &mut Block(_, ref mut block) | &mut If(_, ref mut block) | &mut Loop(_, ref mut block) => {
                update_call_index(block, eliminated_index)
            },
            &mut Call(ref mut call_index) | &mut CallIndirect(ref mut call_index, _) => {
                if *call_index > eliminated_index { *call_index -= 1}
            },
            _ => { },
        }
    }
}

pub fn update_global_index(opcodes: &mut elements::Opcodes, eliminated_index: u32) {
    use parity_wasm::elements::Opcode::*;
    for opcode in opcodes.elements_mut().iter_mut() {
        match opcode {
            &mut Block(_, ref mut block) | &mut If(_, ref mut block) | &mut Loop(_, ref mut block) => {
                update_global_index(block, eliminated_index)
            },
            &mut GetGlobal(ref mut index) | &mut SetGlobal(ref mut index) => {
                if *index > eliminated_index { *index -= 1}
            },
            _ => { },
        }
    }
}

pub fn import_section<'a>(module: &'a mut elements::Module) -> Option<&'a mut elements::ImportSection> {
   for section in module.sections_mut() {
        match section {
            &mut elements::Section::Import(ref mut sect) => {
                return Some(sect);
            },
            _ => { }
        }
    }
    None
}

pub fn global_section<'a>(module: &'a mut elements::Module) -> Option<&'a mut elements::GlobalSection> {
   for section in module.sections_mut() {
        match section {
            &mut elements::Section::Global(ref mut sect) => {
                return Some(sect);
            },
            _ => { }
        }
    }
    None
}

pub fn functions_section<'a>(module: &'a mut elements::Module) -> Option<&'a mut elements::FunctionsSection> {
   for section in module.sections_mut() {
        match section {
            &mut elements::Section::Function(ref mut sect) => {
                return Some(sect);
            },
            _ => { }
        }
    }
    None
}

pub fn code_section<'a>(module: &'a mut elements::Module) -> Option<&'a mut elements::CodeSection> {
   for section in module.sections_mut() {
        match section {
            &mut elements::Section::Code(ref mut sect) => {
                return Some(sect);
            },
            _ => { }
        }
    }
    None
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

    // Keep track of referreable symbols to rewire calls/globals
    let mut eliminated_funcs = Vec::new();
    let mut eliminated_globals = Vec::new();

    // First iterate throgh imports 
    let mut index = 0;
    let mut old_index = 0;
    let mut top_funcs = 0;
    let mut top_globals = 0;

    {
        let imports = import_section(&mut module).expect("Import section to exist");
        loop {
            let mut remove = false;
            match imports.entries()[index].external() {
                &elements::External::Function(_) => {
                    if stay.contains(&Symbol::Import(old_index)) {
                        index += 1;
                    } else {
                        remove = true;
                        eliminated_funcs.push(top_funcs);
                        println!("Eliminated import({}) func({}, {})", old_index, top_funcs, imports.entries()[index].field());
                    }
                    top_funcs += 1;
                },
                &elements::External::Global(_) => {
                    if stay.contains(&Symbol::Import(old_index)) {
                        index += 1;
                    } else {
                        remove = true;
                        eliminated_globals.push(top_globals);
                        println!("Eliminated import({}) global({}, {})", old_index, top_globals, imports.entries()[index].field());                        
                    }
                    top_globals += 1;
                },
                _ => {
                    index += 1;
                }
            }
            if remove {
                imports.entries_mut().remove(index);
            }

            old_index += 1;

            if index == imports.entries().len() { break; }
        }
    }

    // Senond, iterate through globals
    {
        let globals = global_section(&mut module).expect("Global section to exist");

        index = 0;
        old_index = 0;

        loop {
            if globals.entries_mut().len() == index { break; }
            if stay.contains(&Symbol::Global(old_index)) {
                index += 1;
            } else {
                globals.entries_mut().remove(index);
                eliminated_globals.push(top_globals + old_index);
                println!("Eliminated global({})", top_globals + old_index);
            }
            old_index += 1;
        }
    }

    // Third, delete orphaned functions
    index = 0;
    old_index = 0;

    loop {
        if functions_section(&mut module).expect("Functons section to exist").entries_mut().len() == index { break; }
        if stay.contains(&Symbol::Function(old_index)) {
            index += 1;
        } else {
            functions_section(&mut module).expect("Functons section to exist").entries_mut().remove(index);
            code_section(&mut module).expect("Functons section to exist").bodies_mut().remove(index);

            eliminated_funcs.push(top_funcs + old_index);
            println!("Eliminated function({})", top_funcs + old_index);
        }
        old_index += 1;
    }

    // Finally, delete all items one by one, updating reference indices in the process
    //   (todo: initial naive impementation can be optimized to avoid multiple passes)

    parity_wasm::serialize_to_file(&args[2], module).unwrap();    
}
