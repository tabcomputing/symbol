# symbol-rs

A Rust port of the Crystal SYMBOL expression language (`../symbol-cr`, shard
`symbols`, module `SYMBOL`). It was built as an experiment to help decide
whether the browser runtime (the WAM + Rete engine, which calls SYMBOL for
constraints) should move from Crystal to Rust. The port covers the lexer,
parser, tacit evaluator, program mode, `{{ }}` inline templates and the WASM
C ABI. The REPL and CLI are not ported. It uses only std.

```rust
use symbol::{Bindings, EvalResult, Value};

let result = symbol::eval("Σ [1, 2, 3, 4]", &Bindings::new())?;
assert_eq!(result, EvalResult::Resolved(Value::Int(10)));

let mut bindings = Bindings::new();
symbol::eval_program("x = 3. y = x * 2. y + 1", &mut bindings)?; // Resolved(7), y = 6
symbol::inline("x is {{ 2 + 3 }}", &mut bindings);               // "x is 5"
```

`SYMBOL.eval(src, b, program: true)` becomes `symbol::eval_program(src, &mut b)`.
Exceptions become `Result<_, symbol::Error>`, and each error's `Display` text
is exactly the Crystal exception message.

## Results at a glance

| | Crystal | Rust |
|---|---|---|
| Spec suite | 134 examples pass | **134/134 ported, all pass** (plus 5 unit tests, 1 doctest, 2 golden-file tests) |
| Differential test vs Crystal, native | — | **0 differences in 503,529 inputs** (before the poly-fix evaluator, below) |
| WASM size, raw / gzip -9 / brotli | 684,208 / 241,049 / 188,995 B | **188,166 / 71,089 / 61,174 B** |
| WASM memory after 100k evals of `Σ [1, 2, 3, 4]` | 0.19 → **171 MB** (grows ~1.7 KB/eval) | 1.06 → **1.13 MB** (flat) |
| WASM memory after 100k program-mode evals | 0.19 → **380 MB** (~3.8 KB/eval) | 1.06 → **1.13 MB** (flat) |
| WASM behaviour on any error (e.g. `(1`) | **traps** (`unreachable`); instance corrupted after ~200–4,100 traps | returns `"Error: 1:3: Expected RParen, got EOF"` |
| Native speed (`Σ [1, 2, 3, 4]`) | 2.5 µs/eval | 0.43 µs/eval |
| WASM speed (`Σ [1, 2, 3, 4]`, incl. JS marshalling) | 2.2 µs/eval | 1.3 µs/eval |

Environment: Crystal 1.21.0 (LLVM 22), Rust 1.90.0, Node 26.8.2, x86-64 Linux.

## Tests and behavioural parity

```
cargo test        # 134 spec ports + differential fixtures + unit tests
```

* `tests/symbol_spec.rs`, `tacit_spec.rs`, `statement_spec.rs` and
  `inline_spec.rs` port the four Crystal spec files one-to-one, with one
  `#[test]` per `it`: 80 + 11 + 17 + 26 = 134. Every example ported faithfully.
  `Value`'s `PartialEq` is type-exact, so `assert_eq!(v, Value::Int(5))` checks
  both Crystal's `be_a(Int64)` and `eq(5)`. That makes the Rust assertions
  slightly stricter than the originals.
* `tests/differential.rs` compares against golden output. Until 2026-09-30
  that was the **Crystal implementation**'s output. Since the poly-fix
  evaluator, it is this implementation's own, reviewed
  (`BLESS=1 cargo test --test differential` rewrites it). It covers
  `tests/fixtures/handwritten.txt` (529 edge cases aimed at every quirk
  below) and `random.txt` (3,000 generated expressions/programs). Each line
  is evaluated with the same 16 bindings (ints, floats, `-0.0`, strings like
  `"nan"`, nested arrays, nil, false). The output is a type-tagged rendering
  plus the exact WASM `format_result` string, and the bindings too in program
  mode.
* Beyond the fixtures, 500,000 further random lines (5 seeds × 100k,
  `examples/gen_corpus.rs`) went through both implementations: **0
  differences**. Outcome mix: 131k resolved values, 158k suspended, 124k
  `Index out of bounds`, 75k parse errors, and ~6k overflow, division, empty
  or NaN-comparison errors.
* Fuzzing found exactly one mismatch during development, an error-precedence
  corner case in quirk 1. It was fixed, and the poly-fix evaluator later
  removed the quirk itself.

### Crystal quirks deliberately preserved

The port keeps these because the brief was to match observable behaviour.
Several look like bugs worth fixing on the Crystal side. Quirks 1, 4 and 6
are no longer preserved: 1 and 6 were bugs in the evaluator, which the
poly-fix evaluator (below) doesn't have, and 4 went with list expressions.

1. *No longer preserved.* **`1 + + 2`, `1 + Σ` raised `Index out of bounds`.**
   `execute` ran a nested `Suspended` that still lacked arguments. This hit
   25% of random inputs.
2. Vectorised ops over two arrays raise `Index out of bounds` if the right
   array is shorter, but silently truncate if the left one is (`Array#zip`).
3. `-5 @> [10, 20, 30]` is `20`: a negative index wraps twice, because
   `arr[size + n]?` wraps again. Only indices below `-2 × len` give nil.
4. *No longer preserved.* Inside a list literal, a variable bound to `false`
   became nil (`bindings[name]? || nil`).
5. Unknown string escapes drop the character: `"a\qb"` is `a\b`.
6. *No longer preserved.* An unresolved `( … )` group discarded everything
   to its right. Of two adjacent values, the leftmost won (`1 + 2 3` was 3).
   A value to the left of an unbound variable was dropped.
7. `nil` means "no value": an operator applied to a nil result stays suspended.
8. `/ 0` gives `+Infinity` whatever the signs (`-1 / 0`, `0 / 0`), but `% 0`
   raises `Division by 0`.
9. Strings take part in arithmetic and ordering through `to_f64?`
   (`"3" + 1` = `4.0`, `"abc" < "b"` is false).
10. `⌈`/`⌊` raise on empty arrays (`Empty enumerable`) and when NaN is
    involved (`Comparison of NaN and 1.0 failed`).
11. In program mode, a parse error at end of input reports position `1:1`,
    because a bare EOF token is appended.
12. Integer arithmetic is overflow-checked (Crystal raises `OverflowError`).
    `Int64::MIN / -1` raises `Overflow: Int64::MIN / -1`, while
    `Int64::MIN % -1` is `0`.
13. `Suspended#to_s` embeds object addresses
    (`#<SYMBOL::Tacit::Resolved:0x7f… @value=5>`). Rust prints the address of
    the argument slot, which is equally meaningless.

### Where Rust differs from Crystal

* **Poly-fix evaluation (new, 2026-09-30).** SYMBOL is meant to be poly-fix:
  with fixed arities, `+ 1 1`, `1 + 1` and `1 1 +` all mean 2. The Crystal
  evaluator folded the terms into a single accumulator, which can't hold two
  unfinished things at once. So `+ 1 1` dropped a 1 and stayed suspended, and
  in `2 3 4 + *` the waiting `*` became an argument of `+`, which raised
  `Index out of bounds`. The evaluator now keeps a stack of pieces, with the
  rules in the crate docs, and `2 3 4 + *` is 14. What doesn't combine is a
  partial application. That is a waiting operator (`1 +`), as before, or a
  new `EvalResult::Sequence` of several pieces, such as values waiting for an
  operator (`1 2`). A group's pieces take part in the enclosing expression:
  `(1 +) 2` and `(1 2) +` are both 3. `(x + 1) * 2` with `x` unbound keeps
  the `* 2` (Crystal lost it).

  Of the 3,529 golden lines, 1,842 changed. 759 had raised
  `Index out of bounds`, which now comes only from operators themselves (two
  arrays of unequal length). The other 1,083 had lost a value or part of the
  expression, or (5 of them) had raised an error from an operator given the
  wrong arguments. 904 of those are now sequences. Every line was checked
  against a separate implementation of the rules that ran each operator
  through the Crystal-verified evaluator, and the two agreed on all of them.
* **List elements are expressions (new, 2026-09-30).** Commas separate a
  list's parts, each an expression, and the values a part leaves are the
  list's elements: `[1 + 1, 2 * 3]` is `[2, 6]`, while `[1 2 3]` and
  `[1 2, 3 4]` stay flat lists of three and four. `+` in `[1 + 1 2]` takes
  the two values to its right, as anywhere, so that is `[1, 3]`. An unknown
  element (an unbound variable, or a computation that can't run) is nil, and
  an operator left waiting (`[1 +]`) is an error. Crystal allowed only
  literals, variables and nested lists (`[1 + 2]` was a parse error).
* **A glued `-` subtracts (new, 2026-09-30).** A `-` before a digit is a
  sign unless it comes right after a value, as in Julia. `3-3`, `x-1` and
  `(5)-1` subtract, and `3 - 3` does too, while `3 -3` is two values and
  `-3`, `(-3)` and `+ -3 5` have negative literals. Crystal always read a
  sign, so `x-1` was `x` followed by `-1`.
* **Operator names (new, 2026-09-30).** Every symbolic operator can also be
  typed as its LaTeX command, or as a descriptive name where LaTeX has none.
  `\sum`, `\prod`, `\count`, `\max` (or `\lceil`), `\min` (or `\lfloor`),
  `\neq`/`\ne`, `\leq`/`\le`, `\geq`/`\ge`, `\top`, `\bot`,
  `\uparrow`, `\downarrow`, `\reverse`, `\gradeup` and `\gradedown` are the
  same tokens as `Σ Π # ⌈ ⌊ ≠ ≤ ≥ ⊤ ⊥ ↑ ↓ ⌽ ⍋ ⍒`. Bare words stay variables,
  so a name like `sum` never clashes with an operator. An unknown name
  (`\summ`) is an error. Crystal has no such names.
* **WASM `^` with non-integer operands can differ by 1 ULP.** Rust's
  `wasm32-unknown-unknown` `powf` comes from the `libm` crate. The Crystal
  module uses wasi-libc's `pow`, which is correctly rounded here (glibc agrees).
  For example, `1234567.891 ^ -2` gives `6.561000108781381e-13` (Rust WASM,
  0.66 ULP off) versus `6.56100010878138e-13`. This happened once in 33,529
  WASM comparisons; natively the two implementations are identical. Targeting
  `wasm32-wasip1` would probably remove it (untested).
* **WASM errors.** Rust returns `"Error: <message>"`, which is what
  `symbol_wasm.cr` intends. Crystal's wasm32 target cannot unwind, so its
  `rescue` never runs and every error traps. The port matches native Crystal.
* API-level differences only: `Suspended` derives its arity from `Op` instead
  of storing it. An AST can't hold an unknown operator symbol (Crystal would
  build a `Suspended` that never runs). `needs_args` saturates at 0. The AST
  has no `Location`, which Crystal's parser never sets anyway. The unused
  `ExprSuspended` node is not ported.
* `inspect` of exotic non-printable Unicode (e.g. format characters) in
  nested-array output approximates Crystal's `Char#printable?`, because Rust
  exposes no general-category API beyond `is_control`. Never observed in
  testing.
* Extra exports: `symbol_eval_program` (program mode, fresh bindings per call)
  and a no-op `_start`. Hosts written for the Crystal module call `_start`, so
  **the unmodified `symbol/test_wasm.mjs` runs against the Rust module with
  identical output.**

### Rust std semantics that silently differed from Crystal's

All of these live in `src/compat.rs` (210 code lines, half of which is the
sort):

* `char::is_ascii_whitespace` excludes `\v`, and `char::is_whitespace`
  includes U+0085. Both differ from Crystal.
* `str::parse::<f64>` rounds `"1e400"`/`"1e-400"` to inf/0, where Crystal
  rejects them.
* `impl Sum for f64` starts from `-0.0`, so `Σ [-0.0]` would print `-0.0`
  instead of `0.0`.
* `slice::sort_by` (driftsort) can **panic** on a non-total comparator, and
  grading treats NaN as equal to everything. With that comparator it
  panicked on 1,309 of 2,000 random arrays containing NaN, which would abort
  the WASM module. So the old Rust merge sort that Crystal's `Array#sort`
  itself derives from is ported.
* `i64::MIN % -1` panics in Rust, and `as` saturates float→int where Crystal
  raises.
* `{}` prints `5` rather than Crystal's `5.0`, and never uses exponents, so
  `format_float` reproduces `Float64#to_s`.

## WASM build

```
cargo build --profile wasm --target wasm32-unknown-unknown
# -> target/wasm32-unknown-unknown/wasm/symbol.wasm
```

Profile: `opt-level = "s"`, `lto = true`, `codegen-units = 1`,
`panic = "abort"`, `strip = true`. The module has no imports.

| profile | raw | gzip -9 | brotli -q 11 | speed |
|---|---|---|---|---|
| `opt-level = "s"` (chosen) | 188,166 | 71,089 | 61,174 | baseline |
| `opt-level = "z"` | 184,498 | 68,574 | 59,971 | 35–45% slower |
| `opt-level = 3` | 187,787 | 73,658 | 62,827 | — |
| Crystal (`--release -Dwithout_mt`, wasm32-wasi) | 684,208 | 241,049 | 188,995 | |

Where the Rust module's 160 KB of code goes: ~43 KB this crate's own
functions, ~21 KB float formatting (`flt2dec`), ~6 KB float parsing, ~12 KB
other `core::fmt`, ~8 KB dlmalloc, ~3.5 KB HashMap/SipHash, ~3 KB panic
machinery. The remaining ~61 KB is std generic code, much of it monomorphised
for this crate's types. There are also 26 KB of data. `wasm-opt` was not
available, so it wasn't tried.

## Memory in WASM (the key question)

Under Node, each case was run 100,000 times through the C ABI:
`symbol_alloc` input, `symbol_eval`/`symbol_eval_program`, read the result,
then `symbol_free` both. Each case ran in a fresh instance.
`memory.buffer.byteLength` is in MB:

| case | Rust: before → 1k → 10k → 100k | Crystal: before → 1k → 10k → 100k |
|---|---|---|
| `Σ [1, 2, 3, 4]` | 1.06 → 1.13 → 1.13 → 1.13 | 0.19 → 1.94 → 17.31 → **171.13** |
| `x = 3. y = x * 2. Σ [x, y, 10]` (program) | 1.06 → 1.13 → 1.13 → 1.13 | 0.19 → 4.00 → 38.19 → **380.13** |
| `x = 3. y = 9. (x + y) > 10` (program) | 1.06 → 1.13 → 1.13 → 1.13 | 0.19 → 3.88 → 36.81 → **366.44** |
| `(⍋ [30, 10, 20, 50, 40]) @> […]` | 1.06 → 1.13 → 1.13 → 1.13 | 0.19 → 4.75 → 46.00 → **457.94** |
| `Σ [1, 2, 3, 4, 5] * [2, 3, 4, 5, 6]` | 1.06 → 1.13 → 1.13 → 1.13 | 0.19 → 4.19 → 40.31 → **401.50** |
| `Σ 1 .. 100` | 1.06 → 1.13 → 1.13 → 1.13 | 0.19 → 3.19 → 30.13 → **299.25** |

Rust's 1.06 MB baseline is mostly its 1 MB shadow stack. dlmalloc reuses
freed blocks, so memory never grows past the first eval. Crystal's wasm32
target has no GC, so everything is leaked: 1.7–4.6 KB per eval. Neither
module declares a maximum, so Crystal would trap at the engine's 4 GiB wasm32
limit after roughly 0.9–2.4 million evals (extrapolated, not run).

**Robustness under errors.** Crystal on wasm32 can't unwind either. Any raise
(parse error, overflow, index error) hits `unreachable`, and the trap skips
restoring the shadow-stack pointer. After ~4,100 parse errors, ~300 index
errors or ~200 overflow errors, the instance is corrupted: even `Σ [1, 2, 3, 4]`
then fails with `memory access out of bounds`. The Rust module handled
20,000 consecutive errors with memory unchanged.

With nesting 1,000 levels deep, the Rust module evaluates correctly, while
Crystal's overflows its shadow stack and stays corrupted. At 10,000+ levels,
both hit V8's call-stack limit, and Rust's instance keeps working afterwards.
Natively, both crash on 100,000-deep input. That limitation is shared, and an
explicit depth limit would fix it in either implementation.

## Speed

Native, 200,000 evaluations from source text (lex + parse + eval), in ms.
Figures are the median of 3 runs (`cargo run --release --example bench`, and
a Crystal `--release` twin):

| case | Crystal | Rust | ratio |
|---|---|---|---|
| `Σ [1, 2, 3, 4]` | 506 | 86 | 5.9× |
| `(x + y) > 10` with bindings | 511 | 106 | 4.8× |
| program `x = 3. y = x * 2. Σ [x, y, 10]` | 1,395 | 250 | 5.6× |
| `(⍋ [30, 10, 20, 50, 40]) @> […]` | 1,846 | 267 | 6.9× |
| `Σ [1, 2, 3, 4, 5] * [2, 3, 4, 5, 6]` | 1,380 | 215 | 6.4× |
| `Σ 1 .. 100` | 547 | 262 | 2.1× |

WASM under Node, 100,000 evaluations including JS-side marshalling, in ms
(two runs):

| case | Crystal | Rust (`"s"`) | ratio |
|---|---|---|---|
| `Σ [1, 2, 3, 4]` | 217 / 217 | 129 / 127 | 1.7× |
| program `x = 3. y = x * 2. Σ [x, y, 10]` | 514 / 516 | 271 / 277 | 1.9× |
| program `x = 3. y = 9. (x + y) > 10` | 348 / 354 | 242 / 252 | 1.4× |
| `(⍋ …) @> […]` | 870 / 851 | 348 / 354 | 2.5× |
| `Σ [1, 2, 3, 4, 5] * […]` | 605 / 606 | 241 / 249 | 2.5× |
| `Σ 1 .. 100` | 243 / 231 | 201 / 191 | 1.2× |

A caveat on both tables: in WASM, Crystal never frees, so its allocations are
cheap; the gap narrows there. Natively, part of the gap is design, not
language. The Crystal evaluator allocates a heap object per `Resolved` and a
new array per argument (`[x] + args`), and indexes non-ASCII strings by
character in the lexer. A tuned Crystal version would close some of the gap.

## Readability: Crystal vs Rust

Line counts (excluding tests; "code" excludes blank and comment lines, and
Rust's inline `#[cfg(test)]` modules):

| Crystal | lines | code | Rust | lines | code |
|---|---|---|---|---|---|
| symbols.cr | 51 | 28 | lib.rs | 58 | 31 |
| lexer.cr | 391 | 351 | lexer.rs | 320 | 278 |
| parser.cr | 329 | 253 | parser.rs | 182 | 161 |
| ast.cr | 137 | 93 | ast.rs | 208 | 181 |
| tacit/term.cr | 58 | 40 | value.rs | 242 | 189 |
| tacit/eval.cr | 663 | 558 | eval.rs + ops.rs | 469 | 369 |
| statement.cr | 85 | 54 | program.rs | 41 | 32 |
| inline.cr | 263 | 186 | inline.rs | 156 | 124 |
| symbol_wasm.cr | 90 | 58 | wasm.rs | 111 | 67 |
| — | | | error.rs | 55 | 36 |
| — | | | compat.rs | 265 | 210 |
| **total** | 2,067 | 1,621 | | 2,107 | 1,678 |

Without `compat.rs` (Crystal stdlib behaviour that Crystal gets for free), the
Rust code is 1,468 code lines. `ast.rs` and `value.rs` are longer because
rustfmt puts one enum variant per line, and because of the `From`, `Display`
and `inspect` implementations. `parser.rs` and `lexer.rs` are shorter partly
through different factoring (a token→operator table, `eat` helpers), not
only through language.

### 1. Arithmetic dispatch

Crystal:
```crystal
when "+"
  vectorize_binary(values[0], values[1]) { |a, b| num_add(a, b) }
# …
private def num_add(a : TacitValue, b : TacitValue) : TacitValue
  if a.is_a?(Int64) && b.is_a?(Int64)
    (a + b).as(TacitValue)
  else
    (to_float(a) + to_float(b)).as(TacitValue)
  end
end

private def num_div(a : TacitValue, b : TacitValue) : TacitValue
  if a.is_a?(Int64) && b.is_a?(Int64)
    return Float64::INFINITY.as(TacitValue) if b == 0
    if a % b == 0
      a // b
    else
      a.to_f64 / b.to_f64
    end
  else
    fb = to_float(b)
    return Float64::INFINITY.as(TacitValue) if fb == 0.0
    (to_float(a) / fb).as(TacitValue)
  end
end
```

Rust:
```rust
Op::Add => vectorize(&a, &b, |x, y| arith(x, y, i64::checked_add, |x, y| x + y))?,
// …
fn arith(a: &Value, b: &Value, int: fn(i64, i64) -> Option<i64>, float: fn(f64, f64) -> f64) -> Result<Value> {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => int(*x, *y).map(Value::Int).ok_or(Error::Overflow),
        _ => Ok(Value::Float(float(a.to_float(), b.to_float()))),
    }
}

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
```

Crystal reads more naturally (`a + b`, `a // b`), but its failure modes are
invisible. `+`, `%` and `//` can each raise, and those hidden exceptions are
exactly what traps the WASM build. Rust spells every failure out
(`checked_add`, `ok_or`, `?`), which is noisier but visible, and the `match`
on `(a, b)` replaces `is_a?` plus 107 `.as(TacitValue)` casts in `eval.cr`.

### 2. The right-to-left fold

This is the evaluator as first ported. On 2026-09-30 the stack-based
poly-fix evaluator replaced it (see "Where Rust differs from Crystal").

Crystal:
```crystal
def eval_terms(terms : Array(AST::ExprTerm), bindings : Bindings) : EvalResult
  return Resolved.new(nil) if terms.empty?
  result : EvalResult = Resolved.new(nil)
  terms.reverse_each do |term|
    result = apply_term(term, result, bindings)
  end
  result
end

private def apply_value(value : TacitValue, current : EvalResult) : EvalResult
  case current
  when Resolved
    Resolved.new(value)          # (two identical branches in the original)
  when Suspended
    new_args = [Resolved.new(value).as(EvalResult)] + current.args
    new_suspended = Suspended.new(current.op, current.arity, new_args)
    if new_suspended.complete?
      execute(new_suspended)
    else
      new_suspended
    end
  when Unbound
    current
  else
    Resolved.new(value)
  end
end
```

Rust:
```rust
fn eval_terms(&self, terms: &[Term]) -> Result<EvalResult> {
    terms.iter().rev().try_fold(NOTHING, |acc, term| self.apply_term(term, acc))
}

fn apply_value(value: Value, acc: EvalResult) -> Result<EvalResult> {
    match acc {
        EvalResult::Suspended(mut pending) => {
            pending.args.insert(0, EvalResult::Resolved(value));
            if pending.is_complete() { execute(pending) } else { Ok(pending.into()) }
        }
        // Nothing combines with an unbound variable, so the value is dropped.
        EvalResult::Unbound(_) => Ok(acc),
        // Adjacent values with no operator between them: the leftmost wins.
        EvalResult::Resolved(_) => Ok(EvalResult::Resolved(value)),
    }
}
```

Here Rust is clearer. The `match` is exhaustive, so there's no defensive
`else` for impossible subclasses. Owning `acc` lets it extend the pending
computation in place instead of allocating a new one. `try_fold` is denser
than the loop, though a `for` loop with `?` would be just as idiomatic.

### 3. A lexer case

Crystal:
```crystal
when '<'
  if peek == '-' && peek_next == '>'
    advance; advance
    Token.new(TokenType::RemoveBoth, "<->", @line, start_col)
  elsif peek == '-'
    advance
    Token.new(TokenType::RemoveFront, "<-", @line, start_col)
  elsif peek == '>'
    advance
    Token.new(TokenType::Wrap, "<>", @line, start_col)
  # … four more branches …
  else
    Token.new(TokenType::LessThan, "<", @line, start_col)
  end
```

Rust:
```rust
'<' if self.eat_pair('-', '>') => K::RemoveBoth,
'<' if self.eat('-') => K::RemoveFront,
'<' if self.eat('>') => K::Wrap,
'<' if self.eat('+') => K::Snoc,
'<' if self.eat('~') => K::Piz,
'<' if self.eat('@') => K::IndexLeft,
'<' if self.eat('=') => K::LessEq,
'<' => K::LessThan,
```

Half of this is language: Crystal's `case/when` has no guards. The other half
is factoring: the token text is taken from the consumed slice once, and
Crystal could do the same.

### Where Rust is noisier, candidly

* Collection plumbing in `execute`:
  `pending.args.iter().map(resolve_arg).collect::<Result<_>>()?`, then
  `.collect::<Option<Vec<_>>>()`, where Crystal has `args.map { … }` and
  `args.any?(&.nil?)`.
* Ownership choices everywhere: `into_array(self)` versus borrowing, 11
  `.clone()`s, and `&a, &b`.
* `wasm.rs` needs `unsafe`, `Layout`, and a size header, because `dealloc`
  needs the size. Crystal just says `Pointer(UInt8).malloc`.
* rustfmt's vertical enums, plus boilerplate `Display` and `From`
  implementations.
* The `compat.rs` tax of matching another language's stdlib, which would
  recur in any faithful port.

### Where Rust is clearer

* Exhaustive `match` on an `Op` enum: an operator without an implementation
  is a compile error. Crystal dispatches on strings (`when "Σ", "sum"`) and
  falls through to `else suspended`.
* A closed `Value` enum instead of union casts.
* Failure in signatures (`Result`) instead of hidden exceptions.
* Slice patterns, e.g. assignment detection in program mode is
  `[name, assign, expr @ ..] if …`.

## Reproducing

```
cargo test                                            # everything
cargo run --release --example bench                   # native speed
cargo run --release --example gen_corpus -- 100000 7 > corpus.txt
cargo run --release --example diff_driver -- corpus.txt > rust.out
```

The Crystal-side harness is in `bench/` (copied out of the experiment's
temporary scratch directory):

* `bench/diff_driver.cr`: the Crystal twin of `tests/support/mod.rs`; it
  produced `tests/fixtures/*.expected`.
* `bench/bench.cr`: native Crystal speed.
* `bench/symbol_wasm_ext.cr`: the unmodified `symbol_wasm.cr` plus a
  `symbol_eval_program` export.
* `bench/wasm_bench.mjs` (`node wasm_bench.mjs <module.wasm> <case> <n>`) and
  `bench/wasm_deep.mjs` (`node wasm_deep.mjs <module.wasm>...`): WASM memory,
  speed, error and deep-nesting tests.
* `bench/stdcheck.rs`: the probe of Rust std vs Crystal stdlib behaviour that
  led to `src/compat.rs`.

The Crystal files `require "symbols"`; build them from `../symbol-cr` (or with
`CRYSTAL_PATH` pointing at it).

Build the Crystal WASM with
`crystal build --release -Dwithout_mt --target wasm32-wasi --link-flags="-L/usr/share/wasi-sysroot/lib/wasm32-wasi" src/symbol_wasm.cr`.
