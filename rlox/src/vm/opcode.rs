use std::fmt::Display;

#[repr(u8)]
#[derive(Debug)]
pub enum OpCode {
    Constant,
    Nil,
    True,
    False,

    Pop,
    GetGlobal,
    DefineGlobal,
    SetGlobal,

    Equal,
    Greater,
    Less,

    Add,
    Subtract,
    Multiply,
    Divide,
    Not,
    Negate,
    Print,

    Return,

    Max = OpCode::Return as u8 + 1,
}

impl OpCode {
    #[inline]
    pub fn decode_unchecked(val: u8) -> Self {
        unsafe { std::mem::transmute(val) }
    }

    #[inline]
    pub fn decode(v: u8) -> Option<Self> {
        if v >= Self::Max as u8 {
            None
        } else {
            Some(Self::decode_unchecked(v))
        }
    }

    pub fn operands(&self) -> usize {
        match self {
            Self::Constant | Self::DefineGlobal | Self::GetGlobal | Self::SetGlobal => 1,
            _ => 0,
        }
    }
}

impl Display for OpCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
