use std::{
    fmt::Display,
    ops::{Add, Div, Mul, Neg, Not, Sub},
};

use super::object::*;

#[derive(Clone, PartialEq)]
pub enum Value {
    Number(f64),
    Nil,
    Bool(bool),
    Obj(Obj),
}

impl Value {
    pub fn is_falsey(&self) -> bool {
        match self {
            Value::Number(n) => n < &1.0,
            Value::Nil => false,
            Value::Bool(n) => n == &false,
            Value::Obj(_) => false,
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Value::Number(v) => v.to_string(),
                Value::Nil => "Nil".to_string(),
                Value::Bool(v) => v.to_string(),
                Value::Obj(v) => v.to_string(),
            }
        )
    }
}

impl Neg for Value {
    type Output = Option<Value>;

    fn neg(self) -> Self::Output {
        match self {
            Self::Number(v) => Some(Self::Number(-v)),
            _ => None,
        }
    }
}

impl Add for Value {
    type Output = Option<Self>;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Number(l0), Self::Number(r0)) => Some(Self::Number(l0 + r0)),
            (Self::Obj(l0), Self::Obj(r0)) => (l0 + r0).map(Self::Obj),
            _ => None,
        }
    }
}

impl Sub for Value {
    type Output = Option<Self>;

    fn sub(self, rhs: Self) -> Self::Output {
        match self {
            Self::Number(a) => match rhs {
                Self::Number(b) => Some(Self::Number(a - b)),
                _ => None,
            },
            _ => None,
        }
    }
}

impl Mul for Value {
    type Output = Option<Self>;

    fn mul(self, rhs: Self) -> Self::Output {
        match self {
            Self::Number(a) => match rhs {
                Self::Number(b) => Some(Self::Number(a * b)),
                _ => None,
            },
            _ => None,
        }
    }
}

impl Div for Value {
    type Output = Option<Self>;

    fn div(self, rhs: Self) -> Self::Output {
        match self {
            Self::Number(a) => match rhs {
                Self::Number(b) => Some(Self::Number(a / b)),
                _ => None,
            },
            _ => None,
        }
    }
}

impl Not for Value {
    type Output = Option<Self>;

    fn not(self) -> Self::Output {
        match self {
            Self::Bool(a) => Some(Self::Bool(!a)),
            Self::Nil => Some(Self::Bool(true)),
            _ => None,
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
