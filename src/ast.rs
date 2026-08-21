#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(f64),
    Str(String),
    Bool(bool),
    Nil,
    Ident(String),
    Unary(UnaryOp, Box<Expr>),
    Binary(Box<Expr>, BinOp, Box<Expr>),
    Logical(Box<Expr>, LogicOp, Box<Expr>),
    Assign(String, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogicOp {
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Let(String, Expr),
    Expr(Expr),
    If(Expr, Block, Option<Block>),
    While(Expr, Block),
    FnDecl(String, Vec<String>, Block),
    Return(Option<Expr>),
    Block(Block),
}

pub type Block = Vec<Stmt>;
