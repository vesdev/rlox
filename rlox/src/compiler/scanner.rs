use std::str::Chars;

pub struct Scanner<'a> {
    source: &'a str,
    start: usize,
    current: usize,
    line: usize,
}

impl<'a> Scanner<'a> {
    pub fn new() -> Self {
        Self {
            source: "",
            start: 0,
            current: 0,
            line: 1,
        }
    }

    pub fn scan_token(&mut self) -> Token<'a> {
        self.skip_whitespace();
        self.start = self.current;

        if self.is_at_end() {
            return self.make_token(TokenKind::Eof);
        };
        let c = self.advance();

        if c.is_alphabetic() {
            return self.identifier();
        }
        if c.is_digit(10) {
            return self.number();
        }

        match c {
            '(' => return self.make_token(TokenKind::LeftParen),
            ')' => return self.make_token(TokenKind::RightParen),
            '{' => return self.make_token(TokenKind::LeftBrace),
            '}' => return self.make_token(TokenKind::RightBrace),
            ';' => return self.make_token(TokenKind::Semicolon),
            ',' => return self.make_token(TokenKind::Comma),
            '.' => return self.make_token(TokenKind::Dot),
            '-' => return self.make_token(TokenKind::Minus),
            '+' => return self.make_token(TokenKind::Plus),
            '/' => return self.make_token(TokenKind::Slash),
            '*' => return self.make_token(TokenKind::Star),
            '!' => {
                let kind = self.compare('=', TokenKind::BangEqual, TokenKind::Bang);
                return self.make_token(kind);
            }
            '=' => {
                let kind = self.compare('=', TokenKind::EqualEqual, TokenKind::Equal);
                return self.make_token(kind);
            }
            '<' => {
                let kind = self.compare('=', TokenKind::LessEqual, TokenKind::Less);
                return self.make_token(kind);
            }
            '>' => {
                let kind = self.compare('=', TokenKind::GreaterEqual, TokenKind::Greater);
                return self.make_token(kind);
            }
            '"' => return self.string(),
            _ => {}
        }

        self.error_token("unexpected character")
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }

    fn advance(&mut self) -> char {
        self.current += 1;
        return self.source.as_bytes()[self.current - 1] as char;
    }

    fn peek(&self) -> char {
        return self.source.as_bytes()[self.current] as char;
    }

    fn peek_next(&self) -> char {
        return self.source.as_bytes()[self.current + 1] as char;
    }

    fn lexeme(&self) -> &str {
        &self.source[self.start..self.current]
    }

    fn compare(&mut self, expected: char, kind_a: TokenKind, kind_b: TokenKind) -> TokenKind {
        if self.is_at_end() || self.peek() != expected {
            return kind_b;
        }

        self.current += 1;
        kind_a
    }

    fn make_token(&mut self, kind: TokenKind) -> Token<'a> {
        Token::new(kind, &self.source[self.start..self.current], self.line)
    }

    fn error_token(&mut self, message: &'a str) -> Token<'a> {
        Token::new(
            TokenKind::Error,
            &self.source[self.start..self.current],
            self.line,
        )
    }

    fn skip_whitespace(&mut self) {
        while !self.is_at_end() {
            let c = self.peek();
            if c.is_whitespace() {
                if c == '\n' {
                    self.line = 1;
                }
                self.advance();
            } else {
                if c == '/' && self.peek_next() == '/' {
                    while self.peek() != '\n' && !self.is_at_end() {
                        self.advance();
                    }
                }
                return;
            }
        }
    }

    fn string(&mut self) -> Token<'a> {
        while self.peek() != '"' && !self.is_at_end() {
            if self.peek() == '\n' {
                self.line += 1;
            }

            self.advance();
        }

        if self.is_at_end() {
            return self.error_token("Unterminated string");
        }

        self.advance();
        self.make_token(TokenKind::String)
    }

    fn number(&mut self) -> Token<'a> {
        while self.peek().is_digit(10) {
            self.advance();
        }

        if self.peek() == '.' && self.peek_next().is_digit(10) {
            self.advance();

            while self.peek().is_digit(10) {
                self.advance();
            }
        }

        return self.make_token(TokenKind::Number);
    }

    fn identifier(&mut self) -> Token<'a> {
        while self.peek().is_alphabetic() || self.peek().is_digit(10) {
            self.advance();
        }
        self.make_token(self.identifier_type())
    }

    fn identifier_type(&self) -> TokenKind {
        //copy pasted LULE
        match self.lexeme() {
            "and" => TokenKind::And,
            "class" => TokenKind::Class,
            "else" => TokenKind::Else,
            "false" => TokenKind::False,
            "for" => TokenKind::For,
            "fun" => TokenKind::Fun,
            "if" => TokenKind::If,
            "nil" => TokenKind::Nil,
            "or" => TokenKind::Or,
            "print" => TokenKind::Print,
            "return" => TokenKind::Return,
            "super" => TokenKind::Super,
            "this" => TokenKind::This,
            "true" => TokenKind::True,
            "var" => TokenKind::Var,
            "while" => TokenKind::While,
            _ => TokenKind::Identifier,
        }
    }
}

//copy pasted LULE
#[derive(Clone, Copy, Debug)]
pub struct Token<'a> {
    pub kind: TokenKind,
    pub lexeme: &'a str,
    pub line: usize,
}

impl<'a> Token<'a> {
    pub fn new(kind: TokenKind, lexeme: &'a str, line: usize) -> Token<'a> {
        Token { kind, lexeme, line }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TokenKind {
    LeftParen, // Single-character tokens.
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Dot,
    Minus,
    Plus,
    Semicolon,
    Slash,
    Star,
    Bang, // One or two character tokens.
    BangEqual,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    Identifier, // Literals.
    String,
    Number,
    And, // Keywords.
    Class,
    Else,
    False,
    For,
    Fun,
    If,
    Nil,
    Or,
    Print,
    Return,
    Super,
    This,
    True,
    Var,
    While,
    Error,
    Eof,
}
