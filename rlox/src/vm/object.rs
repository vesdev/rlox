use std::{fmt::Display, ops::Add, rc::Rc, string::String};

use super::{chunk::Chunk, value::Value};
use crate::error::Error;

#[derive(Clone)]
pub enum Obj {
    String(String),
    Fun(Rc<Fun>),
    NativeFun(Rc<Box<dyn NativeFun>>),
}

impl Display for Obj {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Obj::Fun(v) => v.to_string(),
            Obj::NativeFun(v) => v.to_string(),
            Obj::String(v) => v.to_string(),
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
    type Output = Option<Self>;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Obj::String(l0), Obj::String(r0)) => Some(Obj::String(l0 + &r0)),
            _ => None,
        }
    }
}

#[derive(Clone, Default)]
pub struct Fun {
    pub name: String,
    pub arity: usize,
    pub chunk: Chunk,
}

impl Fun {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            arity: 0,
            chunk: Chunk::new(),
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
