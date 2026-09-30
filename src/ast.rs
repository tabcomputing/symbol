//! Expression syntax tree (port of `ast.cr`).

use std::fmt;

use crate::value::Value;

/// A parsed expression: a flat sequence of terms, read right to left.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Expression {
    pub terms: Vec<Term>,
}

impl Expression {
    pub fn new(terms: Vec<Term>) -> Self {
        Expression { terms }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Term {
    /// A number, string or boolean.
    Literal(Value),
    /// `[1 + 1, 2]` — comma-separated parts, each an expression. The values
    /// a part leaves are the list's elements, so `[1 2 3]` has three.
    List(Vec<Vec<Term>>),
    /// `(expr)` — evaluated first; its pieces then take part in the
    /// enclosing expression, so `(1 +) 2` is 3.
    Group(Vec<Term>),
    Variable(String),
    Operator(Op),
}

/// The built-in operators. Every operator is strictly unary or binary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Op {
    // Arithmetic, vectorized over arrays
    Add,
    /// `a +- b` is a + (−b).
    Sub,
    /// `a -+ b` is (−a) + b.
    SubReverse,
    Mul,
    Div,
    Mod,
    Pow,
    // Comparison
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    // Logic and negation
    Not,
    Neg,
    // Wrapped operators: boolean for two booleans, bitwise otherwise
    BitOr,
    BitAnd,
    BitXor,
    BitNot,
    // Aggregation
    Sum,
    Product,
    Count,
    CeilMax,
    FloorMin,
    // Structural
    Range,
    Concat,
    Wrap,
    Cons,
    Snoc,
    Zip,
    Piz,
    RemoveBack,
    RemoveFront,
    RemoveBoth,
    Take,
    Drop,
    IndexRight,
    IndexLeft,
    Reverse,
    GradeUp,
    GradeDown,
    /// `?` — parsed, but has no semantics yet: it always stays suspended.
    Query,
}

impl Op {
    pub const ALL: [Op; 42] = {
        use Op::*;
        [
            Add,
            Sub,
            SubReverse,
            Mul,
            Div,
            Mod,
            Pow,
            Eq,
            NotEq,
            Lt,
            Gt,
            LtEq,
            GtEq,
            Not,
            Neg,
            BitOr,
            BitAnd,
            BitXor,
            BitNot,
            Sum,
            Product,
            Count,
            CeilMax,
            FloorMin,
            Range,
            Concat,
            Wrap,
            Cons,
            Snoc,
            Zip,
            Piz,
            RemoveBack,
            RemoveFront,
            RemoveBoth,
            Take,
            Drop,
            IndexRight,
            IndexLeft,
            Reverse,
            GradeUp,
            GradeDown,
            Query,
        ]
    };

    pub fn symbol(self) -> &'static str {
        use Op::*;
        match self {
            Add => "+",
            Sub => "+-",
            SubReverse => "-+",
            Mul => "*",
            Div => "/",
            Mod => "%",
            Pow => "^",
            Eq => "==",
            NotEq => "≠",
            Lt => "<",
            Gt => ">",
            LtEq => "≤",
            GtEq => "≥",
            Not => "!",
            Neg => "-",
            BitOr => "[+]",
            BitAnd => "[*]",
            BitXor => "[-]",
            BitNot => "[~]",
            Sum => "Σ",
            Product => "Π",
            Count => "#",
            CeilMax => "⌈",
            FloorMin => "⌊",
            Range => "..",
            Concat => "><",
            Wrap => "<>",
            Cons => "+>",
            Snoc => "<+",
            Zip => "~>",
            Piz => "<~",
            RemoveBack => "->",
            RemoveFront => "<-",
            RemoveBoth => "<->",
            Take => "↑",
            Drop => "↓",
            IndexRight => "@>",
            IndexLeft => "<@",
            Reverse => "⌽",
            GradeUp => "⍋",
            GradeDown => "⍒",
            Query => "?",
        }
    }

    pub fn arity(self) -> usize {
        use Op::*;
        match self {
            Not | Neg | BitNot | Sum | Product | Count | CeilMax | FloorMin | Reverse | GradeUp
            | GradeDown => 1,
            _ => 2,
        }
    }

    /// Look up an operator by symbol, including the ASCII spellings the
    /// Crystal evaluator also accepts (`!=`, `<=`, `>=`, `sum`, `prod`, `count`).
    pub fn from_symbol(symbol: &str) -> Option<Op> {
        match symbol {
            "!=" => Some(Op::NotEq),
            "<=" => Some(Op::LtEq),
            ">=" => Some(Op::GtEq),
            "sum" => Some(Op::Sum),
            "prod" => Some(Op::Product),
            "count" => Some(Op::Count),
            _ => Op::ALL.into_iter().find(|op| op.symbol() == symbol),
        }
    }
}

impl fmt::Display for Op {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.symbol())
    }
}
