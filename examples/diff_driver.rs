//! Print the canonical rendering of every line of a corpus file, for
//! comparison with the Crystal driver's output (see tests/support/mod.rs).
//!
//!     cargo run --release --example diff_driver -- corpus.txt > rust.out

#[path = "../tests/support/mod.rs"]
mod support;

use std::io::{BufWriter, Write};

fn main() {
    let path = std::env::args().nth(1).expect("usage: diff_driver <corpus.txt>");
    let corpus = std::fs::read_to_string(path).expect("readable corpus");
    let mut out = BufWriter::new(std::io::stdout().lock());
    for line in corpus.lines().filter(|l| !l.is_empty()) {
        writeln!(out, "{}", support::run_line(line)).unwrap();
    }
}
