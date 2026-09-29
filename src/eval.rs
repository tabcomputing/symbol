//! The tacit evaluator (port of `tacit/eval.cr`).
//!
//! Terms are folded right to left into a single accumulator. A value meeting
//! a waiting operator becomes its next (leftward) argument; an operator
//! meeting a value takes it as its rightmost argument. Operators run as soon
//! as their argument list is complete; otherwise they stay [`Suspended`].

use std::collections::HashMap;

use crate::ast::{Expression, Op, Term};
use crate::error::{Error, Result};
use crate::ops;
use crate::value::{EvalResult, Suspended, Value};

/// Variable bindings.
pub type Bindings = HashMap<String, Value>;

/// The accumulator before any term has been seen.
const NOTHING: EvalResult = EvalResult::Resolved(Value::Nil);

/// Evaluate a parsed expression.
pub fn evaluate(expr: &Expression, bindings: &Bindings) -> Result<EvalResult> {
    Evaluator { bindings }.eval_terms(&expr.terms)
}

struct Evaluator<'a> {
    bindings: &'a Bindings,
}

impl Evaluator<'_> {
    fn eval_terms(&self, terms: &[Term]) -> Result<EvalResult> {
        terms.iter().rev().try_fold(NOTHING, |acc, term| self.apply_term(term, acc))
    }

    fn apply_term(&self, term: &Term, acc: EvalResult) -> Result<EvalResult> {
        match term {
            Term::Literal(value) => apply_value(value.clone(), acc),
            Term::List(items) => apply_value(self.eval_list(items), acc),
            Term::Group(terms) => match self.eval_terms(terms)? {
                EvalResult::Resolved(value) => apply_value(value, acc),
                // An unresolved group replaces everything to its right.
                unresolved => Ok(unresolved),
            },
            Term::Variable(name) => match self.bindings.get(name) {
                Some(value) => apply_value(value.clone(), acc),
                None => Ok(apply_unbound(name, acc)),
            },
            Term::Operator(op) => apply_operator(*op, acc),
        }
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

/// A value to the left of the accumulator.
fn apply_value(value: Value, acc: EvalResult) -> Result<EvalResult> {
    match acc {
        EvalResult::Suspended(mut pending) => {
            pending.args.insert(0, EvalResult::Resolved(value));
            if pending.is_complete() { execute(pending) } else { Ok(pending.into()) }
        }
        // Nothing combines with an unbound variable, so the value is dropped.
        EvalResult::Unbound(_) => Ok(acc),
        // Adjacent values with no operator between them: the leftmost wins.
        EvalResult::Resolved(_) => Ok(EvalResult::Resolved(value)),
    }
}

/// An unbound variable to the left of the accumulator.
fn apply_unbound(name: &str, acc: EvalResult) -> EvalResult {
    let unbound = EvalResult::Unbound(name.to_owned());
    match acc {
        // Collected as an argument, without trying to execute.
        EvalResult::Suspended(mut pending) => {
            pending.args.insert(0, unbound);
            pending.into()
        }
        _ => unbound,
    }
}

/// An operator to the left of the accumulator.
fn apply_operator(op: Op, acc: EvalResult) -> Result<EvalResult> {
    match acc {
        EvalResult::Resolved(Value::Nil) => Ok(Suspended::new(op, vec![]).into()),
        EvalResult::Resolved(_) => {
            let pending = Suspended::new(op, vec![acc]);
            if pending.is_complete() { execute(pending) } else { Ok(pending.into()) }
        }
        // A pending computation or unbound variable becomes the rightmost argument.
        other => Ok(Suspended::new(op, vec![other]).into()),
    }
}

/// Run a suspended operator. If any argument is still unresolved, it stays suspended.
fn execute(pending: Suspended) -> Result<EvalResult> {
    // Every argument is resolved before checking: an error in any nested
    // computation surfaces even when a sibling argument is unbound.
    let resolved: Vec<Option<Value>> = pending.args.iter().map(resolve_arg).collect::<Result<_>>()?;
    let Some(values) = resolved.into_iter().collect::<Option<Vec<_>>>() else {
        return Ok(pending.into());
    };
    if pending.op == Op::Query {
        return Ok(pending.into());
    }
    let mut values = values.into_iter();
    let result = match (pending.arity(), values.next(), values.next()) {
        (1, Some(a), _) => ops::unary(pending.op, a),
        (2, Some(a), Some(b)) => ops::binary(pending.op, a, b),
        // A nested computation ran short of arguments: Crystal indexes past the
        // end, though some operators convert their left operand first.
        (2, Some(a), None) => ops::convert_left(pending.op, &a).and(Err(Error::IndexOutOfBounds)),
        _ => Err(Error::IndexOutOfBounds),
    };
    result.map(EvalResult::Resolved)
}

/// The concrete value of an argument, if it has one. Nil counts as unresolved.
fn resolve_arg(arg: &EvalResult) -> Result<Option<Value>> {
    let value = match arg {
        EvalResult::Resolved(value) => value.clone(),
        EvalResult::Suspended(nested) => match execute(nested.clone())? {
            EvalResult::Resolved(value) => value,
            _ => return Ok(None),
        },
        EvalResult::Unbound(_) => return Ok(None),
    };
    Ok((!value.is_nil()).then_some(value))
}
