//! Golden-file tests.
//!
//! `tests/fixtures/*.txt` are corpora, and `*.expected` hold their canonical
//! output (see `support` and README.md). Until the poly-fix evaluator
//! (2026-09-30) that output was the Crystal implementation's; it is now this
//! implementation's own, reviewed. After a deliberate change, review the
//! failures and rewrite the files with `BLESS=1 cargo test --test differential`.

mod support;

use std::fs;
use std::path::Path;

fn check(corpus: &str) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let lines = fs::read_to_string(dir.join(format!("{corpus}.txt"))).unwrap();
    let expected = fs::read_to_string(dir.join(format!("{corpus}.expected"))).unwrap();
    let lines: Vec<&str> = lines.lines().filter(|l| !l.is_empty()).collect();
    if std::env::var_os("BLESS").is_some() {
        let output: String = lines.iter().map(|line| support::run_line(line) + "\n").collect();
        fs::write(dir.join(format!("{corpus}.expected")), output).unwrap();
        return;
    }
    let expected: Vec<&str> = expected.lines().collect();
    assert_eq!(lines.len(), expected.len(), "corpus and expected output differ in length");

    let mismatches: Vec<String> = lines
        .iter()
        .zip(&expected)
        .filter_map(|(line, want)| {
            let got = support::run_line(line);
            (got != *want).then(|| format!("  input:    {line}\n  expected: {want}\n  got:      {got}"))
        })
        .collect();
    assert!(
        mismatches.is_empty(),
        "{} of {} lines differ from the expected output:\n{}",
        mismatches.len(),
        lines.len(),
        mismatches.join("\n")
    );
}

#[test]
fn handwritten_edge_cases() {
    check("handwritten");
}

#[test]
fn random_expressions() {
    check("random");
}
