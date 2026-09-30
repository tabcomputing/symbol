//! `-` negates and is never subtraction: `a +- b` is a + (−b), and `a -+ b`
//! is (−a) + b. That leaves `-` free inside names (`data-id`, `x-1`).

use symbol::{Bindings, Error, EvalResult, Value, eval};

fn run(expr: &str) -> Result<EvalResult, Error> {
    let mut bindings = Bindings::new();
    bindings.insert("x".into(), Value::Int(3));
    bindings.insert("y".into(), Value::Int(10));
    bindings.insert("data-id".into(), Value::Int(7));
    bindings.insert("x-1".into(), Value::Int(100));
    eval(expr, &bindings)
}

fn value(expr: &str) -> EvalResult {
    run(expr).unwrap_or_else(|error| panic!("`{expr}` failed: {error}"))
}

fn int(i: i64) -> EvalResult {
    EvalResult::Resolved(Value::Int(i))
}

#[test]
fn plus_minus_subtracts_and_minus_plus_subtracts_the_other_way() {
    for (expr, want) in
        [("4+-1", 3), ("4 +- 1", 3), ("4-+1", -3), ("4 -+ 1", -3), ("+- 4 1", 3), ("4 1 -+", -3)]
    {
        assert_eq!(value(expr), int(want), "{expr}");
    }
    assert_eq!(value("[5, 6] +- 1"), EvalResult::Resolved(Value::from(vec![4i64, 5])));
    assert_eq!(value("[5, 6] -+ 10"), EvalResult::Resolved(Value::from(vec![5i64, 4])));
}

#[test]
fn minus_negates() {
    for (expr, want) in
        [("-x", -3), ("- x", -3), ("x -", -3), ("--x", 3), ("-(x + 1)", -4), ("-3", -3), ("- -3", 3)]
    {
        assert_eq!(value(expr), int(want), "{expr}");
    }
    assert_eq!(value("-[1, 2]"), EvalResult::Resolved(Value::from(vec![-1i64, -2])));
}

#[test]
fn negation_combines_with_subtraction() {
    assert_eq!(value("x+--y"), int(13)); // x +- (-y)
    assert_eq!(value("x-+-y"), int(-13)); // (-x) + (-y)
    assert_eq!(value("x + -y"), int(-7));
}

#[test]
fn names_can_contain_minus() {
    assert_eq!(value("data-id + 1"), int(8));
    assert_eq!(value("x-1"), int(100));
    assert_eq!(value("x -1"), EvalResult::Sequence(vec![int(3), int(-1)]));
}

#[test]
fn minus_right_after_a_value_is_an_error() {
    for expr in ["3-3", "x--y", "(5)-1", "[5, 6]-1", "\"a\"-1"] {
        let error = run(expr).expect_err(expr).to_string();
        assert!(error.contains("Unexpected - right after a value (subtract with +-)"), "{expr}: {error}");
    }
}

#[test]
fn tilde_is_not_an_operator() {
    assert!(matches!(run("~ 5"), Err(Error::Parse { .. })));
}
