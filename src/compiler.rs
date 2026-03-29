use crate::scanner::Scanner;

pub fn comipile(source: String){
    let scanner = Scanner::new();

    let line = -1;

    loop {
        let token = scanner.scan_token();
        if token.line != line {
            print!("{:4} ", token.line);
            line = token.line;
        }else {
            print!("    |");
        }

        println!("{:?} {}", token.type, token.value);

        if token.type == TokenKind::EOF{
            break;
        }
    }
}
