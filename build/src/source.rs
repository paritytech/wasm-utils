//! Configuration of source binaries

/// Target configiration of previous build step
#[derive(Debug)]
pub enum SourceTarget {
	Emscripten,
	Unknown,
}

/// Configuration of previous build step (cargo compilation)
#[derive(Debug)]
pub struct SourceInput<'a> {
	target_dir: &'a str,
	bin_name: &'a str,
	target: SourceTarget,
}

impl<'a> SourceInput<'a> {
	pub fn new<'b>(target_dir: &'b str, bin_name: &'b str) -> SourceInput<'b> {
		SourceInput {
			target_dir: target_dir,
			bin_name: bin_name,
			target: SourceTarget::Emscripten,
		}
	}

	pub fn unknown(mut self) -> Self {
		self.target = SourceTarget::Unknown;
		self
	}

	pub fn emscripten(mut self) -> Self {
		self.target = SourceTarget::Emscripten;
		self
	}

	pub fn target_dir(&self) -> &str {
		&self.target_dir
	}

	pub fn bin_name(&self) -> &str {
		&self.bin_name
	}
}