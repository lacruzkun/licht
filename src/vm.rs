use crate::chunk::Chunk;
use crate::chunk::Int;
use crate::chunk::OpCode;

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

    pub fn interpret(&mut self, chunks: &Chunk) -> InterpretResult {
        self.chunks = chunks.clone();
        self.ip = 0;
        self.run()
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
}
