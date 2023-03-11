pub mod chunk;
pub mod object;
pub mod opcode;
pub mod value;

use std::collections::HashMap;

use crate::error::*;

use crate::vm::{
    chunk::{disassemble_instruction, Chunk},
    opcode::OpCode,
    value::Value,
};

use self::object::Obj;

pub type Result<T, E = Error> = std::result::Result<T, E>;

pub struct Vm {
    ip: usize,
    stack: Vec<Value>,
    globals: HashMap<String, Value>,
}

impl Vm {
    pub fn new() -> Self {
        Self {
            ip: 0,
            stack: Vec::new(),
            globals: HashMap::new(),
        }
    }

    pub fn interpret(&mut self, chunk: &Chunk) -> Result<()> {
        self.run(chunk)
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
                OpCode::Nil => self.stack.push(Value::Nil),
                OpCode::True => self.stack.push(Value::Bool(true)),
                OpCode::False => self.stack.push(Value::Bool(false)),
                OpCode::Pop => {
                    self.stack.pop();
                }
                OpCode::GetLocal => {
                    let slot = self.stack.pop().unwrap();

                    if let Value::Number(slot) = slot {
                        println!("slot:{}", slot);
                        self.stack.push(self.stack[slot as usize].clone());
                    }
                }
                OpCode::SetLocal => {
                    let slot = self.stack.pop().unwrap();

                    if let Value::Number(slot) = slot {
                        self.stack[slot as usize] = self.stack.last().unwrap().clone();
                    }
                }
                OpCode::GetGlobal => {
                    let name = self.read_constant(chunk);

                    if let Value::Obj(name) = name {
                        if let Obj::String(name) = &*name {
                            if let Some(val) = self.globals.get(name) {
                                self.stack.push(val.clone());
                            } else {
                                let msg = format!("Undefined variable {}", name).to_string();
                                return Err(Error::Interpret(msg, chunk.get_line(self.ip)));
                            }
                        }
                    }
                }
                OpCode::DefineGlobal => {
                    let name = self.read_constant(chunk);
                    if let Value::Obj(name) = name {
                        if let Obj::String(name) = &*name {
                            self.globals
                                .insert(name.clone(), self.stack.last().unwrap().clone());
                        }
                    }
                }
                OpCode::SetGlobal => {
                    let name = self.read_constant(chunk);
                    if let Value::Obj(name) = name {
                        if let Obj::String(name) = &*name {
                            if self
                                .globals
                                .insert(name.clone(), self.stack.last().unwrap().clone())
                                .is_none()
                            {
                                self.globals.remove(name);
                                let msg = format!("Undefined variable {}", name).to_string();
                                return Err(Error::Interpret(msg, chunk.get_line(self.ip)));
                            }
                        }
                    }
                }
                OpCode::Equal => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(Value::Bool(a == b));
                }
                OpCode::Greater => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(Value::Bool(a > b));
                }
                OpCode::Less => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(Value::Bool(a < b));
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
                OpCode::Not => {
                    let val = !self.stack.pop().unwrap();
                    if let Some(val) = val {
                        self.stack.push(val);
                    } else {
                        return Err(Error::Interpret(
                            "Not '!' expected boolean".to_string(),
                            chunk.get_line(self.ip),
                        ));
                    }
                }
                OpCode::Negate => {
                    let val = -self.stack.pop().unwrap();
                    self.stack.push(val);
                }
                OpCode::Print => {
                    println!("{}", self.stack.pop().unwrap());
                }
                OpCode::Return => {
                    return Ok(());
                }
                _ => {
                    return Err(Error::Interpret(
                        "unknown opcode".to_string(),
                        chunk.get_line(self.ip),
                    ))
                }
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
