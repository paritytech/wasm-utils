use parity_wasm::{serialize,elements, builder, deserialize_buffer};
use self::elements::{ External, Section, ResizableLimits, Opcode, DataSegment, InitExpr, Internal };

/// TODO: desc
pub fn pack_instance(raw_module: Vec<u8>, ctor_module: &mut elements::Module) {
    let raw_len = raw_module.len();
    let mem_required = (raw_len / (64 * 1024) + 1) as u32;

    // Func
    let create_func_id = {
        let export_section = ctor_module.export_section().expect("No export section found");
        let found_entry = export_section.entries().iter()
            .find(|entry| "_create" == entry.field()).expect("No export with name _create found");

        let function_index: usize = match found_entry.internal() {
            &Internal::Function(index) => index as usize,
            _ => panic!("_create export is not a function"),
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

    let mut code_data_address = 0i32;
    for section in ctor_module.sections_mut() {
        match section {
            &mut Section::Data(ref mut data_section) => {
                let (index, offset) = if let Some(ref entry) = data_section.entries().iter().last() {
                    if let Opcode::I32Const(offst) = entry.offset().code()[0] {
                        let len = entry.value().len() as i32;
                        let offst = offst as i32;
                        (entry.index(), offst + len + len % 32)
                    } else {
                        (0, 0)
                    }
                } else {
                    (0, 0)
                };
                let code_data = DataSegment::new(index, InitExpr::new(vec![Opcode::I32Const(offset),Opcode::End]), raw_module.clone());
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
                    if "_create" == entry.field() {
                        // change _create export name into default _call
                        *entry.field_mut() = "_call".to_owned();
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
                    Opcode::I32Const(raw_len as i32),
                    Opcode::I32Store(0, 12),
                    Opcode::End].iter().cloned());
            },

            _ => {;},
        }
    };
}
