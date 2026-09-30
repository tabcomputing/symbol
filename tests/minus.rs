//! A `-` before a digit is a sign, unless it comes right after a value
//! (Julia's rule): `3-3` and `x-1` subtract, `3 -3` is two values.

use symbol::{Bindings, EvalResult, Value, eval};

fn run(expr: &str) -> EvalResult {
    let mut bindings = Bindings::new();
    bindings.insert("x".into(), Value::Int(3));
    eval(expr, &bindings).unwrap_or_else(|error| panic!("`{expr}` failed: {error}"))
}

fn int(i: i64) -> EvalResult {
    EvalResult::Resolved(Value::Int(i))
}

#[test]
fn right_after_a_value_it_subtracts() {
    for (expr, want) in [("3-3", 0), ("x-1", 2), ("(5)-1", 4), ("-1-1", -2), ("1--1", 2)] {
        assert_eq!(run(expr), int(want), "{expr}");
    }
    assert_eq!(run("[5, 6]-1"), EvalResult::Resolved(Value::from(vec![4i64, 5])));
    assert_eq!(run("⊤-1"), EvalResult::Resolved(Value::Float(0.0)));
}

#[test]
fn with_spaces_on_both_sides_it_subtracts() {
    assert_eq!(run("3 - 3"), int(0));
    assert_eq!(run("x - 1"), int(2));
}

#[test]
fn elsewhere_it_is_a_sign() {
    assert_eq!(run("-3"), int(-3));
    assert_eq!(run("(-3)"), int(-3));
    assert_eq!(run("+ -3 5"), int(2));
    assert_eq!(run("3 -3"), EvalResult::Sequence(vec![int(3), int(-3)]));
    assert_eq!(run("[1 -1]"), EvalResult::Resolved(Value::from(vec![1i64, -1])));
    assert_eq!(run("1..-2"), EvalResult::Resolved(Value::from(vec![1i64, 0, -1, -2])));
}
