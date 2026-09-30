//! Port of spec/symbol_spec.cr. One `#[test]` per Crystal `it`.
//!
//! `Value`'s `PartialEq` is type-exact, so `assert_eq!(.., Value::Int(5))`
//! covers both of Crystal's `should be_a(Int64)` and `should eq(5)`.

#![allow(clippy::approx_constant)] // "3.14" is the literal the Crystal spec uses

use symbol::{Bindings, EvalResult, Value};

fn eval(source: &str) -> Value {
    eval_with(source, &Bindings::new())
}

fn eval_with(source: &str, bindings: &Bindings) -> Value {
    match symbol::eval(source, bindings).unwrap() {
        EvalResult::Resolved(value) => value,
        other => panic!("expected a resolved value, got {other}"),
    }
}

fn eval_result(source: &str) -> EvalResult {
    symbol::eval(source, &Bindings::new()).unwrap()
}

fn bindings<const N: usize>(pairs: [(&str, Value); N]) -> Bindings {
    pairs.into_iter().map(|(k, v)| (k.to_owned(), v)).collect()
}

fn ints(items: &[i64]) -> Value {
    Value::from(items.to_vec())
}

#[test]
fn has_a_version() {
    assert_eq!(symbol::VERSION, "0.2.0");
}

mod arithmetic {
    use super::*;

    #[test]
    fn adds_integers() {
        assert_eq!(eval("2 + 3"), Value::Int(5));
    }

    #[test]
    fn subtracts_integers() {
        assert_eq!(eval("10 +- 3"), Value::Int(7));
    }

    #[test]
    fn multiplies_integers() {
        assert_eq!(eval("4 * 5"), Value::Int(20));
    }

    #[test]
    fn divides_integers_exactly() {
        assert_eq!(eval("20 / 4"), Value::Int(5));
    }

    #[test]
    fn divides_integers_inexactly_to_float() {
        assert_eq!(eval("7 / 2"), Value::Float(3.5));
    }

    #[test]
    fn modulo_integers() {
        assert_eq!(eval("7 % 3"), Value::Int(1));
    }

    #[test]
    fn exponentiation_integers() {
        assert_eq!(eval("2 ^ 3"), Value::Int(8));
    }

    #[test]
    fn division_by_zero_returns_infinity() {
        assert_eq!(eval("1 / 0"), Value::Float(f64::INFINITY));
    }

    #[test]
    fn evaluates_a_single_integer_literal() {
        assert_eq!(eval("42"), Value::Int(42));
    }

    #[test]
    fn evaluates_a_single_float_literal() {
        assert_eq!(eval("3.14"), Value::Float(3.14));
    }

    #[test]
    fn handles_negative_numbers() {
        assert_eq!(eval("-5 + 3"), Value::Int(-2));
    }
}

mod type_promotion {
    use super::*;

    #[test]
    fn int_plus_float_is_float() {
        assert_eq!(eval("2 + 3.5"), Value::Float(5.5));
    }

    #[test]
    fn float_plus_int_is_float() {
        assert_eq!(eval("3.5 + 2"), Value::Float(5.5));
    }

    #[test]
    fn float_plus_float_is_float() {
        assert_eq!(eval("1.5 + 2.5"), Value::Float(4.0));
    }

    #[test]
    fn int_times_float_is_float() {
        assert_eq!(eval("3 * 2.0"), Value::Float(6.0));
    }

    #[test]
    fn float_binding_plus_int_literal_is_float() {
        assert_eq!(eval_with("k + 1", &bindings([("k", Value::Float(5.0))])), Value::Float(6.0));
    }
}

mod comparison {
    use super::*;

    #[test]
    fn equal_true() {
        assert_eq!(eval("5 == 5"), Value::Bool(true));
    }

    #[test]
    fn equal_false() {
        assert_eq!(eval("5 == 3"), Value::Bool(false));
    }

    #[test]
    fn not_equal() {
        assert_eq!(eval("5 != 3"), Value::Bool(true));
    }

    #[test]
    fn less_than() {
        assert_eq!(eval("3 < 5"), Value::Bool(true));
    }

    #[test]
    fn greater_than() {
        assert_eq!(eval("5 > 3"), Value::Bool(true));
    }

    #[test]
    fn less_than_or_equal() {
        assert_eq!(eval("3 <= 3"), Value::Bool(true));
    }

    #[test]
    fn greater_than_or_equal() {
        assert_eq!(eval("5 >= 6"), Value::Bool(false));
    }
}

mod logic {
    use super::*;

    #[test]
    fn negates_falsy() {
        assert_eq!(eval("! 0"), Value::Bool(true));
    }

    #[test]
    fn negates_truthy() {
        assert_eq!(eval("! 1"), Value::Bool(false));
    }

    #[test]
    fn unary_negation_preserves_int() {
        assert_eq!(eval("- 5"), Value::Int(-5));
    }

    #[test]
    fn unary_negation_preserves_float() {
        assert_eq!(eval("- 5.0"), Value::Float(-5.0));
    }
}

mod variables {
    use super::*;

    #[test]
    fn resolves_a_bound_float_variable() {
        assert_eq!(eval_with("k + 1", &bindings([("k", Value::Float(5.0))])), Value::Float(6.0));
    }

    #[test]
    fn resolves_a_bound_int_variable() {
        assert_eq!(eval_with("k + 1", &bindings([("k", Value::Int(5))])), Value::Int(6));
    }

    #[test]
    fn resolves_multiple_variables() {
        let b = bindings([("x", Value::Int(3)), ("y", Value::Int(4))]);
        assert_eq!(eval_with("x + y", &b), Value::Int(7));
    }

    #[test]
    fn returns_unbound_for_missing_variable() {
        assert_eq!(eval_result("unknown"), EvalResult::Unbound("unknown".into()));
    }
}

mod ranges {
    use super::*;

    #[test]
    fn ascending_range_produces_integers() {
        assert_eq!(eval("1 .. 5"), ints(&[1, 2, 3, 4, 5]));
    }

    #[test]
    fn descending_range() {
        assert_eq!(eval("5 .. 1"), ints(&[5, 4, 3, 2, 1]));
    }

    #[test]
    fn single_element_range() {
        assert_eq!(eval("3 .. 3"), ints(&[3]));
    }
}

mod lists {
    use super::*;

    #[test]
    fn evaluates_a_list_of_integers() {
        assert_eq!(eval("[1, 2, 3]"), ints(&[1, 2, 3]));
    }

    #[test]
    fn evaluates_a_list_with_mixed_types() {
        assert_eq!(eval("[1, 2.5, 3]"), Value::Array(vec![Value::Int(1), Value::Float(2.5), Value::Int(3)]));
    }

    #[test]
    fn evaluates_an_empty_list() {
        assert_eq!(eval("[]"), Value::Array(vec![]));
    }
}

mod aggregation {
    use super::*;

    #[test]
    fn sum_of_integers() {
        assert_eq!(eval("Σ [1, 2, 3, 4]"), Value::Int(10));
    }

    #[test]
    fn sum_of_mixed_promotes_to_float() {
        assert!(matches!(eval("Σ [1, 2.0, 3]"), Value::Float(_)));
    }

    #[test]
    fn product_of_integers() {
        assert_eq!(eval("Π [1, 2, 3, 4]"), Value::Int(24));
    }

    #[test]
    fn count_returns_integer() {
        assert_eq!(eval("# [10, 20, 30]"), Value::Int(3));
    }

    #[test]
    fn max_of_integer_array() {
        assert_eq!(eval("⌈ [3, 1, 4, 1, 5]"), Value::Int(5));
    }

    #[test]
    fn min_of_integer_array() {
        assert_eq!(eval("⌊ [3, 1, 4, 1, 5]"), Value::Int(1));
    }
}

mod ceiling_floor_scalar {
    use super::*;

    #[test]
    fn ceiling_of_float() {
        assert_eq!(eval("⌈ 3.2"), Value::Int(4));
    }

    #[test]
    fn ceiling_of_negative_float() {
        assert_eq!(eval("⌈ -2.7"), Value::Int(-2));
    }

    #[test]
    fn ceiling_of_integer_is_identity() {
        assert_eq!(eval("⌈ 5"), Value::Int(5));
    }

    #[test]
    fn floor_of_float() {
        assert_eq!(eval("⌊ 3.8"), Value::Int(3));
    }

    #[test]
    fn floor_of_negative_float() {
        assert_eq!(eval("⌊ -2.3"), Value::Int(-3));
    }

    #[test]
    fn floor_of_integer_is_identity() {
        assert_eq!(eval("⌊ 5"), Value::Int(5));
    }
}

mod structural_operators {
    use super::*;

    #[test]
    fn concat() {
        assert_eq!(eval("[1, 2] >< [3, 4]"), ints(&[1, 2, 3, 4]));
    }

    #[test]
    fn wrap() {
        assert_eq!(eval("1 <> 2"), ints(&[1, 2]));
    }

    #[test]
    fn cons_prepends() {
        assert_eq!(eval("0 +> [1, 2, 3]"), ints(&[0, 1, 2, 3]));
    }

    #[test]
    fn snoc_appends() {
        assert_eq!(eval("[1, 2, 3] <+ 4"), ints(&[1, 2, 3, 4]));
    }

    #[test]
    fn index_is_one_based() {
        assert_eq!(eval("2 @> [10, 20, 30]"), Value::Int(20));
    }

    #[test]
    fn negative_index_counts_from_end() {
        assert_eq!(eval("-1 @> [10, 20, 30]"), Value::Int(30));
    }

    #[test]
    fn index_with_array_on_left() {
        assert_eq!(eval("[10, 20, 30] <@ 2"), Value::Int(20));
    }

    #[test]
    fn index_with_array_of_indices() {
        assert_eq!(eval("[2, 3, 1] @> [10, 20, 30]"), ints(&[20, 30, 10]));
    }

    #[test]
    fn grade_up_returns_one_based_indices() {
        assert_eq!(eval("⍋ [30, 10, 20]"), ints(&[2, 3, 1]));
    }

    #[test]
    fn grade_down_returns_one_based_indices() {
        assert_eq!(eval("⍒ [30, 10, 20]"), ints(&[1, 3, 2]));
    }

    #[test]
    fn grade_up_then_index_sorts_ascending() {
        assert_eq!(eval("(⍋ [30, 10, 20]) @> [30, 10, 20]"), ints(&[10, 20, 30]));
    }

    #[test]
    fn grade_down_then_index_sorts_descending() {
        assert_eq!(eval("(⍒ [30, 10, 20]) @> [30, 10, 20]"), ints(&[30, 20, 10]));
    }

    #[test]
    fn reverse() {
        assert_eq!(eval("⌽ [1, 2, 3]"), ints(&[3, 2, 1]));
    }

    #[test]
    fn take() {
        assert_eq!(eval("2 ↑ [10, 20, 30, 40]"), ints(&[10, 20]));
    }

    #[test]
    fn drop() {
        assert_eq!(eval("2 ↓ [10, 20, 30, 40]"), ints(&[30, 40]));
    }

    #[test]
    fn zip() {
        assert_eq!(eval("[1, 2, 3] ~> [4, 5, 6]"), ints(&[1, 4, 2, 5, 3, 6]));
    }
}

mod vectorization {
    use super::*;

    #[test]
    fn adds_scalar_to_int_array() {
        assert_eq!(eval("[1, 2, 3] + 10"), ints(&[11, 12, 13]));
    }

    #[test]
    fn multiplies_int_arrays_element_wise() {
        assert_eq!(eval("[2, 3, 4] * [10, 20, 30]"), ints(&[20, 60, 120]));
    }

    #[test]
    fn scalar_minus_int_array() {
        assert_eq!(eval("10 +- [1, 2, 3]"), ints(&[9, 8, 7]));
    }

    #[test]
    fn int_array_plus_float_scalar_promotes_to_float() {
        assert_eq!(eval("[1, 2, 3] + 0.5"), Value::from(vec![1.5, 2.5, 3.5]));
    }
}

mod grouped_expressions {
    use super::*;

    #[test]
    fn evaluates_parenthesized_sub_expression() {
        assert_eq!(eval("2 * (3 + 4)"), Value::Int(14));
    }
}

mod partial_application {
    use super::*;

    #[test]
    fn creates_suspended_computation_for_missing_left_arg() {
        let EvalResult::Suspended(suspended) = eval_result("+ 5") else { panic!("expected Suspended") };
        assert_eq!(suspended.op.symbol(), "+");
        assert_eq!(suspended.needs_args(), 1);
    }
}

mod boolean_literals {
    use super::*;

    #[test]
    fn true_literal() {
        assert_eq!(eval("true"), Value::Bool(true));
    }

    #[test]
    fn false_literal() {
        assert_eq!(eval("false"), Value::Bool(false));
    }
}

mod strings {
    use super::*;

    #[test]
    fn evaluates_a_string_literal() {
        assert_eq!(eval("\"hello\""), Value::from("hello"));
    }
}

mod bitwise {
    use super::*;

    #[test]
    fn boolean_or() {
        assert_eq!(eval("true [+] false"), Value::Bool(true));
    }

    #[test]
    fn boolean_and() {
        assert_eq!(eval("true [*] false"), Value::Bool(false));
    }

    #[test]
    fn boolean_xor() {
        assert_eq!(eval("true [-] true"), Value::Bool(false));
    }

    #[test]
    fn bitwise_or_returns_int() {
        assert_eq!(eval("5 [+] 3"), Value::Int(7));
    }

    #[test]
    fn bitwise_and_returns_int() {
        assert_eq!(eval("5 [*] 3"), Value::Int(1));
    }
}
