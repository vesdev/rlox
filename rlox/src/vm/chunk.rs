use std::fmt::Write;

use crate::vm::{opcode::OpCode, value::Value};
use colored::Colorize;

#[derive(Clone)]
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

    #[inline]
    pub fn get_op(&self, index: usize) -> OpCode {
        self.code[index]
    }

    #[inline]
    pub fn get_constant(&self, index: usize) -> Value {
        self.constants[index].clone()
    }

    pub fn get_line(&self, index: usize) -> usize {
        self.lines[index]
    }

    pub fn disassemble(&self, name: impl Into<String>) -> Result<String, std::fmt::Error> {
        let out = String::new();
        disassemble_chunk(out, self, name.into().as_str())
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
    writeln!(out, "     >--< {}", name)?;

    let mut offset = 0;
    while offset < chunk.code.len() {
        offset += disassemble_instruction(&mut out, chunk, offset)?;
        writeln!(out)?;
    }

    writeln!(out, "     >--<")?;

    Ok(out)
}

pub fn disassemble_instruction(
    out: &mut String,
    chunk: &Chunk,
    offset: usize,
) -> Result<usize, std::fmt::Error> {
    let op = chunk.get_op(offset);

    if offset > 0 && chunk.lines[offset] == chunk.lines[offset - 1] {
        write!(out, "{:<5}", "")?;
    } else {
        write!(out, "{:<5}", chunk.lines[offset].to_string().blue())?;
    }

    write!(out, "{} ", format!("{:04}", offset).green())?;

    //TOO LAZY TO PROPERLY OUTPUT OPERANDS
    let operands = match op {
        OpCode::Constant(opr)
        | OpCode::DefineGlobal(opr)
        | OpCode::GetGlobal(opr)
        | OpCode::SetGlobal(opr)
        | OpCode::GetLocal(opr)
        | OpCode::SetLocal(opr)
        | OpCode::Jump(opr)
        | OpCode::JumpIfFalse(opr)
        | OpCode::Loop(opr)
        | OpCode::SetUpValue(opr)
        | OpCode::GetUpValue(opr)
        | OpCode::Call(opr)
        | OpCode::Closure(opr)
        | OpCode::Method(opr) => Value::Number(opr as f64),
        OpCode::GetProperty(opr) | OpCode::SetProperty(opr) => chunk.constants[opr].clone(),
        _ => {
            write!(out, "{:<25}", op.to_string().blue())?;
            return Ok(1);
        }
    };

    let op = op.to_string();
    let operands = operands.to_string();

    write!(
        out,
        "{}[{}]",
        op.blue(),
        operands.replace('\n', "\\n").green()
    )?;

    // manual padding for color output
    // + 2 for the additional []
    write!(out, "{}", " ".repeat(25 - (op.len() + operands.len() + 2)))?;

    Ok(1)
}
