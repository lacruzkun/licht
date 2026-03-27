use crate::chunk::Chunk;
use crate::chunk::OpCode;
use crate::vm::VM;
use std::env;

mod chunk;
mod vm;

fn main() {
    let args = env::args();
    let mut vm = VM::new();
    let mut chunk = Chunk::new();

    let constant = chunk.add_constant(12);
    chunk.write(OpCode::OpConstant(constant), 123);
    chunk.write(OpCode::OpReturn, 123);
    chunk.disassemble("test chunk");
    vm.interpret(&chunk);
}
