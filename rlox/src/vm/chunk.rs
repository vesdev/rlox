use std::fmt::Write;

use crate::vm::{opcode::OpCode, value::Value};

pub struct Chunk {
    code: Vec<OpCode>,
    constants: Vec<Value>,
    lines: Vec<usize>,
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            constants: Vec::new(),
            lines: Vec::new(),
        }
    }

    pub fn push_op(&mut self, op: OpCode, line: usize) {
        self.code.push(op);
        self.lines.push(line);
    }

    pub fn push_constant(&mut self, value: Value) -> usize {
        self.constants.push(value);
        self.constants.len() - 1
    }

    pub fn insert_op(&mut self, op: OpCode, index: usize) {
        self.code[index] = op;
    }

    pub fn get_op(&self, index: usize) -> OpCode {
        self.code[index]
    }

    pub fn get_constant(&self, index: usize) -> Value {
        self.constants[index].clone()
    }

    pub fn get_line(&self, index: usize) -> usize {
        self.lines[index]
    }

    pub fn disassemble(&self, name: &str) -> Result<String, std::fmt::Error> {
        let out = String::new();
        disassemble_chunk(out, self, name)
    }

    pub fn len(&self) -> usize {
        self.code.len()
    }

    pub fn is_empty(&self) -> bool {
        self.code.is_empty()
    }
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new()
    }
}

pub fn disassemble_chunk(
    mut out: String,
    chunk: &Chunk,
    name: &str,
) -> Result<String, std::fmt::Error> {
    writeln!(out, "\t>--\t>{}<", name.to_uppercase())?;

    let mut offset = 0;
    while offset < chunk.code.len() {
        offset += disassemble_instruction(&mut out, chunk, offset)?;
    }

    writeln!(out, "\t>--")?;

    Ok(out)
}

pub fn disassemble_instruction(
    out: &mut String,
    chunk: &Chunk,
    offset: usize,
) -> Result<usize, std::fmt::Error> {
    let op = chunk.get_op(offset);

    if offset > 0 && chunk.lines[offset] == chunk.lines[offset - 1] {
        write!(out, "\t|\t")?;
    } else {
        write!(out, "{}\t|\t", chunk.lines[offset])?;
    }

    write!(out, "{:04} {}", offset, op)?;
    let operands = match op {
        OpCode::Constant(opr)
        | OpCode::DefineGlobal(opr)
        | OpCode::GetGlobal(opr)
        | OpCode::SetGlobal(opr) => chunk.constants[opr as usize].clone(),
        OpCode::GetLocal(opr)
        | OpCode::SetLocal(opr)
        | OpCode::Jump(opr)
        | OpCode::JumpIfFalse(opr)
        | OpCode::Loop(opr) => Value::Number(opr as f64),
        _ => {
            writeln!(out)?;
            return Ok(1);
        }
    };

    writeln!(out, "( {} )", operands)?;

    Ok(1)
}
