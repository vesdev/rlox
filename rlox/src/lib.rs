use std::path::PathBuf;

pub mod compiler;
pub mod error;
mod rlox_std;
pub mod vm;

use compiler::State;
use error::*;
use vm::Vm;

pub fn run_file(path: PathBuf) -> Result<(), Vec<Error>> {
    let src = std::fs::read_to_string(path).map_err(|e| vec![Error::Io(e.to_string())])?;
    run(src.as_str())
}

pub fn run(source: &str) -> Result<(), Vec<Error>> {
    let mut compiler =
        compiler::Compiler::new(source, State::new("", compiler::FunctionKind::Script));
    let mut vm = Vm::new();
    vm.define_native("clock", rlox_std::Clock::new());
    vm.execute(compiler.compile()?).map_err(|e| vec![e])?;

    Ok(())
}

#[cfg(test)]
pub mod tests;
