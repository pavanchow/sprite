use crate::error::{SpriteError, SpriteResult};

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Number(f64),
    Str(String),
    Ident(String),
    True,
    False,
    Nil,
    Let,
    Fn,
    If,
    Else,
    While,
    Return,
    And,
    Or,
    Not,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    EqEq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Semicolon,
    Eof,
}

pub struct Lexer<'a> {
    chars: Vec<char>,
    pos: usize,
    _src: &'a str,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Lexer {
            chars: src.chars().collect(),
            pos: 0,
            _src: src,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.peek();
        self.pos += 1;
        c
    }

    fn skip_ws_and_comments(&mut self) {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() => {
                    self.advance();
                }
                Some('#') => {
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.advance();
                    }
                }
                _ => break,
            }
        }
    }

    fn read_number(&mut self) -> SpriteResult<Token> {
        let start = self.pos;
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.advance();
        }
        if self.peek() == Some('.') && matches!(self.peek_next(), Some(c) if c.is_ascii_digit()) {
            self.advance();
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.advance();
            }
        }
        let text: String = self.chars[start..self.pos].iter().collect();
        text.parse::<f64>()
            .map(Token::Number)
            .map_err(|_| SpriteError::Lex(format!("invalid number literal '{}'", text)))
    }

    fn read_string(&mut self) -> SpriteResult<Token> {
        self.advance(); // consume opening quote
        let mut s = String::new();
        loop {
            match self.advance() {
                Some('"') => return Ok(Token::Str(s)),
                Some('\\') => match self.advance() {
                    Some('n') => s.push('\n'),
                    Some('t') => s.push('\t'),
                    Some('"') => s.push('"'),
                    Some('\\') => s.push('\\'),
                    Some(other) => s.push(other),
                    None => return Err(SpriteError::Lex("unterminated string".into())),
                },
                Some(c) => s.push(c),
                None => return Err(SpriteError::Lex("unterminated string".into())),
            }
        }
    }

    fn read_ident(&mut self) -> Token {
        let start = self.pos;
        while matches!(self.peek(), Some(c) if c.is_alphanumeric() || c == '_') {
            self.advance();
        }
        let text: String = self.chars[start..self.pos].iter().collect();
        match text.as_str() {
            "true" => Token::True,
            "false" => Token::False,
            "nil" => Token::Nil,
            "let" => Token::Let,
            "fn" => Token::Fn,
            "if" => Token::If,
            "else" => Token::Else,
            "while" => Token::While,
            "return" => Token::Return,
            "and" => Token::And,
            "or" => Token::Or,
            "not" => Token::Not,
            _ => Token::Ident(text),
        }
    }

    pub fn tokenize(mut self) -> SpriteResult<Vec<Token>> {
        let mut tokens = Vec::new();
        loop {
            self.skip_ws_and_comments();
            let c = match self.peek() {
                None => {
                    tokens.push(Token::Eof);
                    break;
                }
                Some(c) => c,
            };
            let tok = match c {
                '0'..='9' => self.read_number()?,
                '"' => self.read_string()?,
                c if c.is_alphabetic() || c == '_' => self.read_ident(),
                '+' => {
                    self.advance();
                    Token::Plus
                }
                '-' => {
                    self.advance();
                    Token::Minus
                }
                '*' => {
                    self.advance();
                    Token::Star
                }
                '/' => {
                    self.advance();
                    Token::Slash
                }
                '%' => {
                    self.advance();
                    Token::Percent
                }
                '=' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        Token::EqEq
                    } else {
                        Token::Eq
                    }
                }
                '!' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        Token::NotEq
                    } else {
                        return Err(SpriteError::Lex("unexpected character '!'".into()));
                    }
                }
                '<' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        Token::LtEq
                    } else {
                        Token::Lt
                    }
                }
                '>' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        Token::GtEq
                    } else {
                        Token::Gt
                    }
                }
                '(' => {
                    self.advance();
                    Token::LParen
                }
                ')' => {
                    self.advance();
                    Token::RParen
                }
                '{' => {
                    self.advance();
                    Token::LBrace
                }
                '}' => {
                    self.advance();
                    Token::RBrace
                }
                ',' => {
                    self.advance();
                    Token::Comma
                }
                ';' => {
                    self.advance();
                    Token::Semicolon
                }
                other => {
                    return Err(SpriteError::Lex(format!(
                        "unexpected character '{}'",
                        other
                    )))
                }
            };
            tokens.push(tok);
        }
        Ok(tokens)
    }
}
