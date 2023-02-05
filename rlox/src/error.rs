#[derive(Debug)]
pub enum Error {
    Io(String),
    Interpret(String, usize),
    Compile(String, usize),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
