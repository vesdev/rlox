use std::{
    fmt::Display,
    ops::{Add, Div, Mul, Neg, Sub},
};

#[derive(Clone, Debug)]
pub enum Value {
    Number(f64),
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Value::Number(v) => v,
            }
        )
    }
}

impl Neg for Value {
    type Output = Value;

    fn neg(self) -> Self::Output {
        match self {
            Self::Number(v) => Self::Number(-v),
        }
    }
}

impl Add for Value {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        match self {
            Self::Number(a) => match rhs {
                Self::Number(b) => Self::Number(a + b),
            },
        }
    }
}

impl Sub for Value {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        match self {
            Self::Number(a) => match rhs {
                Self::Number(b) => Self::Number(a - b),
            },
        }
    }
}

impl Mul for Value {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        match self {
            Self::Number(a) => match rhs {
                Self::Number(b) => Self::Number(a * b),
            },
        }
    }
}

impl Div for Value {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        match self {
            Self::Number(a) => match rhs {
                Self::Number(b) => Self::Number(a / b),
            },
        }
    }
}
