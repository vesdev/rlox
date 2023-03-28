use std::{fmt::Display, ops::Add, rc::Rc, string::String};

use super::chunk::Chunk;

#[derive(Clone, Debug)]
pub enum Obj {
    String(String),
    Function(Function),
}

impl Display for Obj {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Obj::String(v) => v.clone(),
            Obj::Function(v) => v.to_string(),
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
            (Obj::String(_), Obj::Function(_)) => None,
            (Obj::Function(_), Obj::String(_)) => None,
            (Obj::Function(_), Obj::Function(_)) => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Function {
    pub name: String,
    pub arity: usize,
    pub chunk: Chunk,
}

impl Function {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            arity: 0,
            chunk: Chunk::new(),
        }
    }
}

impl Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.name.is_empty() {
            write!(f, "<script>")
        } else {
            write!(f, "<fn {}>", self.name)
        }
    }
}

impl Default for Function {
    fn default() -> Self {
        Self {
            name: Default::default(),
            arity: Default::default(),
            chunk: Default::default(),
        }
    }
}
