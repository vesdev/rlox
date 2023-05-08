pub mod chunk;
pub mod object;
pub mod opcode;
pub mod value;

use crate::error::*;
use colored::Colorize;
use indexmap::IndexMap;

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::vm::{
    chunk::{disassemble_instruction, Chunk},
    opcode::OpCode,
    value::Value,
};

use self::object::{Closure, FunDescriptor, NativeFun, Obj};

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
    open_upvalues: IndexMap<usize, Rc<RefCell<Value>>>,
}

impl Vm {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            globals: HashMap::new(),
            frames: Vec::new(),
            open_upvalues: IndexMap::new(),
        }
    }

    pub fn execute(&mut self, function: FunDescriptor) -> Result<()> {
        let func_rc = Rc::new(function);
        let closure_rc = Rc::new(Closure::new(func_rc, Vec::new()));
        self.frames.push(CallFrame::new(closure_rc.clone(), 0));
        self.stack.push(Value::Obj(Obj::Closure(closure_rc)));

        self.run()
    }

    fn run(&mut self) -> Result<()> {
        let mut frame = self.frames.last_mut().unwrap().clone();
        let mut chunk = &frame.closure.function.chunk;
        let mut exit = false;

        while !exit {
            let absolute_ip = frame.slot + frame.ip;
            let instruction: OpCode = chunk.get_op(frame.ip);
            let mut ip_offset = 1;

            if cfg!(trace_exec) {
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
                    let constant = chunk.get_constant(opr);
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
                    self.stack.push(self.stack[frame.slot + opr].clone());
                }
                OpCode::SetLocal(opr) => {
                    self.stack[frame.slot + opr] = self.stack.last().unwrap().clone();
                }
                OpCode::GetGlobal(opr) => {
                    let name = chunk.get_constant(opr).to_string();

                    if let Some(val) = self.globals.get(&name) {
                        self.stack.push(val.clone());
                    } else {
                        let msg = format!("Undefined variable {}", name);
                        return Err(Error::Runtime(msg, chunk.get_line(absolute_ip)));
                    }
                }
                OpCode::DefineGlobal(opr) => {
                    let name = chunk.get_constant(opr).to_string();

                    self.globals.insert(name, self.stack.pop().unwrap().clone());
                }
                OpCode::SetGlobal(opr) => {
                    let name = chunk.get_constant(opr).to_string();

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
                OpCode::GetUpValue(slot) => {
                    self.stack
                        .push(frame.closure.upvalues[slot].borrow().clone());
                }
                OpCode::SetUpValue(slot) => {
                    *frame.closure.upvalues[slot].borrow_mut() = self.stack.last().unwrap().clone();
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
                    self.stack.push((a + b)?);
                }
                OpCode::Subtract => {
                    stack_operands!(self.stack, b, a);
                    self.stack.push((a - b)?);
                }
                OpCode::Multiply => {
                    stack_operands!(self.stack, b, a);
                    self.stack.push((a * b)?);
                }
                OpCode::Divide => {
                    stack_operands!(self.stack, b, a);
                    self.stack.push((a / b)?);
                }
                OpCode::Not => {
                    stack_operands!(self.stack, a);
                    self.stack.push((!a)?);
                }
                OpCode::Negate => {
                    stack_operands!(self.stack, a);
                    self.stack.push((-a)?);
                }
                OpCode::Print => {
                    stack_operands!(self.stack, a);
                    let mut a = a.to_string();

                    if cfg!(trace_exec) {
                        println!("{}", "     -----Print-----".magenta());
                        a.insert_str(0, "     ");
                        a = a.replace('\n', "\n     ");
                    }

                    println!("{}", a);

                    if cfg!(trace_exec) {
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
                    chunk = &frame.closure.function.chunk;
                    ip_offset = 0;
                }
                OpCode::CloseUpValue => {
                    stack_operands!(self.open_upvalues, upvalue);

                    *upvalue.1.borrow_mut() = self.stack[frame.slot + upvalue.0].clone();
                }
                OpCode::Return => {
                    stack_operands!(self.stack, result);

                    self.frames.pop();
                    if self.frames.is_empty() {
                        self.stack.pop();
                        exit = true;
                    } else {
                        for upvalue in self.open_upvalues.drain(..).rev() {
                            *upvalue.1.borrow_mut() = self.stack.remove(frame.slot + upvalue.0);
                        }

                        self.open_upvalues.clear();

                        self.stack.truncate(frame.slot - 1);
                        self.stack.push(result);

                        frame = self.frames.last().unwrap().clone();
                        if cfg!(trace_exec) {
                            println!(
                                "     Returned to: {}[{:04}]",
                                frame.closure.function, frame.ip
                            );
                        }
                        chunk = &frame.closure.function.chunk;
                    }

                    ip_offset = 0;
                }
                OpCode::Closure(offset) => {
                    let func = chunk.get_constant(offset);
                    if let Value::Obj(Obj::Fun(func)) = func {
                        let mut closure_upvalues = Vec::new();
                        for upvalue_descriptor in &func.upvalues {
                            if upvalue_descriptor.is_local {
                                let open_upvalue =
                                    self.open_upvalues.get(&upvalue_descriptor.index);
                                if let Some(open_upvalue) = open_upvalue {
                                    closure_upvalues.push(open_upvalue.clone());
                                } else {
                                    let upvalue = Rc::new(RefCell::new(Value::Nil));
                                    self.open_upvalues.insert(
                                        upvalue_descriptor.index,
                                        //sentinel value until upvalue closed
                                        //so we can share the reference to other capturing closures before it exists
                                        upvalue.clone(),
                                    );
                                    closure_upvalues.push(upvalue);
                                }
                            } else {
                                closure_upvalues
                                    .push(frame.closure.upvalues[upvalue_descriptor.index].clone());
                            }
                        }
                        println!("{:?}", func.upvalues);
                        println!("{:?}", self.open_upvalues);
                        let closure = Closure::new(func, closure_upvalues);
                        self.stack.push(Value::Obj(Obj::Closure(Rc::new(closure))));
                    }
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
            Value::Obj(object::Obj::Closure(closure)) => {
                if closure.function.arity != arg_count {
                    return Err(Error::Runtime(
                        format!(
                            "Expected {} arguments but got {}.",
                            closure.function.arity, arg_count
                        ),
                        closure.function.chunk.get_line(0),
                    ));
                }

                self.frames.push(CallFrame::new(closure.clone(), index + 1));

                if cfg!(trace_exec) {
                    println!("     Called: {}()", closure.function);
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
    closure: Rc<Closure>,
    ip: usize,
    slot: usize,
}

impl CallFrame {
    pub fn new(closure: Rc<Closure>, slot: usize) -> CallFrame {
        CallFrame {
            closure,
            ip: 0,
            slot,
        }
    }
}
