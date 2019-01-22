#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), feature(alloc))]

#[cfg(not(feature = "std"))]
#[macro_use]
extern crate alloc;

extern crate parity_wasm;
extern crate byteorder;
#[macro_use] extern crate log;

pub mod rules;

mod build;
mod optimizer;
mod gas;
mod symbols;
mod ext;
mod pack;
mod runtime_type;
mod graph;
mod ref_list;

pub mod stack_height;

pub use build::{build, SourceTarget, Error as BuildError};
pub use optimizer::{optimize, Error as OptimizerError};
pub use gas::inject_gas_counter;
pub use ext::{externalize, externalize_mem, underscore_funcs, ununderscore_funcs, shrink_unknown_stack};
pub use pack::{pack_instance, Error as PackingError};
pub use runtime_type::inject_runtime_type;

pub struct TargetRuntime {
	pub create_symbol: &'static str,
	pub call_symbol: &'static str,
	pub return_symbol: &'static str,
}

impl TargetRuntime {
	pub fn substrate() -> TargetRuntime {
		TargetRuntime {
			create_symbol: "deploy",
			call_symbol: "call",
			return_symbol: "ext_return",
		}
	}

	pub fn pwasm() -> TargetRuntime {
		TargetRuntime {
			create_symbol: "deploy",
			call_symbol: "call",
			return_symbol: "ret",
		}
	}
}

#[cfg(not(feature = "std"))]
mod std {
	pub use core::*;
	pub use alloc::{vec, string, boxed, borrow};

	pub mod rc {
		pub use alloc::rc::Rc;
	}

	pub mod collections {
		pub use alloc::collections::{BTreeMap, BTreeSet};
	}
}
