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
    GetUpValue(usize),
    SetUpValue(usize),
    GetProperty(usize),
    SetProperty(usize),
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
    Jump(usize),
    JumpIfFalse(usize),
    Loop(usize),
    Call(usize),
    Closure(usize),
    CloseUpValue,
    Return,
    Class(usize),
    Method(usize),
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
