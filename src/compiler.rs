use crate::scanner::Scanner;
use crate::scanner::TokenKind;
use crate::scanner::Token;
use crate::chunk::Chunk;


pub struct Parser<'a> {
    current: Option<Token>,
    pub previous: Option<Token>,
    had_error: bool,
    panic_mode: bool,
    scanner: &'a mut Scanner<'a>,
}

impl<'a> Parser<'a> {
    fn new(s: &'a mut Scanner<'a>) -> Parser<'a> {
        Self {
            previous: None,
            current: None,
            had_error: false,
            panic_mode: false,
            scanner: s
        }
    }

    fn advance(&mut self) {
        self.previous = self.current.clone();

        loop {
            self.current = Some(self.scanner.scan_token());

            if let Some(tok) = self.current.clone() {
                if let TokenKind::Error(msg) = &tok.kind {
                    self.error_at_current(&msg);
                }

                break;
            }
        }
    }

    fn error_at_current(&mut self, msg: &str){
        let tok = self.current.clone().unwrap();
        self.error_at(&tok, msg);
    }

    pub fn error(&mut self, tok: &Token, msg: &str){
        self.error_at(tok, msg);
    }
    fn error_at(&mut self, tok: &Token, msg: &str) {
        if self.panic_mode {
            return;
        }

        self.panic_mode = true;

        eprint!("[line {}] Error", tok.line);

        match tok.kind {
            TokenKind::EOF => {
                eprintln!(" at end");
            }
            TokenKind::Error(_) => (),
            _ => eprint!(" at {:?}", tok.kind),
        };

        eprintln!(": {}", msg);
        self.had_error = true;
    }

    fn expression(&self){
    }

    fn consume(&mut self, kind: TokenKind, msg: &str){
        if let Some(k) = &self.current {
            if k.kind == kind {
                self.advance();
                return;
            }

            self.error_at_current(msg);
        }
    }
}


pub fn compile(source: String, chunk: &mut Chunk) -> bool{
    let mut scanner = Scanner::new(&source);
    let mut parser = Parser::new(&mut scanner);
    let compiling_chunk = chunk;
    parser.advance();
    parser.expression();
    parser.consume(TokenKind::EOF, "Expect end of expression");

    compiling_chunk.end_chunk(&parser);

    !parser.had_error
}
