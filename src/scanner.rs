pub struct Scanner {
     start: usize,
     current: usize,
     line: usize,
     no_of_line: usize,
}

enum TokenKind {
    //Single Character Tokens
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Dot,
    Minus,
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

    //Literal
    Identifier,
    StringLiteral,
    Number,

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

    Error,
    EOF,
}

struct Token{
    kind: TokenKind,
    //value: String,
    line: usize,
}

impl Token {
    pub fn new(kind: TokenKind, value: String, line: usize) -> Self {
        Self {kind, line}
    }
}

impl Scanner {
    pub fn new(ln: usize) -> Self{
        Self {
            start: 0, 
            current: 0,
            line: 1,
            no_of_line: ln,
        }
    }

    pub fn scan_token(&mut self) -> Token {
        self.start = self.current;

        if self.is_at_end() {
            return self.make_token(TokenKind::EOF);
        }

        return self.error_token("unexpected character");
    }

    fn is_at_end(&self) -> bool {
        return self.current == self.no_of_line;
    }

    fn make_token(&mut self, tok: TokenKind) -> Token {
        let t = Token::new(tok, "".to_string(), self.line);
        t
    }

    fn error_token(&mut self, text: &str) -> Token {
        let t = Token::new(TokenKind::Error, text.to_string(), self.line);
        t
    }
}
