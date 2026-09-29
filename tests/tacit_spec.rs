//! Port of spec/tacit_spec.cr: the evaluator driven with hand-built syntax trees.

use symbol::{Bindings, EvalResult, Expression, Op, Term, Value};

fn lit(x: f64) -> Term {
    Term::Literal(Value::Float(x))
}

fn op(op: Op) -> Term {
    Term::Operator(op)
}

fn var(name: &str) -> Term {
    Term::Variable(name.to_owned())
}

fn evaluate(terms: Vec<Term>) -> EvalResult {
    evaluate_with(terms, &Bindings::new())
}

fn evaluate_with(terms: Vec<Term>, bindings: &Bindings) -> EvalResult {
    symbol::evaluate(&Expression::new(terms), bindings).unwrap()
}

fn resolved(value: impl Into<Value>) -> EvalResult {
    EvalResult::Resolved(value.into())
}

#[test]
fn evaluates_a_simple_literal() {
    assert_eq!(evaluate(vec![lit(42.0)]), resolved(42.0));
}

#[test]
fn evaluates_addition_right_to_left() {
    assert_eq!(evaluate(vec![lit(3.0), op(Op::Add), lit(5.0)]), resolved(8.0));
}

#[test]
fn evaluates_subtraction() {
    assert_eq!(evaluate(vec![lit(10.0), op(Op::Sub), lit(3.0)]), resolved(7.0));
}

#[test]
fn evaluates_multiplication() {
    assert_eq!(evaluate(vec![lit(4.0), op(Op::Mul), lit(5.0)]), resolved(20.0));
}

#[test]
fn evaluates_division() {
    assert_eq!(evaluate(vec![lit(20.0), op(Op::Div), lit(4.0)]), resolved(5.0));
}

#[test]
fn evaluates_comparison_operators() {
    assert_eq!(evaluate(vec![lit(5.0), op(Op::Gt), lit(3.0)]), resolved(true));
}

#[test]
fn evaluates_equality() {
    assert_eq!(evaluate(vec![lit(5.0), op(Op::Eq), lit(5.0)]), resolved(true));
}

#[test]
fn evaluates_logical_not() {
    assert_eq!(evaluate(vec![op(Op::Not), lit(0.0)]), resolved(true));
}

#[test]
fn resolves_variables_from_bindings() {
    let bindings = Bindings::from([("k".to_owned(), Value::Float(5.0))]);
    assert_eq!(evaluate_with(vec![var("k"), op(Op::Add), lit(1.0)], &bindings), resolved(6.0));
}

#[test]
fn returns_unbound_for_missing_variables() {
    assert_eq!(evaluate(vec![var("unknown")]), EvalResult::Unbound("unknown".into()));
}

#[test]
fn creates_suspended_computation_for_partial_application() {
    let EvalResult::Suspended(suspended) = evaluate(vec![op(Op::Add), lit(5.0)]) else {
        panic!("expected Suspended")
    };
    assert_eq!(suspended.op.symbol(), "+");
    assert_eq!(suspended.needs_args(), 1);
}
