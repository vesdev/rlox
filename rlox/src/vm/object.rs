use std::{fmt::Display, ops::Add, rc::Rc, string::String};

use super::{chunk::Chunk, value::Value};
use crate::error::*;

#[derive(Clone)]
pub enum Obj {
    String(String),
    Fun(Rc<Fun>),
    Closure(Rc<Closure>),
    NativeFun(Rc<Box<dyn NativeFun>>),
}

impl Display for Obj {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Obj::Fun(v) => v.to_string(),
            Obj::NativeFun(v) => v.to_string(),
            Obj::String(v) => v.to_string(),
            Obj::Closure(v) => v.to_string(),
        };
        write!(f, "{}", s)
    }
}

impl PartialEq for Obj {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::String(l0), Self::String(r0)) => l0 == r0,
            _ => false,
        }
    }
}

impl Add for Obj {
    type Output = Result<Self>;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Obj::String(l0), Obj::String(r0)) => Ok(Obj::String(l0 + &r0)),
            _ => Err(Error::Arithmetic("'+' Invalid operands".into())),
        }
    }
}

#[derive(Clone, Default)]
pub struct Fun {
    pub name: String,
    pub arity: usize,
    pub chunk: Chunk,
    pub upvalue_count: usize,
    pub upvalues: Vec<UpValue>,
}

impl Fun {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            arity: 0,
            chunk: Chunk::new(),
            upvalue_count: 0,
            upvalues: Vec::new(),
        }
    }
}

impl Display for Fun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.name.is_empty() {
            write!(f, "<script>")
        } else {
            write!(f, "<fn {}>", self.name)
        }
    }
}

pub type NativeFunction = &'static dyn Fn(&[Value]) -> Value;

#[derive(Clone)]
pub struct Closure {
    pub function: Rc<Fun>,
}

impl Display for Closure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<closure {}>", self.function)
    }
}

impl Closure {
    pub fn new(function: Rc<Fun>) -> Self {
        Self { function }
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
    fn call(&self, args: &[Value]) -> Result<Value, Error>;
}

impl Display for Box<dyn NativeFun> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<native fn>")
    }
}

#[derive(Clone)]

pub struct UpValue {
    pub index: usize,
    pub is_local: bool,
}
