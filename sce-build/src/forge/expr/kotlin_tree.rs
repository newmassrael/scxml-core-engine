// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Kotlin a forge expression is written as, as a tree of the Kotlin
//! expressions it is made of, and the one printer that writes such a tree as
//! text.
//!
//! The Kotlin emitter ([`super::emit_kotlin`]) builds this tree and prints it
//! once, where the text leaves it; the Kotlin walks over a composed value
//! ([`crate::forge::composed_lowering`]) build the expressions they read a held
//! value with too. A conversion (`x.toLong()`), a call that takes a lambda
//! (`x.let { v -> … }`), a safe call and a `when` are nodes of their own wherever
//! they come from; nothing is read back out of text.
//!
//! The printer decides every parenthesis, at each parent, from the kind of the
//! child and the precedence and associativity the Kotlin Language
//! Specification gives its operators ("Expressions", the table of operator
//! precedence, and the productions of "Syntax and grammar"), tightest first: a
//! postfix expression (a call, a member read, a safe call, an index), a prefix
//! operator, `*  /  %`, `+  -`, a named infix function (`and  or  xor  shl  shr
//! ushr`), the elvis `?:`, a comparison `<  >  <=  >=`, an equality `==  !=`,
//! `&&`, `||`; each binary level associates from the left. This is the textbook
//! unparsing rule (Ramsey, "Unparsing Expressions with Prefix and Postfix
//! Operators", 1998). The rules of Kotlin's grammar that precedence does not
//! see are the printer's too:
//!
//! * a comparison does not associate: `comparison` takes at most one
//!   `comparisonOperator` between two `genericCallLikeComparison`s, so one
//!   comparison under another is grouped on either side;
//! * `if (c) a else b` is a primary expression whose `else` branch is a
//!   `controlStructureBody`, an expression that reaches as far right as it can:
//!   `if (c) 1 else 2 + 3` adds the 3 to the else branch. It binds more
//!   loosely than any operator, so it is grouped wherever it is not enclosed;
//! * a token is the longest sequence of characters that forms one, so two
//!   prefix operators written together can read as one token: `-` and `-` as
//!   the decrement `--`, `+` and `+` as the increment `++`, `!` and `!` as the
//!   not-null assertion `!!`. A prefix operator over one it would fuse with
//!   groups it: `-(-x)`.
//!
//! Everything else stands bare: `when` closes its arms with a brace and is a
//! primary expression, and an equality is grouped under an equality by
//! precedence alone, on the right.
//!
//! No Kotlin parser is among the workspace's dependencies, so the tests hold
//! the printer to the grammar by reading what it prints back with a parser
//! written from the specification's productions, apart from the printer's
//! table, and comparing the tree read with the tree printed. The Kotlin lanes
//! compile what the emitter prints.

use std::fmt;

/// A Kotlin expression the emitter writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum KotlinExpr {
    /// Text written as it is that stands as one operand: a literal, a name, a
    /// qualified name, or a fragment its writer made one
    /// ([`super::ExprKind::Raw`]).
    Atom(String),
    /// `callee(args)`, followed by the lambda it takes where there is one; a
    /// call with no arguments and a lambda is written `callee { … }`.
    Call {
        callee: Box<KotlinExpr>,
        args: Vec<KotlinExpr>,
        lambda: Option<Box<KotlinLambda>>,
    },
    /// `base.member`, or `base?.member` where `safe` says so.
    Field {
        base: Box<KotlinExpr>,
        member: String,
        safe: bool,
    },
    /// `base[index]`.
    Index {
        base: Box<KotlinExpr>,
        index: Box<KotlinExpr>,
    },
    /// A prefix operator over its operand.
    Unary {
        op: KotlinUnOp,
        operand: Box<KotlinExpr>,
    },
    /// `left op right`.
    Binary {
        op: KotlinBinOp,
        left: Box<KotlinExpr>,
        right: Box<KotlinExpr>,
    },
    /// `if (condition) then else otherwise`.
    If {
        condition: Box<KotlinExpr>,
        then: Box<KotlinExpr>,
        otherwise: Box<KotlinExpr>,
    },
    /// `when (subject) { arms }`, the arms apart by `;`.
    When {
        subject: Box<KotlinExpr>,
        arms: Vec<KotlinWhenArm>,
    },
}

/// The lambda a call takes after its arguments: `{ parameter -> body }`, or
/// `{ body }` where it names none and reads `it`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KotlinLambda {
    pub(crate) parameter: Option<String>,
    pub(crate) body: KotlinExpr,
}

impl KotlinLambda {
    /// The lambda of `body` that names `parameter`, or reads `it` where it names
    /// none.
    pub(crate) fn new(parameter: Option<&str>, body: KotlinExpr) -> Self {
        Self {
            parameter: parameter.map(str::to_string),
            body,
        }
    }
}

/// One arm of a [`KotlinExpr::When`]: `condition -> body`, the condition written
/// as it is (`is T`, a value, `else`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KotlinWhenArm {
    pub(crate) condition: String,
    pub(crate) body: KotlinExpr,
}

/// The prefix operators the emitter writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KotlinUnOp {
    /// `-`
    Neg,
    /// `+`
    Pos,
    /// `!`
    Not,
}

impl KotlinUnOp {
    fn token(self) -> &'static str {
        match self {
            Self::Neg => "-",
            Self::Pos => "+",
            Self::Not => "!",
        }
    }

    /// Whether this operator written right before `inner` reads as one token of
    /// Kotlin's: `--`, `++` or `!!`.
    fn fuses_with(self, inner: KotlinUnOp) -> bool {
        self == inner
    }
}

/// The binary operators the emitter writes, the named infix functions of the
/// standard library among them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KotlinBinOp {
    Mul,
    Div,
    Rem,
    Add,
    Sub,
    /// `and`, the bitwise one.
    BitAnd,
    /// `or`, the bitwise one.
    BitOr,
    /// `xor`
    BitXor,
    /// `shl`
    Shl,
    /// `shr`
    Shr,
    /// `ushr`
    Ushr,
    /// `?:`
    Elvis,
    Lt,
    Gt,
    Le,
    Ge,
    Eq,
    Ne,
    /// `&&`
    And,
    /// `||`
    Or,
}

impl KotlinBinOp {
    fn token(self) -> &'static str {
        match self {
            Self::Mul => "*",
            Self::Div => "/",
            Self::Rem => "%",
            Self::Add => "+",
            Self::Sub => "-",
            Self::BitAnd => "and",
            Self::BitOr => "or",
            Self::BitXor => "xor",
            Self::Shl => "shl",
            Self::Shr => "shr",
            Self::Ushr => "ushr",
            Self::Elvis => "?:",
            Self::Lt => "<",
            Self::Gt => ">",
            Self::Le => "<=",
            Self::Ge => ">=",
            Self::Eq => "==",
            Self::Ne => "!=",
            Self::And => "&&",
            Self::Or => "||",
        }
    }

    /// The row of the specification's table the operator is on.
    fn precedence(self) -> Precedence {
        match self {
            Self::Mul | Self::Div | Self::Rem => Precedence::Multiplicative,
            Self::Add | Self::Sub => Precedence::Additive,
            Self::BitAnd | Self::BitOr | Self::BitXor | Self::Shl | Self::Shr | Self::Ushr => {
                Precedence::Infix
            }
            Self::Elvis => Precedence::Elvis,
            Self::Lt | Self::Gt | Self::Le | Self::Ge => Precedence::Comparison,
            Self::Eq | Self::Ne => Precedence::Equality,
            Self::And => Precedence::Conjunction,
            Self::Or => Precedence::Disjunction,
        }
    }
}

/// How tightly an expression binds, loosest first: an `if`, whose else branch
/// reaches as far right as it can, the rows of the specification's table the
/// emitter writes, and above them every postfix and primary expression, which
/// no operator can split.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Precedence {
    Conditional,
    Disjunction,
    Conjunction,
    Equality,
    Comparison,
    Elvis,
    Infix,
    Additive,
    Multiplicative,
    Prefix,
    Postfix,
}

/// Where a child stands in its parent, which is all the printer needs to
/// decide whether it is grouped.
#[derive(Debug, Clone, Copy)]
enum Slot {
    /// Between delimiters of the parent's own (an argument, an index, a
    /// condition, a branch, an arm, a lambda's body), or the whole expression.
    Enclosed,
    /// What a call calls, or what a member read, a safe call or an index reads.
    Postfix,
    /// The operand of the prefix operator.
    Prefix(KotlinUnOp),
    /// The left operand of a binary operator.
    Left(KotlinBinOp),
    /// The right operand of a binary operator.
    Right(KotlinBinOp),
}

impl KotlinExpr {
    /// Text that stands as one operand, written as it is.
    pub(crate) fn atom(text: impl Into<String>) -> Self {
        Self::Atom(text.into())
    }

    /// A call of `callee` with `args`.
    pub(crate) fn call(callee: KotlinExpr, args: Vec<KotlinExpr>) -> Self {
        Self::Call {
            callee: Box::new(callee),
            args,
            lambda: None,
        }
    }

    /// A call of the function the qualified name `path` names.
    pub(crate) fn path_call(path: impl Into<String>, args: Vec<KotlinExpr>) -> Self {
        Self::call(Self::atom(path), args)
    }

    /// The member `member` of this value.
    pub(crate) fn field(self, member: impl Into<String>) -> Self {
        Self::Field {
            base: Box::new(self),
            member: member.into(),
            safe: false,
        }
    }

    /// The member `member` of this value where it is present: `value?.member`.
    fn safe_field(self, member: impl Into<String>) -> Self {
        Self::Field {
            base: Box::new(self),
            member: member.into(),
            safe: true,
        }
    }

    /// The method `method` of this value called with `args`.
    pub(crate) fn method(self, method: impl Into<String>, args: Vec<KotlinExpr>) -> Self {
        Self::call(self.field(method), args)
    }

    /// The method `method` of this value called with `args` where the value is
    /// present: `value?.method(args)`.
    pub(crate) fn safe_method(self, method: impl Into<String>, args: Vec<KotlinExpr>) -> Self {
        Self::call(self.safe_field(method), args)
    }

    /// The method `method` of this value called with the lambda `lambda` alone:
    /// `value.method { lambda }`.
    pub(crate) fn method_taking(self, method: impl Into<String>, lambda: KotlinLambda) -> Self {
        Self::Call {
            callee: Box::new(self.field(method)),
            args: Vec::new(),
            lambda: Some(Box::new(lambda)),
        }
    }

    /// [`Self::method_taking`] where the value is present:
    /// `value?.method { lambda }`.
    pub(crate) fn safe_method_taking(
        self,
        method: impl Into<String>,
        lambda: KotlinLambda,
    ) -> Self {
        Self::Call {
            callee: Box::new(self.safe_field(method)),
            args: Vec::new(),
            lambda: Some(Box::new(lambda)),
        }
    }

    /// This value indexed at `index`.
    pub(crate) fn indexed(self, index: KotlinExpr) -> Self {
        Self::Index {
            base: Box::new(self),
            index: Box::new(index),
        }
    }

    /// The prefix operator `op` over this value.
    pub(crate) fn prefixed(self, op: KotlinUnOp) -> Self {
        Self::Unary {
            op,
            operand: Box::new(self),
        }
    }

    /// `left op right`.
    pub(crate) fn binary(op: KotlinBinOp, left: KotlinExpr, right: KotlinExpr) -> Self {
        Self::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    /// `if (condition) then else otherwise`.
    pub(crate) fn if_else(condition: KotlinExpr, then: KotlinExpr, otherwise: KotlinExpr) -> Self {
        Self::If {
            condition: Box::new(condition),
            then: Box::new(then),
            otherwise: Box::new(otherwise),
        }
    }

    /// `when (subject) { arms }`.
    pub(crate) fn when_of(subject: KotlinExpr, arms: Vec<KotlinWhenArm>) -> Self {
        Self::When {
            subject: Box::new(subject),
            arms,
        }
    }

    /// This expression printed to stand where a fragment is spliced as it is
    /// (a read a walk writes, [`super::ExprKind::Raw`]): grouped unless it is one
    /// operand that no operator or postfix read around it can take apart.
    pub(crate) fn as_operand(&self) -> String {
        let mut out = String::new();
        self.write(Slot::Postfix, &mut out);
        out
    }

    fn precedence(&self) -> Precedence {
        match self {
            Self::If { .. } => Precedence::Conditional,
            Self::Binary { op, .. } => op.precedence(),
            Self::Unary { .. } => Precedence::Prefix,
            Self::Atom(_)
            | Self::Call { .. }
            | Self::Field { .. }
            | Self::Index { .. }
            | Self::When { .. } => Precedence::Postfix,
        }
    }

    /// Whether this expression is grouped where it stands at `slot`.
    fn needs_parentheses(&self, slot: Slot) -> bool {
        let own = self.precedence();
        match slot {
            Slot::Enclosed => false,
            Slot::Postfix => own < Precedence::Postfix,
            Slot::Prefix(op) => {
                own < Precedence::Prefix
                    || matches!(self, Self::Unary { op: inner, .. } if op.fuses_with(*inner))
            }
            Slot::Left(op) => {
                own < op.precedence()
                    || (own == Precedence::Comparison && op.precedence() == Precedence::Comparison)
            }
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
            Self::Call {
                callee,
                args,
                lambda,
            } => {
                callee.write(Slot::Postfix, out);
                // A lambda after a call's own argument list is that call's
                // trailing lambda (Kotlin grammar, callSuffix), so a lambda
                // handed to what a call returns keeps its empty list: `f()() { … }`,
                // where `f() { … }` would hand it to `f`.
                let absorbs = matches!(&**callee, Self::Call { lambda: None, .. });
                if !args.is_empty() || lambda.is_none() || absorbs {
                    out.push('(');
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 {
                            out.push_str(", ");
                        }
                        arg.write(Slot::Enclosed, out);
                    }
                    out.push(')');
                }
                if let Some(lambda) = lambda {
                    out.push_str(" { ");
                    if let Some(parameter) = &lambda.parameter {
                        out.push_str(parameter);
                        out.push_str(" -> ");
                    }
                    lambda.body.write(Slot::Enclosed, out);
                    out.push_str(" }");
                }
            }
            Self::Field { base, member, safe } => {
                base.write(Slot::Postfix, out);
                out.push_str(if *safe { "?." } else { "." });
                out.push_str(member);
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
            Self::If {
                condition,
                then,
                otherwise,
            } => {
                out.push_str("if (");
                condition.write(Slot::Enclosed, out);
                out.push_str(") ");
                then.write(Slot::Enclosed, out);
                out.push_str(" else ");
                otherwise.write(Slot::Enclosed, out);
            }
            Self::When { subject, arms } => {
                out.push_str("when (");
                subject.write(Slot::Enclosed, out);
                out.push_str(") {");
                for (i, arm) in arms.iter().enumerate() {
                    if i > 0 {
                        out.push(';');
                    }
                    out.push(' ');
                    out.push_str(&arm.condition);
                    out.push_str(" -> ");
                    arm.body.write(Slot::Enclosed, out);
                }
                out.push_str(" }");
            }
        }
    }
}

/// The whole expression, as the emitter hands it on.
impl fmt::Display for KotlinExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = String::new();
        self.write(Slot::Enclosed, &mut out);
        f.write_str(&out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str) -> KotlinExpr {
        KotlinExpr::atom(text)
    }

    fn op(op: KotlinBinOp, left: KotlinExpr, right: KotlinExpr) -> KotlinExpr {
        KotlinExpr::binary(op, left, right)
    }

    fn choice() -> KotlinExpr {
        KotlinExpr::if_else(at("c"), at("1"), at("2"))
    }

    /// An operator groups an operand that binds more loosely than itself, and a
    /// right operand that binds as tightly, since every binary operator of
    /// Kotlin associates to the left; a prefix operator groups a binary operand,
    /// and a postfix read any operand that is no postfix expression.
    #[test]
    fn an_operand_is_grouped_by_precedence_and_left_association() {
        let (a, b, c) = (at("a"), at("b"), at("c"));
        let sum = op(KotlinBinOp::Add, a.clone(), b.clone());
        assert_eq!(
            op(KotlinBinOp::Mul, sum.clone(), c.clone()).to_string(),
            "(a + b) * c"
        );
        assert_eq!(
            op(
                KotlinBinOp::Sub,
                a.clone(),
                op(KotlinBinOp::Sub, b.clone(), c.clone())
            )
            .to_string(),
            "a - (b - c)"
        );
        // A named infix function binds more loosely than `+` and more tightly
        // than the elvis, which binds more tightly than a comparison.
        assert_eq!(
            op(KotlinBinOp::Shl, sum.clone(), c.clone()).to_string(),
            "a + b shl c"
        );
        assert_eq!(
            op(
                KotlinBinOp::Add,
                a.clone(),
                op(KotlinBinOp::BitAnd, b.clone(), c.clone())
            )
            .to_string(),
            "a + (b and c)"
        );
        assert_eq!(
            op(
                KotlinBinOp::Elvis,
                op(KotlinBinOp::BitOr, a.clone(), b.clone()),
                c.clone()
            )
            .to_string(),
            "a or b ?: c"
        );
        assert_eq!(
            op(
                KotlinBinOp::Eq,
                op(KotlinBinOp::Elvis, a.clone(), b.clone()),
                c.clone()
            )
            .to_string(),
            "a ?: b == c"
        );
        assert_eq!(
            op(
                KotlinBinOp::Elvis,
                a.clone(),
                op(KotlinBinOp::Elvis, b.clone(), c.clone())
            )
            .to_string(),
            "a ?: (b ?: c)"
        );
        assert_eq!(
            op(
                KotlinBinOp::Or,
                a.clone(),
                op(KotlinBinOp::And, b.clone(), c.clone())
            )
            .to_string(),
            "a || b && c"
        );
        assert_eq!(
            sum.clone().prefixed(KotlinUnOp::Neg).to_string(),
            "-(a + b)"
        );
        assert_eq!(
            a.clone()
                .field("size")
                .prefixed(KotlinUnOp::Neg)
                .to_string(),
            "-a.size"
        );
        assert_eq!(
            sum.clone().method("toInt", Vec::new()).to_string(),
            "(a + b).toInt()"
        );
        assert_eq!(
            a.clone()
                .prefixed(KotlinUnOp::Neg)
                .method("toShort", Vec::new())
                .to_string(),
            "(-a).toShort()"
        );
        assert_eq!(
            a.clone().method("toLong", Vec::new()).to_string(),
            "a.toLong()"
        );
        assert_eq!(
            sum.clone().safe_method("toInt", Vec::new()).to_string(),
            "(a + b)?.toInt()"
        );
        assert_eq!(a.indexed(sum).to_string(), "a[a + b]");
    }

    /// A comparison does not associate in Kotlin's grammar: one under another
    /// is grouped on either side. An equality associates, and a comparison
    /// binds more tightly than it.
    #[test]
    fn a_comparison_is_grouped_under_a_comparison_and_an_equality_by_precedence() {
        let (a, b, c) = (at("a"), at("b"), at("c"));
        let less = op(KotlinBinOp::Lt, a.clone(), b.clone());
        assert_eq!(
            op(KotlinBinOp::Lt, less.clone(), c.clone()).to_string(),
            "(a < b) < c"
        );
        assert_eq!(
            op(KotlinBinOp::Ge, c.clone(), less.clone()).to_string(),
            "c >= (a < b)"
        );
        assert_eq!(
            op(KotlinBinOp::Eq, less.clone(), less.clone()).to_string(),
            "a < b == a < b"
        );
        let same = op(KotlinBinOp::Eq, a, b);
        assert_eq!(
            op(KotlinBinOp::Eq, same.clone(), c.clone()).to_string(),
            "a == b == c"
        );
        assert_eq!(op(KotlinBinOp::Ne, c, same).to_string(), "c != (a == b)");
    }

    /// An `if`'s else branch reaches as far right as it can, so an `if` is
    /// grouped wherever it is not enclosed, and stands bare where it is.
    #[test]
    fn an_if_is_grouped_wherever_it_is_not_enclosed() {
        assert_eq!(choice().to_string(), "if (c) 1 else 2");
        assert_eq!(
            op(KotlinBinOp::Add, choice(), at("a")).to_string(),
            "(if (c) 1 else 2) + a"
        );
        assert_eq!(
            op(KotlinBinOp::Add, at("a"), choice()).to_string(),
            "a + (if (c) 1 else 2)"
        );
        assert_eq!(
            choice().method("toUByte", Vec::new()).to_string(),
            "(if (c) 1 else 2).toUByte()"
        );
        assert_eq!(
            choice().prefixed(KotlinUnOp::Neg).to_string(),
            "-(if (c) 1 else 2)"
        );
        assert_eq!(
            KotlinExpr::path_call("f", vec![choice()]).to_string(),
            "f(if (c) 1 else 2)"
        );
        assert_eq!(
            KotlinExpr::if_else(at("d"), choice(), choice()).to_string(),
            "if (d) if (c) 1 else 2 else if (c) 1 else 2"
        );
        assert_eq!(choice().as_operand(), "(if (c) 1 else 2)");
    }

    /// Two prefix operators written together that would read as one token of
    /// Kotlin's are kept apart by a pair; any other two stand together.
    #[test]
    fn a_prefix_operator_groups_one_it_would_fuse_with() {
        let twice = |outer: KotlinUnOp, inner: KotlinUnOp| {
            at("x").prefixed(inner).prefixed(outer).to_string()
        };
        assert_eq!(twice(KotlinUnOp::Neg, KotlinUnOp::Neg), "-(-x)");
        assert_eq!(twice(KotlinUnOp::Pos, KotlinUnOp::Pos), "+(+x)");
        assert_eq!(twice(KotlinUnOp::Not, KotlinUnOp::Not), "!(!x)");
        assert_eq!(twice(KotlinUnOp::Neg, KotlinUnOp::Pos), "-+x");
        assert_eq!(twice(KotlinUnOp::Not, KotlinUnOp::Neg), "!-x");
    }

    /// A call that takes a lambda, a safe call and a `when` are postfix or
    /// primary expressions, which stand bare where a postfix reads them.
    #[test]
    fn a_lambda_call_a_safe_call_and_a_when_stand_bare() {
        let round = at("x").method_taking(
            "let",
            KotlinLambda::new(
                Some("v"),
                KotlinExpr::if_else(
                    op(KotlinBinOp::Lt, at("v"), at("0.0")),
                    KotlinExpr::path_call("ceil", vec![op(KotlinBinOp::Sub, at("v"), at("0.5"))]),
                    KotlinExpr::path_call("floor", vec![op(KotlinBinOp::Add, at("v"), at("0.5"))]),
                ),
            ),
        );
        assert_eq!(
            round.method("toInt", Vec::new()).to_string(),
            "x.let { v -> if (v < 0.0) ceil(v - 0.5) else floor(v + 0.5) }.toInt()"
        );
        assert_eq!(
            at("x")
                .safe_method_taking("let", KotlinLambda::new(None, at("it")))
                .to_string(),
            "x?.let { it }"
        );
        let read = KotlinExpr::when_of(
            at("h"),
            vec![
                KotlinWhenArm {
                    condition: "is A".to_string(),
                    body: at("1"),
                },
                KotlinWhenArm {
                    condition: "else".to_string(),
                    body: at("2"),
                },
            ],
        );
        assert_eq!(read.to_string(), "when (h) { is A -> 1; else -> 2 }");
        assert_eq!(read.as_operand(), read.to_string());
        assert_eq!(
            op(KotlinBinOp::Add, read, at("a")).to_string(),
            "when (h) { is A -> 1; else -> 2 } + a"
        );
        assert_eq!(KotlinExpr::call(at("f"), Vec::new()).to_string(), "f()");
    }

    // ── The printer read back by a parser of Kotlin's productions ─────

    /// A token of the text read: a name, a number, a string literal, or an
    /// operator or a delimiter, matched longest first as Kotlin's lexer does.
    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Token {
        Name(String),
        Number(String),
        Text(String),
        Mark(&'static str),
    }

    /// Kotlin's operators and punctuation, every longer one before the shorter
    /// ones it begins with.
    const MARKS: [&str; 43] = [
        "===", "!==", "..<", "?.", "?:", "->", "&&", "||", "==", "!=", "<=", ">=", "--", "++",
        "!!", "..", "+=", "-=", "*=", "/=", "%=", "::", "+", "-", "*", "/", "%", "<", ">", "=",
        "!", "(", ")", "[", "]", "{", "}", ",", ".", ";", "?", ":", "@",
    ];

    fn tokens(text: &str) -> Vec<Token> {
        let mut out = Vec::new();
        let mut rest = text;
        while let Some(first) = rest.chars().next() {
            if first == ' ' {
                rest = &rest[1..];
            } else if first.is_ascii_alphabetic() || first == '_' {
                let end = rest
                    .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                    .unwrap_or(rest.len());
                out.push(Token::Name(rest[..end].to_string()));
                rest = &rest[end..];
            } else if first.is_ascii_digit() {
                // A real literal needs a digit after its point, so `7.f` is the
                // integer 7 and a member read.
                let mut end = rest
                    .find(|c: char| !c.is_ascii_digit())
                    .unwrap_or(rest.len());
                if rest[end..].starts_with('.')
                    && rest[end + 1..].starts_with(|c: char| c.is_ascii_digit())
                {
                    end += 1 + rest[end + 1..]
                        .find(|c: char| !c.is_ascii_digit())
                        .unwrap_or(rest.len() - end - 1);
                }
                out.push(Token::Number(rest[..end].to_string()));
                rest = &rest[end..];
            } else if first == '"' {
                let end = 1 + rest[1..].find('"').expect("a closed string literal");
                out.push(Token::Text(rest[..=end].to_string()));
                rest = &rest[end + 1..];
            } else {
                let mark = MARKS
                    .iter()
                    .find(|mark| rest.starts_with(**mark))
                    .unwrap_or_else(|| panic!("no token of Kotlin's begins {rest:?}"));
                out.push(Token::Mark(mark));
                rest = &rest[mark.len()..];
            }
        }
        out
    }

    /// The named infix functions the printer writes.
    const INFIX: [(&str, KotlinBinOp); 6] = [
        ("and", KotlinBinOp::BitAnd),
        ("or", KotlinBinOp::BitOr),
        ("xor", KotlinBinOp::BitXor),
        ("shl", KotlinBinOp::Shl),
        ("shr", KotlinBinOp::Shr),
        ("ushr", KotlinBinOp::Ushr),
    ];

    /// A reader of the expressions the printer writes, by the productions of
    /// Kotlin's grammar: `disjunction` of `conjunction`s of `equality`s, an
    /// `equality` of `comparison`s, a `comparison` of at most two
    /// `elvisExpression`s, an `elvisExpression` of `infixFunctionCall`s, those of
    /// `additiveExpression`s of `multiplicativeExpression`s of
    /// `prefixUnaryExpression`s, each a `postfixUnaryExpression`: a primary
    /// expression (a parenthesised one, an `if`, a `when`, a name or a literal)
    /// followed by member reads, safe calls, indices, arguments and lambdas.
    /// Each pair of parentheses is dropped where it is read.
    struct Reader {
        tokens: Vec<Token>,
        at: usize,
    }

    impl Reader {
        fn whole(text: &str) -> KotlinExpr {
            let mut reader = Reader {
                tokens: tokens(text),
                at: 0,
            };
            let read = reader.expression();
            assert_eq!(
                reader.at,
                reader.tokens.len(),
                "{text:?} goes on after one expression"
            );
            read
        }

        fn peek(&self) -> Option<&Token> {
            self.tokens.get(self.at)
        }

        fn take(&mut self) -> Token {
            let token = self
                .tokens
                .get(self.at)
                .cloned()
                .unwrap_or_else(|| panic!("the text ends early: {:?}", self.tokens));
            self.at += 1;
            token
        }

        fn at_mark(&self, mark: &str) -> bool {
            matches!(self.peek(), Some(Token::Mark(m)) if *m == mark)
        }

        fn at_name(&self, name: &str) -> bool {
            matches!(self.peek(), Some(Token::Name(n)) if n == name)
        }

        fn expect(&mut self, mark: &str) {
            assert!(
                self.at_mark(mark),
                "{mark:?} expected at {}: {:?}",
                self.at,
                self.tokens
            );
            self.at += 1;
        }

        fn expect_name(&mut self, name: &str) {
            assert!(
                self.at_name(name),
                "{name:?} expected at {}: {:?}",
                self.at,
                self.tokens
            );
            self.at += 1;
        }

        /// The operator among `marks` the next token is, taken.
        fn operator(&mut self, marks: &[(&str, KotlinBinOp)]) -> Option<KotlinBinOp> {
            let Some(Token::Mark(mark)) = self.peek() else {
                return None;
            };
            let &(_, op) = marks.iter().find(|(token, _)| token == mark)?;
            self.at += 1;
            Some(op)
        }

        fn expression(&mut self) -> KotlinExpr {
            let mut left = self.conjunction();
            while let Some(op) = self.operator(&[("||", KotlinBinOp::Or)]) {
                left = KotlinExpr::binary(op, left, self.conjunction());
            }
            left
        }

        fn conjunction(&mut self) -> KotlinExpr {
            let mut left = self.equality();
            while let Some(op) = self.operator(&[("&&", KotlinBinOp::And)]) {
                left = KotlinExpr::binary(op, left, self.equality());
            }
            left
        }

        fn equality(&mut self) -> KotlinExpr {
            let mut left = self.comparison();
            while let Some(op) = self.operator(&[("==", KotlinBinOp::Eq), ("!=", KotlinBinOp::Ne)])
            {
                left = KotlinExpr::binary(op, left, self.comparison());
            }
            left
        }

        fn comparison(&mut self) -> KotlinExpr {
            const COMPARISONS: [(&str, KotlinBinOp); 4] = [
                ("<", KotlinBinOp::Lt),
                (">", KotlinBinOp::Gt),
                ("<=", KotlinBinOp::Le),
                (">=", KotlinBinOp::Ge),
            ];
            let left = self.elvis();
            let Some(op) = self.operator(&COMPARISONS) else {
                return left;
            };
            let read = KotlinExpr::binary(op, left, self.elvis());
            assert!(
                self.operator(&COMPARISONS).is_none(),
                "a comparison follows a comparison: {:?}",
                self.tokens
            );
            read
        }

        fn elvis(&mut self) -> KotlinExpr {
            let mut left = self.infix();
            while let Some(op) = self.operator(&[("?:", KotlinBinOp::Elvis)]) {
                left = KotlinExpr::binary(op, left, self.infix());
            }
            left
        }

        fn infix(&mut self) -> KotlinExpr {
            let mut left = self.additive();
            loop {
                let Some(Token::Name(name)) = self.peek() else {
                    return left;
                };
                let Some(&(_, op)) = INFIX.iter().find(|(token, _)| token == name) else {
                    return left;
                };
                self.at += 1;
                left = KotlinExpr::binary(op, left, self.additive());
            }
        }

        fn additive(&mut self) -> KotlinExpr {
            let mut left = self.multiplicative();
            while let Some(op) = self.operator(&[("+", KotlinBinOp::Add), ("-", KotlinBinOp::Sub)])
            {
                left = KotlinExpr::binary(op, left, self.multiplicative());
            }
            left
        }

        fn multiplicative(&mut self) -> KotlinExpr {
            let mut left = self.prefix();
            while let Some(op) = self.operator(&[
                ("*", KotlinBinOp::Mul),
                ("/", KotlinBinOp::Div),
                ("%", KotlinBinOp::Rem),
            ]) {
                left = KotlinExpr::binary(op, left, self.prefix());
            }
            left
        }

        fn prefix(&mut self) -> KotlinExpr {
            for (mark, op) in [
                ("-", KotlinUnOp::Neg),
                ("+", KotlinUnOp::Pos),
                ("!", KotlinUnOp::Not),
            ] {
                if self.at_mark(mark) {
                    self.at += 1;
                    return self.prefix().prefixed(op);
                }
            }
            self.postfix()
        }

        fn postfix(&mut self) -> KotlinExpr {
            let mut read = self.primary();
            loop {
                if self.at_mark(".") || self.at_mark("?.") {
                    let safe = self.at_mark("?.");
                    self.at += 1;
                    let Token::Name(member) = self.take() else {
                        panic!("a member read names no member: {:?}", self.tokens);
                    };
                    read = if safe {
                        read.safe_field(member)
                    } else {
                        read.field(member)
                    };
                } else if self.at_mark("(") {
                    self.at += 1;
                    let args = self.arguments();
                    let lambda = self.at_mark("{").then(|| Box::new(self.lambda()));
                    read = KotlinExpr::Call {
                        callee: Box::new(read),
                        args,
                        lambda,
                    };
                } else if self.at_mark("{") {
                    read = KotlinExpr::Call {
                        callee: Box::new(read),
                        args: Vec::new(),
                        lambda: Some(Box::new(self.lambda())),
                    };
                } else if self.at_mark("[") {
                    self.at += 1;
                    let index = self.expression();
                    self.expect("]");
                    read = read.indexed(index);
                } else {
                    return read;
                }
            }
        }

        fn arguments(&mut self) -> Vec<KotlinExpr> {
            let mut args = Vec::new();
            if self.at_mark(")") {
                self.at += 1;
                return args;
            }
            loop {
                args.push(self.expression());
                if self.at_mark(",") {
                    self.at += 1;
                } else {
                    self.expect(")");
                    return args;
                }
            }
        }

        fn lambda(&mut self) -> KotlinLambda {
            self.expect("{");
            let parameter = match (self.peek(), self.tokens.get(self.at + 1)) {
                (Some(Token::Name(name)), Some(Token::Mark("->"))) => Some(name.clone()),
                _ => None,
            };
            if parameter.is_some() {
                self.at += 2;
            }
            let body = self.expression();
            self.expect("}");
            KotlinLambda { parameter, body }
        }

        fn primary(&mut self) -> KotlinExpr {
            match self.take() {
                Token::Mark("(") => {
                    let inner = self.expression();
                    self.expect(")");
                    inner
                }
                Token::Name(name) if name == "if" => {
                    self.expect("(");
                    let condition = self.expression();
                    self.expect(")");
                    let then = self.expression();
                    self.expect_name("else");
                    let otherwise = self.expression();
                    KotlinExpr::if_else(condition, then, otherwise)
                }
                Token::Name(name) if name == "when" => {
                    self.expect("(");
                    let subject = self.expression();
                    self.expect(")");
                    self.expect("{");
                    let mut arms = Vec::new();
                    loop {
                        let mut condition = Vec::new();
                        while !self.at_mark("->") {
                            condition.push(match self.take() {
                                Token::Name(text) | Token::Number(text) | Token::Text(text) => text,
                                Token::Mark(mark) => mark.to_string(),
                            });
                        }
                        self.at += 1;
                        let body = self.expression();
                        arms.push(KotlinWhenArm {
                            condition: condition.join(" "),
                            body,
                        });
                        if self.at_mark(";") {
                            self.at += 1;
                        } else {
                            self.expect("}");
                            return KotlinExpr::when_of(subject, arms);
                        }
                    }
                }
                Token::Name(text) | Token::Number(text) | Token::Text(text) => {
                    KotlinExpr::Atom(text)
                }
                token => panic!("{token:?} begins no expression: {:?}", self.tokens),
            }
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

    const OPERATORS: [KotlinBinOp; 20] = [
        KotlinBinOp::Mul,
        KotlinBinOp::Div,
        KotlinBinOp::Rem,
        KotlinBinOp::Add,
        KotlinBinOp::Sub,
        KotlinBinOp::BitAnd,
        KotlinBinOp::BitOr,
        KotlinBinOp::BitXor,
        KotlinBinOp::Shl,
        KotlinBinOp::Shr,
        KotlinBinOp::Ushr,
        KotlinBinOp::Elvis,
        KotlinBinOp::Lt,
        KotlinBinOp::Gt,
        KotlinBinOp::Le,
        KotlinBinOp::Ge,
        KotlinBinOp::Eq,
        KotlinBinOp::Ne,
        KotlinBinOp::And,
        KotlinBinOp::Or,
    ];

    const PREFIXES: [KotlinUnOp; 3] = [KotlinUnOp::Neg, KotlinUnOp::Pos, KotlinUnOp::Not];

    /// The seed every drawing starts from.
    const SEED: u64 = 0x9E37_79B9_7F4A_7C15;

    /// How many trees a drawing holds.
    const TREES: usize = 4000;

    /// A tree of every node kind, at most `depth` deep.
    fn tree(draw: &mut Draw, depth: u32) -> KotlinExpr {
        if depth == 0 || draw.below(6) == 0 {
            return at(draw.pick(&["a", "b", "x", "null", "\"s\"", "7"]));
        }
        let d = depth - 1;
        match draw.below(14) {
            0 => {
                let callee = if draw.below(4) == 0 {
                    tree(draw, d)
                } else {
                    at("f")
                };
                let count = draw.below(3);
                let args = (0..count).map(|_| tree(draw, d)).collect();
                let lambda = (draw.below(3) == 0).then(|| Box::new(lambda(draw, d)));
                KotlinExpr::Call {
                    callee: Box::new(callee),
                    args,
                    lambda,
                }
            }
            1 => {
                let receiver = tree(draw, d);
                let method = draw.pick(&["toInt", "m"]);
                let count = draw.below(3);
                let args = (0..count).map(|_| tree(draw, d)).collect();
                if draw.below(3) == 0 {
                    receiver.safe_method(method, args)
                } else {
                    receiver.method(method, args)
                }
            }
            2 => {
                let receiver = tree(draw, d);
                let method = draw.pick(&["let", "map"]);
                let lambda = lambda(draw, d);
                if draw.below(2) == 0 {
                    receiver.safe_method_taking(method, lambda)
                } else {
                    receiver.method_taking(method, lambda)
                }
            }
            3 => tree(draw, d).field(draw.pick(&["size", "value"])),
            4 => {
                let base = tree(draw, d);
                base.indexed(tree(draw, d))
            }
            5 | 6 => {
                let prefix = PREFIXES[draw.below(PREFIXES.len())];
                tree(draw, d).prefixed(prefix)
            }
            7..=10 => {
                let operator = OPERATORS[draw.below(OPERATORS.len())];
                let left = tree(draw, d);
                let right = tree(draw, d);
                KotlinExpr::binary(operator, left, right)
            }
            11 | 12 => {
                let condition = tree(draw, d);
                let then = tree(draw, d);
                let otherwise = tree(draw, d);
                KotlinExpr::if_else(condition, then, otherwise)
            }
            _ => {
                let subject = tree(draw, d);
                let count = 1 + draw.below(3);
                let arms = (0..count)
                    .map(|_| KotlinWhenArm {
                        condition: draw.pick(&["is A", "else", "\"n\""]).to_string(),
                        body: tree(draw, d),
                    })
                    .collect();
                KotlinExpr::when_of(subject, arms)
            }
        }
    }

    fn lambda(draw: &mut Draw, depth: u32) -> KotlinLambda {
        let parameter = draw.pick(&["", "v"]);
        KotlinLambda::new(
            (!parameter.is_empty()).then_some(parameter),
            tree(draw, depth),
        )
    }

    /// Every tree the printer is handed is read back as itself: as a whole
    /// expression, and spliced as one operand where a postfix read and a prefix
    /// operator could take it apart. Every parenthesis the grammar needs is
    /// printed, and none the printer leaves out changes what is read.
    #[test]
    fn what_the_printer_writes_reads_back_as_the_tree_it_was_handed() {
        let mut draw = Draw(SEED);
        for _ in 0..TREES {
            let tree = tree(&mut draw, 5);
            let text = tree.to_string();
            assert_eq!(Reader::whole(&text), tree, "{text:?} read as an expression");
            let operand = tree.as_operand();
            let spliced = format!("{operand}.m() * -{operand}");
            assert_eq!(
                Reader::whole(&spliced),
                op(
                    KotlinBinOp::Mul,
                    tree.clone().method("m", Vec::new()),
                    tree.prefixed(KotlinUnOp::Neg)
                ),
                "{operand:?} stands as one operand"
            );
        }
    }

    /// The kind of a node, as its variant is named.
    fn kind(tree: &KotlinExpr) -> &'static str {
        match tree {
            KotlinExpr::Atom(_) => "Atom",
            KotlinExpr::Call { lambda: None, .. } => "Call",
            KotlinExpr::Call { .. } => "Call with a lambda",
            KotlinExpr::Field { safe: false, .. } => "Field",
            KotlinExpr::Field { .. } => "Safe field",
            KotlinExpr::Index { .. } => "Index",
            KotlinExpr::Unary { .. } => "Unary",
            KotlinExpr::Binary { .. } => "Binary",
            KotlinExpr::If { .. } => "If",
            KotlinExpr::When { .. } => "When",
        }
    }

    /// The nodes directly inside `tree`.
    fn children(tree: &KotlinExpr) -> Vec<&KotlinExpr> {
        match tree {
            KotlinExpr::Atom(_) => Vec::new(),
            KotlinExpr::Call {
                callee,
                args,
                lambda,
            } => std::iter::once(&**callee)
                .chain(args)
                .chain(lambda.iter().map(|lambda| &lambda.body))
                .collect(),
            KotlinExpr::Field { base, .. } => vec![&**base],
            KotlinExpr::Index { base, index } => vec![&**base, &**index],
            KotlinExpr::Unary { operand, .. } => vec![&**operand],
            KotlinExpr::Binary { left, right, .. } => vec![&**left, &**right],
            KotlinExpr::If {
                condition,
                then,
                otherwise,
            } => vec![&**condition, &**then, &**otherwise],
            KotlinExpr::When { subject, arms } => std::iter::once(&**subject)
                .chain(arms.iter().map(|arm| &arm.body))
                .collect(),
        }
    }

    /// Whether `node` is a comparison.
    fn is_comparison(node: &KotlinExpr) -> bool {
        matches!(node, KotlinExpr::Binary { op, .. } if op.precedence() == Precedence::Comparison)
    }

    /// Every kind of node is drawn, and an `if`, a prefix operator and a binary
    /// one in every parent, two prefix operators that fuse and a comparison
    /// under a comparison among them, so the read-back above reaches each rule
    /// of the printer.
    #[test]
    fn the_trees_drawn_hold_every_kind_and_an_operator_in_every_parent() {
        let mut seen = std::collections::BTreeSet::new();
        let mut draw = Draw(SEED);
        for _ in 0..TREES {
            let mut stack = vec![tree(&mut draw, 5)];
            while let Some(node) = stack.pop() {
                seen.insert(kind(&node).to_string());
                match &node {
                    KotlinExpr::Unary { op, operand } => {
                        if let KotlinExpr::Unary { op: inner, .. } = &**operand {
                            if op.fuses_with(*inner) {
                                seen.insert("a fused pair".to_string());
                            }
                        }
                    }
                    KotlinExpr::Binary { op, left, right }
                        if op.precedence() == Precedence::Comparison
                            && (is_comparison(left) || is_comparison(right)) =>
                    {
                        seen.insert("a comparison in a comparison".to_string());
                    }
                    _ => {}
                }
                for child in children(&node) {
                    if matches!(
                        child,
                        KotlinExpr::If { .. }
                            | KotlinExpr::Unary { .. }
                            | KotlinExpr::Binary { .. }
                    ) {
                        seen.insert(format!("{} in {}", kind(child), kind(&node)));
                    }
                    stack.push(child.clone());
                }
            }
        }
        for kind in [
            "Atom",
            "Call",
            "Call with a lambda",
            "Field",
            "Safe field",
            "Index",
            "Unary",
            "Binary",
            "If",
            "When",
            "a fused pair",
            "a comparison in a comparison",
        ] {
            assert!(seen.contains(kind), "no {kind} was drawn: {seen:?}");
        }
        for parent in [
            "Call",
            "Call with a lambda",
            "Field",
            "Safe field",
            "Index",
            "Unary",
            "Binary",
            "If",
            "When",
        ] {
            for child in ["If", "Unary", "Binary"] {
                assert!(
                    seen.contains(&format!("{child} in {parent}")),
                    "no {child} was drawn in a {parent}: {seen:?}"
                );
            }
        }
    }
}
