use std::path::PathBuf;

pub mod compiler;
pub mod error;
pub mod vm;

use error::*;
use vm::{chunk::Chunk, Vm};

pub fn run_file(path: PathBuf) -> Result<()> {
    let src = std::fs::read_to_string(path).map_err(|e| Error::Io(e.to_string()))?;
    run(src.as_str())
}

pub fn run<'b>(source: &'b str) -> Result<()> {
    let mut compiler = compiler::Compiler::new(source);
    let mut vm = Vm::new();
    let mut chunk = Chunk::new();

    compiler.compile(&mut chunk)?;

    Ok(vm.interpret(chunk).unwrap())
}
