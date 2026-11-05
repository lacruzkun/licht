use std::rc::Rc;

use crate::ast::*;
use crate::error::{ErrorKind, LichtError, Pos};
use crate::lexer::{Tok, Token};
use crate::types::Type;

type PResult<T> = Result<T, LichtError>;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

/// Parses a token sequence (ending in `Tok::Eof`) into a program.
pub fn parse(tokens: Vec<Token>) -> Result<Program, LichtError> {
    let mut tokens = tokens;
    // Be forgiving if a caller built the token list by hand without an end marker.
    let needs_eof = match tokens.last() {
        Some(t) => t.tok != Tok::Eof,
        None => true,
    };
    if needs_eof {
        let pos = match tokens.last() {
            Some(t) => t.pos,
            None => Pos::new(1, 1),
        };
        tokens.push(Token { tok: Tok::Eof, pos });
    }
    let mut parser = Parser { tokens, pos: 0 };
    parser.parse_program()
}

impl Parser {
    // ---------- token helpers ----------

    fn peek(&self) -> &Tok {
        &self.tokens[self.pos].tok
    }

    fn here(&self) -> Pos {
        self.tokens[self.pos].pos
    }

    fn advance(&mut self) -> Token {
        let token = self.tokens[self.pos].clone();
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        token
    }

    fn check(&self, tok: &Tok) -> bool {
        self.peek() == tok
    }

    fn eat(&mut self, tok: &Tok) -> bool {
        if self.check(tok) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn error_expected(&self, what: &str) -> LichtError {
        LichtError::new(
            ErrorKind::Syntax,
            format!("expected {}, found {}", what, self.peek().describe()),
            self.here(),
        )
    }

    fn expect(&mut self, tok: &Tok, what: &str) -> PResult<Token> {
        if self.check(tok) {
            Ok(self.advance())
        } else {
            Err(self.error_expected(what))
        }
    }

    fn expect_ident(&mut self, what: &str) -> PResult<(String, Pos)> {
        let pos = self.here();
        match self.peek().clone() {
            Tok::Ident(name) => {
                self.advance();
                Ok((name, pos))
            }
            _ => Err(self.error_expected(what)),
        }
    }

    // ---------- program, functions, blocks ----------

    fn parse_program(&mut self) -> PResult<Program> {
        let mut items = Vec::new();
        while !self.check(&Tok::Eof) {
            if self.check(&Tok::Fn) {
                let func = self.parse_function()?;
                items.push(Item::Function(Rc::new(func)));
            } else {
                let (stmt, _) = self.parse_statement(false)?;
                items.push(Item::Stmt(stmt));
            }
        }
        Ok(Program { items })
    }

    fn parse_function(&mut self) -> PResult<FuncDecl> {
        let pos = self.here();
        self.expect(&Tok::Fn, "'fn'")?;
        let (name, _) = self.expect_ident("a function name")?;
        self.expect(&Tok::LParen, "'(' after the function name")?;
        let mut params = Vec::new();
        if !self.check(&Tok::RParen) {
            loop {
                let (pname, ppos) = self.expect_ident("a parameter name")?;
                self.expect(&Tok::Colon, "':' after the parameter name")?;
                let ty = self.parse_type()?;
                params.push(Param {
                    name: pname,
                    ty,
                    pos: ppos,
                });
                if !self.eat(&Tok::Comma) {
                    break;
                }
            }
        }
        self.expect(&Tok::RParen, "')' after the parameters")?;
        let ret = if self.eat(&Tok::Arrow) {
            self.parse_type()?
        } else {
            Type::Unit
        };
        let body = self.parse_block()?;
        Ok(FuncDecl {
            name,
            params,
            ret,
            body,
            pos,
        })
    }

    fn parse_type(&mut self) -> PResult<Type> {
        let pos = self.here();
        match self.peek().clone() {
            Tok::Ident(name) => match Type::from_name(&name) {
                Some(ty) => {
                    self.advance();
                    Ok(ty)
                }
                None => Err(LichtError::new(
                    ErrorKind::Syntax,
                    format!(
                        "unknown type '{}'; expected int, float, bool, or string",
                        name
                    ),
                    pos,
                )),
            },
            _ => Err(self.error_expected("a type (int, float, bool, or string)")),
        }
    }

    fn parse_block(&mut self) -> PResult<Block> {
        let pos = self.here();
        self.expect(&Tok::LBrace, "'{'")?;
        let mut stmts = Vec::new();
        let mut tail: Option<Box<Expr>> = None;
        while !self.check(&Tok::RBrace) {
            if self.check(&Tok::Eof) {
                return Err(self.error_expected("'}' to close the block"));
            }
            let (stmt, is_tail) = self.parse_statement(true)?;
            if is_tail {
                if let Stmt::Expr(e) = stmt {
                    tail = Some(Box::new(e));
                }
            } else {
                stmts.push(stmt);
            }
        }
        self.expect(&Tok::RBrace, "'}'")?;
        Ok(Block { stmts, tail, pos })
    }

    // ---------- statements ----------

    /// Returns the statement and whether it is the tail expression of a block.
    fn parse_statement(&mut self, allow_tail: bool) -> PResult<(Stmt, bool)> {
        let pos = self.here();
        match self.peek().clone() {
            Tok::Let => {
                self.advance();
                let (name, _) = self.expect_ident("a variable name after 'let'")?;
                self.expect(&Tok::Assign, "'=' in the variable declaration")?;
                let init = self.parse_expression()?;
                self.expect(&Tok::Semi, "';' after the variable declaration")?;
                Ok((Stmt::Let { name, init, pos }, false))
            }
            Tok::For => {
                self.advance();
                let (var, _) = self.expect_ident("a loop variable after 'for'")?;
                self.expect(&Tok::In, "'in' after the loop variable")?;
                let iter = self.parse_expression()?;
                let body = self.parse_block()?;
                Ok((
                    Stmt::For {
                        var,
                        iter,
                        body,
                        pos,
                    },
                    false,
                ))
            }
            Tok::Return => {
                self.advance();
                let value = if self.check(&Tok::Semi) {
                    None
                } else {
                    Some(self.parse_expression()?)
                };
                self.expect(&Tok::Semi, "';' after 'return'")?;
                Ok((Stmt::Return { value, pos }, false))
            }
            Tok::If => {
                let e = self.parse_if()?;
                if self.eat(&Tok::Semi) {
                    return Ok((Stmt::Expr(e), false));
                }
                if allow_tail && self.check(&Tok::RBrace) {
                    return Ok((Stmt::Expr(e), true));
                }
                Ok((Stmt::Expr(e), false))
            }
            _ => {
                let e = self.parse_expression()?;
                if self.eat(&Tok::Semi) {
                    return Ok((Stmt::Expr(e), false));
                }
                if allow_tail && self.check(&Tok::RBrace) {
                    return Ok((Stmt::Expr(e), true));
                }
                Err(self.error_expected("';' after the expression"))
            }
        }
    }

    fn parse_if(&mut self) -> PResult<Expr> {
        let pos = self.here();
        self.expect(&Tok::If, "'if'")?;
        let cond = self.parse_expression()?;
        let then_block = self.parse_block()?;
        let else_block = if self.eat(&Tok::Else) {
            if self.check(&Tok::If) {
                // `else if` is sugar for `else { if ... }`
                let nested = self.parse_if()?;
                let npos = nested.pos;
                Some(Block {
                    stmts: Vec::new(),
                    tail: Some(Box::new(nested)),
                    pos: npos,
                })
            } else {
                Some(self.parse_block()?)
            }
        } else {
            None
        };
        Ok(Expr {
            kind: ExprKind::If {
                cond: Box::new(cond),
                then_block,
                else_block,
            },
            pos,
        })
    }

    // ---------- expressions (one function per precedence level) ----------

    fn parse_expression(&mut self) -> PResult<Expr> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> PResult<Expr> {
        let left = self.parse_or()?;
        if self.check(&Tok::Assign) {
            let eq_pos = self.here();
            self.advance();
            let value = self.parse_assignment()?;
            let left_pos = left.pos;
            match left.kind {
                ExprKind::Var(name) => Ok(Expr {
                    kind: ExprKind::Assign {
                        name,
                        value: Box::new(value),
                    },
                    pos: left_pos,
                }),
                _ => Err(LichtError::new(
                    ErrorKind::Syntax,
                    "invalid assignment target: only a variable can be assigned to",
                    eq_pos,
                )),
            }
        } else {
            Ok(left)
        }
    }

    fn binary_level(
        &mut self,
        next: fn(&mut Parser) -> PResult<Expr>,
        ops: &[(Tok, BinOp)],
    ) -> PResult<Expr> {
        let mut left = next(self)?;
        loop {
            let mut matched: Option<BinOp> = None;
            for (tok, op) in ops.iter() {
                if self.check(tok) {
                    matched = Some(*op);
                    break;
                }
            }
            match matched {
                Some(op) => {
                    let pos = self.here();
                    self.advance();
                    let right = next(self)?;
                    left = Expr {
                        kind: ExprKind::Binary {
                            op,
                            left: Box::new(left),
                            right: Box::new(right),
                        },
                        pos,
                    };
                }
                None => break,
            }
        }
        Ok(left)
    }

    fn parse_or(&mut self) -> PResult<Expr> {
        self.binary_level(Parser::parse_and, &[(Tok::OrOr, BinOp::Or)])
    }

    fn parse_and(&mut self) -> PResult<Expr> {
        self.binary_level(Parser::parse_comparison, &[(Tok::AndAnd, BinOp::And)])
    }

    fn parse_comparison(&mut self) -> PResult<Expr> {
        self.binary_level(
            Parser::parse_term,
            &[
                (Tok::Lt, BinOp::Lt),
                (Tok::Gt, BinOp::Gt),
                (Tok::Le, BinOp::Le),
                (Tok::Ge, BinOp::Ge),
                (Tok::EqEq, BinOp::Eq),
                (Tok::Ne, BinOp::Ne),
            ],
        )
    }

    fn parse_term(&mut self) -> PResult<Expr> {
        self.binary_level(
            Parser::parse_factor,
            &[(Tok::Plus, BinOp::Add), (Tok::Minus, BinOp::Sub)],
        )
    }

    fn parse_factor(&mut self) -> PResult<Expr> {
        self.binary_level(
            Parser::parse_unary,
            &[
                (Tok::Star, BinOp::Mul),
                (Tok::Slash, BinOp::Div),
                (Tok::Percent, BinOp::Rem),
            ],
        )
    }

    fn parse_unary(&mut self) -> PResult<Expr> {
        let pos = self.here();
        if self.check(&Tok::Minus) {
            self.advance();
            let operand = self.parse_unary()?;
            return Ok(Expr {
                kind: ExprKind::Unary {
                    op: UnOp::Neg,
                    operand: Box::new(operand),
                },
                pos,
            });
        }
        if self.check(&Tok::Bang) {
            self.advance();
            let operand = self.parse_unary()?;
            return Ok(Expr {
                kind: ExprKind::Unary {
                    op: UnOp::Not,
                    operand: Box::new(operand),
                },
                pos,
            });
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> PResult<Expr> {
        let mut expr = self.parse_primary()?;
        while self.check(&Tok::LBracket) {
            let pos = self.here();
            self.advance();
            let index = self.parse_expression()?;
            self.expect(&Tok::RBracket, "']' after the index")?;
            expr = Expr {
                kind: ExprKind::Index {
                    target: Box::new(expr),
                    index: Box::new(index),
                },
                pos,
            };
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> PResult<Expr> {
        let pos = self.here();
        match self.peek().clone() {
            Tok::Int(v) => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Int(v),
                    pos,
                })
            }
            Tok::Float(v) => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Float(v),
                    pos,
                })
            }
            Tok::Str(s) => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Str(s),
                    pos,
                })
            }
            Tok::True => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Bool(true),
                    pos,
                })
            }
            Tok::False => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Bool(false),
                    pos,
                })
            }
            Tok::Ident(name) => {
                self.advance();
                if self.check(&Tok::LParen) {
                    self.advance();
                    let mut args = Vec::new();
                    if !self.check(&Tok::RParen) {
                        loop {
                            args.push(self.parse_expression()?);
                            if !self.eat(&Tok::Comma) {
                                break;
                            }
                        }
                    }
                    self.expect(&Tok::RParen, "')' after the arguments")?;
                    Ok(Expr {
                        kind: ExprKind::Call { name, args },
                        pos,
                    })
                } else {
                    Ok(Expr {
                        kind: ExprKind::Var(name),
                        pos,
                    })
                }
            }
            Tok::LBracket => {
                self.advance();
                let mut items = Vec::new();
                if !self.check(&Tok::RBracket) {
                    loop {
                        items.push(self.parse_expression()?);
                        if !self.eat(&Tok::Comma) {
                            break;
                        }
                    }
                }
                self.expect(&Tok::RBracket, "']' to close the list")?;
                Ok(Expr {
                    kind: ExprKind::List(items),
                    pos,
                })
            }
            Tok::LParen => {
                self.advance();
                let inner = self.parse_expression()?;
                self.expect(&Tok::RParen, "')'")?;
                Ok(inner)
            }
            Tok::If => self.parse_if(),
            _ => Err(self.error_expected("an expression")),
        }
    }
}
