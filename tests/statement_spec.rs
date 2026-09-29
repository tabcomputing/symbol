//! Port of spec/statement_spec.cr: program mode (`.` separators, `=` assignment).

use symbol::{Bindings, Error, EvalResult, Value};

fn eval_multi(source: &str, bindings: &mut Bindings) -> Value {
    match symbol::eval_program(source, bindings).unwrap() {
        EvalResult::Resolved(value) => value,
        other => panic!("expected a resolved value, got {other}"),
    }
}

fn run(source: &str) -> Value {
    eval_multi(source, &mut Bindings::new())
}

mod single_expression {
    use super::*;

    #[test]
    fn evaluates_a_simple_expression() {
        assert_eq!(run("2 + 3"), Value::Int(5));
    }

    #[test]
    fn evaluates_a_single_literal() {
        assert_eq!(run("42"), Value::Int(42));
    }
}

mod assignment {
    use super::*;

    #[test]
    fn assigns_and_returns_the_value() {
        let mut bindings = Bindings::new();
        assert_eq!(eval_multi("x = 4.", &mut bindings), Value::Int(4));
        assert_eq!(bindings["x"], Value::Int(4));
    }

    #[test]
    fn assigns_and_uses_in_next_statement() {
        assert_eq!(run("x = 4. x + 2."), Value::Int(6));
    }

    #[test]
    fn chains_assignments() {
        let mut bindings = Bindings::new();
        assert_eq!(eval_multi("x = 3. y = x * 2. y + 1.", &mut bindings), Value::Int(7));
        assert_eq!(bindings["x"], Value::Int(3));
        assert_eq!(bindings["y"], Value::Int(6));
    }
}

mod trailing_period {
    use super::*;

    #[test]
    fn works_without_trailing_period() {
        assert_eq!(run("x = 4. x + 2"), Value::Int(6));
    }

    #[test]
    fn works_with_trailing_period() {
        assert_eq!(run("x = 4. x + 2."), Value::Int(6));
    }
}

mod mixed_types {
    use super::*;

    #[test]
    fn handles_float_assignment() {
        assert_eq!(run("x = 3.14. ⌊ x."), Value::Int(3));
    }

    #[test]
    fn handles_string_assignment() {
        let mut bindings = Bindings::new();
        assert_eq!(eval_multi("s = \"hello\".", &mut bindings), Value::from("hello"));
        assert_eq!(bindings["s"], Value::from("hello"));
    }

    #[test]
    fn handles_boolean_assignment() {
        let mut bindings = Bindings::new();
        assert_eq!(eval_multi("b = true.", &mut bindings), Value::Bool(true));
        assert_eq!(bindings["b"], Value::Bool(true));
    }

    #[test]
    fn handles_list_assignment() {
        let mut bindings = Bindings::new();
        assert_eq!(eval_multi("xs = [1, 2, 3].", &mut bindings), Value::from(vec![1i64, 2, 3]));
    }
}

mod empty_program {
    use super::*;

    #[test]
    fn returns_nil_for_empty_periods() {
        assert_eq!(run(". ."), Value::Nil);
    }

    #[test]
    fn returns_nil_for_empty_string() {
        assert_eq!(run(""), Value::Nil);
    }
}

mod bindings_mutation {
    use super::*;

    #[test]
    fn mutates_the_provided_bindings() {
        let mut bindings = Bindings::new();
        symbol::eval_program("x = 10. y = 20.", &mut bindings).unwrap();
        assert_eq!(bindings["x"], Value::Int(10));
        assert_eq!(bindings["y"], Value::Int(20));
    }

    #[test]
    fn can_use_pre_existing_bindings() {
        let mut bindings = Bindings::from([("x".to_owned(), Value::Int(5))]);
        assert_eq!(eval_multi("x + 10", &mut bindings), Value::Int(15));
    }

    #[test]
    fn can_override_pre_existing_bindings() {
        let mut bindings = Bindings::from([("x".to_owned(), Value::Int(5))]);
        assert_eq!(eval_multi("x = 10. x", &mut bindings), Value::Int(10));
        assert_eq!(bindings["x"], Value::Int(10));
    }
}

mod assignment_rejected_in_expression_context {
    use super::*;

    #[test]
    fn raises_parse_error_for_assign_in_eval() {
        let error = symbol::eval("x = 5", &Bindings::new()).unwrap_err();
        assert!(matches!(error, Error::Parse { .. }));
        assert!(error.to_string().to_lowercase().contains("assignment"));
    }
}
