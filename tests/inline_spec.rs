//! Port of spec/inline_spec.cr: `{{ expr }}` templates.

use symbol::{Bindings, EvalResult, Value};

fn inline(text: &str) -> String {
    symbol::inline(text, &mut Bindings::new())
}

fn inline_with<const N: usize>(text: &str, pairs: [(&str, Value); N]) -> String {
    let mut bindings = pairs.into_iter().map(|(k, v)| (k.to_owned(), v)).collect();
    symbol::inline(text, &mut bindings)
}

mod basic_evaluation {
    use super::*;

    #[test]
    fn evaluates_a_simple_expression() {
        assert_eq!(inline("x is {{ 2 + 3 }}"), "x is 5");
    }

    #[test]
    fn evaluates_with_variable_bindings() {
        assert_eq!(inline_with("{{ x }}", [("x", Value::Int(10))]), "10");
    }

    #[test]
    fn evaluates_multiple_expressions() {
        let text = inline_with("{{ a }} and {{ b }}", [("a", Value::Int(1)), ("b", Value::Int(2))]);
        assert_eq!(text, "1 and 2");
    }

    #[test]
    fn passes_through_plain_text_unchanged() {
        assert_eq!(inline("plain text"), "plain text");
    }

    #[test]
    fn handles_adjacent_expressions() {
        assert_eq!(inline("{{ 1 }}{{ 2 }}"), "12");
    }

    #[test]
    fn trims_whitespace_in_expressions() {
        assert_eq!(inline("{{  2 + 3  }}"), "5");
    }
}

mod result_formatting {
    use super::*;

    #[test]
    fn formats_integers_without_decimal() {
        assert_eq!(inline("{{ 5 }}"), "5");
    }

    #[test]
    fn formats_floats_with_decimal() {
        assert_eq!(inline("{{ 5.0 }}"), "5.0");
    }

    #[test]
    fn keeps_decimal_for_fractional_values() {
        assert_ne!(inline("{{ 1 / 3 }}"), "0");
    }

    #[test]
    fn formats_arrays() {
        assert_eq!(inline("{{ [1, 2, 3] }}"), "[1, 2, 3]");
    }

    #[test]
    fn formats_booleans() {
        assert_eq!(inline("{{ 5 == 5 }}"), "true");
        assert_eq!(inline("{{ 5 == 3 }}"), "false");
    }

    #[test]
    fn formats_strings() {
        assert_eq!(inline(r#"{{ "hello" }}"#), "hello");
    }
}

mod code_span_passthrough {
    use super::*;

    #[test]
    fn does_not_evaluate_inside_code_spans() {
        assert_eq!(inline("use `{{ expr }}` in code"), "use `{{ expr }}` in code");
    }

    #[test]
    fn handles_double_backtick_code_spans() {
        assert_eq!(inline("use `` {{ expr }} `` here"), "use `` {{ expr }} `` here");
    }

    #[test]
    fn evaluates_outside_code_spans() {
        assert_eq!(inline("`code` and {{ 2 + 3 }}"), "`code` and 5");
    }
}

mod code_fence_passthrough {
    use super::*;

    #[test]
    fn does_not_evaluate_inside_code_fences() {
        let text = "before\n```\n{{ 2 + 3 }}\n```\nafter {{ 1 }}";
        assert_eq!(inline(text), "before\n```\n{{ 2 + 3 }}\n```\nafter 1");
    }

    #[test]
    fn handles_code_fences_with_language_tag() {
        let text = "```crystal\n{{ x }}\n```";
        assert_eq!(inline(text), text);
    }

    #[test]
    fn handles_code_fences_with_more_than_3_backticks() {
        let text = "````\n{{ x }}\n````";
        assert_eq!(inline(text), text);
    }
}

mod escaping {
    use super::*;

    #[test]
    fn emits_literal_braces_for_escaped_expression() {
        assert_eq!(inline("literal \\{{ not eval }}"), "literal {{ not eval }}");
    }

    #[test]
    fn does_not_evaluate_escaped_expressions() {
        assert_eq!(inline("\\{{ 2 + 3 }}"), "{{ 2 + 3 }}");
    }
}

mod error_handling {
    use super::*;

    #[test]
    fn leaves_unbound_variables_unchanged() {
        assert_eq!(inline("{{ unknown }}"), "{{ unknown }}");
    }

    #[test]
    fn leaves_parse_errors_unchanged() {
        assert_eq!(inline("{{ 2 + + }}"), "{{ 2 + + }}");
    }

    #[test]
    fn leaves_empty_expressions_unchanged() {
        assert_eq!(inline("{{ }}"), "{{ }}");
    }

    #[test]
    fn leaves_unclosed_braces_as_literal() {
        assert_eq!(inline("start {{ no close"), "start {{ no close");
    }
}

mod format_helper {
    use super::*;

    #[test]
    fn formats_nil_as_empty_string() {
        assert_eq!(symbol::inline::format(&Value::Nil), "");
    }

    #[test]
    fn formats_nested_arrays() {
        let EvalResult::Resolved(value) = symbol::eval("[1, 2] <> [3, 4]", &Bindings::new()).unwrap() else {
            panic!("expected a resolved value")
        };
        assert_eq!(symbol::inline::format(&value), "[[1, 2], [3, 4]]");
    }
}
