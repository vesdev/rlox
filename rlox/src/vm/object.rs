use std::{fmt::Display, string::String};

// strings get dropped when popped from the stack so no leak?
// doesnt need Rc yet, pretty sure

#[derive(Clone, Debug)]
pub enum Obj {
    String(String),
}

impl Display for Obj {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Obj::String(v) => '"'.to_string() + v + &'"'.to_string(),
            },
        )
    }
}
// impossible challenge, try not to look at moscow impl
