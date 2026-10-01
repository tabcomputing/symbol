//! The examples in README.md hold. Each line of a `symbol` block (an
//! expression) or a `symbol-program` block (a program) is `source ⇒ result`,
//! where the result is the value as `Value::inspect` writes it,
//! `error: message`, or `partial…` for a result that isn't finished. The
//! README's Rust blocks run as doctests (see src/lib.rs).
//!
//! The blocks are fenced as ```` ```apl symbol ````: GitHub highlights them
//! by the first word, with APL's highlighter (the closest relative), and this
//! test finds them by the second.

use std::fs;
use std::path::Path;

use symbol::{Bindings, EvalResult, Value};

/// The variables the README's examples use.
fn bindings() -> Bindings {
    let mut bindings = Bindings::new();
    bindings.insert("xs".into(), Value::from(vec![3i64, 1, 4, 1, 5]));
    bindings.insert("price".into(), Value::Int(20));
    bindings.insert("qty".into(), Value::Int(3));
    bindings
}

/// The lines of the fenced blocks whose info string has the word `info`.
fn block_lines(readme: &str, info: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut inside = false;
    for line in readme.lines() {
        let fence = line.trim_start().strip_prefix("```");
        match (inside, fence) {
            (false, Some(rest)) if rest.split_whitespace().any(|word| word == info) => inside = true,
            (true, Some(_)) => inside = false,
            (true, None) if !line.trim().is_empty() => lines.push(line.to_owned()),
            _ => {}
        }
    }
    lines
}

fn render(result: symbol::Result<EvalResult>) -> String {
    match result {
        Ok(EvalResult::Resolved(value)) => value.inspect(),
        Ok(_) => "partial".to_owned(),
        Err(error) => format!("error: {error}"),
    }
}

fn check(info: &str, run: fn(&str) -> symbol::Result<EvalResult>) -> usize {
    let readme = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md")).unwrap();
    let lines = block_lines(&readme, info);
    let mut wrong = Vec::new();
    for line in &lines {
        let (source, want) = line.split_once('⇒').unwrap_or_else(|| panic!("no ⇒ in `{line}`"));
        let (source, want) = (source.trim(), want.trim());
        let got = render(run(source));
        let ok = if want.starts_with("partial") { got == "partial" } else { got == want };
        if !ok {
            wrong.push(format!("  {source}\n    README: {want}\n    actual: {got}"));
        }
    }
    assert!(wrong.is_empty(), "README examples that don't hold:\n{}", wrong.join("\n"));
    lines.len()
}

#[test]
fn expression_examples_hold() {
    let count = check("symbol", |source| symbol::eval(source, &bindings()));
    assert!(count >= 50, "only {count} expression examples found");
}

#[test]
fn program_examples_hold() {
    let count = check("symbol-program", |source| symbol::eval_program(source, &mut bindings()));
    assert!(count >= 2, "only {count} program examples found");
}
