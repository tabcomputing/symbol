//! The evaluator. The rules are in the crate docs.
//!
//! Terms are read right to left onto a stack of pieces. A finished piece (a
//! value, an unbound variable, or an operator that has all its arguments but
//! cannot run) joins the waiting operator to its right. An operator takes the
//! finished pieces to its right. A group's pieces are pushed in turn, as if
//! they were terms. What is left at the end is the result.

use std::collections::HashMap;

use crate::ast::{Expression, Op, Term};
use crate::error::Result;
use crate::ops;
use crate::value::{EvalResult, Suspended, Value};

/// Variable bindings.
pub type Bindings = HashMap<String, Value>;

/// Evaluate a parsed expression.
pub fn evaluate(expr: &Expression, bindings: &Bindings) -> Result<EvalResult> {
    let mut pieces: Vec<EvalResult> =
        Evaluator { bindings }.stack(&expr.terms)?.into_iter().rev().map(Piece::into_result).collect();
    Ok(match pieces.len() {
        0 => EvalResult::Resolved(Value::Nil),
        1 => pieces.remove(0),
        _ => EvalResult::Sequence(pieces),
    })
}

/// A piece on the stack.
enum Piece {
    /// A value, an unbound variable, or an operator that has all its
    /// arguments but cannot run, because one of them is unknown.
    Finished(EvalResult),
    /// An operator still waiting for arguments.
    Waiting(Suspended),
}

impl Piece {
    fn into_result(self) -> EvalResult {
        match self {
            Piece::Finished(result) => result,
            Piece::Waiting(op) => op.into(),
        }
    }
}

/// The pieces of an expression, rightmost first. Finished pieces are always
/// to the right of waiting operators, since a finished piece joins the
/// waiting operator to its right.
type Stack = Vec<Piece>;

struct Evaluator<'a> {
    bindings: &'a Bindings,
}

impl Evaluator<'_> {
    fn stack(&self, terms: &[Term]) -> Result<Stack> {
        let mut stack = Stack::new();
        for term in terms.iter().rev() {
            match term {
                Term::Literal(value) => push_finished(&mut stack, EvalResult::Resolved(value.clone()))?,
                Term::List(items) => push_finished(&mut stack, EvalResult::Resolved(self.eval_list(items)))?,
                Term::Variable(name) => {
                    let piece = match self.bindings.get(name) {
                        Some(value) => EvalResult::Resolved(value.clone()),
                        None => EvalResult::Unbound(name.clone()),
                    };
                    push_finished(&mut stack, piece)?;
                }
                Term::Operator(op) => push_waiting(&mut stack, Suspended::new(*op, vec![]))?,
                Term::Group(terms) => {
                    for piece in self.stack(terms)? {
                        match piece {
                            Piece::Finished(result) => push_finished(&mut stack, result)?,
                            Piece::Waiting(op) => push_waiting(&mut stack, op)?,
                        }
                    }
                }
            }
        }
        Ok(stack)
    }

    fn eval_list(&self, items: &[Term]) -> Value {
        let values = items.iter().map(|item| match item {
            Term::Literal(value) => value.clone(),
            // Crystal's `bindings[name]? || nil` also turns a bound `false` into nil.
            Term::Variable(name) => match self.bindings.get(name) {
                None | Some(Value::Bool(false)) => Value::Nil,
                Some(value) => value.clone(),
            },
            Term::List(nested) => self.eval_list(nested),
            _ => Value::Nil,
        });
        Value::Array(values.collect())
    }
}

/// A finished piece joins the waiting operator to its right as its leftmost
/// argument. If that completes the operator, its result does the same.
fn push_finished(stack: &mut Stack, mut piece: EvalResult) -> Result<()> {
    loop {
        match stack.pop() {
            Some(Piece::Waiting(mut op)) => {
                op.args.insert(0, piece);
                if !op.is_complete() {
                    stack.push(Piece::Waiting(op));
                    return Ok(());
                }
                piece = execute(op)?;
            }
            top => {
                stack.extend(top);
                stack.push(Piece::Finished(piece));
                return Ok(());
            }
        }
    }
}

/// An operator takes the finished pieces to its right, as many as it still
/// needs, and waits for the rest.
fn push_waiting(stack: &mut Stack, mut op: Suspended) -> Result<()> {
    while !op.is_complete() {
        match stack.pop() {
            Some(Piece::Finished(arg)) => op.args.push(arg),
            top => {
                stack.extend(top);
                break;
            }
        }
    }
    if op.is_complete() {
        push_finished(stack, execute(op)?)
    } else {
        stack.push(Piece::Waiting(op));
        Ok(())
    }
}

/// Run an operator that has all its arguments. It stays suspended if one of
/// them is unknown: an unbound variable, nil, or a computation that could
/// not run.
fn execute(pending: Suspended) -> Result<EvalResult> {
    let known = |arg: &EvalResult| matches!(arg, EvalResult::Resolved(value) if !value.is_nil());
    if pending.op == Op::Query || !pending.args.iter().all(known) {
        return Ok(pending.into());
    }
    let op = pending.op;
    let mut values = pending.args.into_iter().map(|arg| match arg {
        EvalResult::Resolved(value) => value,
        _ => unreachable!("every argument is known"),
    });
    let first = values.next().expect("an operator takes at least one argument");
    let result = match values.next() {
        None => ops::unary(op, first),
        Some(second) => ops::binary(op, first, second),
    };
    result.map(EvalResult::Resolved)
}
