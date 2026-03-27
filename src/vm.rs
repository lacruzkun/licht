use crate::chunk::Chunk;
use crate::chunk::OpCode;
use crate::chunk::Int

pub struct VM {
    chunks: Chunk,
    ip: usize,
    stack: Vec<Int>,
    stack_top: usize,
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
            stack_top: 0,
        }
    }

    pub fn interpret(&mut self, chunks: &Chunk) -> InterpretResult {
        self.chunks = chunks.clone();
        self.ip = 0;
        self.run()
    }

    fn run(&mut self) -> InterpretResult {
        loop {
            self.chunks.disassemble_instruction(self.ip);
            let instruction = self.chunks.code[self.ip];
            self.ip += 1;
            match instruction {
                OpCode::OpConstant(value) => {
                    println!("{}", value);
                }
                OpCode::OpReturn => {
                    return InterpretResult::InterpretOk;
                }
            }
        }
    }
}
