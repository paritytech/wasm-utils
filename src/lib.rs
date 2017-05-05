extern crate parity_wasm;
extern crate env_logger;
#[macro_use] extern crate log;
#[macro_use] extern crate lazy_static;

mod optimizer;
mod gas;
mod symbols;
mod logger;
mod ext;

pub use optimizer::optimize;
pub use gas::inject_gas_counter;
pub use logger::init_log;
pub use ext::externalize;