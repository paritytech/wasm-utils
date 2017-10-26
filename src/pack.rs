use parity_wasm::{serialize,elements, builder, deserialize_buffer};
use self::elements::{ External, Section, ResizableLimits, Opcode, DataSegment, InitExpr, Internal };

use super::CREATE_SYMBOL;
use super::CALL_SYMBOL;

/// If module has an exported "_create" function we want to pack it into "constructor".
/// `raw_module` is the actual contract code
/// `ctor_module` is the constructor which should return `raw_module`
pub fn pack_instance(raw_module: Vec<u8>, ctor_module: &mut elements::Module) {

    // We need to find an internal ID of function witch is exported as "_create"
    // in order to find it in the Code section of the module
    let create_func_id = {
        let found_entry = ctor_module.export_section().expect("No export section found").entries().iter()
            .find(|entry| CREATE_SYMBOL == entry.field()).expect("No export with name _create found");

        let function_index: usize = match found_entry.internal() {
            &Internal::Function(index) => index as usize,
            _ => panic!("export is not a function"),
        };

        let import_section_len: usize = match ctor_module.import_section() {
            Some(import) =>
                import.entries().iter().filter(|entry| match entry.external() {
                    &External::Function(_) => true,
                    _ => false,
                    }).count(),
            None => 0,
        };

        // Calculates a function index within module's function section
        function_index - import_section_len
    };

    // Code data address is an address where we put the contract's code (raw_module)
    let mut code_data_address = 0i32;
    for section in ctor_module.sections_mut() {
        match section {
            &mut Section::Data(ref mut data_section) => {
                let (index, offset) = if let Some(ref entry) = data_section.entries().iter().last() {
                    if let Opcode::I32Const(offst) = entry.offset().code()[0] {
                        let len = entry.value().len() as i32;
                        let offst = offst as i32;
                        (entry.index(), offst + (len + 32) - len % 32)
                    } else {
                        (0, 0)
                    }
                } else {
                    (0, 0)
                };
                let code_data = DataSegment::new(
                    index,
                    InitExpr::new(vec![Opcode::I32Const(offset),Opcode::End]),
                    raw_module.clone()
                );
                data_section.entries_mut().push(code_data);
                code_data_address = offset;
            },
            _ => {;}
        }
    }

    for section in ctor_module.sections_mut() {
        match section {
            &mut Section::Export(ref mut export_section) => {
                for entry in export_section.entries_mut().iter_mut() {
                    if CREATE_SYMBOL == entry.field() {
                        // change _create export name into default _call
                        *entry.field_mut() = CALL_SYMBOL.to_owned();
                    }
                }
            }

            &mut Section::Code(ref mut code_section) => {
                let code = code_section.bodies_mut()[create_func_id].code_mut().elements_mut();
                code.pop();
                code.extend([
                    Opcode::GetLocal(0),
                    Opcode::I32Const(code_data_address),
                    Opcode::I32Store(0, 8),
                    Opcode::GetLocal(0),
                    Opcode::I32Const(raw_module.len() as i32),
                    Opcode::I32Store(0, 12),
                    Opcode::End].iter().cloned());
            },

            _ => {;},
        }
    };
}
