use env_logger::Builder;
use lazy_static::lazy_static;
use log::{trace, LevelFilter};

lazy_static! {
	static ref LOG_DUMMY: bool = {
		let mut builder = Builder::new();
		builder.filter(None, LevelFilter::Info);
		builder.parse_default_env();
		builder.init();
		trace!("logger initialized");
		true
	};
}

/// Intialize log with default settings
pub fn init() {
	let _ = *LOG_DUMMY;
}
