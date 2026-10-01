//! A result prints as SYMBOL source: a value as SYMBOL writes it, and a
//! partial as the expression it is, which reads back as the same partial.

mod support;

use std::fs;
use std::path::Path;

use symbol::{Bindings, EvalResult, Value, eval};

fn printed(source: &str) -> String {
    eval(source, &Bindings::new()).unwrap().to_string()
}

#[test]
fn values_print_as_symbol_writes_them() {
    for (source, want) in [
        ("1 + 2", "3"),
        ("7 / 2", "3.5"),
        ("[1 2] <> [3]", "[[1 2] [3]]"),
        (r#""say \"hi\"""#, r#""say \"hi\"""#),
        (r#""a#{b}""#, r#""a#{b}""#),
        ("0 @> [1]", "nil"),
    ] {
        assert_eq!(printed(source), want, "{source}");
    }
}

#[test]
fn partials_print_as_the_expression_they_are() {
    for (source, want) in [
        ("1 +", "1 +"),
        ("+ 1", "1 +"),
        ("Σ", "Σ"),
        ("x + 1", "x + 1"),
        ("2 * x + 1", "2 * (x + 1)"),
        ("(x + 1) * 2", "(x + 1) * 2"),
        ("Σ x", "Σ x"),
        ("x +- -3", "x +- -3"),
        ("1 2", "1 2"),
        ("1 + + 2", "(1 +) (2 +)"),
        ("x", "x"),
    ] {
        assert_eq!(printed(source), want, "{source}");
    }
}

#[test]
fn a_waiting_operator_takes_its_missing_argument_from_either_side() {
    // `1 +-` and `+- 1` are the same partial, which is why one form prints
    for (source, want) in [("(5 +-) 1", 4), ("(+- 5) 1", 4), ("1 (5 +-)", -4), ("1 (+- 5)", -4)] {
        assert_eq!(
            eval(source, &Bindings::new()).unwrap(),
            EvalResult::Resolved(Value::Int(want)),
            "{source}"
        );
    }
}

/// A value SYMBOL has no literal for, in printed source.
fn has_no_literal(printed: &str) -> bool {
    printed.split(|c: char| !c.is_alphanumeric() && !matches!(c, '.' | '+' | '-')).any(|word| {
        let exponent = (word.contains("e+") || word.contains("e-"))
            && word.starts_with(|c: char| c.is_ascii_digit() || c == '-');
        matches!(word, "nil" | "NaN" | "Infinity") || exponent
    })
}

/// Every partial result of the golden corpora reads back as itself, unless
/// it holds a value SYMBOL has no literal for (nil, NaN, the infinities, a
/// float written with an exponent).
#[test]
fn every_partial_in_the_corpora_reads_back_as_itself() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let (mut checked, mut wrong) = (0, Vec::new());
    for corpus in ["handwritten", "random"] {
        for line in fs::read_to_string(dir.join(format!("{corpus}.txt"))).unwrap().lines() {
            let Some(raw) = line.strip_prefix("E\t") else { continue };
            let bindings = support::base_bindings();
            let Ok(result) = eval(&support::unescape(raw), &bindings) else { continue };
            let text = result.to_string();
            if matches!(result, EvalResult::Resolved(_)) || has_no_literal(&text) {
                continue;
            }
            checked += 1;
            match eval(&text, &bindings) {
                Ok(back) if back == result => {}
                other => wrong.push(format!("  {raw}\n    printed: {text}\n    reads back: {other:?}")),
            }
        }
    }
    assert!(checked > 1000, "only {checked} partials checked");
    assert!(wrong.is_empty(), "{} partials don't read back:\n{}", wrong.len(), wrong.join("\n"));
}
