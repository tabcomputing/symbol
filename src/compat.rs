//! Crystal standard-library behaviour that Rust's std doesn't share.
//!
//! Everything in this module exists only so the port matches the Crystal
//! implementation exactly; each item names the Crystal method it mirrors.

use std::cmp::Ordering;

use crate::error::{Error, Result};

/// `Char#ascii_whitespace?`. Unlike `char::is_ascii_whitespace`, includes vertical tab.
pub fn is_ascii_whitespace(c: char) -> bool {
    matches!(c, ' ' | '\t'..='\r')
}

/// `Char#whitespace?`. Unlike `char::is_whitespace`, excludes U+0085 (NEL).
pub fn is_whitespace(c: char) -> bool {
    if c.is_ascii() { is_ascii_whitespace(c) } else { c != '\u{85}' && c.is_whitespace() }
}

/// `String#strip`.
pub fn strip(s: &str) -> &str {
    s.trim_matches(is_whitespace)
}

/// `String#to_f64?`: surrounding whitespace is allowed, and literals that
/// overflow to infinity or underflow to zero are rejected (Rust rounds them).
pub fn parse_float(s: &str) -> Option<f64> {
    let s = strip(s);
    let x: f64 = s.parse().ok()?;
    let mantissa = s.split(['e', 'E']).next().unwrap_or_default();
    let overflowed = x.is_infinite() && !mantissa.to_ascii_lowercase().contains("inf");
    let underflowed = x == 0.0 && mantissa.bytes().any(|b| matches!(b, b'1'..=b'9'));
    (!overflowed && !underflowed).then_some(x)
}

/// `String#to_i64?`.
pub fn parse_int(s: &str) -> Option<i64> {
    strip(s).parse().ok()
}

/// `Float64#to_i64`: truncates, failing outside `[-2^63, 2^63)` and for NaN.
pub fn float_to_i64(x: f64) -> Result<i64> {
    const LIMIT: f64 = 9_223_372_036_854_775_808.0; // 2^63
    if (-LIMIT..LIMIT).contains(&x) { Ok(x as i64) } else { Err(Error::Overflow) }
}

/// `Int64#%`: floored modulo (the sign follows the divisor).
pub fn int_mod(x: i64, y: i64) -> Result<i64> {
    if y == 0 {
        return Err(Error::DivisionByZero);
    }
    let r = x.checked_rem(y).unwrap_or(0); // MIN % -1 overflows in Rust; it is 0
    Ok(if r != 0 && (r < 0) != (y < 0) { r + y } else { r })
}

/// `Float64#%`: `x - y * floor(x / y)`, failing for a zero divisor.
pub fn float_mod(x: f64, y: f64) -> Result<f64> {
    if y == 0.0 { Err(Error::DivisionByZero) } else { Ok(x - y * (x / y).floor()) }
}

/// `Int64#**` with a non-negative exponent: square-and-multiply, failing on overflow.
pub fn int_pow(base: i64, mut exponent: i64) -> Result<i64> {
    let (mut result, mut square) = (1i64, base);
    while exponent > 0 {
        if exponent & 1 != 0 {
            result = result.checked_mul(square).ok_or(Error::Overflow)?;
        }
        exponent >>= 1;
        if exponent > 0 {
            square = square.checked_mul(square).ok_or(Error::Overflow)?;
        }
    }
    Ok(result)
}

/// `Float64#to_s`: shortest round-trip digits, always with a decimal point,
/// switching to scientific notation outside 1e-4 ..< 1e16 (`1.0e+16`, `1.5e-5`).
pub fn format_float(x: f64) -> String {
    if x.is_nan() {
        return "NaN".to_owned();
    }
    let sign = if x.is_sign_negative() { "-" } else { "" };
    let x = x.abs();
    if x.is_infinite() {
        return format!("{sign}Infinity");
    }
    if x == 0.0 {
        return format!("{sign}0.0");
    }

    // Rust's `{:e}` gives the same shortest digits, e.g. "1.2345e-7".
    let scientific = format!("{x:e}");
    let (mantissa, exponent) = scientific.split_once('e').expect("`{:e}` has an exponent");
    let digits = mantissa.replace('.', "");
    let exponent: i32 = exponent.parse().expect("`{:e}` exponent is an integer");
    let point = exponent + 1; // position of the decimal point within `digits`

    let body = if !(-3..=15).contains(&point) {
        let (first, rest) = digits.split_at(1);
        let rest = if rest.is_empty() { "0" } else { rest };
        let plus = if exponent > 0 { "+" } else { "" };
        format!("{first}.{rest}e{plus}{exponent}")
    } else if point <= 0 {
        format!("0.{}{digits}", "0".repeat(point.unsigned_abs() as usize))
    } else if point as usize >= digits.len() {
        format!("{digits}{}.0", "0".repeat(point as usize - digits.len()))
    } else {
        let (whole, fraction) = digits.split_at(point as usize);
        format!("{whole}.{fraction}")
    };
    format!("{sign}{body}")
}

/// `String#inspect`: quoted, with Crystal's escapes (including `\#{`).
pub fn inspect_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{7}' => out.push_str("\\a"),
            '\u{8}' => out.push_str("\\b"),
            '\u{1b}' => out.push_str("\\e"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{b}' => out.push_str("\\v"),
            '#' if chars.peek() == Some(&'{') => {
                chars.next();
                out.push_str("\\#{");
            }
            // `Char#printable?` (Rust has no general-category API beyond Cc).
            c if !c.is_control() && (c == ' ' || !is_whitespace(c)) => out.push(c),
            c if (c as u32) > 0xFFFF => out.push_str(&format!("\\u{{{:X}}}", c as u32)),
            c => out.push_str(&format!("\\u{:04X}", c as u32)),
        }
    }
    out.push('"');
    out
}

/// `Array#sort` with a comparator block: a stable merge sort that Crystal
/// ported from Rust's pre-1.81 `slice::sort`.
///
/// Needed because grading treats NaN as equal to everything. With such a
/// non-total order, the result depends on the algorithm, and today's
/// `slice::sort_by` may even panic (an abort in WASM).
pub fn stable_sort_by<T: Copy>(v: &mut [T], mut cmp: impl FnMut(T, T) -> Ordering) {
    const MAX_INSERTION: usize = 10;
    const MIN_RUN: usize = 10;
    let len = v.len();

    if len <= MAX_INSERTION {
        for i in (0..len).rev() {
            insert_head(&mut v[i..], &mut cmp);
        }
        return;
    }

    let mut buf = Vec::with_capacity(len / 2);
    let mut runs: Vec<(usize, usize)> = Vec::new(); // (start, end) of sorted runs
    let mut end = len;
    while end > 0 {
        // Find the next natural run backwards, reversing it if strictly descending.
        let mut start = end - 1;
        if start > 0 {
            start -= 1;
            if cmp(v[start + 1], v[start]) == Ordering::Less {
                while start > 0 && cmp(v[start], v[start - 1]) == Ordering::Less {
                    start -= 1;
                }
                v[start..end].reverse();
            } else {
                while start > 0 && cmp(v[start], v[start - 1]) == Ordering::Greater {
                    start -= 1;
                }
            }
        }
        // Extend short runs with insertion sort.
        while start > 0 && end - start < MIN_RUN {
            start -= 1;
            insert_head(&mut v[start..end], &mut cmp);
        }
        runs.push((start, end));
        end = start;

        while let Some(r) = collapse(&runs) {
            let (left, right) = (runs[r + 1], runs[r]);
            merge(&mut v[left.0..right.1], left.1 - left.0, &mut buf, &mut cmp);
            runs[r] = (left.0, right.1);
            runs.remove(r + 1);
        }
    }
}

/// Insert `v[0]` into the already sorted `v[1..]`.
fn insert_head<T: Copy>(v: &mut [T], cmp: &mut impl FnMut(T, T) -> Ordering) {
    if v.len() < 2 || cmp(v[1], v[0]) != Ordering::Less {
        return;
    }
    let x = v[0];
    v[0] = v[1];
    for i in 2..v.len() {
        if cmp(v[i], x) != Ordering::Less {
            v[i - 1] = x;
            return;
        }
        v[i - 1] = v[i];
    }
    let last = v.len() - 1;
    v[last] = x;
}

/// Merge the sorted runs `v[..mid]` and `v[mid..]`, copying the shorter one into `buf`.
fn merge<T: Copy>(v: &mut [T], mid: usize, buf: &mut Vec<T>, cmp: &mut impl FnMut(T, T) -> Ordering) {
    let len = v.len();
    buf.clear();
    if mid <= len - mid {
        buf.extend_from_slice(&v[..mid]);
        let (mut left, mut right, mut out) = (0, mid, 0);
        while left < mid && right < len {
            // On ties take from the left run, for stability.
            if cmp(v[right], buf[left]) == Ordering::Less {
                v[out] = v[right];
                right += 1;
            } else {
                v[out] = buf[left];
                left += 1;
            }
            out += 1;
        }
        v[out..out + mid - left].copy_from_slice(&buf[left..mid]);
    } else {
        buf.extend_from_slice(&v[mid..]);
        let (mut left, mut right, mut out) = (mid, len - mid, len);
        while left > 0 && right > 0 {
            out -= 1;
            // On ties take from the right run, for stability.
            if cmp(buf[right - 1], v[left - 1]) == Ordering::Less {
                left -= 1;
                v[out] = v[left];
            } else {
                right -= 1;
                v[out] = buf[right];
            }
        }
        v[left..left + right].copy_from_slice(&buf[..right]);
    }
}

/// Which adjacent runs to merge next (TimSort's invariants on the top four runs), if any.
fn collapse(runs: &[(usize, usize)]) -> Option<usize> {
    let n = runs.len();
    let size = |i: usize| runs[i].1 - runs[i].0;
    let merge_needed = n >= 2
        && (runs[n - 1].0 == 0
            || size(n - 2) <= size(n - 1)
            || (n >= 3 && size(n - 3) <= size(n - 2) + size(n - 1))
            || (n >= 4 && size(n - 4) <= size(n - 3) + size(n - 2)));
    merge_needed.then(|| if n >= 3 && size(n - 3) < size(n - 1) { n - 3 } else { n - 2 })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_floats_like_crystal() {
        let cases = [
            (5.0, "5.0"),
            (2.5, "2.5"),
            (100.0, "100.0"),
            (0.1, "0.1"),
            (0.001, "0.001"),
            (0.0001, "0.0001"),
            (1.5e-5, "1.5e-5"),
            (1e-7, "1.0e-7"),
            (1e15, "1.0e+15"),
            (1e20, "1.0e+20"),
            (123456789012345.6, "123456789012345.6"),
            (999999999999999.9, "999999999999999.9"),
            (9999999999999998.0, "9.999999999999998e+15"),
            (5e-324, "5.0e-324"),
            (-0.0, "-0.0"),
            (f64::NEG_INFINITY, "-Infinity"),
            (f64::NAN, "NaN"),
        ];
        for (x, expected) in cases {
            assert_eq!(format_float(x), expected, "{x:?}");
        }
    }

    #[test]
    fn parses_floats_like_crystal() {
        assert_eq!(parse_float(" 3.5 "), Some(3.5));
        assert_eq!(parse_float("\u{b}3"), Some(3.0));
        assert_eq!(parse_float("\u{85}3"), None);
        assert_eq!(parse_float("1e400"), None);
        assert_eq!(parse_float("2e-324"), None);
        assert_eq!(parse_float("-inf"), Some(f64::NEG_INFINITY));
        assert_eq!(parse_float("1_000"), None);
        assert!(parse_float("nan").is_some_and(f64::is_nan));
    }

    #[test]
    fn stable_sort_matches_std_for_total_orders() {
        let mut seed = 42u64;
        for len in 0..200 {
            let data: Vec<u8> = (0..len)
                .map(|_| {
                    seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                    (seed >> 59) as u8
                })
                .collect();
            let mut ours: Vec<usize> = (0..len).collect();
            stable_sort_by(&mut ours, |i, j| data[i].cmp(&data[j]));
            let mut std: Vec<usize> = (0..len).collect();
            std.sort_by_key(|&i| data[i]);
            assert_eq!(ours, std);
        }
    }
}
