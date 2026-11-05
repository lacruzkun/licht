//! Licht: a minimal, beginner-oriented language with a tree-walking interpreter.
//!
//! Pipeline: source -> lexer -> tokens -> parser -> AST -> type checker -> evaluator.
//! The evaluator only runs when the type checker accepts the whole program.

pub mod ast;
pub mod checker;
pub mod error;
pub mod interpreter;
pub mod lexer;
pub mod parser;
pub mod repl;
pub mod types;

use error::LichtError;

/// Lexes, parses and type-checks a program without running it.
pub fn compile(source: &str) -> Result<ast::Program, LichtError> {
    let tokens = lexer::lex(source)?;
    let program = parser::parse(tokens)?;
    checker::check(&program)?;
    Ok(program)
}

/// Runs a program and returns everything it printed (used by the tests).
pub fn run_source(source: &str) -> Result<String, LichtError> {
    let program = compile(source)?;
    let mut interpreter = interpreter::Interpreter::new(true);
    interpreter.run(&program)?;
    Ok(interpreter.take_output())
}

/// Runs a program, printing straight to standard output (used by the command line tool).
pub fn run_source_to_stdout(source: &str) -> Result<(), LichtError> {
    let program = compile(source)?;
    let mut interpreter = interpreter::Interpreter::new(false);
    interpreter.run(&program)
}
