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
            let last_ip = self.ip;

            let instruction: OpCode = chunk.get_op(self.ip);

            match instruction {
                OpCode::Constant(opr) => {
                    let constant = chunk.get_constant(opr as usize);
                    self.stack.push(constant);
                }
                OpCode::Nil => self.stack.push(Value::Nil),
                OpCode::True => self.stack.push(Value::Bool(true)),
                OpCode::False => self.stack.push(Value::Bool(false)),
                OpCode::Pop => {
                    self.stack.pop();
                }
                OpCode::GetLocal(opr) => self.stack.push(self.stack[opr as usize].clone()),
                OpCode::SetLocal(opr) => {
                    self.stack[opr as usize] = self.stack.last().unwrap().clone();
                }
                OpCode::GetGlobal(opr) => {
                    let name = chunk.get_constant(opr as usize);

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
                OpCode::DefineGlobal(opr) => {
                    let name = chunk.get_constant(opr as usize);

                    if let Value::Obj(name) = name {
                        if let Obj::String(name) = &*name {
                            self.globals
                                .insert(name.clone(), self.stack.pop().unwrap().clone());
                        }
                    }
                }
                OpCode::SetGlobal(opr) => {
                    let name = chunk.get_constant(opr as usize);

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
                OpCode::Jump(offset) => {
                    self.ip += offset - 1;
                }
                OpCode::JumpIfFalse(offset) => {
                    if self.stack.last().unwrap().is_falsey() {
                        self.ip += offset - 1;
                    }
                }
                OpCode::Loop(offset) => {
                    self.ip -= offset + 1;
                }
                OpCode::Return => {
                    return Ok(());
                }
            }

            self.ip += 1;

            if cfg!(debug_trace_execution) {
                let mut out = String::new();
                disassemble_instruction(&mut out, chunk, last_ip).unwrap();
                print!("{}", out);

                if cfg!(debug_trace_stack) {
                    print!("\t|\n\t|\t");
                    for value in &self.stack {
                        print!("| {} ", value);
                    }
                    println!("|\n\t|");
                }
            }
        }
    }
}

impl Default for Vm {
    fn default() -> Self {
        Self::new()
    }
}
