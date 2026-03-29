use crate::scanner::Scanner;
use crate::scanner::TokenKind;

pub fn compile(source: String) {
    let mut scanner = Scanner::new(&source);

    let mut line = 0;

    loop {
        let token = scanner.scan_token();
        if token.line != line || token.line == 0 {
            print!("{:4} ", token.line);
            line = token.line;
        } else {
            print!("    |");
        }

        println!("{:?}", token.kind);

        if token.kind == TokenKind::EOF {
            break;
        }
    }
}
