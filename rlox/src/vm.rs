pub mod chunk;
pub mod opcode;
pub mod value;

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

    pub fn interpret(&mut self, chunk: &Chunk) -> Result<Value> {
        self.run(chunk)
    }

    fn run(&mut self, chunk: &Chunk) -> Result<Value> {
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
                    return Ok(self.stack.pop().unwrap());
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
