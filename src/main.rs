use crate::chunk::Chunk;
use crate::chunk::OpCode;
use crate::vm::VM;
use std::env;

mod chunk;
mod vm;

fn main() {
    let args = env::args();
    let mut vm = VM::new();

    if args.len() == 1 {
        repl();
    } else if args.len() == 2{
        vm.run_file(args[1]);
    } else {
        print_help();
        std::process::exit(65);
    }
}


fn print_help() {
    println!("Usage: licht [path]");
} 
