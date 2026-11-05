# Licht

A minimal, beginner-oriented programming language and its reference interpreter, written in Rust
using only the standard library (no external crates).

Pipeline: `source -> lexer -> tokens -> parser -> AST -> type checker -> evaluator`.
The evaluator only runs when the type checker accepts the whole program.

## Build and run

```bash
cargo build --release
cargo run                                # starts the interactive REPL
cargo run -- examples/factorial.licht
cargo run -- examples/sample.licht       # needs data.txt in the current directory
cargo test                               # Chapter Four tests plus the REPL tests
```

Run `sample.licht` from inside `examples/` so that `data.txt` is found:

```bash
cd examples && cargo run --manifest-path ../Cargo.toml -- sample.licht
```

## Interactive REPL

Run `licht` with no arguments (or with `-i`/`--repl`) to start the REPL:

```text
$ cargo run
Licht 0.1.0 interactive REPL
Type :help for help, :quit to exit.
licht> let x = 41;
licht> x + 1
42
licht> fn square(n: int) -> int { n * n }
licht> square(7)
49
```

The final semicolon of an expression or declaration is optional. Expression results are printed
automatically, and functions and variables are kept between submissions. Each submission is atomic:
if parsing, type checking or evaluation fails, nothing from that submission is kept.

| Command | Meaning |
| --- | --- |
| `:help`, `:h` | Show the command list |
| `:quit`, `:q`, `:exit` | Exit the REPL (Ctrl-D also works) |
| `:reset`, `:clear` | Forget all functions and variables |
| `:load <file>` | Run a Licht file in the current session |

Function bodies can span multiple lines; the REPL keeps prompting with `...>` until braces,
brackets and parentheses are balanced. A function can be redefined as long as its signature
(parameter and return types) stays the same; use `:reset` to change a signature.

## Layout

| File | Role (chapter reference) |
| --- | --- |
| `src/lexer.rs` | Hand-written scanner (3.4.1) |
| `src/parser.rs` | Recursive-descent parser, one function per precedence level (3.4.2) |
| `src/ast.rs` | AST as Rust enums (3.4.3) |
| `src/checker.rs` | Separate type-checking pass (3.4.4) |
| `src/interpreter.rs` | Tree-walking evaluator and built-ins (3.4.5) |
| `src/repl.rs` | Interactive REPL with multi-line input and persistent state |
| `src/error.rs` | Lexical, syntax, type and runtime errors with line and column (4.2.6) |
| `src/types.rs` | Type representation shared by the checker and parser |
| `tests/chapter4_tests.rs` | Test cases L1-L10, P1-P11, T1-T13, E1-E10, I1-I4 (4.3) |
| `tests/repl_tests.rs` | REPL state, rollback, multi-line input and CLI tests |
| `tests/print_tests.rs` | Variadic `print` behavior |
| `tests/index_tests.rs` | List indexing and bounds checks |
| `examples/` | The Section 3.5 sample program and a few small programs |

## Language quick reference

```licht
fn add(x: int, y: int) -> int {      // parameter and return types are explicit
    x + y                            // the last expression (no ;) is the return value
}

let total = add(3, 4);               // local variable types are inferred
let ok = total > 5 && !false;

if ok { print("big"); } else { print("small"); }

for i in range(0, 3) { print(i); }   // prints 0, 1, 2 (the end is excluded)
print("x", 1, true);                 // x,1,true (arguments are separated by commas)

let xs = range(10, 13);
print(xs[0], xs[2]);                 // 10,12 (indexes start at 0)

fn is_even(n: int) -> bool { n % 2 == 0 }
print(filter(is_even, [1, 2, 3, 4])); // [2, 4]
fn double(n: int) -> int { n * 2 }
print(map(double, [1, 2, 3]));        // [2, 4, 6]: named functions are first-class values

write_file("out.txt", read_file("data.txt"));
```

Types you can write: `int`, `float`, `bool`, `string`. Built-ins: `print`, `range`, `map`, `filter`,
`read_file`, `write_file`. Comments start with `//`.

## Decisions the chapters left open

These are choices made in the implementation; change them if your Chapter 3 says otherwise.

1. `-> type` is optional. A function without it returns no value (so a function that only prints can exist).
2. `if` can also be used as an expression (`let x = if c { 1 } else { 2 };`) and `else if` is accepted.
   Both branches must then have the same type.
3. `range(a, b)` builds a list of ints from `a` up to but excluding `b`. `for` works over any list.
4. `int` and `float` never mix: `1 + 2.0` is a type error (no implicit conversion).
5. Strings can be compared (`==`, `!=`, `<`, ...) but there is no string concatenation.
6. A function can only see its own parameters and other functions: no closures, no access to top-level variables.
7. `print` accepts any number of arguments and shows them separated by commas. Strings are shown raw,
   strings inside lists quoted, floats with a decimal point (`2.0`), lists as `[2, 4, 6]`.
8. Integer overflow and division by zero are runtime errors. Recursion deeper than 3000 calls is a runtime error.
9. A function name can be called before its declaration (functions are collected first), which allows recursion.
10. Lists support `xs[i]` indexing (also chained, as in `grid[1][0]`). Indexes start at 0; a negative or
    too-large index is a runtime error.
