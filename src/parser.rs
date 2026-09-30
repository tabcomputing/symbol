//! Parser (port of `parser.cr`). Expressions are flat: operators have no
//! precedence, so the parser only builds literals, lists and groups.

use crate::ast::{Expression, Op, Term};
use crate::compat;
use crate::error::{Error, Result};
use crate::lexer::{Token, TokenKind as K};
use crate::value::Value;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    /// `tokens` must end with an `Eof` token, as [`crate::Lexer::tokenize`] produces.
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    pub fn parse(mut self) -> Result<Expression> {
        let mut terms = Vec::new();
        while !self.at_end() {
            terms.push(self.parse_term()?);
        }
        Ok(Expression::new(terms))
    }

    fn parse_term(&mut self) -> Result<Term> {
        let token = self.current();
        if let Some(op) = operator(token.kind) {
            self.advance();
            return Ok(Term::Operator(op));
        }
        match token.kind {
            K::Number | K::String | K::True | K::False => {
                let value = literal(token)?;
                self.advance();
                Ok(Term::Literal(value))
            }
            K::Identifier => {
                let name = token.value.clone();
                self.advance();
                Ok(Term::Variable(name))
            }
            K::LParen => self.parse_group(),
            K::LBracket => self.parse_list(),
            K::Assign => Err(error(token, "Assignment '=' not allowed in expression context")),
            K::Period => Err(error(token, "Unexpected '.'")),
            K::Eof => Err(error(token, "Unexpected end of input")),
            // The lexer's own message: "Unexpected character: &" and the like.
            K::Error => Err(error(token, token.value.clone())),
            kind => Err(error(token, format!("Unexpected token: {kind}"))),
        }
    }

    fn parse_group(&mut self) -> Result<Term> {
        self.advance(); // (
        let mut terms = Vec::new();
        while !self.check(K::RParen) && !self.at_end() {
            terms.push(self.parse_term()?);
        }
        self.expect(K::RParen)?;
        Ok(Term::Group(terms))
    }

    /// `[a, b, ...]`: comma-separated parts, each an expression. Empty
    /// parts (`[1, 2,]`) are dropped.
    fn parse_list(&mut self) -> Result<Term> {
        self.advance(); // [
        let mut parts = vec![Vec::new()];
        while !self.check(K::RBracket) && !self.at_end() {
            if self.check(K::Comma) {
                self.advance();
                parts.push(Vec::new());
            } else {
                let term = self.parse_term()?;
                parts.last_mut().expect("there is always a part").push(term);
            }
        }
        self.expect(K::RBracket)?;
        parts.retain(|part| !part.is_empty());
        Ok(Term::List(parts))
    }

    fn current(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn at_end(&self) -> bool {
        self.check(K::Eof)
    }

    fn check(&self, kind: K) -> bool {
        self.current().kind == kind
    }

    fn advance(&mut self) {
        if !self.at_end() {
            self.pos += 1;
        }
    }

    fn expect(&mut self, kind: K) -> Result<()> {
        if !self.check(kind) {
            let token = self.current();
            return Err(error(token, format!("Expected {kind}, got {}", token.kind)));
        }
        self.advance();
        Ok(())
    }
}

fn error(token: &Token, message: impl Into<String>) -> Error {
    Error::Parse { message: message.into(), line: token.line, column: token.column }
}

fn literal(token: &Token) -> Result<Value> {
    let text = &token.value;
    let invalid = |type_name| Error::InvalidNumber { type_name, text: text.clone() };
    Ok(match token.kind {
        K::Number if text.contains('.') => {
            Value::Float(compat::parse_float(text).ok_or_else(|| invalid("Float64"))?)
        }
        K::Number => Value::Int(text.parse().map_err(|_| invalid("Int64"))?),
        K::True => Value::Bool(true),
        K::False => Value::Bool(false),
        _ => Value::Str(text.clone()),
    })
}

/// The operator a token denotes, if any.
fn operator(kind: K) -> Option<Op> {
    Some(match kind {
        K::Plus => Op::Add,
        K::PlusMinus => Op::Sub,
        K::MinusPlus => Op::SubReverse,
        K::Minus => Op::Neg,
        K::Star => Op::Mul,
        K::Slash => Op::Div,
        K::Percent => Op::Mod,
        K::Caret => Op::Pow,
        K::Range => Op::Range,
        K::Equals => Op::Eq,
        K::NotEq | K::NotEqual => Op::NotEq,
        K::LessThan => Op::Lt,
        K::GreaterThan => Op::Gt,
        K::LessEq | K::LessEqual => Op::LtEq,
        K::GreaterEq | K::GreaterEqual => Op::GtEq,
        K::Bang => Op::Not,
        K::Question => Op::Query,
        K::IndexRight => Op::IndexRight,
        K::IndexLeft => Op::IndexLeft,
        K::Hash => Op::Count,
        K::Sum => Op::Sum,
        K::Product => Op::Product,
        K::CeilMax => Op::CeilMax,
        K::FloorMin => Op::FloorMin,
        K::BitOr => Op::BitOr,
        K::BitAnd => Op::BitAnd,
        K::BitXor => Op::BitXor,
        K::BitNot => Op::BitNot,
        K::Concat => Op::Concat,
        K::Wrap => Op::Wrap,
        K::Cons => Op::Cons,
        K::Snoc => Op::Snoc,
        K::Zip => Op::Zip,
        K::Piz => Op::Piz,
        K::RemoveBack => Op::RemoveBack,
        K::RemoveFront => Op::RemoveFront,
        K::RemoveBoth => Op::RemoveBoth,
        K::Take => Op::Take,
        K::Drop => Op::Drop,
        K::Reverse => Op::Reverse,
        K::GradeUp => Op::GradeUp,
        K::GradeDown => Op::GradeDown,
        _ => return None,
    })
}
