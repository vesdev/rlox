use std::path::PathBuf;

pub mod compiler;
pub mod error;
pub mod vm;

use error::*;
use vm::Vm;

pub fn run_file(path: PathBuf) -> Result<(), Vec<Error>> {
    let src = std::fs::read_to_string(path).map_err(|e| vec![Error::Io(e.to_string())])?;
    run(src.as_str())
}

pub fn run(source: &str) -> Result<(), Vec<Error>> {
    let mut compiler = compiler::Compiler::new(source, compiler::FunctionKind::Script);
    Vm::new()
        .call(&compiler.compile()?, 0)
        .map_err(|e| vec![e])?;
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

        fun foo() {
            print "aaa";
        }

        print foo;
    "#};

    println!("{}", src);

    if let Err(e) = run(src) {
        println!("{:#?}", e);
        panic!();
    }
}
