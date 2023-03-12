use std::fmt::Display;

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum OpCode {
    Constant(u8),
    Nil,
    True,
    False,

    Pop,
    GetLocal(u8),
    SetLocal(u8),
    GetGlobal(u8),
    DefineGlobal(u8),
    SetGlobal(u8),

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
}

impl OpCode {
    pub fn operands(&self) -> usize {
        match self {
            Self::Constant(_)
            | Self::DefineGlobal(_)
            | Self::GetGlobal(_)
            | Self::SetGlobal(_)
            | OpCode::GetLocal(_)
            | OpCode::SetLocal(_) => 1,
            _ => 0,
        }
    }
}

impl Display for OpCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
