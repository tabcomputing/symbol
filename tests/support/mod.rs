//! Differential testing against the Crystal implementation.
//!
//! A corpus line is `E<TAB>source` (expression mode) or `P<TAB>source`
//! (program mode), with `\n`, `\t`, `\r`, `\v` and `\\` escaped. Each line is
//! evaluated with the same fixed bindings in both implementations and
//! rendered in a canonical, type-tagged form that must match byte for byte.
//! The Crystal twin of this file lives with the benchmark scripts
//! (`diff_driver.cr`); see README.md.

#![allow(dead_code)]

use symbol::{Bindings, EvalResult, Value};

pub fn base_bindings() -> Bindings {
    let pairs: [(&str, Value); 16] = [
        ("x", Value::Int(3)),
        ("y", Value::Float(2.5)),
        ("s", "hello".into()),
        ("n", "42".into()),
        ("xs", vec![1i64, 2, 3].into()),
        ("e", Value::Array(vec![])),
        ("b", Value::Bool(true)),
        ("f", Value::Bool(false)),
        ("z", Value::Nil),
        (
            "m",
            Value::Array(vec![
                Value::Int(1),
                "a".into(),
                Value::Array(vec![Value::Int(2), Value::Float(3.5)]),
                Value::Bool(true),
                Value::Nil,
                Value::Bool(false),
            ]),
        ),
        ("big", Value::Int(i64::MAX)),
        ("w", "nan".into()),
        ("nz", Value::Float(-0.0)),
        ("fs", vec![2.5, 1.0, 3.0].into()),
        ("ns", vec!["3", "1", "2"].into()),
        (
            "q",
            Value::Array(vec![
                Value::Int(1),
                "nan".into(),
                Value::Int(2),
                "nan".into(),
                Value::Int(0),
                Value::Float(-1.5),
            ]),
        ),
    ];
    pairs.into_iter().map(|(k, v)| (k.to_owned(), v)).collect()
}

/// Evaluate one corpus line and render the canonical output line.
pub fn run_line(line: &str) -> String {
    let (mode, raw) = line.split_once('\t').unwrap_or((line, ""));
    let source = unescape(raw);
    let mut bindings = base_bindings();
    let result = if mode == "P" {
        symbol::eval_program(&source, &mut bindings)
    } else {
        symbol::eval(&source, &bindings)
    };
    match result {
        Err(error) => format!("ERR {}", escape(&error.to_string())),
        Ok(result) => {
            let wasm = mask_addresses(&symbol::wasm::format_result(&result));
            let mut out = format!("{} || \"{}\"", canon_result(&result), escape(&wasm));
            if mode == "P" {
                let mut keys: Vec<_> = bindings.keys().collect();
                keys.sort();
                let rendered: Vec<String> =
                    keys.iter().map(|k| format!("{k}={}", canon_value(&bindings[*k]))).collect();
                out.push_str(" || ");
                out.push_str(&rendered.join(";"));
            }
            out
        }
    }
}

pub fn canon_value(value: &Value) -> String {
    match value {
        Value::Int(i) => format!("i:{i}"),
        Value::Float(_) => format!("f:{value}"),
        Value::Str(s) => format!("s:\"{}\"", escape(s)),
        Value::Bool(b) => format!("b:{b}"),
        Value::Nil => "nil".to_owned(),
        Value::Array(items) => {
            format!("[{}]", items.iter().map(canon_value).collect::<Vec<_>>().join(","))
        }
    }
}

pub fn canon_result(result: &EvalResult) -> String {
    match result {
        EvalResult::Resolved(value) => format!("R {}", canon_value(value)),
        EvalResult::Unbound(name) => format!("U {name}"),
        EvalResult::Suspended(s) => format!(
            "S({}/{}|{})",
            s.op,
            s.arity(),
            s.args.iter().map(canon_result).collect::<Vec<_>>().join(";")
        ),
    }
}

pub fn escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => out.push_str(&format!("\\x{:x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

pub fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('v') => out.push('\u{b}'),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

/// Replace `0x<hex>` object addresses (which differ run to run) with `0xADDR`.
pub fn mask_addresses(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(at) = rest.find("0x") {
        out.push_str(&rest[..at]);
        let digits = rest[at + 2..].bytes().take_while(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')).count();
        if digits == 0 {
            out.push_str("0x");
        } else {
            out.push_str("0xADDR");
        }
        rest = &rest[at + 2 + digits..];
    }
    out.push_str(rest);
    out
}
