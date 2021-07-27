//! Configuration of source binaries

pub const UNKNOWN_TRIPLET: &str = "wasm32-unknown-unknown";
pub const EMSCRIPTEN_TRIPLET: &str = "wasm32-unknown-emscripten";

use pwasm_utils::SourceTarget;

/// Configuration of previous build step (cargo compilation)
#[derive(Debug)]
pub struct SourceInput<'a> {
	target_dir: &'a str,
	bin_name: &'a str,
	final_name: &'a str,
	target: SourceTarget,
}

impl<'a> SourceInput<'a> {
	pub fn new<'b>(target_dir: &'b str, bin_name: &'b str) -> SourceInput<'b> {
		SourceInput { target_dir, bin_name, final_name: bin_name, target: SourceTarget::Emscripten }
	}

	pub fn unknown(mut self) -> Self {
		self.target = SourceTarget::Unknown;
		self
	}

	pub fn emscripten(mut self) -> Self {
		self.target = SourceTarget::Emscripten;
		self
	}

	pub fn with_final(mut self, final_name: &'a str) -> Self {
		self.final_name = final_name;
		self
	}

	pub fn target_dir(&self) -> &str {
		self.target_dir
	}

	pub fn bin_name(&self) -> &str {
		self.bin_name
	}

	pub fn final_name(&self) -> &str {
		self.final_name
	}

	pub fn target(&self) -> SourceTarget {
		self.target
	}
}
