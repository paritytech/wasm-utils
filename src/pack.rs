use parity_wasm::{elements, builder};

pub fn pack_instance(raw_module: Vec<u8>) -> elements::Module {

    let raw_len = raw_module.len();
    let mem_required = (raw_len / (64 * 1024) + 1) as u32;

    let module = builder::module()
        .import()
            .module("env")
            .field("memory")
            .external()
            .memory(mem_required as u32, Some(mem_required as u32))
            .build()
        .data()
            .offset(elements::Opcode::I32Const(0))
            .value(raw_module)
            .build()
        .function()
            .signature().param().i32().build()
            .body().with_opcodes(elements::Opcodes::new(vec![
                elements::Opcode::GetLocal(0),
                elements::Opcode::I32Const(raw_len as i32),
                elements::Opcode::I32Store(0, 12),
                elements::Opcode::End,
            ])).build()
            .build()
        .export()
            .field("_call")
            .internal().func(0)
            .build()
        .build();

    module
}