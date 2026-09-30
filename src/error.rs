//! Errors. Each variant corresponds to an exception the Crystal implementation
//! raises, and `Display` reproduces that exception's message exactly (the WASM
//! interface returns it as `"Error: <message>"`).

use std::fmt;

use crate::ast::Op;
use crate::compat;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    /// Syntax error (`SYMBOL::ParseError`).
    Parse { message: String, line: usize, column: usize },
    /// A numeric literal that doesn't fit its type (`ArgumentError`).
    InvalidNumber { type_name: &'static str, text: String },
    /// Checked integer arithmetic or a float-to-int conversion overflowed (`OverflowError`).
    Overflow,
    /// `Int64::MIN / -1` (`ArgumentError`).
    DivisionOverflow,
    /// Modulo by zero (`DivisionByZeroError`). Division by zero yields `Infinity` instead.
    DivisionByZero,
    /// A vectorized operation's right array is shorter than its left (`IndexError`).
    IndexOutOfBounds,
    /// `⌈` / `⌊` of an empty array (`Enumerable::EmptyError`).
    Empty,
    /// `⌈` / `⌊` over values that include NaN (`ArgumentError`).
    Comparison(f64, f64),
    /// A list element with an operator still waiting for arguments (`[1 +]`).
    /// Crystal has no such error: its list elements can't be expressions.
    IncompleteElement(Op),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Parse { message, line, column } => write!(f, "{line}:{column}: {message}"),
            Error::InvalidNumber { type_name, text } => {
                write!(f, "Invalid {type_name}: {}", compat::inspect_str(text))
            }
            Error::Overflow => f.write_str("Arithmetic overflow"),
            Error::DivisionOverflow => f.write_str("Overflow: Int64::MIN / -1"),
            Error::DivisionByZero => f.write_str("Division by 0"),
            Error::IndexOutOfBounds => f.write_str("Index out of bounds"),
            Error::Empty => f.write_str("Empty enumerable"),
            Error::Comparison(a, b) => write!(
                f,
                "Comparison of {} and {} failed",
                compat::format_float(*a),
                compat::format_float(*b)
            ),
            Error::IncompleteElement(op) => write!(f, "A list element is incomplete: {op} needs an argument"),
        }
    }
}

impl std::error::Error for Error {}
