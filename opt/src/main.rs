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
            &Call(idx) => {
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

pub fn update_call_index(opcodes: &mut elements::Opcodes, eliminated_indices: &[usize]) {
    use parity_wasm::elements::Opcode::*;
    for opcode in opcodes.elements_mut().iter_mut() {
        match opcode {
            &mut Block(_, ref mut block) | &mut If(_, ref mut block) | &mut Loop(_, ref mut block) => {
                update_call_index(block, eliminated_indices)
            },
            &mut Call(ref mut call_index) => {
                let totalle = eliminated_indices.iter().take_while(|i| (**i as u32) < *call_index).count();
                println!("rewired call {} -> call {}", *call_index, *call_index - totalle as u32);
                *call_index -= totalle as u32;
            },
            _ => { },
        }
    }
}

/// Updates global references considering the _ordered_ list of eliminated indices
pub fn update_global_index(opcodes: &mut Vec<elements::Opcode>, eliminated_indices: &[usize]) {
    use parity_wasm::elements::Opcode::*;
    for opcode in opcodes.iter_mut() {
        match opcode {
            &mut Block(_, ref mut block) | &mut If(_, ref mut block) | &mut Loop(_, ref mut block) => {
                update_global_index(block.elements_mut(), eliminated_indices)
            },
            &mut GetGlobal(ref mut index) | &mut SetGlobal(ref mut index) => {
                let totalle = eliminated_indices.iter().take_while(|i| (**i as u32) < *index).count();
                println!("rewired global {} -> global {}", *index, *index - totalle as u32);
                *index -= totalle as u32;
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

pub fn export_section<'a>(module: &'a mut elements::Module) -> Option<&'a mut elements::ExportSection> {
   for section in module.sections_mut() {
        match section {
            &mut elements::Section::Export(ref mut sect) => {
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

    // All symbols used in data/element segments are also should be preserved
    let mut init_symbols = Vec::new();
    if let Some(data_section) = module.data_section() {
        for segment in data_section.entries() {
            push_code_symbols(&module, segment.offset().code(), &mut init_symbols);
        }
    }
    if let Some(elements_section) = module.elements_section() {
        for segment in elements_section.entries() {
            push_code_symbols(&module, segment.offset().code(), &mut init_symbols);
            for func_index in segment.members() {
                stay.insert(resolve_function(&module, *func_index));
            }
        }
    }
    for symbol in init_symbols.drain(..) { stay.insert(symbol); }

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
            code_section(&mut module).expect("Code section to exist").bodies_mut().remove(index);

            eliminated_funcs.push(top_funcs + old_index);
            println!("Eliminated function({})", top_funcs + old_index);
        }
        old_index += 1;
    }

    // Forth, eliminate unused exports
    {
        let exports = export_section(&mut module).expect("Export section to exist");

        index = 0;
        old_index = 0;

        loop {
            if exports.entries_mut().len() == index { break; }
            if stay.contains(&Symbol::Export(old_index)) {
                index += 1;
            } else {
                println!("Eliminated export({}, {})", old_index, exports.entries_mut()[index].field());
                exports.entries_mut().remove(index);
            }
            old_index += 1;
        }
    }

    if eliminated_globals.len() > 0 || eliminated_funcs.len() > 0 {
        // Finaly, rewire all calls and globals references to the new indices
        //   (only if there is anything to do)
        eliminated_globals.sort();
        eliminated_funcs.sort();

        for section in module.sections_mut() {
            match section {
                &mut elements::Section::Code(ref mut code_section) => {
                    for ref mut func_body in code_section.bodies_mut() {
                        update_call_index(func_body.code_mut(), &eliminated_funcs);
                        update_global_index(func_body.code_mut().elements_mut(), &eliminated_globals)
                    }
                },
                &mut elements::Section::Export(ref mut export_section) => {
                    for ref mut export in export_section.entries_mut() {
                        match export.internal_mut() {
                            &mut elements::Internal::Function(ref mut func_index) => {
                                let totalle = eliminated_funcs.iter().take_while(|i| (**i as u32) < *func_index).count();
                                *func_index -= totalle as u32;
                            },
                            &mut elements::Internal::Global(ref mut global_index) => {
                                let totalle = eliminated_globals.iter().take_while(|i| (**i as u32) < *global_index).count();
                                *global_index -= totalle as u32;
                            },
                            _ => {}
                        } 
                    }
                },
                &mut elements::Section::Global(ref mut global_section) => {
                    for ref mut global_entry in global_section.entries_mut() {
                        update_global_index(global_entry.init_expr_mut().code_mut(), &eliminated_globals)
                    }
                },
                &mut elements::Section::Data(ref mut data_section) => {
                    for ref mut segment in data_section.entries_mut() {
                        update_global_index(segment.offset_mut().code_mut(), &eliminated_globals)
                    }
                },
                &mut elements::Section::Element(ref mut elements_section) => {
                    for ref mut segment in elements_section.entries_mut() {
                        update_global_index(segment.offset_mut().code_mut(), &eliminated_globals);
                        // update all indirect call addresses initial values
                        for func_index in segment.members_mut() {
                            let totalle = eliminated_funcs.iter().take_while(|i| (**i as u32) < *func_index).count();     
                            *func_index -= totalle as u32;
                        }
                    }
                },
                _ => { }
            }
        }
    }

    parity_wasm::serialize_to_file(&args[2], module).unwrap();    
}
