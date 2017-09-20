use parity_wasm::{elements};
use parity_wasm::elements::{ Section, Opcode };
use parity_wasm::elements::Opcode::*;

fn check_opcodes (opcodes: &[Opcode]) -> bool {
	for opcode in opcodes {
		match *opcode {
			F32Abs |
			F32Neg |
			F32Ceil |
			F32Floor |
			F32Trunc |
			F32Nearest |
			F32Sqrt |
			F32Add |
			F32Sub |
			F32Mul |
			F32Div |
			F32Min |
			F32Max |
			F32Copysign |
			F64Abs |
			F64Neg |
			F64Ceil |
			F64Floor |
			F64Trunc |
			F64Nearest |
			F64Sqrt |
			F64Add |
			F64Sub |
			F64Mul |
			F64Div |
			F64Min |
			F64Max |
			F64Copysign |
			I32TruncSF32 |
			I32TruncUF32 |
			I32TruncSF64 |
			I32TruncUF64 |
			I64TruncSF32 |
			I64TruncUF32 |
			I64TruncSF64 |
			I64TruncUF64 |
			F32ConvertSI32 |
			F32ConvertUI32 |
			F32ConvertSI64 |
			F32ConvertUI64 |
			F32DemoteF64 |
			F64ConvertSI32 |
			F64ConvertUI32 |
			F64ConvertSI64 |
			F64ConvertUI64 |
			F64PromoteF32 |
			I32ReinterpretF32 |
			I64ReinterpretF64 |
			F32ReinterpretI32 |
			F64ReinterpretI64 |
			F32Eq |
			F32Ne |
			F32Lt |
			F32Gt |
			F32Le |
			F32Ge |
			F64Eq |
			F64Ne |
			F64Lt |
			F64Gt |
			F64Le |
			F64Ge
				=> return true,
			_ 	=> continue
		}
	}
	false
}




pub fn have_indeterminism(module: elements::Module) -> bool {
	for section in module.sections() {
		match *section {
			Section::Code(ref cs) => {
				for body in cs.bodies() {
					if check_opcodes(body.code().elements()) {
						return true;
					}
				}
			},
			Section::Global(ref global) => {
				for entry in global.entries() {
					if check_opcodes(entry.init_expr().code()) {
						return true;
					}
				}
			},
			Section::Element(ref element) => {
				for entry in element.entries() {
					if check_opcodes(entry.offset().code()) {
						return true;
					}
				}
			}
			_ => continue
			}
		}
	false
}

#[cfg(test)]
mod tests {
	use parity_wasm::{builder, elements};
	use super::*;

	#[test]
	fn indeterminism_found() {
		let module = builder::module()
			.function().signature().return_type().f32().build()
			.body()
				.with_opcodes(elements::Opcodes::new(
					vec![
						elements::Opcode::F32Const(1), // unrelated to this test matter
						elements::Opcode::F32Const(1), // unrelated to this test matter
						elements::Opcode::F32Add,
						elements::Opcode::End
					]
				))
				.build()
			.build()
		.build();
		assert_eq!(true, have_indeterminism(module));
	}

	#[test]
	fn indeterminism_not() {
		let module = builder::module()
			.function().signature().return_type().f32().build()
			.body()
				.with_opcodes(elements::Opcodes::new(
					vec![
						elements::Opcode::I32Const(1),
						elements::Opcode::I32Const(1),
						elements::Opcode::I32Add,
						elements::Opcode::End
					]
				))
				.build()
			.build()
		.build();
		assert_eq!(false, have_indeterminism(module));
	}
}
