// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
//
// SCE Forge expression transpiler — ECMAScript subset to target language.
//
// Architecture: source text -> tokenize -> parse (recursive descent) -> AST -> emit.
//
// The AST captures ECMAScript semantics. Each target language has its own
// precedence function; the emitter adds parentheses wherever the target
// would parse the expression differently from the AST's intended meaning.
//
// Limitations:
// - `>>>` (unsigned right shift) maps to `>>` in C++/Rust/Go. For signed
//   types this produces arithmetic shift, not logical shift. SCXML authors
//   should ensure `>>>` operands are unsigned, or use explicit masking.
// - String escape sequences (e.g. `\'`) are passed through verbatim.
//   Target languages may have different escape rules; SCXML strings should
//   use only ASCII content without language-specific escapes.

use std::fmt;
use std::sync::LazyLock;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Public API
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Target language for expression transpilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExprTarget {
    Cpp,
    Kotlin,
    Rust,
    Go,
    Python,
}

/// Transpile an ECMAScript expression to the target language.
pub fn transpile(expr: &str, target: ExprTarget) -> Result<String, String> {
    let expr = expr.trim();
    if expr.is_empty() {
        return Err("Empty expression".to_string());
    }

    let tokens = tokenize(expr)?;
    let ast = Parser::new(&tokens).parse_expression()?;
    emit(&ast, target)
}

/// Replace string literal contents with spaces (used by forge generator).
pub fn strip_string_literals(expr: &str) -> String {
    static RE_STR: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r#"'[^']*'|"[^"]*""#).unwrap()
    });
    RE_STR
        .replace_all(expr, |caps: &regex::Captures| " ".repeat(caps[0].len()))
        .to_string()
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// AST
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Expr {
    NumberLit(String),
    StringLit { value: String, quote: char },
    BoolLit(bool),
    NullLit,
    Ident(String),
    Binary { op: BinOp, left: Box<Expr>, right: Box<Expr> },
    Unary { op: UnaryOp, operand: Box<Expr> },
    Conditional { condition: Box<Expr>, consequent: Box<Expr>, alternate: Box<Expr> },
    Member { object: Box<Expr>, property: String },
    Index { object: Box<Expr>, index: Box<Expr> },
    Call { callee: Box<Expr>, args: Vec<Expr> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BinOp {
    Add, Sub, Mul, Div, Mod,
    StrictEq, StrictNeq, Lt, Gt, LtEq, GtEq,
    And, Or,
    BitAnd, BitOr, BitXor,
    Shl, Shr, UShr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnaryOp {
    Neg, Pos, Not, BitNot,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Tokens
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(String),
    String { value: String, quote: char },
    Ident(String),
    Plus, Minus, Star, Slash, Percent,
    StrictEq, StrictNeq,
    Lt, Gt, LtEq, GtEq,
    AmpAmp, PipePipe,
    Amp, Pipe, Caret, Tilde, Bang,
    Shl, Shr, UShr,
    Question, Colon, Dot, Comma,
    LParen, RParen, LBracket, RBracket,
    Eof,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Number(n) => write!(f, "{n}"),
            Token::String { value, .. } => write!(f, "'{value}'"),
            Token::Ident(s) => write!(f, "{s}"),
            Token::Plus => write!(f, "+"), Token::Minus => write!(f, "-"),
            Token::Star => write!(f, "*"), Token::Slash => write!(f, "/"),
            Token::Percent => write!(f, "%"),
            Token::StrictEq => write!(f, "==="), Token::StrictNeq => write!(f, "!=="),
            Token::Lt => write!(f, "<"), Token::Gt => write!(f, ">"),
            Token::LtEq => write!(f, "<="), Token::GtEq => write!(f, ">="),
            Token::AmpAmp => write!(f, "&&"), Token::PipePipe => write!(f, "||"),
            Token::Amp => write!(f, "&"), Token::Pipe => write!(f, "|"),
            Token::Caret => write!(f, "^"), Token::Tilde => write!(f, "~"),
            Token::Bang => write!(f, "!"),
            Token::Shl => write!(f, "<<"), Token::Shr => write!(f, ">>"),
            Token::UShr => write!(f, ">>>"),
            Token::Question => write!(f, "?"), Token::Colon => write!(f, ":"),
            Token::Dot => write!(f, "."), Token::Comma => write!(f, ","),
            Token::LParen => write!(f, "("), Token::RParen => write!(f, ")"),
            Token::LBracket => write!(f, "["), Token::RBracket => write!(f, "]"),
            Token::Eof => write!(f, "EOF"),
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Tokenizer
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }

        // String literals
        if bytes[i] == b'\'' || bytes[i] == b'"' {
            let quote = bytes[i] as char;
            i += 1;
            let start = i;
            while i < len && bytes[i] != quote as u8 {
                if bytes[i] == b'\\' {
                    i += 1;
                }
                i += 1;
            }
            if i >= len {
                return Err(format!("Unterminated string literal starting at position {start}"));
            }
            let value = input[start..i].to_string();
            i += 1;
            tokens.push(Token::String { value, quote });
            continue;
        }

        // Numbers: decimal, hex, binary, octal, float
        if bytes[i].is_ascii_digit()
            || (bytes[i] == b'.' && i + 1 < len && bytes[i + 1].is_ascii_digit())
        {
            let start = i;
            let starts_with_dot = bytes[i] == b'.';
            if bytes[i] == b'0' && i + 1 < len {
                match bytes[i + 1] {
                    b'x' | b'X' => {
                        i += 2;
                        while i < len && bytes[i].is_ascii_hexdigit() { i += 1; }
                        tokens.push(Token::Number(input[start..i].to_string()));
                        continue;
                    }
                    b'b' | b'B' => {
                        i += 2;
                        while i < len && (bytes[i] == b'0' || bytes[i] == b'1') { i += 1; }
                        tokens.push(Token::Number(input[start..i].to_string()));
                        continue;
                    }
                    b'o' | b'O' => {
                        i += 2;
                        while i < len && bytes[i] >= b'0' && bytes[i] <= b'7' { i += 1; }
                        tokens.push(Token::Number(input[start..i].to_string()));
                        continue;
                    }
                    _ => {}
                }
            }
            while i < len && bytes[i].is_ascii_digit() { i += 1; }
            if i < len && bytes[i] == b'.' && (i + 1 >= len || bytes[i + 1] != b'.') {
                i += 1;
                while i < len && bytes[i].is_ascii_digit() { i += 1; }
            }
            if i < len && (bytes[i] == b'e' || bytes[i] == b'E') {
                i += 1;
                if i < len && (bytes[i] == b'+' || bytes[i] == b'-') { i += 1; }
                while i < len && bytes[i].is_ascii_digit() { i += 1; }
            }
            // Normalize `.5` -> `0.5` (not valid in Rust/Kotlin)
            let mut num = input[start..i].to_string();
            if starts_with_dot {
                num.insert(0, '0');
            }
            tokens.push(Token::Number(num));
            continue;
        }

        // Identifiers and keywords
        if bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' || bytes[i] == b'$' {
            let start = i;
            while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'$') {
                i += 1;
            }
            let word = &input[start..i];
            validate_keyword(word)?;
            tokens.push(Token::Ident(word.to_string()));
            continue;
        }

        // Reject unsupported multi-char constructs with clear diagnostics.
        // These would otherwise produce confusing parse errors.
        if i + 1 < len && &input[i..i + 2] == "=>" {
            return Err(
                "Unsupported ECMAScript construct: arrow function (=>). \
                 Extended SCXML expressions must use the stateless subset."
                    .to_string(),
            );
        }
        if i + 1 < len && &input[i..i + 2] == "??" {
            return Err(
                "Unsupported ECMAScript construct: nullish coalescing (??). \
                 Extended SCXML expressions must use the stateless subset."
                    .to_string(),
            );
        }
        if i + 1 < len && &input[i..i + 2] == "?." {
            return Err(
                "Unsupported ECMAScript construct: optional chaining (?.). \
                 Extended SCXML expressions must use the stateless subset."
                    .to_string(),
            );
        }
        if i + 2 < len && &input[i..i + 3] == "..." {
            return Err(
                "Unsupported ECMAScript construct: spread/rest (...). \
                 Extended SCXML expressions must use the stateless subset."
                    .to_string(),
            );
        }
        if bytes[i] == b'`' {
            return Err(
                "Unsupported ECMAScript construct: template literal (`). \
                 Extended SCXML expressions must use the stateless subset."
                    .to_string(),
            );
        }

        // Multi-char operators (longest match first)
        if i + 2 < len && &input[i..i + 3] == ">>>" {
            tokens.push(Token::UShr); i += 3; continue;
        }
        if i + 2 < len && &input[i..i + 3] == "===" {
            tokens.push(Token::StrictEq); i += 3; continue;
        }
        if i + 2 < len && &input[i..i + 3] == "!==" {
            tokens.push(Token::StrictNeq); i += 3; continue;
        }
        if i + 1 < len {
            let two = &input[i..i + 2];
            let tok = match two {
                // `!==` is already consumed by the 3-char check above.
                // Any remaining `==` or `!=` is the loose variant — reject.
                "==" => {
                    return Err(
                        "Loose equality '==' is not permitted in Extended SCXML. \
                         Use '===' (strict equality) instead."
                            .to_string(),
                    );
                }
                "!=" => {
                    return Err(
                        "Loose inequality '!=' is not permitted in Extended SCXML. \
                         Use '!==' (strict inequality) instead."
                            .to_string(),
                    );
                }
                "&&" => Some(Token::AmpAmp),
                "||" => Some(Token::PipePipe),
                "<<" => Some(Token::Shl),
                ">>" => Some(Token::Shr),
                "<=" => Some(Token::LtEq),
                ">=" => Some(Token::GtEq),
                _ => None,
            };
            if let Some(t) = tok {
                tokens.push(t); i += 2; continue;
            }
        }

        // Single-char operators
        let tok = match bytes[i] {
            b'+' => Token::Plus, b'-' => Token::Minus,
            b'*' => Token::Star, b'/' => Token::Slash, b'%' => Token::Percent,
            b'<' => Token::Lt, b'>' => Token::Gt,
            b'&' => Token::Amp, b'|' => Token::Pipe,
            b'^' => Token::Caret, b'~' => Token::Tilde, b'!' => Token::Bang,
            b'?' => Token::Question, b':' => Token::Colon,
            b'.' => Token::Dot, b',' => Token::Comma,
            b'(' => Token::LParen, b')' => Token::RParen,
            b'[' => Token::LBracket, b']' => Token::RBracket,
            ch => return Err(format!("Unexpected character: '{}'", ch as char)),
        };
        tokens.push(tok);
        i += 1;
    }

    tokens.push(Token::Eof);
    Ok(tokens)
}

fn validate_keyword(word: &str) -> Result<(), String> {
    const REJECTED: &[(&str, &str)] = &[
        ("new", "new"), ("delete", "delete"), ("typeof", "typeof"),
        ("instanceof", "instanceof"), ("this", "this"), ("eval", "eval()"),
        ("async", "async"), ("await", "await"), ("yield", "yield"),
        ("function", "function declaration"), ("class", "class declaration"),
        ("var", "var declaration"), ("let", "let declaration"),
        ("const", "const declaration"),
    ];
    for &(kw, desc) in REJECTED {
        if word == kw {
            return Err(format!(
                "Unsupported ECMAScript construct: {desc}. \
                 Extended SCXML expressions must use the stateless subset."
            ));
        }
    }
    Ok(())
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Recursive descent parser
//
// Precedence (lowest to highest, matching ECMAScript):
//   1. Conditional     ( ? : )
//   2. Logical OR      ( || )
//   3. Logical AND     ( && )
//   4. Bitwise OR      ( | )
//   5. Bitwise XOR     ( ^ )
//   6. Bitwise AND     ( & )
//   7. Equality        ( === !== )
//   8. Relational      ( < > <= >= )
//   9. Shift           ( << >> >>> )
//  10. Additive        ( + - )
//  11. Multiplicative  ( * / % )
//  12. Unary           ( - + ! ~ )
//  13. Postfix         ( . [] () )
//  14. Primary         ( literals, identifiers, grouping )
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token]) -> Self { Self { tokens, pos: 0 } }

    fn peek(&self) -> &Token { self.tokens.get(self.pos).unwrap_or(&Token::Eof) }

    fn advance(&mut self) -> &Token {
        let tok = self.tokens.get(self.pos).unwrap_or(&Token::Eof);
        self.pos += 1;
        tok
    }

    fn expect(&mut self, expected: &Token) -> Result<(), String> {
        let tok = self.advance().clone();
        if &tok == expected { Ok(()) }
        else { Err(format!("Expected '{expected}', got '{tok}'")) }
    }

    fn parse_expression(&mut self) -> Result<Expr, String> {
        let expr = self.parse_conditional()?;
        if *self.peek() != Token::Eof {
            return Err(format!("Unexpected token after expression: '{}'", self.peek()));
        }
        Ok(expr)
    }

    fn parse_conditional(&mut self) -> Result<Expr, String> {
        let expr = self.parse_logical_or()?;
        if *self.peek() == Token::Question {
            self.advance();
            let consequent = self.parse_conditional()?;
            self.expect(&Token::Colon)?;
            let alternate = self.parse_conditional()?;
            return Ok(Expr::Conditional {
                condition: Box::new(expr),
                consequent: Box::new(consequent),
                alternate: Box::new(alternate),
            });
        }
        Ok(expr)
    }

    fn parse_logical_or(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_logical_and()?;
        while *self.peek() == Token::PipePipe {
            self.advance();
            let right = self.parse_logical_and()?;
            left = Expr::Binary { op: BinOp::Or, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_logical_and(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_bitwise_or()?;
        while *self.peek() == Token::AmpAmp {
            self.advance();
            let right = self.parse_bitwise_or()?;
            left = Expr::Binary { op: BinOp::And, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_bitwise_or(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_bitwise_xor()?;
        while *self.peek() == Token::Pipe {
            self.advance();
            let right = self.parse_bitwise_xor()?;
            left = Expr::Binary { op: BinOp::BitOr, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_bitwise_xor(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_bitwise_and()?;
        while *self.peek() == Token::Caret {
            self.advance();
            let right = self.parse_bitwise_and()?;
            left = Expr::Binary { op: BinOp::BitXor, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_bitwise_and(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_equality()?;
        while *self.peek() == Token::Amp {
            self.advance();
            let right = self.parse_equality()?;
            left = Expr::Binary { op: BinOp::BitAnd, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_relational()?;
        loop {
            let op = match self.peek() {
                Token::StrictEq => BinOp::StrictEq,
                Token::StrictNeq => BinOp::StrictNeq,
                _ => break,
            };
            self.advance();
            let right = self.parse_relational()?;
            left = Expr::Binary { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_relational(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_shift()?;
        loop {
            let op = match self.peek() {
                Token::Lt => BinOp::Lt, Token::Gt => BinOp::Gt,
                Token::LtEq => BinOp::LtEq, Token::GtEq => BinOp::GtEq,
                _ => break,
            };
            self.advance();
            let right = self.parse_shift()?;
            left = Expr::Binary { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_shift(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_additive()?;
        loop {
            let op = match self.peek() {
                Token::Shl => BinOp::Shl, Token::Shr => BinOp::Shr, Token::UShr => BinOp::UShr,
                _ => break,
            };
            self.advance();
            let right = self.parse_additive()?;
            left = Expr::Binary { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = match self.peek() {
                Token::Plus => BinOp::Add, Token::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            left = Expr::Binary { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Token::Star => BinOp::Mul, Token::Slash => BinOp::Div, Token::Percent => BinOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::Binary { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        let op = match self.peek() {
            Token::Minus => Some(UnaryOp::Neg), Token::Plus => Some(UnaryOp::Pos),
            Token::Bang => Some(UnaryOp::Not), Token::Tilde => Some(UnaryOp::BitNot),
            _ => None,
        };
        if let Some(op) = op {
            self.advance();
            let operand = self.parse_unary()?;
            return Ok(Expr::Unary { op, operand: Box::new(operand) });
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;
        loop {
            match self.peek() {
                Token::Dot => {
                    self.advance();
                    let prop = match self.advance().clone() {
                        Token::Ident(s) => s,
                        other => return Err(format!("Expected property name after '.', got '{other}'")),
                    };
                    expr = Expr::Member { object: Box::new(expr), property: prop };
                }
                Token::LBracket => {
                    self.advance();
                    let index = self.parse_conditional()?;
                    self.expect(&Token::RBracket)?;
                    expr = Expr::Index { object: Box::new(expr), index: Box::new(index) };
                }
                Token::LParen => {
                    self.advance();
                    let mut args = Vec::new();
                    if *self.peek() != Token::RParen {
                        args.push(self.parse_conditional()?);
                        while *self.peek() == Token::Comma {
                            self.advance();
                            args.push(self.parse_conditional()?);
                        }
                    }
                    self.expect(&Token::RParen)?;
                    expr = Expr::Call { callee: Box::new(expr), args };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.advance().clone() {
            Token::Number(n) => Ok(Expr::NumberLit(n)),
            Token::String { value, quote } => Ok(Expr::StringLit { value, quote }),
            Token::Ident(s) if s == "true" => Ok(Expr::BoolLit(true)),
            Token::Ident(s) if s == "false" => Ok(Expr::BoolLit(false)),
            Token::Ident(s) if s == "null" => Ok(Expr::NullLit),
            Token::Ident(s) => Ok(Expr::Ident(s)),
            Token::LParen => {
                let inner = self.parse_conditional()?;
                self.expect(&Token::RParen)?;
                Ok(inner)
            }
            other => Err(format!("Unexpected token: '{other}'")),
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Operator Precedence (per target language)
//
// The AST captures ECMAScript semantics. When emitting, we must add
// parentheses wherever the TARGET language's precedence would cause
// a different parse tree.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// ECMAScript operator precedence (higher = binds tighter).
/// Also matches C++ relative ordering (equality > bitwise).
fn ecma_precedence(op: BinOp) -> u8 {
    match op {
        BinOp::Or => 1,
        BinOp::And => 2,
        BinOp::BitOr => 3,
        BinOp::BitXor => 4,
        BinOp::BitAnd => 5,
        BinOp::StrictEq | BinOp::StrictNeq => 6,
        BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => 7,
        BinOp::Shl | BinOp::Shr | BinOp::UShr => 8,
        BinOp::Add | BinOp::Sub => 9,
        BinOp::Mul | BinOp::Div | BinOp::Mod => 10,
    }
}

/// Kotlin operator precedence. Bitwise ops (and/or/xor/shl/shr/ushr) are infix
/// functions sharing one precedence level, HIGHER than equality and comparison.
fn kotlin_precedence(op: BinOp) -> u8 {
    match op {
        BinOp::Or => 1,
        BinOp::And => 2,
        BinOp::StrictEq | BinOp::StrictNeq => 3,
        BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => 4,
        BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor
        | BinOp::Shl | BinOp::Shr | BinOp::UShr => 7,
        BinOp::Add | BinOp::Sub => 9,
        BinOp::Mul | BinOp::Div | BinOp::Mod => 10,
    }
}

/// Go operator precedence. Differs significantly from ECMAScript:
/// - `&`, `<<`, `>>` grouped with `*` at level 5 (higher than `+`)
/// - `|`, `^` grouped with `+` at level 4
/// - All comparison at level 3 (lower than all bitwise ops)
///
/// Reference: Go spec "Operator precedence"
///   5: *  /  %  <<  >>  &  &^
///   4: +  -  |  ^
///   3: ==  !=  <  <=  >  >=
///   2: &&
///   1: ||
fn go_precedence(op: BinOp) -> u8 {
    match op {
        BinOp::Or => 1,
        BinOp::And => 2,
        BinOp::StrictEq | BinOp::StrictNeq
        | BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => 3,
        BinOp::Add | BinOp::Sub | BinOp::BitOr | BinOp::BitXor => 4,
        BinOp::Mul | BinOp::Div | BinOp::Mod
        | BinOp::Shl | BinOp::Shr | BinOp::UShr | BinOp::BitAnd => 5,
    }
}

/// Rust operator precedence. Bitwise ops are above comparison (like Python,
/// unlike ECMAScript/C++ where equality is above bitwise AND/XOR/OR).
///
/// Reference: Rust Reference "Expression precedence"
///   &  >  ^  >  |  >  ==, !=, <, >, <=, >=
fn rust_precedence(op: BinOp) -> u8 {
    match op {
        BinOp::Or => 1,
        BinOp::And => 2,
        BinOp::StrictEq | BinOp::StrictNeq
        | BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => 4,
        BinOp::BitOr => 5,
        BinOp::BitXor => 6,
        BinOp::BitAnd => 7,
        BinOp::Shl | BinOp::Shr | BinOp::UShr => 8,
        BinOp::Add | BinOp::Sub => 9,
        BinOp::Mul | BinOp::Div | BinOp::Mod => 10,
    }
}

/// Python operator precedence. Same relative ordering as Rust:
/// bitwise ops are above comparison.
fn python_precedence(op: BinOp) -> u8 {
    match op {
        BinOp::Or => 1,
        BinOp::And => 2,
        BinOp::StrictEq | BinOp::StrictNeq
        | BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => 4,
        BinOp::BitOr => 5,
        BinOp::BitXor => 6,
        BinOp::BitAnd => 7,
        BinOp::Shl | BinOp::Shr | BinOp::UShr => 8,
        BinOp::Add | BinOp::Sub => 9,
        BinOp::Mul | BinOp::Div | BinOp::Mod => 10,
    }
}

/// Check if a child expression needs parentheses in a binary context.
fn child_needs_parens(child: &Expr, parent_op: BinOp, is_left: bool, prec: fn(BinOp) -> u8) -> bool {
    match child {
        Expr::Binary { op: child_op, .. } => {
            let cp = prec(*child_op);
            let pp = prec(parent_op);
            // Left child: parens if child has strictly lower precedence
            // Right child: parens if lower or equal (left-to-right associativity)
            if is_left { cp < pp } else { cp <= pp }
        }
        Expr::Conditional { .. } => true,
        _ => false,
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Emitters
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn emit(expr: &Expr, target: ExprTarget) -> Result<String, String> {
    match target {
        ExprTarget::Cpp => Ok(emit_cpp(expr)),
        ExprTarget::Kotlin => Ok(emit_kotlin(expr)),
        ExprTarget::Rust => Ok(emit_rust(expr)),
        ExprTarget::Go => emit_go(expr),
        ExprTarget::Python => Ok(emit_python(expr)),
    }
}

/// Wrap emitted sub-expression in parens when used as postfix base (before `.`, `[`, `(`).
fn wrap_postfix(expr: &Expr, emitted: String) -> String {
    if matches!(expr, Expr::Binary { .. } | Expr::Conditional { .. } | Expr::Unary { .. }) {
        format!("({emitted})")
    } else {
        emitted
    }
}

/// Emit binary child with precedence-aware parenthesization.
fn emit_child(
    child: &Expr,
    parent_op: BinOp,
    is_left: bool,
    prec: fn(BinOp) -> u8,
    emit_fn: &dyn Fn(&Expr) -> String,
) -> String {
    let raw = emit_fn(child);
    if child_needs_parens(child, parent_op, is_left, prec) { format!("({raw})") } else { raw }
}

// ── C++ ──────────────────────────────────────────────────────────

fn emit_cpp(expr: &Expr) -> String {
    match expr {
        Expr::NumberLit(n) => n.clone(),
        Expr::StringLit { value, .. } => format!("\"{value}\""),
        Expr::BoolLit(b) => if *b { "true" } else { "false" }.to_string(),
        Expr::NullLit => "nullptr".to_string(),
        Expr::Ident(s) => s.clone(),
        Expr::Binary { op, left, right } => {
            let l = emit_child(left, *op, true, ecma_precedence, &emit_cpp);
            let r = emit_child(right, *op, false, ecma_precedence, &emit_cpp);
            format!("{l} {} {r}", cpp_binop(*op))
        }
        Expr::Unary { op, operand } => {
            let inner = emit_cpp(operand);
            let wrap = matches!(operand.as_ref(), Expr::Binary { .. } | Expr::Conditional { .. });
            if wrap { format!("{}({inner})", cpp_unary(*op)) }
            else { format!("{}{inner}", cpp_unary(*op)) }
        }
        Expr::Conditional { condition, consequent, alternate } =>
            format!("{} ? {} : {}", emit_cpp(condition), emit_cpp(consequent), emit_cpp(alternate)),
        Expr::Member { object, property } => format!("{}.{property}", wrap_postfix(object, emit_cpp(object))),
        Expr::Index { object, index } => format!("{}[{}]", wrap_postfix(object, emit_cpp(object)), emit_cpp(index)),
        Expr::Call { callee, args } => {
            let a: Vec<_> = args.iter().map(|a| emit_cpp(a)).collect();
            format!("{}({})", wrap_postfix(callee, emit_cpp(callee)), a.join(", "))
        }
    }
}

fn cpp_binop(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+", BinOp::Sub => "-", BinOp::Mul => "*", BinOp::Div => "/", BinOp::Mod => "%",
        BinOp::StrictEq => "==", BinOp::StrictNeq => "!=",
        BinOp::Lt => "<", BinOp::Gt => ">", BinOp::LtEq => "<=", BinOp::GtEq => ">=",
        BinOp::And => "&&", BinOp::Or => "||",
        BinOp::BitAnd => "&", BinOp::BitOr => "|", BinOp::BitXor => "^",
        BinOp::Shl => "<<", BinOp::Shr => ">>",
        BinOp::UShr => ">>", // W3C SCXML: unsigned shift — correct only for unsigned operands
    }
}

fn cpp_unary(op: UnaryOp) -> &'static str {
    match op { UnaryOp::Neg => "-", UnaryOp::Pos => "+", UnaryOp::Not => "!", UnaryOp::BitNot => "~" }
}

// ── Kotlin ───────────────────────────────────────────────────────

fn emit_kotlin(expr: &Expr) -> String {
    match expr {
        Expr::NumberLit(n) => n.clone(),
        Expr::StringLit { value, .. } => format!("\"{value}\""),
        Expr::BoolLit(b) => if *b { "true" } else { "false" }.to_string(),
        Expr::NullLit => "null".to_string(),
        Expr::Ident(s) => s.clone(),
        Expr::Binary { op, left, right } => {
            let l = emit_child(left, *op, true, kotlin_precedence, &emit_kotlin);
            let r = emit_child(right, *op, false, kotlin_precedence, &emit_kotlin);
            format!("{l} {} {r}", kotlin_binop(*op))
        }
        Expr::Unary { op: UnaryOp::BitNot, operand } =>
            format!("{}.inv()", wrap_postfix(operand, emit_kotlin(operand))),
        Expr::Unary { op, operand } => {
            let inner = emit_kotlin(operand);
            let prefix = match op { UnaryOp::Neg => "-", UnaryOp::Pos => "+", UnaryOp::Not => "!", _ => unreachable!() };
            let wrap = matches!(operand.as_ref(), Expr::Binary { .. } | Expr::Conditional { .. });
            if wrap { format!("{prefix}({inner})") } else { format!("{prefix}{inner}") }
        }
        Expr::Conditional { condition, consequent, alternate } =>
            format!("if ({}) {} else {}", emit_kotlin(condition), emit_kotlin(consequent), emit_kotlin(alternate)),
        Expr::Member { object, property } => format!("{}.{property}", wrap_postfix(object, emit_kotlin(object))),
        Expr::Index { object, index } => format!("{}[{}]", wrap_postfix(object, emit_kotlin(object)), emit_kotlin(index)),
        Expr::Call { callee, args } => {
            let a: Vec<_> = args.iter().map(|a| emit_kotlin(a)).collect();
            format!("{}({})", wrap_postfix(callee, emit_kotlin(callee)), a.join(", "))
        }
    }
}

fn kotlin_binop(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+", BinOp::Sub => "-", BinOp::Mul => "*", BinOp::Div => "/", BinOp::Mod => "%",
        BinOp::StrictEq => "==", BinOp::StrictNeq => "!=",
        BinOp::Lt => "<", BinOp::Gt => ">", BinOp::LtEq => "<=", BinOp::GtEq => ">=",
        BinOp::And => "&&", BinOp::Or => "||",
        BinOp::BitAnd => "and", BinOp::BitOr => "or", BinOp::BitXor => "xor",
        BinOp::Shl => "shl", BinOp::Shr => "shr", BinOp::UShr => "ushr",
    }
}

// ── Rust ─────────────────────────────────────────────────────────
// Note: Rust uses `!` for both logical NOT (bool) and bitwise NOT (integer).
// The AST distinguishes `UnaryOp::Not` from `UnaryOp::BitNot`, but both
// emit `!` in Rust. This is correct because Rust's type system resolves
// the operation at compile time.

fn emit_rust(expr: &Expr) -> String {
    match expr {
        Expr::NumberLit(n) => n.clone(),
        Expr::StringLit { value, .. } => format!("\"{value}\""),
        Expr::BoolLit(b) => if *b { "true" } else { "false" }.to_string(),
        Expr::NullLit => "None".to_string(),
        Expr::Ident(s) => s.clone(),
        Expr::Binary { op, left, right } => {
            let l = emit_child(left, *op, true, rust_precedence, &emit_rust);
            let r = emit_child(right, *op, false, rust_precedence, &emit_rust);
            format!("{l} {} {r}", rust_binop(*op))
        }
        Expr::Unary { op, operand } => {
            let inner = emit_rust(operand);
            let wrap = matches!(operand.as_ref(), Expr::Binary { .. } | Expr::Conditional { .. });
            let prefix = match op { UnaryOp::Neg => "-", UnaryOp::Pos => "", UnaryOp::Not | UnaryOp::BitNot => "!" };
            if wrap { format!("{prefix}({inner})") } else { format!("{prefix}{inner}") }
        }
        Expr::Conditional { condition, consequent, alternate } =>
            format!("if {} {{ {} }} else {{ {} }}", emit_rust(condition), emit_rust(consequent), emit_rust(alternate)),
        Expr::Member { object, property } => format!("{}.{property}", wrap_postfix(object, emit_rust(object))),
        Expr::Index { object, index } => format!("{}[{}]", wrap_postfix(object, emit_rust(object)), emit_rust(index)),
        Expr::Call { callee, args } => {
            let a: Vec<_> = args.iter().map(|a| emit_rust(a)).collect();
            format!("{}({})", wrap_postfix(callee, emit_rust(callee)), a.join(", "))
        }
    }
}

fn rust_binop(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+", BinOp::Sub => "-", BinOp::Mul => "*", BinOp::Div => "/", BinOp::Mod => "%",
        BinOp::StrictEq => "==", BinOp::StrictNeq => "!=",
        BinOp::Lt => "<", BinOp::Gt => ">", BinOp::LtEq => "<=", BinOp::GtEq => ">=",
        BinOp::And => "&&", BinOp::Or => "||",
        BinOp::BitAnd => "&", BinOp::BitOr => "|", BinOp::BitXor => "^",
        BinOp::Shl => "<<", BinOp::Shr => ">>",
        BinOp::UShr => ">>", // W3C SCXML: unsigned shift — correct only for unsigned operands
    }
}

// ── Go ───────────────────────────────────────────────────────────

fn emit_go(expr: &Expr) -> Result<String, String> {
    if has_ternary(expr) {
        return Err(
            "Go does not support ternary expressions. \
             Refactor the SCXML expression to avoid `? :` for Go targets.".to_string(),
        );
    }
    Ok(emit_go_inner(expr))
}

fn has_ternary(expr: &Expr) -> bool {
    match expr {
        Expr::Conditional { .. } => true,
        Expr::Binary { left, right, .. } => has_ternary(left) || has_ternary(right),
        Expr::Unary { operand, .. } => has_ternary(operand),
        Expr::Member { object, .. } => has_ternary(object),
        Expr::Index { object, index } => has_ternary(object) || has_ternary(index),
        Expr::Call { callee, args } => has_ternary(callee) || args.iter().any(has_ternary),
        _ => false,
    }
}

fn emit_go_inner(expr: &Expr) -> String {
    match expr {
        Expr::NumberLit(n) => n.clone(),
        Expr::StringLit { value, .. } => format!("\"{value}\""),
        Expr::BoolLit(b) => if *b { "true" } else { "false" }.to_string(),
        Expr::NullLit => "nil".to_string(),
        Expr::Ident(s) => s.clone(),
        Expr::Binary { op, left, right } => {
            let l = emit_child(left, *op, true, go_precedence, &emit_go_inner);
            let r = emit_child(right, *op, false, go_precedence, &emit_go_inner);
            format!("{l} {} {r}", go_binop(*op))
        }
        Expr::Unary { op, operand } => {
            let inner = emit_go_inner(operand);
            let prefix = match op { UnaryOp::Neg => "-", UnaryOp::Pos => "+", UnaryOp::Not => "!", UnaryOp::BitNot => "^" };
            let wrap = matches!(operand.as_ref(), Expr::Binary { .. });
            if wrap { format!("{prefix}({inner})") } else { format!("{prefix}{inner}") }
        }
        Expr::Conditional { .. } => unreachable!("ternary rejected before emission"),
        Expr::Member { object, property } => format!("{}.{property}", wrap_postfix(object, emit_go_inner(object))),
        Expr::Index { object, index } => format!("{}[{}]", wrap_postfix(object, emit_go_inner(object)), emit_go_inner(index)),
        Expr::Call { callee, args } => {
            let a: Vec<_> = args.iter().map(|a| emit_go_inner(a)).collect();
            format!("{}({})", wrap_postfix(callee, emit_go_inner(callee)), a.join(", "))
        }
    }
}

fn go_binop(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+", BinOp::Sub => "-", BinOp::Mul => "*", BinOp::Div => "/", BinOp::Mod => "%",
        BinOp::StrictEq => "==", BinOp::StrictNeq => "!=",
        BinOp::Lt => "<", BinOp::Gt => ">", BinOp::LtEq => "<=", BinOp::GtEq => ">=",
        BinOp::And => "&&", BinOp::Or => "||",
        BinOp::BitAnd => "&", BinOp::BitOr => "|", BinOp::BitXor => "^",
        BinOp::Shl => "<<", BinOp::Shr => ">>",
        BinOp::UShr => ">>", // W3C SCXML: unsigned shift — correct only for unsigned operands
    }
}

// ── Python ───────────────────────────────────────────────────────

fn emit_python(expr: &Expr) -> String {
    match expr {
        Expr::NumberLit(n) => n.clone(),
        Expr::StringLit { value, quote } => format!("{quote}{value}{quote}"),
        Expr::BoolLit(b) => if *b { "True" } else { "False" }.to_string(),
        Expr::NullLit => "None".to_string(),
        Expr::Ident(s) => s.clone(),
        Expr::Binary { op, left, right } => {
            let l = emit_child(left, *op, true, python_precedence, &emit_python);
            let r = emit_child(right, *op, false, python_precedence, &emit_python);
            format!("{l} {} {r}", python_binop(*op))
        }
        Expr::Unary { op, operand } => {
            let inner = emit_python(operand);
            let wrap = matches!(operand.as_ref(), Expr::Binary { .. } | Expr::Conditional { .. });
            match op {
                UnaryOp::Not => if wrap { format!("not ({inner})") } else { format!("not {inner}") },
                UnaryOp::Neg => if wrap { format!("-({inner})") } else { format!("-{inner}") },
                UnaryOp::Pos => if wrap { format!("+({inner})") } else { format!("+{inner}") },
                UnaryOp::BitNot => if wrap { format!("~({inner})") } else { format!("~{inner}") },
            }
        }
        Expr::Conditional { condition, consequent, alternate } => {
            // In Python `X if C else Y`, a ternary in X position must be
            // parenthesized: `(1 if b else 2) if a else 3`, otherwise
            // Python parses `1 if b else 2 if a else 3` as `1 if b else (2 if a else 3)`.
            let cons = emit_python(consequent);
            let cons = if matches!(consequent.as_ref(), Expr::Conditional { .. }) {
                format!("({cons})")
            } else {
                cons
            };
            format!("{cons} if {} else {}", emit_python(condition), emit_python(alternate))
        }
        Expr::Member { object, property } => format!("{}.{property}", wrap_postfix(object, emit_python(object))),
        Expr::Index { object, index } => format!("{}[{}]", wrap_postfix(object, emit_python(object)), emit_python(index)),
        Expr::Call { callee, args } => {
            let a: Vec<_> = args.iter().map(|a| emit_python(a)).collect();
            format!("{}({})", wrap_postfix(callee, emit_python(callee)), a.join(", "))
        }
    }
}

fn python_binop(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+", BinOp::Sub => "-", BinOp::Mul => "*", BinOp::Div => "/", BinOp::Mod => "%",
        BinOp::StrictEq => "==", BinOp::StrictNeq => "!=",
        BinOp::Lt => "<", BinOp::Gt => ">", BinOp::LtEq => "<=", BinOp::GtEq => ">=",
        BinOp::And => "and", BinOp::Or => "or",
        BinOp::BitAnd => "&", BinOp::BitOr => "|", BinOp::BitXor => "^",
        BinOp::Shl => "<<", BinOp::Shr => ">>", BinOp::UShr => ">>",
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Tests
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(test)]
mod tests {
    use super::*;

    // ── Original tests (backwards compatible) ───────────────────

    #[test]
    fn test_transpile_arithmetic_cpp() {
        assert_eq!(transpile("raw * 0.1 - 40.0", ExprTarget::Cpp).unwrap(), "raw * 0.1 - 40.0");
    }

    #[test]
    fn test_transpile_strict_equality_cpp() {
        assert_eq!(transpile("status === 'OK'", ExprTarget::Cpp).unwrap(), "status == \"OK\"");
    }

    #[test]
    fn test_transpile_logical_cpp() {
        assert_eq!(transpile("engineStop && ignOn", ExprTarget::Cpp).unwrap(), "engineStop && ignOn");
    }

    #[test]
    fn test_transpile_comparison_rust() {
        assert_eq!(transpile("rpm > 8000", ExprTarget::Rust).unwrap(), "rpm > 8000");
    }

    #[test]
    fn test_transpile_ternary_rust() {
        assert_eq!(
            transpile("status === 'OK' ? 1 : 0", ExprTarget::Rust).unwrap(),
            "if status == \"OK\" { 1 } else { 0 }"
        );
    }

    #[test]
    fn test_transpile_logical_python() {
        assert_eq!(transpile("engineStop && ignOn", ExprTarget::Python).unwrap(), "engineStop and ignOn");
    }

    #[test]
    fn test_transpile_booleans_python() {
        assert_eq!(
            transpile("ignition === true && engineStop === false", ExprTarget::Python).unwrap(),
            "ignition == True and engineStop == False"
        );
    }

    #[test]
    fn test_reject_arrow_function() {
        assert!(transpile("() => x + 1", ExprTarget::Cpp).is_err());
    }

    #[test]
    fn test_reject_new() {
        let r = transpile("new Map()", ExprTarget::Cpp);
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("new"));
    }

    #[test]
    fn test_bitwise_cpp() {
        assert_eq!(transpile("raw & 0x0F", ExprTarget::Cpp).unwrap(), "raw & 0x0F");
    }

    #[test]
    fn test_shift_cpp() {
        assert_eq!(transpile("(raw[1] >> 4) & 0x0F", ExprTarget::Cpp).unwrap(), "raw[1] >> 4 & 0x0F");
    }

    #[test]
    fn test_string_literal_not_flagged() {
        assert_eq!(transpile("status === 'new'", ExprTarget::Cpp).unwrap(), "status == \"new\"");
    }

    #[test]
    fn test_string_literal_contents_preserved() {
        assert_eq!(transpile("x === 'a === b'", ExprTarget::Cpp).unwrap(), "x == \"a === b\"");
    }

    #[test]
    fn test_go_rejects_ternary() {
        let r = transpile("x > 0 ? 1 : 0", ExprTarget::Go);
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("ternary"));
    }

    #[test]
    fn test_kotlin_bitwise_shift() {
        assert_eq!(transpile("(byte >> 4) & 0x0F", ExprTarget::Kotlin).unwrap(), "byte shr 4 and 0x0F");
    }

    #[test]
    fn test_kotlin_bitwise_preserves_logical() {
        assert_eq!(transpile("a && b || c", ExprTarget::Kotlin).unwrap(), "a && b || c");
    }

    #[test]
    fn test_kotlin_bitwise_mixed() {
        assert_eq!(
            transpile("(x >> 4) & 0x0F && y === true", ExprTarget::Kotlin).unwrap(),
            "x shr 4 and 0x0F && y == true"
        );
    }

    #[test]
    fn test_kotlin_left_shift() {
        assert_eq!(transpile("x << 8", ExprTarget::Kotlin).unwrap(), "x shl 8");
    }

    #[test]
    fn test_kotlin_unsigned_shift() {
        assert_eq!(transpile("x >>> 4", ExprTarget::Kotlin).unwrap(), "x ushr 4");
    }

    #[test]
    fn test_kotlin_xor() {
        assert_eq!(transpile("a ^ b", ExprTarget::Kotlin).unwrap(), "a xor b");
    }

    #[test]
    fn test_kotlin_bitwise_not() {
        assert_eq!(transpile("~mask", ExprTarget::Kotlin).unwrap(), "mask.inv()");
    }

    // ── Complex expressions ─────────────────────────────────────

    #[test]
    fn test_nested_ternary_rust() {
        assert_eq!(
            transpile("a > 0 ? (b > 1 ? 2 : 3) : 4", ExprTarget::Rust).unwrap(),
            "if a > 0 { if b > 1 { 2 } else { 3 } } else { 4 }"
        );
    }

    #[test]
    fn test_nested_bitwise_not_kotlin() {
        assert_eq!(transpile("~(a & (b | c))", ExprTarget::Kotlin).unwrap(), "(a and (b or c)).inv()");
    }

    #[test]
    fn test_chained_member_access() {
        assert_eq!(transpile("_event.data.payload", ExprTarget::Cpp).unwrap(), "_event.data.payload");
    }

    #[test]
    fn test_function_call_multiple_args() {
        assert_eq!(transpile("computeKey(seed, 0x01)", ExprTarget::Cpp).unwrap(), "computeKey(seed, 0x01)");
    }

    #[test]
    fn test_method_call_on_member() {
        assert_eq!(
            transpile("securityResponse.decode(_event.data)", ExprTarget::Cpp).unwrap(),
            "securityResponse.decode(_event.data)"
        );
    }

    #[test]
    fn test_complex_codec_expression() {
        assert_eq!(
            transpile("(raw[2] << 16) | (raw[3] << 8) | raw[4]", ExprTarget::Cpp).unwrap(),
            "raw[2] << 16 | raw[3] << 8 | raw[4]"
        );
    }

    #[test]
    fn test_complex_codec_kotlin() {
        // In Kotlin, shl and or are infix functions at the same precedence.
        // Right-child parens required to prevent left-to-right misparsing.
        assert_eq!(
            transpile("(raw[2] << 16) | (raw[3] << 8) | raw[4]", ExprTarget::Kotlin).unwrap(),
            "raw[2] shl 16 or (raw[3] shl 8) or raw[4]"
        );
    }

    #[test]
    fn test_operator_precedence_bitwise_vs_comparison() {
        assert_eq!(transpile("a & 0xFF === b", ExprTarget::Cpp).unwrap(), "a & 0xFF == b");
    }

    #[test]
    fn test_nested_function_calls() {
        assert_eq!(transpile("encode(computeKey(seed), 0x02)", ExprTarget::Rust).unwrap(), "encode(computeKey(seed), 0x02)");
    }

    #[test]
    fn test_unary_in_binary() {
        assert_eq!(transpile("-a + b", ExprTarget::Cpp).unwrap(), "-a + b");
    }

    #[test]
    fn test_reject_loose_equality() {
        let r = transpile("x == y", ExprTarget::Cpp);
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("==="));
    }

    #[test]
    fn test_reject_loose_inequality() {
        let r = transpile("x != y", ExprTarget::Cpp);
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("!=="));
    }

    #[test]
    fn test_hex_literal() {
        assert_eq!(transpile("0xF190", ExprTarget::Cpp).unwrap(), "0xF190");
    }

    #[test]
    fn test_python_ternary() {
        assert_eq!(transpile("x > 0 ? 1 : 0", ExprTarget::Python).unwrap(), "1 if x > 0 else 0");
    }

    #[test]
    fn test_python_not() {
        assert_eq!(transpile("!valid", ExprTarget::Python).unwrap(), "not valid");
    }

    #[test]
    fn test_go_bitwise_not() {
        assert_eq!(transpile("~mask", ExprTarget::Go).unwrap(), "^mask");
    }

    #[test]
    fn test_encode_method_with_args() {
        assert_eq!(
            transpile("writeDataRequest.encode(0xF190, writeData)", ExprTarget::Cpp).unwrap(),
            "writeDataRequest.encode(0xF190, writeData)"
        );
    }

    #[test]
    fn test_complex_guard_expression() {
        assert_eq!(
            transpile("engineStatus === 'STOP' && ignition === true", ExprTarget::Kotlin).unwrap(),
            "engineStatus == \"STOP\" && ignition == true"
        );
    }

    #[test]
    fn test_null_literal() {
        assert_eq!(transpile("null", ExprTarget::Cpp).unwrap(), "nullptr");
        assert_eq!(transpile("null", ExprTarget::Rust).unwrap(), "None");
        assert_eq!(transpile("null", ExprTarget::Python).unwrap(), "None");
        assert_eq!(transpile("null", ExprTarget::Go).unwrap(), "nil");
        assert_eq!(transpile("null", ExprTarget::Kotlin).unwrap(), "null");
    }

    // ── Precedence correctness ──────────────────────────────────

    #[test]
    fn test_deeply_nested_parens() {
        assert_eq!(transpile("((a + b) * (c - d))", ExprTarget::Cpp).unwrap(), "(a + b) * (c - d)");
    }

    #[test]
    fn test_precedence_add_in_mul_right() {
        assert_eq!(transpile("a * (b + c)", ExprTarget::Cpp).unwrap(), "a * (b + c)");
    }

    #[test]
    fn test_precedence_sub_associativity() {
        assert_eq!(transpile("a - (b - c)", ExprTarget::Cpp).unwrap(), "a - (b - c)");
    }

    #[test]
    fn test_precedence_left_assoc_no_parens() {
        assert_eq!(transpile("(a - b) - c", ExprTarget::Cpp).unwrap(), "a - b - c");
    }

    #[test]
    fn test_kotlin_precedence_eq_vs_infix() {
        assert_eq!(transpile("a & (b === c)", ExprTarget::Kotlin).unwrap(), "a and (b == c)");
    }

    #[test]
    fn test_python_precedence_eq_vs_bitwise() {
        assert_eq!(transpile("a & (b === c)", ExprTarget::Python).unwrap(), "a & (b == c)");
    }

    #[test]
    fn test_go_precedence_bitwise_vs_comparison() {
        // Go: & (prec 5) > == (prec 3), opposite of ECMAScript.
        // ECMAScript AST: BitAnd(a, StrictEq(0xFF, b)) means `a & (0xFF === b)`.
        // Go without parens: `a & 0xFF == b` → Go parses as `(a & 0xFF) == b` (wrong).
        // Emitter must add parens: `a & (0xFF == b)`.
        assert_eq!(transpile("a & 0xFF === b", ExprTarget::Go).unwrap(), "a & (0xFF == b)");
    }

    #[test]
    fn test_go_precedence_bitor_vs_comparison() {
        // Go: | (prec 4) > == (prec 3), opposite of ECMAScript.
        assert_eq!(transpile("a | b === c", ExprTarget::Go).unwrap(), "a | (b == c)");
    }

    #[test]
    fn test_rust_precedence_bitwise_vs_comparison() {
        // Rust: & > ^ > | > == (bitwise above comparison, opposite of ECMAScript/C++).
        // ECMAScript `a & (b === c)` must keep parens in Rust.
        assert_eq!(transpile("a & (b === c)", ExprTarget::Rust).unwrap(), "a & (b == c)");
    }

    #[test]
    fn test_rust_precedence_bitor_vs_comparison() {
        // Rust: | > == (bitwise OR above comparison).
        assert_eq!(transpile("a | b === c", ExprTarget::Rust).unwrap(), "a | (b == c)");
    }

    #[test]
    fn test_rust_precedence_implicit_from_ecma() {
        // ECMAScript: `a & 0xFF === b` means `a & (0xFF === b)` (=== binds tighter).
        // Rust emitter must add parens because Rust has & > ==.
        assert_eq!(transpile("a & 0xFF === b", ExprTarget::Rust).unwrap(), "a & (0xFF == b)");
    }

    #[test]
    fn test_python_nested_ternary_consequent() {
        // ECMAScript: `a ? (b ? 1 : 2) : 3` → consequent is itself a ternary.
        // Python must wrap consequent: `(1 if b else 2) if a else 3`.
        assert_eq!(
            transpile("a ? (b ? 1 : 2) : 3", ExprTarget::Python).unwrap(),
            "(1 if b else 2) if a else 3"
        );
    }

    #[test]
    fn test_python_nested_ternary_alternate() {
        // Alternate ternary is naturally right-associative in Python — no parens needed.
        assert_eq!(
            transpile("a ? 1 : b ? 2 : 3", ExprTarget::Python).unwrap(),
            "1 if a else 2 if b else 3"
        );
    }

    // ── Tokenizer edge cases ────────────────────────────────────

    #[test]
    fn test_float_starting_with_dot() {
        assert_eq!(transpile(".5 + 1", ExprTarget::Cpp).unwrap(), "0.5 + 1");
    }

    #[test]
    fn test_scientific_notation() {
        assert_eq!(transpile("1e3 + 2.5e-1", ExprTarget::Cpp).unwrap(), "1e3 + 2.5e-1");
    }

    #[test]
    fn test_kotlin_conditional() {
        assert_eq!(transpile("x > 0 ? 1 : 0", ExprTarget::Kotlin).unwrap(), "if (x > 0) 1 else 0");
    }

    // ── Parser/emitter edge cases ───────────────────────────────

    #[test]
    fn test_empty_call() {
        assert_eq!(transpile("f()", ExprTarget::Cpp).unwrap(), "f()");
    }

    #[test]
    fn test_chained_index() {
        assert_eq!(transpile("a[0][1]", ExprTarget::Cpp).unwrap(), "a[0][1]");
    }

    #[test]
    fn test_double_not() {
        assert_eq!(transpile("!!valid", ExprTarget::Cpp).unwrap(), "!!valid");
        assert_eq!(transpile("!!valid", ExprTarget::Python).unwrap(), "not not valid");
    }

    #[test]
    fn test_reject_arrow_function_explicit() {
        let r = transpile("x => x + 1", ExprTarget::Cpp);
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("arrow function"));
    }

    #[test]
    fn test_reject_nullish_coalescing() {
        let r = transpile("a ?? b", ExprTarget::Cpp);
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("nullish coalescing"));
    }

    #[test]
    fn test_reject_optional_chaining() {
        let r = transpile("a?.b", ExprTarget::Cpp);
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("optional chaining"));
    }

    #[test]
    fn test_reject_spread() {
        let r = transpile("f(...args)", ExprTarget::Cpp);
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("spread/rest"));
    }

    #[test]
    fn test_reject_template_literal() {
        let r = transpile("`hello ${x}`", ExprTarget::Cpp);
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("template literal"));
    }

    #[test]
    fn test_double_bitnot() {
        assert_eq!(transpile("~~mask", ExprTarget::Cpp).unwrap(), "~~mask");
        assert_eq!(transpile("~~mask", ExprTarget::Kotlin).unwrap(), "(mask.inv()).inv()");
    }
}
