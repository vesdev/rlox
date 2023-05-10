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

use self::object::{BoundMethod, Class, Closure, FunDescriptor, Instance, NativeFun, Obj};

pub type Result<T, E = Error> = std::result::Result<T, E>;

macro_rules! stack_operands {
    ( $opcode:literal, $vec:expr $(, $name:ident)+ ) => {
        $(
            let $name = $vec.pop().ok_or(Error::EmptyStack($opcode.to_string()))?;
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
        let mut frame = self
            .frames
            .last_mut()
            .ok_or(Error::EmptyStack("rlox vm".to_string()))?
            .clone();
        let mut chunk = &frame.closure.function.chunk;
        let mut exit = false;

        while !exit {
            let absolute_ip = frame.slot + frame.ip;
            let instruction: OpCode = chunk.get_op(frame.ip);
            let mut ip_offset = 1;

            if cfg!(trace_exec) {
                let mut out = String::new();
                disassemble_instruction(&mut out, chunk, frame.ip)
                    .map_err(|_| Error::Runtime("Could not disassemble".to_string(), 0))?;

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
                OpCode::GetLocal(offset) => {
                    self.stack.push(self.stack[frame.slot + offset].clone());
                }
                OpCode::SetLocal(offset) => {
                    self.stack[frame.slot + offset] = self
                        .stack
                        .last()
                        .ok_or(Error::EmptyStack("OpCode::SetLocal".to_string()))?
                        .clone();
                }
                OpCode::GetGlobal(offset) => {
                    let name = chunk.get_constant(offset).to_string();

                    if let Some(val) = self.globals.get(&name) {
                        self.stack.push(val.clone());
                    } else {
                        Self::error(
                            format!("Undefined variable {}", name),
                            frame.closure.function.chunk.get_line(absolute_ip),
                        )?
                    }
                }
                OpCode::DefineGlobal(offset) => {
                    let name = chunk.get_constant(offset).to_string();

                    self.globals.insert(
                        name,
                        self.stack
                            .pop()
                            .ok_or(Error::EmptyStack("OpCode::DefineGlobal".to_string()))?
                            .clone(),
                    );
                }
                OpCode::SetGlobal(offset) => {
                    let name = chunk.get_constant(offset).to_string();

                    if self
                        .globals
                        .insert(
                            name.clone(),
                            self.stack
                                .last()
                                .ok_or(Error::EmptyStack("OpCode::SetGlobal".to_string()))?
                                .clone(),
                        )
                        .is_none()
                    {
                        Self::error(
                            format!("Undefined variable {}", name),
                            frame.closure.function.chunk.get_line(absolute_ip),
                        )?
                    }
                }
                OpCode::GetUpValue(offset) => {
                    self.stack
                        .push(frame.closure.upvalues[offset].borrow().clone());
                }
                OpCode::SetUpValue(offset) => {
                    *frame.closure.upvalues[offset].borrow_mut() = self
                        .stack
                        .last()
                        .ok_or(Error::EmptyStack("OpCode::SetUpValue".to_string()))?
                        .clone();
                }
                OpCode::GetProperty(offset) => {
                    let name = chunk.get_constant(offset).to_string();
                    let obj = self.stack.last().cloned();
                    match obj {
                        Some(Value::Obj(Obj::Instance(instance))) => {
                            if let Some(value) = instance.borrow().fields.get(&name) {
                                self.stack.pop();
                                self.stack.push(value.clone())
                            } else {
                                self.method(
                                    instance.borrow().class.clone(),
                                    name,
                                    chunk,
                                    absolute_ip,
                                    Some(instance.clone()),
                                )?;
                            }
                        }
                        _ => Self::error(
                            "Only instances have properties.",
                            frame.closure.function.chunk.get_line(absolute_ip),
                        )?,
                    }
                }
                OpCode::SetProperty(offset) => {
                    stack_operands!("OpCode::SetProperty", self.stack, value, instance);

                    let name = chunk.get_constant(offset).to_string();

                    if let Value::Obj(Obj::Instance(instance)) = instance {
                        instance.borrow_mut().fields.insert(name, value.clone());
                    } else {
                        Self::error(
                            "Only instances have properties.",
                            frame.closure.function.chunk.get_line(absolute_ip),
                        )?
                    }

                    self.stack.push(value);
                }
                OpCode::GetSuper(offset) => {
                    stack_operands!("OpCode::GetSuper", self.stack, superclass, receiver);
                    let name = chunk.get_constant(offset).to_string();

                    if let (Value::Obj(Obj::Class(class)), Value::Obj(Obj::Instance(receiver))) =
                        (superclass, receiver)
                    {
                        self.method(class, name, chunk, absolute_ip, Some(receiver))?;
                    } else {
                        Self::error("Super only works for a instance", chunk.get_line(frame.ip))?;
                    }
                }
                OpCode::Equal => {
                    stack_operands!("OpCode::Equal", self.stack, b, a);
                    self.stack.push(Value::Bool(a == b));
                }
                OpCode::Greater => {
                    stack_operands!("OpCode::Greater", self.stack, b, a);
                    self.stack.push(Value::Bool(a > b));
                }
                OpCode::Less => {
                    stack_operands!("OpCode::Less", self.stack, b, a);
                    self.stack.push(Value::Bool(a < b));
                }
                OpCode::Add => {
                    stack_operands!("OpCode::Add", self.stack, b, a);
                    self.stack.push((a + b)?);
                }
                OpCode::Subtract => {
                    stack_operands!("OpCode::Subtract", self.stack, b, a);
                    self.stack.push((a - b)?);
                }
                OpCode::Multiply => {
                    stack_operands!("OpCode::Multiply", self.stack, b, a);
                    self.stack.push((a * b)?);
                }
                OpCode::Divide => {
                    stack_operands!("OpCode::Divide", self.stack, b, a);
                    self.stack.push((a / b)?);
                }
                OpCode::Not => {
                    stack_operands!("OpCode::Not", self.stack, a);
                    self.stack.push((!a)?);
                }
                OpCode::Negate => {
                    stack_operands!("OpCode::Negate", self.stack, a);
                    self.stack.push((-a)?);
                }
                OpCode::Print => {
                    stack_operands!("OpCode::Print", self.stack, a);
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
                    if self
                        .stack
                        .last()
                        .ok_or(Error::EmptyStack("OpCode::JumpIfFalse".to_string()))?
                        .is_falsey()
                    {
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
                    frame = self
                        .frames
                        .last_mut()
                        .ok_or(Error::EmptyStack("OpCode::Call".to_string()))?
                        .clone();
                    chunk = &frame.closure.function.chunk;
                    ip_offset = 0;
                }
                OpCode::CloseUpValue => {
                    stack_operands!("OpCode::CloseUpValue", self.open_upvalues, upvalue);

                    *upvalue.1.borrow_mut() = self.stack[frame.slot + upvalue.0].clone();
                }
                OpCode::Return => {
                    stack_operands!("OpCode::Return", self.stack, result);

                    self.frames.pop();
                    if self.frames.is_empty() {
                        self.stack.pop();
                        exit = true;
                    } else {
                        for upvalue in self.open_upvalues.drain(..).rev() {
                            *upvalue.1.borrow_mut() = self.stack.remove(frame.slot + upvalue.0);
                        }

                        self.open_upvalues.clear();

                        self.stack.truncate(frame.slot);
                        self.stack.push(result);

                        frame = self
                            .frames
                            .last_mut()
                            .ok_or(Error::EmptyStack("OpCode::Return".to_string()))?
                            .clone();
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

                        let closure = Closure::new(func, closure_upvalues);
                        self.stack.push(Value::Obj(Obj::Closure(Rc::new(closure))));
                    }
                }
                OpCode::Class(offset) => {
                    let name = chunk.get_constant(offset).to_string();
                    self.stack.push(Value::Obj(Obj::Class(Class::new(name))))
                }
                OpCode::Inerhit => {
                    stack_operands!("OpCode::Inerhit", self.stack, subclass);
                    let superclass = self
                        .stack
                        .last_mut()
                        .ok_or(Error::EmptyStack("OpCode::Inerhit".to_string()))?;
                    if let (Value::Obj(Obj::Class(subclass)), Value::Obj(Obj::Class(superclass))) =
                        (subclass, superclass)
                    {
                        subclass
                            .borrow_mut()
                            .methods
                            .extend(superclass.borrow_mut().methods.clone());
                    } else {
                        Self::error(
                            "Superclass must be a class.",
                            frame.closure.function.chunk.get_line(absolute_ip),
                        )?;
                    }
                }
                OpCode::Method(offset) => {
                    stack_operands!("OpCode::Method", self.stack, method);
                    let name = chunk.get_constant(offset).to_string();

                    if let Some(Value::Obj(Obj::Class(class))) = self.stack.last_mut().cloned() {
                        if let Value::Obj(Obj::Closure(method)) = method {
                            class.borrow_mut().methods.insert(name, method);
                        }
                    }
                }
            }

            frame.ip += ip_offset;
        }

        Ok(())
    }

    fn error(message: impl Into<String>, line: usize) -> Result<()> {
        Err(Error::Runtime(message.into(), line))
    }

    fn call(&mut self, method: Rc<Closure>, slot: usize) {
        if cfg!(trace_exec) {
            println!("     Called: {}()", method.function);
        }

        self.frames.push(CallFrame::new(method, slot));
    }

    fn call_value(&mut self, arg_count: usize, chunk: &Chunk, ip: usize) -> Result<()> {
        let index = self.stack.len() - arg_count - 1;
        let callee = &self.stack[index];

        match callee {
            Value::Obj(object::Obj::BoundMethod(bound)) => {
                if bound.method.function.arity != arg_count {
                    return Err(Error::Runtime(
                        format!(
                            "Expected {} arguments but got {}.",
                            bound.method.function.arity, arg_count
                        ),
                        bound.method.function.chunk.get_line(0),
                    ));
                }
                let this = Value::Obj(Obj::Instance(bound.receiver.clone()));
                let method = bound.method.clone();

                self.stack[index] = this;
                self.call(method, index);

                Ok(())
            }
            Value::Obj(object::Obj::Class(class)) => {
                let class = class.clone();
                let instance = Instance::new(class.clone());
                let stack_len = self.stack.len();
                self.stack[stack_len - arg_count - 1] = Value::Obj(Obj::Instance(instance));

                if let Some(init) = class.borrow().methods.get("init") {
                    self.call(init.clone(), index);
                }

                Ok(())
            }
            Value::Obj(object::Obj::Closure(closure)) => {
                if closure.function.arity != arg_count {
                    Self::error(
                        format!(
                            "Expected {} arguments but got {}.",
                            closure.function.arity, arg_count
                        ),
                        closure.function.chunk.get_line(0),
                    )?
                }

                self.call(closure.clone(), index);
                Ok(())
            }
            Value::Obj(object::Obj::NativeFun(func)) => {
                let result = func.call(&self.stack[index..])?;
                self.stack.truncate(index);
                self.stack.push(result);
                Ok(())
            }
            _ => Self::error("Call Failed", chunk.get_line(ip)),
        }
    }

    fn method(
        &mut self,
        class: Rc<RefCell<Class>>,
        name: String,
        chunk: &Chunk,
        ip: usize,
        receiver: Option<Rc<RefCell<Instance>>>,
    ) -> Result<()> {
        if let Some(method) = class.borrow().methods.get(&name) {
            self.stack.pop();

            if let Some(receiver) = receiver {
                self.stack
                    .push(Value::Obj(Obj::BoundMethod(BoundMethod::new(
                        receiver,
                        method.clone(),
                    ))))
            } else {
                self.stack.push(Value::Obj(Obj::Closure(method.clone())));
            }
        } else {
            Self::error(format!("Undefined property {}.", name), chunk.get_line(ip))?;
        }

        Ok(())
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
