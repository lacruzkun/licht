//! Interactive read-eval-print loop for Licht.
//!
//! Each submission is parsed, type-checked and evaluated against state kept
//! from earlier submissions. A submission is atomic: if any stage fails, no
//! functions or variables from it are kept.

use std::io::{self, BufRead, IsTerminal, Write};

use crate::ast::Program;
use crate::checker::Checker;
use crate::error::LichtError;
use crate::interpreter::{Interpreter, Value};
use crate::{lexer, parser};

/// A persistent REPL session.
#[derive(Clone)]
pub struct Repl {
    checker: Checker,
    interpreter: Interpreter,
}

impl Default for Repl {
    fn default() -> Repl {
        Repl::new()
    }
}

impl Repl {
    pub fn new() -> Repl {
        Repl {
            checker: Checker::new(),
            interpreter: Interpreter::new_repl(),
        }
    }

    /// Forgets every function and variable defined so far.
    pub fn reset(&mut self) {
        *self = Repl::new();
    }

    /// Parses, type-checks and runs one submission.
    ///
    /// Returns the value of every top-level expression statement, in order.
    /// A trailing semicolon is optional.
    pub fn execute(&mut self, source: &str) -> Result<Vec<Value>, LichtError> {
        let mut checker = self.checker.clone();
        let mut interpreter = self.interpreter.clone();
        let program = parse_for_repl(source)?;
        checker.check_repl_program(&program)?;
        let values = interpreter.run_repl_program(&program)?;
        self.checker = checker;
        self.interpreter = interpreter;
        Ok(values)
    }
}

fn parse_source(source: &str) -> Result<Program, LichtError> {
    let tokens = lexer::lex(source)?;
    parser::parse(tokens)
}

fn parse_for_repl(source: &str) -> Result<Program, LichtError> {
    match parse_source(source) {
        Ok(program) => Ok(program),
        Err(first) => {
            // Convenience: an expression or declaration typed without its
            // final semicolon is retried with one appended on a new line.
            let mut extended = String::with_capacity(source.len() + 2);
            extended.push_str(source);
            if !source.ends_with('\n') {
                extended.push('\n');
            }
            extended.push(';');
            match parse_source(&extended) {
                Ok(program) => Ok(program),
                Err(_) => Err(first),
            }
        }
    }
}

/// True when all delimiters in `source` are balanced and no string is open.
/// An unmatched closing delimiter counts as complete so the error is reported
/// immediately rather than prompting for more input.
pub fn input_complete(source: &str) -> bool {
    let mut stack: Vec<char> = Vec::new();
    let mut chars = source.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;

    while let Some(c) = chars.next() {
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        match c {
            '"' => in_string = true,
            '/' if chars.peek() == Some(&'/') => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
            }
            '(' | '[' | '{' => stack.push(c),
            ')' | ']' | '}' => {
                let expected = match c {
                    ')' => '(',
                    ']' => '[',
                    _ => '{',
                };
                match stack.pop() {
                    Some(open) if open == expected => {}
                    _ => return true,
                }
            }
            _ => {}
        }
    }

    !in_string && stack.is_empty()
}

/// Runs the interactive loop, returning the process exit code.
pub fn run() -> i32 {
    let interactive = io::stdin().is_terminal();
    let mut repl = Repl::new();
    let stdin = io::stdin();
    let mut input = stdin.lock().lines();
    let mut buffer = String::new();

    if interactive {
        println!("Licht {} interactive REPL", env!("CARGO_PKG_VERSION"));
        println!("Type :help for help, :quit to exit.");
    }

    loop {
        if interactive {
            let prompt = if buffer.is_empty() {
                "licht> "
            } else {
                "   ...> "
            };
            print!("{}", prompt);
            let _ = io::stdout().flush();
        }

        let line = match input.next() {
            Some(Ok(line)) => line,
            Some(Err(err)) => {
                eprintln!("Error: cannot read input: {}", err);
                return 1;
            }
            None => {
                if !buffer.trim().is_empty() {
                    submit(&mut repl, &buffer);
                }
                if interactive {
                    println!();
                }
                return 0;
            }
        };

        if buffer.is_empty() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.starts_with(':') {
                if let Command::Quit = run_command(&mut repl, trimmed) {
                    return 0;
                }
                continue;
            }
        }

        buffer.push_str(&line);
        buffer.push('\n');

        if !input_complete(&buffer) {
            continue;
        }

        let source = std::mem::take(&mut buffer);
        submit(&mut repl, &source);
    }
}

enum Command {
    Continue,
    Quit,
}

fn submit(repl: &mut Repl, source: &str) {
    if source.trim().is_empty() {
        return;
    }
    match repl.execute(source) {
        Ok(values) => {
            for value in values {
                if !matches!(value, Value::Unit) {
                    println!("{}", value.render(true));
                }
            }
        }
        Err(err) => eprintln!("{}", err.render(source)),
    }
}

fn run_command(repl: &mut Repl, line: &str) -> Command {
    let mut parts = line.splitn(2, char::is_whitespace);
    let name = parts.next().unwrap_or("");
    let rest = parts.next().unwrap_or("").trim();

    match name {
        ":help" | ":h" | ":?" => {
            print_help();
            Command::Continue
        }
        ":quit" | ":q" | ":exit" => Command::Quit,
        ":reset" | ":clear" => {
            repl.reset();
            println!("Session state cleared.");
            Command::Continue
        }
        ":load" => {
            if rest.is_empty() {
                eprintln!("Usage: :load <file.licht>");
            } else {
                let path = strip_quotes(rest);
                match std::fs::read_to_string(path) {
                    Ok(source) => {
                        if let Err(err) = repl.execute(&source) {
                            eprintln!("{}", err.render(&source));
                        }
                    }
                    Err(err) => eprintln!("Error: cannot read '{}': {}", path, err),
                }
            }
            Command::Continue
        }
        _ => {
            eprintln!(
                "Unknown command '{}'. Type :help for the list of commands.",
                name
            );
            Command::Continue
        }
    }
}

fn strip_quotes(text: &str) -> &str {
    for quote in ['"', '\''] {
        if let Some(inner) = text
            .strip_prefix(quote)
            .and_then(|rest| rest.strip_suffix(quote))
        {
            return inner;
        }
    }
    text
}

fn print_help() {
    println!(
        "\
Commands:
  :help, :h, :?     Show this help
  :quit, :q, :exit  Exit the REPL
  :reset, :clear    Forget all functions and variables
  :load <file>      Run a Licht file in the current session

Enter statements and expressions at the prompt. The final semicolon of an
expression or declaration is optional. Expression results are printed
automatically; state is kept between submissions.

Examples:
  let x = 41;
  x + 1
  fn square(n: int) -> int {{ n * n }}
  square(7)
  for i in range(0, 3) {{ print(i); }}"
    );
}
