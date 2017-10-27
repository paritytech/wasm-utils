use parity_wasm::{elements};
use self::elements::{ External, Section, Opcode, DataSegment, InitExpr, Internal };

use super::{CREATE_SYMBOL, CALL_SYMBOL};

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
            // TODO: add data section is there no one
            &mut Section::Data(ref mut data_section) => {
                let (index, offset) = if let Some(ref entry) = data_section.entries().iter().last() {
                    if let Opcode::I32Const(offst) = entry.offset().code()[0] {
                        let len = entry.value().len() as i32;
                        let offst = offst as i32;
                        (entry.index(), offst + (len + 4) - len % 4)
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

#[cfg(test)]
mod test {
    extern crate parity_wasm;
    extern crate byteorder;

    use parity_wasm::builder;
    use parity_wasm::interpreter;
    use parity_wasm::interpreter::RuntimeValue;
    use parity_wasm::ModuleInstanceInterface;
    use super::*;
    use super::super::optimize;
    use super::super::SET_TEMP_RET_SYMBOL;
    use byteorder::{ByteOrder, LittleEndian};

    #[test]
    fn call_returns_code() {
        let mut module = builder::module()
            .import()
                .module("env")
                .field("memory")
                .external()
                .memory(1 as u32, Some(1 as u32))
            .build()
            .data()
                .offset(elements::Opcode::I32Const(16))
                .value(vec![0u8])
            .build()
            .function()
                .signature().param().i32().build()
                .body()
                    .with_opcodes(elements::Opcodes::new(
                        vec![
                            elements::Opcode::End
                        ]
                    ))
                    .build()
            .build()
            .function()
                .signature().param().i32().build()
                .body()
                    .with_opcodes(elements::Opcodes::new(
                        vec![
                            elements::Opcode::End
                        ]
                    ))
                    .build()
            .build()
            .export()
                .field("_call")
                .internal().func(0)
            .build()
            .export()
                .field("_create")
                .internal().func(1)
            .build()
        .build();

        let mut ctor_module = module.clone();
        optimize(&mut module, vec![CALL_SYMBOL, SET_TEMP_RET_SYMBOL]).expect("Optimizer to finish without errors");
        optimize(&mut ctor_module, vec![CREATE_SYMBOL, SET_TEMP_RET_SYMBOL]).expect("Optimizer to finish without errors");

        let raw_module = parity_wasm::serialize(module).unwrap();
        pack_instance(raw_module.clone(), &mut ctor_module);

        let program = parity_wasm::DefaultProgramInstance::new().expect("Program instance to load");
        let env_instance = program.module("env").expect("Wasm program to contain env module");
        let env_memory = env_instance.memory(interpreter::ItemIndex::Internal(0)).expect("Linear memory to exist in wasm runtime");

        let execution_params = interpreter::ExecutionParams::default();
        let module = program.add_module("contract", ctor_module, None).expect("Failed to initialize module");

        let _ = module.execute_export(CALL_SYMBOL, execution_params.add_argument(RuntimeValue::I32(1024)));

        let pointer = LittleEndian::read_u32(&env_memory.get(1024 + 8, 4).unwrap());
        let len = LittleEndian::read_u32(&env_memory.get(1024 + 12, 4).unwrap());

        let result_code = env_memory.get(pointer, len as usize).expect("Failed to get code");

        assert_eq!(raw_module, result_code);

        let result_module: elements::Module = parity_wasm::deserialize_buffer(result_code).expect("Result module is not valid");

        let program = parity_wasm::DefaultProgramInstance::new().expect("Program2 instance to load");
        let module = program.add_module("contract", result_module, None).expect("Failed to initialize module");
        let execution_params = interpreter::ExecutionParams::default();

        let _ = module.execute_export(CALL_SYMBOL, execution_params);
    }
}
