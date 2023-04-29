use std::time::Instant;

use crate::{
    error::Error,
    vm::{object::NativeFun, value::Value},
};

pub struct Clock {
    now: Instant,
}

impl NativeFun for Clock {
    fn call(&self, _args: &[Value]) -> std::result::Result<Value, Error> {
        Ok(Value::Number(self.now.elapsed().as_secs_f64()))
    }
}

impl Clock {
    pub fn new() -> Box<Self> {
        Box::new(Self {
            now: Instant::now(),
        })
    }
}
