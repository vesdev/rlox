use std::{
    fmt::Display,
    ops::{Add, Div, Mul, Neg, Not, Sub},
};

use super::object::*;

use crate::error::*;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Number(f64),
    Nil,
    Bool(bool),
    String(String),
    Obj(Obj),
}

impl Value {
    pub fn is_falsey(&self) -> bool {
        match self {
            Value::Number(n) => n < &1.0,
            Value::Nil => false,
            Value::Bool(n) => n == &false,
            Value::Obj(_) => false,
            Value::String(_) => false,
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Value::String(v) = self {
            return f.write_str(v);
        }
        f.write_str(&match self {
            Value::Number(v) => v.to_string(),
            Value::Nil => "Nil".to_string(),
            Value::Bool(v) => v.to_string(),
            Value::Obj(v) => v.to_string(),
            _ => "<undefined>".to_string(),
        })
    }
}

impl Neg for Value {
    type Output = Result<Value>;

    fn neg(self) -> Self::Output {
        match self {
            Self::Number(v) => Ok(Self::Number(-v)),
            _ => Err(Error::Arithmetic("'-'(neg) Invalid operands".into())),
        }
    }
}

impl Add for Value {
    type Output = Result<Self>;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Number(l0), Self::Number(r0)) => Ok(Self::Number(l0 + r0)),
            (Self::Obj(l0), Self::Obj(r0)) => (l0 + r0).map(Self::Obj),
            (Self::String(l0), Self::String(r0)) => Ok(Self::String(l0 + &r0)),
            _ => Err(Error::Arithmetic("'+' Invalid operands".into())),
        }
    }
}

impl Sub for Value {
    type Output = Result<Self>;

    fn sub(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Number(a), Self::Number(b)) => Ok(Self::Number(a - b)),
            _ => Err(Error::Arithmetic("'-'(sub) Invalid operands".into())),
        }
    }
}

impl Mul for Value {
    type Output = Result<Self>;

    fn mul(self, rhs: Self) -> Self::Output {
        match self {
            Self::Number(a) => match rhs {
                Self::Number(b) => Ok(Self::Number(a * b)),
                _ => Err(Error::Arithmetic("'*' Invalid operands".into())),
            },
            _ => Err(Error::Arithmetic("'*' Invalid operands".into())),
        }
    }
}

impl Div for Value {
    type Output = Result<Self>;

    fn div(self, rhs: Self) -> Self::Output {
        match self {
            Self::Number(a) => match rhs {
                Self::Number(b) => Ok(Self::Number(a / b)),
                _ => Err(Error::Arithmetic("'/' Invalid operands".into())),
            },
            _ => Err(Error::Arithmetic("'/' Invalid operands".into())),
        }
    }
}

impl Not for Value {
    type Output = Result<Self>;

    fn not(self) -> Self::Output {
        match self {
            Self::Bool(a) => Ok(Self::Bool(!a)),
            Self::Nil => Ok(Self::Bool(true)),
            _ => Err(Error::Arithmetic("'!' Invalid operands".into())),
        }
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Self::Number(l0), Self::Number(r0)) => l0.partial_cmp(r0),
            (Self::Bool(l0), Self::Bool(r0)) => l0.partial_cmp(r0),
            _ => None,
        }
    }
}
