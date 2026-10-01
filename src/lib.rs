//! SYMBOL — Set-Yielding Model of Bound Operations and Logic.
//!
//! An APL-inspired tacit expression language, ported from the Crystal
//! `symbols` shard. Expressions are read right to left, with no operator
//! precedence.
//!
//! SYMBOL is poly-fix. Every operator has a fixed arity, so it can stand
//! before, between or after its arguments: `+ 1 2`, `1 + 2` and `1 2 +` are
//! all 3.
//!
//! - An operator takes the finished values to its right, up to its arity,
//!   and waits for the rest from its left.
//! - A value goes to the waiting operator on its right first (postfix
//!   precedence), so `2 3 4 + *` is `2 * (3 + 4)`.
//! - An operator runs as soon as it has all its arguments, and its result is
//!   a value like any other.
//! - Arguments keep their textual order, wherever the operator stands.
//! - A group is evaluated first, and its pieces then take part in the
//!   enclosing expression: `(1 +) 2` and `(1 2) +` are both 3.
//!
//! What doesn't combine is a partial application: an operator waiting for
//! arguments ([`EvalResult::Suspended`]), or several pieces
//! ([`EvalResult::Sequence`]), such as values waiting for an operator.
//!
//! ```
//! use symbol::{Bindings, EvalResult, Value};
//!
//! let result = symbol::eval("2 * 3 + 4", &Bindings::new()).unwrap();
//! assert_eq!(result, EvalResult::Resolved(Value::Int(14)));
//!
//! for expr in ["2 * (3 + 4)", "* 2 (+ 3 4)", "2 3 4 + *"] {
//!     assert_eq!(symbol::eval(expr, &Bindings::new()).unwrap(), EvalResult::Resolved(Value::Int(14)));
//! }
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
pub use lexer::{Lexer, Token, TokenKind, latex_operator};
pub use parser::Parser;
pub use program::eval_program;
pub use value::{EvalResult, Suspended, Value};

pub const VERSION: &str = "0.2.0";

// The README's Rust examples run as doctests.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;

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
