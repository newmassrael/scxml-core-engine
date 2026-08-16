// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
//! Recursive-descent parser for the W3C SCXML ECMAScript subset.
//!
//! Reads the token stream [`crate::forge::expr::tokenize_as`] produces in
//! [`LexMode::EcmaScript`], so the two dialects cannot disagree about what
//! a string literal, a numeric literal or an identifier is.
//!
//! Precedence follows ECMA-262 §11 (lowest binding first):
//!
//! ```text
//!   assignment    =  +=  -=  *=  /=  %=        (right-associative)
//!   conditional   ? :
//!   logical or    ||
//!   logical and   &&
//!   bitwise       |  ^  &
//!   equality      ===  !==  ==  !=
//!   relational    <  >  <=  >=  instanceof  in
//!   shift         <<  >>  >>>
//!   additive      +  -
//!   multiplicative *  /  %
//!   unary         -  +  !  ~  typeof  ++  --
//!   postfix       .  []  ()  ++  --
//!   primary       literals, identifiers, this, ( ), [ ], { }, function, new
//! ```

use super::{BinOp, Expr, LogicalOp, Stmt, UnaryOp, UpdateOp};
use crate::forge::error::ExprError;
use crate::forge::expr::{tokenize_as, LexMode, Token};

/// Parse a single ECMAScript expression — the whole input must be consumed.
pub fn parse_expression(source: &str) -> Result<Expr, ExprError> {
    // §scxml-B-2: the accepted language is ECMAScript, so this reads the
    // ECMAScript grammar rather than pattern-matching the shapes the W3C
    // test corpus happens to use.
    let trimmed = source.trim();
    if trimmed.is_empty() {
        return Err(ExprError::Empty { what: "expression" });
    }
    let tokens = tokenize_as(trimmed, LexMode::EcmaScript)?;
    let mut parser = Parser::new(&tokens);
    let expr = parser.parse_assignment()?;
    // A trailing `;` is what `<data expr="new testobject();">` writes: the
    // author copied a statement into an attribute that takes an expression.
    // W3C's own corpus does it, so it is accepted and dropped rather than
    // rejected.
    if parser.at(&Token::Semi) {
        parser.advance();
    }
    parser.expect_end()?;
    Ok(expr)
}

/// Parse a `<script>` body — a statement list.
pub fn parse_script(source: &str) -> Result<Vec<Stmt>, ExprError> {
    let tokens = tokenize_as(source, LexMode::EcmaScript)?;
    let mut parser = Parser::new(&tokens);
    let stmts = parser.parse_statements_until(&Token::Eof)?;
    parser.expect_end()?;
    Ok(stmts)
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    fn peek_at(&self, ahead: usize) -> &Token {
        self.tokens.get(self.pos + ahead).unwrap_or(&Token::Eof)
    }

    fn at(&self, token: &Token) -> bool {
        self.peek() == token
    }

    fn at_word(&self, word: &str) -> bool {
        matches!(self.peek(), Token::Ident(s) if s == word)
    }

    fn advance(&mut self) -> Token {
        let token = self.tokens.get(self.pos).cloned().unwrap_or(Token::Eof);
        self.pos += 1;
        token
    }

    fn eat(&mut self, token: &Token) -> bool {
        if self.at(token) {
            self.pos += 1;
            return true;
        }
        false
    }

    fn eat_word(&mut self, word: &str) -> bool {
        if self.at_word(word) {
            self.pos += 1;
            return true;
        }
        false
    }

    fn expect(&mut self, expected: &Token) -> Result<(), ExprError> {
        let got = self.advance();
        if &got == expected {
            return Ok(());
        }
        Err(ExprError::ParseMismatch {
            expected: format!("'{expected}'"),
            got: got.to_string(),
        })
    }

    fn expect_ident(&mut self) -> Result<String, ExprError> {
        match self.advance() {
            Token::Ident(name) => Ok(name),
            other => Err(ExprError::ParseMismatch {
                expected: "identifier".into(),
                got: other.to_string(),
            }),
        }
    }

    fn expect_end(&mut self) -> Result<(), ExprError> {
        if self.at(&Token::Eof) {
            return Ok(());
        }
        Err(ExprError::UnexpectedToken {
            token: self.peek().to_string(),
        })
    }

    // ── Statements ──────────────────────────────────────────────────

    fn parse_statements_until(&mut self, end: &Token) -> Result<Vec<Stmt>, ExprError> {
        let mut stmts = Vec::new();
        while !self.at(end) && !self.at(&Token::Eof) {
            stmts.push(self.parse_statement()?);
        }
        Ok(stmts)
    }

    /// One statement, or a braced block flattened into the caller's list.
    fn parse_statement(&mut self) -> Result<Stmt, ExprError> {
        if self.eat(&Token::Semi) {
            return Ok(Stmt::Empty);
        }
        if self.at(&Token::LBrace) {
            // A bare block. ECMAScript scopes `var` to the function, and so
            // does the Lua this emits (the body is spliced inline), so the
            // braces carry no meaning of their own here.
            self.advance();
            let body = self.parse_statements_until(&Token::RBrace)?;
            self.expect(&Token::RBrace)?;
            return Ok(Stmt::If {
                condition: Expr::Bool(true),
                consequent: body,
                alternate: Vec::new(),
            });
        }
        if self.at_word("var") || self.at_word("let") || self.at_word("const") {
            self.advance();
            let mut bindings = Vec::new();
            loop {
                let name = self.expect_ident()?;
                let init = if self.eat(&Token::Assign) {
                    Some(self.parse_assignment()?)
                } else {
                    None
                };
                bindings.push((name, init));
                if !self.eat(&Token::Comma) {
                    break;
                }
            }
            self.eat(&Token::Semi);
            return Ok(Stmt::VarDecl(bindings));
        }
        if self.at_word("if") {
            self.advance();
            self.expect(&Token::LParen)?;
            let condition = self.parse_assignment()?;
            self.expect(&Token::RParen)?;
            let consequent = self.parse_block_or_statement()?;
            let alternate = if self.eat_word("else") {
                self.parse_block_or_statement()?
            } else {
                Vec::new()
            };
            return Ok(Stmt::If {
                condition,
                consequent,
                alternate,
            });
        }
        if self.at_word("while") {
            self.advance();
            self.expect(&Token::LParen)?;
            let condition = self.parse_assignment()?;
            self.expect(&Token::RParen)?;
            let body = self.parse_block_or_statement()?;
            return Ok(Stmt::While { condition, body });
        }
        if self.at_word("for") {
            return self.parse_for();
        }
        if self.at_word("return") {
            self.advance();
            let value = if self.at(&Token::Semi) || self.at(&Token::RBrace) || self.at(&Token::Eof)
            {
                None
            } else {
                Some(self.parse_assignment()?)
            };
            self.eat(&Token::Semi);
            return Ok(Stmt::Return(value));
        }
        if self.at_word("break") {
            self.advance();
            self.eat(&Token::Semi);
            return Ok(Stmt::Break);
        }
        if self.at_word("continue") {
            self.advance();
            self.eat(&Token::Semi);
            return Ok(Stmt::Continue);
        }
        // A named function *declaration*; `function (…)` with no name is an
        // expression and falls through to the expression statement below.
        if self.at_word("function") && matches!(self.peek_at(1), Token::Ident(_)) {
            self.advance();
            let name = self.expect_ident()?;
            let params = self.parse_params()?;
            let body = self.parse_braced_body()?;
            return Ok(Stmt::FunctionDecl { name, params, body });
        }
        let expr = self.parse_assignment()?;
        self.eat(&Token::Semi);
        Ok(Stmt::Expr(expr))
    }

    /// The body of an `if`/`while`/`for`: either a braced block or the
    /// single statement ECMAScript allows in its place.
    fn parse_block_or_statement(&mut self) -> Result<Vec<Stmt>, ExprError> {
        if self.at(&Token::LBrace) {
            self.advance();
            let body = self.parse_statements_until(&Token::RBrace)?;
            self.expect(&Token::RBrace)?;
            return Ok(body);
        }
        Ok(vec![self.parse_statement()?])
    }

    fn parse_for(&mut self) -> Result<Stmt, ExprError> {
        self.advance(); // `for`
        self.expect(&Token::LParen)?;

        // `for (var k in o)` / `for (k in o)` — decided by looking for the
        // `in` that follows the binding rather than by parsing the init and
        // backtracking, so the two forms never half-consume each other.
        let declares = self.at_word("var") || self.at_word("let") || self.at_word("const");
        let name_at = if declares { 1 } else { 0 };
        if matches!(self.peek_at(name_at), Token::Ident(_))
            && matches!(self.peek_at(name_at + 1), Token::Ident(w) if w == "in")
        {
            if declares {
                self.advance();
            }
            let name = self.expect_ident()?;
            self.advance(); // `in`
            let object = self.parse_assignment()?;
            self.expect(&Token::RParen)?;
            let body = self.parse_block_or_statement()?;
            return Ok(Stmt::ForIn {
                declares,
                name,
                object,
                body,
            });
        }

        let init = if self.at(&Token::Semi) {
            self.advance();
            None
        } else {
            let stmt = self.parse_statement()?;
            // `parse_statement` consumes the `;` that closes the init.
            Some(Box::new(stmt))
        };
        let test = if self.at(&Token::Semi) {
            None
        } else {
            Some(self.parse_assignment()?)
        };
        self.expect(&Token::Semi)?;
        let update = if self.at(&Token::RParen) {
            None
        } else {
            Some(self.parse_assignment()?)
        };
        self.expect(&Token::RParen)?;
        let body = self.parse_block_or_statement()?;
        Ok(Stmt::For {
            init,
            test,
            update,
            body,
        })
    }

    fn parse_params(&mut self) -> Result<Vec<String>, ExprError> {
        self.expect(&Token::LParen)?;
        let mut params = Vec::new();
        if !self.at(&Token::RParen) {
            loop {
                params.push(self.expect_ident()?);
                if !self.eat(&Token::Comma) {
                    break;
                }
            }
        }
        self.expect(&Token::RParen)?;
        Ok(params)
    }

    fn parse_braced_body(&mut self) -> Result<Vec<Stmt>, ExprError> {
        self.expect(&Token::LBrace)?;
        let body = self.parse_statements_until(&Token::RBrace)?;
        self.expect(&Token::RBrace)?;
        Ok(body)
    }

    // ── Expressions ─────────────────────────────────────────────────

    fn parse_assignment(&mut self) -> Result<Expr, ExprError> {
        let left = self.parse_conditional()?;
        let op = match self.peek() {
            Token::Assign => None,
            Token::OpAssign(op) => Some(map_arith(*op)?),
            _ => return Ok(left),
        };
        self.advance();
        let value = self.parse_assignment()?;
        Ok(Expr::Assign {
            op,
            target: Box::new(left),
            value: Box::new(value),
        })
    }

    fn parse_conditional(&mut self) -> Result<Expr, ExprError> {
        let condition = self.parse_logical_or()?;
        if !self.eat(&Token::Question) {
            return Ok(condition);
        }
        let consequent = self.parse_assignment()?;
        self.expect(&Token::Colon)?;
        let alternate = self.parse_assignment()?;
        Ok(Expr::Conditional {
            condition: Box::new(condition),
            consequent: Box::new(consequent),
            alternate: Box::new(alternate),
        })
    }

    fn parse_logical_or(&mut self) -> Result<Expr, ExprError> {
        let mut left = self.parse_logical_and()?;
        while self.eat(&Token::PipePipe) {
            let right = self.parse_logical_and()?;
            left = Expr::Logical {
                op: LogicalOp::Or,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_logical_and(&mut self) -> Result<Expr, ExprError> {
        let mut left = self.parse_bitwise_or()?;
        while self.eat(&Token::AmpAmp) {
            let right = self.parse_bitwise_or()?;
            left = Expr::Logical {
                op: LogicalOp::And,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_bitwise_or(&mut self) -> Result<Expr, ExprError> {
        let mut left = self.parse_bitwise_xor()?;
        while self.eat(&Token::Pipe) {
            let right = self.parse_bitwise_xor()?;
            left = binary(BinOp::BitOr, left, right);
        }
        Ok(left)
    }

    fn parse_bitwise_xor(&mut self) -> Result<Expr, ExprError> {
        let mut left = self.parse_bitwise_and()?;
        while self.eat(&Token::Caret) {
            let right = self.parse_bitwise_and()?;
            left = binary(BinOp::BitXor, left, right);
        }
        Ok(left)
    }

    fn parse_bitwise_and(&mut self) -> Result<Expr, ExprError> {
        let mut left = self.parse_equality()?;
        while self.eat(&Token::Amp) {
            let right = self.parse_equality()?;
            left = binary(BinOp::BitAnd, left, right);
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, ExprError> {
        let mut left = self.parse_relational()?;
        loop {
            let op = match self.peek() {
                Token::StrictEq => BinOp::StrictEq,
                Token::StrictNeq => BinOp::StrictNeq,
                Token::LooseEq => BinOp::LooseEq,
                Token::LooseNeq => BinOp::LooseNeq,
                _ => return Ok(left),
            };
            self.advance();
            let right = self.parse_relational()?;
            left = binary(op, left, right);
        }
    }

    fn parse_relational(&mut self) -> Result<Expr, ExprError> {
        let mut left = self.parse_shift()?;
        loop {
            let op = match self.peek() {
                Token::Lt => BinOp::Lt,
                Token::Gt => BinOp::Gt,
                Token::LtEq => BinOp::LtEq,
                Token::GtEq => BinOp::GtEq,
                Token::Ident(w) if w == "instanceof" => BinOp::InstanceOf,
                Token::Ident(w) if w == "in" => BinOp::In,
                _ => return Ok(left),
            };
            self.advance();
            let right = self.parse_shift()?;
            left = binary(op, left, right);
        }
    }

    fn parse_shift(&mut self) -> Result<Expr, ExprError> {
        let mut left = self.parse_additive()?;
        loop {
            let op = match self.peek() {
                Token::Shl => BinOp::Shl,
                Token::Shr => BinOp::Shr,
                Token::UShr => BinOp::UShr,
                _ => return Ok(left),
            };
            self.advance();
            let right = self.parse_additive()?;
            left = binary(op, left, right);
        }
    }

    fn parse_additive(&mut self) -> Result<Expr, ExprError> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = match self.peek() {
                Token::Plus => BinOp::Add,
                Token::Minus => BinOp::Sub,
                _ => return Ok(left),
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            left = binary(op, left, right);
        }
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, ExprError> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Token::Star => BinOp::Mul,
                Token::Slash => BinOp::Div,
                Token::Percent => BinOp::Mod,
                _ => return Ok(left),
            };
            self.advance();
            let right = self.parse_unary()?;
            left = binary(op, left, right);
        }
    }

    fn parse_unary(&mut self) -> Result<Expr, ExprError> {
        let op = match self.peek() {
            Token::Minus => Some(UnaryOp::Neg),
            Token::Plus => Some(UnaryOp::Pos),
            Token::Bang => Some(UnaryOp::Not),
            Token::Tilde => Some(UnaryOp::BitNot),
            Token::Ident(w) if w == "typeof" => Some(UnaryOp::TypeOf),
            _ => None,
        };
        if let Some(op) = op {
            self.advance();
            let operand = self.parse_unary()?;
            return Ok(Expr::Unary {
                op,
                operand: Box::new(operand),
            });
        }
        let update = match self.peek() {
            Token::PlusPlus => Some(UpdateOp::Inc),
            Token::MinusMinus => Some(UpdateOp::Dec),
            _ => None,
        };
        if let Some(op) = update {
            self.advance();
            let target = self.parse_unary()?;
            return Ok(Expr::Update {
                op,
                prefix: true,
                target: Box::new(target),
            });
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, ExprError> {
        let mut expr = self.parse_primary()?;
        loop {
            match self.peek() {
                Token::Dot => {
                    self.advance();
                    let property = self.expect_ident()?;
                    expr = Expr::Member {
                        object: Box::new(expr),
                        property,
                    };
                }
                Token::LBracket => {
                    self.advance();
                    let index = self.parse_assignment()?;
                    self.expect(&Token::RBracket)?;
                    // ECMA-262 11.2.1 defines `obj.name` as `obj["name"]`
                    // — one operation, and the dot is the sugar. A
                    // literal key is folded into the same node here so
                    // that everything downstream sees one shape: the
                    // emitter's rules hang off the property being named,
                    // and while the two spellings arrived as two nodes,
                    // `t['length']` reached a nil where `t.length` was
                    // measured and `_event['name']()` generated cleanly
                    // where `_event.name()` was refused.
                    //
                    // The lowering is unchanged for every key that is
                    // not one of those names: a `Member` this datamodel
                    // has no rule for emits `obj["key"]` through the
                    // same encoder `Index` used.
                    expr = match index {
                        Expr::Str(property) => Expr::Member {
                            object: Box::new(expr),
                            property,
                        },
                        index => Expr::Index {
                            object: Box::new(expr),
                            index: Box::new(index),
                        },
                    };
                }
                Token::LParen => {
                    let args = self.parse_arguments()?;
                    expr = Expr::Call {
                        callee: Box::new(expr),
                        args,
                    };
                }
                Token::PlusPlus => {
                    self.advance();
                    expr = Expr::Update {
                        op: UpdateOp::Inc,
                        prefix: false,
                        target: Box::new(expr),
                    };
                }
                Token::MinusMinus => {
                    self.advance();
                    expr = Expr::Update {
                        op: UpdateOp::Dec,
                        prefix: false,
                        target: Box::new(expr),
                    };
                }
                _ => return Ok(expr),
            }
        }
    }

    fn parse_arguments(&mut self) -> Result<Vec<Expr>, ExprError> {
        self.expect(&Token::LParen)?;
        let mut args = Vec::new();
        if !self.at(&Token::RParen) {
            loop {
                args.push(self.parse_assignment()?);
                if !self.eat(&Token::Comma) {
                    break;
                }
            }
        }
        self.expect(&Token::RParen)?;
        Ok(args)
    }

    fn parse_primary(&mut self) -> Result<Expr, ExprError> {
        match self.peek().clone() {
            Token::Number(n) => {
                self.advance();
                Ok(Expr::Number(n))
            }
            Token::String { value, .. } => {
                self.advance();
                Ok(Expr::Str(decode_escapes(&value)))
            }
            Token::LParen => {
                self.advance();
                let inner = self.parse_assignment()?;
                self.expect(&Token::RParen)?;
                Ok(inner)
            }
            Token::LBracket => {
                self.advance();
                let mut items = Vec::new();
                if !self.at(&Token::RBracket) {
                    loop {
                        items.push(self.parse_assignment()?);
                        if !self.eat(&Token::Comma) {
                            break;
                        }
                        // `[1, 2, ]` — a trailing comma is legal ECMAScript
                        // and carries no element.
                        if self.at(&Token::RBracket) {
                            break;
                        }
                    }
                }
                self.expect(&Token::RBracket)?;
                Ok(Expr::Array(items))
            }
            Token::LBrace => {
                self.advance();
                let mut props = Vec::new();
                if !self.at(&Token::RBrace) {
                    loop {
                        let key = match self.advance() {
                            Token::Ident(name) => name,
                            Token::String { value, .. } => decode_escapes(&value),
                            Token::Number(n) => n,
                            other => {
                                return Err(ExprError::ParseMismatch {
                                    expected: "property name".into(),
                                    got: other.to_string(),
                                })
                            }
                        };
                        self.expect(&Token::Colon)?;
                        let value = self.parse_assignment()?;
                        props.push((key, value));
                        if !self.eat(&Token::Comma) {
                            break;
                        }
                        if self.at(&Token::RBrace) {
                            break;
                        }
                    }
                }
                self.expect(&Token::RBrace)?;
                Ok(Expr::Object(props))
            }
            Token::Ident(word) => {
                match word.as_str() {
                    "true" => {
                        self.advance();
                        return Ok(Expr::Bool(true));
                    }
                    "false" => {
                        self.advance();
                        return Ok(Expr::Bool(false));
                    }
                    "null" | "undefined" => {
                        self.advance();
                        return Ok(Expr::Nullish);
                    }
                    "this" => {
                        self.advance();
                        return Ok(Expr::This);
                    }
                    "function" => {
                        self.advance();
                        let name = match self.peek() {
                            Token::Ident(_) => Some(self.expect_ident()?),
                            _ => None,
                        };
                        let params = self.parse_params()?;
                        let body = self.parse_braced_body()?;
                        return Ok(Expr::Function { name, params, body });
                    }
                    "new" => {
                        self.advance();
                        // `new a.b.C(args)` — the callee is a member path,
                        // and the argument list belongs to the `new`, not to
                        // a call applied to its result.
                        let mut callee = match self.advance() {
                            Token::Ident(name) => Expr::Ident(name),
                            other => {
                                return Err(ExprError::ParseMismatch {
                                    expected: "constructor name after 'new'".into(),
                                    got: other.to_string(),
                                })
                            }
                        };
                        while self.eat(&Token::Dot) {
                            let property = self.expect_ident()?;
                            callee = Expr::Member {
                                object: Box::new(callee),
                                property,
                            };
                        }
                        let args = if self.at(&Token::LParen) {
                            self.parse_arguments()?
                        } else {
                            Vec::new()
                        };
                        return Ok(Expr::New {
                            callee: Box::new(callee),
                            args,
                        });
                    }
                    _ => {}
                }
                // ECMA-262 12.7.2: a reserved word is not an identifier, so
                // an expression that uses one where a value belongs is a
                // syntax error. W3C test 344 (`cond="return"`) is exactly
                // that, and §5.9.1 says an unevaluable cond raises
                // error.execution and reads as false — so this rejection is
                // what the *seam* turns into that runtime error, rather than
                // something the emitter quietly renames into a global.
                if is_reserved_word(&word) {
                    return Err(ExprError::UnsupportedConstruct {
                        construct: format!("reserved word '{word}' used as a value"),
                    });
                }
                self.advance();
                Ok(Expr::Ident(word))
            }
            other => Err(ExprError::UnexpectedToken {
                token: other.to_string(),
            }),
        }
    }
}

/// ECMA-262 12.7.2 reserved words, minus the ones this parser handles as
/// syntax of their own (`this`, `new`, `typeof`, `function`, `instanceof`,
/// `in`, `var`, `null`, `true`, `false`) and minus the statement keywords a
/// statement position consumes before reaching here.
fn is_reserved_word(word: &str) -> bool {
    const RESERVED: &[&str] = &[
        "await", "break", "case", "catch", "class", "const", "continue", "debugger", "default",
        "delete", "do", "else", "enum", "export", "extends", "finally", "for", "if", "import",
        "let", "return", "static", "super", "switch", "throw", "try", "void", "while", "with",
        "yield",
    ];
    RESERVED.contains(&word)
}

fn binary(op: BinOp, left: Expr, right: Expr) -> Expr {
    Expr::Binary {
        op,
        left: Box::new(left),
        right: Box::new(right),
    }
}

/// Map the arithmetic operator a compound assignment carries from the
/// shared lexer's [`crate::forge::expr::BinOp`] to this dialect's.
fn map_arith(op: crate::forge::expr::BinOp) -> Result<BinOp, ExprError> {
    use crate::forge::expr::BinOp as Shared;
    Ok(match op {
        Shared::Add => BinOp::Add,
        Shared::Sub => BinOp::Sub,
        Shared::Mul => BinOp::Mul,
        Shared::Div => BinOp::Div,
        Shared::Mod => BinOp::Mod,
        other => {
            return Err(ExprError::UnsupportedConstruct {
                construct: format!("compound assignment with {other:?}"),
            })
        }
    })
}

/// Decode ECMAScript string escapes to the characters they denote.
///
/// Done here, once, so the emitter re-encodes for Lua from actual
/// characters. The alternative — passing the source spelling through and
/// hoping the two escape grammars coincide — is what made `\uXXXX` and
/// `\/` produce Lua that would not compile.
fn decode_escapes(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        let Some(escape) = chars.next() else {
            out.push('\\');
            break;
        };
        match escape {
            'n' => out.push('\n'),
            't' => out.push('\t'),
            'r' => out.push('\r'),
            'b' => out.push('\u{8}'),
            'f' => out.push('\u{c}'),
            'v' => out.push('\u{b}'),
            '0' => out.push('\0'),
            'x' => {
                let hex: String = (0..2).filter_map(|_| chars.next()).collect();
                match u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                    Some(c) => out.push(c),
                    None => {
                        out.push('x');
                        out.push_str(&hex);
                    }
                }
            }
            'u' => {
                let hex: String = (0..4).filter_map(|_| chars.next()).collect();
                match u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                    Some(c) => out.push(c),
                    None => {
                        out.push('u');
                        out.push_str(&hex);
                    }
                }
            }
            // `\'`, `\"`, `\\`, `\/` and any other escape denote the
            // character itself.
            other => out.push(other),
        }
    }
    out
}
