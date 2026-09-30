//! Tokenizer (port of `lexer.cr`).

use std::fmt;

use crate::compat;

/// Token kinds. Variant names match Crystal's `TokenType`, because they
/// appear in parse error messages ("Expected RParen, got EOF").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    // Literals
    Number,
    String,
    Identifier,
    True,
    False,
    // Operators
    Plus,
    Minus,
    PlusMinus,
    MinusPlus,
    Star,
    Slash,
    Percent,
    Caret,
    Equals,
    NotEq,
    LessThan,
    GreaterThan,
    LessEq,
    GreaterEq,
    Bang,
    Question,
    IndexRight,
    IndexLeft,
    Hash,
    Tilde,
    Dollar,
    // Unicode operators
    Sum,
    Product,
    NotEqual,
    LessEqual,
    GreaterEqual,
    CeilMax,
    FloorMin,
    Take,
    Drop,
    Reverse,
    GradeUp,
    GradeDown,
    // Program mode
    Assign,
    Range,
    Period,
    // Structural operators
    Concat,
    Wrap,
    Cons,
    Snoc,
    Zip,
    Piz,
    RemoveBack,
    RemoveFront,
    RemoveBoth,
    // Wrapped operators
    BitOr,
    BitAnd,
    BitXor,
    BitNot,
    // Delimiters
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Comma,
    Semicolon,
    // Special
    Eof,
    Error,
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Eof => f.write_str("EOF"),
            kind => fmt::Debug::fmt(kind, f),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    /// Source text of the token; for strings, the unescaped contents;
    /// for errors, the message.
    pub value: String,
    pub line: usize,
    pub column: usize,
}

impl Token {
    pub fn new(kind: TokenKind, value: impl Into<String>, line: usize, column: usize) -> Self {
        Token { kind, value: value.into(), line, column }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({})", self.kind, self.value)
    }
}

pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    column: usize,
    /// Where the last token ended, if it can end a value.
    value_end: Option<usize>,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Lexer { chars: source.chars().collect(), pos: 0, line: 1, column: 1, value_end: None }
    }

    /// All tokens in the source, ending with `Eof`. Never fails: unexpected
    /// characters become `Error` tokens, which the parser rejects.
    pub fn tokenize(mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace();
            if self.at_end() {
                break;
            }
            let token = self.next_token();
            self.value_end = ends_value(token.kind).then_some(self.pos);
            tokens.push(token);
        }
        tokens.push(Token::new(TokenKind::Eof, "", self.line, self.column));
        tokens
    }

    fn next_token(&mut self) -> Token {
        use TokenKind as K;
        let start = self.pos;
        let column = self.column;
        let c = self.advance();

        // Guards that call `eat` consume the rest of a multi-character operator.
        let kind = match c {
            '+' if self.eat('>') => K::Cons,
            '+' if self.eat('-') => K::PlusMinus,
            '+' => K::Plus,
            '-' if self.eat('>') => K::RemoveBack,
            '-' if self.eat('+') => K::MinusPlus,
            // `-` negates, so right after a value it has nothing to do: `3-3`
            // is a mistake. (In `x-y` the `-` is part of the name.)
            '-' if self.value_end == Some(start) => {
                return self.error("Unexpected - right after a value (subtract with +-)", column);
            }
            '-' if self.peek().is_ascii_digit() => return self.number(c, column),
            '-' => K::Minus,
            '*' => K::Star,
            '/' => K::Slash,
            '%' => K::Percent,
            '^' => K::Caret,
            '=' if self.eat('=') => K::Equals,
            '=' => K::Assign,
            '!' if self.eat('=') => K::NotEq,
            '!' => K::Bang,
            '<' if self.eat_pair('-', '>') => K::RemoveBoth,
            '<' if self.eat('-') => K::RemoveFront,
            '<' if self.eat('>') => K::Wrap,
            '<' if self.eat('+') => K::Snoc,
            '<' if self.eat('~') => K::Piz,
            '<' if self.eat('@') => K::IndexLeft,
            '<' if self.eat('=') => K::LessEq,
            '<' => K::LessThan,
            '>' if self.eat('<') => K::Concat,
            '>' if self.eat('=') => K::GreaterEq,
            '>' => K::GreaterThan,
            '?' => K::Question,
            '@' if self.eat('>') => K::IndexRight,
            '@' => return self.error("Unexpected character: @ (use @> or <@)", column),
            '#' => K::Hash,
            '~' if self.eat('>') => K::Zip,
            '~' => K::Tilde,
            '$' => K::Dollar,
            '(' => K::LParen,
            ')' => K::RParen,
            '[' if self.eat_pair('+', ']') => K::BitOr,
            '[' if self.eat_pair('*', ']') => K::BitAnd,
            '[' if self.eat_pair('-', ']') => K::BitXor,
            '[' if self.eat_pair('~', ']') => K::BitNot,
            '[' => K::LBracket,
            ']' => K::RBracket,
            '{' => K::LBrace,
            '}' => K::RBrace,
            ',' => K::Comma,
            ';' => K::Semicolon,
            '.' if self.eat('.') => K::Range,
            '.' => K::Period,
            '"' => return self.string(column),
            '\\' => return self.latex_name(column),
            c if unicode_operator(c).is_some() => unicode_operator(c).expect("checked"),
            c if c.is_ascii_digit() => return self.number(c, column),
            c if c.is_ascii_alphabetic() || c == '_' => return self.identifier(c, column),
            c => return self.error(format!("Unexpected character: {c}"), column),
        };
        let text: String = self.chars[start..self.pos].iter().collect();
        Token::new(kind, text, self.line, column)
    }

    /// Digits with at most one `.` that is followed by a digit (`4.` is `4` then `.`).
    fn number(&mut self, first: char, column: usize) -> Token {
        let mut text = String::from(first);
        let mut has_dot = false;
        loop {
            if self.peek().is_ascii_digit() {
                text.push(self.advance());
            } else if self.peek() == '.' && !has_dot && self.peek_next().is_ascii_digit() {
                has_dot = true;
                text.push(self.advance());
            } else {
                break;
            }
        }
        Token::new(TokenKind::Number, text, self.line, column)
    }

    fn string(&mut self, column: usize) -> Token {
        let mut text = String::new();
        while !self.at_end() && self.peek() != '"' {
            let c = self.advance();
            if c != '\\' {
                text.push(c);
            } else if !self.at_end() {
                text.push(match self.advance() {
                    'n' => '\n',
                    't' => '\t',
                    '\\' => '\\',
                    '"' => '"',
                    // Unknown escapes keep the backslash and drop the character (as in Crystal).
                    _ => '\\',
                });
            }
        }
        if !self.at_end() {
            self.advance(); // closing quote
        }
        Token::new(TokenKind::String, text, self.line, column)
    }

    /// `\sum` and the like: an operator typed by its LaTeX name (or, where
    /// LaTeX has none, a descriptive one). It is the same token as the
    /// operator's symbol, so `\sum x` and `Σ x` are the same expression.
    fn latex_name(&mut self, column: usize) -> Token {
        let mut name = String::new();
        while self.peek().is_ascii_alphabetic() {
            name.push(self.advance());
        }
        match latex_operator(&name) {
            Some(symbol) => {
                let kind = unicode_operator(symbol).unwrap_or(TokenKind::Hash);
                Token::new(kind, symbol.to_string(), self.line, column)
            }
            None => self.error(format!("Unknown operator name: \\{name}"), column),
        }
    }

    /// Letters, digits and `_`, and `-` when a letter or digit follows
    /// (`data-id`, `x-1`).
    fn identifier(&mut self, first: char, column: usize) -> Token {
        let mut text = String::from(first);
        while self.peek().is_ascii_alphanumeric()
            || self.peek() == '_'
            || (self.peek() == '-' && self.peek_next().is_ascii_alphanumeric())
        {
            text.push(self.advance());
        }
        let kind = match text.as_str() {
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            _ => TokenKind::Identifier,
        };
        Token::new(kind, text, self.line, column)
    }

    fn error(&self, message: impl Into<String>, column: usize) -> Token {
        Token::new(TokenKind::Error, message, self.line, column)
    }

    fn skip_whitespace(&mut self) {
        while !self.at_end() && compat::is_ascii_whitespace(self.peek()) {
            if self.peek() == '\n' {
                self.line += 1;
                self.column = 0;
            }
            self.advance();
        }
    }

    fn at_end(&self) -> bool {
        self.pos >= self.chars.len()
    }

    fn peek(&self) -> char {
        self.chars.get(self.pos).copied().unwrap_or('\0')
    }

    fn peek_next(&self) -> char {
        self.chars.get(self.pos + 1).copied().unwrap_or('\0')
    }

    fn advance(&mut self) -> char {
        let c = self.chars[self.pos];
        self.pos += 1;
        self.column += 1;
        c
    }

    fn eat(&mut self, expected: char) -> bool {
        let matched = self.peek() == expected;
        if matched {
            self.advance();
        }
        matched
    }

    fn eat_pair(&mut self, first: char, second: char) -> bool {
        let matched = self.peek() == first && self.peek_next() == second;
        if matched {
            self.advance();
            self.advance();
        }
        matched
    }
}

/// Tokens that can end a value.
fn ends_value(kind: TokenKind) -> bool {
    use TokenKind as K;
    matches!(kind, K::Number | K::String | K::Identifier | K::True | K::False | K::RParen | K::RBracket)
}

/// The token of an operator written as a (non-ASCII) symbol.
fn unicode_operator(c: char) -> Option<TokenKind> {
    use TokenKind as K;
    Some(match c {
        'Σ' => K::Sum,
        'Π' => K::Product,
        '≠' => K::NotEqual,
        '≤' => K::LessEqual,
        '≥' => K::GreaterEqual,
        '⌈' => K::CeilMax,
        '⌊' => K::FloorMin,
        '⊤' => K::True,
        '⊥' => K::False,
        '↑' => K::Take,
        '↓' => K::Drop,
        '⌽' => K::Reverse,
        '⍋' => K::GradeUp,
        '⍒' => K::GradeDown,
        _ => return None,
    })
}

/// The symbol an operator name stands for: its LaTeX command where LaTeX
/// has one, otherwise a descriptive name. `#` is ASCII, but `\count` reads
/// better in words.
pub fn latex_operator(name: &str) -> Option<char> {
    Some(match name {
        "sum" => 'Σ',
        "prod" => 'Π',
        "neq" | "ne" => '≠',
        "leq" | "le" => '≤',
        "geq" | "ge" => '≥',
        "lceil" | "max" => '⌈',
        "lfloor" | "min" => '⌊',
        "top" => '⊤',
        "bot" => '⊥',
        "uparrow" => '↑',
        "downarrow" => '↓',
        "reverse" => '⌽',
        "gradeup" => '⍋',
        "gradedown" => '⍒',
        "count" => '#',
        _ => return None,
    })
}
