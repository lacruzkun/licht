use licht::error::ErrorKind;
use licht::run_source;

#[test]
fn print_joins_multiple_arguments_with_commas() {
    assert_eq!(run_source("print(1, 2, 3);").unwrap(), "1,2,3\n");
}

#[test]
fn print_uses_the_normal_value_rendering_for_each_argument() {
    assert_eq!(
        run_source("print(\"a\", true, [\"b\", \"c\"], 2.0);").unwrap(),
        "a,true,[\"b\", \"c\"],2.0\n"
    );
}

#[test]
fn print_with_no_arguments_prints_an_empty_line() {
    assert_eq!(run_source("print();").unwrap(), "\n");
}

#[test]
fn every_print_argument_must_have_a_value() {
    let err = run_source("print(1, print(2));").expect_err("expected a type error");
    assert_eq!(err.kind, ErrorKind::Type);
    assert!(err.message.contains("print needs a value"));
}
