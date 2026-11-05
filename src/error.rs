use std::fmt;

/// The four error categories described in Section 4.2.6.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Lexical,
    Syntax,
    Type,
    Runtime,
}

impl ErrorKind {
    pub fn label(&self) -> &'static str {
        match self {
            ErrorKind::Lexical => "Lexical error",
            ErrorKind::Syntax => "Syntax error",
            ErrorKind::Type => "Type error",
            ErrorKind::Runtime => "Runtime error",
        }
    }
}

/// A position in the source file (both numbers start at 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pos {
    pub line: usize,
    pub col: usize,
}

impl Pos {
    pub fn new(line: usize, col: usize) -> Pos {
        Pos { line, col }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LichtError {
    pub kind: ErrorKind,
    pub message: String,
    pub pos: Pos,
}

impl LichtError {
    pub fn new(kind: ErrorKind, message: impl Into<String>, pos: Pos) -> LichtError {
        LichtError {
            kind,
            message: message.into(),
            pos,
        }
    }

    /// Formats the error together with the offending source line and a caret.
    pub fn render(&self, source: &str) -> String {
        let mut out = self.to_string();
        if let Some(text) = source.lines().nth(self.pos.line.saturating_sub(1)) {
            let number = self.pos.line.to_string();
            let pad = " ".repeat(number.len());
            let caret_pad = " ".repeat(self.pos.col.saturating_sub(1));
            out.push_str(&format!(
                "\n {} | {}\n {} | {}^",
                number, text, pad, caret_pad
            ));
        }
        out
    }
}

impl fmt::Display for LichtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at line {}, column {}: {}",
            self.kind.label(),
            self.pos.line,
            self.pos.col,
            self.message
        )
    }
}

impl std::error::Error for LichtError {}
