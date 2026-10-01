# SYMBOL

**SYMBOL** (Set-Yielding Model of Bound Operations and Logic) is a small
expression language in the spirit of APL. Its operators are symbols,
expressions read from right to left with no precedence rules, and an
operator can stand before, between or after its arguments.

```symbol
Σ [3, 1, 4, 1, 5]            ⇒ 14
(Σ xs) / # xs                ⇒ 2.8
(⍋ xs) @> xs                 ⇒ [1, 1, 3, 4, 5]
[1, 2, 3] * 10               ⇒ [10, 20, 30]
price * qty +- 5             ⇒ -40
(price * qty) +- 5           ⇒ 55
```

The examples on this page use `xs = [3, 1, 4, 1, 5]`, `price = 20` and
`qty = 3`, and every one of them is checked by the test suite.

SYMBOL is the expression language of HYMNAL, a logic language written as
markup (in development), where it computes and checks the values in rules.
It works on its own too: embedded in a Rust program, in `{{ }}` text
templates, or as a 191 KB WebAssembly module with no imports. The
implementation uses only Rust's standard library.

> **Status: early.** The language is still settling on its way to 1.0, and
> some of its syntax will change. SYMBOL began in Crystal
> ([symbol-cr](https://github.com/tabcomputing/symbol-cr)); this Rust
> implementation is the one going forward.

## Reading an expression

**Right to left, without precedence.** An operator takes everything to its
right as its right argument, so `2 * 3 + 4` is `2 * (3 + 4)`. Parentheses
group as usual.

```symbol
2 * 3 + 4                    ⇒ 14
(2 * 3) + 4                  ⇒ 10
```

**Operators go anywhere.** SYMBOL borrows from three traditions. Write
`10 +- 3` with the operator between its arguments, as in APL; `10 3 +-`
with it after them, as in Forth; or `+- 10 3` before them, as in the Polish
notation Lisp grew from. All three are 7, and the arguments keep their
order wherever the operator stands. Every operator takes exactly one or two
arguments, so no form needs parentheses to say what belongs to what.

```symbol
+- 10 3                      ⇒ 7
10 +- 3                      ⇒ 7
10 3 +-                      ⇒ 7
2 3 4 + *                    ⇒ 14
```

Reading from the right, an operator takes the finished values to its right,
up to the number it needs, and waits for the rest from its left. A value
goes first to a waiting operator on its right, so `2 3 4 + *` is
`2 * (3 + 4)`. An operator runs as soon as it has its arguments, and its
result is a value like any other.

A good default style is APL's: a two-argument operator between its
arguments and a one-argument operator before its argument (`a +- b`,
`Σ xs`). The other forms are there where they read better, such as a
postfix chain or a partial application like `(1 +)`.

**An unfinished expression is a partial application,** not an error. In
parentheses, the rest of the expression can finish it:

```symbol
1 +                          ⇒ partial: + waits for an argument
(1 +) 2                      ⇒ 3
1 2                          ⇒ partial: two values wait for an operator
(1 2) +                      ⇒ 3
```

**A variable without a value** leaves the computation waiting for it. The
result is partial, and evaluating again once the variable is bound
finishes it.

```symbol
total + 1                    ⇒ partial: waits for total
```

## Values

**Numbers** are 64-bit integers or floating point. Integer arithmetic stays
integral when the result is exact, and an integer overflow is an error. A
`-` directly before a digit makes a negative number, as in `-3`.

```symbol
8 / 2                        ⇒ 4
7 / 2                        ⇒ 3.5
2 ^ 10                       ⇒ 1024
9223372036854775807 + 1      ⇒ error: Arithmetic overflow
```

**Strings** are written in double quotes, with the escapes `\n`, `\t`, `\"`
and `\\`. To the list operators, a string is the list of its characters.

**Booleans** are `true` and `false`, or `⊤` and `⊥`.

**Lists** are written in square brackets. Commas separate the elements, and
each element can be an expression; plain values can also be separated by
spaces.

```symbol
[1 + 1, 2 * 3]               ⇒ [2, 6]
[1 2 3]                      ⇒ [1, 2, 3]
```

**nil** means no value: an index past the end of a list gives nil, for
example. An operator given nil waits, as for a variable without a value.

**Names** of variables are made of letters, digits and `_`, starting with a
letter or `_`. A `-` may join two parts of a name, as in `data-id`, which is
why subtraction is written `+-` (below).

## Operators

Each operator is a symbol. Most can also be typed by a LaTeX-style name,
such as `\sum` for `Σ`. A word without a backslash is always a variable, so
a variable can be called `sum`.

### Arithmetic

| Operator | Arguments | Meaning |
|---|---|---|
| `+` | 2 | add |
| `+-` | 2 | subtract: `a +- b` is a + (−b) |
| `-+` | 2 | subtract the other way round: `a -+ b` is (−a) + b |
| `*` | 2 | multiply |
| `/` | 2 | divide (by zero gives `Infinity`) |
| `%` | 2 | remainder (by zero is an error) |
| `^` | 2 | power |
| `-` | 1 | negate |

`-` never subtracts. It negates, and a `-` directly after a value (`3-3`) is
an error. Arithmetic works element by element when either side is a list.

```symbol
10 +- 3                      ⇒ 7
10 -+ 3                      ⇒ -7
- 5                          ⇒ -5
[2, 3, 4] * [10, 20, 30]     ⇒ [20, 60, 120]
-[1, 2]                      ⇒ [-1, -2]
7 % 0                        ⇒ error: Division by 0
```

### Comparison

| Operator | Name | Meaning |
|---|---|---|
| `==` | | equal |
| `!=`, `≠` | `\neq`, `\ne` | not equal |
| `<`, `>` | | less than, greater than |
| `<=`, `≤` | `\leq`, `\le` | less than or equal |
| `>=`, `≥` | `\geq`, `\ge` | greater than or equal |

`==` and `!=` compare any two values, lists element by element. The ordering
comparisons compare numbers.

```symbol
3 < 5                        ⇒ true
[1, 2] == [1, 2]             ⇒ true
1 \neq 2                     ⇒ true
```

### Logic and bits

| Operator | Arguments | Meaning |
|---|---|---|
| `!` | 1 | not: true for `false`, `0`, `""`, `[]` and nil |
| `[+]` | 2 | or |
| `[*]` | 2 | and |
| `[-]` | 2 | exclusive or |
| `[~]` | 1 | not |

The bracketed operators are logic on booleans and bitwise on integers
(element by element on lists).

```symbol
! true                       ⇒ false
true [+] false               ⇒ true
⊤ [*] ⊥                      ⇒ false
6 [*] 3                      ⇒ 2
6 [-] 3                      ⇒ 5
[~] 5                        ⇒ -6
```

### Aggregates

These take one argument.

| Operator | Name | Meaning |
|---|---|---|
| `Σ` | `\sum` | sum |
| `Π` | `\prod` | product |
| `#` | `\count` | count: the length of a list or a string |
| `⌈` | `\max`, `\lceil` | the maximum of a list; the ceiling of a number |
| `⌊` | `\min`, `\lfloor` | the minimum of a list; the floor of a number |

```symbol
Σ xs                         ⇒ 14
Π [1, 2, 3, 4]               ⇒ 24
# "hello"                    ⇒ 5
⌈ xs                         ⇒ 5
⌈ 2.5                        ⇒ 3
\sum [1, 2, 3]               ⇒ 6
```

### Lists

| Operator | Name | Arguments | Meaning |
|---|---|---|---|
| `..` | | 2 | range, inclusive; it counts down if the start is larger |
| `><` | | 2 | concatenate |
| `<>` | | 2 | pair: a list of the two |
| `+>` | | 2 | add an element at the front |
| `<+` | | 2 | add an element at the end |
| `~>` | | 2 | interleave |
| `<~` | | 2 | interleave, the right list first |
| `->` | | 2 | remove a suffix, if the list ends with it |
| `<-` | | 2 | remove a prefix, if the list starts with it |
| `<->` | | 2 | remove from both ends |
| `↑` | `\uparrow` | 2 | take the first n elements (the last n if negative) |
| `↓` | `\downarrow` | 2 | drop the first n elements (the last n if negative) |
| `@>` | | 2 | index, the index on the left; it counts from 1, and from the end if negative |
| `<@` | | 2 | index, the list on the left |
| `⌽` | `\reverse` | 1 | reverse |
| `⍋` | `\gradeup` | 1 | grade up: the indices that sort the list ascending |
| `⍒` | `\gradedown` | 1 | grade down: the indices that sort it descending |

The arrows point the way the elements move: `+>` adds at the front, `<+` at
the end, `->` removes from the end and `<-` from the front. Indexing with a
list of indices gives a list, so grading and then indexing sorts, as in the
example at the top.

```symbol
1 .. 5                       ⇒ [1, 2, 3, 4, 5]
5 .. 1                       ⇒ [5, 4, 3, 2, 1]
[1, 2] >< [3, 4]             ⇒ [1, 2, 3, 4]
0 +> [1, 2]                  ⇒ [0, 1, 2]
[1, 2] <+ 3                  ⇒ [1, 2, 3]
[1, 2, 3] ~> [10, 20, 30]    ⇒ [1, 10, 2, 20, 3, 30]
[1, 2, 3, 4] -> [3, 4]       ⇒ [1, 2]
2 ↑ [1, 2, 3, 4]             ⇒ [1, 2]
-2 ↑ [1, 2, 3, 4]            ⇒ [3, 4]
2 @> [10, 20, 30]            ⇒ 20
-1 @> [10, 20, 30]           ⇒ 30
0 @> [10, 20, 30]            ⇒ nil
⍋ [30, 10, 20]               ⇒ [2, 3, 1]
⌽ "abc"                      ⇒ ["c", "b", "a"]
```

`?` is reserved: it parses, but has no meaning yet.

## Programs

In a program, statements are separated by `.`, and `name = value` assigns to
a variable. The result is the last statement's. A line break counts as a
space, so a statement ends only at its `.`.

```symbol-program
x = 3. y = x * 2. y + 1      ⇒ 7
x = 4. 5 + x                 ⇒ 9
```

A `.` between two digits is a decimal point, so `4.5` is a number: leave a
space after the `.` when the next statement starts with a digit.

## Templates

`symbol::inline` fills in each `{{ expression }}` in a text. It evaluates
them as a program, so a template can assign a variable and use it later.
Code spans and fenced code blocks are copied as they are, `\{{` writes a
literal `{{`, and an expression that fails or doesn't finish is left as
written.

```rust
use symbol::Bindings;

let mut bindings = Bindings::new();
let text = symbol::inline("{{ n = 3 }} squared is {{ n * n }}.", &mut bindings);
assert_eq!(text, "3 squared is 9.");
```

## Using SYMBOL from Rust

SYMBOL isn't on crates.io yet. Depend on it from GitHub:

```toml
[dependencies]
symbol = { package = "symbol-rs", git = "https://github.com/tabcomputing/symbol" }
```

```rust
use symbol::{Bindings, EvalResult, Value};

let mut bindings = Bindings::new();
bindings.insert("xs".into(), Value::from(vec![3i64, 1, 4, 1, 5]));

let result = symbol::eval("(Σ xs) / # xs", &bindings).unwrap();
assert_eq!(result, EvalResult::Resolved(Value::Float(2.8)));

// Parse once, evaluate many times
let square = symbol::parse("x * x").unwrap();
for x in 1..=3 {
    bindings.insert("x".into(), Value::Int(x));
    let result = symbol::evaluate(&square, &bindings).unwrap();
    assert_eq!(result, EvalResult::Resolved(Value::Int(x * x)));
}

// A program assigns into the bindings
symbol::eval_program("total = Σ xs", &mut bindings).unwrap();
assert_eq!(bindings["total"], Value::Int(14));
```

A result is `EvalResult::Resolved` with a value; `Suspended`, an operator
waiting for arguments or for a variable; `Unbound`, a variable on its own;
or `Sequence`, parts that didn't combine. Errors are values of
`symbol::Error`, such as a remainder by zero, an integer overflow, or a
parse error with its line and column. `Value` derives `PartialEq`, which is exact
(`Int(1)` isn't `Float(1.0)`); SYMBOL's own `==` is `Value::loose_eq`.

## Using SYMBOL from JavaScript

Build the WebAssembly module, with the target installed
(`rustup target add wasm32-unknown-unknown`):

```sh
cargo build --profile wasm --target wasm32-unknown-unknown
# target/wasm32-unknown-unknown/wasm/symbol.wasm: 191 KB, 71 KB gzipped
```

The module exports `symbol_alloc`, `symbol_free`, `symbol_eval` and
`symbol_eval_program`. Strings cross as NUL-terminated UTF-8, and every
buffer, from `symbol_alloc` or returned by an evaluation, is released with
`symbol_free`. An evaluation returns text: the value, or `Error: ` and the
message.

```js
import { readFile } from "node:fs/promises";

const { instance } = await WebAssembly.instantiate(await readFile("symbol.wasm"));
const { memory, symbol_alloc, symbol_eval, symbol_free } = instance.exports;

function evaluate(source) {
  const bytes = new TextEncoder().encode(source);
  const input = symbol_alloc(bytes.length + 1);
  new Uint8Array(memory.buffer, input, bytes.length + 1).set([...bytes, 0]);
  const output = symbol_eval(input);
  const view = new Uint8Array(memory.buffer, output);
  const text = new TextDecoder().decode(view.subarray(0, view.indexOf(0)));
  symbol_free(input);
  symbol_free(output);
  return text;
}

console.log(evaluate("Σ [1, 2, 3, 4]")); // 10
console.log(evaluate("[1, 2, 3] * 10")); // [10, 20, 30]
console.log(evaluate("1 % 0"));          // Error: Division by 0
```

## Development

```sh
cargo test                            # specs, golden files, and this README's examples
cargo run --release --example bench   # speed
```

`tests/fixtures/*.expected` hold the expected output for about 3,500
expressions and programs. After a deliberate change, review the differences,
then rewrite them with `BLESS=1 cargo test --test differential`.
[docs/crystal-port.md](docs/crystal-port.md) tells the story of the port from
Crystal.

## License

MIT
