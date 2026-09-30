//! Operators typed by their LaTeX names (`\sum` for `Σ`): the same tokens as
//! the symbols, so the same results.

use symbol::{Bindings, EvalResult, Lexer, TokenKind, Value, eval};

fn run(expr: &str) -> String {
    let mut bindings = Bindings::new();
    bindings.insert("values".into(), Value::Array(vec![Value::Int(3), Value::Int(1), Value::Int(2)]));
    bindings.insert("sum".into(), Value::Int(40));
    match eval(expr, &bindings) {
        Ok(result) => format!("{result:?}"),
        Err(error) => format!("error: {error}"),
    }
}

#[test]
fn every_name_means_its_symbol() {
    for (named, symbol) in [
        (r"\sum values", "Σ values"),
        (r"\prod values", "Π values"),
        (r"\count values", "# values"),
        (r"\max values", "⌈ values"),
        (r"\min values", "⌊ values"),
        (r"\lceil 2.5", "⌈ 2.5"),
        (r"\lfloor 2.5", "⌊ 2.5"),
        (r"1 \neq 2", "1 ≠ 2"),
        (r"1 \ne 2", "1 ≠ 2"),
        (r"1 \leq 2", "1 ≤ 2"),
        (r"1 \le 2", "1 ≤ 2"),
        (r"1 \geq 2", "1 ≥ 2"),
        (r"1 \ge 2", "1 ≥ 2"),
        (r"\top", "⊤"),
        (r"\bot", "⊥"),
        (r"2 \uparrow values", "2 ↑ values"),
        (r"2 \downarrow values", "2 ↓ values"),
        (r"\reverse values", "⌽ values"),
        (r"\gradeup values", "⍋ values"),
        (r"\gradedown values", "⍒ values"),
    ] {
        assert_eq!(run(named), run(symbol), "{named}");
        assert!(!run(named).starts_with("error"), "{named}: {}", run(named));
    }
}

#[test]
fn a_name_works_in_postfix_position_too() {
    assert_eq!(run(r"values \sum"), run(r"\sum values"));
    assert_eq!(run(r"values \sum"), format!("{:?}", EvalResult::Resolved(Value::Int(6))));
}

#[test]
fn the_bare_word_is_just_a_variable() {
    // `sum` is bound to 40 here; only `\sum` is the operator
    assert_eq!(run("sum + 2"), format!("{:?}", EvalResult::Resolved(Value::Int(42))));
    assert_eq!(run(r"\sum values + sum"), run("Σ values + sum"));
}

#[test]
fn an_unknown_name_is_an_error() {
    assert!(run(r"\summ values").starts_with("error"), "{}", run(r"\summ values"));
    assert!(run(r"\ values").starts_with("error"), "{}", run(r"\ values"));
    // The parser reports lexer errors generically (as the Crystal one does);
    // the token carries the message
    let token = &Lexer::new(r"\summ").tokenize()[0];
    assert_eq!((token.kind, token.value.as_str()), (TokenKind::Error, r"Unknown operator name: \summ"));
}
