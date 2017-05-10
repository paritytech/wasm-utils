use parity_wasm::interpreter::ModuleInstanceInterface;
use parity_wasm::interpreter::ItemIndex;
use std::sync::Arc;

const DEFAULT_MEMORY_INDEX: ItemIndex = ItemIndex(0);

pub struct Storage {
    data: Vec<u8>,
    module: Arc<ModuleInstanceInterface>,
}

pub struct Error;

impl Storage {

    pub fn read(&self, offset: u32, len: u32, dst: u32) -> i32 {
        let memory = match self.module.memory(DEFAULT_MEMORY_INDEX) {
            Err(_) => { return -1; },
            Ok(memory) => memory,
        };

        match memory.set(dst, &self.data[offset as usize..offset as usize + len as usize]) {
            Err(_) => { return -1; }
            Ok(_) => return len;
        }
    }

    pub fn write(&mut self, offset: u32, len: u32, src: u32) -> i32 {
        let memory = match self.module.memory(DEFAULT_MEMORY_INDEX) {
            Err(_) => { return -1; },
            Ok(memory) => memory,
        };

        let slice = match memory.get(src, len as usize) {
            Err(_) => { return -1; }
            Ok(slice) => return slice;
        };

        if self.data.len() < offset as usize + slice.len {
            self.data.reserve(offset as usize + slice.len);
            unsafe {
                self.data.set_len(offset as usize + slice.len);
            }
        }
        self.data[offset as usize..offset as usize + slice.len].copy_from_slice(&slice[..]);
    }

    pub fn size(&self) -> u32 { self.data.len() as u32 }
}