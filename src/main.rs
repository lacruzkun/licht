use std::process;
use std::thread;

enum Mode {
    File(String),
    Repl,
    Help,
    Version,
}

const USAGE: &str = "\
Usage: licht [options] [file.licht]

With no file, start the interactive REPL.

Options:
  -i, --repl     Start the interactive REPL
  -h, --help     Show this help
  -V, --version  Show the version";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mode = match parse_args(&args) {
        Ok(mode) => mode,
        Err(()) => {
            eprintln!("{}", USAGE);
            process::exit(2);
        }
    };

    match mode {
        Mode::Help => {
            println!("{}", USAGE);
        }
        Mode::Version => {
            println!("licht {}", env!("CARGO_PKG_VERSION"));
        }
        Mode::File(path) => process::exit(run_in_thread(move || run_file(&path))),
        Mode::Repl => process::exit(run_in_thread(licht::repl::run)),
    }
}

fn parse_args(args: &[String]) -> Result<Mode, ()> {
    match args {
        [] => Ok(Mode::Repl),
        [arg] if arg == "-h" || arg == "--help" => Ok(Mode::Help),
        [arg] if arg == "-V" || arg == "--version" => Ok(Mode::Version),
        [arg] if arg == "-i" || arg == "--repl" => Ok(Mode::Repl),
        [file] if !file.starts_with('-') => Ok(Mode::File(file.clone())),
        _ => Err(()),
    }
}

/// Runs on a thread with a large stack so deeply recursive Licht programs hit
/// the interpreter's own depth limit before the Rust stack runs out.
fn run_in_thread<F>(work: F) -> i32
where
    F: FnOnce() -> i32 + Send + 'static,
{
    let handle = thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(work)
        .expect("failed to start the interpreter thread");
    handle.join().unwrap_or(1)
}

fn run_file(path: &str) -> i32 {
    let source = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) => {
            eprintln!("Error: cannot read '{}': {}", path, err);
            return 2;
        }
    };
    match licht::run_source_to_stdout(&source) {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("{}", err.render(&source));
            1
        }
    }
}
