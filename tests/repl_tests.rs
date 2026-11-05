use std::io::Write;
use std::process::{Command, Stdio};

use licht::error::ErrorKind;
use licht::interpreter::Value;
use licht::repl::{input_complete, Repl};

fn eval(repl: &mut Repl, source: &str) -> Vec<Value> {
    repl.execute(source)
        .unwrap_or_else(|err| panic!("unexpected error for {:?}: {}", source, err))
}

#[test]
fn variables_persist_between_submissions() {
    let mut repl = Repl::new();
    eval(&mut repl, "let x = 40;");
    assert_eq!(eval(&mut repl, "x + 2"), vec![Value::Int(42)]);
}

#[test]
fn functions_persist_between_submissions() {
    let mut repl = Repl::new();
    eval(&mut repl, "fn square(n: int) -> int { n * n }");
    assert_eq!(eval(&mut repl, "square(7)"), vec![Value::Int(49)]);
}

#[test]
fn a_trailing_semicolon_is_optional() {
    let mut repl = Repl::new();
    assert_eq!(eval(&mut repl, "1 + 2"), vec![Value::Int(3)]);
    eval(&mut repl, "let x = 5");
    assert_eq!(eval(&mut repl, "x"), vec![Value::Int(5)]);
    eval(&mut repl, "let y = if true { 1 } else { 2 }");
    assert_eq!(eval(&mut repl, "y"), vec![Value::Int(1)]);
}

#[test]
fn every_top_level_expression_is_returned_in_order() {
    let mut repl = Repl::new();
    let values = eval(&mut repl, "let x = 1; x + 1; x + 2;");
    assert_eq!(values, vec![Value::Int(2), Value::Int(3)]);
}

#[test]
fn functions_can_be_redefined_with_the_same_signature() {
    let mut repl = Repl::new();
    eval(&mut repl, "fn double(n: int) -> int { n * 2 }");
    assert_eq!(eval(&mut repl, "double(4)"), vec![Value::Int(8)]);
    eval(&mut repl, "fn double(n: int) -> int { n + 100 }");
    assert_eq!(eval(&mut repl, "double(4)"), vec![Value::Int(104)]);
}

#[test]
fn changing_a_function_signature_is_rejected() {
    let mut repl = Repl::new();
    eval(&mut repl, "fn f(x: int) -> int { x }");
    let err = repl
        .execute("fn f(x: string) -> string { x }")
        .expect_err("a signature change should be rejected");
    assert_eq!(err.kind, ErrorKind::Type);
    assert!(err.message.contains("different signature"));
    assert_eq!(eval(&mut repl, "f(3)"), vec![Value::Int(3)]);
}

#[test]
fn a_failed_type_check_rolls_back_the_whole_submission() {
    let mut repl = Repl::new();
    eval(&mut repl, "let kept = 1;");

    let err = repl
        .execute("let temporary = \"text\"; let bad = temporary + 1;")
        .expect_err("expected a type error");
    assert_eq!(err.kind, ErrorKind::Type);

    // `kept` is from an earlier successful submission and must still exist.
    assert_eq!(eval(&mut repl, "kept"), vec![Value::Int(1)]);

    // `temporary` came from the failed submission and must not leak into the
    // checker or the interpreter.
    let err = repl
        .execute("temporary")
        .expect_err("the failed submission must not define variables");
    assert_eq!(err.kind, ErrorKind::Type);
}

#[test]
fn a_runtime_error_rolls_back_the_whole_submission() {
    let mut repl = Repl::new();
    let err = repl
        .execute("let z = 1; let boom = 10 / 0;")
        .expect_err("expected a runtime error");
    assert_eq!(err.kind, ErrorKind::Runtime);
    let err = repl
        .execute("z")
        .expect_err("the failed submission must not define variables");
    assert_eq!(err.kind, ErrorKind::Type);
}

#[test]
fn definitions_from_a_failed_submission_are_not_kept() {
    let mut repl = Repl::new();
    let err = repl
        .execute("fn broken(n: int) -> bool { n }")
        .expect_err("expected a type error");
    assert_eq!(err.kind, ErrorKind::Type);
    let err = repl
        .execute("broken(1)")
        .expect_err("the failed function must not exist");
    assert_eq!(err.kind, ErrorKind::Type);
}

#[test]
fn input_completeness_tracks_delimiters_and_strings() {
    assert!(!input_complete("fn f() {\n"));
    assert!(input_complete("fn f() {\n}\n"));
    assert!(!input_complete("let s = \"open\n"));
    assert!(input_complete("let s = \"closed\";\n"));
    assert!(input_complete("1 + 2\n"));
    assert!(input_complete("}\n"));
    assert!(input_complete("// just a comment\n"));
    assert!(!input_complete("let xs = [1, 2\n"));
}

#[test]
fn repl_binary_processes_piped_input_without_prompts() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_licht"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("could not start the licht binary");

    child
        .stdin
        .take()
        .expect("stdin should be piped")
        .write_all(b"let x = 2;\nx * 21\n:quit\n")
        .expect("could not write to the REPL");

    let output = child
        .wait_with_output()
        .expect("could not wait for the REPL");
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "42\n");
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");
}
