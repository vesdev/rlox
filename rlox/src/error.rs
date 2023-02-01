#[derive(Debug)]
pub enum Error {
    Io(String),
    Interpret(String),
    Compile(String),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
