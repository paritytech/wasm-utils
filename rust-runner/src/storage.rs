use parity_wasm::interpreter::{self, ItemIndex, ModuleInstanceInterface};
use std::sync::Arc;

use DEFAULT_MEMORY_INDEX;
use runtime::Runtime;

pub struct Storage {
    runtime: Runtime,
}

pub struct Error;

impl Storage {

    pub fn new(runtime: Runtime) -> Self {
        Storage {
            runtime: runtime,
        }
    }

    pub fn read(&self, module: &interpreter::ModuleInstance, offset: u32, len: u32, dst: u32) -> i32 {
        let data = self.runtime.env().storage.borrow();
        
        let memory = match module.memory(DEFAULT_MEMORY_INDEX) {
            Err(_) => { return -1; },
            Ok(memory) => memory,
        };

        match memory.set(dst, &data[offset as usize..offset as usize + len as usize]) {
            Err(_) => { return -1; }
            Ok(_) => { return len as i32; }
        }
    }

    pub fn write(&mut self, module: &interpreter::ModuleInstance, offset: u32, len: u32, src: u32) -> i32 {
        let mut data = self.runtime.env().storage.borrow_mut();

        let memory = match module.memory(DEFAULT_MEMORY_INDEX) {
            Err(_) => { return -1; },
            Ok(memory) => memory,
        };

        let slice = match memory.get(src, len as usize) {
            Err(_) => { return -1; }
            Ok(slice) => slice,
        };

        if data.len() < offset as usize + slice.len() {
            data.reserve(offset as usize + slice.len());
            unsafe {
                data.set_len(offset as usize + slice.len());
            }
        }
        data[offset as usize..offset as usize + slice.len()].copy_from_slice(&slice[..]);

        slice.len() as i32
    }

    pub fn size(&self, _module: &interpreter::ModuleInstance) -> u32 { self.runtime.env().storage.borrow().len() as u32 }

    pub fn writer(self) -> StorageWrite {
        StorageWrite(self)
    }

    pub fn reader(self) -> StorageRead {
        StorageRead(self)
    }

    pub fn sizer(self) -> StorageSize {
        StorageSize(self)
    }
}

pub struct StorageWrite(Storage);

impl interpreter::UserFunctionInterface for StorageWrite {
    fn call(&mut self, 
        module: &interpreter::ModuleInstance, 
        context: interpreter::CallerContext,
    ) -> Result<Option<interpreter::RuntimeValue>, interpreter::Error> {
        let offset = context.value_stack.pop_as::<i32>()?;
        let len = context.value_stack.pop_as::<i32>()?;
        let ptr = context.value_stack.pop_as::<i32>()?;

        Ok(Some(self.0.write(module, offset as u32, len as u32, ptr as u32).into()))
    }    
}

pub struct StorageRead(Storage);

impl interpreter::UserFunctionInterface for StorageRead {
    fn call(&mut self, 
        module: &interpreter::ModuleInstance, 
        context: interpreter::CallerContext,
    ) -> Result<Option<interpreter::RuntimeValue>, interpreter::Error> {
        let offset = context.value_stack.pop_as::<i32>()?;
        let len = context.value_stack.pop_as::<i32>()?;
        let ptr = context.value_stack.pop_as::<i32>()?;

        Ok(Some(self.0.read(module, offset as u32, len as u32, ptr as u32).into()))
    }    
}

pub struct StorageSize(Storage);

impl interpreter::UserFunctionInterface for StorageSize {
    fn call(&mut self, 
        module: &interpreter::ModuleInstance, 
        context: interpreter::CallerContext,
    ) -> Result<Option<interpreter::RuntimeValue>, interpreter::Error> 
    {        
        Ok(Some((self.0.size(module) as i32).into()))
    }    
}