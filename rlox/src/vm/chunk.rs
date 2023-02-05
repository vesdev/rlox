use std::fmt::Write;

use crate::vm::{opcode::OpCode, value::Value};

pub struct Chunk {
    code: Vec<u8>,
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

    pub fn push_byte(&mut self, byte: u8, line: usize) {
        self.code.push(byte);
        self.lines.push(line);
    }

    pub fn push_constant(&mut self, value: Value) -> u8 {
        self.constants.push(value);
        (self.constants.len() - 1) as u8
    }

    pub fn get_byte(&self, index: usize) -> u8 {
        self.code[index]
    }

    pub fn get_constant(&self, index: usize) -> Value {
        self.constants[index].clone()
    }

    pub fn get_line(&self, index: usize) -> usize {
        self.lines[index].clone()
    }

    pub fn disassemble(&self, name: &str) -> Result<String, std::fmt::Error> {
        let out = String::new();
        disassemble_chunk(out, self, name)
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
    let op = OpCode::decode_unchecked(chunk.code[offset]);

    if offset > 0 && chunk.lines[offset] == chunk.lines[offset - 1] {
        write!(out, "\t|\t")?;
    } else {
        write!(out, "{}\t|\t", chunk.lines[offset])?;
    }

    write!(out, "{:04} {}", offset, op)?;
    let operands = match op {
        OpCode::Constant => &chunk.constants[chunk.code[offset + 1] as usize],
        _ => {
            writeln!(out)?;
            return Ok(1);
        }
    };

    writeln!(out, "\t{}", operands)?;

    Ok(op.operands() + 1)
}
