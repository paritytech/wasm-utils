use parity_wasm::interpreter;
use runtime::Runtime;

pub struct GasCounter {
    pub runtime: Runtime,
}

impl interpreter::UserFunctionInterface for GasCounter {
    fn call(&mut self, context: interpreter::CallerContext) -> Result<Option<interpreter::RuntimeValue>, interpreter::Error> {
        let prev = self.runtime.env().gas_counter.get();
        let update = context.value_stack.pop_as::<i32>()? as u64;
        if prev + update > self.runtime.env().gas_limit {
            // exceeds gas
            Err(interpreter::Error::Trap(format!("Gas exceeds limits of {}", self.runtime.env().gas_limit)))
        } else {
            self.runtime.env().gas_counter.set(prev + update);
            Ok(None)
        }
    }
}