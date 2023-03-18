use crate::vm::chunk::*;
use crate::vm::value::Value;

use std::fmt::Write;
use std::rc::Rc;

mod scanner;
use crate::compiler::scanner::*;
use crate::error::*;
use crate::vm::object::*;
use crate::vm::opcode::OpCode;
pub struct Compiler<'a> {
    scanner: Scanner<'a>,
    previous: Token<'a>,
    current: Token<'a>,
    current_locals: Locals<'a>,
    panic_mode: bool,
    errors: Vec<Error>,
    chunk: Chunk,
}

impl<'a> Compiler<'a> {
    pub fn compile(&mut self) -> Result<&Chunk, Vec<Error>> {
        self.panic_mode = false;
        self.advance();

        while !self.matches(TokenKind::Eof) {
            self.declaration();
        }

        self.end();

        if !self.errors.is_empty() {
            return Err(std::mem::take(&mut self.errors));
        }
        Ok(&self.chunk)
    }

    pub fn new(source: &'a str) -> Self {
        Self {
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
            current_locals: Locals::new(),
            errors: Vec::new(),
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

    fn check(&mut self, kind: TokenKind) -> bool {
        self.current.kind == kind
    }

    fn matches(&mut self, kind: TokenKind) -> bool {
        if !self.check(kind) {
            return false;
        }

        self.advance();
        true
    }

    fn current_kind(&mut self) -> TokenKind {
        let kind = self.current.kind;
        self.advance();
        kind
    }

    fn end(&mut self) {
        self.emit_return();

        if cfg!(debug_print_code) {
            println!("{}", self.chunk.disassemble("compiler output").unwrap());
        }
    }

    fn begin_scope(&mut self) {
        self.current_locals.scope_depth += 1;
    }

    fn end_scope(&mut self) {
        self.current_locals.scope_depth -= 1;

        while !self.current_locals.locals.is_empty()
            && self.current_locals.locals[self.current_locals.locals.len() - 1].depth
                > self.current_locals.scope_depth
        {
            self.emit_op(OpCode::Pop);
            self.current_locals.locals.pop();
        }
    }

    fn parse_precedence(&mut self, precendence: Precedence) -> Result<()> {
        self.advance();

        if let Some(prefix) = get_rule(self.previous.kind).prefix {
            let can_assign = precendence <= Precedence::Assignment;
            prefix(self, can_assign)?;

            while precendence <= get_rule(self.current.kind).precedence {
                self.advance();

                if let Some(infix) = get_rule(self.previous.kind).infix {
                    infix(self, can_assign)?;
                }
            }

            if can_assign && self.matches(TokenKind::Equal) {
                self.error("Invalid assignment target.".to_string())?;
            }

            Ok(())
        } else {
            self.error_at_current("Expect expression.".to_string())
        }
    }

    fn identifier_constant(&mut self, name: Token) -> Result<usize> {
        self.make_constant(Value::Obj(Rc::new(Obj::String(name.lexeme.to_string()))))
    }

    fn identifiers_equal(&mut self, a: Token, b: Token) -> bool {
        a.lexeme == b.lexeme
    }

    fn resolve_local(&mut self, name: Token) -> Result<isize> {
        for i in (0..self.current_locals.locals.len()).rev() {
            let local = self.current_locals.locals[i].clone();
            if self.identifiers_equal(name, local.name) {
                if local.depth == -1 {
                    self.error("Can't read local variable in its own initializer.".to_string())?;
                }
                return Ok(i as isize);
            }
        }

        Ok(-1)
    }

    fn add_local(&mut self, name: Token<'a>) -> Result<()> {
        let local = Local::new(name, -1);
        self.current_locals.locals.push(local);

        Ok(())
    }

    fn declare_variable(&mut self) -> Result<()> {
        if self.current_locals.scope_depth == 0 {
            return Ok(());
        };

        let name = self.previous;

        for i in (0..self.current_locals.locals.len()).rev() {
            let local = &self.current_locals.locals[i];

            if local.depth != -1 && local.depth < self.current_locals.scope_depth {
                break;
            }

            if self.identifiers_equal(name, local.name) {
                return self.error("Already a variable with this name in this scope.".to_string());
            }
        }

        self.add_local(name)
    }

    fn parse_variable(&mut self, message: String) -> Result<usize> {
        self.consume(TokenKind::Identifier, message)?;

        self.declare_variable()?;
        if self.current_locals.scope_depth > 0 {
            return Ok(0);
        }

        self.identifier_constant(self.previous)
    }

    fn mark_initialized(&mut self) {
        let index = self.current_locals.locals.len() - 1;
        self.current_locals.locals[index].depth = self.current_locals.scope_depth;
    }

    fn define_variable(&mut self, global: usize) {
        if self.current_locals.scope_depth > 0 {
            self.mark_initialized();
            return;
        }

        self.emit_op(OpCode::DefineGlobal(global))
    }

    fn expression(&mut self) -> Result<()> {
        self.parse_precedence(Precedence::Assignment)
    }

    fn block(&mut self) -> Result<()> {
        while !self.check(TokenKind::RightBrace) && !self.check(TokenKind::Eof) {
            self.declaration();
        }

        self.consume(TokenKind::RightBrace, "Expect '}' after block.".to_string())
    }

    fn var_declaration(&mut self) -> Result<()> {
        let global = self.parse_variable("Expect variable name.".to_string())?;

        if self.matches(TokenKind::Equal) {
            self.expression()?;
        } else {
            self.emit_op(OpCode::Nil);
        }

        self.consume(
            TokenKind::Semicolon,
            "Expect ';' after variable declaration.".to_string(),
        )?;

        self.define_variable(global);
        Ok(())
    }

    fn expression_statement(&mut self) -> Result<()> {
        self.expression()?;
        self.consume(
            TokenKind::Semicolon,
            "Expect ';' after expression.".to_string(),
        )?;
        self.emit_op(OpCode::Pop);
        Ok(())
    }

    fn for_statement(&mut self) -> Result<()> {
        self.begin_scope();
        self.consume(TokenKind::LeftParen, "Expect '(' after 'for'.".to_string())?;

        if self.matches(TokenKind::Semicolon) {
            // no initializer
        } else if self.matches(TokenKind::Var) {
            self.var_declaration()?;
        } else {
            self.expression_statement()?;
        }

        let mut loop_start = self.current_chunk().len();

        //condition
        let mut exit_jump = 0;
        if !self.matches(TokenKind::Semicolon) {
            self.expression()?;
            self.consume(
                TokenKind::Semicolon,
                "Expect ';' after loop condition.".to_string(),
            )?;

            exit_jump = self.emit_jump(OpCode::JumpIfFalse(0));
            self.emit_op(OpCode::Pop);
        }

        //increment
        if !self.matches(TokenKind::RightParen) {
            let body_jump = self.emit_jump(OpCode::Jump(0));
            let increment_start = self.current_chunk().len();

            self.expression()?;
            self.emit_op(OpCode::Pop);
            self.consume(
                TokenKind::RightParen,
                "Expect ')' after for clauses.".to_string(),
            )?;

            self.emit_loop(loop_start);

            loop_start = increment_start;
            self.patch_jump(body_jump, OpCode::Jump(0));
        }

        self.statement()?;
        self.emit_loop(loop_start);

        //condition
        if exit_jump > 0 {
            self.patch_jump(exit_jump, OpCode::JumpIfFalse(0));
            self.emit_op(OpCode::Pop);
        }

        self.end_scope();

        Ok(())
    }

    fn if_statement(&mut self) -> Result<()> {
        self.consume(TokenKind::LeftParen, "Expect '(' after 'if'.".to_string())?;
        self.expression()?;
        self.consume(
            TokenKind::RightParen,
            "Expect ')' after condition.".to_string(),
        )?;

        let then_jump = self.emit_jump(OpCode::JumpIfFalse(0));
        self.emit_op(OpCode::Pop);

        self.statement()?;

        let else_jump = self.emit_jump(OpCode::Jump(0));

        self.patch_jump(then_jump, OpCode::JumpIfFalse(0));
        self.emit_op(OpCode::Pop);

        if self.matches(TokenKind::Else) {
            self.statement()?;
        }
        self.patch_jump(else_jump, OpCode::Jump(0));

        Ok(())
    }

    fn print_statement(&mut self) -> Result<()> {
        self.expression()?;
        self.consume(TokenKind::Semicolon, "Excpect ';' after value.".to_string())?;
        self.emit_op(OpCode::Print);
        Ok(())
    }

    fn while_statement(&mut self) -> Result<()> {
        let loop_start = self.current_chunk().len();
        self.consume(
            TokenKind::LeftParen,
            "Expect '(' after 'while'.".to_string(),
        )?;

        self.expression()?;
        self.consume(
            TokenKind::RightParen,
            "Expect ')' after condition.".to_string(),
        )?;

        let exit_jump = self.emit_jump(OpCode::JumpIfFalse(0));
        self.emit_op(OpCode::Pop);
        self.statement()?;
        self.emit_loop(loop_start);

        self.patch_jump(exit_jump, OpCode::JumpIfFalse(0));
        self.emit_op(OpCode::Pop);
        Ok(())
    }

    fn synchronize(&mut self) {
        self.panic_mode = false;

        while self.current.kind != TokenKind::Eof {
            if self.previous.kind == TokenKind::Semicolon {
                return;
            }
            match self.current.kind {
                TokenKind::Class
                | TokenKind::Fun
                | TokenKind::Var
                | TokenKind::For
                | TokenKind::If
                | TokenKind::While
                | TokenKind::Print
                | TokenKind::Return => {
                    return;
                }
                _ => {}
            }

            self.advance();
        }
    }

    fn statement(&mut self) -> Result<()> {
        match self.current_kind() {
            TokenKind::Print => self.print_statement(),
            TokenKind::For => self.for_statement(),
            TokenKind::If => self.if_statement(),
            TokenKind::While => self.while_statement(),
            TokenKind::LeftBrace => {
                self.begin_scope();
                self.block()?;
                self.end_scope();

                Ok(())
            }
            _ => self.expression_statement(),
        }
    }

    fn declaration(&mut self) {
        if self.matches(TokenKind::Var) {
            let _res = self.var_declaration();
        } else {
            let _res = self.statement();
        }

        if self.panic_mode {
            self.synchronize();
        }
    }

    fn current_chunk(&mut self) -> &mut Chunk {
        &mut self.chunk
    }

    fn emit_op(&mut self, op: OpCode) {
        let line = self.previous.line;
        self.current_chunk().push_op(op, line)
    }

    fn emit_ops(&mut self, op: OpCode, op2: OpCode) {
        self.emit_op(op);
        self.emit_op(op2);
    }

    fn emit_loop(&mut self, loop_start: usize) {
        let offset = self.current_chunk().len() - loop_start;
        self.emit_op(OpCode::Loop(offset));
    }

    fn emit_jump(&mut self, op: OpCode) -> usize {
        self.emit_op(op);
        self.current_chunk().len() - 1
    }

    fn emit_return(&mut self) {
        self.emit_op(OpCode::Return)
    }

    fn make_constant(&mut self, value: Value) -> Result<usize> {
        let constant = self.current_chunk().push_constant(value);

        Ok(constant)
    }

    fn emit_constant(&mut self, value: Value) -> Result<()> {
        let constant = self.make_constant(value)?;
        self.emit_op(OpCode::Constant(constant));
        Ok(())
    }

    fn patch_jump(&mut self, offset: usize, op: OpCode) {
        let jump = self.current_chunk().len() - offset;

        match op {
            OpCode::JumpIfFalse(_) => {
                self.current_chunk()
                    .insert_op(OpCode::JumpIfFalse(jump), offset);
            }
            OpCode::Jump(_) => {
                self.current_chunk().insert_op(OpCode::Jump(jump), offset);
            }
            _ => (),
        }
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

        self.errors.push(Error::Compile(out.clone(), token.line));

        Err(Error::Compile(out, token.line))
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
        TokenKind::Bang => Rule::new(Some(&unary), None, Precedence::None),
        TokenKind::BangEqual => Rule::new(None, Some(&binary), Precedence::Equality),
        TokenKind::Equal => Rule::new(None, None, Precedence::None),
        TokenKind::EqualEqual => Rule::new(None, Some(&binary), Precedence::Equality),
        TokenKind::Greater => Rule::new(None, Some(&binary), Precedence::Comparison),
        TokenKind::GreaterEqual => Rule::new(None, Some(&binary), Precedence::Comparison),
        TokenKind::Less => Rule::new(None, Some(&binary), Precedence::Comparison),
        TokenKind::LessEqual => Rule::new(None, Some(&binary), Precedence::Comparison),
        TokenKind::Identifier => Rule::new(Some(&variable), None, Precedence::None),
        TokenKind::String => Rule::new(Some(&string), None, Precedence::None),
        TokenKind::Number => Rule::new(Some(&number), None, Precedence::None),
        TokenKind::And => Rule::new(None, Some(&and), Precedence::And),
        TokenKind::Class => Rule::new(None, None, Precedence::None),
        TokenKind::Else => Rule::new(None, None, Precedence::None),
        TokenKind::False => Rule::new(Some(&literal), None, Precedence::None),
        TokenKind::For => Rule::new(None, None, Precedence::None),
        TokenKind::Fun => Rule::new(None, None, Precedence::None),
        TokenKind::If => Rule::new(None, None, Precedence::None),
        TokenKind::Nil => Rule::new(Some(&literal), None, Precedence::None),
        TokenKind::Or => Rule::new(None, Some(&or), Precedence::Or),
        TokenKind::Print => Rule::new(None, None, Precedence::None),
        TokenKind::Return => Rule::new(None, None, Precedence::None),
        TokenKind::Super => Rule::new(None, None, Precedence::None),
        TokenKind::This => Rule::new(None, None, Precedence::None),
        TokenKind::True => Rule::new(Some(&literal), None, Precedence::None),
        TokenKind::Var => Rule::new(None, None, Precedence::None),
        TokenKind::While => Rule::new(None, None, Precedence::None),
        TokenKind::Error => Rule::new(None, None, Precedence::None),
        TokenKind::Eof => Rule::new(None, None, Precedence::None),
    }
}

fn grouping(compiler: &mut Compiler, _can_assign: bool) -> Result<()> {
    compiler.expression()?;
    compiler.consume(
        TokenKind::RightParen,
        "Expect ')' after expression.".to_string(),
    )
}

fn binary(compiler: &mut Compiler, _can_assign: bool) -> Result<()> {
    let operator_kind = compiler.previous.kind;

    let compiler_rule = get_rule(operator_kind);
    compiler.parse_precedence(compiler_rule.precedence.next())?;

    match operator_kind {
        TokenKind::BangEqual => compiler.emit_ops(OpCode::Equal, OpCode::Not),
        TokenKind::EqualEqual => compiler.emit_op(OpCode::Equal),
        TokenKind::Greater => compiler.emit_op(OpCode::Greater),
        TokenKind::GreaterEqual => compiler.emit_ops(OpCode::Less, OpCode::Not),
        TokenKind::Less => compiler.emit_op(OpCode::Less),
        TokenKind::LessEqual => compiler.emit_ops(OpCode::Greater, OpCode::Not),
        TokenKind::Plus => compiler.emit_op(OpCode::Add),
        TokenKind::Minus => compiler.emit_op(OpCode::Subtract),
        TokenKind::Star => compiler.emit_op(OpCode::Multiply),
        TokenKind::Slash => compiler.emit_op(OpCode::Divide),
        _ => {}
    }
    Ok(())
}

fn number(compiler: &mut Compiler, _can_assign: bool) -> Result<()> {
    let value = compiler.previous.lexeme.parse::<f64>().unwrap();
    compiler.emit_constant(Value::Number(value))
}

fn unary(compiler: &mut Compiler, _can_assign: bool) -> Result<()> {
    let operator_kind = compiler.previous.kind;

    compiler.parse_precedence(Precedence::Unary)?;

    match operator_kind {
        TokenKind::Bang => compiler.emit_op(OpCode::Not),
        TokenKind::Minus => compiler.emit_op(OpCode::Negate),
        _ => {}
    }
    Ok(())
}

fn literal(compiler: &mut Compiler, _can_assign: bool) -> Result<()> {
    match compiler.previous.kind {
        TokenKind::False => compiler.emit_op(OpCode::False),
        TokenKind::Nil => compiler.emit_op(OpCode::Nil),
        TokenKind::True => compiler.emit_op(OpCode::True),
        _ => {}
    }

    Ok(())
}

fn string(compiler: &mut Compiler, _can_assign: bool) -> Result<()> {
    compiler.emit_constant(Value::Obj(Rc::new(Obj::String(String::from(
        compiler.previous.lexeme.trim_matches('"'),
    )))))
}

fn variable(compiler: &mut Compiler, can_assign: bool) -> Result<()> {
    named_variable(compiler, compiler.previous, can_assign)
}

fn named_variable(compiler: &mut Compiler, name: Token, can_assign: bool) -> Result<()> {
    let (get_op, set_op);
    let arg = compiler.resolve_local(name)?;

    if arg != -1 {
        get_op = OpCode::GetLocal(arg as usize);
        set_op = OpCode::SetLocal(arg as usize);
    } else {
        let arg = compiler.identifier_constant(name)?;
        get_op = OpCode::GetGlobal(arg);
        set_op = OpCode::SetGlobal(arg);
    }

    if can_assign && compiler.matches(TokenKind::Equal) {
        compiler.expression()?;
        compiler.emit_op(set_op);
    } else {
        compiler.emit_op(get_op);
    }

    Ok(())
}

fn and(compiler: &mut Compiler, _can_assign: bool) -> Result<()> {
    let end_jump = compiler.emit_jump(OpCode::JumpIfFalse(0));

    compiler.emit_op(OpCode::Pop);
    compiler.parse_precedence(Precedence::And)?;

    compiler.patch_jump(end_jump, OpCode::JumpIfFalse(0));
    Ok(())
}

fn or(compiler: &mut Compiler, _can_assign: bool) -> Result<()> {
    let else_jump = compiler.emit_jump(OpCode::JumpIfFalse(0));
    let end_jump = compiler.emit_jump(OpCode::Jump(0));

    compiler.patch_jump(else_jump, OpCode::JumpIfFalse(0));
    compiler.emit_op(OpCode::Pop);

    compiler.parse_precedence(Precedence::Or)?;
    compiler.patch_jump(end_jump, OpCode::Jump(0));

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
    pub fn next(&self) -> Self {
        match self {
            Self::None => Self::Assignment,
            Self::Assignment => Self::Or,
            Self::Or => Self::And,
            Self::And => Self::Equality,
            Self::Equality => Self::Comparison,
            Self::Comparison => Self::Term,
            Self::Term => Self::Factor,
            Self::Factor => Self::Unary,
            Self::Unary => Self::Call,
            Self::Call => Self::Primary,
            Self::Primary => Self::Primary,
        }
    }
}

type RuleFn = &'static dyn Fn(&mut Compiler, bool) -> Result<()>;

#[derive(Clone, Copy)]
struct Rule {
    prefix: Option<RuleFn>,
    infix: Option<RuleFn>,
    precedence: Precedence,
}

impl Rule {
    fn new(prefix: Option<RuleFn>, infix: Option<RuleFn>, precedence: Precedence) -> Rule {
        Rule {
            prefix,
            infix,
            precedence,
        }
    }
}

//TODO: rename this
struct Locals<'a> {
    locals: Vec<Local<'a>>,
    scope_depth: isize,
}

impl Locals<'_> {
    pub fn new() -> Self {
        Self {
            locals: Vec::new(),
            scope_depth: 0,
        }
    }
}

#[derive(Clone)]
struct Local<'a> {
    pub name: Token<'a>,
    pub depth: isize,
}

impl<'a> Local<'a> {
    pub fn new(name: Token<'a>, depth: isize) -> Self {
        Self { name, depth }
    }
}
