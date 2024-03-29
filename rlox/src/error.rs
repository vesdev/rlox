use std::fmt::Debug;
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum Error {
    #[error("Io error")]
    Io(String),
    #[error("Runtime, {0}. Line {1}")]
    Runtime(String, usize),
    #[error("{0} tried to operate on an empty stack")]
    EmptyStack(String),
    #[error("Arithmetic, {0}")]
    Arithmetic(String),
    #[error("Compile, {0}. Line {1}")]
    Compile(String, usize),
    #[error("Native, {0}")]
    Native(String),
}
pub type Result<T, E = Error> = std::result::Result<T, E>;
