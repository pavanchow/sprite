use std::rc::Rc;

use crate::ast::{BinOp, Block, Expr, LogicOp, Stmt, UnaryOp};
use crate::error::{SpriteError, SpriteResult};
use crate::value::{Env, FunctionDef, Scope, Value};

const MAX_CALL_DEPTH: usize = 512;

pub struct Interpreter {
    globals: Env,
    call_depth: usize,
}

enum Signal {
    None(Value),
    Return(Value),
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            globals: Scope::new(),
            call_depth: 0,
        }
    }

    pub fn eval_program(&mut self, program: &Block) -> SpriteResult<Value> {
        let env = Rc::clone(&self.globals);
        match self.exec_stmts(program, &env)? {
            Signal::None(v) => Ok(v),
            Signal::Return(v) => Ok(v),
        }
    }

    fn exec_stmts(&mut self, stmts: &[Stmt], env: &Env) -> SpriteResult<Signal> {
        let mut last = Value::Nil;
        for stmt in stmts {
            match self.exec_stmt(stmt, env)? {
                Signal::None(v) => last = v,
                sig @ Signal::Return(_) => return Ok(sig),
            }
        }
        Ok(Signal::None(last))
    }

    fn exec_block(&mut self, block: &Block, env: &Env) -> SpriteResult<Signal> {
        let scope = Scope::child(env);
        self.exec_stmts(block, &scope)
    }

    fn exec_stmt(&mut self, stmt: &Stmt, env: &Env) -> SpriteResult<Signal> {
        match stmt {
            Stmt::Let(name, expr) => {
                let value = self.eval_expr(expr, env)?;
                env.borrow_mut().define(name, value);
                Ok(Signal::None(Value::Nil))
            }
            Stmt::Expr(expr) => {
                let value = self.eval_expr(expr, env)?;
                Ok(Signal::None(value))
            }
            Stmt::If(cond, then_block, else_block) => {
                if self.eval_expr(cond, env)?.is_truthy() {
                    self.exec_block(then_block, env)
                } else if let Some(else_block) = else_block {
                    self.exec_block(else_block, env)
                } else {
                    Ok(Signal::None(Value::Nil))
                }
            }
            Stmt::While(cond, body) => {
                while self.eval_expr(cond, env)?.is_truthy() {
                    match self.exec_block(body, env)? {
                        Signal::None(_) => {}
                        sig @ Signal::Return(_) => return Ok(sig),
                    }
                }
                Ok(Signal::None(Value::Nil))
            }
            Stmt::FnDecl(name, params, body) => {
                let func = Value::Function(Rc::new(FunctionDef {
                    name: name.clone(),
                    params: params.clone(),
                    body: body.clone(),
                    closure: Rc::clone(env),
                }));
                env.borrow_mut().define(name, func);
                Ok(Signal::None(Value::Nil))
            }
            Stmt::Return(expr) => {
                let value = match expr {
                    Some(e) => self.eval_expr(e, env)?,
                    None => Value::Nil,
                };
                Ok(Signal::Return(value))
            }
            Stmt::Block(block) => self.exec_block(block, env),
        }
    }

    fn eval_expr(&mut self, expr: &Expr, env: &Env) -> SpriteResult<Value> {
        match expr {
            Expr::Number(n) => Ok(Value::Number(*n)),
            Expr::Str(s) => Ok(Value::Str(s.clone())),
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::Nil => Ok(Value::Nil),
            Expr::Ident(name) => env
                .borrow()
                .get(name)
                .ok_or_else(|| SpriteError::Runtime(format!("undefined variable '{}'", name))),
            Expr::Unary(op, rhs) => {
                let v = self.eval_expr(rhs, env)?;
                match op {
                    UnaryOp::Neg => match v {
                        Value::Number(n) => Ok(Value::Number(-n)),
                        other => Err(SpriteError::Runtime(format!(
                            "cannot negate a {}",
                            other.type_name()
                        ))),
                    },
                    UnaryOp::Not => Ok(Value::Bool(!v.is_truthy())),
                }
            }
            Expr::Logical(lhs, op, rhs) => {
                let l = self.eval_expr(lhs, env)?;
                match op {
                    LogicOp::And => {
                        if !l.is_truthy() {
                            Ok(l)
                        } else {
                            self.eval_expr(rhs, env)
                        }
                    }
                    LogicOp::Or => {
                        if l.is_truthy() {
                            Ok(l)
                        } else {
                            self.eval_expr(rhs, env)
                        }
                    }
                }
            }
            Expr::Binary(lhs, op, rhs) => {
                let l = self.eval_expr(lhs, env)?;
                let r = self.eval_expr(rhs, env)?;
                self.eval_binary(*op, l, r)
            }
            Expr::Assign(name, value) => {
                let v = self.eval_expr(value, env)?;
                if env.borrow_mut().assign(name, v.clone()) {
                    Ok(v)
                } else {
                    Err(SpriteError::Runtime(format!(
                        "cannot assign to undefined variable '{}'",
                        name
                    )))
                }
            }
            Expr::Call(callee, args) => self.eval_call(callee, args, env),
        }
    }

    fn eval_binary(&self, op: BinOp, l: Value, r: Value) -> SpriteResult<Value> {
        use BinOp::*;
        match op {
            Add => match (&l, &r) {
                (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a + b)),
                (Value::Str(_), _) | (_, Value::Str(_)) => {
                    Ok(Value::Str(format!("{}{}", l, r)))
                }
                _ => Err(type_err("+", &l, &r)),
            },
            Sub => numeric(l, r, "-", |a, b| a - b),
            Mul => numeric(l, r, "*", |a, b| a * b),
            Div => match (&l, &r) {
                (Value::Number(_), Value::Number(b)) if *b == 0.0 => {
                    Err(SpriteError::Runtime("division by zero".into()))
                }
                (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a / b)),
                _ => Err(type_err("/", &l, &r)),
            },
            Mod => match (&l, &r) {
                (Value::Number(_), Value::Number(b)) if *b == 0.0 => {
                    Err(SpriteError::Runtime("division by zero".into()))
                }
                (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a % b)),
                _ => Err(type_err("%", &l, &r)),
            },
            Eq => Ok(Value::Bool(l.values_equal(&r))),
            NotEq => Ok(Value::Bool(!l.values_equal(&r))),
            Lt => compare(l, r, "<", |o| o == std::cmp::Ordering::Less),
            Gt => compare(l, r, ">", |o| o == std::cmp::Ordering::Greater),
            LtEq => compare(l, r, "<=", |o| o != std::cmp::Ordering::Greater),
            GtEq => compare(l, r, ">=", |o| o != std::cmp::Ordering::Less),
        }
    }

    fn eval_call(&mut self, callee: &Expr, args: &[Expr], env: &Env) -> SpriteResult<Value> {
        if let Expr::Ident(name) = callee {
            if name == "print" {
                let mut parts = Vec::new();
                for a in args {
                    parts.push(self.eval_expr(a, env)?.to_string());
                }
                println!("{}", parts.join(" "));
                return Ok(Value::Nil);
            }
        }

        let callee_val = self.eval_expr(callee, env)?;
        let mut arg_vals = Vec::with_capacity(args.len());
        for a in args {
            arg_vals.push(self.eval_expr(a, env)?);
        }

        match callee_val {
            Value::Function(func) => self.call_function(&func, arg_vals),
            other => Err(SpriteError::Runtime(format!(
                "cannot call a {}",
                other.type_name()
            ))),
        }
    }

    fn call_function(&mut self, func: &Rc<FunctionDef>, args: Vec<Value>) -> SpriteResult<Value> {
        if args.len() != func.params.len() {
            return Err(SpriteError::Runtime(format!(
                "function '{}' expects {} argument(s), got {}",
                func.name,
                func.params.len(),
                args.len()
            )));
        }
        self.call_depth += 1;
        if self.call_depth > MAX_CALL_DEPTH {
            self.call_depth -= 1;
            return Err(SpriteError::Runtime("stack overflow".into()));
        }
        let scope = Scope::child(&func.closure);
        for (param, value) in func.params.iter().zip(args) {
            scope.borrow_mut().define(param, value);
        }
        let result = match self.exec_block(&func.body, &scope)? {
            Signal::Return(v) => Ok(v),
            Signal::None(_) => Ok(Value::Nil),
        };
        self.call_depth -= 1;
        result
    }
}

fn numeric(l: Value, r: Value, op: &str, f: impl Fn(f64, f64) -> f64) -> SpriteResult<Value> {
    match (&l, &r) {
        (Value::Number(a), Value::Number(b)) => Ok(Value::Number(f(*a, *b))),
        _ => Err(type_err(op, &l, &r)),
    }
}

fn compare(l: Value, r: Value, op: &str, f: impl Fn(std::cmp::Ordering) -> bool) -> SpriteResult<Value> {
    match (&l, &r) {
        (Value::Number(a), Value::Number(b)) => match a.partial_cmp(b) {
            Some(ord) => Ok(Value::Bool(f(ord))),
            None => Err(SpriteError::Runtime("cannot compare NaN".into())),
        },
        (Value::Str(a), Value::Str(b)) => Ok(Value::Bool(f(a.cmp(b)))),
        _ => Err(type_err(op, &l, &r)),
    }
}

fn type_err(op: &str, l: &Value, r: &Value) -> SpriteError {
    SpriteError::Runtime(format!(
        "type error: cannot apply '{}' to {} and {}",
        op,
        l.type_name(),
        r.type_name()
    ))
}
