use std::collections::HashSet;
use parity_wasm::elements;

use symbols::{Symbol, expand_symbols, push_code_symbols, resolve_function};

#[derive(Debug)]
pub enum Error {
    /// Since optimizer starts with export entries, export
    ///   section is supposed to exist.
    NoExportSection,
}

pub fn optimize(
    module: &mut elements::Module, // Module to optimize
    used_exports: Vec<&str>,       // List of only exports that will be usable after optimization
) -> Result<(), Error> {
    // WebAssembly exports optimizer
    // Motivation: emscripten compiler backend compiles in many unused exports
    //   which in turn compile in unused imports and leaves unused functions

    // Algo starts from the top, listing all items that should stay
    let mut stay = HashSet::new();
    for (index, entry) in module.export_section().ok_or(Error::NoExportSection)?.entries().iter().enumerate() {
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
    expand_symbols(module, &mut stay);

    for symbol in stay.iter() {
        trace!("symbol to stay: {:?}", symbol);
    }

    // Keep track of referreable symbols to rewire calls/globals
    let mut eliminated_funcs = Vec::new();
    let mut eliminated_globals = Vec::new();
    let mut eliminated_types = Vec::new();

    // First, iterate through types
    let mut index = 0;
    let mut old_index = 0;

    {
        loop {
            if type_section(module).expect("Functons section to exist").types_mut().len() == index { break; }

            if stay.contains(&Symbol::Type(old_index)) {
                index += 1;
            } else {
                type_section(module).expect("Code section to exist").types_mut().remove(index);
                eliminated_types.push(old_index);
                trace!("Eliminated type({})", old_index);
            }
            old_index += 1;
        }
    }

    // Second, iterate through imports
    let mut top_funcs = 0;
    let mut top_globals = 0;

    {
        index = 0;
        old_index = 0;
        let imports = import_section(module).expect("Import section to exist");
        loop {
            let mut remove = false;
            match imports.entries()[index].external() {
                &elements::External::Function(_) => {
                    if stay.contains(&Symbol::Import(old_index)) {
                        index += 1;
                    } else {
                        remove = true;
                        eliminated_funcs.push(top_funcs);
                        trace!("Eliminated import({}) func({}, {})", old_index, top_funcs, imports.entries()[index].field());
                    }
                    top_funcs += 1;
                },
                &elements::External::Global(_) => {
                    if stay.contains(&Symbol::Import(old_index)) {
                        index += 1;
                    } else {
                        remove = true;
                        eliminated_globals.push(top_globals);
                        trace!("Eliminated import({}) global({}, {})", old_index, top_globals, imports.entries()[index].field());                        
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

    // Third, iterate through globals
    {
        let globals = global_section(module).expect("Global section to exist");

        index = 0;
        old_index = 0;

        loop {
            if globals.entries_mut().len() == index { break; }
            if stay.contains(&Symbol::Global(old_index)) {
                index += 1;
            } else {
                globals.entries_mut().remove(index);
                eliminated_globals.push(top_globals + old_index);
                trace!("Eliminated global({})", top_globals + old_index);
            }
            old_index += 1;
        }
    }

    // Forth, delete orphaned functions
    index = 0;
    old_index = 0;

    loop {
        if functions_section(module).expect("Functons section to exist").entries_mut().len() == index { break; }
        if stay.contains(&Symbol::Function(old_index)) {
            index += 1;
        } else {
            functions_section(module).expect("Functons section to exist").entries_mut().remove(index);
            code_section(module).expect("Code section to exist").bodies_mut().remove(index);

            eliminated_funcs.push(top_funcs + old_index);
            trace!("Eliminated function({})", top_funcs + old_index);
        }
        old_index += 1;
    }

    // Fivth, eliminate unused exports
    {
        let exports = export_section(module).expect("Export section to exist");

        index = 0;
        old_index = 0;

        loop {
            if exports.entries_mut().len() == index { break; }
            if stay.contains(&Symbol::Export(old_index)) {
                index += 1;
            } else {
                trace!("Eliminated export({}, {})", old_index, exports.entries_mut()[index].field());
                exports.entries_mut().remove(index);
            }
            old_index += 1;
        }
    }

    if eliminated_globals.len() > 0 || eliminated_funcs.len() > 0 || eliminated_types.len() > 0 {
        // Finaly, rewire all calls, globals references and types to the new indices
        //   (only if there is anything to do)
        eliminated_globals.sort();
        eliminated_funcs.sort();
        eliminated_types.sort();

        for section in module.sections_mut() {
            match section {
                &mut elements::Section::Function(ref mut function_section) => {
                    for ref mut func_signature in function_section.entries_mut() {
                        let totalle = eliminated_types.iter().take_while(|i| (**i as u32) < func_signature.type_ref()).count();
                        *func_signature.type_ref_mut() -= totalle as u32;                        
                    }                    
                },
                &mut elements::Section::Import(ref mut import_section) => {
                    for ref mut import_entry in import_section.entries_mut() {
                        if let &mut elements::External::Function(ref mut type_ref) = import_entry.external_mut() { 
                            let totalle = eliminated_types.iter().take_while(|i| (**i as u32) < *type_ref).count();
                            *type_ref -= totalle as u32;                
                        }        
                    }                     
                },
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

    Ok(())
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
                trace!("rewired call {} -> call {}", *call_index, *call_index - totalle as u32);
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
                trace!("rewired global {} -> global {}", *index, *index - totalle as u32);
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

pub fn type_section<'a>(module: &'a mut elements::Module) -> Option<&'a mut elements::TypeSection> {
   for section in module.sections_mut() {
        match section {
            &mut elements::Section::Type(ref mut sect) => {
                return Some(sect);
            },
            _ => { }
        }
    }
    None
}

#[cfg(test)]
mod tests {

    use parity_wasm::builder;
    use super::*;

    /// @spec
    /// Optimizer presumes that export section exists and contains
    /// all symbols passed as a second parameter. Since empty module
    /// obviously contains no export section, optimizer should return
    /// error on it. 
    #[test]
    fn empty() {
        let mut module = builder::module().build();
        let result = optimize(&mut module, vec!["_call"]);

        assert!(result.is_err());
    }
}