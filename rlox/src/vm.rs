pub mod chunk;
mod opcode;
mod value;

use crate::error::*;

use crate::vm::{
    chunk::{disassemble_instruction, Chunk},
    opcode::OpCode,
    value::Value,
};

pub type Result<T, E = Error> = std::result::Result<T, E>;

pub struct Vm {
    ip: usize,
    stack: Vec<Value>,
}

impl Vm {
    pub fn new() -> Self {
        Self {
            ip: 0,
            stack: Vec::new(),
        }
    }

    pub fn interpret(&mut self, chunk: Chunk) -> Result<()> {
        self.run(&chunk)
    }

    fn run(&mut self, chunk: &Chunk) -> Result<()> {
        loop {
            if cfg!(debug_trace_execution) {
                print!("\t|\t\t\t");
                for value in &self.stack {
                    print!("| {} ", value);
                }
                println!("|");

                let mut out = String::new();
                disassemble_instruction(&mut out, chunk, self.ip).unwrap();
                print!("{}", out);
            }

            let instruction: OpCode = OpCode::decode_unchecked(self.read_byte(chunk));
            match instruction {
                OpCode::Constant => {
                    let constant = self.read_constant(chunk);
                    self.stack.push(constant);
                }
                OpCode::Add => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a + b);
                }
                OpCode::Subtract => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a - b);
                }
                OpCode::Multiply => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a * b);
                }
                OpCode::Divide => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a / b);
                }
                OpCode::Negate => {
                    let val = -self.stack.pop().unwrap();
                    self.stack.push(val);
                }
                OpCode::Return => {
                    println!("\n({})", self.stack.pop().unwrap());
                    return Ok(());
                }
                _ => return Err(Error::Interpret("unknown opcode".to_string())),
            }
        }
    }

    #[inline]
    fn read_byte(&mut self, chunk: &Chunk) -> u8 {
        self.ip += 1;
        chunk.get_byte(self.ip - 1)
    }

    #[inline]
    fn read_constant(&mut self, chunk: &Chunk) -> Value {
        chunk.get_constant(self.read_byte(chunk) as usize)
    }
}

#[cfg(test)]
mod tests {
    use crate::vm::{chunk::Chunk, opcode::OpCode};

    use super::*;

    #[test]
    fn expression() {
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

        println!("\t\t-- trace --\n");

        Vm::new().interpret(chunk).unwrap();
    }
}
