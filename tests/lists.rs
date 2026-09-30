//! List elements are expressions: commas separate the parts, and the values
//! a part leaves are the list's elements.

use symbol::{Bindings, Error, EvalResult, Op, Value, eval};

fn run(expr: &str) -> Result<EvalResult, Error> {
    let mut bindings = Bindings::new();
    bindings.insert("x".into(), Value::Int(3));
    bindings.insert("f".into(), Value::Bool(false));
    bindings.insert("xs".into(), Value::from(vec![1i64, 2, 3]));
    eval(expr, &bindings)
}

fn list(expr: &str) -> Value {
    match run(expr) {
        Ok(EvalResult::Resolved(value)) => value,
        other => panic!("`{expr}` gave {other:?}"),
    }
}

fn ints(items: &[i64]) -> Value {
    Value::from(items.to_vec())
}

#[test]
fn elements_can_be_expressions() {
    assert_eq!(list("[1 + 1, 2 * 3]"), ints(&[2, 6]));
    assert_eq!(list("[xs Σ, x - 1, -1]"), ints(&[6, 2, -1]));
    assert_eq!(list("[(1 +) 2]"), ints(&[3]));
    assert_eq!(run("Σ [1 + 1, 2]").unwrap(), EvalResult::Resolved(Value::Int(4)));
}

#[test]
fn a_part_with_several_values_gives_them_all() {
    assert_eq!(list("[1 2 3]"), ints(&[1, 2, 3]));
    assert_eq!(list("[1, 2, 3]"), ints(&[1, 2, 3]));
    assert_eq!(list("[1 2, 3 4]"), ints(&[1, 2, 3, 4]));
    // `+` takes the two values to its right, as anywhere else
    assert_eq!(list("[1 + 1 2]"), ints(&[1, 3]));
}

#[test]
fn nesting_and_empty_parts() {
    assert_eq!(list("[[1 2] 3]"), Value::Array(vec![ints(&[1, 2]), Value::Int(3)]));
    assert_eq!(list("[1, 2,]"), ints(&[1, 2]));
    assert_eq!(list("[]"), ints(&[]));
}

#[test]
fn values_keep_their_type_and_unknowns_are_nil() {
    assert_eq!(list("[f]"), Value::Array(vec![Value::Bool(false)]));
    assert_eq!(list("[u, u + 1]"), Value::Array(vec![Value::Nil, Value::Nil]));
}

#[test]
fn an_element_waiting_for_an_argument_is_an_error() {
    assert_eq!(run("[1 +]"), Err(Error::IncompleteElement(Op::Add)));
    assert_eq!(run("[1 +]").unwrap_err().to_string(), "A list element is incomplete: + needs an argument");
    assert!(matches!(run("[(1, 2)]"), Err(Error::Parse { .. })));
}
