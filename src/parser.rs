use crate::ast::{BinOp, Block, Expr, LogicOp, Stmt, UnaryOp};
use crate::error::{SpriteError, SpriteResult};
use crate::lexer::Token;

const MAX_DEPTH: usize = 200;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    depth: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            pos: 0,
            depth: 0,
        }
    }

    fn enter(&mut self) -> SpriteResult<()> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(SpriteError::Parse("expression nested too deeply".into()));
        }
        Ok(())
    }

    fn exit(&mut self) {
        self.depth -= 1;
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) -> Token {
        let t = self.tokens.get(self.pos).cloned().unwrap_or(Token::Eof);
        if !matches!(t, Token::Eof) {
            self.pos += 1;
        }
        t
    }

    fn check(&self, t: &Token) -> bool {
        self.peek() == t
    }

    fn matches(&mut self, t: &Token) -> bool {
        if self.check(t) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, t: &Token, what: &str) -> SpriteResult<()> {
        if self.matches(t) {
            Ok(())
        } else {
            Err(SpriteError::Parse(format!(
                "expected {}, found {:?}",
                what,
                self.peek()
            )))
        }
    }

    pub fn parse_program(&mut self) -> SpriteResult<Block> {
        let mut stmts = Vec::new();
        while !self.check(&Token::Eof) {
            stmts.push(self.parse_stmt()?);
        }
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> SpriteResult<Stmt> {
        self.enter()?;
        let result = self.parse_stmt_inner();
        self.exit();
        result
    }

    fn parse_stmt_inner(&mut self) -> SpriteResult<Stmt> {
        match self.peek().clone() {
            Token::Let => {
                self.advance();
                let name = self.expect_ident()?;
                self.expect(&Token::Eq, "'=' after let name")?;
                let expr = self.parse_expr()?;
                self.matches(&Token::Semicolon);
                Ok(Stmt::Let(name, expr))
            }
            Token::Fn => {
                self.advance();
                let name = self.expect_ident()?;
                self.expect(&Token::LParen, "'(' after function name")?;
                let mut params = Vec::new();
                if !self.check(&Token::RParen) {
                    loop {
                        params.push(self.expect_ident()?);
                        if !self.matches(&Token::Comma) {
                            break;
                        }
                    }
                }
                self.expect(&Token::RParen, "')' after parameters")?;
                let body = self.parse_block()?;
                Ok(Stmt::FnDecl(name, params, body))
            }
            Token::If => {
                self.advance();
                let cond = self.parse_expr()?;
                let then_block = self.parse_block()?;
                let else_block = if self.matches(&Token::Else) {
                    if self.check(&Token::If) {
                        Some(vec![self.parse_stmt()?])
                    } else {
                        Some(self.parse_block()?)
                    }
                } else {
                    None
                };
                Ok(Stmt::If(cond, then_block, else_block))
            }
            Token::While => {
                self.advance();
                let cond = self.parse_expr()?;
                let body = self.parse_block()?;
                Ok(Stmt::While(cond, body))
            }
            Token::Return => {
                self.advance();
                let expr = if self.check(&Token::Semicolon)
                    || self.check(&Token::RBrace)
                    || self.check(&Token::Eof)
                {
                    None
                } else {
                    Some(self.parse_expr()?)
                };
                self.matches(&Token::Semicolon);
                Ok(Stmt::Return(expr))
            }
            Token::LBrace => Ok(Stmt::Block(self.parse_block()?)),
            _ => {
                let expr = self.parse_expr()?;
                self.matches(&Token::Semicolon);
                Ok(Stmt::Expr(expr))
            }
        }
    }

    fn parse_block(&mut self) -> SpriteResult<Block> {
        self.enter()?;
        self.expect(&Token::LBrace, "'{'")?;
        let mut stmts = Vec::new();
        while !self.check(&Token::RBrace) && !self.check(&Token::Eof) {
            stmts.push(self.parse_stmt()?);
        }
        self.expect(&Token::RBrace, "'}'")?;
        self.exit();
        Ok(stmts)
    }

    fn expect_ident(&mut self) -> SpriteResult<String> {
        match self.advance() {
            Token::Ident(name) => Ok(name),
            other => Err(SpriteError::Parse(format!(
                "expected identifier, found {:?}",
                other
            ))),
        }
    }

    fn parse_expr(&mut self) -> SpriteResult<Expr> {
        self.enter()?;
        let result = self.parse_assignment();
        self.exit();
        result
    }

    fn parse_assignment(&mut self) -> SpriteResult<Expr> {
        let expr = self.parse_or()?;
        if self.matches(&Token::Eq) {
            let value = self.parse_assignment()?;
            if let Expr::Ident(name) = expr {
                return Ok(Expr::Assign(name, Box::new(value)));
            }
            return Err(SpriteError::Parse("invalid assignment target".into()));
        }
        Ok(expr)
    }

    fn parse_or(&mut self) -> SpriteResult<Expr> {
        let mut expr = self.parse_and()?;
        while self.matches(&Token::Or) {
            let rhs = self.parse_and()?;
            expr = Expr::Logical(Box::new(expr), LogicOp::Or, Box::new(rhs));
        }
        Ok(expr)
    }

    fn parse_and(&mut self) -> SpriteResult<Expr> {
        let mut expr = self.parse_equality()?;
        while self.matches(&Token::And) {
            let rhs = self.parse_equality()?;
            expr = Expr::Logical(Box::new(expr), LogicOp::And, Box::new(rhs));
        }
        Ok(expr)
    }

    fn parse_equality(&mut self) -> SpriteResult<Expr> {
        let mut expr = self.parse_comparison()?;
        loop {
            let op = match self.peek() {
                Token::EqEq => BinOp::Eq,
                Token::NotEq => BinOp::NotEq,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_comparison()?;
            expr = Expr::Binary(Box::new(expr), op, Box::new(rhs));
        }
        Ok(expr)
    }

    fn parse_comparison(&mut self) -> SpriteResult<Expr> {
        let mut expr = self.parse_term()?;
        loop {
            let op = match self.peek() {
                Token::Lt => BinOp::Lt,
                Token::Gt => BinOp::Gt,
                Token::LtEq => BinOp::LtEq,
                Token::GtEq => BinOp::GtEq,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_term()?;
            expr = Expr::Binary(Box::new(expr), op, Box::new(rhs));
        }
        Ok(expr)
    }

    fn parse_term(&mut self) -> SpriteResult<Expr> {
        let mut expr = self.parse_factor()?;
        loop {
            let op = match self.peek() {
                Token::Plus => BinOp::Add,
                Token::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_factor()?;
            expr = Expr::Binary(Box::new(expr), op, Box::new(rhs));
        }
        Ok(expr)
    }

    fn parse_factor(&mut self) -> SpriteResult<Expr> {
        let mut expr = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Token::Star => BinOp::Mul,
                Token::Slash => BinOp::Div,
                Token::Percent => BinOp::Mod,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_unary()?;
            expr = Expr::Binary(Box::new(expr), op, Box::new(rhs));
        }
        Ok(expr)
    }

    fn parse_unary(&mut self) -> SpriteResult<Expr> {
        self.enter()?;
        let result = match self.peek() {
            Token::Minus => {
                self.advance();
                let rhs = self.parse_unary()?;
                Ok(Expr::Unary(UnaryOp::Neg, Box::new(rhs)))
            }
            Token::Not => {
                self.advance();
                let rhs = self.parse_unary()?;
                Ok(Expr::Unary(UnaryOp::Not, Box::new(rhs)))
            }
            _ => self.parse_call(),
        };
        self.exit();
        result
    }

    fn parse_call(&mut self) -> SpriteResult<Expr> {
        let mut expr = self.parse_primary()?;
        loop {
            if self.matches(&Token::LParen) {
                let mut args = Vec::new();
                if !self.check(&Token::RParen) {
                    loop {
                        args.push(self.parse_expr()?);
                        if !self.matches(&Token::Comma) {
                            break;
                        }
                    }
                }
                self.expect(&Token::RParen, "')' after arguments")?;
                expr = Expr::Call(Box::new(expr), args);
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> SpriteResult<Expr> {
        self.enter()?;
        let result = self.parse_primary_inner();
        self.exit();
        result
    }

    fn parse_primary_inner(&mut self) -> SpriteResult<Expr> {
        match self.advance() {
            Token::Number(n) => Ok(Expr::Number(n)),
            Token::Str(s) => Ok(Expr::Str(s)),
            Token::True => Ok(Expr::Bool(true)),
            Token::False => Ok(Expr::Bool(false)),
            Token::Nil => Ok(Expr::Nil),
            Token::Ident(name) => Ok(Expr::Ident(name)),
            Token::LParen => {
                let expr = self.parse_expr()?;
                self.expect(&Token::RParen, "')' after expression")?;
                Ok(expr)
            }
            other => Err(SpriteError::Parse(format!(
                "unexpected token {:?}",
                other
            ))),
        }
    }
}

pub fn parse(tokens: Vec<Token>) -> SpriteResult<Block> {
    Parser::new(tokens).parse_program()
}
