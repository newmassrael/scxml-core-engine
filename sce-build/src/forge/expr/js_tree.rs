// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The ECMAScript a forge expression is written as for the Interpreter's
//! script engine, as a tree of the ECMAScript expressions it is made of, and
//! the one printer that writes such a tree as text.
//!
//! The ECMAScript emitter ([`super::emit_js`]) builds this tree; nothing is
//! read back out of text. A call of the runtime library (a checked operation,
//! an operation at its width, a conversion to a Number or a BigInt) is a node
//! of its own, whatever operator of the document it stands for.
//!
//! The printer decides every parenthesis, at each parent, from the kind of the
//! child and the precedence ECMA-262 gives its operators by the nesting of its
//! expression productions (ECMAScript Language Specification, "ECMAScript
//! Language: Expressions"): the textbook unparsing rule (Ramsey, "Unparsing
//! Expressions with Prefix and Postfix Operators", 1998). Every binary operator
//! the emitter writes associates to the left, and a conditional to the right.
//! The rules of the grammar that precedence does not see are the printer's too:
//!
//! * the punctuators `--` and `++` are one token each, read by the longest
//!   match ("Punctuators"), so `- -x` written `--x` is a decrement: a `-` or
//!   `+` over the same sign is grouped;
//! * a decimal integer literal followed by `.` reads the dot as its fraction
//!   ("Numeric Literals"), so `1.x` is a syntax error: a decimal integer whose
//!   member is read is grouped;
//! * an expression statement cannot begin with `function` (the lookahead
//!   restriction of the ExpressionStatement production), where a function
//!   declaration would be read, and the Interpreter's engine reads the text it
//!   is handed as a script: a function expression is grouped wherever it would
//!   be the first token of what holds it, and where it would be the first token
//!   of the whole text;
//! * the condition of a conditional is a ShortCircuitExpression and each branch
//!   an AssignmentExpression, so a conditional is grouped as the condition and
//!   bare as either branch.
//!
//! Everything else stands bare. The tests hold the printer to the grammar by
//! reading what it prints back with the forge expression parser, which reads
//! ECMAScript's expression grammar for the operators the printer writes
//! ([`super::parse_to_ast`]), and comparing the tree read with the tree
//! printed.

use std::fmt;

/// An ECMAScript expression the emitter writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum JsExpr {
    /// Text written as it is that stands as one operand: a literal, a name, a
    /// dotted path, or a fragment its writer made one
    /// ([`super::ExprKind::Raw`]).
    Atom(String),
    /// `callee(args)`.
    Call {
        callee: Box<JsExpr>,
        args: Vec<JsExpr>,
    },
    /// `base.name`.
    Member { base: Box<JsExpr>, name: String },
    /// `base[index]`.
    Index {
        base: Box<JsExpr>,
        index: Box<JsExpr>,
    },
    /// A prefix operator over its operand.
    Unary { op: JsUnOp, operand: Box<JsExpr> },
    /// `left op right`.
    Binary {
        op: JsBinOp,
        left: Box<JsExpr>,
        right: Box<JsExpr>,
    },
    /// `test ? consequent : alternate`.
    Conditional {
        test: Box<JsExpr>,
        consequent: Box<JsExpr>,
        alternate: Box<JsExpr>,
    },
    /// `function () { return body; }`, a function of no parameter that
    /// answers `body`.
    Function(Box<JsExpr>),
}

/// The prefix operators the emitter writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum JsUnOp {
    /// `-`
    Neg,
    /// `+`
    Pos,
    /// `!`
    Not,
}

impl JsUnOp {
    fn token(self) -> &'static str {
        match self {
            Self::Neg => "-",
            Self::Pos => "+",
            Self::Not => "!",
        }
    }
}

/// The binary operators of ECMAScript the emitter has a spelling for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum JsBinOp {
    Mul,
    Div,
    Mod,
    Add,
    Sub,
    Shl,
    Shr,
    UShr,
    BitAnd,
    BitXor,
    BitOr,
    StrictEq,
    StrictNe,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
}

impl JsBinOp {
    /// The operator as ECMAScript spells it.
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::Mul => "*",
            Self::Div => "/",
            Self::Mod => "%",
            Self::Add => "+",
            Self::Sub => "-",
            Self::Shl => "<<",
            Self::Shr => ">>",
            Self::UShr => ">>>",
            Self::BitAnd => "&",
            Self::BitXor => "^",
            Self::BitOr => "|",
            Self::StrictEq => "===",
            Self::StrictNe => "!==",
            Self::Lt => "<",
            Self::Gt => ">",
            Self::Le => "<=",
            Self::Ge => ">=",
            Self::And => "&&",
            Self::Or => "||",
        }
    }

    /// The production of ECMA-262's expression grammar the operator is
    /// written in.
    fn precedence(self) -> Precedence {
        match self {
            Self::Mul | Self::Div | Self::Mod => Precedence::Multiplicative,
            Self::Add | Self::Sub => Precedence::Additive,
            Self::Shl | Self::Shr | Self::UShr => Precedence::Shift,
            Self::BitAnd => Precedence::BitAnd,
            Self::BitXor => Precedence::BitXor,
            Self::BitOr => Precedence::BitOr,
            Self::StrictEq | Self::StrictNe => Precedence::Equality,
            Self::Lt | Self::Gt | Self::Le | Self::Ge => Precedence::Relational,
            Self::And => Precedence::And,
            Self::Or => Precedence::Or,
        }
    }
}

/// How tightly an expression binds, loosest first: the productions of
/// ECMA-262's expression grammar the emitter writes, and above them every
/// expression no operator can split (a literal, a name, a function).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Precedence {
    Conditional,
    Or,
    And,
    BitOr,
    BitXor,
    BitAnd,
    Equality,
    Relational,
    Shift,
    Additive,
    Multiplicative,
    Unary,
    LeftHandSide,
    Primary,
}

/// Where a child stands in its parent, which is all the printer needs to
/// decide whether it is grouped.
#[derive(Debug, Clone, Copy)]
enum Slot {
    /// Between delimiters of the parent's own (an argument, an index, the
    /// answer of a function) or a branch of a conditional: where an
    /// AssignmentExpression stands.
    Enclosed,
    /// The whole text, which the engine may read as a statement.
    Statement,
    /// What a call calls or an index reads.
    Postfix,
    /// What a member is read of.
    Member,
    /// The operand of a prefix operator.
    Prefix(JsUnOp),
    /// The condition of a conditional.
    Test,
    /// The left operand of a binary operator.
    Left(JsBinOp),
    /// The right operand of a binary operator.
    Right(JsBinOp),
}

impl JsExpr {
    /// Text that stands as one operand, written as it is.
    pub(crate) fn atom(text: impl Into<String>) -> Self {
        Self::Atom(text.into())
    }

    /// A call of `callee` with `args`.
    pub(crate) fn call(callee: JsExpr, args: Vec<JsExpr>) -> Self {
        Self::Call {
            callee: Box::new(callee),
            args,
        }
    }

    /// A call of the function the dotted path `path` names.
    pub(crate) fn path_call(path: impl Into<String>, args: Vec<JsExpr>) -> Self {
        Self::call(Self::atom(path), args)
    }

    /// The member `name` of this value.
    pub(crate) fn member(self, name: impl Into<String>) -> Self {
        Self::Member {
            base: Box::new(self),
            name: name.into(),
        }
    }

    /// This value indexed at `index`.
    pub(crate) fn indexed(self, index: JsExpr) -> Self {
        Self::Index {
            base: Box::new(self),
            index: Box::new(index),
        }
    }

    /// The prefix operator `op` over this value.
    pub(crate) fn prefixed(self, op: JsUnOp) -> Self {
        Self::Unary {
            op,
            operand: Box::new(self),
        }
    }

    /// `left op right`.
    pub(crate) fn binary(op: JsBinOp, left: JsExpr, right: JsExpr) -> Self {
        Self::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    /// `test ? consequent : alternate`.
    pub(crate) fn conditional(test: JsExpr, consequent: JsExpr, alternate: JsExpr) -> Self {
        Self::Conditional {
            test: Box::new(test),
            consequent: Box::new(consequent),
            alternate: Box::new(alternate),
        }
    }

    /// A function of no parameter that answers `body`.
    pub(crate) fn function(body: JsExpr) -> Self {
        Self::Function(Box::new(body))
    }

    fn precedence(&self) -> Precedence {
        match self {
            Self::Conditional { .. } => Precedence::Conditional,
            Self::Binary { op, .. } => op.precedence(),
            Self::Unary { .. } => Precedence::Unary,
            Self::Call { .. } | Self::Member { .. } | Self::Index { .. } => {
                Precedence::LeftHandSide
            }
            Self::Atom(_) | Self::Function(_) => Precedence::Primary,
        }
    }

    /// Whether this is a decimal integer literal, whose dot a `.` after it
    /// would be.
    fn is_decimal_integer(&self) -> bool {
        matches!(self, Self::Atom(text) if !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()))
    }

    /// Whether this is a function expression, which an expression statement
    /// cannot begin with.
    fn is_function(&self) -> bool {
        matches!(self, Self::Function(_))
    }

    /// Whether this expression is grouped where it stands at `slot`.
    fn needs_parentheses(&self, slot: Slot) -> bool {
        let own = self.precedence();
        match slot {
            Slot::Enclosed => false,
            Slot::Statement => self.is_function(),
            Slot::Postfix => own < Precedence::LeftHandSide || self.is_function(),
            Slot::Member => {
                own < Precedence::LeftHandSide || self.is_function() || self.is_decimal_integer()
            }
            Slot::Prefix(op) => {
                own < Precedence::Unary
                    || matches!(
                        (op, self),
                        (
                            JsUnOp::Neg,
                            Self::Unary {
                                op: JsUnOp::Neg,
                                ..
                            }
                        ) | (
                            JsUnOp::Pos,
                            Self::Unary {
                                op: JsUnOp::Pos,
                                ..
                            }
                        )
                    )
            }
            Slot::Test => own <= Precedence::Conditional || self.is_function(),
            Slot::Left(op) => own < op.precedence() || self.is_function(),
            Slot::Right(op) => own <= op.precedence(),
        }
    }

    fn write(&self, slot: Slot, out: &mut String) {
        if self.needs_parentheses(slot) {
            out.push('(');
            self.write_bare(out);
            out.push(')');
        } else {
            self.write_bare(out);
        }
    }

    fn write_bare(&self, out: &mut String) {
        match self {
            Self::Atom(text) => out.push_str(text),
            Self::Call { callee, args } => {
                callee.write(Slot::Postfix, out);
                out.push('(');
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    arg.write(Slot::Enclosed, out);
                }
                out.push(')');
            }
            Self::Member { base, name } => {
                base.write(Slot::Member, out);
                out.push('.');
                out.push_str(name);
            }
            Self::Index { base, index } => {
                base.write(Slot::Postfix, out);
                out.push('[');
                index.write(Slot::Enclosed, out);
                out.push(']');
            }
            Self::Unary { op, operand } => {
                out.push_str(op.token());
                operand.write(Slot::Prefix(*op), out);
            }
            Self::Binary { op, left, right } => {
                left.write(Slot::Left(*op), out);
                out.push(' ');
                out.push_str(op.token());
                out.push(' ');
                right.write(Slot::Right(*op), out);
            }
            Self::Conditional {
                test,
                consequent,
                alternate,
            } => {
                test.write(Slot::Test, out);
                out.push_str(" ? ");
                consequent.write(Slot::Enclosed, out);
                out.push_str(" : ");
                alternate.write(Slot::Enclosed, out);
            }
            Self::Function(body) => {
                out.push_str("function () { return ");
                body.write(Slot::Enclosed, out);
                out.push_str("; }");
            }
        }
    }
}

/// The whole expression, as the emitter hands it on.
impl fmt::Display for JsExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = String::new();
        self.write(Slot::Statement, &mut out);
        f.write_str(&out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::expr::{parse_to_ast, BinOp, ExprKind, TypedExpr, UnaryOp};

    fn at(text: &str) -> JsExpr {
        JsExpr::atom(text)
    }

    fn op(op: JsBinOp, left: JsExpr, right: JsExpr) -> JsExpr {
        JsExpr::binary(op, left, right)
    }

    fn choice() -> JsExpr {
        JsExpr::conditional(at("c"), at("a"), at("b"))
    }

    /// An operator groups an operand that binds more loosely than itself, and a
    /// right operand that binds as tightly, every binary operator of ECMAScript
    /// the emitter writes associating to the left; a library call is one
    /// operand whatever operator it stands for.
    #[test]
    fn an_operand_is_grouped_by_precedence_and_left_association() {
        let (a, b, c) = (at("a"), at("b"), at("c"));
        let sum = op(JsBinOp::Add, a.clone(), b.clone());
        assert_eq!(
            op(JsBinOp::Mul, sum.clone(), c.clone()).to_string(),
            "(a + b) * c"
        );
        assert_eq!(
            op(
                JsBinOp::Sub,
                a.clone(),
                op(JsBinOp::Sub, b.clone(), c.clone())
            )
            .to_string(),
            "a - (b - c)"
        );
        // Comparisons associate to the left in ECMAScript, and a relational
        // operator binds tighter than an equality.
        assert_eq!(
            op(
                JsBinOp::StrictEq,
                op(JsBinOp::Lt, a.clone(), b.clone()),
                c.clone()
            )
            .to_string(),
            "a < b === c"
        );
        assert_eq!(
            op(
                JsBinOp::StrictEq,
                a.clone(),
                op(JsBinOp::StrictEq, b.clone(), c.clone())
            )
            .to_string(),
            "a === (b === c)"
        );
        let helper = JsExpr::path_call("SceStatic.U8.and", vec![a.clone(), b.clone()]);
        assert_eq!(
            op(JsBinOp::Add, helper, c.clone()).to_string(),
            "SceStatic.U8.and(a, b) + c"
        );
        assert_eq!(
            JsExpr::path_call("Math.trunc", vec![op(JsBinOp::Div, a.clone(), b)]).to_string(),
            "Math.trunc(a / b)"
        );
        assert_eq!(sum.clone().member("length").to_string(), "(a + b).length");
        assert_eq!(sum.indexed(c).to_string(), "(a + b)[c]");
        assert_eq!(a.member("f").prefixed(JsUnOp::Neg).to_string(), "-a.f");
    }

    /// `--` and `++` are one token each, so a sign over the same sign is
    /// grouped; a sign over any other prefix is not.
    #[test]
    fn a_sign_over_the_same_sign_is_grouped() {
        let negated = at("x").prefixed(JsUnOp::Neg);
        assert_eq!(negated.clone().prefixed(JsUnOp::Neg).to_string(), "-(-x)");
        assert_eq!(negated.prefixed(JsUnOp::Pos).to_string(), "+-x");
        let plus = at("x").prefixed(JsUnOp::Pos);
        assert_eq!(plus.clone().prefixed(JsUnOp::Pos).to_string(), "+(+x)");
        assert_eq!(plus.prefixed(JsUnOp::Neg).to_string(), "-+x");
        assert_eq!(
            at("x")
                .prefixed(JsUnOp::Not)
                .prefixed(JsUnOp::Neg)
                .to_string(),
            "-!x"
        );
        assert_eq!(
            op(JsBinOp::Add, at("a"), at("b"))
                .prefixed(JsUnOp::Not)
                .to_string(),
            "!(a + b)"
        );
    }

    /// A conditional is grouped as a condition and as an operand, and bare as
    /// either branch, where ECMAScript reads an AssignmentExpression.
    #[test]
    fn a_conditional_is_grouped_as_a_condition_and_bare_as_a_branch() {
        assert_eq!(
            JsExpr::conditional(choice(), choice(), choice()).to_string(),
            "(c ? a : b) ? c ? a : b : c ? a : b"
        );
        assert_eq!(
            op(JsBinOp::Add, choice(), at("d")).to_string(),
            "(c ? a : b) + d"
        );
        assert_eq!(choice().member("f").to_string(), "(c ? a : b).f");
    }

    /// A function is bare as an argument and grouped wherever it would be the
    /// first token of what holds it, the whole text among them, so no text
    /// begins with `function`.
    #[test]
    fn a_function_never_begins_the_text() {
        let answer = JsExpr::function(op(JsBinOp::Mul, at("h"), at("h")));
        assert_eq!(
            JsExpr::path_call("SceStatic.otherwise", vec![at("x"), answer.clone()]).to_string(),
            "SceStatic.otherwise(x, function () { return h * h; })"
        );
        assert_eq!(answer.to_string(), "(function () { return h * h; })");
        assert_eq!(
            JsExpr::call(answer.clone(), Vec::new()).to_string(),
            "(function () { return h * h; })()"
        );
        assert_eq!(
            op(JsBinOp::Add, answer.clone(), at("a")).to_string(),
            "(function () { return h * h; }) + a"
        );
        assert_eq!(
            op(JsBinOp::Add, at("a"), answer).to_string(),
            "a + function () { return h * h; }"
        );
    }

    /// A decimal integer whose member is read would lend the member's dot to
    /// its fraction, so it is grouped; a real and a name are not.
    #[test]
    fn a_decimal_integer_whose_member_is_read_is_grouped() {
        assert_eq!(at("2").member("length").to_string(), "(2).length");
        assert_eq!(at("1.5").member("f").to_string(), "1.5.f");
        assert_eq!(at("2").indexed(at("0")).to_string(), "2[0]");
    }

    // ── The printer read back by the forge expression parser ────────

    /// The tree of the text the parser read: the printer's text means what the
    /// printed tree does exactly where this reads it back to that tree,
    /// grouping dropped.
    fn read(expr: &TypedExpr) -> JsExpr {
        match &expr.kind {
            ExprKind::NumberLit(n) => JsExpr::atom(n.clone()),
            ExprKind::Ident(name) => JsExpr::atom(name.clone()),
            ExprKind::BoolLit(b) => JsExpr::atom(b.to_string()),
            ExprKind::NullLit => JsExpr::atom("null"),
            ExprKind::Member { object, property } => read(object).member(property.as_str()),
            ExprKind::Index { object, index } => read(object).indexed(read(index)),
            ExprKind::Call { callee, args, .. } => {
                JsExpr::call(read(callee), args.iter().map(read).collect())
            }
            ExprKind::Unary { op, operand } => read(operand).prefixed(match op {
                UnaryOp::Neg => JsUnOp::Neg,
                UnaryOp::Pos => JsUnOp::Pos,
                UnaryOp::Not => JsUnOp::Not,
                UnaryOp::BitNot => panic!("a `~` was read, which the printer never writes"),
            }),
            ExprKind::Binary { op, left, right } => {
                JsExpr::binary(binary(*op), read(left), read(right))
            }
            ExprKind::Conditional {
                condition,
                consequent,
                alternate,
            } => JsExpr::conditional(read(condition), read(consequent), read(alternate)),
            _ => panic!("a node the printer never writes was read"),
        }
    }

    fn binary(op: BinOp) -> JsBinOp {
        match op {
            BinOp::Mul => JsBinOp::Mul,
            BinOp::Div => JsBinOp::Div,
            BinOp::Mod => JsBinOp::Mod,
            BinOp::Add => JsBinOp::Add,
            BinOp::Sub => JsBinOp::Sub,
            BinOp::Shl => JsBinOp::Shl,
            BinOp::Shr => JsBinOp::Shr,
            BinOp::UShr => JsBinOp::UShr,
            BinOp::BitAnd => JsBinOp::BitAnd,
            BinOp::BitXor => JsBinOp::BitXor,
            BinOp::BitOr => JsBinOp::BitOr,
            BinOp::StrictEq => JsBinOp::StrictEq,
            BinOp::StrictNeq => JsBinOp::StrictNe,
            BinOp::Lt => JsBinOp::Lt,
            BinOp::Gt => JsBinOp::Gt,
            BinOp::LtEq => JsBinOp::Le,
            BinOp::GtEq => JsBinOp::Ge,
            BinOp::And => JsBinOp::And,
            BinOp::Or => JsBinOp::Or,
            BinOp::Coalesce => panic!("a `??` was read, which the printer never writes"),
        }
    }

    /// A small generator of draws (xorshift), so the trees are the same on every
    /// run.
    struct Draw(u64);

    impl Draw {
        fn step(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }

        fn below(&mut self, n: usize) -> usize {
            (self.step() % n as u64) as usize
        }

        fn pick<'a>(&mut self, from: &[&'a str]) -> &'a str {
            from[self.below(from.len())]
        }
    }

    const OPERATORS: [JsBinOp; 19] = [
        JsBinOp::Mul,
        JsBinOp::Div,
        JsBinOp::Mod,
        JsBinOp::Add,
        JsBinOp::Sub,
        JsBinOp::Shl,
        JsBinOp::Shr,
        JsBinOp::UShr,
        JsBinOp::BitAnd,
        JsBinOp::BitXor,
        JsBinOp::BitOr,
        JsBinOp::StrictEq,
        JsBinOp::StrictNe,
        JsBinOp::Lt,
        JsBinOp::Gt,
        JsBinOp::Le,
        JsBinOp::Ge,
        JsBinOp::And,
        JsBinOp::Or,
    ];

    const PREFIXES: [JsUnOp; 3] = [JsUnOp::Neg, JsUnOp::Pos, JsUnOp::Not];

    /// The seed every drawing starts from.
    const SEED: u64 = 0x9E37_79B9_7F4A_7C15;

    /// How many trees a drawing holds.
    const TREES: usize = 4000;

    /// A tree of every node kind the parser reads, at most `depth` deep; a
    /// function, which it does not read, is held to the printer by the tests
    /// above.
    fn tree(draw: &mut Draw, depth: u32) -> JsExpr {
        if depth == 0 || draw.below(6) == 0 {
            return at(draw.pick(&["a", "b", "x", "2", "7", "true", "null"]));
        }
        let d = depth - 1;
        match draw.below(12) {
            0 => {
                let callee = if draw.below(4) == 0 {
                    tree(draw, d)
                } else {
                    at("f")
                };
                let count = draw.below(3);
                let args = (0..count).map(|_| tree(draw, d)).collect();
                JsExpr::call(callee, args)
            }
            1 => {
                let base = tree(draw, d);
                base.member(draw.pick(&["f", "length"]))
            }
            2 => {
                let base = tree(draw, d);
                base.indexed(tree(draw, d))
            }
            3 | 4 => {
                let prefix = PREFIXES[draw.below(PREFIXES.len())];
                tree(draw, d).prefixed(prefix)
            }
            5..=9 => {
                let operator = OPERATORS[draw.below(OPERATORS.len())];
                let left = tree(draw, d);
                let right = tree(draw, d);
                JsExpr::binary(operator, left, right)
            }
            _ => {
                let test = tree(draw, d);
                let consequent = tree(draw, d);
                let alternate = tree(draw, d);
                JsExpr::conditional(test, consequent, alternate)
            }
        }
    }

    /// Every tree the printer is handed is read back as itself by the parser of
    /// ECMAScript's expression grammar: every parenthesis the grammar needs is
    /// printed (a `--` the printer let through is a token the parser refuses),
    /// and none the printer leaves out changes what is read.
    #[test]
    fn what_the_printer_writes_reads_back_as_the_tree_it_was_handed() {
        let mut draw = Draw(SEED);
        for _ in 0..TREES {
            let tree = tree(&mut draw, 5);
            let text = tree.to_string();
            let parsed =
                parse_to_ast(&text).unwrap_or_else(|e| panic!("`{text}` is no expression: {e:?}"));
            assert_eq!(read(&parsed), tree, "`{text}` read back");
        }
    }

    /// The kind of a node, as its variant is named.
    fn kind(tree: &JsExpr) -> &'static str {
        match tree {
            JsExpr::Atom(_) => "Atom",
            JsExpr::Call { .. } => "Call",
            JsExpr::Member { .. } => "Member",
            JsExpr::Index { .. } => "Index",
            JsExpr::Unary { .. } => "Unary",
            JsExpr::Binary { .. } => "Binary",
            JsExpr::Conditional { .. } => "Conditional",
            JsExpr::Function(_) => "Function",
        }
    }

    /// The nodes directly inside `tree`.
    fn children(tree: &JsExpr) -> Vec<&JsExpr> {
        match tree {
            JsExpr::Atom(_) => Vec::new(),
            JsExpr::Call { callee, args } => std::iter::once(&**callee).chain(args).collect(),
            JsExpr::Member { base, .. } => vec![&**base],
            JsExpr::Index { base, index } => vec![&**base, &**index],
            JsExpr::Unary { operand, .. } => vec![&**operand],
            JsExpr::Binary { left, right, .. } => vec![&**left, &**right],
            JsExpr::Conditional {
                test,
                consequent,
                alternate,
            } => vec![&**test, &**consequent, &**alternate],
            JsExpr::Function(body) => vec![&**body],
        }
    }

    /// Every kind of node the parser reads is drawn, a prefix in a prefix and a
    /// conditional and a binary operator in every parent among them, and a
    /// decimal integer whose member is read, so the read-back above reaches
    /// each rule of the printer it can.
    #[test]
    fn the_trees_drawn_hold_every_kind_in_every_parent() {
        let mut seen = std::collections::BTreeSet::new();
        let mut draw = Draw(SEED);
        for _ in 0..TREES {
            let mut stack = vec![tree(&mut draw, 5)];
            while let Some(node) = stack.pop() {
                seen.insert(kind(&node).to_string());
                for child in children(&node) {
                    if matches!(
                        child,
                        JsExpr::Conditional { .. } | JsExpr::Unary { .. } | JsExpr::Binary { .. }
                    ) {
                        seen.insert(format!("{} in {}", kind(child), kind(&node)));
                    }
                    if child.is_decimal_integer() {
                        seen.insert(format!("integer in {}", kind(&node)));
                    }
                    stack.push(child.clone());
                }
            }
        }
        for kind in [
            "Atom",
            "Call",
            "Member",
            "Index",
            "Unary",
            "Binary",
            "Conditional",
            "integer in Member",
        ] {
            assert!(seen.contains(kind), "no {kind} was drawn: {seen:?}");
        }
        for child in ["Conditional", "Unary", "Binary"] {
            for parent in ["Call", "Member", "Index", "Unary", "Binary", "Conditional"] {
                assert!(
                    seen.contains(&format!("{child} in {parent}")),
                    "no {child} was drawn in a {parent}: {seen:?}"
                );
            }
        }
    }
}
