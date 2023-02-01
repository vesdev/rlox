use crate::vm::chunk::*;
use std::fmt::Write;

mod scanner;
use crate::compiler::scanner::*;
use crate::error::*;

pub struct Compiler<'a> {
    source: &'a str,
    scanner: Scanner<'a>,
    previous: Token<'a>,
    current: Token<'a>,
    panic_mode: bool,
}

impl<'a> Compiler<'a> {
    pub fn compile(&mut self, _chunk: &mut Chunk) -> Result<()> {
        self.panic_mode = false;
        self.advance();
        self.expression();
        if let Err(e) = self.consume(TokenKind::Eof, "Expect end of expression".to_string()) {
            println!("{:?}", e);
        }

        Ok(())
    }

    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            scanner: Scanner::new(),
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

    fn expression(&self) {
        todo!()
    }

    fn consume(&mut self, kind: TokenKind, message: String) -> Result<()> {
        if self.current.kind == kind {
            self.advance();
            return Ok(());
        }

        self.error_at_current(message)
    }

    fn emit_byte(_byte: u8) {}

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
