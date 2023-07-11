use std::{
    cell::RefCell,
    collections::HashMap,
    fmt::{Debug, Display},
    ops::Add,
    rc::Rc,
    string::String,
};

use super::{chunk::Chunk, value::Value};
use crate::error::*;

#[derive(Clone)]
pub enum Obj {
    Fun(Rc<FunDescriptor>),
    Closure(Rc<Closure>),
    NativeFun(Rc<Box<dyn NativeFun>>),
    Class(Rc<RefCell<Class>>),
    Instance(Rc<RefCell<Instance>>),
    BoundMethod(Rc<BoundMethod>),
}

impl Debug for Obj {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Display for Obj {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Obj::Fun(v) => v.to_string(),
            Obj::NativeFun(v) => v.to_string(),
            Obj::Closure(v) => v.to_string(),
            Obj::Class(v) => v.borrow().to_string(),
            Obj::Instance(v) => v.borrow().to_string(),
            Obj::BoundMethod(v) => v.to_string(),
        };
        write!(f, "{}", s)
    }
}

impl PartialEq for Obj {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl Add for Obj {
    type Output = Result<Self>;

    fn add(self, _rhs: Self) -> Self::Output {
        Err(Error::Arithmetic("'+' Invalid operands".into()))
    }
}

#[derive(Clone, Default)]
pub struct FunDescriptor {
    pub name: String,
    pub arity: usize,
    pub chunk: Chunk,
    pub upvalues: Vec<UpValueDescriptor>,
}

impl FunDescriptor {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            arity: 0,
            chunk: Chunk::new(),
            upvalues: Vec::new(),
        }
    }
}

impl Display for FunDescriptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.name.is_empty() {
            write!(f, "<script>")
        } else {
            write!(f, "<fn {}>", self.name)
        }
    }
}

pub type NativeFunction = &'static dyn Fn(&[Value]) -> Value;

#[derive(Clone, Debug)]

pub struct UpValueDescriptor {
    pub index: usize,
    pub is_local: bool,
}

#[derive(Clone)]
pub struct Closure {
    pub function: Rc<FunDescriptor>,
    pub upvalues: Vec<Rc<RefCell<Value>>>,
}

impl Display for Closure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<closure {}>", self.function)
    }
}

impl Closure {
    pub fn new(upvalues: Vec<Rc<RefCell<Value>>>, function: Rc<FunDescriptor>) -> Self {
        Self { function, upvalues }
    }
}

#[derive(Clone)]
pub struct Native {
    pub function: NativeFunction,
}

impl Native {
    pub fn new(function: NativeFunction) -> Self {
        Self { function }
    }
}
pub trait NativeFun {
    fn call(&self, args: &[Value]) -> Result<Value, String>;
}

impl Display for Box<dyn NativeFun> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<native fn>")
    }
}

#[derive(Clone)]
pub struct Class {
    pub name: String,
    pub methods: HashMap<String, Rc<Closure>>,
}

impl Class {
    pub fn new(name: String) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            name,
            methods: HashMap::new(),
        }))
    }
}

impl Display for Class {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<class {}>", self.name)
    }
}

#[derive(Clone)]
pub struct Instance {
    pub class: Rc<RefCell<Class>>,
    pub fields: HashMap<String, Value>,
}

impl Instance {
    pub fn new(class: Rc<RefCell<Class>>) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            class,
            fields: HashMap::new(),
        }))
    }
}

impl Display for Instance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<instance of {}>", self.class.borrow().name)
    }
}

#[derive(Clone)]
pub struct BoundMethod {
    pub receiver: Rc<RefCell<Instance>>,
    pub method: Rc<Closure>,
}

impl BoundMethod {
    pub fn new(receiver: Rc<RefCell<Instance>>, method: Rc<Closure>) -> Rc<Self> {
        Rc::new(Self { receiver, method })
    }
}

impl Display for BoundMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<bound method {}>", self.method.function)
    }
}
