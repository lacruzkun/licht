use licht::error::ErrorKind;
use licht::run_source;

#[test]
fn indexing_reads_a_list_element() {
    assert_eq!(run_source("print([10, 20, 30][1]);").unwrap(), "20\n");
}

#[test]
fn indexing_works_through_variables_and_expressions() {
    assert_eq!(
        run_source("let xs = range(5, 9); let i = 2; print(xs[i], xs[0], xs[3]);").unwrap(),
        "7,5,8\n"
    );
}

#[test]
fn indexing_can_be_chained() {
    assert_eq!(
        run_source("let grid = [[1, 2], [3, 4]]; print(grid[1][0], grid[0][1]);").unwrap(),
        "3,2\n"
    );
}

#[test]
fn indexing_produces_the_element_type() {
    assert_eq!(run_source("print([\"a\", \"b\"][1]);").unwrap(), "b\n");
    assert_eq!(run_source("print([1, 2, 3][2] * 10);").unwrap(), "30\n");
}

#[test]
fn out_of_bounds_is_a_runtime_error() {
    let err = run_source("print([1, 2][2]);").expect_err("expected a runtime error");
    assert_eq!(err.kind, ErrorKind::Runtime);
    assert!(
        err.message.contains("out of bounds"),
        "message was: {}",
        err.message
    );
}

#[test]
fn negative_index_is_a_runtime_error() {
    let err = run_source("print([1, 2][-1]);").expect_err("expected a runtime error");
    assert_eq!(err.kind, ErrorKind::Runtime);
    assert!(
        err.message.contains("out of bounds"),
        "message was: {}",
        err.message
    );
}

#[test]
fn indexing_a_non_list_is_a_type_error() {
    let err = run_source("let x = 1; print(x[0]);").expect_err("expected a type error");
    assert_eq!(err.kind, ErrorKind::Type);
    assert!(
        err.message.contains("cannot index"),
        "message was: {}",
        err.message
    );
}

#[test]
fn a_non_int_index_is_a_type_error() {
    let err = run_source("print([1, 2][\"a\"]);").expect_err("expected a type error");
    assert_eq!(err.kind, ErrorKind::Type);
    assert!(
        err.message.contains("index must be int"),
        "message was: {}",
        err.message
    );
}
