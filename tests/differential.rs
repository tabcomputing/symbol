//! Golden-file comparison with the Crystal implementation.
//!
//! `tests/fixtures/*.txt` are corpora; `*.expected` hold the Crystal
//! implementation's canonical output for them (see `support` and README.md).

mod support;

use std::fs;
use std::path::Path;

fn check(corpus: &str) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let lines = fs::read_to_string(dir.join(format!("{corpus}.txt"))).unwrap();
    let expected = fs::read_to_string(dir.join(format!("{corpus}.expected"))).unwrap();
    let lines: Vec<&str> = lines.lines().filter(|l| !l.is_empty()).collect();
    let expected: Vec<&str> = expected.lines().collect();
    assert_eq!(lines.len(), expected.len(), "corpus and expected output differ in length");

    let mismatches: Vec<String> = lines
        .iter()
        .zip(&expected)
        .filter_map(|(line, want)| {
            let got = support::run_line(line);
            (got != *want).then(|| format!("  input:  {line}\n  crystal: {want}\n  rust:    {got}"))
        })
        .collect();
    assert!(
        mismatches.is_empty(),
        "{} of {} lines differ from Crystal:\n{}",
        mismatches.len(),
        lines.len(),
        mismatches.join("\n")
    );
}

#[test]
fn matches_crystal_on_handwritten_edge_cases() {
    check("handwritten");
}

#[test]
fn matches_crystal_on_random_expressions() {
    check("random");
}
