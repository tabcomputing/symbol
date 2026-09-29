//! SYMBOL — Set-Yielding Model of Bound Operations and Logic.
//!
//! An APL-inspired tacit expression language, ported from the Crystal
//! `symbols` shard. Expressions are written left to right and evaluated
//! right to left, with no operator precedence:
//!
//! ```
//! use symbol::{Bindings, EvalResult, Value};
//!
//! let result = symbol::eval("2 * (3 + 4)", &Bindings::new()).unwrap();
//! assert_eq!(result, EvalResult::Resolved(Value::Int(14)));
//!
//! let mut bindings = Bindings::new();
//! symbol::eval_program("x = 3. y = x * 2. y + 1", &mut bindings).unwrap();
//! assert_eq!(bindings["y"], Value::Int(6));
//! ```

mod ast;
mod compat;
mod error;
mod eval;
pub mod inline;
mod lexer;
mod ops;
mod parser;
mod program;
mod value;
pub mod wasm;

pub use ast::{Expression, Op, Term};
pub use error::{Error, Result};
pub use eval::{Bindings, evaluate};
pub use lexer::{Lexer, Token, TokenKind};
pub use parser::Parser;
pub use program::eval_program;
pub use value::{EvalResult, Suspended, Value};

pub const VERSION: &str = "0.2.0";

/// Evaluate a single expression. `=` and `.` are syntax errors here; see [`eval_program`].
pub fn eval(source: &str, bindings: &Bindings) -> Result<EvalResult> {
    evaluate(&parse(source)?, bindings)
}

pub fn parse(source: &str) -> Result<Expression> {
    Parser::new(tokenize(source)).parse()
}

pub fn tokenize(source: &str) -> Vec<Token> {
    Lexer::new(source).tokenize()
}

/// Replace each `{{ expr }}` in `text` with its value (see [`inline`](mod@inline)).
/// Expressions run in program mode, so assignments update `bindings`.
pub fn inline(text: &str, bindings: &mut Bindings) -> String {
    inline::process(text, bindings)
}
