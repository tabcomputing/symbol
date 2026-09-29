//! Native throughput: each case is evaluated end to end from its source
//! string, N times. The Crystal twin (bench.cr) runs the same cases.
//!
//!     cargo run --release --example bench -- [iterations]

use std::hint::black_box;
use std::time::Instant;

use symbol::{Bindings, Value};

const CASES: &[(&str, bool, &str)] = &[
    ("sum", false, "Σ [1, 2, 3, 4]"),
    ("constraint", false, "(x + y) > 10"),
    ("program", true, "x = 3. y = x * 2. Σ [x, y, 10]"),
    ("grade+index", false, "(⍋ [30, 10, 20, 50, 40]) @> [30, 10, 20, 50, 40]"),
    ("vector", false, "Σ [1, 2, 3, 4, 5] * [2, 3, 4, 5, 6]"),
    ("range", false, "Σ 1 .. 100"),
];

fn main() {
    let iterations: usize = std::env::args().nth(1).and_then(|a| a.parse().ok()).unwrap_or(200_000);
    let bindings = Bindings::from([("x".to_owned(), Value::Int(3)), ("y".to_owned(), Value::Int(9))]);
    for &(name, program, source) in CASES {
        let start = Instant::now();
        for _ in 0..iterations {
            let result = if program {
                symbol::eval_program(black_box(source), &mut Bindings::new())
            } else {
                symbol::eval(black_box(source), &bindings)
            };
            black_box(result.unwrap());
        }
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        let ns_per_eval = ms * 1e6 / iterations as f64;
        println!("{name:<12} {iterations} evals  {ms:8.1} ms  {ns_per_eval:7.0} ns/eval");
    }
}
