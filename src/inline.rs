//! `{{ expr }}` templates in text (port of `inline.cr`).
//!
//! Expressions are evaluated in program mode. Markdown code spans and fences
//! are copied literally, `\{{` produces a literal `{{`, and an expression
//! that fails or doesn't resolve is left as written.
//!
//! Scanning works on bytes: every delimiter is ASCII, so byte offsets always
//! fall on character boundaries.

use crate::compat;
use crate::eval::Bindings;
use crate::value::{EvalResult, Value, list};

pub fn process(text: &str, bindings: &mut Bindings) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut fence: Option<usize> = None; // backtick count of the open code fence
    let mut i = 0;

    while i < bytes.len() {
        if let Some(ticks) = fence {
            let c = char_at(text, i);
            out.push(c);
            i += c.len_utf8();
            if c == '\n'
                && let Some(end) = fence_close(bytes, i, ticks)
            {
                out.push_str(&text[i..end]);
                i = end;
                fence = None;
            }
            continue;
        }

        if bytes[i] == b'`'
            && at_line_start(bytes, i)
            && let Some((end, ticks)) = fence_open(bytes, i)
        {
            out.push_str(&text[i..end]);
            i = end;
            fence = Some(ticks);
            continue;
        }
        if bytes[i] == b'`'
            && let Some(end) = code_span(bytes, i)
        {
            out.push_str(&text[i..end]);
            i = end;
            continue;
        }
        if bytes[i..].starts_with(b"\\{{") {
            out.push_str("{{");
            i += 3;
            continue;
        }
        if bytes[i..].starts_with(b"{{")
            && let Some(close) = find_closing_braces(bytes, i + 2)
        {
            let original = &text[i..close + 2];
            let expr = compat::strip(&text[i + 2..close]);
            let evaluated = if expr.is_empty() { None } else { evaluate(expr, bindings) };
            out.push_str(evaluated.as_deref().unwrap_or(original));
            i = close + 2;
            continue;
        }

        let c = char_at(text, i);
        out.push(c);
        i += c.len_utf8();
    }
    out
}

/// How a value is written into text: like `Display`, but array items use
/// this format too (so nested strings stay unquoted).
pub fn format(value: &Value) -> String {
    match value {
        Value::Array(items) => list(items.iter().map(format)),
        other => other.to_string(),
    }
}

fn evaluate(expr: &str, bindings: &mut Bindings) -> Option<String> {
    match crate::eval_program(expr, bindings) {
        Ok(EvalResult::Resolved(value)) => Some(format(&value)),
        _ => None,
    }
}

fn char_at(text: &str, i: usize) -> char {
    text[i..].chars().next().expect("index is in bounds and on a char boundary")
}

fn at_line_start(bytes: &[u8], i: usize) -> bool {
    i == 0 || bytes[i - 1] == b'\n'
}

fn count_backticks(bytes: &[u8], from: usize) -> usize {
    bytes[from..].iter().take_while(|&&b| b == b'`').count()
}

/// Index of the first `}` of the next `}}`.
fn find_closing_braces(bytes: &[u8], from: usize) -> Option<usize> {
    bytes[from..].windows(2).position(|w| w == b"}}").map(|p| from + p)
}

/// A fence opens with 3+ backticks; the whole line (with any info string) is part of it.
/// Returns the end of that line and the backtick count.
fn fence_open(bytes: &[u8], start: usize) -> Option<(usize, usize)> {
    let ticks = count_backticks(bytes, start);
    if ticks < 3 {
        return None;
    }
    let end = match bytes[start..].iter().position(|&b| b == b'\n') {
        Some(newline) => start + newline + 1,
        None => bytes.len(),
    };
    Some((end, ticks))
}

/// A line of at least `ticks` backticks, then optional spaces, closes the fence.
/// Returns the end of that line.
fn fence_close(bytes: &[u8], start: usize, ticks: usize) -> Option<usize> {
    let run = count_backticks(bytes, start);
    if run < ticks {
        return None;
    }
    let mut i = start + run;
    while bytes.get(i) == Some(&b' ') {
        i += 1;
    }
    match bytes.get(i) {
        None => Some(i),
        Some(b'\n') => Some(i + 1),
        Some(_) => None,
    }
}

/// A code span closes with a run of exactly as many backticks as opened it.
/// Returns the index just past the closing run.
fn code_span(bytes: &[u8], start: usize) -> Option<usize> {
    let ticks = count_backticks(bytes, start);
    if ticks >= 3 && at_line_start(bytes, start) {
        return None; // a fence, not a span
    }
    let mut i = start + ticks;
    while i < bytes.len() {
        let run = count_backticks(bytes, i);
        if run == ticks {
            return Some(i + run);
        }
        i += run.max(1);
    }
    None
}
