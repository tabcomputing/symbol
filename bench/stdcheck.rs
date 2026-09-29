use std::cmp::Ordering;
fn main() {
    let s: f64 = [-0.0f64].iter().sum();
    println!("sum of [-0.0] = {s:?}; empty sum = {:?}", std::iter::empty::<f64>().sum::<f64>());
    println!("'\\u{{b}}'.is_ascii_whitespace() = {}, '\\u{{85}}'.is_whitespace() = {}", '\u{b}'.is_ascii_whitespace(), '\u{85}'.is_whitespace());
    // Grade-style comparator: NaN compares Equal to everything.
    let mut seed = 1u64;
    let mut panics = 0;
    for trial in 0..2000 {
        let len = 2 + trial % 60;
        let keys: Vec<f64> = (0..len).map(|_| { seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); if (seed >> 60) < 4 { f64::NAN } else { (seed >> 40) as f64 } }).collect();
        let r = std::panic::catch_unwind(|| {
            let mut idx: Vec<usize> = (0..keys.len()).collect();
            idx.sort_by(|&i, &j| keys[i].partial_cmp(&keys[j]).unwrap_or(Ordering::Equal));
        });
        if r.is_err() { panics += 1; }
    }
    println!("slice::sort_by panicked in {panics} of 2000 trials");
}
