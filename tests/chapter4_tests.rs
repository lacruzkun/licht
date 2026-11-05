//! The 48 test cases of Chapter Four, Section 4.3.
//! Test names carry the case IDs: l1_..l10_ (lexer), p1_..p11_ (parser),
//! t1_..t13_ (type checker), e1_..e10_ (evaluator), i1_..i4_ (integration).

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use licht::ast::{BinOp, Expr, ExprKind, Item, Program, Stmt, UnOp};
use licht::error::{ErrorKind, LichtError};
use licht::lexer::{lex, Tok};
use licht::parser::parse;
use licht::types::Type;
use licht::{compile, run_source};

// ---------------------------------------------------------------- helpers

fn toks(src: &str) -> Vec<Tok> {
    lex(src)
        .expect("lexing should succeed")
        .into_iter()
        .map(|t| t.tok)
        .collect()
}

fn id(name: &str) -> Tok {
    Tok::Ident(name.to_string())
}

fn parse_src(src: &str) -> Program {
    parse(lex(src).expect("lexing should succeed")).expect("parsing should succeed")
}

fn parse_err(src: &str) -> LichtError {
    parse(lex(src).expect("lexing should succeed")).expect_err("parsing should fail")
}

/// Parses `src` as a single expression statement and returns the expression.
fn expr_of(src: &str) -> Expr {
    let program = parse_src(&format!("{};", src));
    match program.items.into_iter().next() {
        Some(Item::Stmt(Stmt::Expr(e))) => e,
        other => panic!("expected an expression statement, got {:?}", other),
    }
}

fn first_stmt(src: &str) -> Stmt {
    match parse_src(src).items.into_iter().next() {
        Some(Item::Stmt(s)) => s,
        other => panic!("expected a statement, got {:?}", other),
    }
}

/// Functions and variables assumed by the type-checker tests (Section 4.3.4).
const PRELUDE: &str = "fn add(x: int, y: int) -> int { x + y }\n\
fn is_even(n: int) -> bool { n % 2 == 0 }\n\
fn double(n: int) -> int { n * 2 }\n\
let numbers = [1, 2, 3, 4, 5, 6];\n\
let is_ready = true;\n\
let pi = 3.14159;\n";

/// The prelude plus `abs`, used by the evaluator tests.
const EVAL_PRELUDE: &str = "fn add(x: int, y: int) -> int { x + y }\n\
fn is_even(n: int) -> bool { n % 2 == 0 }\n\
fn double(n: int) -> int { n * 2 }\n\
fn abs(n: int) -> int { if n < 0 { return -n; } n }\n";

fn check_with_prelude(body: &str) -> Result<(), LichtError> {
    compile(&format!("{}{}", PRELUDE, body)).map(|_| ())
}

fn type_error(body: &str) -> LichtError {
    let err = check_with_prelude(body).expect_err("expected a type error");
    assert_eq!(err.kind, ErrorKind::Type, "wrong error kind: {}", err);
    err
}

fn run_prog(body: &str) -> String {
    run_source(&format!("{}{}", EVAL_PRELUDE, body)).expect("program should run")
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("licht_{}_{}", name, std::process::id()));
    fs::create_dir_all(&dir).expect("could not create a temporary directory");
    dir
}

fn slash(path: &PathBuf) -> String {
    path.to_string_lossy().replace('\\', "/")
}

// ---------------------------------------------------------------- lexer (L1-L10)

#[test]
fn l1_let_declaration_tokens() {
    assert_eq!(
        toks("let x = 5;"),
        vec![Tok::Let, id("x"), Tok::Assign, Tok::Int(5), Tok::Semi, Tok::Eof]
    );
}

#[test]
fn l2_equals_and_double_equals() {
    assert_eq!(toks("a == b"), vec![id("a"), Tok::EqEq, id("b"), Tok::Eof]);
    assert_eq!(toks("a = b"), vec![id("a"), Tok::Assign, id("b"), Tok::Eof]);
}

#[test]
fn l3_return_arrow_is_one_token() {
    assert_eq!(
        toks("fn f() -> int"),
        vec![
            Tok::Fn,
            id("f"),
            Tok::LParen,
            Tok::RParen,
            Tok::Arrow,
            id("int"),
            Tok::Eof
        ]
    );
}

#[test]
fn l4_float_literal() {
    assert_eq!(toks("3.14"), vec![Tok::Float(3.14), Tok::Eof]);
}

#[test]
fn l5_not_equal_and_lone_bang() {
    assert_eq!(toks("x != y"), vec![id("x"), Tok::Ne, id("y"), Tok::Eof]);
    assert_eq!(toks("!z"), vec![Tok::Bang, id("z"), Tok::Eof]);
}

#[test]
fn l6_logical_and() {
    assert_eq!(toks("a && b"), vec![id("a"), Tok::AndAnd, id("b"), Tok::Eof]);
}

#[test]
fn l7_string_literal() {
    assert_eq!(
        toks("\"Licht\""),
        vec![Tok::Str("Licht".to_string()), Tok::Eof]
    );
}

#[test]
fn l8_comment_is_discarded() {
    assert_eq!(
        toks("let x = 1; // note"),
        vec![Tok::Let, id("x"), Tok::Assign, Tok::Int(1), Tok::Semi, Tok::Eof]
    );
}

#[test]
fn l9_invalid_character_is_a_lexical_error() {
    let err = lex("let x = @;").expect_err("expected a lexical error");
    assert_eq!(err.kind, ErrorKind::Lexical);
}

#[test]
fn l10_unterminated_string_is_a_lexical_error() {
    let err = lex("\"abc").expect_err("expected a lexical error");
    assert_eq!(err.kind, ErrorKind::Lexical);
}

// ---------------------------------------------------------------- parser (P1-P11)

#[test]
fn p1_multiplication_binds_tighter_than_addition() {
    let e = expr_of("x + 2 * 3");
    match &e.kind {
        ExprKind::Binary { op, right, .. } => {
            assert_eq!(*op, BinOp::Add);
            match &right.kind {
                ExprKind::Binary { op, .. } => assert_eq!(*op, BinOp::Mul),
                other => panic!("right child should be a multiplication, got {:?}", other),
            }
        }
        other => panic!("expected an addition, got {:?}", other),
    }
}

#[test]
fn p2_and_binds_tighter_than_or() {
    let e = expr_of("a || b && c");
    match &e.kind {
        ExprKind::Binary { op, right, .. } => {
            assert_eq!(*op, BinOp::Or);
            match &right.kind {
                ExprKind::Binary { op, .. } => assert_eq!(*op, BinOp::And),
                other => panic!("right child should be an &&, got {:?}", other),
            }
        }
        other => panic!("expected an ||, got {:?}", other),
    }
}

#[test]
fn p3_not_binds_tighter_than_and() {
    let e = expr_of("!a && b");
    match &e.kind {
        ExprKind::Binary { op, left, .. } => {
            assert_eq!(*op, BinOp::And);
            match &left.kind {
                ExprKind::Unary { op, .. } => assert_eq!(*op, UnOp::Not),
                other => panic!("left child should be a !, got {:?}", other),
            }
        }
        other => panic!("expected an &&, got {:?}", other),
    }
}

#[test]
fn p4_let_with_call() {
    match first_stmt("let total = add(3, 4);") {
        Stmt::Let { name, init, .. } => {
            assert_eq!(name, "total");
            match &init.kind {
                ExprKind::Call { name, args } => {
                    assert_eq!(name, "add");
                    assert_eq!(args.len(), 2);
                }
                other => panic!("expected a call, got {:?}", other),
            }
        }
        other => panic!("expected a let statement, got {:?}", other),
    }
}

#[test]
fn p5_function_declaration() {
    let program = parse_src("fn add(x: int, y: int) -> int { x + y }");
    match program.items.into_iter().next() {
        Some(Item::Function(f)) => {
            assert_eq!(f.name, "add");
            assert_eq!(f.params.len(), 2);
            assert_eq!(f.ret, Type::Int);
            assert!(f.body.stmts.is_empty());
            assert!(f.body.tail.is_some());
        }
        other => panic!("expected a function, got {:?}", other),
    }
}

#[test]
fn p6_if_else_statement() {
    match first_stmt("if total > 5 { print(\"big\"); } else { print(\"small\"); }") {
        Stmt::Expr(e) => match &e.kind {
            ExprKind::If { else_block, .. } => assert!(else_block.is_some()),
            other => panic!("expected an if, got {:?}", other),
        },
        other => panic!("expected an expression statement, got {:?}", other),
    }
}

#[test]
fn p7_for_loop() {
    match first_stmt("for i in range(0, 10) { print(i); }") {
        Stmt::For { var, iter, body, .. } => {
            assert_eq!(var, "i");
            match &iter.kind {
                ExprKind::Call { name, .. } => assert_eq!(name, "range"),
                other => panic!("expected a call to range, got {:?}", other),
            }
            assert_eq!(body.stmts.len(), 1);
        }
        other => panic!("expected a for statement, got {:?}", other),
    }
}

#[test]
fn p8_list_literal() {
    match first_stmt("let numbers = [1, 2, 3];") {
        Stmt::Let { init, .. } => match &init.kind {
            ExprKind::List(items) => assert_eq!(items.len(), 3),
            other => panic!("expected a list, got {:?}", other),
        },
        other => panic!("expected a let statement, got {:?}", other),
    }
}

#[test]
fn p9_missing_identifier_is_a_syntax_error() {
    assert_eq!(parse_err("let = 5;").kind, ErrorKind::Syntax);
}

#[test]
fn p10_missing_block_is_a_syntax_error() {
    assert_eq!(parse_err("if x > 1 print(x);").kind, ErrorKind::Syntax);
}

#[test]
fn p11_missing_semicolon_is_a_syntax_error() {
    assert_eq!(parse_err("let x = 1").kind, ErrorKind::Syntax);
}

// ---------------------------------------------------------------- type checker (T1-T13)

#[test]
fn t1_local_type_is_inferred_from_a_call() {
    check_with_prelude("let total = add(3, 4);\nlet again = total + 1;").expect("should be accepted");
}

#[test]
fn t2_bool_from_logical_and_comparison() {
    check_with_prelude("let can_run = is_ready && pi > 3.0;\nlet b = !can_run;")
        .expect("should be accepted");
}

#[test]
fn t3_wrong_argument_type() {
    let err = type_error("add(3, \"four\");");
    assert!(err.message.contains("argument 2"), "message was: {}", err.message);
}

#[test]
fn t4_wrong_argument_count() {
    type_error("add(3);");
}

#[test]
fn t5_condition_must_be_bool() {
    type_error("if 5 { print(\"x\"); }");
}

#[test]
fn t6_arithmetic_needs_numbers() {
    type_error("let x = true + 1;");
}

#[test]
fn t7_logical_and_needs_bools() {
    type_error("let b = 1 && true;");
}

#[test]
fn t8_body_type_must_match_declared_return_type() {
    type_error("fn f(n: int) -> bool { n + 1 }");
}

#[test]
fn t9_filter_accepts_a_predicate() {
    check_with_prelude("let evens = filter(is_even, numbers);").expect("should be accepted");
}

#[test]
fn t10_map_accepts_a_function() {
    check_with_prelude("let doubled = map(double, numbers);").expect("should be accepted");
}

#[test]
fn t11_map_function_must_match_element_type() {
    type_error("let r = map(is_even, [\"a\", \"b\"]);");
}

#[test]
fn t12_filter_function_must_return_bool() {
    type_error("let r = filter(double, numbers);");
}

#[test]
fn t13_type_error_stops_everything_before_evaluation() {
    // compile() is the lexer + parser + type checker. It fails, so the evaluator
    // is never created and the print on the first line can never run.
    let src = format!("{}print(\"hello\"); add(3, \"four\");", PRELUDE);
    let err = compile(&src).expect_err("expected a type error");
    assert_eq!(err.kind, ErrorKind::Type);
    let run_err = run_source(&src).expect_err("expected a type error");
    assert_eq!(run_err.kind, ErrorKind::Type);
}

// ---------------------------------------------------------------- evaluator (E1-E10)

#[test]
fn e1_function_call() {
    assert_eq!(run_prog("print(add(3, 4));"), "7\n");
}

#[test]
fn e2_early_return() {
    assert_eq!(run_prog("print(abs(-5));"), "5\n");
}

#[test]
fn e3_operator_precedence() {
    assert_eq!(run_prog("let x = 1; print(x + 2 * 3);"), "7\n");
}

#[test]
fn e4_logical_operators() {
    assert_eq!(
        run_prog("print(!true); print(true && false); print(true || false);"),
        "false\nfalse\ntrue\n"
    );
}

#[test]
fn e5_only_the_chosen_branch_runs() {
    assert_eq!(
        run_prog("let total = 7; if total > 5 { print(\"big\"); } else { print(\"small\"); }"),
        "big\n"
    );
}

#[test]
fn e6_for_loop_over_a_range() {
    assert_eq!(run_prog("for i in range(0, 3) { print(i); }"), "0\n1\n2\n");
}

#[test]
fn e7_filter() {
    assert_eq!(
        run_prog("print(filter(is_even, [1, 2, 3, 4, 5, 6]));"),
        "[2, 4, 6]\n"
    );
}

#[test]
fn e8_map() {
    assert_eq!(
        run_prog("print(map(double, [1, 2, 3, 4, 5, 6]));"),
        "[2, 4, 6, 8, 10, 12]\n"
    );
}

#[test]
fn e9_write_file_copies_read_file() {
    let dir = temp_dir("e9");
    let data = dir.join("data.txt");
    let out = dir.join("out.txt");
    fs::write(&data, "hello licht").expect("could not write data.txt");
    let src = format!(
        "write_file(\"{}\", read_file(\"{}\"));",
        slash(&out),
        slash(&data)
    );
    run_source(&src).expect("program should run");
    assert_eq!(fs::read_to_string(&out).expect("out.txt should exist"), "hello licht");
}

#[test]
fn e10_missing_file_is_a_runtime_error() {
    let err = run_source("let c = read_file(\"licht_definitely_missing_file.txt\");")
        .expect_err("expected a runtime error");
    assert_eq!(err.kind, ErrorKind::Runtime);
}

// ---------------------------------------------------------------- integration (I1-I4)

const SAMPLE: &str = include_str!("../examples/sample.licht");

fn run_binary(dir: &PathBuf, file: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_licht"))
        .arg(file)
        .current_dir(dir)
        .output()
        .expect("could not run the licht binary")
}

#[test]
fn i1_sample_program_runs_end_to_end() {
    let dir = temp_dir("i1");
    fs::write(dir.join("data.txt"), "some data\n").expect("could not write data.txt");
    fs::write(dir.join("sample.licht"), SAMPLE).expect("could not write sample.licht");
    let output = run_binary(&dir, "sample.licht");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert_eq!(
        stdout,
        "big\n0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n[2, 4, 6]\n[2, 4, 6, 8, 10, 12]\n"
    );
    assert_eq!(
        fs::read_to_string(dir.join("out.txt")).expect("out.txt should exist"),
        "some data\n"
    );
}

#[test]
fn i2_type_error_in_the_sample_prints_nothing() {
    let dir = temp_dir("i2");
    fs::write(dir.join("data.txt"), "some data\n").expect("could not write data.txt");
    let broken = SAMPLE.replace("add(3, 4)", "add(3, \"4\")");
    assert_ne!(broken, SAMPLE, "the replacement should change the sample");
    fs::write(dir.join("broken.licht"), broken).expect("could not write broken.licht");
    let output = run_binary(&dir, "broken.licht");
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty(), "nothing should be printed");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Type error"), "stderr was: {}", stderr);
    assert!(!dir.join("out.txt").exists(), "out.txt must not be created");
}

#[test]
fn i3_invalid_character_stops_before_parsing() {
    let err = compile("let x = @;").expect_err("expected an error");
    assert_eq!(err.kind, ErrorKind::Lexical);
}

#[test]
fn i4_missing_semicolon_stops_before_type_checking() {
    // `add` is never defined, which would be a type error, but the syntax error
    // is reported first because the type checker never runs.
    let err = compile("let x = add(1, 2)").expect_err("expected an error");
    assert_eq!(err.kind, ErrorKind::Syntax);
}
