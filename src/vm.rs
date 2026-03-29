use crate::chunk::Chunk;
use crate::chunk::Int;
use crate::chunk::OpCode;
use std::io::{self, Write};

pub struct VM {
    chunks: Chunk,
    ip: usize,
    stack: Vec<Int>,
}

pub enum InterpretResult {
    InterpretOk,
    InterpretCompileError,
    InterpretRuntimeError,
}

impl VM {
    pub fn new() -> Self {
        Self {
            chunks: Chunk::new(),
            ip: 0,
            stack: vec![],
        }
    }

    pub fn interpret(&mut self, source: String) -> InterpretResult {
        crate::compiler::compile(source);
        InterpretResult::InterpretOk
    }

    fn run(&mut self) -> InterpretResult {
        loop {
            print!("            ");
            for slot in &self.stack {
                print!("[ {} ]", slot);
            }
            println!();

            self.chunks.disassemble_instruction(self.ip);
            let instruction = self.chunks.code[self.ip];
            self.ip += 1;
            match instruction {
                OpCode::OpConstant(value) => {
                    self.stack.push(value);
                }
                OpCode::OpNegate => {
                    let popped = self.stack.pop().expect("Stack is not empty");
                    self.stack.push(-popped);
                }
                OpCode::OpReturn => {
                    return InterpretResult::InterpretOk;
                }
                OpCode::OpAdd => {
                    self.binary_op('+');
                }
                OpCode::OpSubtract => {
                    self.binary_op('-');
                }
                OpCode::OpMultiply => {
                    self.binary_op('*');
                }
                OpCode::OpDivide => {
                    self.binary_op('/');
                }
                OpCode::OpModulo => {
                    self.binary_op('%');
                }
                OpCode::OpExp => {
                    self.binary_op('^');
                }
            }
        }
    }
    fn binary_op(&mut self, op: char) {
        let b = self.stack.pop().expect("Stack is not empty");
        let a = self.stack.pop().expect("Stack is not empty");
        match op {
            '+' => self.stack.push(a + b),
            '-' => self.stack.push(a - b),
            '*' => self.stack.push(a * b),
            '/' => self.stack.push(a / b),
            '%' => self.stack.push(a % b),
            '^' => {
                if b < 0 {
                    println!("raising to a negative interger can result in fractions but variable is of type Int");
                }
                self.stack.push(a.pow(b as u32));
            }
            _ => {}
        };
    }
    pub fn repl(&mut self) {
        let mut line_number = 0;
        loop {
            print!("<<{:2}>> ", line_number);
            io::stdout().flush().unwrap();
            let line = read_line().expect("succeful line");
            println!();
            line_number += 1;
            self.interpret(line);
        }
    }

    pub fn run_file(&self, file: &str){
    }

}

fn read_line() -> Result<String, ()> {
    let mut buffer = String::new();
    match io::stdin().read_line(&mut buffer){
        Ok(_) => {},
        Err(e) => {println!("{e}"); std::process::exit(0)},
    };
    Ok(buffer)
}
