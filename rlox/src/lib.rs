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
    vm.interpret(compiler.compile()?)?;
    Ok(())
}

#[test]
fn variables() {
    let src = indoc::indoc! {r#"
        var breakfast = "beignets";
        var beverage = "cafe au lait";
        breakfast = "beignets with " + beverage;

        print breakfast;
    "#};

    println!("{}", src);
    run(src).unwrap();
}
