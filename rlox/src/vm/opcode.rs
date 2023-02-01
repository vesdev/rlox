use std::fmt::Display;

#[repr(u8)]
#[derive(Debug)]
pub enum OpCode {
    Constant,

    Add,
    Subtract,
    Multiply,
    Divide,
    Negate,

    Return,

    Max = OpCode::Return as u8 + 1,
}

impl OpCode {
    #[inline]
    pub fn decode_unchecked(val: u8) -> Self {
        unsafe { std::mem::transmute(val) }
    }

    #[inline]
    pub fn decode(v: u8) -> Option<OpCode> {
        if v >= OpCode::Max as u8 {
            None
        } else {
            Some(Self::decode_unchecked(v))
        }
    }

    pub fn operands(&self) -> usize {
        match self {
            OpCode::Constant => 1,
            _ => 0,
        }
    }
}

impl Display for OpCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
