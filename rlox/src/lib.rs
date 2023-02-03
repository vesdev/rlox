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
    println!("result: {}\n", vm.interpret(compiler.compile()?).unwrap());
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::vm::{chunk::Chunk, opcode::OpCode, value};

    use super::*;

    #[test]
    fn bytecode_expression() {
        // >CODE<
        // Constant   1.2
        // Constant   3.4
        // Add
        // Constant   5.6
        // Divide
        // Negate
        // Return

        let mut chunk = Chunk::new();

        chunk.push_byte(OpCode::Constant as u8, 123);
        let constant = chunk.push_constant(value::Value::Number(1.2));
        chunk.push_byte(constant, 123);

        chunk.push_byte(OpCode::Constant as u8, 123);
        let constant = chunk.push_constant(value::Value::Number(3.4));
        chunk.push_byte(constant, 123);

        chunk.push_byte(OpCode::Add as u8, 123);

        chunk.push_byte(OpCode::Constant as u8, 123);
        let constant = chunk.push_constant(value::Value::Number(5.6));
        chunk.push_byte(constant, 123);

        chunk.push_byte(OpCode::Divide as u8, 123);
        chunk.push_byte(OpCode::Negate as u8, 123);

        chunk.push_byte(OpCode::Return as u8, 123);

        println!("{}", chunk.disassemble("code").unwrap());

        println!("\t>--\t>TRACE<");

        Vm::new().interpret(&chunk).unwrap();
    }
}
