use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum SpriteError {
    Lex(String),
    Parse(String),
    Runtime(String),
}

impl fmt::Display for SpriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpriteError::Lex(msg) => write!(f, "lex error: {}", msg),
            SpriteError::Parse(msg) => write!(f, "parse error: {}", msg),
            SpriteError::Runtime(msg) => write!(f, "runtime error: {}", msg),
        }
    }
}

impl std::error::Error for SpriteError {}

pub type SpriteResult<T> = Result<T, SpriteError>;
