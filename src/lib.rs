pub mod ast;
pub mod error;
pub mod interpreter;
pub mod lexer;
pub mod parser;
pub mod value;

pub use error::{SpriteError, SpriteResult};
pub use value::Value;

use interpreter::Interpreter;

/// Evaluate a Sprite source string and return the value of the last
/// expression statement, or `Value::Nil` if the program ends in a
/// non-expression statement.
///
/// ```
/// let v = sprite::eval("2 + 3 * 4").unwrap();
/// assert_eq!(v.to_string(), "14");
/// ```
pub fn eval(source: &str) -> SpriteResult<Value> {
    let tokens = lexer::Lexer::new(source).tokenize()?;
    let program = parser::parse(tokens)?;
    let mut interp = Interpreter::new();
    interp.eval_program(&program)
}

/// A reusable interpreter session. Each call to `run` shares the same
/// global environment, so variables and functions persist across calls.
/// This is what the REPL uses.
pub struct Session {
    interp: Interpreter,
}

impl Session {
    pub fn new() -> Self {
        Session {
            interp: Interpreter::new(),
        }
    }

    pub fn run(&mut self, source: &str) -> SpriteResult<Value> {
        let tokens = lexer::Lexer::new(source).tokenize()?;
        let program = parser::parse(tokens)?;
        self.interp.eval_program(&program)
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}
