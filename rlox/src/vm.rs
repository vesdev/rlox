pub mod chunk;
pub mod object;
pub mod opcode;
pub mod value;

use crate::error::*;
use colored::Colorize;
use std::{collections::HashMap, ops::Deref, rc::Rc};

use crate::vm::{
    chunk::{disassemble_instruction, Chunk},
    opcode::OpCode,
    value::Value,
};

use self::object::Function;

pub type Result<T, E = Error> = std::result::Result<T, E>;

macro_rules! stack_operands {
    ( $vec:expr $(, $name:ident)+ ) => {
        $(
            let $name = $vec.pop().unwrap();
        )*
    };
}

pub struct Vm<'a> {
    stack: Vec<Value>,
    globals: HashMap<String, Value>,
    frames: Vec<CallFrame<'a>>,
}

impl<'a> Vm<'a> {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            globals: HashMap::new(),
            frames: Vec::new(),
        }
    }

    pub fn call(&mut self, function: &'a Function, arg_count: usize) -> Result<()> {
        self.frames
            .push(CallFrame::new(function, self.stack.len() - arg_count));
        self.run(function)
    }

    fn run(&mut self, function: &Function) -> Result<()> {
        let chunk = &function.chunk;
        let mut frame = self.frames.last_mut().unwrap();

        loop {
            let last_ip = frame.ip;
            let instruction: OpCode = chunk.get_op(frame.ip);

            if cfg!(debug_trace_execution) {
                let mut out = String::new();
                disassemble_instruction(&mut out, chunk, last_ip).unwrap();

                print!("{}|", out);

                for value in &self.stack {
                    print!(" {}, ", value.to_string().green());
                }
                println!("|");
            }

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
                OpCode::GetLocal(opr) => self
                    .stack
                    .push(self.stack[opr as usize + frame.slot].clone()),
                OpCode::SetLocal(opr) => {
                    self.stack[opr as usize + frame.slot] = self.stack.last().unwrap().clone();
                }
                OpCode::GetGlobal(opr) => {
                    let name = chunk.get_constant(opr as usize).to_string();

                    if let Some(val) = self.globals.get(&name) {
                        self.stack.push(val.clone());
                    } else {
                        let msg = format!("Undefined variable {}", name);
                        return Err(Error::Runtime(msg, chunk.get_line(frame.ip + frame.slot)));
                    }
                }
                OpCode::DefineGlobal(opr) => {
                    let name = chunk.get_constant(opr as usize).to_string();

                    self.globals.insert(name, self.stack.pop().unwrap().clone());
                }
                OpCode::SetGlobal(opr) => {
                    let name = chunk.get_constant(opr as usize).to_string();

                    if self
                        .globals
                        .insert(name.clone(), self.stack.last().unwrap().clone())
                        .is_none()
                    {
                        self.globals.remove(&name);
                        let msg = format!("Undefined variable {}", name);
                        return Err(Error::Runtime(msg, chunk.get_line(frame.ip + frame.slot)));
                    }
                }
                OpCode::Equal => {
                    stack_operands!(self.stack, b, a);
                    self.stack.push(Value::Bool(a == b));
                }
                OpCode::Greater => {
                    stack_operands!(self.stack, b, a);
                    self.stack.push(Value::Bool(a > b));
                }
                OpCode::Less => {
                    stack_operands!(self.stack, b, a);
                    self.stack.push(Value::Bool(a < b));
                }
                OpCode::Add => {
                    stack_operands!(self.stack, b, a);
                    self.stack.push((a + b).unwrap());
                }
                OpCode::Subtract => {
                    stack_operands!(self.stack, b, a);
                    self.stack.push((a - b).unwrap());
                }
                OpCode::Multiply => {
                    stack_operands!(self.stack, b, a);
                    self.stack.push((a * b).unwrap());
                }
                OpCode::Divide => {
                    stack_operands!(self.stack, b, a);
                    self.stack.push((a / b).unwrap());
                }
                OpCode::Not => {
                    stack_operands!(self.stack, a);
                    self.stack.push((!a).unwrap());
                }
                OpCode::Negate => {
                    stack_operands!(self.stack, a);
                    self.stack.push((-a).unwrap());
                }
                OpCode::Print => {
                    stack_operands!(self.stack, a);
                    let mut a = a.to_string();

                    if cfg!(debug_trace_execution) {
                        println!("{}", "     -----Print-----".magenta());
                        a.insert_str(0, "     ");
                        a = a.replace('\n', "\n     ");
                    }

                    println!("{}", a);

                    if cfg!(debug_trace_execution) {
                        println!("{}", "     ---------------".magenta());
                    }
                }
                OpCode::Jump(offset) => {
                    //self.ip += offset - 1;
                    frame.ip += offset - 1;
                }
                OpCode::JumpIfFalse(offset) => {
                    if self.stack.last().unwrap().is_falsey() {
                        frame.ip += offset - 1;
                    }
                }
                OpCode::Loop(offset) => {
                    frame.ip -= offset + 1;
                }
                OpCode::Call(count) => {
                    if self.call_value(frame.slot + count, count).is_err() {
                        return Err(Error::Runtime(
                            "Call Failed".to_string(),
                            chunk.get_line(frame.ip + frame.slot),
                        ));
                    }

                    frame = self.frames.last_mut().unwrap();
                }
                OpCode::Return => {
                    return Ok(());
                }
            }

            frame.ip += 1;
        }
    }

    fn call_value(&mut self, callee: usize, arg_count: usize) -> Result<()> {
        match &self.stack[callee] {
            Value::Obj(object::Obj::Function(func)) => {
                return self.call(&func, arg_count);
            }
            _ => {}
        }
        Err(Error::Runtime("".to_string(), 0))
    }
}

impl Default for Vm<'_> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CallFrame<'a> {
    function: &'a Function,
    ip: usize,
    slot: usize,
}

impl<'a> CallFrame<'a> {
    pub fn new(function: &'a Function, slot: usize) -> CallFrame {
        CallFrame {
            function,
            ip: 0,
            slot,
        }
    }
}
