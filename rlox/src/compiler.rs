use crate::vm::chunk::*;
use crate::vm::value::Value;
use std::fmt::Write;

mod scanner;
use crate::compiler::scanner::*;
use crate::error::*;
use crate::vm::opcode::OpCode;

pub struct Compiler<'a> {
    source: &'a str,
    scanner: Scanner<'a>,
    previous: Token<'a>,
    current: Token<'a>,
    panic_mode: bool,
    chunk: Chunk,
}

impl<'a> Compiler<'a> {
    pub fn compile(&mut self) -> Result<&Chunk> {
        self.panic_mode = false;
        self.advance();
        self.expression()?;
        if let Err(e) = self.consume(TokenKind::Eof, "Expect end of expression".to_string()) {
            println!("{:?}", e);
        }
        self.end();

        Ok(&self.chunk)
    }

    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            scanner: Scanner::new(source),
            previous: Token {
                kind: TokenKind::Error,
                lexeme: "n/a",
                line: 0,
            },
            current: Token {
                kind: TokenKind::Error,
                lexeme: "n/a",
                line: 0,
            },
            panic_mode: false,
            chunk: Chunk::new(),
        }
    }

    fn advance(&mut self) {
        self.previous = self.current;

        loop {
            self.current = self.scanner.scan_token();

            if self.current.kind != TokenKind::Error {
                break;
            }
        }
    }

    fn consume(&mut self, kind: TokenKind, message: String) -> Result<()> {
        if self.current.kind == kind {
            self.advance();
            return Ok(());
        }

        self.error_at_current(message)
    }

    fn end(&mut self) {
        self.emit_return();

        if cfg!(debug_print_code) {
            println!("{}", self.chunk.disassemble("compiler output").unwrap());
        }
    }

    fn parse_precedence(&mut self, precendence: Precedence) -> Result<()> {
        self.advance();

        if let Some(prefix) = get_rule(self.previous.kind).prefix {
            prefix(self)?;

            while precendence <= get_rule(self.current.kind).precedence {
                self.advance();

                if let Some(infix) = get_rule(self.previous.kind).infix {
                    infix(self)?;
                }
            }

            return Ok(());
        } else {
            Err(Error::Compile("Expect expression.".to_string()))
        }
    }

    fn expression(&mut self) -> Result<()> {
        self.parse_precedence(Precedence::Assignment)
    }

    fn current_chunk(&mut self) -> &mut Chunk {
        &mut self.chunk
    }

    fn emit_byte(&mut self, byte: u8) {
        let line = self.previous.line;
        self.current_chunk().push_byte(byte, line)
    }

    fn emit_bytes(&mut self, byte: u8, byte2: u8) {
        self.emit_byte(byte);
        self.emit_byte(byte2);
    }

    fn emit_return(&mut self) {
        self.emit_byte(OpCode::Return as u8)
    }

    fn make_constant(&mut self, value: Value) -> Result<u8> {
        let constant = self.current_chunk().push_constant(value);
        if constant > u8::MAX {
            self.error("Too many constants in one chunk.".to_string())?;
        }

        return Ok(constant);
    }

    fn emit_constant(&mut self, value: Value) -> Result<()> {
        let constant = self.make_constant(value)?;
        self.emit_bytes(OpCode::Constant as u8, constant);
        Ok(())
    }

    fn error_at_current(&mut self, message: String) -> Result<()> {
        self.error_at(self.current, message)
    }

    fn error(&mut self, message: String) -> Result<()> {
        self.error_at(self.previous, message)
    }

    fn error_at(&mut self, token: Token, message: String) -> Result<()> {
        if self.panic_mode {
            return Ok(());
        }
        self.panic_mode = true;

        let mut out = String::new();
        write!(out, "[line {}] Error", token.line).unwrap();

        if token.kind == TokenKind::Eof {
            write!(out, " at end").unwrap();
        } else if token.kind == TokenKind::Error {
        } else {
            write!(out, " at '{}'", token.lexeme).unwrap();
        }

        writeln!(out, ": {}", message).unwrap();

        Err(Error::Compile(out))
    }
}

fn get_rule(kind: TokenKind) -> Rule {
    match kind {
        TokenKind::LeftParen => Rule::new(Some(&grouping), None, Precedence::None),
        TokenKind::RightParen => Rule::new(None, None, Precedence::None),
        TokenKind::LeftBrace => Rule::new(None, None, Precedence::None),
        TokenKind::RightBrace => Rule::new(None, None, Precedence::None),
        TokenKind::Comma => Rule::new(None, None, Precedence::None),
        TokenKind::Dot => Rule::new(None, None, Precedence::None),
        TokenKind::Minus => Rule::new(Some(&unary), Some(&binary), Precedence::Term),
        TokenKind::Plus => Rule::new(None, Some(&binary), Precedence::Term),
        TokenKind::Semicolon => Rule::new(None, None, Precedence::None),
        TokenKind::Slash => Rule::new(None, Some(&binary), Precedence::Factor),
        TokenKind::Star => Rule::new(None, Some(&binary), Precedence::Factor),
        TokenKind::Bang => Rule::new(None, None, Precedence::None),
        TokenKind::BangEqual => Rule::new(None, None, Precedence::None),
        TokenKind::Equal => Rule::new(None, None, Precedence::None),
        TokenKind::EqualEqual => Rule::new(None, None, Precedence::None),
        TokenKind::Greater => Rule::new(None, None, Precedence::None),
        TokenKind::GreaterEqual => Rule::new(None, None, Precedence::None),
        TokenKind::Less => Rule::new(None, None, Precedence::None),
        TokenKind::LessEqual => Rule::new(None, None, Precedence::None),
        TokenKind::Identifier => Rule::new(None, None, Precedence::None),
        TokenKind::String => Rule::new(None, None, Precedence::None),
        TokenKind::Number => Rule::new(Some(&number), None, Precedence::None),
        TokenKind::And => Rule::new(None, None, Precedence::None),
        TokenKind::Class => Rule::new(None, None, Precedence::None),
        TokenKind::Else => Rule::new(None, None, Precedence::None),
        TokenKind::False => Rule::new(None, None, Precedence::None),
        TokenKind::For => Rule::new(None, None, Precedence::None),
        TokenKind::Fun => Rule::new(None, None, Precedence::None),
        TokenKind::If => Rule::new(None, None, Precedence::None),
        TokenKind::Nil => Rule::new(None, None, Precedence::None),
        TokenKind::Or => Rule::new(None, None, Precedence::None),
        TokenKind::Print => Rule::new(None, None, Precedence::None),
        TokenKind::Return => Rule::new(None, None, Precedence::None),
        TokenKind::Super => Rule::new(None, None, Precedence::None),
        TokenKind::This => Rule::new(None, None, Precedence::None),
        TokenKind::True => Rule::new(None, None, Precedence::None),
        TokenKind::Var => Rule::new(None, None, Precedence::None),
        TokenKind::While => Rule::new(None, None, Precedence::None),
        TokenKind::Error => Rule::new(None, None, Precedence::None),
        TokenKind::Eof => Rule::new(None, None, Precedence::None),
    }
}

fn grouping(compiler: &mut Compiler) -> Result<()> {
    compiler.expression();
    compiler.consume(
        TokenKind::RightParen,
        "Expect ')' after expression.".to_string(),
    )
}

fn binary(compiler: &mut Compiler) -> Result<()> {
    let operator_kind = compiler.previous.kind;

    let compiler_rule = get_rule(operator_kind);
    compiler.parse_precedence(Precedence::decode_unchecked(
        compiler_rule.precedence as u8 + 1,
    ));

    match operator_kind {
        TokenKind::Plus => compiler.emit_byte(OpCode::Add as u8),
        TokenKind::Minus => compiler.emit_byte(OpCode::Subtract as u8),
        TokenKind::Star => compiler.emit_byte(OpCode::Multiply as u8),
        TokenKind::Slash => compiler.emit_byte(OpCode::Divide as u8),
        _ => {}
    }
    Ok(())
}

fn number(compiler: &mut Compiler) -> Result<()> {
    let value = compiler.previous.lexeme.parse::<f64>().unwrap();
    compiler.emit_constant(Value::Number(value))
}

fn unary(compiler: &mut Compiler) -> Result<()> {
    let operator_kind = compiler.previous.kind;

    compiler.parse_precedence(Precedence::Unary);

    match operator_kind {
        TokenKind::Minus => compiler.emit_byte(OpCode::Negate as u8),
        _ => {}
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq, PartialOrd)]
enum Precedence {
    None,
    Assignment,
    Or,
    And,
    Equality,
    Comparison,
    Term,
    Factor,
    Unary,
    Call,
    Primary,
}

impl Precedence {
    #[inline]
    pub fn decode_unchecked(val: u8) -> Self {
        unsafe { std::mem::transmute(val) }
    }

    #[inline]
    pub fn decode(v: u8) -> Option<Precedence> {
        if v >= OpCode::Max as u8 {
            None
        } else {
            Some(Self::decode_unchecked(v))
        }
    }
}
//bing problem
//oh im ritard

#[derive(Clone, Copy)]
struct Rule {
    prefix: Option<&'static dyn Fn(&mut Compiler) -> Result<()>>,
    infix: Option<&'static dyn Fn(&mut Compiler) -> Result<()>>,
    precedence: Precedence,
}

impl Rule {
    fn new(
        prefix: Option<&'static dyn Fn(&mut Compiler) -> Result<()>>,
        infix: Option<&'static dyn Fn(&mut Compiler) -> Result<()>>,
        precedence: Precedence,
    ) -> Rule {
        Rule {
            prefix,
            infix,
            precedence,
        }
    }
}
