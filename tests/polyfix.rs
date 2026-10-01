//! Poly-fix evaluation: an operator can stand before, between or after its
//! arguments, and whatever doesn't combine is a partial application.

use symbol::{Bindings, EvalResult, Op, Suspended, Value, eval};

fn run(expr: &str) -> EvalResult {
    let mut bindings = Bindings::new();
    bindings.insert("xs".into(), Value::from(vec![1i64, 2, 3]));
    eval(expr, &bindings).unwrap_or_else(|error| panic!("`{expr}` failed: {error}"))
}

fn int(i: i64) -> EvalResult {
    EvalResult::Resolved(Value::Int(i))
}

fn waiting(op: Op, args: Vec<EvalResult>) -> EvalResult {
    Suspended::new(op, args).into()
}

fn unbound(name: &str) -> EvalResult {
    EvalResult::Unbound(name.into())
}

#[test]
fn an_operator_can_stand_anywhere() {
    for expr in ["1 + 2", "+ 1 2", "1 2 +"] {
        assert_eq!(run(expr), int(3), "{expr}");
    }
}

#[test]
fn arguments_keep_their_textual_order() {
    for expr in ["5 +- 1", "+- 5 1", "5 1 +-", "(5 +-) 1", "(+- 5) 1", "5 (+- 1)", "(5 1) +-", "+- (5 1)"] {
        assert_eq!(run(expr), int(4), "{expr}");
    }
}

#[test]
fn right_to_left_without_precedence() {
    assert_eq!(run("2 * 3 + 4"), int(14));
    assert_eq!(run("1 + 2 * 3"), int(7));
    assert_eq!(run("(2 * 3) + 4"), int(10));
    assert_eq!(run("Σ xs + 1"), int(9));
}

#[test]
fn a_value_goes_to_the_waiting_operator_on_its_right_first() {
    assert_eq!(run("2 3 4 + *"), int(14));
    assert_eq!(run("1 2 + + 3"), int(6));
    assert_eq!(run("2 Σ + 1"), int(3));
    assert_eq!(run("+ 1 2 3 *"), int(7));
}

#[test]
fn an_operator_takes_the_values_to_its_right_up_to_its_arity() {
    assert_eq!(run("* 2 3 + 4"), int(14));
    assert_eq!(run("- 1 2"), EvalResult::Sequence(vec![int(-1), int(2)]));
}

#[test]
fn a_missing_argument_leaves_a_waiting_operator() {
    assert_eq!(run("+ 5"), waiting(Op::Add, vec![int(5)]));
    assert_eq!(run("5 +"), waiting(Op::Add, vec![int(5)]));
    assert_eq!(run("Σ"), waiting(Op::Sum, vec![]));
}

#[test]
fn values_without_an_operator_are_a_sequence() {
    assert_eq!(run("1 2"), EvalResult::Sequence(vec![int(1), int(2)]));
    assert_eq!(run("1 + 2 3"), EvalResult::Sequence(vec![int(1), int(5)]));
    assert_eq!(run("1 u"), EvalResult::Sequence(vec![int(1), unbound("u")]));
}

#[test]
fn several_waiting_operators_are_a_sequence() {
    assert_eq!(
        run("1 + + 2"),
        EvalResult::Sequence(vec![waiting(Op::Add, vec![int(1)]), waiting(Op::Add, vec![int(2)])])
    );
}

#[test]
fn a_group_takes_part_in_the_enclosing_expression() {
    assert_eq!(run("(1 +) 2"), int(3));
    assert_eq!(run("(1 2) +"), int(3));
    assert_eq!(run("(+) 1 2"), int(3));
    assert_eq!(run("(1 + + 2) 3"), int(6));
    assert_eq!(run("(2 * (3 +)) 4"), int(14));
    assert_eq!(run("1 (2) 3"), EvalResult::Sequence(vec![int(1), int(2), int(3)]));
}

#[test]
fn an_empty_group_is_nothing() {
    assert_eq!(run("()"), EvalResult::Resolved(Value::Nil));
    assert_eq!(run("() + 1"), waiting(Op::Add, vec![int(1)]));
}

#[test]
fn an_unknown_argument_suspends_the_computation_but_keeps_its_place() {
    let x_plus_1 = waiting(Op::Add, vec![unbound("x"), int(1)]);
    assert_eq!(run("x + 1"), x_plus_1);
    assert_eq!(run("(x + 1) * 2"), waiting(Op::Mul, vec![x_plus_1.clone(), int(2)]));
    assert_eq!(run("2 * x + 1"), waiting(Op::Mul, vec![int(2), x_plus_1]));
    assert_eq!(
        run("1 + (u) + 2"),
        waiting(Op::Add, vec![int(1), waiting(Op::Add, vec![unbound("u"), int(2)])])
    );
}

#[test]
fn a_sequence_displays_its_pieces() {
    assert_eq!(run("1 u").to_string(), "1 u");
}
