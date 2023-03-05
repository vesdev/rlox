use std::fmt::Debug;

pub enum Error {
    Io(String),
    Interpret(String, usize),
    Compile(String, usize),
}

impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(arg0) => f.debug_tuple("Io").field(arg0).finish(),
            Self::Interpret(arg0, arg1) => {
                write!(f, "Interpret, {}. Line {}", arg0, arg1)
            }
            Self::Compile(arg0, arg1) => {
                write!(f, "Compile, {}. Line {}", arg0, arg1)
            }
        }
    }
}
pub type Result<T, E = Error> = std::result::Result<T, E>;
