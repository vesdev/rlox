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
        let closure_rc = Rc::new(Closure::new(Vec::new(), func_rc));
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
        let _exit = false;

        loop {
            let absolute_ip = frame.slot + frame.ip;
            let instruction: OpCode = chunk.get_op(frame.ip);

            if cfg!(trace_exec) {
                let mut out = String::new();
                disassemble_instruction(&mut out, chunk, frame.ip)
                    .map_err(|_| Error::Runtime("Could not disassemble".to_string(), 0))?;

                print!("{}>> ", out);
                if self.stack.len() > 5 {
                    print!(" ... ",);
                }
                for value in self.stack[self.stack.len().saturating_sub(5)..].iter() {
                    print!(" {}, ", value.to_string().replace('\n', "\\n").green());
                }
                println!();
            }

            match instruction {
                OpCode::Constant { constant } => {
                    let constant = chunk.get_constant(constant);
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
                OpCode::GetLocal { local } => {
                    self.stack.push(self.stack[frame.slot + local].clone());
                }
                OpCode::SetLocal { local } => {
                    self.stack[frame.slot + local] = self
                        .stack
                        .last()
                        .ok_or(Error::EmptyStack("OpCode::SetLocal".to_string()))?
                        .clone();
                }
                OpCode::GetGlobal { name } => {
                    let name = Self::identifier(chunk.get_constant(name));

                    if let Some(val) = self.globals.get(&name) {
                        self.stack.push(val.clone());
                    } else {
                        Self::error(
                            format!("Undefined variable {}", name),
                            frame.closure.function.chunk.get_line(absolute_ip),
                        )?
                    }
                }
                OpCode::DefineGlobal { name } => {
                    let name = Self::identifier(chunk.get_constant(name));

                    self.globals.insert(
                        name,
                        self.stack
                            .pop()
                            .ok_or(Error::EmptyStack("OpCode::DefineGlobal".to_string()))?
                            .clone(),
                    );
                }
                OpCode::SetGlobal { name } => {
                    if self
                        .globals
                        .insert(
                            Self::identifier(chunk.get_constant(name)),
                            self.stack
                                .last()
                                .ok_or(Error::EmptyStack("OpCode::SetGlobal".to_string()))?
                                .clone(),
                        )
                        .is_none()
                    {
                        Self::error(
                            format!(
                                "Undefined variable {}",
                                Self::identifier(chunk.get_constant(name))
                            ),
                            frame.closure.function.chunk.get_line(absolute_ip),
                        )?
                    }
                }
                OpCode::GetUpValue { upvalue } => {
                    self.stack
                        .push(frame.closure.upvalues[upvalue].borrow().clone());
                }
                OpCode::SetUpValue { upvalue } => {
                    *frame.closure.upvalues[upvalue].borrow_mut() = self
                        .stack
                        .last()
                        .ok_or(Error::EmptyStack("OpCode::SetUpValue".to_string()))?
                        .clone();
                }
                OpCode::GetProperty { prop_name } => match self.stack.last().cloned() {
                    Some(Value::Obj(Obj::Instance(instance))) => {
                        let name = Self::identifier(chunk.get_constant(prop_name));
                        if let Some(value) = instance.borrow().fields.get(&name) {
                            self.stack.pop();
                            self.stack.push(value.clone());
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
                },
                OpCode::SetProperty { prop_name } => {
                    stack_operands!("OpCode::SetProperty", self.stack, value, instance);

                    match instance {
                        Value::Obj(Obj::Instance(instance)) => {
                            instance.borrow_mut().fields.insert(
                                Self::identifier(chunk.get_constant(prop_name)),
                                value.clone(),
                            );
                        }
                        _ => Self::error(
                            "Only instances have properties.",
                            frame.closure.function.chunk.get_line(absolute_ip),
                        )?,
                    }

                    self.stack.push(value);
                }

                OpCode::GetSuper { name } => {
                    stack_operands!("OpCode::GetSuper", self.stack, superclass, receiver);

                    match (superclass, receiver) {
                        (
                            Value::Obj(Obj::Class(superclass)),
                            Value::Obj(Obj::Instance(receiver)),
                        ) => self.method(
                            superclass,
                            Self::identifier(chunk.get_constant(name)),
                            chunk,
                            absolute_ip,
                            Some(receiver),
                        ),
                        _ => {
                            Self::error("Super only works for a instance", chunk.get_line(frame.ip))
                        }
                    }?;
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
                OpCode::Jump { offset } => {
                    frame.ip += offset;
                    continue;
                }
                OpCode::JumpIfFalse { offset } => {
                    if self
                        .stack
                        .last()
                        .ok_or(Error::EmptyStack("OpCode::JumpIfFalse".to_string()))?
                        .is_falsey()
                    {
                        frame.ip += offset;
                        continue;
                    }
                }
                OpCode::Loop { offset } => {
                    frame.ip -= offset;
                    continue;
                }
                OpCode::Call { arg_count } => {
                    let len = self.frames.len();
                    frame.ip += 1;
                    let ip = frame.ip;
                    self.frames[len - 1] = frame;

                    let err = self.call_value(arg_count, ip);
                    frame = self
                        .frames
                        .last_mut()
                        .ok_or(Error::EmptyStack("OpCode::Call".to_string()))?
                        .clone();
                    chunk = &frame.closure.function.chunk;

                    err.map_err(|e| Error::Runtime(e, chunk.get_line(frame.ip)))?;
                    continue;
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
                        break;
                    } else {
                        for upvalue in self.open_upvalues.drain(..) {
                            *upvalue.1.borrow_mut() = self.stack.remove(frame.slot + upvalue.0);
                        }

                        self.stack.truncate(frame.slot);
                        self.stack.push(result);

                        frame = self
                            .frames
                            .last_mut()
                            .ok_or(Error::EmptyStack("OpCode::Return".to_string()))?
                            .clone();
                        if cfg!(trace_exec) {
                            println!(
                                "{}",
                                format!(
                                    "\n      Returned to: {}[{:04}]\n",
                                    frame.closure.function, frame.ip
                                )
                                .magenta()
                            );
                        }
                        chunk = &frame.closure.function.chunk;
                    }

                    continue;
                }
                OpCode::Invoke { method, arg_count } => {
                    let index = self.stack.len() - arg_count - 1;
                    if let Some(Value::Obj(Obj::Instance(receiver))) =
                        self.stack.get(index).cloned()
                    {
                        let name = Self::identifier(chunk.get_constant(method));
                        if let Some(method) = receiver.borrow().class.borrow().methods.get(&name) {
                            frame.ip += 1;
                            let len = self.frames.len();
                            self.frames[len - 1] = frame;
                            self.call(method.clone(), index);

                            frame = self
                                .frames
                                .last_mut()
                                .ok_or(Error::EmptyStack("OpCode::Call".to_string()))?
                                .clone();
                            chunk = &frame.closure.function.chunk;
                            continue;
                        }
                    } else {
                        Self::error(
                            "Invoke only on instances",
                            frame.closure.function.chunk.get_line(absolute_ip),
                        )?;
                    }
                }
                OpCode::SuperInvoke { method, arg_count } => {
                    stack_operands!("OpCode::SuperInvoke", self.stack, superclass);

                    if let Value::Obj(Obj::Class(superclass)) = superclass {
                        let name = Self::identifier(chunk.get_constant(method));
                        if let Some(method) = superclass.borrow().methods.get(&name) {
                            frame.ip += 1;
                            let len = self.frames.len();
                            self.frames[len - 1] = frame;
                            self.call(method.clone(), self.stack.len() - arg_count - 1);

                            frame = self
                                .frames
                                .last_mut()
                                .ok_or(Error::EmptyStack("OpCode::SuperInvoke".to_string()))?
                                .clone();
                            chunk = &frame.closure.function.chunk;
                            continue;
                        }
                    } else {
                        Self::error(
                            "Invoke only on instances",
                            frame.closure.function.chunk.get_line(absolute_ip),
                        )?;
                    }
                }
                OpCode::Closure { func } => {
                    if let Value::Obj(Obj::Fun(func)) = chunk.get_constant(func) {
                        let closure =
                            Closure::new(self.open_upvalues(frame.closure.clone(), &func), func);
                        self.stack.push(Value::Obj(Obj::Closure(Rc::new(closure))));
                    }
                }
                OpCode::Class { name } => {
                    self.stack
                        .push(Value::Obj(Obj::Class(Class::new(Self::identifier(
                            chunk.get_constant(name),
                        )))))
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
                OpCode::Method { name } => {
                    stack_operands!("OpCode::Method", self.stack, method);

                    if let Some(Value::Obj(Obj::Class(class))) = self.stack.last_mut().cloned() {
                        if let Value::Obj(Obj::Closure(method)) = method {
                            class
                                .borrow_mut()
                                .methods
                                .insert(Self::identifier(chunk.get_constant(name)), method);
                        }
                    }
                }
            }

            frame.ip += 1;
        }

        Ok(())
    }

    fn identifier(value: Value) -> String {
        match value {
            Value::String(v) => v,
            _ => String::default(),
        }
    }

    fn error(message: impl Into<String>, line: usize) -> Result<()> {
        Err(Error::Runtime(message.into(), line))
    }

    fn call(&mut self, method: Rc<Closure>, slot: usize) {
        if cfg!(trace_exec) {
            println!(
                "{}",
                format!("\n      Called: {}()\n", method.function).magenta()
            );
        }

        self.frames.push(CallFrame::new(method, slot));
    }

    fn call_method(&mut self, method: Rc<Closure>, slot: usize, receiver: Value) {
        self.stack[slot] = receiver;
        self.call(method, slot)
    }

    fn call_value(&mut self, arg_count: usize, _ip: usize) -> Result<(), String> {
        let index = self.stack.len() - arg_count - 1;
        let callee = &self.stack[index];

        match callee {
            Value::Obj(object::Obj::BoundMethod(bound)) => {
                if bound.method.function.arity != arg_count {
                    return Err(format!(
                        "Expected {} arguments but got {}.",
                        bound.method.function.arity, arg_count
                    ));
                }
                let this = Value::Obj(Obj::Instance(bound.receiver.clone()));
                let method = bound.method.clone();

                self.call_method(method, index, this);
                Ok(())
            }
            Value::Obj(object::Obj::Class(class)) => {
                let class = class.clone();
                let instance = Instance::new(class.clone());
                self.stack[index] = Value::Obj(Obj::Instance(instance));

                if let Some(init) = class.borrow().methods.get("init") {
                    self.call(init.clone(), index);
                }

                Ok(())
            }
            Value::Obj(object::Obj::Closure(closure)) => {
                if closure.function.arity != arg_count {
                    return Err(format!(
                        "Expected {} arguments but got {}.",
                        closure.function.arity, arg_count
                    ));
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
            _ => Err("Call Failed".to_string()),
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

    fn open_upvalues(
        &mut self,
        closure: Rc<Closure>,
        func: &Rc<FunDescriptor>,
    ) -> Vec<Rc<RefCell<Value>>> {
        func.upvalues
            .iter()
            .cloned()
            .map(|upvalue_descriptor| {
                if upvalue_descriptor.is_local {
                    if let Some(open_upvalue) = self.open_upvalues.get(&upvalue_descriptor.index) {
                        open_upvalue.clone()
                    } else {
                        let upvalue = Rc::new(RefCell::new(Value::Nil));
                        self.open_upvalues.insert(
                            upvalue_descriptor.index,
                            //sentinel value until upvalue closed
                            //so we can share the reference to other capturing closures before it exists
                            upvalue.clone(),
                        );
                        upvalue
                    }
                } else {
                    closure.upvalues[upvalue_descriptor.index].clone()
                }
            })
            .collect()
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
