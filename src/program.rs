//! Multi-statement programs (port of `statement.cr`): statements are
//! separated by `.`, and `name = expr` assigns to a variable.

use crate::error::Result;
use crate::eval::{Bindings, evaluate};
use crate::lexer::{Lexer, Token, TokenKind};
use crate::parser::Parser;
use crate::value::{EvalResult, Value};

/// Evaluate each statement in turn, returning the last result.
/// Assignments are written into `bindings` as they happen.
pub fn eval_program(source: &str, bindings: &mut Bindings) -> Result<EvalResult> {
    let mut tokens = Lexer::new(source).tokenize();
    tokens.pop(); // Eof

    let mut result = EvalResult::Resolved(Value::Nil);
    for statement in tokens.split(|t| t.kind == TokenKind::Period).filter(|s| !s.is_empty()) {
        result = match statement {
            [name, assign, expr @ ..]
                if name.kind == TokenKind::Identifier
                    && assign.kind == TokenKind::Assign
                    && !expr.is_empty() =>
            {
                let result = eval_tokens(expr, bindings)?;
                if let EvalResult::Resolved(value) = &result {
                    bindings.insert(name.value.clone(), value.clone());
                }
                result
            }
            expr => eval_tokens(expr, bindings)?,
        };
    }
    Ok(result)
}

fn eval_tokens(tokens: &[Token], bindings: &Bindings) -> Result<EvalResult> {
    let mut tokens = tokens.to_vec();
    tokens.push(Token::new(TokenKind::Eof, "", 1, 1)); // Crystal appends a bare EOF at 1:1
    evaluate(&Parser::new(tokens).parse()?, bindings)
}
