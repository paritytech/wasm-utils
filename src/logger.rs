use std::env;
use log::LevelFilter;
use env_logger::Builder;
use lazy_static::lazy_static;

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
pub fn init() {
	let _ = *LOG_DUMMY;
}
