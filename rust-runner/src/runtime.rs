use std::sync::Arc;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use parity_wasm::{interpreter, elements};
use {alloc, gas_counter, storage};

#[derive(Default)]
pub struct RuntimeEnv {
    pub gas_counter: Cell<u64>,
    pub gas_limit: u64,
    pub dynamic_top: Cell<u32>,
    pub storage: RefCell<HashMap<storage::StorageKey, storage::StorageValue>>,
}

#[derive(Default, Clone)]
pub struct Runtime(Arc<RuntimeEnv>);

impl Runtime {
    pub fn with_params(stack_space: u32, gas_limit: u64) -> Runtime {
        Runtime(Arc::new(RuntimeEnv { 
            gas_counter: Cell::new(0),
            gas_limit: gas_limit,
            dynamic_top: Cell::new(stack_space),
            storage: RefCell::new(HashMap::new()),
        }))
    }

    pub fn allocator(&self) -> alloc::Arena {
        alloc::Arena {
            runtime: self.clone(),
        }
    }

    pub fn gas_counter(&self) -> gas_counter::GasCounter {
        gas_counter::GasCounter {
            runtime: self.clone(),
        }
    }

    pub fn storage(&self) -> storage::Storage {
        storage::Storage::new(self.clone())
    }

    pub fn env(&self) -> &RuntimeEnv {
        &*self.0
    }
}

pub fn user_trap(funcs: &mut interpreter::UserFunctions, func_name: &str) {
    let func_str = func_name.to_owned();
    funcs.insert(func_str.clone(), 
        interpreter::UserFunction {
            params: vec![],
            result: Some(elements::ValueType::I32),
            closure: Box::new(UserTrap(func_str)),
        }
    );    
}

struct UserTrap(String);

impl interpreter::UserFunctionInterface for UserTrap {
    fn call(&mut self, 
        _module: &interpreter::ModuleInstance, 
        _context: interpreter::CallerContext
    ) -> Result<Option<interpreter::RuntimeValue>, interpreter::Error> {
        Err(interpreter::Error::Trap(self.0.clone()))
    }
}

struct UserNoop;

pub fn user_noop(funcs: &mut interpreter::UserFunctions, func_name: &str) {
    let func_str = func_name.to_owned();
    funcs.insert(func_str.clone(), 
        interpreter::UserFunction {
            params: vec![],
            result: None,
            closure: Box::new(UserNoop),
        }
    );
}

impl interpreter::UserFunctionInterface for UserNoop {
    fn call(&mut self, 
        _module: &interpreter::ModuleInstance, 
        _context: interpreter::CallerContext
    ) -> Result<Option<interpreter::RuntimeValue>, interpreter::Error> {
        Ok(None)
    }
}