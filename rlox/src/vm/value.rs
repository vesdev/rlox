use std::{
    fmt::Display,
    ops::{Add, Div, Mul, Neg, Not, Sub},
    rc::Rc,
};

use super::object::*;

#[derive(Clone, Debug)]
pub enum Value {
    Number(f64),
    Nil,
    Bool(bool),
    Obj(Rc<Obj>),
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
    type Output = Value;

    fn neg(self) -> Self::Output {
        match self {
            Self::Number(v) => Self::Number(-v),
            _ => Self::Nil,
        }
    }
}

impl Add for Value {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Number(l0), Self::Number(r0)) => Self::Number(l0 + r0),
            (Self::Obj(l0), Self::Obj(r0)) => match (l0.as_ref(), r0.as_ref()) {
                (Obj::String(l0), Obj::String(r0)) => {
                    Self::Obj(Rc::new(Obj::String(l0.clone() + r0)))
                }
            },
            _ => Self::Nil,
        }
    }
}

impl Sub for Value {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        match self {
            Self::Number(a) => match rhs {
                Self::Number(b) => Self::Number(a - b),
                _ => Self::Nil,
            },
            _ => Self::Nil,
        }
    }
}

impl Mul for Value {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        match self {
            Self::Number(a) => match rhs {
                Self::Number(b) => Self::Number(a * b),
                _ => Self::Nil,
            },
            _ => Self::Nil,
        }
    }
}

impl Div for Value {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        match self {
            Self::Number(a) => match rhs {
                Self::Number(b) => Self::Number(a / b),
                _ => Self::Nil,
            },
            _ => Self::Nil,
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

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Number(l0), Self::Number(r0)) => l0 == r0,
            (Self::Bool(l0), Self::Bool(r0)) => l0 == r0,
            (Self::Obj(l0), Self::Obj(r0)) => match (l0.as_ref(), r0.as_ref()) {
                (Obj::String(l0), Obj::String(r0)) => l0 == r0,
            },
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
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
