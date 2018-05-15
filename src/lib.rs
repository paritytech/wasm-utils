#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), feature(alloc))]

#[cfg(not(feature = "std"))]
#[macro_use]
extern crate alloc;

extern crate parity_wasm;
extern crate byteorder;
#[macro_use] extern crate log;

pub static CREATE_SYMBOL: &'static str = "deploy";
pub static CALL_SYMBOL: &'static str = "call";
pub static RET_SYMBOL: &'static str = "ret";

pub mod rules;

mod optimizer;
mod gas;
mod symbols;
mod ext;
mod pack;
mod runtime_type;

pub mod stack_height;

pub use optimizer::{optimize, Error as OptimizerError};
pub use gas::inject_gas_counter;
pub use ext::{externalize, externalize_mem, underscore_funcs, ununderscore_funcs, shrink_unknown_stack};
pub use pack::{pack_instance, Error as PackingError};
pub use runtime_type::inject_runtime_type;

#[cfg(not(feature = "std"))]
mod std {
	pub use core::*;
	pub use alloc::{vec, string, boxed, borrow};

	pub mod collections {
		pub use alloc::{BTreeMap, BTreeSet};
	}
}
