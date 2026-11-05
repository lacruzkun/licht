use crate::error::{ErrorKind, LichtError, Pos};

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    // keywords
    Fn,
    Let,
    If,
    Else,
    For,
    In,
    Return,
    True,
    False,
    // names and literals
    Ident(String),
    Int(i64),
    Float(f64),
    Str(String),
    // arithmetic operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    // comparison operators
    Lt,
    Gt,
    Le,
    Ge,
    EqEq,
    Ne,
    // logical operators
    AndAnd,
    OrOr,
    Bang,
    // assignment and return arrow
    Assign,
    Arrow,
    // punctuation
    Comma,
    Colon,
    Semi,
    // delimiters
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Eof,
}

impl Tok {
    /// A short description used in syntax error messages.
    pub fn describe(&self) -> String {
        match self {
            Tok::Ident(name) => format!("identifier '{}'", name),
            Tok::Int(v) => format!("integer {}", v),
            Tok::Float(v) => format!("float {:?}", v),
            Tok::Str(s) => format!("string \"{}\"", s),
            Tok::Eof => "end of input".to_string(),
            other => format!("'{}'", other.text()),
        }
    }

    fn text(&self) -> &'static str {
        match self {
            Tok::Fn => "fn",
            Tok::Let => "let",
            Tok::If => "if",
            Tok::Else => "else",
            Tok::For => "for",
            Tok::In => "in",
            Tok::Return => "return",
            Tok::True => "true",
            Tok::False => "false",
            Tok::Plus => "+",
            Tok::Minus => "-",
            Tok::Star => "*",
            Tok::Slash => "/",
            Tok::Percent => "%",
            Tok::Lt => "<",
            Tok::Gt => ">",
            Tok::Le => "<=",
            Tok::Ge => ">=",
            Tok::EqEq => "==",
            Tok::Ne => "!=",
            Tok::AndAnd => "&&",
            Tok::OrOr => "||",
            Tok::Bang => "!",
            Tok::Assign => "=",
            Tok::Arrow => "->",
            Tok::Comma => ",",
            Tok::Colon => ":",
            Tok::Semi => ";",
            Tok::LParen => "(",
            Tok::RParen => ")",
            Tok::LBracket => "[",
            Tok::RBracket => "]",
            Tok::LBrace => "{",
            Tok::RBrace => "}",
            Tok::Ident(_) | Tok::Int(_) | Tok::Float(_) | Tok::Str(_) | Tok::Eof => "",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Token {
    pub tok: Tok,
    pub pos: Pos,
}

fn lex_error(message: impl Into<String>, pos: Pos) -> LichtError {
    LichtError::new(ErrorKind::Lexical, message, pos)
}

/// Converts source text into tokens. The final token is always `Tok::Eof`.
pub fn lex(src: &str) -> Result<Vec<Token>, LichtError> {
    let chars: Vec<char> = src.chars().collect();
    let mut tokens: Vec<Token> = Vec::new();
    let mut i = 0usize;
    let mut line = 1usize;
    let mut col = 1usize;

    while i < chars.len() {
        let c = chars[i];
        let pos = Pos::new(line, col);

        // whitespace
        if c == '\n' {
            i += 1;
            line += 1;
            col = 1;
            continue;
        }
        if c.is_whitespace() {
            i += 1;
            col += 1;
            continue;
        }

        // comments: // to the end of the line
        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
                col += 1;
            }
            continue;
        }

        // integer and float literals
        if c.is_ascii_digit() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let mut is_float = false;
            if i < chars.len() && chars[i] == '.' {
                if i + 1 < chars.len() && chars[i + 1].is_ascii_digit() {
                    is_float = true;
                    i += 1;
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                } else {
                    return Err(lex_error(
                        "malformed number: digits are required on both sides of the decimal point",
                        pos,
                    ));
                }
            }
            let text: String = chars[start..i].iter().collect();
            col += i - start;
            if is_float {
                match text.parse::<f64>() {
                    Ok(v) => tokens.push(Token { tok: Tok::Float(v), pos }),
                    Err(_) => return Err(lex_error("malformed float literal", pos)),
                }
            } else {
                match text.parse::<i64>() {
                    Ok(v) => tokens.push(Token { tok: Tok::Int(v), pos }),
                    Err(_) => return Err(lex_error("integer literal is too large", pos)),
                }
            }
            continue;
        }

        // identifiers and keywords
        if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            col += i - start;
            let tok = match text.as_str() {
                "fn" => Tok::Fn,
                "let" => Tok::Let,
                "if" => Tok::If,
                "else" => Tok::Else,
                "for" => Tok::For,
                "in" => Tok::In,
                "return" => Tok::Return,
                "true" => Tok::True,
                "false" => Tok::False,
                _ => Tok::Ident(text),
            };
            tokens.push(Token { tok, pos });
            continue;
        }

        // string literals
        if c == '"' {
            i += 1;
            col += 1;
            let mut s = String::new();
            let mut closed = false;
            while i < chars.len() {
                let ch = chars[i];
                if ch == '"' {
                    i += 1;
                    col += 1;
                    closed = true;
                    break;
                }
                if ch == '\\' {
                    if i + 1 >= chars.len() {
                        break;
                    }
                    let esc = chars[i + 1];
                    match esc {
                        'n' => s.push('\n'),
                        't' => s.push('\t'),
                        'r' => s.push('\r'),
                        '\\' => s.push('\\'),
                        '"' => s.push('"'),
                        other => {
                            return Err(lex_error(
                                format!("unknown escape sequence '\\{}'", other),
                                Pos::new(line, col),
                            ));
                        }
                    }
                    i += 2;
                    col += 2;
                    continue;
                }
                if ch == '\n' {
                    s.push('\n');
                    i += 1;
                    line += 1;
                    col = 1;
                    continue;
                }
                s.push(ch);
                i += 1;
                col += 1;
            }
            if !closed {
                return Err(lex_error("unterminated string literal", pos));
            }
            tokens.push(Token { tok: Tok::Str(s), pos });
            continue;
        }

        // operators and punctuation: look at the next character to tell
        // `=` from `==`, `-` from `->`, `!` from `!=`, and so on
        let next = if i + 1 < chars.len() {
            Some(chars[i + 1])
        } else {
            None
        };
        let (tok, len) = match (c, next) {
            ('=', Some('=')) => (Tok::EqEq, 2),
            ('=', _) => (Tok::Assign, 1),
            ('!', Some('=')) => (Tok::Ne, 2),
            ('!', _) => (Tok::Bang, 1),
            ('<', Some('=')) => (Tok::Le, 2),
            ('<', _) => (Tok::Lt, 1),
            ('>', Some('=')) => (Tok::Ge, 2),
            ('>', _) => (Tok::Gt, 1),
            ('&', Some('&')) => (Tok::AndAnd, 2),
            ('|', Some('|')) => (Tok::OrOr, 2),
            ('-', Some('>')) => (Tok::Arrow, 2),
            ('-', _) => (Tok::Minus, 1),
            ('+', _) => (Tok::Plus, 1),
            ('*', _) => (Tok::Star, 1),
            ('/', _) => (Tok::Slash, 1),
            ('%', _) => (Tok::Percent, 1),
            (',', _) => (Tok::Comma, 1),
            (':', _) => (Tok::Colon, 1),
            (';', _) => (Tok::Semi, 1),
            ('(', _) => (Tok::LParen, 1),
            (')', _) => (Tok::RParen, 1),
            ('[', _) => (Tok::LBracket, 1),
            (']', _) => (Tok::RBracket, 1),
            ('{', _) => (Tok::LBrace, 1),
            ('}', _) => (Tok::RBrace, 1),
            ('&', _) => return Err(lex_error("invalid character '&' (did you mean '&&'?)", pos)),
            ('|', _) => return Err(lex_error("invalid character '|' (did you mean '||'?)", pos)),
            _ => return Err(lex_error(format!("invalid character '{}'", c), pos)),
        };
        tokens.push(Token { tok, pos });
        i += len;
        col += len;
    }

    tokens.push(Token {
        tok: Tok::Eof,
        pos: Pos::new(line, col),
    });
    Ok(tokens)
}
