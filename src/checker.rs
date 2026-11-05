use std::collections::HashMap;

use crate::ast::*;
use crate::error::{ErrorKind, LichtError, Pos};
use crate::types::{compatible, Type};

/// Names of the built-in operations. User code cannot redefine them.
pub const BUILTINS: [&str; 6] = ["print", "range", "map", "filter", "read_file", "write_file"];

pub fn is_builtin(name: &str) -> bool {
    BUILTINS.contains(&name)
}

#[derive(Clone)]
struct FuncSig {
    params: Vec<Type>,
    ret: Type,
}

#[derive(Clone)]
pub struct Checker {
    funcs: HashMap<String, FuncSig>,
    scopes: Vec<HashMap<String, Type>>,
    /// The declared return type of the function being checked (None at top level).
    ret: Option<Type>,
}

type TResult<T> = Result<T, LichtError>;

fn type_err<T>(message: impl Into<String>, pos: Pos) -> TResult<T> {
    Err(LichtError::new(ErrorKind::Type, message, pos))
}

/// Runs the type-checking pass over a whole program.
/// The evaluator is only ever invoked when this returns Ok.
pub fn check(program: &Program) -> Result<(), LichtError> {
    let mut checker = Checker {
        funcs: HashMap::new(),
        scopes: Vec::new(),
        ret: None,
    };
    checker.check_program(program)
}

impl Checker {
    /// Creates a checker with an empty top-level scope, for use across REPL submissions.
    pub fn new() -> Checker {
        Checker {
            funcs: HashMap::new(),
            scopes: vec![HashMap::new()],
            ret: None,
        }
    }

    /// Type-checks one REPL submission against the persistent environment.
    ///
    /// Unlike a whole program, a submission may redefine an existing function
    /// as long as its signature stays the same. If checking fails, the
    /// checker's state is left exactly as it was before the call.
    pub fn check_repl_program(&mut self, program: &Program) -> TResult<()> {
        let saved = self.clone();
        match self.check_repl_program_inner(program) {
            Ok(()) => Ok(()),
            Err(err) => {
                *self = saved;
                Err(err)
            }
        }
    }

    fn check_repl_program_inner(&mut self, program: &Program) -> TResult<()> {
        // 1. collect signatures, allowing same-signature redefinitions
        for item in &program.items {
            if let Item::Function(f) = item {
                if is_builtin(&f.name) {
                    return type_err(
                        format!("cannot redefine the built-in function '{}'", f.name),
                        f.pos,
                    );
                }
                let sig = FuncSig {
                    params: f.params.iter().map(|p| p.ty.clone()).collect(),
                    ret: f.ret.clone(),
                };
                if let Some(existing) = self.funcs.get(&f.name) {
                    if existing.params != sig.params || existing.ret != sig.ret {
                        return type_err(
                            format!(
                                "function '{}' is already defined with a different signature; use :reset to start over",
                                f.name
                            ),
                            f.pos,
                        );
                    }
                }
                self.funcs.insert(f.name.clone(), sig);
            }
        }

        // 2. check function bodies without disturbing the top-level scope
        let top_scopes = std::mem::take(&mut self.scopes);
        let top_ret = self.ret.take();
        for item in &program.items {
            if let Item::Function(f) = item {
                if let Err(err) = self.check_function(f) {
                    self.scopes = top_scopes;
                    self.ret = top_ret;
                    return Err(err);
                }
            }
        }
        self.scopes = top_scopes;
        self.ret = top_ret;

        // 3. check the top-level statements in the persistent scope
        for item in &program.items {
            if let Item::Stmt(s) = item {
                self.check_stmt(s)?;
            }
        }
        Ok(())
    }

    fn check_program(&mut self, program: &Program) -> TResult<()> {
        // 1. collect every function signature so functions can call each other
        for item in &program.items {
            if let Item::Function(f) = item {
                if is_builtin(&f.name) {
                    return type_err(
                        format!("cannot redefine the built-in function '{}'", f.name),
                        f.pos,
                    );
                }
                if self.funcs.contains_key(&f.name) {
                    return type_err(format!("function '{}' is already defined", f.name), f.pos);
                }
                let sig = FuncSig {
                    params: f.params.iter().map(|p| p.ty.clone()).collect(),
                    ret: f.ret.clone(),
                };
                self.funcs.insert(f.name.clone(), sig);
            }
        }

        // 2. check every function body
        for item in &program.items {
            if let Item::Function(f) = item {
                self.check_function(f)?;
            }
        }

        // 3. check the top-level statements
        self.scopes = vec![HashMap::new()];
        self.ret = None;
        for item in &program.items {
            if let Item::Stmt(s) = item {
                self.check_stmt(s)?;
            }
        }
        Ok(())
    }

    fn check_function(&mut self, f: &FuncDecl) -> TResult<()> {
        let mut scope: HashMap<String, Type> = HashMap::new();
        for p in &f.params {
            if scope.contains_key(&p.name) {
                return type_err(format!("duplicate parameter name '{}'", p.name), p.pos);
            }
            scope.insert(p.name.clone(), p.ty.clone());
        }
        // functions do not capture outer variables: only their own parameters are visible
        self.scopes = vec![scope];
        self.ret = Some(f.ret.clone());
        let body_ty = self.check_block(&f.body)?;
        if f.ret != Type::Unit && !compatible(&f.ret, &body_ty) {
            return type_err(
                format!(
                    "function '{}' must return {}, but its body produces {}",
                    f.name, f.ret, body_ty
                ),
                f.pos,
            );
        }
        Ok(())
    }

    // ---------- scopes ----------

    fn declare(&mut self, name: &str, ty: Type) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), ty);
        }
    }

    fn lookup(&self, name: &str) -> Option<&Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(t) = scope.get(name) {
                return Some(t);
            }
        }
        None
    }

    // ---------- blocks and statements ----------

    fn check_block(&mut self, block: &Block) -> TResult<Type> {
        self.scopes.push(HashMap::new());
        let result = self.check_block_inner(block);
        self.scopes.pop();
        result
    }

    fn check_block_inner(&mut self, block: &Block) -> TResult<Type> {
        let mut diverges = false;
        for stmt in &block.stmts {
            let t = self.check_stmt(stmt)?;
            if t == Type::Never {
                diverges = true;
            }
        }
        let ty = match &block.tail {
            Some(e) => self.check_expr(e)?,
            None => Type::Unit,
        };
        if diverges {
            Ok(Type::Never)
        } else {
            Ok(ty)
        }
    }

    fn check_stmt(&mut self, stmt: &Stmt) -> TResult<Type> {
        match stmt {
            Stmt::Let { name, init, pos } => {
                let t = self.check_expr(init)?;
                if t == Type::Unit || t == Type::Never {
                    return type_err(
                        format!("cannot store a value of type {} in variable '{}'", t, name),
                        *pos,
                    );
                }
                self.declare(name, t);
                Ok(Type::Unit)
            }
            Stmt::For {
                var,
                iter,
                body,
                pos,
            } => {
                let it = self.check_expr(iter)?;
                let elem = match it {
                    Type::List(inner) => *inner,
                    other => {
                        return type_err(
                            format!("a for loop needs a list or a range, found {}", other),
                            *pos,
                        );
                    }
                };
                self.scopes.push(HashMap::new());
                self.declare(var, elem);
                let result = self.check_block(body);
                self.scopes.pop();
                result?;
                Ok(Type::Unit)
            }
            Stmt::Return { value, pos } => {
                let expected = match &self.ret {
                    Some(t) => t.clone(),
                    None => {
                        return type_err("'return' can only be used inside a function", *pos);
                    }
                };
                match value {
                    Some(e) => {
                        let t = self.check_expr(e)?;
                        if !compatible(&expected, &t) {
                            return type_err(
                                format!(
                                    "this function returns {}, but the returned value has type {}",
                                    expected, t
                                ),
                                e.pos,
                            );
                        }
                    }
                    None => {
                        if expected != Type::Unit {
                            return type_err(
                                format!(
                                    "this function returns {}, but 'return' has no value",
                                    expected
                                ),
                                *pos,
                            );
                        }
                    }
                }
                Ok(Type::Never)
            }
            Stmt::Expr(e) => match &e.kind {
                // an `if` used as a statement throws its value away, so its branches
                // do not have to agree on a type
                ExprKind::If {
                    cond,
                    then_block,
                    else_block,
                } => self.check_if(cond, then_block, else_block, e.pos, false),
                _ => self.check_expr(e),
            },
        }
    }

    fn check_if(
        &mut self,
        cond: &Expr,
        then_block: &Block,
        else_block: &Option<Block>,
        pos: Pos,
        strict: bool,
    ) -> TResult<Type> {
        let ct = self.check_expr(cond)?;
        if ct != Type::Bool {
            return type_err(format!("an if condition must be bool, found {}", ct), cond.pos);
        }
        let then_ty = self.check_block(then_block)?;
        match else_block {
            None => Ok(Type::Unit),
            Some(eb) => {
                let else_ty = self.check_block(eb)?;
                if then_ty == Type::Never {
                    return Ok(else_ty);
                }
                if else_ty == Type::Never {
                    return Ok(then_ty);
                }
                if compatible(&then_ty, &else_ty) {
                    return Ok(then_ty);
                }
                if strict {
                    type_err(
                        format!(
                            "the branches of this if have different types: {} and {}",
                            then_ty, else_ty
                        ),
                        pos,
                    )
                } else {
                    Ok(Type::Unit)
                }
            }
        }
    }

    // ---------- expressions ----------

    fn check_expr(&mut self, e: &Expr) -> TResult<Type> {
        match &e.kind {
            ExprKind::Int(_) => Ok(Type::Int),
            ExprKind::Float(_) => Ok(Type::Float),
            ExprKind::Bool(_) => Ok(Type::Bool),
            ExprKind::Str(_) => Ok(Type::Str),
            ExprKind::List(items) => {
                let mut elem = Type::Never;
                for (i, item) in items.iter().enumerate() {
                    let t = self.check_expr(item)?;
                    if t == Type::Unit || t == Type::Never {
                        return type_err("a list element must have a value", item.pos);
                    }
                    if i == 0 {
                        elem = t;
                    } else if !compatible(&elem, &t) {
                        return type_err(
                            format!(
                                "all list elements must have the same type: expected {}, found {}",
                                elem, t
                            ),
                            item.pos,
                        );
                    }
                }
                Ok(Type::List(Box::new(elem)))
            }
            ExprKind::Var(name) => {
                if let Some(t) = self.lookup(name) {
                    return Ok(t.clone());
                }
                if let Some(sig) = self.funcs.get(name) {
                    return Ok(Type::Func(sig.params.clone(), Box::new(sig.ret.clone())));
                }
                if is_builtin(name) {
                    return type_err(
                        format!("built-in function '{}' cannot be used as a value", name),
                        e.pos,
                    );
                }
                type_err(format!("undefined variable '{}'", name), e.pos)
            }
            ExprKind::Unary { op, operand } => {
                let t = self.check_expr(operand)?;
                match op {
                    UnOp::Neg => {
                        if t == Type::Int || t == Type::Float {
                            Ok(t)
                        } else {
                            type_err(
                                format!("unary '-' needs an int or a float, found {}", t),
                                e.pos,
                            )
                        }
                    }
                    UnOp::Not => {
                        if t == Type::Bool {
                            Ok(Type::Bool)
                        } else {
                            type_err(format!("'!' needs a bool, found {}", t), e.pos)
                        }
                    }
                }
            }
            ExprKind::Binary { op, left, right } => self.check_binary(*op, left, right, e.pos),
            ExprKind::Index { target, index } => {
                let target_ty = self.check_expr(target)?;
                let index_ty = self.check_expr(index)?;
                if index_ty != Type::Int {
                    return type_err(
                        format!("a list index must be int, found {}", index_ty),
                        index.pos,
                    );
                }
                match target_ty {
                    Type::List(elem) => Ok(*elem),
                    other => type_err(
                        format!("cannot index a value of type {}", other),
                        target.pos,
                    ),
                }
            }
            ExprKind::Assign { name, value } => {
                let vt = self.check_expr(value)?;
                let target = match self.lookup(name) {
                    Some(t) => t.clone(),
                    None => {
                        return type_err(
                            format!("cannot assign to undefined variable '{}'", name),
                            e.pos,
                        );
                    }
                };
                if vt == Type::Unit || !compatible(&target, &vt) {
                    return type_err(
                        format!(
                            "cannot assign a value of type {} to variable '{}' of type {}",
                            vt, name, target
                        ),
                        e.pos,
                    );
                }
                Ok(target)
            }
            ExprKind::Call { name, args } => self.check_call(name, args, e.pos),
            ExprKind::If {
                cond,
                then_block,
                else_block,
            } => self.check_if(cond, then_block, else_block, e.pos, true),
        }
    }

    fn check_binary(&mut self, op: BinOp, left: &Expr, right: &Expr, pos: Pos) -> TResult<Type> {
        let lt = self.check_expr(left)?;
        let rt = self.check_expr(right)?;
        match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem => {
                if lt == rt && (lt == Type::Int || lt == Type::Float) {
                    Ok(lt)
                } else {
                    type_err(
                        format!(
                            "operator '{}' needs two ints or two floats, found {} and {}",
                            op.symbol(),
                            lt,
                            rt
                        ),
                        pos,
                    )
                }
            }
            BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge => {
                if lt == rt && (lt == Type::Int || lt == Type::Float || lt == Type::Str) {
                    Ok(Type::Bool)
                } else {
                    type_err(
                        format!(
                            "operator '{}' needs two ints, two floats, or two strings, found {} and {}",
                            op.symbol(),
                            lt,
                            rt
                        ),
                        pos,
                    )
                }
            }
            BinOp::Eq | BinOp::Ne => {
                let comparable = |t: &Type| !matches!(t, Type::Unit | Type::Never | Type::Func(_, _));
                if comparable(&lt) && comparable(&rt) && (compatible(&lt, &rt) || compatible(&rt, &lt)) {
                    Ok(Type::Bool)
                } else {
                    type_err(
                        format!(
                            "operator '{}' cannot compare {} with {}",
                            op.symbol(),
                            lt,
                            rt
                        ),
                        pos,
                    )
                }
            }
            BinOp::And | BinOp::Or => {
                if lt == Type::Bool && rt == Type::Bool {
                    Ok(Type::Bool)
                } else {
                    type_err(
                        format!(
                            "operator '{}' needs two bools, found {} and {}",
                            op.symbol(),
                            lt,
                            rt
                        ),
                        pos,
                    )
                }
            }
        }
    }

    // ---------- calls ----------

    fn expect_arity(&self, name: &str, given: usize, wanted: usize, pos: Pos) -> TResult<()> {
        if given != wanted {
            return type_err(
                format!(
                    "'{}' takes {} argument(s) but {} were given",
                    name, wanted, given
                ),
                pos,
            );
        }
        Ok(())
    }

    fn expect_arg(&mut self, fname: &str, index: usize, arg: &Expr, expected: &Type) -> TResult<()> {
        let t = self.check_expr(arg)?;
        if !compatible(expected, &t) {
            return type_err(
                format!(
                    "argument {} of '{}' should be {}, found {}",
                    index + 1,
                    fname,
                    expected,
                    t
                ),
                arg.pos,
            );
        }
        Ok(())
    }

    fn check_call(&mut self, name: &str, args: &[Expr], pos: Pos) -> TResult<Type> {
        match name {
            "print" => {
                for arg in args {
                    let t = self.check_expr(arg)?;
                    if t == Type::Unit || t == Type::Never {
                        return type_err("print needs a value to show", arg.pos);
                    }
                }
                Ok(Type::Unit)
            }
            "range" => {
                self.expect_arity(name, args.len(), 2, pos)?;
                self.expect_arg(name, 0, &args[0], &Type::Int)?;
                self.expect_arg(name, 1, &args[1], &Type::Int)?;
                Ok(Type::List(Box::new(Type::Int)))
            }
            "read_file" => {
                self.expect_arity(name, args.len(), 1, pos)?;
                self.expect_arg(name, 0, &args[0], &Type::Str)?;
                Ok(Type::Str)
            }
            "write_file" => {
                self.expect_arity(name, args.len(), 2, pos)?;
                self.expect_arg(name, 0, &args[0], &Type::Str)?;
                self.expect_arg(name, 1, &args[1], &Type::Str)?;
                Ok(Type::Unit)
            }
            "map" | "filter" => {
                // dedicated rules instead of an ordinary signature (Section 3.4.4)
                self.expect_arity(name, args.len(), 2, pos)?;
                let ft = self.check_expr(&args[0])?;
                let lt = self.check_expr(&args[1])?;
                let (params, ret) = match ft {
                    Type::Func(p, r) => (p, *r),
                    other => {
                        return type_err(
                            format!(
                                "the first argument of {} must be a function, found {}",
                                name, other
                            ),
                            args[0].pos,
                        );
                    }
                };
                if params.len() != 1 {
                    return type_err(
                        format!(
                            "{} needs a function with exactly one parameter, but this one takes {}",
                            name,
                            params.len()
                        ),
                        args[0].pos,
                    );
                }
                let elem = match lt {
                    Type::List(inner) => *inner,
                    other => {
                        return type_err(
                            format!(
                                "the second argument of {} must be a list, found {}",
                                name, other
                            ),
                            args[1].pos,
                        );
                    }
                };
                if !compatible(&params[0], &elem) {
                    return type_err(
                        format!(
                            "{} cannot apply a function that takes {} to a list of {}",
                            name, params[0], elem
                        ),
                        args[0].pos,
                    );
                }
                if name == "map" {
                    if ret == Type::Unit || ret == Type::Never {
                        return type_err("map needs a function that returns a value", args[0].pos);
                    }
                    Ok(Type::List(Box::new(ret)))
                } else {
                    if ret != Type::Bool {
                        return type_err(
                            format!(
                                "filter needs a function that returns bool, but this one returns {}",
                                ret
                            ),
                            args[0].pos,
                        );
                    }
                    Ok(Type::List(Box::new(elem)))
                }
            }
            _ => {
                let (params, ret) = match self.funcs.get(name) {
                    Some(sig) => (sig.params.clone(), sig.ret.clone()),
                    None => {
                        return type_err(format!("undefined function '{}'", name), pos);
                    }
                };
                self.expect_arity(name, args.len(), params.len(), pos)?;
                for (i, arg) in args.iter().enumerate() {
                    self.expect_arg(name, i, arg, &params[i])?;
                }
                Ok(ret)
            }
        }
    }
}

impl Default for Checker {
    fn default() -> Checker {
        Checker::new()
    }
}
