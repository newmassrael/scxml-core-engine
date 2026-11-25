// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Swift a forge expression is written as, as a tree of the Swift
//! expressions it is made of, and the one printer that writes such a tree as
//! text.
//!
//! The Swift emitter ([`super::swift`]) builds this tree; nothing is read back
//! out of text. A call that can throw is the `try` over it, a node of its own,
//! and a closure the emitter hands the runtime is one too.
//!
//! The printer decides every parenthesis, at each parent, from the kind of the
//! child and the precedence groups the Swift standard library declares for its
//! operators (The Swift Programming Language, "Operator Declarations", and the
//! standard library's `BitwiseShiftPrecedence` through `TernaryPrecedence`):
//! the textbook unparsing rule (Ramsey, "Unparsing Expressions with Prefix and
//! Postfix Operators", 1998). The groups' associativity is the printer's: a
//! multiplicative, an additive and a logical operator associate to the left,
//! `??` and the conditional to the right, and a comparison and a shift not at
//! all, so one under another of its group is grouped on either side. The rules
//! of Swift's grammar that the groups do not see are the printer's too:
//!
//! * an operator is a run of operator characters read whole ("Operators", in
//!   "Lexical Structure"), so a prefix operator written before another,
//!   `-!x`, is one operator Swift does not declare: a prefix operator over a
//!   prefix operator is grouped;
//! * `try` covers everything to its right in the expression it begins, and
//!   Swift refuses one to the right of any operator but an assignment ("Try
//!   Operator", in "Expressions"): it is grouped wherever it is an operand of
//!   an operator or of a postfix, and bare as an argument, a subscript, the
//!   body of a closure and the middle of a conditional, each an expression of
//!   its own;
//! * the text the emitter hands on is spliced by a writer this layer does not
//!   see, possibly to the right of an operator, so a `try` that is the whole
//!   text is grouped: the one spelling of a throwing call where it leaves
//!   (docs/adr/0099, section 3.2).
//!
//! Everything else stands bare. No parser of Swift's operator grammar is in
//! the build (tree-sitter-swift folds no precedence groups), so the tests hold
//! the printer to the groups by a reader written from their declarations,
//! which reads what the printer writes back and compares the tree read with
//! the tree printed; the Swift lanes compile what it prints.

use std::fmt;

/// A Swift expression the emitter writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SwiftExpr {
    /// Text written as it is that stands as one operand: a literal, a name, a
    /// dotted path, or a fragment its writer made one
    /// ([`super::ExprKind::Raw`]).
    Atom(String),
    /// `callee(args)`.
    Call {
        callee: Box<SwiftExpr>,
        args: Vec<SwiftArg>,
    },
    /// `base.name`.
    Member { base: Box<SwiftExpr>, name: String },
    /// `base[index]`.
    Index {
        base: Box<SwiftExpr>,
        index: Box<SwiftExpr>,
    },
    /// A prefix operator over its operand.
    Unary {
        op: SwiftUnOp,
        operand: Box<SwiftExpr>,
    },
    /// `left op right`.
    Binary {
        op: SwiftBinOp,
        left: Box<SwiftExpr>,
        right: Box<SwiftExpr>,
    },
    /// `test ? then : otherwise`.
    Conditional {
        test: Box<SwiftExpr>,
        then: Box<SwiftExpr>,
        otherwise: Box<SwiftExpr>,
    },
    /// `try value`.
    Try(Box<SwiftExpr>),
    /// `{ signature in body }`, a closure of one expression.
    Closure {
        signature: String,
        body: Box<SwiftExpr>,
    },
}

/// One argument of a [`SwiftExpr::Call`], with the label it is passed by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SwiftArg {
    pub(crate) label: Option<String>,
    pub(crate) value: SwiftExpr,
}

impl SwiftArg {
    /// An argument passed by position.
    pub(crate) fn bare(value: SwiftExpr) -> Self {
        Self { label: None, value }
    }

    /// An argument passed by the label `label`.
    pub(crate) fn labeled(label: &str, value: SwiftExpr) -> Self {
        Self {
            label: Some(label.to_string()),
            value,
        }
    }
}

/// The prefix operators the emitter writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SwiftUnOp {
    /// `-`
    Neg,
    /// `+`
    Pos,
    /// `!`
    Not,
    /// `~`
    BitNot,
}

impl SwiftUnOp {
    fn token(self) -> &'static str {
        match self {
            Self::Neg => "-",
            Self::Pos => "+",
            Self::Not => "!",
            Self::BitNot => "~",
        }
    }
}

/// The infix operators the emitter writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SwiftBinOp {
    Shl,
    Shr,
    Mul,
    Div,
    Rem,
    BitAnd,
    Add,
    Sub,
    BitOr,
    BitXor,
    Coalesce,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
}

/// How an operator of a precedence group takes an operand of its own group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Associativity {
    Left,
    Right,
    None,
}

impl SwiftBinOp {
    fn token(self) -> &'static str {
        match self {
            Self::Shl => "<<",
            Self::Shr => ">>",
            Self::Mul => "*",
            Self::Div => "/",
            Self::Rem => "%",
            Self::BitAnd => "&",
            Self::Add => "+",
            Self::Sub => "-",
            Self::BitOr => "|",
            Self::BitXor => "^",
            Self::Coalesce => "??",
            Self::Eq => "==",
            Self::Ne => "!=",
            Self::Lt => "<",
            Self::Gt => ">",
            Self::Le => "<=",
            Self::Ge => ">=",
            Self::And => "&&",
            Self::Or => "||",
        }
    }

    /// The standard library's precedence group of the operator.
    fn precedence(self) -> Precedence {
        match self {
            Self::Shl | Self::Shr => Precedence::BitwiseShift,
            Self::Mul | Self::Div | Self::Rem | Self::BitAnd => Precedence::Multiplication,
            Self::Add | Self::Sub | Self::BitOr | Self::BitXor => Precedence::Addition,
            Self::Coalesce => Precedence::NilCoalescing,
            Self::Eq | Self::Ne | Self::Lt | Self::Gt | Self::Le | Self::Ge => {
                Precedence::Comparison
            }
            Self::And => Precedence::LogicalConjunction,
            Self::Or => Precedence::LogicalDisjunction,
        }
    }
}

/// How tightly an expression binds, loosest first: a `try`, the standard
/// library's precedence groups of the operators the emitter writes, and above
/// them a prefix operator, a postfix expression and every expression no
/// operator can split (a literal, a name, a closure).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Precedence {
    Try,
    Ternary,
    LogicalDisjunction,
    LogicalConjunction,
    Comparison,
    NilCoalescing,
    Addition,
    Multiplication,
    BitwiseShift,
    Prefix,
    Postfix,
    Primary,
}

impl Precedence {
    /// The associativity the standard library declares for the group.
    fn associativity(self) -> Associativity {
        match self {
            Self::Ternary | Self::NilCoalescing => Associativity::Right,
            Self::LogicalDisjunction
            | Self::LogicalConjunction
            | Self::Addition
            | Self::Multiplication => Associativity::Left,
            _ => Associativity::None,
        }
    }
}

/// Where a child stands in its parent, which is all the printer needs to
/// decide whether it is grouped.
#[derive(Debug, Clone, Copy)]
enum Slot {
    /// Between delimiters of the parent's own (an argument, a subscript, the
    /// body of a closure, the middle of a conditional), or what a `try`
    /// covers: an expression of its own.
    Enclosed,
    /// The whole text, spliced where a writer this layer does not see puts it.
    Spliced,
    /// What a call calls, a member is read of or a subscript reads.
    Postfix,
    /// The operand of a prefix operator.
    Prefix,
    /// The condition of a conditional.
    Test,
    /// The left operand of an infix operator of the group.
    Left(Precedence),
    /// The right operand of an infix operator of the group, a conditional's
    /// else branch among them.
    Right(Precedence),
}

impl SwiftExpr {
    /// Text that stands as one operand, written as it is.
    pub(crate) fn atom(text: impl Into<String>) -> Self {
        Self::Atom(text.into())
    }

    /// A call of `callee` with `args`.
    pub(crate) fn call(callee: SwiftExpr, args: Vec<SwiftArg>) -> Self {
        Self::Call {
            callee: Box::new(callee),
            args,
        }
    }

    /// A call of the function the dotted path `path` names, each argument
    /// passed by position.
    pub(crate) fn path_call(path: impl Into<String>, args: Vec<SwiftExpr>) -> Self {
        Self::call(
            Self::atom(path),
            args.into_iter().map(SwiftArg::bare).collect(),
        )
    }

    /// The member `name` of this value.
    pub(crate) fn member(self, name: impl Into<String>) -> Self {
        Self::Member {
            base: Box::new(self),
            name: name.into(),
        }
    }

    /// The method `name` of this value called with `args`, each passed by
    /// position.
    pub(crate) fn method(self, name: impl Into<String>, args: Vec<SwiftExpr>) -> Self {
        Self::call(
            self.member(name),
            args.into_iter().map(SwiftArg::bare).collect(),
        )
    }

    /// This value read at the subscript `index`.
    pub(crate) fn indexed(self, index: SwiftExpr) -> Self {
        Self::Index {
            base: Box::new(self),
            index: Box::new(index),
        }
    }

    /// The prefix operator `op` over this value.
    pub(crate) fn prefixed(self, op: SwiftUnOp) -> Self {
        Self::Unary {
            op,
            operand: Box::new(self),
        }
    }

    /// `left op right`.
    pub(crate) fn binary(op: SwiftBinOp, left: SwiftExpr, right: SwiftExpr) -> Self {
        Self::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    /// `test ? then : otherwise`.
    pub(crate) fn conditional(test: SwiftExpr, then: SwiftExpr, otherwise: SwiftExpr) -> Self {
        Self::Conditional {
            test: Box::new(test),
            then: Box::new(then),
            otherwise: Box::new(otherwise),
        }
    }

    /// This value, which can throw, marked `try`.
    pub(crate) fn tried(self) -> Self {
        Self::Try(Box::new(self))
    }

    /// The closure `{ signature in body }`.
    pub(crate) fn closure(signature: impl Into<String>, body: SwiftExpr) -> Self {
        Self::Closure {
            signature: signature.into(),
            body: Box::new(body),
        }
    }

    fn precedence(&self) -> Precedence {
        match self {
            Self::Try(_) => Precedence::Try,
            Self::Conditional { .. } => Precedence::Ternary,
            Self::Binary { op, .. } => op.precedence(),
            Self::Unary { .. } => Precedence::Prefix,
            Self::Call { .. } | Self::Member { .. } | Self::Index { .. } => Precedence::Postfix,
            Self::Atom(_) | Self::Closure { .. } => Precedence::Primary,
        }
    }

    /// Whether this expression is grouped where it stands at `slot`.
    fn needs_parentheses(&self, slot: Slot) -> bool {
        let own = self.precedence();
        match slot {
            Slot::Enclosed => false,
            Slot::Spliced => own == Precedence::Try,
            Slot::Postfix => own < Precedence::Postfix,
            Slot::Prefix => own < Precedence::Prefix || matches!(self, Self::Unary { .. }),
            Slot::Test => own <= Precedence::Ternary,
            Slot::Left(parent) => {
                own < parent || (own == parent && parent.associativity() != Associativity::Left)
            }
            Slot::Right(parent) => {
                own < parent || (own == parent && parent.associativity() != Associativity::Right)
            }
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
                    if let Some(label) = &arg.label {
                        out.push_str(label);
                        out.push_str(": ");
                    }
                    arg.value.write(Slot::Enclosed, out);
                }
                out.push(')');
            }
            Self::Member { base, name } => {
                base.write(Slot::Postfix, out);
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
                operand.write(Slot::Prefix, out);
            }
            Self::Binary { op, left, right } => {
                left.write(Slot::Left(op.precedence()), out);
                out.push(' ');
                out.push_str(op.token());
                out.push(' ');
                right.write(Slot::Right(op.precedence()), out);
            }
            Self::Conditional {
                test,
                then,
                otherwise,
            } => {
                test.write(Slot::Test, out);
                out.push_str(" ? ");
                then.write(Slot::Enclosed, out);
                out.push_str(" : ");
                otherwise.write(Slot::Right(Precedence::Ternary), out);
            }
            Self::Try(value) => {
                out.push_str("try ");
                value.write(Slot::Enclosed, out);
            }
            Self::Closure { signature, body } => {
                out.push_str("{ ");
                out.push_str(signature);
                out.push_str(" in ");
                body.write(Slot::Enclosed, out);
                out.push_str(" }");
            }
        }
    }
}

/// The whole expression, as the emitter hands it on.
impl fmt::Display for SwiftExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = String::new();
        self.write(Slot::Spliced, &mut out);
        f.write_str(&out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str) -> SwiftExpr {
        SwiftExpr::atom(text)
    }

    fn op(op: SwiftBinOp, left: SwiftExpr, right: SwiftExpr) -> SwiftExpr {
        SwiftExpr::binary(op, left, right)
    }

    fn choice() -> SwiftExpr {
        SwiftExpr::conditional(at("c"), at("a"), at("b"))
    }

    fn checked(helper: &str, args: Vec<SwiftExpr>) -> SwiftExpr {
        SwiftExpr::path_call(format!("Checked.{helper}"), args).tried()
    }

    /// An operator groups an operand of a looser group, and one of its own
    /// group where the group does not associate that way: a shift binds
    /// tighter than a multiplication, `&` is a multiplication and `|` an
    /// addition, and neither a shift nor a comparison associates.
    #[test]
    fn an_operand_is_grouped_by_its_precedence_group() {
        let (a, b, c) = (at("a"), at("b"), at("c"));
        let sum = op(SwiftBinOp::Add, a.clone(), b.clone());
        assert_eq!(
            op(SwiftBinOp::Mul, sum.clone(), c.clone()).to_string(),
            "(a + b) * c"
        );
        assert_eq!(
            op(
                SwiftBinOp::BitAnd,
                op(SwiftBinOp::Shr, at("raw"), at("4")),
                at("0x0F")
            )
            .to_string(),
            "raw >> 4 & 0x0F"
        );
        assert_eq!(
            op(
                SwiftBinOp::BitOr,
                op(SwiftBinOp::Add, at("raw"), at("1")),
                at("2")
            )
            .to_string(),
            "raw + 1 | 2"
        );
        assert_eq!(
            op(SwiftBinOp::Shl, sum.clone(), c.clone()).to_string(),
            "(a + b) << c"
        );
        assert_eq!(
            op(
                SwiftBinOp::Shl,
                op(SwiftBinOp::Shl, a.clone(), b.clone()),
                c.clone()
            )
            .to_string(),
            "(a << b) << c"
        );
        assert_eq!(
            op(
                SwiftBinOp::Eq,
                op(SwiftBinOp::Lt, a.clone(), b.clone()),
                c.clone()
            )
            .to_string(),
            "(a < b) == c"
        );
        assert_eq!(
            op(
                SwiftBinOp::Sub,
                a.clone(),
                op(SwiftBinOp::Sub, b.clone(), c.clone())
            )
            .to_string(),
            "a - (b - c)"
        );
        // `??` associates to the right and binds tighter than a comparison.
        assert_eq!(
            op(
                SwiftBinOp::Coalesce,
                a.clone(),
                op(SwiftBinOp::Coalesce, b.clone(), c.clone())
            )
            .to_string(),
            "a ?? b ?? c"
        );
        assert_eq!(
            op(
                SwiftBinOp::Coalesce,
                a.clone(),
                op(SwiftBinOp::Eq, b.clone(), c.clone())
            )
            .to_string(),
            "a ?? (b == c)"
        );
        assert_eq!(
            op(
                SwiftBinOp::Eq,
                op(SwiftBinOp::Coalesce, a.clone(), b.clone()),
                c
            )
            .to_string(),
            "a ?? b == c"
        );
        assert_eq!(sum.clone().member("count").to_string(), "(a + b).count");
        assert_eq!(sum.prefixed(SwiftUnOp::Neg).to_string(), "-(a + b)");
        assert_eq!(
            a.member("count").prefixed(SwiftUnOp::Neg).to_string(),
            "-a.count"
        );
    }

    /// A run of operator characters is one operator, so a prefix operator over
    /// another is grouped.
    #[test]
    fn a_prefix_operator_over_a_prefix_operator_is_grouped() {
        let negated = at("n").prefixed(SwiftUnOp::Neg);
        assert_eq!(
            negated.clone().prefixed(SwiftUnOp::Neg).to_string(),
            "-(-n)"
        );
        assert_eq!(negated.prefixed(SwiftUnOp::Not).to_string(), "!(-n)");
        assert_eq!(
            at("flag")
                .prefixed(SwiftUnOp::Not)
                .prefixed(SwiftUnOp::Not)
                .to_string(),
            "!(!flag)"
        );
    }

    /// A `try` is grouped as an operand and where it is the whole text, and
    /// bare as an argument, in a closure's body and in a conditional's middle.
    #[test]
    fn a_try_is_grouped_as_an_operand_and_where_it_leaves() {
        let add = checked("add", vec![at("count"), at("step")]);
        assert_eq!(add.to_string(), "(try Checked.add(count, step))");
        assert_eq!(
            checked("mul", vec![add.clone(), at("2")]).to_string(),
            "(try Checked.mul(try Checked.add(count, step), 2))"
        );
        assert_eq!(
            op(SwiftBinOp::Gt, add.clone(), at("3")).to_string(),
            "(try Checked.add(count, step)) > 3"
        );
        assert_eq!(
            op(SwiftBinOp::Gt, at("3"), add.clone()).to_string(),
            "3 > (try Checked.add(count, step))"
        );
        let body = op(SwiftBinOp::Gt, add.clone(), at("3"));
        let closure = SwiftExpr::closure("() throws(AlgorithmError) -> Bool", body);
        assert_eq!(
            checked("and", vec![at("flag"), closure]).to_string(),
            "(try Checked.and(flag, { () throws(AlgorithmError) -> Bool in \
             (try Checked.add(count, step)) > 3 }))"
        );
        assert_eq!(
            SwiftExpr::conditional(at("flag"), add.clone(), at("0")).to_string(),
            "flag ? try Checked.add(count, step) : 0"
        );
        assert_eq!(
            SwiftExpr::conditional(at("flag"), at("0"), add.clone()).to_string(),
            "flag ? 0 : (try Checked.add(count, step))"
        );
        assert_eq!(
            add.member("count").to_string(),
            "(try Checked.add(count, step)).count"
        );
    }

    /// A conditional is grouped as a condition and as an operand, and bare as
    /// its middle and as its else branch, where it associates.
    #[test]
    fn a_conditional_associates_through_its_else_branch() {
        assert_eq!(
            SwiftExpr::conditional(choice(), choice(), choice()).to_string(),
            "(c ? a : b) ? c ? a : b : c ? a : b"
        );
        assert_eq!(
            op(SwiftBinOp::Add, choice(), at("1")).to_string(),
            "(c ? a : b) + 1"
        );
    }

    /// A call passes each argument by its label, and an argument is an
    /// expression of its own.
    #[test]
    fn an_argument_is_passed_by_its_label() {
        let shifted = op(
            SwiftBinOp::Shr,
            SwiftExpr::call(
                at("UInt32"),
                vec![SwiftArg::labeled("truncatingIfNeeded", at("word"))],
            ),
            at("3"),
        );
        assert_eq!(
            SwiftExpr::call(
                at("Int32"),
                vec![SwiftArg::labeled("truncatingIfNeeded", shifted)]
            )
            .to_string(),
            "Int32(truncatingIfNeeded: UInt32(truncatingIfNeeded: word) >> 3)"
        );
    }

    // ── The printer read back by the precedence groups ─────────────

    /// The infix operators of a sequence, a conditional's `? middle :` among
    /// them.
    enum Infix {
        Binary(SwiftBinOp),
        Ternary(SwiftExpr),
    }

    /// The standard library's declaration of the precedence group of an infix
    /// operator, as the rank of the group (higher binds tighter) and how it
    /// associates: `BitwiseShiftPrecedence` (none), `MultiplicationPrecedence`
    /// and `AdditionPrecedence` (left), `NilCoalescingPrecedence` (right),
    /// `ComparisonPrecedence` (none), `LogicalConjunctionPrecedence` and
    /// `LogicalDisjunctionPrecedence` (left), `TernaryPrecedence` (right).
    fn group(infix: &Infix) -> (u8, Associativity) {
        let op = match infix {
            Infix::Ternary(_) => return (1, Associativity::Right),
            Infix::Binary(op) => op,
        };
        match op.token() {
            "<<" | ">>" => (9, Associativity::None),
            "*" | "/" | "%" | "&" => (8, Associativity::Left),
            "+" | "-" | "|" | "^" => (7, Associativity::Left),
            "??" => (5, Associativity::Right),
            "==" | "!=" | "<" | ">" | "<=" | ">=" => (4, Associativity::None),
            "&&" => (3, Associativity::Left),
            "||" => (2, Associativity::Left),
            other => panic!("`{other}` has no group here"),
        }
    }

    /// The infix operator Swift reads `run`, a run of operator characters, as.
    fn infix(run: &str) -> Result<SwiftBinOp, String> {
        Ok(match run {
            "<<" => SwiftBinOp::Shl,
            ">>" => SwiftBinOp::Shr,
            "*" => SwiftBinOp::Mul,
            "/" => SwiftBinOp::Div,
            "%" => SwiftBinOp::Rem,
            "&" => SwiftBinOp::BitAnd,
            "+" => SwiftBinOp::Add,
            "-" => SwiftBinOp::Sub,
            "|" => SwiftBinOp::BitOr,
            "^" => SwiftBinOp::BitXor,
            "??" => SwiftBinOp::Coalesce,
            "==" => SwiftBinOp::Eq,
            "!=" => SwiftBinOp::Ne,
            "<" => SwiftBinOp::Lt,
            ">" => SwiftBinOp::Gt,
            "<=" => SwiftBinOp::Le,
            ">=" => SwiftBinOp::Ge,
            "&&" => SwiftBinOp::And,
            "||" => SwiftBinOp::Or,
            other => return Err(format!("`{other}` is no infix operator Swift declares")),
        })
    }

    /// The tree a sequence of operands and infix operators is, folded by the
    /// groups' ranks and associativity, as Swift folds a sequence expression:
    /// two operators of a group that does not associate refuse the sequence.
    fn fold(operands: Vec<SwiftExpr>, operators: Vec<Infix>) -> Result<SwiftExpr, String> {
        fn reduce(values: &mut Vec<SwiftExpr>, pending: &mut Vec<Infix>) {
            let right = values.pop().expect("a right operand");
            let left = values.pop().expect("a left operand");
            values.push(match pending.pop().expect("an operator") {
                Infix::Binary(op) => SwiftExpr::binary(op, left, right),
                Infix::Ternary(then) => SwiftExpr::conditional(left, then, right),
            });
        }
        let mut operands = operands.into_iter();
        let mut values = vec![operands.next().expect("a first operand")];
        let mut pending: Vec<Infix> = Vec::new();
        for (operator, operand) in operators.into_iter().zip(operands) {
            let (rank, associativity) = group(&operator);
            while let Some(top) = pending.last() {
                let (top_rank, _) = group(top);
                if top_rank > rank || (top_rank == rank && associativity == Associativity::Left) {
                    reduce(&mut values, &mut pending);
                } else if top_rank == rank && associativity == Associativity::None {
                    return Err("two operators of a group that does not associate".to_string());
                } else {
                    break;
                }
            }
            pending.push(operator);
            values.push(operand);
        }
        while !pending.is_empty() {
            reduce(&mut values, &mut pending);
        }
        Ok(values.pop().expect("one value"))
    }

    /// A reader of the expressions the printer writes, from Swift's grammar of
    /// an expression: an optional `try`, then a sequence of prefix expressions
    /// and infix operators, each a run of operator characters with space on
    /// both sides (a conditional's `?` and `:` around its middle), folded by
    /// [`fold`]; a prefix operator is a run attached to its operand, and a
    /// postfix expression a primary followed by member reads, calls and
    /// subscripts.
    struct Reader<'a> {
        text: &'a str,
        at: usize,
    }

    /// The characters a Swift operator is made of, of those the printer writes.
    const OPERATOR_CHARACTERS: &str = "/=-+!*%<>&|^~?";

    impl Reader<'_> {
        fn rest(&self) -> &str {
            &self.text[self.at..]
        }

        fn skip_spaces(&mut self) {
            while self.rest().starts_with(' ') {
                self.at += 1;
            }
        }

        fn eat(&mut self, token: &str) -> bool {
            if self.rest().starts_with(token) {
                self.at += token.len();
                true
            } else {
                false
            }
        }

        fn expect(&mut self, token: &str) -> Result<(), String> {
            if self.eat(token) {
                Ok(())
            } else {
                Err(format!("`{token}` expected at `{}`", self.rest()))
            }
        }

        fn operator_run(&self) -> &str {
            let rest = self.rest();
            let end = rest
                .find(|c: char| !OPERATOR_CHARACTERS.contains(c))
                .unwrap_or(rest.len());
            &rest[..end]
        }

        fn word(&mut self) -> Option<String> {
            let rest = self.rest();
            let end = rest
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .unwrap_or(rest.len());
            if end == 0 {
                return None;
            }
            let word = rest[..end].to_string();
            self.at += end;
            Some(word)
        }

        fn expression(&mut self) -> Result<SwiftExpr, String> {
            self.skip_spaces();
            if self.eat("try ") {
                return Ok(self.expression()?.tried());
            }
            let mut operands = vec![self.prefixed()?];
            let mut operators = Vec::new();
            loop {
                self.skip_spaces();
                let run = self.operator_run().to_string();
                if run.is_empty() {
                    break;
                }
                self.at += run.len();
                if run == "?" {
                    let then = self.expression()?;
                    self.skip_spaces();
                    self.expect(":")?;
                    operators.push(Infix::Ternary(then));
                } else {
                    operators.push(Infix::Binary(infix(&run)?));
                }
                self.skip_spaces();
                if self.rest().starts_with("try ") {
                    return Err("a `try` to the right of an operator".to_string());
                }
                operands.push(self.prefixed()?);
            }
            fold(operands, operators)
        }

        fn prefixed(&mut self) -> Result<SwiftExpr, String> {
            let run = self.operator_run().to_string();
            if run.is_empty() {
                return self.postfixed();
            }
            let op = match run.as_str() {
                "-" => SwiftUnOp::Neg,
                "+" => SwiftUnOp::Pos,
                "!" => SwiftUnOp::Not,
                "~" => SwiftUnOp::BitNot,
                other => return Err(format!("`{other}` is no prefix operator Swift declares")),
            };
            self.at += run.len();
            Ok(self.postfixed()?.prefixed(op))
        }

        fn postfixed(&mut self) -> Result<SwiftExpr, String> {
            let mut value = self.primary()?;
            loop {
                if self.eat(".") {
                    let name = self.word().ok_or("a member's name")?;
                    value = value.member(name);
                } else if self.eat("(") {
                    value = SwiftExpr::call(value, self.arguments()?);
                } else if self.eat("[") {
                    let index = self.expression()?;
                    self.skip_spaces();
                    self.expect("]")?;
                    value = value.indexed(index);
                } else {
                    return Ok(value);
                }
            }
        }

        fn arguments(&mut self) -> Result<Vec<SwiftArg>, String> {
            let mut args = Vec::new();
            if self.eat(")") {
                return Ok(args);
            }
            loop {
                self.skip_spaces();
                let start = self.at;
                let label = match self.word() {
                    Some(word) if self.eat(": ") => Some(word),
                    _ => {
                        self.at = start;
                        None
                    }
                };
                let value = self.expression()?;
                args.push(SwiftArg { label, value });
                self.skip_spaces();
                if self.eat(", ") {
                    continue;
                }
                self.expect(")")?;
                return Ok(args);
            }
        }

        fn primary(&mut self) -> Result<SwiftExpr, String> {
            if self.eat("(") {
                let value = self.expression()?;
                self.skip_spaces();
                self.expect(")")?;
                return Ok(value);
            }
            if self.eat("{ ") {
                let end = self.rest().find(" in ").ok_or("a closure's `in`")?;
                let signature = self.rest()[..end].to_string();
                self.at += end + " in ".len();
                let body = self.expression()?;
                self.skip_spaces();
                self.expect("}")?;
                return Ok(SwiftExpr::closure(signature, body));
            }
            self.word()
                .map(SwiftExpr::Atom)
                .ok_or_else(|| format!("an operand expected at `{}`", self.rest()))
        }
    }

    /// The tree `text` is, read whole.
    fn read(text: &str) -> Result<SwiftExpr, String> {
        let mut reader = Reader { text, at: 0 };
        let value = reader.expression()?;
        if reader.at == text.len() {
            Ok(value)
        } else {
            Err(format!("`{}` is left over", reader.rest()))
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

    const OPERATORS: [SwiftBinOp; 19] = [
        SwiftBinOp::Shl,
        SwiftBinOp::Shr,
        SwiftBinOp::Mul,
        SwiftBinOp::Div,
        SwiftBinOp::Rem,
        SwiftBinOp::BitAnd,
        SwiftBinOp::Add,
        SwiftBinOp::Sub,
        SwiftBinOp::BitOr,
        SwiftBinOp::BitXor,
        SwiftBinOp::Coalesce,
        SwiftBinOp::Eq,
        SwiftBinOp::Ne,
        SwiftBinOp::Lt,
        SwiftBinOp::Gt,
        SwiftBinOp::Le,
        SwiftBinOp::Ge,
        SwiftBinOp::And,
        SwiftBinOp::Or,
    ];

    const PREFIXES: [SwiftUnOp; 4] = [
        SwiftUnOp::Neg,
        SwiftUnOp::Pos,
        SwiftUnOp::Not,
        SwiftUnOp::BitNot,
    ];

    /// The seed every drawing starts from.
    const SEED: u64 = 0x9E37_79B9_7F4A_7C15;

    /// How many trees a drawing holds.
    const TREES: usize = 4000;

    /// A tree of every node kind, at most `depth` deep.
    fn tree(draw: &mut Draw, depth: u32) -> SwiftExpr {
        if depth == 0 || draw.below(6) == 0 {
            return at(draw.pick(&["a", "b", "x", "2", "7", "true", "nil"]));
        }
        let d = depth - 1;
        match draw.below(15) {
            0 => {
                let callee = if draw.below(4) == 0 {
                    tree(draw, d)
                } else {
                    at("f")
                };
                let count = draw.below(3);
                let args = (0..count)
                    .map(|_| {
                        let value = tree(draw, d);
                        match draw.below(3) {
                            0 => SwiftArg::labeled("to", value),
                            _ => SwiftArg::bare(value),
                        }
                    })
                    .collect();
                SwiftExpr::call(callee, args)
            }
            1 => {
                let base = tree(draw, d);
                base.member(draw.pick(&["count", "utf8"]))
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
                SwiftExpr::binary(operator, left, right)
            }
            10 | 11 => {
                let test = tree(draw, d);
                let then = tree(draw, d);
                let otherwise = tree(draw, d);
                SwiftExpr::conditional(test, then, otherwise)
            }
            12 | 13 => tree(draw, d).tried(),
            _ => SwiftExpr::closure("() throws(AlgorithmError) -> Bool", tree(draw, d)),
        }
    }

    /// Every tree the printer is handed is read back as itself by the reader
    /// of the precedence groups: every parenthesis Swift needs is printed, a
    /// `try` never stands to the right of an operator, and none the printer
    /// leaves out changes what is read.
    #[test]
    fn what_the_printer_writes_reads_back_as_the_tree_it_was_handed() {
        let mut draw = Draw(SEED);
        for _ in 0..TREES {
            let tree = tree(&mut draw, 5);
            let text = tree.to_string();
            let read = read(&text).unwrap_or_else(|e| panic!("`{text}` is not read: {e}"));
            assert_eq!(read, tree, "`{text}` read back");
        }
    }

    /// The kind of a node, as its variant is named.
    fn kind(tree: &SwiftExpr) -> &'static str {
        match tree {
            SwiftExpr::Atom(_) => "Atom",
            SwiftExpr::Call { .. } => "Call",
            SwiftExpr::Member { .. } => "Member",
            SwiftExpr::Index { .. } => "Index",
            SwiftExpr::Unary { .. } => "Unary",
            SwiftExpr::Binary { .. } => "Binary",
            SwiftExpr::Conditional { .. } => "Conditional",
            SwiftExpr::Try(_) => "Try",
            SwiftExpr::Closure { .. } => "Closure",
        }
    }

    /// The nodes directly inside `tree`.
    fn children(tree: &SwiftExpr) -> Vec<&SwiftExpr> {
        match tree {
            SwiftExpr::Atom(_) => Vec::new(),
            SwiftExpr::Call { callee, args } => std::iter::once(&**callee)
                .chain(args.iter().map(|arg| &arg.value))
                .collect(),
            SwiftExpr::Member { base, .. } => vec![&**base],
            SwiftExpr::Index { base, index } => vec![&**base, &**index],
            SwiftExpr::Unary { operand, .. } => vec![&**operand],
            SwiftExpr::Binary { left, right, .. } => vec![&**left, &**right],
            SwiftExpr::Conditional {
                test,
                then,
                otherwise,
            } => vec![&**test, &**then, &**otherwise],
            SwiftExpr::Try(value) => vec![&**value],
            SwiftExpr::Closure { body, .. } => vec![&**body],
        }
    }

    /// Every kind of node is drawn, and a `try`, a conditional, a prefix
    /// operator and an infix one in every parent, so the read-back above
    /// reaches each rule of the printer.
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
                        SwiftExpr::Try(_)
                            | SwiftExpr::Conditional { .. }
                            | SwiftExpr::Unary { .. }
                            | SwiftExpr::Binary { .. }
                    ) {
                        seen.insert(format!("{} in {}", kind(child), kind(&node)));
                    }
                    stack.push(child.clone());
                }
            }
        }
        let kinds = [
            "Atom",
            "Call",
            "Member",
            "Index",
            "Unary",
            "Binary",
            "Conditional",
            "Try",
            "Closure",
        ];
        for kind in kinds {
            assert!(seen.contains(kind), "no {kind} was drawn: {seen:?}");
        }
        for child in ["Try", "Conditional", "Unary", "Binary"] {
            for parent in kinds.iter().filter(|kind| **kind != "Atom") {
                assert!(
                    seen.contains(&format!("{child} in {parent}")),
                    "no {child} was drawn in a {parent}: {seen:?}"
                );
            }
        }
    }
}
