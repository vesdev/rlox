use std::fmt::Display;

#[derive(Debug, Clone, Copy)]
pub enum OpCode {
    Constant(usize),
    Nil,
    True,
    False,

    Pop,
    GetLocal(usize),
    SetLocal(usize),
    GetGlobal(usize),
    DefineGlobal(usize),
    SetGlobal(usize),

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

impl Display for OpCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            format!("{:?}", self)
                .chars()
                .filter(|c| c.is_alphabetic())
                .collect::<String>()
        )
    }
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
