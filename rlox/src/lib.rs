use std::{path::PathBuf, time::Instant};

pub mod compiler;
pub mod error;
mod rlox_std;
pub mod vm;

use error::*;
use vm::{
    object::{Native, NativeFun},
    value::Value,
    Vm,
};

pub fn run_file(path: PathBuf) -> Result<(), Vec<Error>> {
    let src = std::fs::read_to_string(path).map_err(|e| vec![Error::Io(e.to_string())])?;
    run(src.as_str())
}

pub fn run(source: &str) -> Result<(), Vec<Error>> {
    let mut compiler = compiler::Compiler::new(source, compiler::FunctionKind::Script);
    let mut vm = Vm::new();
    vm.define_native("clock", rlox_std::Clock::new());
    vm.call(compiler.compile()?, 0).map_err(|e| vec![e])?;

    Ok(())
}

#[test]
fn for_loop() {
    let src = indoc::indoc! {r#"

        for(var i = 3; i < 3; i = i + 1)
        {
            for(var j = 0; j < 5; j = j + 1)
            {
                print j;
            }
        }
    "#};

    println!("{}", src);

    if let Err(e) = run(src) {
        println!("{:#?}", e);
        panic!();
    }
}

#[test]
fn func() {
    let src = indoc::indoc! {r#"
    fun fib(n) {
        if (n < 2) return n;
        return fib(n - 2) + fib(n - 1);
    }
      
    var start = clock();
    print fib(35);
    print clock() - start;

    "#};

    println!("{}", src);

    if let Err(e) = run(src) {
        println!("{:#?}", e);
        panic!();
    }
}
