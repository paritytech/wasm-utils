use parity_wasm::elements;
use std::collections::HashSet;

#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub enum Symbol {
    Type(usize),
    Import(usize),
    Global(usize),
    Function(usize),
    Export(usize),
}

pub fn resolve_function(module: &elements::Module, index: u32) -> Symbol {
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

pub fn resolve_global(module: &elements::Module, index: u32) -> Symbol {
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

pub fn push_code_symbols(module: &elements::Module, opcodes: &[elements::Opcode], dest: &mut Vec<Symbol>) {
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

pub fn expand_symbols(module: &elements::Module, set: &mut HashSet<Symbol>) {
    use self::Symbol::*;

    // symbols that were already processed
    let mut stop: HashSet<Symbol> = HashSet::new();
    let mut fringe = set.iter().cloned().collect::<Vec<Symbol>>();
    loop {
        let next = match fringe.pop() {
            Some(s) if stop.contains(&s) => { continue; } 
            Some(s) => s,
            _ => { break; }
        };
        trace!("Processing symbol {:?}", next);

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
            Import(idx) => {
                let entry = &module.import_section().expect("Import section to exist").entries()[idx];
                match entry.external() {
                    &elements::External::Function(type_idx) => {
                        let type_symbol = Symbol::Type(type_idx as usize);
                        if !stop.contains(&type_symbol) {
                            fringe.push(type_symbol);
                        }
                        set.insert(type_symbol);        
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

                let signature = &module.functions_section().expect("Functions section to exist").entries()[idx];
                let type_symbol = Symbol::Type(signature.type_ref() as usize);
                if !stop.contains(&type_symbol) {
                    fringe.push(type_symbol);
                }
                set.insert(type_symbol);
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