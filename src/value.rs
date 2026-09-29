//! Runtime values and evaluation results (port of `tacit/term.cr`).

use std::fmt;

use crate::ast::Op;
use crate::compat;
use crate::error::Result;

/// A runtime value: Crystal's `TacitValue = Int64 | Float64 | String | Bool | Array | Nil`.
///
/// `PartialEq` is structural and type-exact (`Int(1) != Float(1.0)`), which is
/// what Rust callers and tests want. The language's own `==` is [`Value::loose_eq`].
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Value {
    #[default]
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Array(Vec<Value>),
}

impl Value {
    pub fn is_nil(&self) -> bool {
        matches!(self, Value::Nil)
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// Numeric view used by comparisons and mixed arithmetic.
    /// Strings are parsed (0.0 if they aren't numbers); arrays count their items.
    pub fn to_float(&self) -> f64 {
        match self {
            Value::Int(i) => *i as f64,
            Value::Float(x) => *x,
            Value::Str(s) => compat::parse_float(s).unwrap_or(0.0),
            Value::Bool(b) => f64::from(u8::from(*b)),
            Value::Array(items) => items.len() as f64,
            Value::Nil => 0.0,
        }
    }

    /// Integer view used by ranges, indexing and bitwise operators.
    /// Floats truncate, failing if they don't fit in an `i64`.
    pub fn to_int(&self) -> Result<i64> {
        Ok(match self {
            Value::Int(i) => *i,
            Value::Float(x) => compat::float_to_i64(*x)?,
            Value::Str(s) => compat::parse_int(s).unwrap_or(0),
            Value::Bool(b) => i64::from(*b),
            Value::Array(items) => items.len() as i64,
            Value::Nil => 0,
        })
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Int(i) => *i != 0,
            Value::Float(x) => *x != 0.0,
            Value::Str(s) => !s.is_empty(),
            Value::Bool(b) => *b,
            Value::Array(items) => !items.is_empty(),
            Value::Nil => false,
        }
    }

    /// Collection view used by aggregate and structural operators:
    /// strings split into characters, nil is empty, scalars become one-item arrays.
    pub fn into_array(self) -> Vec<Value> {
        match self {
            Value::Array(items) => items,
            Value::Str(s) => s.chars().map(|c| Value::Str(c.to_string())).collect(),
            Value::Nil => Vec::new(),
            scalar => vec![scalar],
        }
    }

    /// SYMBOL's `==`: numbers compare by value across `Int`/`Float`,
    /// arrays compare element-wise, and other types only equal themselves.
    pub fn loose_eq(&self, other: &Value) -> bool {
        use Value::*;
        match (self, other) {
            (Int(a), Int(b)) => a == b,
            (Int(a), Float(b)) | (Float(b), Int(a)) => *a as f64 == *b,
            (Float(a), Float(b)) => a == b,
            (Str(a), Str(b)) => a == b,
            (Bool(a), Bool(b)) => a == b,
            (Nil, Nil) => true,
            (Array(a), Array(b)) => a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.loose_eq(y)),
            _ => false,
        }
    }

    /// Crystal's `inspect`: like `Display`, but strings are quoted and nil is `nil`.
    pub fn inspect(&self) -> String {
        match self {
            Value::Nil => "nil".to_owned(),
            Value::Str(s) => compat::inspect_str(s),
            Value::Array(items) => format!("[{}]", join(items.iter().map(Value::inspect))),
            scalar => scalar.to_string(),
        }
    }
}

/// Crystal's `to_s`: strings bare, nil empty, floats always with a decimal
/// point (`5.0`, `1.0e+20`), arrays showing their items with `inspect`.
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Nil => Ok(()),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Int(i) => write!(f, "{i}"),
            Value::Float(x) => f.write_str(&compat::format_float(*x)),
            Value::Str(s) => f.write_str(s),
            Value::Array(_) => f.write_str(&self.inspect()),
        }
    }
}

impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Value::Int(i)
    }
}

impl From<f64> for Value {
    fn from(x: f64) -> Self {
        Value::Float(x)
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::Str(s.to_owned())
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::Str(s)
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(items: Vec<T>) -> Self {
        Value::Array(items.into_iter().map(Into::into).collect())
    }
}

/// The result of evaluating an expression.
#[derive(Debug, Clone, PartialEq)]
pub enum EvalResult {
    /// A concrete value.
    Resolved(Value),
    /// An operator still waiting for arguments.
    Suspended(Suspended),
    /// A variable with no binding.
    Unbound(String),
}

impl EvalResult {
    /// Crystal's default `Reference#inspect`, including an object address
    /// (meaningless in both implementations, but part of the output format).
    fn inspect(&self) -> String {
        let address = self as *const Self as usize;
        match self {
            EvalResult::Resolved(value) => {
                format!("#<SYMBOL::Tacit::Resolved:{address:#x} @value={}>", value.inspect())
            }
            EvalResult::Unbound(name) => {
                format!("#<SYMBOL::Tacit::Unbound:{address:#x} @name={}>", compat::inspect_str(name))
            }
            EvalResult::Suspended(s) => format!(
                "#<SYMBOL::Tacit::Suspended:{address:#x} @op={}, @arity={}, @args=[{}]>",
                compat::inspect_str(s.op.symbol()),
                s.arity(),
                join(s.args.iter().map(EvalResult::inspect)),
            ),
        }
    }
}

/// Crystal's `to_s`: `Resolved(5)`, `Unbound(x)`, `Suspended(+, args=[...])`.
impl fmt::Display for EvalResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvalResult::Resolved(value) => write!(f, "Resolved({value})"),
            EvalResult::Unbound(name) => write!(f, "Unbound({name})"),
            EvalResult::Suspended(s) => {
                write!(f, "Suspended({}, args=[{}])", s.op, join(s.args.iter().map(EvalResult::inspect)))
            }
        }
    }
}

/// A partially applied operator. Arguments are stored left to right.
#[derive(Debug, Clone, PartialEq)]
pub struct Suspended {
    pub op: Op,
    pub args: Vec<EvalResult>,
}

impl Suspended {
    pub fn new(op: Op, args: Vec<EvalResult>) -> Self {
        Suspended { op, args }
    }

    pub fn arity(&self) -> usize {
        self.op.arity()
    }

    pub fn needs_args(&self) -> usize {
        self.arity().saturating_sub(self.args.len())
    }

    pub fn is_complete(&self) -> bool {
        self.args.len() >= self.arity()
    }
}

impl From<Suspended> for EvalResult {
    fn from(s: Suspended) -> Self {
        EvalResult::Suspended(s)
    }
}

pub(crate) fn join(parts: impl Iterator<Item = String>) -> String {
    parts.collect::<Vec<_>>().join(", ")
}
