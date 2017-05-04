extern crate parity_wasm;

mod optimizer;
mod gas;
mod symbols;

pub use optimizer::optimize;
pub use gas::inject_gas_counter;