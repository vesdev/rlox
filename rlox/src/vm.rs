pub mod chunk;
pub mod object;
pub mod opcode;
pub mod value;

use crate::error::*;
use colored::Colorize;
use std::{collections::HashMap, ops::Deref, rc::Rc, result};

use crate::vm::{
    chunk::{disassemble_instruction, Chunk},
    opcode::OpCode,
    value::Value,
};

use self::object::{Function, Native, NativeFun, NativeFunction, Obj};

pub type Result<T, E = Error> = std::result::Result<T, E>;

macro_rules! stack_operands {
    ( $vec:expr $(, $name:ident)+ ) => {
        $(
            let $name = $vec.pop().unwrap();
        )*
    };
}

pub struct Vm {
    stack: Vec<Value>,
    globals: HashMap<String, Value>,
    frames: Vec<CallFrame>,
}

impl Vm {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            globals: HashMap::new(),
            frames: Vec::new(),
        }
    }

    pub fn call(&mut self, function: Function, arg_count: usize) -> Result<()> {
        let rc = Rc::new(function);
        self.frames
            .push(CallFrame::new(rc.clone(), self.stack.len() - arg_count));
        self.stack.push(Value::Obj(Obj::Fun(rc)));

        self.run()
    }

    fn run(&mut self) -> Result<()> {
        let mut frame = self.frames.last_mut().unwrap().clone();
        let mut chunk = &frame.function.chunk;
        let mut exit = false;

        while !exit {
            let absolute_ip = frame.slot + frame.ip;
            let instruction: OpCode = chunk.get_op(frame.ip);
            let mut ip_offset = 1;

            if cfg!(debug_trace_execution) {
                let mut out = String::new();
                disassemble_instruction(&mut out, chunk, frame.ip).unwrap();

                print!("{}|", out);

                for value in &self.stack {
                    print!(" {}, ", value.to_string().replace('\n', "\\n").green());
                }
                println!("|");
            }

            match instruction {
                OpCode::Constant(opr) => {
                    let constant = chunk.get_constant(opr as usize);
                    self.stack.push(constant);
                }
                OpCode::Nil => {
                    self.stack.push(Value::Nil);
                }
                OpCode::True => {
                    self.stack.push(Value::Bool(true));
                }
                OpCode::False => {
                    self.stack.push(Value::Bool(false));
                }
                OpCode::Pop => {
                    self.stack.pop();
                }
                OpCode::GetLocal(opr) => {
                    self.stack
                        .push(self.stack[opr as usize + frame.slot].clone());
                }
                OpCode::SetLocal(opr) => {
                    self.stack[opr as usize + frame.slot] = self.stack.last().unwrap().clone();
                }
                OpCode::GetGlobal(opr) => {
                    let name = chunk.get_constant(opr as usize).to_string();

                    if let Some(val) = self.globals.get(&name) {
                        self.stack.push(val.clone());
                    } else {
                        let msg = format!("Undefined variable {}", name);
                        return Err(Error::Runtime(msg, chunk.get_line(absolute_ip)));
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
                        return Err(Error::Runtime(msg, chunk.get_line(absolute_ip)));
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
                    frame.ip += offset;
                    ip_offset = 0;
                }
                OpCode::JumpIfFalse(offset) => {
                    if self.stack.last().unwrap().is_falsey() {
                        frame.ip += offset;
                        ip_offset = 0;
                    }
                }
                OpCode::Loop(offset) => {
                    frame.ip -= offset;
                    ip_offset = 0;
                }
                OpCode::Call(arg_count) => {
                    let len = self.frames.len();
                    frame.ip += 1;
                    self.frames[len - 1] = frame.clone();

                    self.call_value(arg_count, chunk, frame.ip)?;
                    frame = self.frames.last_mut().unwrap().clone();
                    chunk = &frame.function.chunk;
                    ip_offset = 0;
                }
                OpCode::Return => {
                    stack_operands!(self.stack, result);

                    self.frames.pop();
                    if self.frames.is_empty() {
                        self.stack.pop();
                        exit = true;
                    } else {
                        self.stack.truncate(frame.slot);
                        self.stack.push(result);

                        frame = self.frames.last().unwrap().clone();
                        if cfg!(debug_trace_execution) {
                            println!("     Returned to: {}[{:04}]", frame.function, frame.ip);
                        }
                        chunk = &frame.function.chunk;
                    }

                    ip_offset = 0;
                }
            }

            frame.ip += ip_offset;
        }

        Ok(())
    }

    fn call_value(&mut self, arg_count: usize, chunk: &Chunk, ip: usize) -> Result<()> {
        let index = self.stack.len() - arg_count - 1;
        let callee = &self.stack[index];

        match callee {
            Value::Obj(object::Obj::Fun(func)) => {
                if func.arity != arg_count {
                    return Err(Error::Runtime(
                        format!("Expected {} arguments but got {}.", func.arity, arg_count),
                        func.chunk.get_line(0),
                    ));
                }

                self.frames.push(CallFrame::new(func.clone(), index));

                if cfg!(debug_trace_execution) {
                    println!("     Called: {}()", func);
                }
                Ok(())
            }
            Value::Obj(object::Obj::NativeFun(func)) => {
                let result = func.call(&self.stack[index + 1..])?;
                self.stack.truncate(index);
                self.stack.push(result);
                Ok(())
            }
            _ => Err(Error::Runtime(
                "Call Failed".to_string(),
                chunk.get_line(ip),
            )),
        }
    }

    pub fn define_native(
        &mut self,
        name: impl Into<String>,
        function: Box<dyn NativeFun>,
    ) -> &mut Self {
        self.globals
            .insert(name.into(), Value::Obj(Obj::NativeFun(Rc::new(function))));
        self
    }
}

impl Default for Vm {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct CallFrame {
    function: Rc<Function>,
    ip: usize,
    slot: usize,
}

impl CallFrame {
    pub fn new(function: Rc<Function>, slot: usize) -> CallFrame {
        CallFrame {
            function,
            ip: 0,
            slot,
        }
    }
}
