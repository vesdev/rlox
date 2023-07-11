use std::fmt::Display;

#[derive(Debug, Clone, Copy)]
pub enum OpCode {
    Constant { constant: usize },
    Nil,
    True,
    False,
    Pop,
    GetLocal { local: usize },
    SetLocal { local: usize },
    GetGlobal { name: usize },
    DefineGlobal { name: usize },
    SetGlobal { name: usize },
    GetUpValue { upvalue: usize },
    SetUpValue { upvalue: usize },
    GetProperty { prop_name: usize },
    SetProperty { prop_name: usize },
    GetSuper { name: usize },
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
    Jump { offset: usize },
    JumpIfFalse { offset: usize },
    Loop { offset: usize },
    Call { arg_count: usize },
    Invoke { method: usize, arg_count: usize },
    SuperInvoke { method: usize, arg_count: usize },
    Closure { func: usize },
    CloseUpValue,
    Return,
    Class { name: usize },
    Inerhit,
    Method { name: usize },
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
