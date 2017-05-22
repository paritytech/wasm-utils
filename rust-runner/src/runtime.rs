use std::sync::Arc;
use std::collections::HashMap;

use parity_wasm::interpreter;

#[derive(Hash, PartialEq, Eq, Debug)]
pub struct StorageKey([u8; 32]);

#[derive(Debug, Default)]
pub struct StorageValue([u8; 32]);

struct ErrorStorage;

impl StorageKey {
	// todo: deal with memory views
	fn from_mem(vec: Vec<u8>) -> Result<Self, ErrorStorage> {
		if vec.len() != 32 { return Err(ErrorStorage); }
		let mut result = StorageKey([0u8; 32]);
		result.0.copy_from_slice(&vec[0..32]);
		Ok(result)
	}
}

impl StorageValue {
	// todo: deal with memory views
	// todo: deal with variable-length values when it comes
	fn from_mem(vec: Vec<u8>) -> Result<Self, ErrorStorage> {
		if vec.len() != 32 { return Err(ErrorStorage); }
		let mut result = StorageValue([0u8; 32]);
		result.0.copy_from_slice(&vec[0..32]);
		Ok(result)
	}

	fn as_slice(&self) -> &[u8] {
		&self.0
	}
}

pub struct Runtime {
	gas_counter: u64,
	gas_limit: u64,
	dynamic_top: u32,
	storage: HashMap<StorageKey, StorageValue>,
	memory: Arc<interpreter::MemoryInstance>,
}

#[derive(Debug)]
pub struct ErrorAlloc;

impl Runtime {
	pub fn with_params(memory: Arc<interpreter::MemoryInstance>, stack_space: u32, gas_limit: u64) -> Runtime {
		Runtime {
			gas_counter: 0,
			gas_limit: gas_limit,
			dynamic_top: stack_space,
			storage: HashMap::new(),
			memory: memory,
		}
	}

	pub fn storage_write(&mut self, context: interpreter::CallerContext) 
		-> Result<Option<interpreter::RuntimeValue>, interpreter::Error>
	{
		let val_ptr = context.value_stack.pop_as::<i32>()?;
		let key_ptr = context.value_stack.pop_as::<i32>()?;

		let key = StorageKey::from_mem(self.memory.get(key_ptr as u32, 32)?)
			.map_err(|_| interpreter::Error::Trap("Memory access violation".to_owned()))?;
		let val = StorageValue::from_mem(self.memory.get(val_ptr as u32, 32)?)
			.map_err(|_| interpreter::Error::Trap("Memory access violation".to_owned()))?;

		println!("write storage {:?} = {:?}", key, val);

		self.storage.insert(key, val);

		Ok(Some(0i32.into()))
	}

	pub fn storage_read(&mut self, context: interpreter::CallerContext) 
		-> Result<Option<interpreter::RuntimeValue>, interpreter::Error>
	{
			// arguments passed are in backward order (since it is stack)
		let val_ptr = context.value_stack.pop_as::<i32>()?;
		let key_ptr = context.value_stack.pop_as::<i32>()?;

		let key = StorageKey::from_mem(self.memory.get(key_ptr as u32, 32)?)
			.map_err(|_| interpreter::Error::Trap("Memory access violation".to_owned()))?;
		let empty = StorageValue([0u8; 32]);
		let val = self.storage.get(&key).unwrap_or(&empty);

		self.memory.set(val_ptr as u32, val.as_slice())?;

		println!("read storage {:?} (evaluated as {:?})", key, val);

		Ok(Some(0.into()))
	}

	pub fn malloc(&mut self, context: interpreter::CallerContext) 
		-> Result<Option<interpreter::RuntimeValue>, interpreter::Error>
	{
		let amount = context.value_stack.pop_as::<i32>()? as u32;
		let previous_top = self.dynamic_top;
		self.dynamic_top = previous_top + amount;
		Ok(Some((previous_top as i32).into()))
	}

	pub fn alloc(&mut self, amount: u32) -> Result<u32, ErrorAlloc> {
		let previous_top = self.dynamic_top;
		self.dynamic_top = previous_top + amount;
		Ok(previous_top.into())
	}

	fn gas(&mut self, context: interpreter::CallerContext) 
		-> Result<Option<interpreter::RuntimeValue>, interpreter::Error> 
	{
		let prev = self.gas_counter;
		let update = context.value_stack.pop_as::<i32>()? as u64;
		if prev + update > self.gas_limit {
			// exceeds gas
			Err(interpreter::Error::Trap(format!("Gas exceeds limits of {}", self.gas_limit)))
		} else {
			self.gas_counter = prev + update;
			Ok(None)
		}
	}

	fn user_trap(&mut self, _context: interpreter::CallerContext) 
		-> Result<Option<interpreter::RuntimeValue>, interpreter::Error> 
	{
		Err(interpreter::Error::Trap("unknown trap".to_owned()))
	}

	fn user_noop(&mut self, 
		_context: interpreter::CallerContext
	) -> Result<Option<interpreter::RuntimeValue>, interpreter::Error> {
		Ok(None)
	}    
}

impl interpreter::UserFunctionExecutor for Runtime {
	fn execute(&mut self, name: &str, context: interpreter::CallerContext) 
		-> Result<Option<interpreter::RuntimeValue>, interpreter::Error>
	{
		match name {
			"_malloc" => {
				self.malloc(context)
			},
			"_free" => {
				self.user_noop(context)
			},
			"_storage_read" => {
				self.storage_read(context)
			},
			"_storage_write" => {
				self.storage_write(context)
			},
			"gas" => {
				self.gas(context)
			},
			_ => {
				self.user_trap(context)
			}
		}
	}
}