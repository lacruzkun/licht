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

    let mut constant_index = chunk.add_constant(12);
    chunk.write(OpCode::OpConstant(constant_index), 123);
    constant_index = chunk.add_constant(34);
    chunk.write(OpCode::OpConstant(constant_index), 123);
    chunk.write(OpCode::OpAdd, 123);
    constant_index = chunk.add_constant(2);
    chunk.write(OpCode::OpConstant(constant_index), 123);
    chunk.write( OpCode::OpDivide, 123);
    chunk.write( OpCode::OpNegate, 123);
    chunk.write( OpCode::OpReturn, 123);
    chunk.disassemble("test chunk");
    vm.interpret(&chunk);
}
