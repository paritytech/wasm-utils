use parity_wasm::interpreter::{self, ModuleInstance};
use runtime::Runtime;

pub struct Arena {
    pub runtime: Runtime,
}

#[derive(Debug)]
pub struct Error;

impl Arena {
    pub fn alloc(&self, size: u32) -> Result<u32, Error> {
        // todo: maybe use unsafe cell since it has nothing to do with threads
        let previous_top = self.runtime.env().dynamic_top.get();
        self.runtime.env().dynamic_top.set(previous_top + size);
        Ok(previous_top)
    }
}

impl interpreter::UserFunctionInterface for Arena {
    fn call(&mut self, _module: &ModuleInstance, context: interpreter::CallerContext) -> Result<Option<interpreter::RuntimeValue>, interpreter::Error> {
        let amount = context.value_stack.pop_as::<i32>()?;
        self.alloc(amount as u32)
            .map(|val| Some((val as i32).into()))
            .map_err(|e| interpreter::Error::Trap(format!("Allocator failure: {}", "todo: format arg")))
    }    
}