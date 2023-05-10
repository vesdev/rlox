use crate::compiler::scanner::*;
use crate::error::*;
use crate::vm::chunk::*;
use crate::vm::object::*;
use crate::vm::opcode::OpCode;
use crate::vm::value::Value;
use std::fmt::Write;
use std::rc::Rc;

mod scanner;

pub struct State<'a> {
    panic_mode: bool,
    function: FunDescriptor,
    kind: FunctionKind,
    scope_depth: isize,
    locals: Vec<Local<'a>>,
    errors: Vec<Error>,
}

impl<'a> State<'a> {
    pub fn new(function_name: impl Into<String>, kind: FunctionKind) -> Self {
        let local = Local::new(
            Token::new(
                TokenKind::Fun,
                if kind == FunctionKind::Method || kind == FunctionKind::Initializer {
                    "this"
                } else {
                    ""
                },
                0,
            ),
            0,
        );
        Self {
            panic_mode: false,
            errors: Vec::new(),

            locals: vec![local],
            scope_depth: 0,
            function: FunDescriptor::new(function_name.into()),
            kind,
        }
    }

    fn chunk(&mut self) -> &mut Chunk {
        &mut self.function.chunk
    }
}

pub struct Compiler<'a> {
    states: Vec<State<'a>>,
    scanner: Scanner<'a>,
    previous: Token<'a>,
    current: Token<'a>,
    classes: Vec<ClassScope>,
}

impl<'a> Compiler<'a> {
    pub fn compile(&mut self) -> Result<FunDescriptor, Vec<Error>> {
        self.state().panic_mode = false;
        self.advance();

        while !self.matches(TokenKind::Eof) {
            self.declaration();
        }

        self.end()
    }

    pub fn new(source: &'a str, state: State<'a>) -> Compiler<'a> {
        Self {
            scanner: Scanner::new(source),
            states: vec![state],
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
            classes: Vec::new(),
        }
    }

    fn state(&mut self) -> &mut State<'a> {
        self.states.last_mut().unwrap()
    }

    fn state_ref(&self) -> &State<'a> {
        self.states.last().unwrap()
    }

    fn state_enclosing(&mut self) -> &mut State<'a> {
        self.states.last_mut().unwrap()
    }

    fn class(&mut self) -> &mut ClassScope {
        self.classes.last_mut().unwrap()
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

    fn consume(&mut self, kind: TokenKind, message: impl Into<String>) {
        if self.current.kind == kind {
            self.advance();
            return;
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

    fn end(&mut self) -> Result<FunDescriptor, Vec<Error>> {
        self.emit_return();

        if cfg!(disassemble) {
            let mut name = "Entry Point".to_string();
            if !self.state().function.name.is_empty() {
                name = self.state().function.name.clone();
            }
            println!("{}", self.state().chunk().disassemble(name).unwrap());
        }

        if !self.state().errors.is_empty() {
            return Err(std::mem::take(&mut self.state().errors));
        }
        Ok(std::mem::take(&mut self.state().function))
    }

    fn begin_scope(&mut self) {
        self.state().scope_depth += 1;
    }

    fn end_scope(&mut self, close_upvalues: bool) {
        self.state().scope_depth -= 1;
        while !self.state().locals.is_empty()
            && self.state().locals.last().unwrap().depth > self.state().scope_depth
        {
            let local = self.state().locals.pop().unwrap();

            if close_upvalues && local.is_captured {
                self.emit_op(OpCode::CloseUpValue);
            } else {
                self.emit_op(OpCode::Pop);
            }
        }
    }

    fn parse_precedence(&mut self, precendence: Precedence) {
        self.advance();

        if let Some(prefix) = get_rule(self.previous.kind).prefix {
            let can_assign = precendence <= Precedence::Assignment;
            prefix(self, can_assign);

            while precendence <= get_rule(self.current.kind).precedence {
                self.advance();

                if let Some(infix) = get_rule(self.previous.kind).infix {
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

    fn identifiers_equal(a: Token, b: Token) -> bool {
        a.lexeme == b.lexeme
    }

    fn resolve_local(state: &mut State, name: Token) -> Option<usize> {
        for i in (0..state.locals.len()).rev() {
            let local = state.locals[i].clone();
            if Self::identifiers_equal(name, local.name) {
                if local.depth == -1 {
                    return None;
                }
                return Some(i);
            }
        }

        None
    }

    fn add_upvalue(state: &mut State, index: usize, is_local: bool) -> usize {
        let upvalue_count = state.function.upvalues.len();

        for i in 0..upvalue_count {
            let upvalue = &state.function.upvalues[i];
            if upvalue.index == index && upvalue.is_local == is_local {
                return i;
            }
        }

        state
            .function
            .upvalues
            .push(UpValueDescriptor { index, is_local });
        upvalue_count
    }

    fn resolve_upvalue(&mut self, state_index: usize, name: Token) -> Option<usize> {
        if state_index < 1 {
            return None;
        }
        let enclosing_index = state_index - 1;

        if let Some(index) = Self::resolve_local(&mut self.states[enclosing_index], name) {
            self.states[enclosing_index].locals[index].is_captured = true;
            return Some(Self::add_upvalue(
                &mut self.states[state_index],
                index,
                true,
            ));
        } else if let Some(index) = self.resolve_upvalue(enclosing_index, name) {
            return Some(Self::add_upvalue(
                &mut self.states[state_index],
                index,
                false,
            ));
        }

        println!("unfound upvalue: {}", name.lexeme);
        None
    }

    fn add_local(&mut self, name: Token<'a>) {
        let local = Local::new(name, self.state().scope_depth);
        self.state().locals.push(local);
    }

    fn declare_variable(&mut self) {
        if self.state().scope_depth == 0 {
            return;
        }

        let name = self.previous;

        for i in (0..self.state().locals.len()).rev() {
            let local = &self.state_ref().locals[i];

            if local.depth != -1 && local.depth < self.state_ref().scope_depth {
                break;
            }

            if Self::identifiers_equal(name, local.name) {
                return self.error("Already a variable with this name in this scope.");
            }
        }

        self.add_local(name)
    }

    fn parse_variable(&mut self, message: impl Into<String>) -> usize {
        self.consume(TokenKind::Identifier, message);

        self.declare_variable();
        if self.state().scope_depth > 0 {
            return 0;
        }

        self.identifier_constant(self.previous)
    }

    fn mark_initialized(&mut self) {
        if self.state().scope_depth == 0 {
            return;
        }
        let index = self.state().locals.len() - 1;
        self.state().locals[index].depth = self.state().scope_depth;
    }

    fn define_variable(&mut self, global: usize) {
        if self.state().scope_depth > 0 {
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
        self.states
            .push(State::new(self.previous.lexeme.to_string(), kind));
        self.begin_scope();

        self.consume(TokenKind::LeftParen, "Expect '(' after function name.");

        if !self.check(TokenKind::RightParen) {
            loop {
                self.state().function.arity += 1;

                if self.state().function.arity > 256 {
                    self.error_at_current("Can't have more than 255 parameters.");
                }

                let constant = self.parse_variable("Expect parameter name.");
                self.define_variable(constant);

                if !self.matches(TokenKind::Comma) {
                    break;
                }
            }
        }

        self.consume(TokenKind::RightParen, "Expect ')' after parameters.");
        self.consume(TokenKind::LeftBrace, "Expect '{' before function body.");

        self.block();

        let result = self.end();

        self.states.pop();

        match result {
            Ok(result) => {
                let func = self.make_constant(Value::Obj(Obj::Fun(Rc::new(result))));
                self.emit_op(OpCode::Closure(func));
            }
            Err(mut e) => {
                //handle errors from nested functions recursively
                self.state().errors.append(&mut e);
            }
        }
    }

    fn method(&mut self) {
        self.consume(TokenKind::Identifier, "Expect method name.");
        let constant = self.identifier_constant(self.previous);

        self.function(if self.previous.lexeme == "init" {
            FunctionKind::Initializer
        } else {
            FunctionKind::Method
        });

        self.emit_op(OpCode::Method(constant));
    }

    fn class_declaration(&mut self) {
        self.consume(TokenKind::Identifier, "Expect class name.");
        let class_name = self.previous;
        let name_constant = self.identifier_constant(self.previous);
        self.declare_variable();

        self.emit_op(OpCode::Class(name_constant));
        self.define_variable(name_constant);

        self.classes.push(ClassScope::new());

        if self.matches(TokenKind::Less) {
            self.consume(TokenKind::Identifier, "Expect superclass name.");
            variable(self, false);
            if Self::identifiers_equal(class_name, self.previous) {
                self.error("A class can't inherit from itself.");
            }

            self.begin_scope();

            let token = Token::new(TokenKind::Super, "super", self.current.line);

            self.add_local(token);
            self.define_variable(0);

            named_variable(self, class_name, false);

            self.emit_op(OpCode::Inerhit);
            self.class().has_super_class = true;
        }

        named_variable(self, class_name, false);
        self.consume(TokenKind::LeftBrace, "Expect '{' before class body.");
        while !self.check(TokenKind::RightBrace) && !self.check(TokenKind::Eof) {
            self.method();
        }
        self.consume(TokenKind::RightBrace, "Expect '}' after class body.");

        if self.class().has_super_class {
            self.end_scope(true);
        }

        self.classes.pop();
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

        let mut loop_start = self.state().chunk().len();

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
            let increment_start = self.state().chunk().len();

            self.expression();
            self.emit_op(OpCode::Pop);
            self.consume(TokenKind::RightParen, "Expect ')' after for clauses.");

            self.emit_loop(loop_start);

            loop_start = increment_start;
            self.patch_jump(body_jump, OpCode::Jump(0));
        }

        self.statement();

        let scope_depth = self.state().scope_depth - 1;
        //manually handle closing upvalues
        //so each iteration will create a new copy
        for local in (0..self.state().locals.len()).rev() {
            if self.state().locals[local].depth <= scope_depth {
                break;
            }
            if self.state().locals[local].is_captured {
                self.emit_op(OpCode::CloseUpValue);
            }
        }

        self.emit_loop(loop_start);

        //condition
        if condition_exists {
            self.patch_jump(exit_jump, OpCode::JumpIfFalse(0));
            self.emit_op(OpCode::Pop);
        }

        self.end_scope(false);
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

    fn return_statement(&mut self) {
        if self.state().kind == FunctionKind::Script {
            self.error("Can't return from top-level code.");
        }

        if self.matches(TokenKind::Semicolon) {
            self.emit_return();
        } else {
            if self.state().kind == FunctionKind::Initializer {
                self.error("Cant't return a value from an initializer.");
            }
            self.expression();
            self.consume(TokenKind::Semicolon, "Expect ';' after return value.");
            self.emit_op(OpCode::Return);
        }
    }

    fn while_statement(&mut self) {
        let loop_start = self.state().chunk().len();
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
        self.state().panic_mode = false;

        while self.current.kind != TokenKind::Eof {
            if self.previous.kind == TokenKind::Semicolon {
                self.state();
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

    fn statement(&mut self) {
        if self.matches(TokenKind::Print) {
            self.print_statement();
        } else if self.matches(TokenKind::For) {
            self.for_statement();
        } else if self.matches(TokenKind::If) {
            self.if_statement();
        } else if self.matches(TokenKind::Return) {
            self.return_statement();
        } else if self.matches(TokenKind::While) {
            self.while_statement();
        } else if self.matches(TokenKind::LeftBrace) {
            self.begin_scope();
            self.block();
            self.end_scope(true);
        } else {
            self.expression_statement()
        }
    }

    fn declaration(&mut self) {
        // ignore self.errors on this level
        // manually synchronized after
        if self.matches(TokenKind::Class) {
            self.class_declaration();
        } else if self.matches(TokenKind::Fun) {
            self.fun_declaration();
        } else if self.matches(TokenKind::Var) {
            self.var_declaration();
        } else {
            self.statement();
        }

        if self.state().panic_mode {
            self.synchronize();
        }
    }

    fn emit_op(&mut self, op: OpCode) {
        let line = self.previous.line;
        self.state().chunk().push_op(op, line)
    }

    fn emit_ops(&mut self, op: OpCode, op2: OpCode) {
        self.emit_op(op);
        self.emit_op(op2);
    }

    fn emit_loop(&mut self, loop_start: usize) {
        let offset = self.state().chunk().len() - loop_start;
        self.emit_op(OpCode::Loop(offset));
    }

    fn emit_jump(&mut self, op: OpCode) -> usize {
        self.emit_op(op);
        self.state().chunk().len() - 1
    }

    fn emit_return(&mut self) {
        if self.state().kind == FunctionKind::Initializer {
            self.emit_op(OpCode::GetLocal(0));
        } else {
            self.emit_op(OpCode::Nil);
        }

        self.emit_op(OpCode::Return)
    }

    fn make_constant(&mut self, value: Value) -> usize {
        let constant = self.state().chunk().push_constant(value);

        constant
    }

    fn emit_constant(&mut self, value: Value) {
        let constant = self.make_constant(value);
        self.emit_op(OpCode::Constant(constant));
    }

    fn patch_jump(&mut self, offset: usize, op: OpCode) {
        let jump = self.state().chunk().len() - offset;

        match op {
            OpCode::JumpIfFalse(_) => {
                self.state()
                    .chunk()
                    .insert_op(OpCode::JumpIfFalse(jump), offset);
            }
            OpCode::Jump(_) => {
                self.state().chunk().insert_op(OpCode::Jump(jump), offset);
            }
            _ => (),
        }
    }

    fn error_at_current(&mut self, message: impl Into<String>) {
        self.error_at(self.current, message)
    }

    fn error(&mut self, message: impl Into<String>) {
        self.error_at(self.previous, message)
    }

    fn error_at(&mut self, token: Token, message: impl Into<String>) {
        if self.state().panic_mode {
            return;
        }
        self.state().panic_mode = true;

        let mut out = String::new();
        write!(
            out,
            "[line {}, {}] Error",
            token.line,
            self.state().function
        )
        .unwrap();

        if token.kind == TokenKind::Eof {
            write!(out, " at end").unwrap();
        } else if token.kind == TokenKind::Error {
        } else {
            write!(out, " at '{}'", token.lexeme).unwrap();
        }

        writeln!(out, ": {}", message.into()).unwrap();

        let err = Error::Compile(out, token.line);
        self.state().errors.push(err);
    }
}

fn get_rule(kind: TokenKind) -> Rule {
    match kind {
        TokenKind::LeftParen => Rule::new(Some(&grouping), Some(&call), Precedence::Call),
        TokenKind::RightParen => Rule::new(None, None, Precedence::None),
        TokenKind::LeftBrace => Rule::new(None, None, Precedence::None),
        TokenKind::RightBrace => Rule::new(None, None, Precedence::None),
        TokenKind::Comma => Rule::new(None, None, Precedence::None),
        TokenKind::Dot => Rule::new(None, Some(&dot), Precedence::Call),
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
        TokenKind::Super => Rule::new(Some(&super_), None, Precedence::None),
        TokenKind::This => Rule::new(Some(&this), None, Precedence::None),
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
    let operator_kind = compiler.previous.kind;

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
    let value = compiler.previous.lexeme.parse::<f64>().unwrap();
    compiler.emit_constant(Value::Number(value))
}

fn unary(compiler: &mut Compiler, _can_assign: bool) {
    let operator_kind = compiler.previous.kind;

    compiler.parse_precedence(Precedence::Unary);

    match operator_kind {
        TokenKind::Bang => compiler.emit_op(OpCode::Not),
        TokenKind::Minus => compiler.emit_op(OpCode::Negate),
        _ => {}
    }
}

fn literal(compiler: &mut Compiler, _can_assign: bool) {
    match compiler.previous.kind {
        TokenKind::False => compiler.emit_op(OpCode::False),
        TokenKind::Nil => compiler.emit_op(OpCode::Nil),
        TokenKind::True => compiler.emit_op(OpCode::True),
        _ => {}
    }
}

fn string(compiler: &mut Compiler, _can_assign: bool) {
    compiler.emit_constant(Value::Obj(Obj::String(
        compiler
            .previous
            .lexeme
            .trim_matches('"')
            .replace("\\n", "\n"),
    )))
}

fn variable(compiler: &mut Compiler, can_assign: bool) {
    named_variable(compiler, compiler.previous, can_assign)
}

fn named_variable(compiler: &mut Compiler, name: Token, can_assign: bool) {
    let (get_op, set_op);

    if let Some(arg) = Compiler::resolve_local(compiler.state(), name) {
        get_op = OpCode::GetLocal(arg);
        set_op = OpCode::SetLocal(arg);
    } else if let Some(arg) = compiler.resolve_upvalue(compiler.states.len() - 1, name) {
        get_op = OpCode::GetUpValue(arg);
        set_op = OpCode::SetUpValue(arg);
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

fn dot(compiler: &mut Compiler, can_assign: bool) {
    compiler.consume(TokenKind::Identifier, "Expect property name after '.'.");
    let name = compiler.identifier_constant(compiler.previous);

    if can_assign && compiler.matches(TokenKind::Equal) {
        compiler.expression();
        compiler.emit_op(OpCode::SetProperty(name));
    } else if compiler.matches(TokenKind::LeftParen) {
        let arg_count = compiler.argument_list();
        compiler.emit_op(OpCode::Invoke(name, arg_count));
    } else {
        compiler.emit_op(OpCode::GetProperty(name));
    }
}

fn this(compiler: &mut Compiler, _can_assign: bool) {
    if compiler.classes.is_empty() {
        compiler.error("Can't use 'this' outside of a class.");
        return;
    }
    variable(compiler, false);
}

fn super_(compiler: &mut Compiler, _can_assign: bool) {
    if compiler.classes.is_empty() {
        compiler.error("Can't use 'super' outside of a class.");
    } else if !compiler.class().has_super_class {
        compiler.error("Can't use 'super' in a class with no superclass.");
    }

    compiler.consume(TokenKind::Dot, "Expect '.' after 'super'.");
    compiler.consume(TokenKind::Identifier, "Expect superclass method name.");
    let name = compiler.identifier_constant(compiler.previous);

    named_variable(
        compiler,
        Token::new(TokenKind::This, "this", compiler.previous.line),
        false,
    );

    if compiler.matches(TokenKind::LeftParen) {
        let arg_count = compiler.argument_list();
        named_variable(
            compiler,
            Token::new(TokenKind::Super, "super", compiler.previous.line),
            false,
        );
        compiler.emit_op(OpCode::SuperInvoke(name, arg_count));
    } else {
        named_variable(
            compiler,
            Token::new(TokenKind::Super, "super", compiler.previous.line),
            false,
        );
        compiler.emit_op(OpCode::GetSuper(name));
    }
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

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum FunctionKind {
    Function,
    Method,
    Initializer,
    Script,
}

#[derive(Clone)]
struct Local<'a> {
    pub name: Token<'a>,
    pub depth: isize,
    pub is_captured: bool,
}

impl<'a> Local<'a> {
    pub fn new(name: Token<'a>, depth: isize) -> Self {
        Self {
            name,
            depth,
            is_captured: false,
        }
    }
}

#[derive(Clone)]
struct ClassScope {
    pub has_super_class: bool,
}

impl ClassScope {
    pub fn new() -> Self {
        Self {
            has_super_class: false,
        }
    }
}
