use std::str::Chars;

pub struct Scanner<'a> {
    file: &'a str,
    start: usize,
    current: usize,
    line: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    //Single Character Tokens
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Plus,
    SemiColon,
    Slash,
    Star,
    Percent,
    Caret,

    //One or two character tokens
    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Dot,
    Range, //..
    Minus,
    Arrow,    //->
    Pipe,     // |
    PipePipe, // ||
    Amp,      // &
    AmpAmp,   // &&

    //Literal
    Identifier(String),
    StringLiteral(String),
    Integer(isize),
    Real(f64),

    //keywords
    Struct,
    If,
    Else,
    True,
    False,
    For,
    In,
    Proc,
    SelfKeyword,
    While,
    Let,
    Do,
    End,
    IntKeyword,
    RealKeyword,

    Error(String),
    EOF,
}

#[derive(Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    //value: String,
    pub line: usize,
}

impl Token {
    pub fn new(kind: TokenKind, line: usize) -> Self {
        Self { kind, line }
    }
}

impl<'a> Scanner<'a> {
    pub fn new(file: &'a str) -> Scanner<'a> {
        Self {
            file: file,
            start: 0,
            current: 0,
            line: 1,
        }
    }

    pub fn scan_token(&mut self) -> Token {
        self.skip_white_space();

        self.start = self.current;

        if self.is_at_end() {
            return self.make_token(TokenKind::EOF);
        }

        let c = self.advance();
        match c {
            '(' => return self.make_token(TokenKind::LeftParen),
            ')' => return self.make_token(TokenKind::RightParen),
            '{' => return self.make_token(TokenKind::LeftBrace),
            '}' => return self.make_token(TokenKind::RightBrace),
            '+' => return self.make_token(TokenKind::Plus),
            ';' => return self.make_token(TokenKind::SemiColon),
            '/' => return self.make_token(TokenKind::Slash),
            '*' => return self.make_token(TokenKind::Star),
            '%' => return self.make_token(TokenKind::Percent),
            '^' => return self.make_token(TokenKind::Caret),
            '"' => return self.string(),
            '0'..='9' => return self.number(),
            'a'..='z' | 'A'..='Z' | '_' => return self.identifier(),
            // double characters
            '!' => {
                let next = self.peek();
                if next == '=' {
                    let _ = self.advance();
                    return self.make_token(TokenKind::BangEqual);
                }
                return self.make_token(TokenKind::Bang);
            }
            '=' => {
                let next = self.peek();
                if next == '=' {
                    let _ = self.advance();
                    return self.make_token(TokenKind::EqualEqual);
                }
                return self.make_token(TokenKind::Equal);
            }
            '<' => {
                let next = self.peek();
                if next == '=' {
                    let _ = self.advance();
                    return self.make_token(TokenKind::LessEqual);
                }
                return self.make_token(TokenKind::Less);
            }
            '>' => {
                let next = self.peek();
                if next == '=' {
                    let _ = self.advance();
                    return self.make_token(TokenKind::GreaterEqual);
                }
                return self.make_token(TokenKind::Greater);
            }
            '.' => {
                let next = self.peek();
                if next == '.' {
                    let _ = self.advance();
                    return self.make_token(TokenKind::Range);
                }
                return self.make_token(TokenKind::Dot);
            }
            '-' => {
                let next = self.peek();
                if next == '>' {
                    let _ = self.advance();
                    return self.make_token(TokenKind::Arrow);
                }
                return self.make_token(TokenKind::Minus);
            }
            '|' => {
                let next = self.peek();
                if next == '|' {
                    let _ = self.advance();
                    return self.make_token(TokenKind::PipePipe);
                }
                return self.make_token(TokenKind::Pipe);
            }
            '&' => {
                let next = self.peek();
                if next == '&' {
                    let _ = self.advance();
                    return self.make_token(TokenKind::AmpAmp);
                }
                return self.make_token(TokenKind::Amp);
            }


            _ => (),
        }

        return self.error_token("unexpected character");
    }

    fn is_at_end(&self) -> bool {
        return self.current == self.file.len();
    }

    fn make_token(&mut self, tok: TokenKind) -> Token {
        let t = Token::new(tok, self.line);
        t
    }

    fn error_token(&mut self, text: &str) -> Token {
        let t = Token::new(TokenKind::Error(text.to_string()), self.line);
        t
    }

    pub fn advance(&mut self) -> char {
        self.current += 1;
        let r = &self.file[self.current - 1..self.current];
        let mut r = r.chars();
        return r.next().unwrap();
    }

    fn peek(&self) -> char {
        if self.is_at_end() {
            return '\0';
        }
        let r = &self.file[self.current..];
        let mut r = r.chars();
        return match r.next() {
            Some(x) => x,
            None => '\0',
        };
    }

    fn peek_next(&self) -> char {
        if self.is_at_end() {
            return '\0';
        }
        let r = &self.file[self.current + 1..];
        let mut r = r.chars();
        return match r.next() {
            Some(x) => x,
            None => '\0',
        };
    }

    fn skip_white_space(&mut self) {
        loop {
            let c = self.peek();

            match c {
                ' ' | '\t' | '\r' => {
                    self.advance();
                }
                '\n' => {
                    self.line += 1;
                    self.advance();
                }
                '/' => {
                    if self.peek_next() == '/' {
                        while self.peek() != '\n' && !self.is_at_end() {
                            self.advance();
                        }
                    } else {
                        return;
                    }
                }
                _ => {
                    return;
                }
            };
        }
    }

    fn string(&mut self) -> Token {
        self.start = self.current;
        while self.peek() != '"' && !self.is_at_end() {
            if self.peek() == '\n' {
                self.line += 1;
            }
            self.advance();
        }

        if self.is_at_end() {
            return self.error_token("Unterminated string");
        }

        let s = &self.file[self.start..self.current];
        let s = s.to_string();
        self.advance();
        return self.make_token(TokenKind::StringLiteral(s));
    }

    fn is_digit(&self, c: char) -> bool {
        c <= '9' && c >= '0'
    }

    fn is_alpha(&self, c: char) -> bool {
        (c <= 'z' && c >= 'a') || (c <= 'Z' && c >= 'A') || (c == '_')
    }

    fn number(&mut self) -> Token {
        self.start = self.current - 1;
        let mut is_real = false;
        while self.is_digit(self.peek()) {
            self.advance();
        }

        if self.peek() == '.' && self.is_digit(self.peek_next()) {
            self.advance();
            is_real = true;

            while self.is_digit(self.peek()) {
                self.advance();
            }
        }

        let s = &self.file[self.start..self.current];
        let s = s.to_string();
        if !is_real {
            return self.make_token(TokenKind::Integer(s.parse().expect("should be an integer")));
        } else {
            return self.make_token(TokenKind::Real(s.parse().expect("should be a float")));
        }
    }

    fn identifier(&mut self) -> Token {
        self.start = self.current - 1;

        while self.is_alpha(self.peek()) || self.is_digit(self.peek()) {
            self.advance();
        }

        let s = &self.file[self.start..self.current];
        return self.make_token(self.identifier_type(s));
    }

    fn identifier_type(&self, s: &str) -> TokenKind {
        let mut s = s.chars();
        match s.next().unwrap_or('\0') {
            's' => {
                match s.next().unwrap_or('\0') {
                    't' => return self.check_keyword(s, "ruct", TokenKind::Struct),
                    'e' => return self.check_keyword(s, "lf", TokenKind::SelfKeyword),
                    _ => (),
                };
            }
            'i' => {
                match s.next().unwrap_or('\0') {
                    'f' => return TokenKind::If,
                    'n' => match s.next().unwrap_or('\0') {
                        '\0' =>
                            return TokenKind::In,
                            't' => return TokenKind::IntKeyword,
                            _ => ()
                    }
                    _ => (),
                };
            }
            'e' => match s.next().unwrap_or('\0') {
                'l' => return self.check_keyword(s, "se", TokenKind::Else),
                'n' => return self.check_keyword(s, "d", TokenKind::End),
                _ => (),
            },
            't' => return self.check_keyword(s, "rue", TokenKind::True),
            'f' => {
                match s.next().unwrap_or('\0') {
                    'a' => return self.check_keyword(s, "lse", TokenKind::False),
                    'o' => return self.check_keyword(s, "r", TokenKind::For),
                    _ => (),
                }
            },
            'p' => return self.check_keyword(s, "roc", TokenKind::Proc),
            'w' => return self.check_keyword(s, "hile", TokenKind::While),
            'l' => return self.check_keyword(s, "et", TokenKind::Let),
            'd' => return self.check_keyword(s, "o", TokenKind::Do),
            'r' => return self.check_keyword(s, "eal", TokenKind::RealKeyword),
            _ => (),
        }
        let s = &self.file[self.start..self.current];
        let s = s.to_string();
        return TokenKind::Identifier(s);
    }

    fn check_keyword(&self, iter: Chars, s: &str, t: TokenKind) -> TokenKind {
        let iter = iter.as_str();
        if iter == s {
            return t;
        }

        let s = &self.file[self.start..self.current];
        let s = s.to_string();
        return TokenKind::Identifier(s);
    }
}
