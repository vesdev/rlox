mod chunk;
mod opcode;
mod value;
mod vm;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use crate::{chunk::Chunk, opcode::OpCode, vm::Vm};

    use super::*;

    #[test]
    fn constant() {
        let mut chunk = Chunk::new();
        chunk.push_byte(OpCode::Constant as u8, 0);
        let constant = chunk.push_constant(value::Value::Number(1.2));
        chunk.push_byte(constant, 0);

        chunk.push_byte(OpCode::Return as u8, 0);

        println!("{}", chunk.disassemble("code").unwrap());
    }

    #[test]
    fn vm() {
        let mut chunk = Chunk::new();
        chunk.push_byte(OpCode::Constant as u8, 123);
        let constant = chunk.push_constant(value::Value::Number(1.2));
        chunk.push_byte(constant, 0);
        chunk.push_byte(OpCode::Negate as u8, 123);
        chunk.push_byte(OpCode::Return as u8, 123);

        println!("{}", chunk.disassemble("code").unwrap());

        println!("\n-- trace --\n");

        Vm::new().interpret(chunk).unwrap();
    }

    #[test]
    fn expression() {
        let mut chunk = Chunk::new();

        chunk.push_byte(OpCode::Constant as u8, 123);
        let constant = chunk.push_constant(value::Value::Number(1.2));
        chunk.push_byte(constant, 0);

        chunk.push_byte(OpCode::Constant as u8, 123);
        let constant = chunk.push_constant(value::Value::Number(3.4));
        chunk.push_byte(constant, 0);

        chunk.push_byte(OpCode::Add as u8, 123);

        chunk.push_byte(OpCode::Constant as u8, 123);
        let constant = chunk.push_constant(value::Value::Number(5.6));
        chunk.push_byte(constant, 0);

        chunk.push_byte(OpCode::Divide as u8, 123);
        chunk.push_byte(OpCode::Negate as u8, 123);

        chunk.push_byte(OpCode::Return as u8, 123);

        println!("{}", chunk.disassemble("code").unwrap());

        println!("\n-- trace --\n");

        Vm::new().interpret(chunk).unwrap();
    }
}
