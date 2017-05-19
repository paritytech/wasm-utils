use std::sync::Arc;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use parity_wasm::{interpreter, elements};
use {alloc, gas_counter, storage};

#[derive(Hash, PartialEq, Eq, Debug)]
pub struct StorageKey([u8; 32]);

#[derive(Debug, Default)]
pub struct StorageValue([u8; 32]);

pub struct Runtime {
    gas_counter: u64,
    gas_limit: u64,
    dynamic_top: u32,
    storage: HashMap<storage::StorageKey, storage::StorageValue>,
}

#[derive(Debug)]
struct ErrorAlloc;

impl Runtime {
    pub fn with_params(stack_space: u32, gas_limit: u64) -> Runtime {
        Runtime(Arc::new(RuntimeEnv { 
            gas_counter: 0,
            gas_limit: gas_limit,
            dynamic_top: stack_space,
            storage: HashMap::new(),
        }))
    }

    pub fn storage_write(&mut self, memory: Arc<interpreter::Memory>, context: interpreter::CallerContext) 
        -> Result<Option<interpreter::RuntimeValue>, interpreter::Error>
    {
        let val_ptr = context.value_stack.pop_as::<i32>()?;
        let key_ptr = context.value_stack.pop_as::<i32>()?;

        let key = StorageKey::from_mem(memory.get(key_ptr as u32, 32)?)
            .map_err(|_| interpreter::Error::Trap("Memory access violation".to_owned()))?;
        let val = StorageValue::from_mem(memory.get(val_ptr as u32, 32)?)
            .map_err(|_| interpreter::Error::Trap("Memory access violation".to_owned()))?;

        self.storage.insert(key, val);

        Ok(0.into())
    }

    pub fn storage_write(&mut self, memory: Arc<interpreter::Memory>, context: interpreter::CallerContext) 
        -> Result<Option<interpreter::RuntimeValue>, interpreter::Error>
    {
            // arguments passed are in backward order (since it is stack)
        let val_ptr = context.value_stack.pop_as::<i32>()?;
        let key_ptr = context.value_stack.pop_as::<i32>()?;

        let key = StorageKey::from_mem(memory.get(key_ptr as u32, 32)?)
            .map_err(|_| interpreter::Error::Trap("Memory access violation".to_owned()))?;
        let empty = StorageValue([0u8; 32]);
        let storage = self.0.runtime.env().storage.borrow(); 
        let val = storage.get(&key).unwrap_or(&empty);

        memory.set(val_ptr as u32, val.as_slice());

        println!("read storage {:?} (evaluated as {:?})", key, val);

        Ok(Some(0.into()))
    }

    pub fn malloc(&mut self, _memory: Arc<interpreter::Memory>, context: interpreter::CallerContext) 
        -> Result<Option<interpreter::RuntimeValue>, interpreter::Error>
    {
        let amount = context.value_stack.pop_as::<i32>()? as u32;
        let previous_top = self.dynamic_top;
        self.dynamic_top = previous_top + size;
        Ok(previous_top.into())
    }

    pub fn alloc(&mut self, amount: u32) -> Result<u32, ErrorAlloc> {
        let previous_top = self.dynamic_top;
        self.dynamic_top = previous_top + size;
        Ok(previous_top.into())        
    }

    fn gas(&mut self, _memory: Arc<interpreter::Memory>, context: interpreter::CallerContext) 
        -> Result<Option<interpreter::RuntimeValue>, interpreter::Error> 
    {
        let prev = self.gas_counter;
        let update = context.value_stack.pop_as::<i32>()? as u64;
        if prev + update > self.gas_limit {
            // exceeds gas
            Err(interpreter::Error::Trap(format!("Gas exceeds limits of {}", self.runtime.env().gas_limit)))
        } else {
            self.gas_counter.set(prev + update);
            Ok(None)
        }
    }

    fn user_trap(&mut self, _memory: Arc<interpreter::Memory>, _context: interpreter::CallerContext) 
        -> Result<Option<interpreter::RuntimeValue>, interpreter::Error> 
    {
        Err(interpreter::Error::Trap(self.0.clone()))
    }

    fn user_noop(&mut self, 
        _memory: Arc<interpreter::Memory>, 
        _context: interpreter::CallerContext
    ) -> Result<Option<interpreter::RuntimeValue>, interpreter::Error> {
        Ok(None)
    }    
}
