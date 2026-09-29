//! Generate a deterministic corpus of random SYMBOL expressions and programs
//! for differential testing (format: see tests/support/mod.rs).
//!
//!     cargo run --example gen_corpus -- <count> [seed] > corpus.txt

const NUMBERS: &[&str] = &[
    "0",
    "1",
    "2",
    "3",
    "-1",
    "-2",
    "10",
    "100",
    "2.5",
    "0.5",
    "-0.5",
    "0.0",
    "-0.0",
    "3.0",
    "4.7",
    "9223372036854775807",
    "-9223372036854775808",
    "3000000000",
    "0.1",
    "1234567.891",
];
const STRINGS: &[&str] =
    &["\"a\"", "\"3\"", "\"\"", "\"2.5\"", "\"nan\"", "\"héllo\"", "\" 7 \"", "\"x#{y}\"", "\"q\\\"t\""];
const BOOLS: &[&str] = &["true", "false", "⊤", "⊥"];
const VARS: &[&str] =
    &["x", "y", "s", "n", "xs", "e", "b", "f", "z", "m", "big", "w", "nz", "fs", "ns", "q", "u", "v"];
const OPS: &[&str] = &[
    "+", "-", "*", "/", "%", "^", "==", "!=", "≠", "<", ">", "<=", ">=", "≤", "≥", "!", "~", "[+]", "[*]",
    "[-]", "[~]", "Σ", "Π", "#", "⌈", "⌊", "><", "<>", "+>", "<+", "~>", "<~", "->", "<-", "<->", "↑", "↓",
    "@>", "<@", "⌽", "⍋", "⍒", "?",
];
const JUNK: &[&str] = &["$", "@", "&", ";", "{", "}", ")", "(", "]", "=", ".", ","];
const NAMES: &[&str] = &["x", "y", "a", "c", "xs", "f", "z"];

/// xorshift64* — small, deterministic, good enough for test generation.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    fn chance(&mut self, percent: usize) -> bool {
        self.below(100) < percent
    }

    fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        items[self.below(items.len())]
    }
}

fn atom(rng: &mut Rng) -> String {
    match rng.below(10) {
        0..=3 => rng.pick(NUMBERS),
        4..=5 => rng.pick(STRINGS),
        6 => rng.pick(BOOLS),
        _ => rng.pick(VARS),
    }
    .to_owned()
}

fn list(rng: &mut Rng, depth: usize) -> String {
    let max_len = if rng.chance(15) { 16 } else { 5 };
    let len = rng.below(max_len);
    let items: Vec<String> = (0..len)
        .map(|_| if depth < 2 && rng.chance(10) { list(rng, depth + 1) } else { atom(rng) })
        .collect();
    let separator = if rng.chance(80) { ", " } else { " " };
    format!("[{}]", items.join(separator))
}

fn term(rng: &mut Rng, depth: usize) -> String {
    match rng.below(100) {
        0..=34 => atom(rng),
        35..=74 => rng.pick(OPS).to_owned(),
        75..=83 => list(rng, depth),
        // `..` only between small literals: `big .. 1` would allocate ~10^19 items in either implementation.
        84..=86 => {
            let bound = ["-3", "-1", "0", "1", "2", "4", "2.5", "\"3\"", "true"];
            format!("({} .. {})", rng.pick(&bound), rng.pick(&bound))
        }
        87..=97 if depth < 3 => format!("({})", expression(rng, depth + 1)),
        _ => rng.pick(JUNK).to_owned(),
    }
}

fn expression(rng: &mut Rng, depth: usize) -> String {
    let len = 1 + rng.below(6);
    let mut out = String::new();
    for i in 0..len {
        if i > 0 {
            out.push_str(match rng.below(20) {
                0 => "",
                1 => "\\n",
                _ => " ",
            });
        }
        out.push_str(&term(rng, depth));
    }
    // Program-mode separators must not appear inside plain expressions too often.
    out.replace(" . ", " ")
}

fn program(rng: &mut Rng) -> String {
    let statements: Vec<String> = (0..1 + rng.below(4))
        .map(|_| {
            if rng.chance(60) {
                format!("{} = {}", rng.pick(NAMES), expression(rng, 1))
            } else {
                expression(rng, 1)
            }
        })
        .collect();
    let mut out = statements.join(if rng.chance(80) { ". " } else { "." });
    if rng.chance(50) {
        out.push('.');
    }
    out
}

fn main() {
    let mut args = std::env::args().skip(1);
    let count: usize = args.next().and_then(|a| a.parse().ok()).unwrap_or(1000);
    let seed: u64 = args.next().and_then(|a| a.parse().ok()).unwrap_or(0x5EED);
    let mut rng = Rng(seed | 1);
    for _ in 0..count {
        if rng.chance(70) {
            println!("E\t{}", expression(&mut rng, 0));
        } else {
            println!("P\t{}", program(&mut rng));
        }
    }
}
