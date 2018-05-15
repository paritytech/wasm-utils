#[macro_use] extern crate log;
#[macro_use] extern crate lazy_static;
extern crate env_logger;

use std::env;
use log::LevelFilter;
use env_logger::Builder;

lazy_static! {
	static ref LOG_DUMMY: bool = {
		let mut builder = Builder::new();
		builder.filter(None, LevelFilter::Info);

		if let Ok(log) = env::var("RUST_LOG") {
			builder.parse(&log);
		}

		builder.init();
		trace!("logger initialized");
		true
	};
}

/// Intialize log with default settings
pub fn init_log() {
	let _ = *LOG_DUMMY;
}
