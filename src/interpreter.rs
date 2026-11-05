use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::*;
use crate::error::{ErrorKind, LichtError, Pos};
use crate::types::Type;

/// Maximum depth of nested Licht function calls before a runtime error is raised.
const MAX_DEPTH: usize = 3000;

/// A runtime value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    List(Vec<Value>),
    Unit,
    /// A named function used as a first-class value (e.g. passed to `map`).
    Func(String),
}

impl Value {
    /// Text shown by `print`. Strings are shown raw at top level and quoted inside lists.
    pub fn render(&self, nested: bool) -> String {
        match self {
            Value::Int(i) => i.to_string(),
            Value::Float(f) => format!("{:?}", f),
            Value::Bool(b) => b.to_string(),
            Value::Str(s) => {
                if nested {
                    format!("\"{}\"", s)
                } else {
                    s.clone()
                }
            }
            Value::List(items) => {
                let parts: Vec<String> = items.iter().map(|v| v.render(true)).collect();
                format!("[{}]", parts.join(", "))
            }
            Value::Unit => "()".to_string(),
            Value::Func(name) => format!("<function {}>", name),
        }
    }
}

/// Non-local control flow: `return` unwinds to the enclosing call, errors unwind everything.
enum Flow {
    Return(Value),
    Error(LichtError),
}

impl From<LichtError> for Flow {
    fn from(e: LichtError) -> Flow {
        Flow::Error(e)
    }
}

type EvalResult<T> = Result<T, Flow>;

fn rt_error(message: impl Into<String>, pos: Pos) -> Flow {
    Flow::Error(LichtError::new(ErrorKind::Runtime, message, pos))
}

#[derive(Clone)]
pub struct Interpreter {
    funcs: HashMap<String, Rc<FuncDecl>>,
    scopes: Vec<HashMap<String, Value>>,
    depth: usize,
    /// `Some` = collect printed output in memory (used by tests); `None` = print to stdout.
    captured: Option<String>,
}

impl Interpreter {
    pub fn new(capture: bool) -> Interpreter {
        Interpreter {
            funcs: HashMap::new(),
            scopes: Vec::new(),
            depth: 0,
            captured: if capture { Some(String::new()) } else { None },
        }
    }

    /// Creates an interpreter with a persistent top-level scope, for REPL use.
    pub fn new_repl() -> Interpreter {
        Interpreter {
            funcs: HashMap::new(),
            scopes: vec![HashMap::new()],
            depth: 0,
            captured: None,
        }
    }

    /// Returns (and clears) the output collected so far in capture mode.
    pub fn take_output(&mut self) -> String {
        match self.captured.as_mut() {
            Some(buf) => std::mem::take(buf),
            None => String::new(),
        }
    }

    /// Runs a program that has already passed the type checker.
    pub fn run(&mut self, program: &Program) -> Result<(), LichtError> {
        for item in &program.items {
            if let Item::Function(f) = item {
                self.funcs.insert(f.name.clone(), Rc::clone(f));
            }
        }
        self.scopes = vec![HashMap::new()];
        for item in &program.items {
            if let Item::Stmt(stmt) = item {
                match self.exec_stmt(stmt) {
                    Ok(_) => {}
                    Err(Flow::Error(e)) => return Err(e),
                    // the checker rejects a top-level `return`, so this cannot happen
                    Err(Flow::Return(_)) => {}
                }
            }
        }
        Ok(())
    }

    /// Runs one REPL submission, keeping previously defined functions and
    /// top-level variables. Returns the value of every top-level expression
    /// statement, in source order.
    ///
    /// If evaluation fails, the interpreter's state is left exactly as it was
    /// before the call (already-produced output is not rewound).
    pub fn run_repl_program(&mut self, program: &Program) -> Result<Vec<Value>, LichtError> {
        let saved = self.clone();
        match self.run_repl_program_inner(program) {
            Ok(values) => Ok(values),
            Err(err) => {
                *self = saved;
                Err(err)
            }
        }
    }

    fn run_repl_program_inner(&mut self, program: &Program) -> Result<Vec<Value>, LichtError> {
        for item in &program.items {
            if let Item::Function(f) = item {
                self.funcs.insert(f.name.clone(), Rc::clone(f));
            }
        }
        let mut values = Vec::new();
        for item in &program.items {
            if let Item::Stmt(stmt) = item {
                match self.exec_stmt(stmt) {
                    Ok(value) => {
                        if matches!(stmt, Stmt::Expr(_)) {
                            values.push(value);
                        }
                    }
                    Err(Flow::Error(err)) => return Err(err),
                    // the checker rejects a top-level `return`
                    Err(Flow::Return(_)) => {}
                }
            }
        }
        Ok(values)
    }

    fn emit(&mut self, text: &str) {
        match self.captured.as_mut() {
            Some(buf) => {
                buf.push_str(text);
                buf.push('\n');
            }
            None => println!("{}", text),
        }
    }

    // ---------- environment ----------

    fn declare(&mut self, name: &str, value: Value) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), value);
        }
    }

    fn lookup(&self, name: &str) -> Option<&Value> {
        for scope in self.scopes.iter().rev() {
            if let Some(v) = scope.get(name) {
                return Some(v);
            }
        }
        None
    }

    fn assign(&mut self, name: &str, value: Value) -> bool {
        for scope in self.scopes.iter_mut().rev() {
            if scope.contains_key(name) {
                scope.insert(name.to_string(), value);
                return true;
            }
        }
        false
    }

    // ---------- statements and blocks ----------

    fn exec_block(&mut self, block: &Block) -> EvalResult<Value> {
        self.scopes.push(HashMap::new());
        let result = self.exec_block_inner(block);
        self.scopes.pop();
        result
    }

    fn exec_block_inner(&mut self, block: &Block) -> EvalResult<Value> {
        for stmt in &block.stmts {
            self.exec_stmt(stmt)?;
        }
        match &block.tail {
            Some(e) => self.eval(e),
            None => Ok(Value::Unit),
        }
    }

    fn exec_stmt(&mut self, stmt: &Stmt) -> EvalResult<Value> {
        match stmt {
            Stmt::Let { name, init, .. } => {
                let v = self.eval(init)?;
                self.declare(name, v);
                Ok(Value::Unit)
            }
            Stmt::For {
                var,
                iter,
                body,
                pos,
            } => {
                let it = self.eval(iter)?;
                let items = match it {
                    Value::List(items) => items,
                    _ => return Err(rt_error("a for loop needs a list", *pos)),
                };
                for item in items {
                    self.scopes.push(HashMap::new());
                    self.declare(var, item);
                    let result = self.exec_block(body);
                    self.scopes.pop();
                    result?;
                }
                Ok(Value::Unit)
            }
            Stmt::Return { value, .. } => {
                let v = match value {
                    Some(e) => self.eval(e)?,
                    None => Value::Unit,
                };
                Err(Flow::Return(v))
            }
            Stmt::Expr(e) => self.eval(e),
        }
    }

    // ---------- expressions ----------

    fn eval(&mut self, e: &Expr) -> EvalResult<Value> {
        match &e.kind {
            ExprKind::Int(v) => Ok(Value::Int(*v)),
            ExprKind::Float(v) => Ok(Value::Float(*v)),
            ExprKind::Bool(v) => Ok(Value::Bool(*v)),
            ExprKind::Str(s) => Ok(Value::Str(s.clone())),
            ExprKind::List(items) => {
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    out.push(self.eval(item)?);
                }
                Ok(Value::List(out))
            }
            ExprKind::Var(name) => {
                if let Some(v) = self.lookup(name) {
                    return Ok(v.clone());
                }
                if self.funcs.contains_key(name) {
                    return Ok(Value::Func(name.clone()));
                }
                Err(rt_error(format!("undefined variable '{}'", name), e.pos))
            }
            ExprKind::Unary { op, operand } => {
                let v = self.eval(operand)?;
                match (*op, v) {
                    (UnOp::Neg, Value::Int(i)) => match i.checked_neg() {
                        Some(n) => Ok(Value::Int(n)),
                        None => Err(rt_error("integer overflow", e.pos)),
                    },
                    (UnOp::Neg, Value::Float(f)) => Ok(Value::Float(-f)),
                    (UnOp::Not, Value::Bool(b)) => Ok(Value::Bool(!b)),
                    _ => Err(rt_error("invalid operand for a unary operator", e.pos)),
                }
            }
            ExprKind::Binary { op, left, right } => self.eval_binary(*op, left, right, e.pos),
            ExprKind::Index { target, index } => {
                let value = self.eval(target)?;
                let position = self.eval(index)?;
                let items = match value {
                    Value::List(items) => items,
                    _ => return Err(rt_error("can only index a list", target.pos)),
                };
                let i = match position {
                    Value::Int(i) => i,
                    _ => return Err(rt_error("a list index must be an int", index.pos)),
                };
                let out_of_bounds = || {
                    rt_error(
                        format!(
                            "list index {} is out of bounds for a list of length {}",
                            i,
                            items.len()
                        ),
                        index.pos,
                    )
                };
                let i = usize::try_from(i).map_err(|_| out_of_bounds())?;
                items.get(i).cloned().ok_or_else(out_of_bounds)
            }
            ExprKind::Assign { name, value } => {
                let v = self.eval(value)?;
                if self.assign(name, v.clone()) {
                    Ok(v)
                } else {
                    Err(rt_error(
                        format!("cannot assign to undefined variable '{}'", name),
                        e.pos,
                    ))
                }
            }
            ExprKind::Call { name, args } => self.eval_call(name, args, e.pos),
            ExprKind::If {
                cond,
                then_block,
                else_block,
            } => {
                let c = self.eval(cond)?;
                match c {
                    Value::Bool(true) => self.exec_block(then_block),
                    Value::Bool(false) => match else_block {
                        Some(b) => self.exec_block(b),
                        None => Ok(Value::Unit),
                    },
                    _ => Err(rt_error("an if condition must be a bool", e.pos)),
                }
            }
        }
    }

    fn eval_binary(&mut self, op: BinOp, left: &Expr, right: &Expr, pos: Pos) -> EvalResult<Value> {
        // && and || short-circuit: the right side is only evaluated when needed
        if op == BinOp::And || op == BinOp::Or {
            let l = self.eval(left)?;
            let lb = match l {
                Value::Bool(b) => b,
                _ => return Err(rt_error("a logical operator needs bool operands", pos)),
            };
            if op == BinOp::And && !lb {
                return Ok(Value::Bool(false));
            }
            if op == BinOp::Or && lb {
                return Ok(Value::Bool(true));
            }
            let r = self.eval(right)?;
            return match r {
                Value::Bool(b) => Ok(Value::Bool(b)),
                _ => Err(rt_error("a logical operator needs bool operands", pos)),
            };
        }

        let l = self.eval(left)?;
        let r = self.eval(right)?;
        match (l, r) {
            (Value::Int(a), Value::Int(b)) => int_op(op, a, b, pos),
            (Value::Float(a), Value::Float(b)) => float_op(op, a, b, pos),
            (Value::Str(a), Value::Str(b)) => str_op(op, &a, &b, pos),
            (Value::Bool(a), Value::Bool(b)) => match op {
                BinOp::Eq => Ok(Value::Bool(a == b)),
                BinOp::Ne => Ok(Value::Bool(a != b)),
                _ => Err(rt_error("invalid operator for bool operands", pos)),
            },
            (Value::List(a), Value::List(b)) => match op {
                BinOp::Eq => Ok(Value::Bool(a == b)),
                BinOp::Ne => Ok(Value::Bool(a != b)),
                _ => Err(rt_error("invalid operator for list operands", pos)),
            },
            _ => Err(rt_error("the operands have incompatible types", pos)),
        }
    }

    // ---------- calls and built-ins ----------

    fn eval_call(&mut self, name: &str, args: &[Expr], pos: Pos) -> EvalResult<Value> {
        let mut values: Vec<Value> = Vec::with_capacity(args.len());
        for a in args {
            values.push(self.eval(a)?);
        }
        match name {
            "print" => {
                let text = values
                    .iter()
                    .map(|v| v.render(false))
                    .collect::<Vec<_>>()
                    .join(",");
                self.emit(&text);
                Ok(Value::Unit)
            }
            "range" => match (&values[0], &values[1]) {
                (Value::Int(a), Value::Int(b)) => {
                    // range(a, b) holds a, a+1, ..., b-1 (the end is excluded)
                    if (*b as i128) - (*a as i128) > 100_000_000 {
                        return Err(rt_error("this range is too large", pos));
                    }
                    let mut out = Vec::new();
                    let mut i = *a;
                    while i < *b {
                        out.push(Value::Int(i));
                        i += 1;
                    }
                    Ok(Value::List(out))
                }
                _ => Err(rt_error("range needs two ints", pos)),
            },
            "read_file" => match &values[0] {
                Value::Str(path) => match std::fs::read_to_string(path) {
                    Ok(text) => Ok(Value::Str(text)),
                    Err(err) => Err(rt_error(
                        format!("cannot read file '{}': {}", path, err),
                        pos,
                    )),
                },
                _ => Err(rt_error("read_file needs a string path", pos)),
            },
            "write_file" => match (&values[0], &values[1]) {
                (Value::Str(path), Value::Str(content)) => match std::fs::write(path, content) {
                    Ok(()) => Ok(Value::Unit),
                    Err(err) => Err(rt_error(
                        format!("cannot write file '{}': {}", path, err),
                        pos,
                    )),
                },
                _ => Err(rt_error("write_file needs two strings", pos)),
            },
            "map" => {
                let fname = match &values[0] {
                    Value::Func(n) => n.clone(),
                    _ => return Err(rt_error("map needs a function", pos)),
                };
                let items = match &values[1] {
                    Value::List(v) => v.clone(),
                    _ => return Err(rt_error("map needs a list", pos)),
                };
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    out.push(self.call_function(&fname, vec![item], pos)?);
                }
                Ok(Value::List(out))
            }
            "filter" => {
                let fname = match &values[0] {
                    Value::Func(n) => n.clone(),
                    _ => return Err(rt_error("filter needs a function", pos)),
                };
                let items = match &values[1] {
                    Value::List(v) => v.clone(),
                    _ => return Err(rt_error("filter needs a list", pos)),
                };
                let mut out = Vec::new();
                for item in items {
                    let keep = self.call_function(&fname, vec![item.clone()], pos)?;
                    match keep {
                        Value::Bool(true) => out.push(item),
                        Value::Bool(false) => {}
                        _ => return Err(rt_error("filter needs a function that returns bool", pos)),
                    }
                }
                Ok(Value::List(out))
            }
            _ => self.call_function(name, values, pos),
        }
    }

    fn call_function(&mut self, name: &str, args: Vec<Value>, pos: Pos) -> EvalResult<Value> {
        let func = match self.funcs.get(name) {
            Some(f) => Rc::clone(f),
            None => return Err(rt_error(format!("undefined function '{}'", name), pos)),
        };
        if self.depth >= MAX_DEPTH {
            return Err(rt_error("recursion is too deep (stack overflow)", pos));
        }
        // a call gets a fresh environment holding only its parameters (no closures)
        let mut frame: HashMap<String, Value> = HashMap::new();
        for (param, value) in func.params.iter().zip(args.into_iter()) {
            frame.insert(param.name.clone(), value);
        }
        let saved = std::mem::replace(&mut self.scopes, vec![frame]);
        self.depth += 1;
        let result = self.exec_block(&func.body);
        self.depth -= 1;
        self.scopes = saved;
        let value = match result {
            Ok(v) => v,
            Err(Flow::Return(v)) => v,
            Err(other) => return Err(other),
        };
        if func.ret == Type::Unit {
            Ok(Value::Unit)
        } else {
            Ok(value)
        }
    }
}

// ---------- operator helpers ----------

fn checked(result: Option<i64>, pos: Pos) -> EvalResult<Value> {
    match result {
        Some(v) => Ok(Value::Int(v)),
        None => Err(rt_error("integer overflow", pos)),
    }
}

fn int_op(op: BinOp, a: i64, b: i64, pos: Pos) -> EvalResult<Value> {
    match op {
        BinOp::Add => checked(a.checked_add(b), pos),
        BinOp::Sub => checked(a.checked_sub(b), pos),
        BinOp::Mul => checked(a.checked_mul(b), pos),
        BinOp::Div => {
            if b == 0 {
                Err(rt_error("division by zero", pos))
            } else {
                checked(a.checked_div(b), pos)
            }
        }
        BinOp::Rem => {
            if b == 0 {
                Err(rt_error("division by zero", pos))
            } else {
                checked(a.checked_rem(b), pos)
            }
        }
        BinOp::Lt => Ok(Value::Bool(a < b)),
        BinOp::Gt => Ok(Value::Bool(a > b)),
        BinOp::Le => Ok(Value::Bool(a <= b)),
        BinOp::Ge => Ok(Value::Bool(a >= b)),
        BinOp::Eq => Ok(Value::Bool(a == b)),
        BinOp::Ne => Ok(Value::Bool(a != b)),
        BinOp::And | BinOp::Or => Err(rt_error("invalid operands for a logical operator", pos)),
    }
}

fn float_op(op: BinOp, a: f64, b: f64, pos: Pos) -> EvalResult<Value> {
    match op {
        BinOp::Add => Ok(Value::Float(a + b)),
        BinOp::Sub => Ok(Value::Float(a - b)),
        BinOp::Mul => Ok(Value::Float(a * b)),
        BinOp::Div => {
            if b == 0.0 {
                Err(rt_error("division by zero", pos))
            } else {
                Ok(Value::Float(a / b))
            }
        }
        BinOp::Rem => {
            if b == 0.0 {
                Err(rt_error("division by zero", pos))
            } else {
                Ok(Value::Float(a % b))
            }
        }
        BinOp::Lt => Ok(Value::Bool(a < b)),
        BinOp::Gt => Ok(Value::Bool(a > b)),
        BinOp::Le => Ok(Value::Bool(a <= b)),
        BinOp::Ge => Ok(Value::Bool(a >= b)),
        BinOp::Eq => Ok(Value::Bool(a == b)),
        BinOp::Ne => Ok(Value::Bool(a != b)),
        BinOp::And | BinOp::Or => Err(rt_error("invalid operands for a logical operator", pos)),
    }
}

fn str_op(op: BinOp, a: &str, b: &str, pos: Pos) -> EvalResult<Value> {
    match op {
        BinOp::Lt => Ok(Value::Bool(a < b)),
        BinOp::Gt => Ok(Value::Bool(a > b)),
        BinOp::Le => Ok(Value::Bool(a <= b)),
        BinOp::Ge => Ok(Value::Bool(a >= b)),
        BinOp::Eq => Ok(Value::Bool(a == b)),
        BinOp::Ne => Ok(Value::Bool(a != b)),
        _ => Err(rt_error("invalid operator for string operands", pos)),
    }
}
