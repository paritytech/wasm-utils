use alloc::{string::ToString, vec::Vec};
use core::{fmt, mem, ops::Range};
use parity_wasm::elements::{External, ImportEntry, Instruction, Instructions, MemoryType, Module};

const PAGE_SIZE: u32 = 64 * 1024;

pub struct Coverage {
	bitmap: Vec<u8>,
	info: Info,
}

pub struct Info {
	bitmap_location: Range<u32>,
	functions: Vec<Function>,
}

pub struct Function {
	pub num_locals: u32,
	pub basic_blocks: Vec<BasicBlock>,
	bitmap_offset: u32,
}

pub struct BasicBlock {
	pub num_instructions: u32,
}

#[derive(Default, Debug)]
pub struct Statistic {
	pub num_functions: u32,
	pub num_locals: u32,
	pub num_basic_blocks: u32,
	pub min_basic_block_size: u32,
	pub max_basic_block_size: u32,
	pub median_basic_block_size: u32,
	pub num_instructions: u32,
	pub used_functions: u32,
	pub used_locals: u32,
	pub used_basic_blocks: u32,
	pub used_instructions: u32,
}

impl fmt::Display for Statistic {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		core::write!(
			f,
			"{}/{} fns {}/{} locals ({}%) {}/{} bbs ({}%) {}/{} instr ({}%) min_bb: {} max_bb: {} avg_bb: {} median_bb: {}",
			self.used_functions,
			self.num_functions,
			self.used_locals,
			self.num_locals,
			(self.used_locals * 100).checked_div(self.num_locals).unwrap_or(100),
			self.used_basic_blocks,
			self.num_basic_blocks,
			(self.used_basic_blocks * 100).checked_div(self.num_basic_blocks).unwrap_or(100),
			self.used_instructions,
			self.num_instructions,
			(self.used_instructions * 100).checked_div(self.num_instructions).unwrap_or(100),
			self.min_basic_block_size,
			self.max_basic_block_size,
			self.num_instructions.checked_div(self.num_basic_blocks).unwrap_or(0),
			self.median_basic_block_size,
		)
	}
}

impl Info {
	pub fn bitmap_location(&self) -> &Range<u32> {
		&self.bitmap_location
	}

	pub fn functions(&self) -> &[Function] {
		&self.functions
	}
}

impl Coverage {
	pub fn new(info: Info, bitmap: Vec<u8>) -> Result<Self, &'static str> {
		if bitmap.len() != info.bitmap_location.len() {
			return Err("Bitmap has the wrong size.")
		}
		Ok(Self { bitmap, info })
	}

	pub fn info(&self) -> &Info {
		&self.info
	}

	pub fn block_was_used(&self, func: &Function, block: u32) -> bool {
		self.bitmap[(func.bitmap_offset + block / 8) as usize] & (1 << (block % 8)) != 0
	}

	pub fn create_statistic(&self) -> Statistic {
		let mut stats = Statistic { min_basic_block_size: u32::MAX, ..Default::default() };
		let mut block_sizes =
			Vec::with_capacity(self.info().functions().iter().map(|f| f.basic_blocks.len()).sum());
		for func in &self.info.functions {
			for (idx, block) in func.basic_blocks.iter().enumerate() {
				block_sizes.push(block.num_instructions);
				stats.num_basic_blocks += 1;
				stats.min_basic_block_size = stats.min_basic_block_size.min(block.num_instructions);
				stats.max_basic_block_size = stats.max_basic_block_size.max(block.num_instructions);
				stats.num_instructions += block.num_instructions;
				if idx == 0 {
					stats.num_functions += 1;
					stats.num_locals += func.num_locals;
				}
				if self.block_was_used(func, idx as u32) {
					stats.used_basic_blocks += 1;
					stats.used_instructions += block.num_instructions;
					if idx == 0 {
						stats.used_functions += 1;
						stats.used_locals += func.num_locals;
					}
				}
			}
		}

		block_sizes.sort_unstable();
		stats.median_basic_block_size = *block_sizes.get(block_sizes.len() / 2).unwrap_or(&0);

		stats
	}
}

pub fn instrument(module: &mut Module, gas_import: (&str, &str)) -> Result<Info, &'static str> {
	let (bitmap_start, gas_func) = {
		let imports = module
			.import_section_mut()
			.ok_or("Valid contracts should have an import section.")?;
		let gas_func =
			imports
				.entries()
				.iter()
				.filter(|e| matches!(e.external(), External::Function(_)))
				.enumerate()
				.find_map(|(idx, e)| {
					if (e.module(), e.field()) == gas_import {
						Some(idx as u32)
					} else {
						None
					}
				})
				.ok_or("Coverage requires the gas import as basic block marker")?;
		let mem = imports.entries_mut().iter_mut().find_map(|e| {
			if let External::Memory(mem) = e.external_mut() {
				Some(mem)
			} else {
				None
			}
		});
		let page = if let Some(mem) = mem {
			let limits = *mem.limits();
			let new_initial = limits.initial() + 1;
			let new_max = limits.maximum().map(|m| m.max(new_initial + 1));
			*mem = MemoryType::new(new_initial, new_max);
			limits.initial()
		} else {
			let mem = MemoryType::new(1, Some(1));
			imports.entries_mut().push(ImportEntry::new(
				"env".to_string(),
				"memory".to_string(),
				External::Memory(mem),
			));
			0
		};
		(page * PAGE_SIZE, gas_func)
	};

	let mut bitmap_current = bitmap_start;
	let functions = module
		.code_section_mut()
		.ok_or("Valid contracts should have a code section.")?
		.bodies_mut()
		.iter_mut()
		.map(|func| {
			let function = Function {
				bitmap_offset: bitmap_current - bitmap_start,
				num_locals: func.locals().len() as u32,
				basic_blocks: inject_coverage_code(func.code_mut(), &mut bitmap_current, gas_func),
			};
			function
		})
		.collect();

	let info = Info { bitmap_location: bitmap_start..bitmap_current, functions };

	if info.bitmap_location.len() as u32 > PAGE_SIZE {
		return Err("Coverage information does not fit into a single page")
	}

	Ok(info)
}

fn inject_coverage_code(
	body: &mut Instructions,
	start_offset: &mut u32,
	gas_func: u32,
) -> Vec<BasicBlock> {
	let original_instrs = mem::take(body.elements_mut());
	let original_len = original_instrs.len();
	let new_instrs = body.elements_mut();
	let mut block_idx = 0u32;

	let block_starts: Vec<_> = {
		let markers = original_instrs.into_iter().enumerate().filter_map(|(pos, instr)| {
			new_instrs.push(instr.clone());
			if matches!(instr, Instruction::Call(idx) if idx == gas_func) {
				let offset = *start_offset + block_idx / 8;
				let value = 1 << (block_idx % 8);
				new_instrs.extend_from_slice(&[
					Instruction::I32Const(0), // address for store
					Instruction::I32Const(0), // address for load
					Instruction::I32Load8U(0, offset),
					Instruction::I32Const(value),
					Instruction::I32Or,
					Instruction::I32Store8(0, offset),
				]);
				block_idx += 1;
				// A gas instruction is always prepended with a const instruction
				pos.checked_sub(1)
			} else {
				None
			}
		});
		core::iter::once(0)
			.chain(markers)
			.chain(core::iter::once(original_len))
			.collect()
	};

	let blocks: Vec<_> = block_starts
		.windows(2)
		.map(|window| BasicBlock { num_instructions: (window[1] - window[0]) as u32 })
		.filter(|block| block.num_instructions > 0)
		.collect();

	*start_offset += rounded_len(blocks.len() as u32);
	blocks
}

fn rounded_len(num: u32) -> u32 {
	num / 8 + if num % 8 == 0 { 0 } else { 1 }
}
