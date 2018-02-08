use parity_wasm::elements::{
    self, Section, Opcode, DataSegment, InitExpr, Internal, External,
    ImportCountType,
};
use parity_wasm::builder;
use super::{CREATE_SYMBOL, CALL_SYMBOL, RET_SYMBOL};
use super::gas::update_call_index;

/// Pack error.
///
/// Pack has number of assumptions of passed module structure.
/// When they are violated, pack_instance returns one of these.
#[derive(Debug)]
pub enum Error {
    MalformedModule,
    NoTypeSection,
    NoExportSection,
    NoCodeSection,
    InvalidCreateSignature,
    NoCreateSymbol,
    InvalidCreateMember,
    NoRetImported,
    NoImportSection,
}

/// If module has an exported "_create" function we want to pack it into "constructor".
/// `raw_module` is the actual contract code
/// `ctor_module` is the constructor which should return `raw_module`
pub fn pack_instance(raw_module: Vec<u8>, mut ctor_module: elements::Module) -> Result<elements::Module, Error> {

    // Total number of constructor module import functions
    let ctor_import_functions = ctor_module.import_section().map(|x| x.functions()).unwrap_or(0);

    // We need to find an internal ID of function witch is exported as "_create"
    // in order to find it in the Code section of the module
    let mut create_func_id = {
        let found_entry = ctor_module.export_section().ok_or(Error::NoExportSection)?.entries().iter()
            .find(|entry| CREATE_SYMBOL == entry.field()).ok_or(Error::NoCreateSymbol)?;

        let function_index: usize = match found_entry.internal() {
            &Internal::Function(index) => index as usize,
            _ => { return Err(Error::InvalidCreateMember) },
        };

        // Calculates a function index within module's function section
        let function_internal_index = function_index - ctor_import_functions;

        // Constructor should be of signature `func(i32)` (void), fail otherwise
        let type_id = ctor_module.function_section().ok_or(Error::NoCodeSection)?
            .entries().get(function_index - ctor_import_functions).ok_or(Error::MalformedModule)?
            .type_ref();

        let &elements::Type::Function(ref func) = ctor_module.type_section().ok_or(Error::NoTypeSection)?
            .types().get(type_id as usize).ok_or(Error::MalformedModule)?;

        // Deploy should have no arguments and also should return nothing
        if !func.params().is_empty() {
            return Err(Error::InvalidCreateSignature);
        }
        if func.return_type().is_some() {
            return Err(Error::InvalidCreateSignature);
        }

        function_internal_index
    };

    let ret_function_id = {
        let mut id = 0;
        let mut found = false;
        for entry in ctor_module.import_section().ok_or(Error::NoImportSection)?.entries().iter() {
            if let External::Function(_) = *entry.external() {
                if entry.field() == RET_SYMBOL { found = true; break; }
                else { id += 1; }
            }
        }
        if !found {
            let mut mbuilder = builder::from_module(ctor_module);
            let import_sig = mbuilder.push_signature(
                builder::signature()
                    .param().i32().param().i32()
                    .build_sig()
                );

            mbuilder.push_import(
                builder::import()
                    .module("env")
                    .field("ret")
                    .external().func(import_sig)
                    .build()
                );

            ctor_module = mbuilder.build();

            let ret_func = ctor_module.import_count(ImportCountType::Function) as u32 - 1;

            for section in ctor_module.sections_mut() {
                match *section {
                    elements::Section::Code(ref mut code_section) => {
                        for ref mut func_body in code_section.bodies_mut() {
                            update_call_index(func_body.code_mut(), ret_func);
                        }
                    },
                    elements::Section::Export(ref mut export_section) => {
                        for ref mut export in export_section.entries_mut() {
                            match export.internal_mut() {
                                &mut elements::Internal::Function(ref mut func_index) => {
                                    if *func_index >= ret_func { *func_index += 1}
                                },
                                _ => {}
                            }
                        }
                    },
                    elements::Section::Element(ref mut elements_section) => {
                        for ref mut segment in elements_section.entries_mut() {
                            // update all indirect call addresses initial values
                            for func_index in segment.members_mut() {
                                if *func_index >= ret_func { *func_index += 1}
                            }
                        }
                    },
                    _ => { }
                }
            }

            create_func_id += 1;
            ret_func
         }
        else { id }
    };

    // If new function is put in ctor module, it will have this callable index
    let last_function_index = ctor_module.functions_space();

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
                    InitExpr::new(vec![Opcode::I32Const(offset), Opcode::End]),
                    raw_module.clone()
                );
                data_section.entries_mut().push(code_data);
                code_data_address = offset;
            },
            _ => {;}
        }
    }

    let mut new_module = builder::from_module(ctor_module)
        .function()
        .signature().build()
        .body().with_opcodes(elements::Opcodes::new(
            vec![
                Opcode::Call((create_func_id + ctor_import_functions) as u32),
                Opcode::I32Const(code_data_address),
                Opcode::I32Const(raw_module.len() as i32),
                Opcode::Call(ret_function_id as u32),
                Opcode::End,
            ])).build()
            .build()
        .build();

    for section in new_module.sections_mut() {
        match section {
            &mut Section::Export(ref mut export_section) => {
                for entry in export_section.entries_mut().iter_mut() {
                    if CREATE_SYMBOL == entry.field() {
                        // change _create export name into default _call
                        *entry.field_mut() = CALL_SYMBOL.to_owned();
                        *entry.internal_mut() = elements::Internal::Function(last_function_index as u32);
                    }
                }
            },
            _ => { },
        }
    };

    Ok(new_module)
}
