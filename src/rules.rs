use std::collections::HashMap;
use parity_wasm::elements;

pub struct UnknownInstruction;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Metering {
    Regular,
    Forbidden,
    Fixed(u32),
}

#[derive(Debug, Hash, PartialEq, Eq, Copy, Clone)]
pub enum InstructionType {
    Bit,
    Add,
    Mul,
    Div,
    Load,
    Store,
    Const,
    FloatConst,
    Local,
    Global,
    ControlFlow,
    IntegerComparsion,
    FloatComparsion,
    Float,
    Conversion,
    FloatConversion,
    Reinterpretation,
    Unreachable,
    Nop,
    CurrentMemory,
    GrowMemory,
}

impl ::std::str::FromStr for InstructionType {
    type Err = UnknownInstruction;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "bit" => Ok(InstructionType::Bit),
            "add" => Ok(InstructionType::Add),
            "mul" => Ok(InstructionType::Mul),
            "div" => Ok(InstructionType::Div),
            "load" => Ok(InstructionType::Load),
            "store" => Ok(InstructionType::Store),
            "const" => Ok(InstructionType::Const),
            "local" => Ok(InstructionType::Local),
            "global" => Ok(InstructionType::Global),
            "flow" => Ok(InstructionType::ControlFlow),
            "integer_comp" => Ok(InstructionType::IntegerComparsion),
            "float_comp" => Ok(InstructionType::FloatComparsion),
            "float" => Ok(InstructionType::Float),
            "conversion" => Ok(InstructionType::Conversion),
            "float_conversion" => Ok(InstructionType::FloatConversion),
            "reinterpret" => Ok(InstructionType::Reinterpretation),
            "unreachable" => Ok(InstructionType::Unreachable),
            "nop" => Ok(InstructionType::Nop),
            "currrent_mem" => Ok(InstructionType::CurrentMemory),
            "grow_mem" => Ok(InstructionType::GrowMemory),
            _ => Err(UnknownInstruction),
        }
    }
}

impl InstructionType {
    pub fn op(opcode: &elements::Opcode) -> Self {
        use parity_wasm::elements::Opcode::*;

        match *opcode {
            Unreachable => InstructionType::Unreachable,
            Nop => InstructionType::Nop,
            Block(_) => InstructionType::ControlFlow,
            Loop(_) => InstructionType::ControlFlow,
            If(_) => InstructionType::ControlFlow,
            Else => InstructionType::ControlFlow,
            End => InstructionType::ControlFlow,
            Br(_) => InstructionType::ControlFlow,
            BrIf(_) => InstructionType::ControlFlow,
            BrTable(_, _) => InstructionType::ControlFlow,
            Return => InstructionType::ControlFlow,
            Call(_) => InstructionType::ControlFlow,
            CallIndirect(_, _) => InstructionType::ControlFlow,
            Drop => InstructionType::ControlFlow,
            Select => InstructionType::ControlFlow,

            GetLocal(_) => InstructionType::Local,
            SetLocal(_) => InstructionType::Local,
            TeeLocal(_) => InstructionType::Local,
            GetGlobal(_) => InstructionType::Local,
            SetGlobal(_) => InstructionType::Local,

            I32Load(_, _) => InstructionType::Load,
            I64Load(_, _) => InstructionType::Load,
            F32Load(_, _) => InstructionType::Load,
            F64Load(_, _) => InstructionType::Load,
            I32Load8S(_, _) => InstructionType::Load,
            I32Load8U(_, _) => InstructionType::Load,
            I32Load16S(_, _) => InstructionType::Load,
            I32Load16U(_, _) => InstructionType::Load,
            I64Load8S(_, _) => InstructionType::Load,
            I64Load8U(_, _) => InstructionType::Load,
            I64Load16S(_, _) => InstructionType::Load,
            I64Load16U(_, _) => InstructionType::Load,
            I64Load32S(_, _) => InstructionType::Load,
            I64Load32U(_, _) => InstructionType::Load,

            I32Store(_, _) => InstructionType::Store,
            I64Store(_, _) => InstructionType::Store,
            F32Store(_, _) => InstructionType::Store,
            F64Store(_, _) => InstructionType::Store,
            I32Store8(_, _) => InstructionType::Store,
            I32Store16(_, _) => InstructionType::Store,
            I64Store8(_, _) => InstructionType::Store,
            I64Store16(_, _) => InstructionType::Store,
            I64Store32(_, _) => InstructionType::Store,

            CurrentMemory(_) => InstructionType::CurrentMemory,
            GrowMemory(_) => InstructionType::GrowMemory,

            I32Const(_) => InstructionType::Const,
            I64Const(_) => InstructionType::Const,

            F32Const(_) => InstructionType::FloatConst,
            F64Const(_) => InstructionType::FloatConst,

            I32Eqz => InstructionType::IntegerComparsion,
            I32Eq => InstructionType::IntegerComparsion,
            I32Ne => InstructionType::IntegerComparsion,
            I32LtS => InstructionType::IntegerComparsion,
            I32LtU => InstructionType::IntegerComparsion,
            I32GtS => InstructionType::IntegerComparsion,
            I32GtU => InstructionType::IntegerComparsion,
            I32LeS => InstructionType::IntegerComparsion,
            I32LeU => InstructionType::IntegerComparsion,
            I32GeS => InstructionType::IntegerComparsion,
            I32GeU => InstructionType::IntegerComparsion,

            I64Eqz => InstructionType::IntegerComparsion,
            I64Eq => InstructionType::IntegerComparsion,
            I64Ne => InstructionType::IntegerComparsion,
            I64LtS => InstructionType::IntegerComparsion,
            I64LtU => InstructionType::IntegerComparsion,
            I64GtS => InstructionType::IntegerComparsion,
            I64GtU => InstructionType::IntegerComparsion,
            I64LeS => InstructionType::IntegerComparsion,
            I64LeU => InstructionType::IntegerComparsion,
            I64GeS => InstructionType::IntegerComparsion,
            I64GeU => InstructionType::IntegerComparsion,

            F32Eq => InstructionType::FloatComparsion,
            F32Ne => InstructionType::FloatComparsion,
            F32Lt => InstructionType::FloatComparsion,
            F32Gt => InstructionType::FloatComparsion,
            F32Le => InstructionType::FloatComparsion,
            F32Ge => InstructionType::FloatComparsion,

            F64Eq => InstructionType::FloatComparsion,
            F64Ne => InstructionType::FloatComparsion,
            F64Lt => InstructionType::FloatComparsion,
            F64Gt => InstructionType::FloatComparsion,
            F64Le => InstructionType::FloatComparsion,
            F64Ge => InstructionType::FloatComparsion,

            I32Clz => InstructionType::Bit,
            I32Ctz => InstructionType::Bit,
            I32Popcnt => InstructionType::Bit,
            I32Add => InstructionType::Add,
            I32Sub => InstructionType::Add,
            I32Mul => InstructionType::Mul,
            I32DivS => InstructionType::Div,
            I32DivU => InstructionType::Div,
            I32RemS => InstructionType::Div,
            I32RemU => InstructionType::Div,
            I32And => InstructionType::Bit,
            I32Or => InstructionType::Bit,
            I32Xor => InstructionType::Bit,
            I32Shl => InstructionType::Bit,
            I32ShrS => InstructionType::Bit,
            I32ShrU => InstructionType::Bit,
            I32Rotl => InstructionType::Bit,
            I32Rotr => InstructionType::Bit,

            I64Clz => InstructionType::Bit,
            I64Ctz => InstructionType::Bit,
            I64Popcnt => InstructionType::Bit,
            I64Add => InstructionType::Add,
            I64Sub => InstructionType::Add,
            I64Mul => InstructionType::Mul,
            I64DivS => InstructionType::Div,
            I64DivU => InstructionType::Div,
            I64RemS => InstructionType::Div,
            I64RemU => InstructionType::Div,
            I64And => InstructionType::Bit,
            I64Or => InstructionType::Bit,
            I64Xor => InstructionType::Bit,
            I64Shl => InstructionType::Bit,
            I64ShrS => InstructionType::Bit,
            I64ShrU => InstructionType::Bit,
            I64Rotl => InstructionType::Bit,
            I64Rotr => InstructionType::Bit,

            F32Abs => InstructionType::Float,
            F32Neg => InstructionType::Float,
            F32Ceil => InstructionType::Float,
            F32Floor => InstructionType::Float,
            F32Trunc => InstructionType::Float,
            F32Nearest => InstructionType::Float,
            F32Sqrt => InstructionType::Float,
            F32Add => InstructionType::Float,
            F32Sub => InstructionType::Float,
            F32Mul => InstructionType::Float,
            F32Div => InstructionType::Float,
            F32Min => InstructionType::Float,
            F32Max => InstructionType::Float,
            F32Copysign => InstructionType::Float,
            F64Abs => InstructionType::Float,
            F64Neg => InstructionType::Float,
            F64Ceil => InstructionType::Float,
            F64Floor => InstructionType::Float,
            F64Trunc => InstructionType::Float,
            F64Nearest => InstructionType::Float,
            F64Sqrt => InstructionType::Float,
            F64Add => InstructionType::Float,
            F64Sub => InstructionType::Float,
            F64Mul => InstructionType::Float,
            F64Div => InstructionType::Float,
            F64Min => InstructionType::Float,
            F64Max => InstructionType::Float,
            F64Copysign => InstructionType::Float,

            I32WrapI64 => InstructionType::Conversion,
            I64ExtendSI32 => InstructionType::Conversion,
            I64ExtendUI32 => InstructionType::Conversion,

            I32TruncSF32 => InstructionType::FloatConversion,
            I32TruncUF32 => InstructionType::FloatConversion,
            I32TruncSF64 => InstructionType::FloatConversion,
            I32TruncUF64 => InstructionType::FloatConversion,
            I64TruncSF32 => InstructionType::FloatConversion,
            I64TruncUF32 => InstructionType::FloatConversion,
            I64TruncSF64 => InstructionType::FloatConversion,
            I64TruncUF64 => InstructionType::FloatConversion,
            F32ConvertSI32 => InstructionType::FloatConversion,
            F32ConvertUI32 => InstructionType::FloatConversion,
            F32ConvertSI64 => InstructionType::FloatConversion,
            F32ConvertUI64 => InstructionType::FloatConversion,
            F32DemoteF64 => InstructionType::FloatConversion,
            F64ConvertSI32 => InstructionType::FloatConversion,
            F64ConvertUI32 => InstructionType::FloatConversion,
            F64ConvertSI64 => InstructionType::FloatConversion,
            F64ConvertUI64 => InstructionType::FloatConversion,
            F64PromoteF32 => InstructionType::FloatConversion,

            I32ReinterpretF32 => InstructionType::Reinterpretation,
            I64ReinterpretF64 => InstructionType::Reinterpretation,
            F32ReinterpretI32 => InstructionType::Reinterpretation,
            F64ReinterpretI64 => InstructionType::Reinterpretation,
        }
    }
}

#[derive(Debug)]
pub struct Set {
    regular: u32,
    entries: HashMap<InstructionType, Metering>,
    grow: u32,
}

impl Default for Set {
    fn default() -> Self {
        Set {
            regular: 1,
            entries: HashMap::new(),
            grow: 0,
        }
    }
}

impl Set {
    pub fn new(regular: u32, entries: HashMap<InstructionType, Metering>) -> Self {
        Set { regular: regular, entries: entries, grow: 0 }
    }

    pub fn process(&self, opcode: &elements::Opcode) -> Result<u32, ()>  {
        match self.entries.get(&InstructionType::op(opcode)).map(|x| *x) {
            None | Some(Metering::Regular) => Ok(self.regular),
            Some(Metering::Forbidden) => Err(()),
            Some(Metering::Fixed(val)) => Ok(val),
        }
    }

    pub fn grow_cost(&self) -> u32 {
        self.grow
    }

    pub fn with_grow_cost(mut self, val: u32) -> Self {
        self.grow = val;
        self
    }

    pub fn with_forbidden_floats(mut self) -> Self {
        self.entries.insert(InstructionType::Float, Metering::Forbidden);
        self.entries.insert(InstructionType::FloatComparsion, Metering::Forbidden);
        self.entries.insert(InstructionType::FloatConst, Metering::Forbidden);
        self.entries.insert(InstructionType::FloatConversion, Metering::Forbidden);
        self
    }
}
