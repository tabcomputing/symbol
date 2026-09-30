//! Operator semantics: the big `case` in Crystal's `Evaluator#execute`.

use std::cmp::Ordering;

use crate::ast::Op;
use crate::compat;
use crate::error::{Error, Result};
use crate::value::Value;

pub(crate) fn unary(op: Op, a: Value) -> Result<Value> {
    Ok(match op {
        Op::Not => Value::Bool(!a.is_truthy()),
        Op::Neg => negate(a)?,
        Op::BitNot => bit_not(a)?,
        Op::Sum => sum(&a.into_array())?,
        Op::Product => product(&a.into_array())?,
        Op::Count => Value::Int(a.into_array().len() as i64),
        Op::CeilMax => match a {
            Value::Int(_) => a,
            Value::Array(items) => extreme(&items, Ordering::Greater)?,
            scalar => Value::Int(compat::float_to_i64(scalar.to_float().ceil())?),
        },
        Op::FloorMin => match a {
            Value::Int(_) => a,
            Value::Array(items) => extreme(&items, Ordering::Less)?,
            scalar => Value::Int(compat::float_to_i64(scalar.to_float().floor())?),
        },
        Op::Reverse => {
            let mut items = a.into_array();
            items.reverse();
            Value::Array(items)
        }
        Op::GradeUp => grade(&a.into_array(), false),
        Op::GradeDown => grade(&a.into_array(), true),
        _ => unreachable!("{op:?} is not unary"),
    })
}

pub(crate) fn binary(op: Op, a: Value, b: Value) -> Result<Value> {
    Ok(match op {
        Op::Add => vectorize(&a, &b, |x, y| arith(x, y, i64::checked_add, |x, y| x + y))?,
        Op::Sub => vectorize(&a, &b, |x, y| arith(x, y, i64::checked_sub, |x, y| x - y))?,
        Op::Mul => vectorize(&a, &b, |x, y| arith(x, y, i64::checked_mul, |x, y| x * y))?,
        Op::Div => vectorize(&a, &b, divide)?,
        Op::Mod => vectorize(&a, &b, modulo)?,
        Op::Pow => vectorize(&a, &b, power)?,

        // Comparison. Ordering is numeric for every type (strings via parsing).
        Op::Eq => Value::Bool(a.loose_eq(&b)),
        Op::NotEq => Value::Bool(!a.loose_eq(&b)),
        Op::Lt => Value::Bool(a.to_float() < b.to_float()),
        Op::Gt => Value::Bool(a.to_float() > b.to_float()),
        Op::LtEq => Value::Bool(a.to_float() <= b.to_float()),
        Op::GtEq => Value::Bool(a.to_float() >= b.to_float()),

        Op::BitOr => bitwise(&a, &b, |x, y| x || y, |x, y| x | y)?,
        Op::BitAnd => bitwise(&a, &b, |x, y| x && y, |x, y| x & y)?,
        Op::BitXor => bitwise(&a, &b, |x, y| x != y, |x, y| x ^ y)?,

        Op::Range => range(a.to_int()?, b.to_int()?),
        Op::Concat => Value::Array(concat(a.into_array(), b.into_array())),
        Op::Wrap => Value::Array(vec![a, b]),
        Op::Cons => Value::Array(concat(vec![a], b.into_array())),
        Op::Snoc => Value::Array(concat(a.into_array(), vec![b])),
        Op::Zip => Value::Array(interleave(a.into_array(), b.into_array())),
        Op::Piz => Value::Array(interleave(b.into_array(), a.into_array())),
        Op::RemoveBack => Value::Array(remove_suffix(a.into_array(), &b.into_array())),
        Op::RemoveFront => Value::Array(remove_prefix(a.into_array(), &b.into_array())),
        Op::RemoveBoth => {
            let affix = b.into_array();
            Value::Array(remove_suffix(remove_prefix(a.into_array(), &affix), &affix))
        }
        Op::Take => Value::Array(take(to_i32(a.to_int()?)?, b.into_array())),
        Op::Drop => Value::Array(drop(to_i32(a.to_int()?)?, b.into_array())),
        Op::IndexRight => index(&a, &b.into_array())?,
        Op::IndexLeft => index(&b, &a.into_array())?,
        _ => unreachable!("{op:?} is not binary"),
    })
}

// ---- Arithmetic --------------------------------------------------------------

/// Apply `f` element-wise when either side is an array.
fn vectorize(a: &Value, b: &Value, f: impl Fn(&Value, &Value) -> Result<Value>) -> Result<Value> {
    let items: Vec<Value> = match (a, b) {
        // Crystal's `zip` fails when the right array is shorter, and truncates when it's longer.
        (Value::Array(xs), Value::Array(ys)) if ys.len() < xs.len() => {
            return Err(Error::IndexOutOfBounds);
        }
        (Value::Array(xs), Value::Array(ys)) => {
            xs.iter().zip(ys).map(|(x, y)| f(x, y)).collect::<Result<_>>()?
        }
        (Value::Array(xs), _) => xs.iter().map(|x| f(x, b)).collect::<Result<_>>()?,
        (_, Value::Array(ys)) => ys.iter().map(|y| f(a, y)).collect::<Result<_>>()?,
        _ => return f(a, b),
    };
    Ok(Value::Array(items))
}

/// Integer arithmetic when both sides are integers (failing on overflow), float otherwise.
fn arith(
    a: &Value,
    b: &Value,
    int: fn(i64, i64) -> Option<i64>,
    float: fn(f64, f64) -> f64,
) -> Result<Value> {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => int(*x, *y).map(Value::Int).ok_or(Error::Overflow),
        _ => Ok(Value::Float(float(a.to_float(), b.to_float()))),
    }
}

/// Integer division stays integral only when exact. Division by zero is +Infinity.
fn divide(a: &Value, b: &Value) -> Result<Value> {
    Ok(match (a, b) {
        (Value::Int(_), Value::Int(0)) => Value::Float(f64::INFINITY),
        // `checked_rem` is None only for MIN % -1, which is 0 in Crystal.
        (Value::Int(x), Value::Int(y)) if x.checked_rem(*y).unwrap_or(0) == 0 => {
            Value::Int(x.checked_div(*y).ok_or(Error::DivisionOverflow)?)
        }
        (Value::Int(x), Value::Int(y)) => Value::Float(*x as f64 / *y as f64),
        _ if b.to_float() == 0.0 => Value::Float(f64::INFINITY),
        _ => Value::Float(a.to_float() / b.to_float()),
    })
}

fn modulo(a: &Value, b: &Value) -> Result<Value> {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => compat::int_mod(*x, *y).map(Value::Int),
        _ => compat::float_mod(a.to_float(), b.to_float()).map(Value::Float),
    }
}

fn power(a: &Value, b: &Value) -> Result<Value> {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) if *y >= 0 => compat::int_pow(*x, *y).map(Value::Int),
        _ => Ok(Value::Float(a.to_float().powf(b.to_float()))),
    }
}

fn negate(a: Value) -> Result<Value> {
    let neg = |v: &Value| match v {
        Value::Int(i) => i.checked_neg().map(Value::Int).ok_or(Error::Overflow),
        other => Ok(Value::Float(-other.to_float())),
    };
    match a {
        Value::Array(items) => items.iter().map(neg).collect::<Result<_>>().map(Value::Array),
        scalar => neg(&scalar),
    }
}

/// Boolean logic for two booleans; otherwise bitwise on integers, vectorized.
fn bitwise(a: &Value, b: &Value, logic: fn(bool, bool) -> bool, bits: fn(i64, i64) -> i64) -> Result<Value> {
    match (a, b) {
        (Value::Bool(x), Value::Bool(y)) => Ok(Value::Bool(logic(*x, *y))),
        _ => vectorize(a, b, |x, y| Ok(Value::Int(bits(x.to_int()?, y.to_int()?)))),
    }
}

fn bit_not(a: Value) -> Result<Value> {
    let not = |v: &Value| match v {
        Value::Bool(b) => Ok(Value::Bool(!b)),
        other => Ok(Value::Int(!other.to_int()?)),
    };
    match a {
        Value::Array(items) => items.iter().map(not).collect::<Result<_>>().map(Value::Array),
        scalar => not(&scalar),
    }
}

// ---- Aggregation ----------------------------------------------------------------

/// The items as integers, if they all are.
fn ints(items: &[Value]) -> Option<Vec<i64>> {
    items.iter().map(Value::as_int).collect()
}

fn sum(items: &[Value]) -> Result<Value> {
    match ints(items) {
        Some(ints) => ints.into_iter().try_fold(0, i64::checked_add).map(Value::Int).ok_or(Error::Overflow),
        // An explicit fold: std's `f64: Sum` starts from -0.0, Crystal from 0.0.
        None => Ok(Value::Float(items.iter().fold(0.0, |acc, v| acc + v.to_float()))),
    }
}

fn product(items: &[Value]) -> Result<Value> {
    match ints(items) {
        Some(ints) => ints.into_iter().try_fold(1, i64::checked_mul).map(Value::Int).ok_or(Error::Overflow),
        None => Ok(Value::Float(items.iter().fold(1.0, |acc, v| acc * v.to_float()))),
    }
}

/// The maximum (`Greater`) or minimum (`Less`) item: an integer if all items are.
fn extreme(items: &[Value], wanted: Ordering) -> Result<Value> {
    if let Some(ints) = ints(items) {
        let best = if wanted == Ordering::Greater { ints.into_iter().max() } else { ints.into_iter().min() };
        return best.map(Value::Int).ok_or(Error::Empty);
    }
    let mut floats = items.iter().map(Value::to_float);
    let mut best = floats.next().ok_or(Error::Empty)?;
    for x in floats {
        // Crystal compares with `<=>`, which fails when NaN is involved.
        match x.partial_cmp(&best) {
            Some(order) if order == wanted => best = x,
            Some(_) => {}
            None => return Err(Error::Comparison(x, best)),
        }
    }
    Ok(Value::Float(best))
}

/// 1-based indices that would sort the items (by numeric value, stably).
fn grade(items: &[Value], descending: bool) -> Value {
    let keys: Vec<f64> = items.iter().map(Value::to_float).collect();
    let mut order: Vec<usize> = (0..keys.len()).collect();
    compat::stable_sort_by(&mut order, |i, j| {
        let (x, y) = if descending { (keys[j], keys[i]) } else { (keys[i], keys[j]) };
        x.partial_cmp(&y).unwrap_or(Ordering::Equal) // NaN ties with everything
    });
    Value::Array(order.into_iter().map(|i| Value::Int(i as i64 + 1)).collect())
}

// ---- Structure ------------------------------------------------------------------

fn concat(mut first: Vec<Value>, second: Vec<Value>) -> Vec<Value> {
    first.extend(second);
    first
}

fn range(start: i64, end: i64) -> Value {
    let items: Vec<Value> = if start <= end {
        (start..=end).map(Value::Int).collect()
    } else {
        (end..=start).rev().map(Value::Int).collect()
    };
    Value::Array(items)
}

/// `first[0], second[0], first[1], second[1], ...`, continuing with the longer one.
fn interleave(first: Vec<Value>, second: Vec<Value>) -> Vec<Value> {
    let mut out = Vec::with_capacity(first.len() + second.len());
    let (mut xs, mut ys) = (first.into_iter(), second.into_iter());
    loop {
        match (xs.next(), ys.next()) {
            (None, None) => return out,
            (x, y) => out.extend(x.into_iter().chain(y)),
        }
    }
}

fn starts_with(items: &[Value], prefix: &[Value]) -> bool {
    items.len() >= prefix.len() && items.iter().zip(prefix).all(|(x, p)| x.loose_eq(p))
}

fn remove_prefix(mut items: Vec<Value>, prefix: &[Value]) -> Vec<Value> {
    if starts_with(&items, prefix) {
        items.drain(..prefix.len());
    }
    items
}

fn remove_suffix(mut items: Vec<Value>, suffix: &[Value]) -> Vec<Value> {
    if let Some(start) = items.len().checked_sub(suffix.len())
        && starts_with(&items[start..], suffix)
    {
        items.truncate(start);
    }
    items
}

/// The first `n` items, or the last `-n` if `n` is negative.
fn take(n: i32, mut items: Vec<Value>) -> Vec<Value> {
    if n >= 0 {
        items.truncate(n as usize);
    } else {
        items.drain(..items.len().saturating_sub(n.unsigned_abs() as usize));
    }
    items
}

/// All but the first `n` items, or all but the last `-n` if `n` is negative.
fn drop(n: i32, mut items: Vec<Value>) -> Vec<Value> {
    if n >= 0 {
        items.drain(..items.len().min(n as usize));
    } else {
        items.truncate(items.len().saturating_sub(n.unsigned_abs() as usize));
    }
    items
}

/// Look up a 1-based index, or an array of them, in `items`.
fn index(index: &Value, items: &[Value]) -> Result<Value> {
    let lookup = |i: &Value| Ok(element(items, to_i32(i.to_int()?)?).cloned().unwrap_or_default());
    match index {
        Value::Array(indices) => indices.iter().map(lookup).collect::<Result<_>>().map(Value::Array),
        single => lookup(single),
    }
}

/// 1-based lookup; negative indices count from the end and 0 is nil.
fn element(items: &[Value], n: i32) -> Option<&Value> {
    let len = items.len() as i64;
    let i = match i64::from(n) {
        0 => return None,
        n if n > 0 => n - 1,
        // Crystal computes `items[len + n]?`, and `[]?` itself wraps negative
        // indices, so -5 on a 3-item array reaches item 2.
        n if len + n < 0 => 2 * len + n,
        n => len + n,
    };
    usize::try_from(i).ok().and_then(|i| items.get(i))
}

fn to_i32(n: i64) -> Result<i32> {
    i32::try_from(n).map_err(|_| Error::Overflow)
}
