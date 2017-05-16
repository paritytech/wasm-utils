use parity_wasm::interpreter::{self, ItemIndex, ModuleInstanceInterface};
use std::sync::Arc;

use DEFAULT_MEMORY_INDEX;
use runtime::Runtime;

#[derive(Hash, PartialEq, Eq, Debug)]
pub struct StorageKey([u8; 32]);

#[derive(Debug, Default)]
pub struct StorageValue([u8; 32]);

impl StorageKey {
    // todo: deal with memory views
    fn from_mem(vec: Vec<u8>) -> Result<Self, Error> {
        if vec.len() != 32 { return Err(Error); }
        let mut result = StorageKey([0u8; 32]);
        result.0.copy_from_slice(&vec[0..32]);
        Ok(result)
    }
}

impl StorageValue {
    // todo: deal with memory views
    // todo: deal with variable-length values when it comes
    fn from_mem(vec: Vec<u8>) -> Result<Self, Error> {
        if vec.len() != 32 { return Err(Error); }
        let mut result = StorageValue([0u8; 32]);
        result.0.copy_from_slice(&vec[0..32]);
        Ok(result)
    }

    fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

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

    pub fn writer(self) -> StorageWrite {
        StorageWrite(self)
    }

    pub fn reader(self) -> StorageRead {
        StorageRead(self)
    }
}

pub struct StorageWrite(Storage);

impl interpreter::UserFunctionInterface for StorageWrite {
    fn call(&mut self, 
        module: &interpreter::ModuleInstance, 
        context: interpreter::CallerContext,
    ) -> Result<Option<interpreter::RuntimeValue>, interpreter::Error> {

        // arguments passed are in backward order (since it is stack)
        let val_ptr = context.value_stack.pop_as::<i32>()?;
        let key_ptr = context.value_stack.pop_as::<i32>()?;
      
        let memory = match module.memory(DEFAULT_MEMORY_INDEX) {
            Err(_) => { return Ok(Some((-1i32).into())) },
            Ok(memory) => memory,
        };

        let key = StorageKey::from_mem(memory.get(key_ptr as u32, 32)?)
            .map_err(|_| interpreter::Error::Trap("Memory access violation".to_owned()))?;
        let val = StorageValue::from_mem(memory.get(val_ptr as u32, 32)?)
            .map_err(|_| interpreter::Error::Trap("Memory access violation".to_owned()))?;

        println!("set storage {:?} = {:?}", key, val);

        Ok(Some(0.into()))
    }    
}

pub struct StorageRead(Storage);

impl interpreter::UserFunctionInterface for StorageRead {
    fn call(&mut self, 
        module: &interpreter::ModuleInstance, 
        context: interpreter::CallerContext,
    ) -> Result<Option<interpreter::RuntimeValue>, interpreter::Error> {

        // arguments passed are in backward order (since it is stack)
        let val_ptr = context.value_stack.pop_as::<i32>()?;
        let key_ptr = context.value_stack.pop_as::<i32>()?;
      
        let memory = match module.memory(DEFAULT_MEMORY_INDEX) {
            Err(_) => { return Ok(Some((-1i32).into())) },
            Ok(memory) => memory,
        };

        let key = StorageKey::from_mem(memory.get(key_ptr as u32, 32)?)
            .map_err(|_| interpreter::Error::Trap("Memory access violation".to_owned()))?;
        let empty = StorageValue([0u8; 32]);
        let storage = self.0.runtime.env().storage.borrow(); 
        let val = storage.get(&key).unwrap_or(&empty);

        memory.set(val_ptr as u32, val.as_slice());

        println!("read storage {:?} (evaluated as {:?})", key, val);

        Ok(Some(0.into()))
    }    
}