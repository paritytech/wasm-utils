use parity_wasm::interpreter::ModuleInstanceInterface;

pub struct Arena {
    dynamic_top: u32,
}

#[derive(Debug)]
pub struct Error;

impl Arena {
    pub fn new(stack_top: u32) -> Self {
        Arena {
            dynamic_top: stack_top,
        }
    }

    pub fn alloc(&mut self, size: u32) -> Result<u32, Error> {
        let previous_top = self.dynamic_top;
        self.dynamic_top += size;
        Ok(previous_top)
    }
}