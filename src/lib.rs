#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
#[macro_use]
extern crate alloc;

extern crate byteorder;
extern crate parity_wasm;
#[macro_use] extern crate log;
#[cfg(test)] #[macro_use] extern crate indoc;
#[cfg(test)] extern crate rand;
#[cfg(test)] extern crate binaryen;


pub mod rules;

mod build;
mod ext;
mod gas;
mod optimizer;
mod pack;
mod runtime_type;
mod graph;
mod ref_list;
mod symbols;
#[cfg(feature = "std")]
mod export_globals;
#[cfg(feature = "cli")]
pub mod logger;

pub mod stack_height;

pub use build::{build, Error as BuildError, SourceTarget};
pub use ext::{
	externalize, externalize_mem, shrink_unknown_stack, underscore_funcs, ununderscore_funcs,
};
pub use gas::inject_gas_counter;
pub use optimizer::{optimize, Error as OptimizerError};
pub use pack::{pack_instance, Error as PackingError};
pub use runtime_type::inject_runtime_type;
pub use graph::{Module, parse as graph_parse, generate as graph_generate};
pub use ref_list::{RefList, Entry, EntryRef, DeleteTransaction};
#[cfg(feature = "std")]
pub use export_globals::export_mutable_globals;
pub use parity_wasm::elements::Instruction;

pub struct TargetSymbols {
	pub create: &'static str,
	pub call: &'static str,
	pub ret: &'static str,
}

pub enum TargetRuntime {
	Substrate(TargetSymbols),
	PWasm(TargetSymbols),
}

impl TargetRuntime {

	pub fn substrate() -> TargetRuntime {
		TargetRuntime::Substrate(TargetSymbols {
			create: "deploy",
			call: "call",
			ret: "ext_return",
		})
	}

	pub fn pwasm() -> TargetRuntime {
		TargetRuntime::PWasm(TargetSymbols {
			create: "deploy",
			call: "call",
			ret: "ret",
		})
	}

	pub fn symbols(&self) -> &TargetSymbols {
		match self {
			TargetRuntime::Substrate(s) => s,
			TargetRuntime::PWasm(s) => s,
		}
	}

}

#[cfg(not(feature = "std"))]
mod std {
	pub use ::alloc::{borrow, boxed, string, vec};
	pub use core::*;

	pub mod rc {
		pub use alloc::rc::Rc;
	}

	pub mod collections {
		pub use alloc::collections::{BTreeMap, BTreeSet};
	}
}


#[cfg(feature = "std")]
mod std {
	pub use std::*;
}
