use std::fmt::Debug;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Io error")]
    Io(String),
    #[error("Interpret, {0}. Line {1}")]
    Interpret(String, usize),
    #[error("Compile, {0}. Line {1}")]
    Compile(String, usize),
}
pub type Result<T, E = Error> = std::result::Result<T, E>;
