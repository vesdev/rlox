use crate::vm::chunk::*;
use crate::vm::value::Value;

use std::fmt::Write;
use std::rc::Rc;

mod scanner;
use crate::compiler::scanner::*;
use crate::error::*;
use crate::vm::object::*;
use crate::vm::opcode::OpCode;

pub struct State<'a> {
    scanner: Scanner<'a>,
    previous: Token<'a>,
    current: Token<'a>,
}

impl<'a> State<'a> {
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
        }
    }
}

pub struct Compiler<'a> {
    state: State<'a>,
    scope: FunctionScope<'a>,
    panic_mode: bool,
    errors: Vec<Error>,
}

impl<'a> Compiler<'a> {
    pub fn compile(&mut self) -> Result<Function, Vec<Error>> {
        self.panic_mode = false;
        self.advance();

        while !self.matches(TokenKind::Eof) {
            self.declaration();
        }

        self.end()
    }

    pub fn new(source: &'a str, kind: FunctionKind) -> Self {
        Self {
            state: State::new(source),
            panic_mode: false,
            scope: FunctionScope::new("", kind),
            errors: Vec::new(),
        }
    }

    fn advance(&mut self) {
        self.state.previous = self.state.current;

        loop {
            self.state.current = self.state.scanner.scan_token();

            if self.state.current.kind != TokenKind::Error {
                break;
            }
        }
    }

    fn consume(&mut self, kind: TokenKind, message: impl Into<String>) {
        if self.state.current.kind == kind {
            self.advance();
            return;
        }

        self.error_at_current(message)
    }

    fn check(&mut self, kind: TokenKind) -> bool {
        self.state.current.kind == kind
    }

    fn matches(&mut self, kind: TokenKind) -> bool {
        if !self.check(kind) {
            return false;
        }

        self.advance();
        true
    }

    fn current_kind(&mut self) -> TokenKind {
        let kind = self.state.current.kind;
        self.advance();
        kind
    }

    fn end(&mut self) -> Result<Function, Vec<Error>> {
        self.emit_return();

        if cfg!(debug_print_code) {
            let mut name = "Entry Point".to_string();
            if !self.scope.function.name.is_empty() {
                name = self.scope.function.name.clone();
            }
            println!("{}", self.current_chunk().disassemble(name).unwrap());
        }

        if !self.errors.is_empty() {
            return Err(std::mem::take(&mut self.errors));
        }
        Ok(std::mem::take(&mut self.scope.function))
    }

    fn begin_scope(&mut self) {
        self.scope.scope_depth += 1;
    }

    fn end_scope(&mut self) {
        self.scope.scope_depth -= 1;

        while !self.scope.locals.is_empty()
            && self.scope.locals[self.scope.locals.len() - 1].depth > self.scope.scope_depth
        {
            self.emit_op(OpCode::Pop);
            self.scope.locals.pop();
        }
    }

    fn parse_precedence(&mut self, precendence: Precedence) {
        self.advance();

        if let Some(prefix) = get_rule(self.state.previous.kind).prefix {
            let can_assign = precendence <= Precedence::Assignment;
            prefix(self, can_assign);

            while precendence <= get_rule(self.state.current.kind).precedence {
                self.advance();

                if let Some(infix) = get_rule(self.state.previous.kind).infix {
                    infix(self, can_assign);
                }
            }

            if can_assign && self.matches(TokenKind::Equal) {
                self.error("Invalid assignment target.");
            }
        } else {
            self.error_at_current("Expect expression.")
        }
    }

    fn identifier_constant(&mut self, name: Token) -> usize {
        self.make_constant(Value::Obj(Obj::String(name.lexeme.to_string())))
    }

    fn identifiers_equal(&mut self, a: Token, b: Token) -> bool {
        a.lexeme == b.lexeme
    }

    fn resolve_local(&mut self, name: Token) -> isize {
        for i in (0..self.scope.locals.len()).rev() {
            let local = self.scope.locals[i].clone();
            if self.identifiers_equal(name, local.name) {
                if local.depth == -1 {
                    self.error("Can't read local variable in its own initializer.");
                }
                return i as isize;
            }
        }

        -1
    }

    fn add_local(&mut self, name: Token<'a>) {
        let local = Local::new(name, self.scope.scope_depth);
        self.scope.locals.push(local);
    }

    fn declare_variable(&mut self) {
        if self.scope.scope_depth == 0 {
            return;
        }

        let name = self.state.previous;

        for i in (0..self.scope.locals.len()).rev() {
            let local = &self.scope.locals[i];

            if local.depth != -1 && local.depth < self.scope.scope_depth {
                break;
            }

            if self.identifiers_equal(name, local.name) {
                return self.error("Already a variable with this name in this scope.");
            }
        }

        self.add_local(name)
    }

    fn parse_variable(&mut self, message: impl Into<String>) -> usize {
        self.consume(TokenKind::Identifier, message);

        self.declare_variable();
        if self.scope.scope_depth > 0 {
            return 0;
        }

        self.identifier_constant(self.state.previous)
    }

    fn mark_initialized(&mut self) {
        if self.scope.scope_depth == 0 {
            return;
        }
        let index = self.scope.locals.len() - 1;
        self.scope.locals[index].depth = self.scope.scope_depth;
    }

    fn define_variable(&mut self, global: usize) {
        if self.scope.scope_depth > 0 {
            self.mark_initialized();
            return;
        }

        self.emit_op(OpCode::DefineGlobal(global))
    }

    fn argument_list(&mut self) -> usize {
        let mut arg_count = 0;
        if !self.check(TokenKind::RightParen) {
            loop {
                self.expression();
                if arg_count == 255 {
                    self.error("Can't have more than 255 arguments.");
                }
                arg_count += 1;
                if !self.matches(TokenKind::Comma) {
                    break;
                }
            }
        }
        self.consume(TokenKind::RightParen, "Expect ')' after arguments.");
        arg_count
    }

    fn expression(&mut self) {
        self.parse_precedence(Precedence::Assignment)
    }

    fn block(&mut self) {
        while !self.check(TokenKind::RightBrace) && !self.check(TokenKind::Eof) {
            self.declaration();
        }

        self.consume(TokenKind::RightBrace, "Expect '}' after block.")
    }

    fn function(&mut self, kind: FunctionKind) {
        let mut compiler = Compiler::new("", kind);
        compiler.scope.function.name = self.state.previous.lexeme.to_string();
        //hijack the new compilers state and swap it with out current one
        //then swap back once done parsing funciton body
        std::mem::swap(&mut compiler.state, &mut self.state);

        compiler.begin_scope();

        compiler.consume(TokenKind::LeftParen, "Expect '(' after function name.");
        if !compiler.check(TokenKind::RightParen) {
            loop {
                compiler.scope.function.arity += 1;

                if compiler.scope.function.arity > 256 {
                    compiler.error_at_current("Can't have more than 255 parameters.");
                }

                let constant = compiler.parse_variable("Expect parameter name.");
                compiler.define_variable(constant);

                if !compiler.matches(TokenKind::Comma) {
                    break;
                }
            }
        }

        compiler.consume(TokenKind::RightParen, "Expect ')' after parameters.");
        compiler.consume(TokenKind::LeftBrace, "Expect '{' before function body.");

        compiler.block();

        match compiler.end() {
            Ok(result) => {
                let func = self.make_constant(Value::Obj(Obj::Function(result)));
                self.emit_op(OpCode::Constant(func));
            }
            Err(mut e) => {
                //handle errors from nested function
                self.errors.append(&mut e);
            }
        }

        std::mem::swap(&mut compiler.state, &mut self.state);
    }

    fn fun_declaration(&mut self) {
        let global = self.parse_variable("Expect function name.");
        self.mark_initialized();
        self.function(FunctionKind::Function);
        self.define_variable(global);
    }

    fn var_declaration(&mut self) {
        let global = self.parse_variable("Expect variable name.");

        if self.matches(TokenKind::Equal) {
            self.expression();
        } else {
            self.emit_op(OpCode::Nil);
        }

        self.consume(
            TokenKind::Semicolon,
            "Expect ';' after variable declaration.",
        );

        self.define_variable(global);
    }

    fn expression_statement(&mut self) {
        self.expression();
        self.consume(TokenKind::Semicolon, "Expect ';' after expression.");
        self.emit_op(OpCode::Pop);
    }

    fn for_statement(&mut self) {
        self.begin_scope();
        self.consume(TokenKind::LeftParen, "Expect '(' after 'for'.");

        if self.matches(TokenKind::Semicolon) {
            // no initializer
        } else if self.matches(TokenKind::Var) {
            self.var_declaration();
        } else {
            self.expression_statement();
        }

        let mut loop_start = self.current_chunk().len();

        //condition
        let mut exit_jump = 0;
        let mut condition_exists = false;
        if !self.matches(TokenKind::Semicolon) {
            self.expression();
            self.consume(TokenKind::Semicolon, "Expect ';' after loop condition.");

            exit_jump = self.emit_jump(OpCode::JumpIfFalse(0));
            condition_exists = true;
            self.emit_op(OpCode::Pop);
        }

        //increment
        if !self.matches(TokenKind::RightParen) {
            let body_jump = self.emit_jump(OpCode::Jump(0));
            let increment_start = self.current_chunk().len();

            self.expression();
            self.emit_op(OpCode::Pop);
            self.consume(TokenKind::RightParen, "Expect ')' after for clauses.");

            self.emit_loop(loop_start);

            loop_start = increment_start;
            self.patch_jump(body_jump, OpCode::Jump(0));
        }

        self.statement();
        self.emit_loop(loop_start);

        //condition
        if condition_exists {
            self.patch_jump(exit_jump, OpCode::JumpIfFalse(0));
            self.emit_op(OpCode::Pop);
        }

        self.end_scope();
    }

    fn if_statement(&mut self) {
        self.consume(TokenKind::LeftParen, "Expect '(' after 'if'.");
        self.expression();
        self.consume(TokenKind::RightParen, "Expect ')' after condition.");

        let then_jump = self.emit_jump(OpCode::JumpIfFalse(0));
        self.emit_op(OpCode::Pop);

        self.statement();

        let else_jump = self.emit_jump(OpCode::Jump(0));

        self.patch_jump(then_jump, OpCode::JumpIfFalse(0));
        self.emit_op(OpCode::Pop);

        if self.matches(TokenKind::Else) {
            self.statement();
        }
        self.patch_jump(else_jump, OpCode::Jump(0));
    }

    fn print_statement(&mut self) {
        self.expression();
        self.consume(TokenKind::Semicolon, "Excpect ';' after value.");
        self.emit_op(OpCode::Print);
    }

    fn while_statement(&mut self) {
        let loop_start = self.current_chunk().len();
        self.consume(TokenKind::LeftParen, "Expect '(' after 'while'.");

        self.expression();
        self.consume(TokenKind::RightParen, "Expect ')' after condition.");

        let exit_jump = self.emit_jump(OpCode::JumpIfFalse(0));
        self.emit_op(OpCode::Pop);
        self.statement();
        self.emit_loop(loop_start);

        self.patch_jump(exit_jump, OpCode::JumpIfFalse(0));
        self.emit_op(OpCode::Pop);
    }

    fn synchronize(&mut self) {
        self.panic_mode = false;

        while self.state.current.kind != TokenKind::Eof {
            if self.state.previous.kind == TokenKind::Semicolon {
                return;
            }
            match self.state.current.kind {
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

    fn statement(&mut self) {
        match self.current_kind() {
            TokenKind::Print => self.print_statement(),
            TokenKind::For => self.for_statement(),
            TokenKind::If => self.if_statement(),
            TokenKind::While => self.while_statement(),
            TokenKind::LeftBrace => {
                self.begin_scope();
                self.block();
                self.end_scope();
            }
            _ => self.expression_statement(),
        }
    }

    fn declaration(&mut self) {
        // ignore errors on this level
        // manually synchronized after
        if self.matches(TokenKind::Fun) {
            self.fun_declaration();
        } else if self.matches(TokenKind::Var) {
            self.var_declaration();
        } else {
            self.statement();
        }

        if self.panic_mode {
            self.synchronize();
        }
    }

    fn current_chunk(&mut self) -> &mut Chunk {
        &mut self.scope.function.chunk
    }

    fn emit_op(&mut self, op: OpCode) {
        let line = self.state.previous.line;
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

    fn make_constant(&mut self, value: Value) -> usize {
        let constant = self.current_chunk().push_constant(value);

        constant
    }

    fn emit_constant(&mut self, value: Value) {
        let constant = self.make_constant(value);
        self.emit_op(OpCode::Constant(constant));
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

    fn error_at_current(&mut self, message: impl Into<String>) {
        self.error_at(self.state.current, message)
    }

    fn error(&mut self, message: impl Into<String>) {
        self.error_at(self.state.previous, message)
    }

    fn error_at(&mut self, token: Token, message: impl Into<String>) {
        if self.panic_mode {
            return;
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

        writeln!(out, ": {}", message.into()).unwrap();

        let err = Error::Compile(out, token.line);
        self.errors.push(err);
    }
}

fn get_rule(kind: TokenKind) -> Rule {
    match kind {
        TokenKind::LeftParen => Rule::new(Some(&grouping), Some(&call), Precedence::Call),
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

fn grouping(compiler: &mut Compiler, _can_assign: bool) {
    compiler.expression();
    compiler.consume(TokenKind::RightParen, "Expect ')' after expression.")
}

fn binary(compiler: &mut Compiler, _can_assign: bool) {
    let operator_kind = compiler.state.previous.kind;

    let compiler_rule = get_rule(operator_kind);
    compiler.parse_precedence(compiler_rule.precedence.next());

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
}

fn number(compiler: &mut Compiler, _can_assign: bool) {
    let value = compiler.state.previous.lexeme.parse::<f64>().unwrap();
    compiler.emit_constant(Value::Number(value))
}

fn unary(compiler: &mut Compiler, _can_assign: bool) {
    let operator_kind = compiler.state.previous.kind;

    compiler.parse_precedence(Precedence::Unary);

    match operator_kind {
        TokenKind::Bang => compiler.emit_op(OpCode::Not),
        TokenKind::Minus => compiler.emit_op(OpCode::Negate),
        _ => {}
    }
}

fn literal(compiler: &mut Compiler, _can_assign: bool) {
    match compiler.state.previous.kind {
        TokenKind::False => compiler.emit_op(OpCode::False),
        TokenKind::Nil => compiler.emit_op(OpCode::Nil),
        TokenKind::True => compiler.emit_op(OpCode::True),
        _ => {}
    }
}

fn string(compiler: &mut Compiler, _can_assign: bool) {
    compiler.emit_constant(Value::Obj(Obj::String(String::from(
        compiler.state.previous.lexeme.trim_matches('"'),
    ))))
}

fn variable(compiler: &mut Compiler, can_assign: bool) {
    named_variable(compiler, compiler.state.previous, can_assign)
}

fn named_variable(compiler: &mut Compiler, name: Token, can_assign: bool) {
    let (get_op, set_op);
    let arg = compiler.resolve_local(name);

    if arg != -1 {
        get_op = OpCode::GetLocal(arg as usize);
        set_op = OpCode::SetLocal(arg as usize);
    } else {
        let arg = compiler.identifier_constant(name);
        get_op = OpCode::GetGlobal(arg);
        set_op = OpCode::SetGlobal(arg);
    }

    if can_assign && compiler.matches(TokenKind::Equal) {
        compiler.expression();
        compiler.emit_op(set_op);
    } else {
        compiler.emit_op(get_op);
    }
}

fn and(compiler: &mut Compiler, _can_assign: bool) {
    let end_jump = compiler.emit_jump(OpCode::JumpIfFalse(0));

    compiler.emit_op(OpCode::Pop);
    compiler.parse_precedence(Precedence::And);

    compiler.patch_jump(end_jump, OpCode::JumpIfFalse(0));
}

fn or(compiler: &mut Compiler, _can_assign: bool) {
    let else_jump = compiler.emit_jump(OpCode::JumpIfFalse(0));
    let end_jump = compiler.emit_jump(OpCode::Jump(0));

    compiler.patch_jump(else_jump, OpCode::JumpIfFalse(0));
    compiler.emit_op(OpCode::Pop);

    compiler.parse_precedence(Precedence::Or);
    compiler.patch_jump(end_jump, OpCode::Jump(0));
}

fn call(compiler: &mut Compiler, _can_assign: bool) {
    let arg_count = compiler.argument_list();
    compiler.emit_op(OpCode::Call(arg_count))
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

type RuleFn = &'static dyn Fn(&mut Compiler, bool);

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

#[derive(PartialEq)]
pub enum FunctionKind {
    Function,
    Script,
}

struct FunctionScope<'a> {
    function: Function,
    kind: FunctionKind,

    locals: Vec<Local<'a>>,
    scope_depth: isize,
}

impl FunctionScope<'_> {
    pub fn new(function_name: &str, kind: FunctionKind) -> Self {
        Self {
            locals: Vec::new(),
            scope_depth: 0,
            function: Function::new(function_name),
            kind,
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
