// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The C11 a forge expression is written as, as a tree of the C expressions it
//! is made of, and the one printer that writes such a tree as text.
//!
//! The C emitter ([`super::emit_c`]) and the C11 union reads of
//! [`crate::forge::composed_lowering`] build this tree; nothing is read back out
//! of text. A conversion is a cast node wherever it comes from: a narrow shift
//! made on its bits, a count handed to the runtime, an unsigned operand where
//! the join is signed, a length met by a signed context.
//!
//! The printer decides every parenthesis, at each parent, from the kind of the
//! child and the precedence and associativity C11 gives its operators: the
//! order of the subclauses of ISO/IEC 9899:2011 clause 6.5 (Expressions), from
//! the primary expressions of 6.5.1 to the conditional operator of 6.5.15, each
//! binary operator associating to the left. That is the textbook unparsing rule
//! (Ramsey, "Unparsing Expressions with Prefix and Postfix Operators", 1998).
//! The rules of the grammar that a precedence table does not say are the
//! printer's too:
//!
//! * the operand of a unary operator and of a cast is a cast-expression
//!   (6.5.3, 6.5.4), so a cast stands bare under either;
//! * the condition of `?:` is a logical-OR-expression and its last operand a
//!   conditional-expression (6.5.15): a conditional is grouped in the first
//!   and stands bare in the last, and the middle operand is an expression,
//!   which stands bare;
//! * `sizeof` is written over a parenthesized expression, a primary expression
//!   (6.5.1), so its operand stands bare inside its parentheses;
//! * a compound literal is a postfix expression (6.5.2.5) and its initializers
//!   stand bare between its braces;
//! * a prefix `-`, `+` or `&` before an operand that begins with the same
//!   character would be read as `--`, `++` or `&&` (the longest token wins,
//!   6.4), so such an operand is grouped.
//!
//! The generated C is built with `-Wall -Wextra -Werror`, so the operands GCC and
//! Clang flag under `-Wparentheses` and `-Wlogical-not-parentheses` are grouped
//! where precedence alone would leave them bare (`CBinOp::flags`): a comparison
//! under a comparison, `&&` under `||`, a sum or a comparison under a bitwise
//! operator, `&` or `^` under `|` and `&` under `^`, a sum under a shift,
//! and a `!` that is the left operand of a comparison or of `&`.
//!
//! Everything else stands bare. The tests hold the printer to the grammar by
//! reading what it prints back with a reader written from the productions of
//! clause 6.5 and comparing the tree read with the tree printed, and compile
//! a printed form of each rule with the flags the generated C is built with.

use std::fmt;

/// A C expression the emitter writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CExpr {
    /// Text written as it is that stands as one operand: a literal, a name, a
    /// macro's call, or a fragment its writer made one
    /// ([`super::ExprKind::Raw`]).
    Atom(String),
    /// `callee(args)`.
    Call {
        callee: Box<CExpr>,
        args: Vec<CExpr>,
    },
    /// `base.member`.
    Field { base: Box<CExpr>, member: String },
    /// `base[index]`.
    Index { base: Box<CExpr>, index: Box<CExpr> },
    /// The compound literal `(ty){ initializers }`, or `(ty){0}`, the object
    /// of the type with every member zero, where `initializers` is `None`.
    Compound {
        ty: String,
        initializers: Option<Vec<CInit>>,
    },
    /// A prefix operator over its operand.
    Unary { op: CUnOp, operand: Box<CExpr> },
    /// `sizeof(value)`.
    Sizeof(Box<CExpr>),
    /// `(ty)value`.
    Cast { ty: String, value: Box<CExpr> },
    /// `left op right`.
    Binary {
        op: CBinOp,
        left: Box<CExpr>,
        right: Box<CExpr>,
    },
    /// `condition ? then : otherwise`.
    Conditional {
        condition: Box<CExpr>,
        then: Box<CExpr>,
        otherwise: Box<CExpr>,
    },
}

/// One initializer of a [`CExpr::Compound`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CInit {
    /// The next member's value.
    Value(CExpr),
    /// `.member = value`.
    Member { member: String, value: CExpr },
}

/// The prefix operators the emitter writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CUnOp {
    /// `-`
    Neg,
    /// `+`
    Plus,
    /// `!`
    Not,
    /// `~`
    BitNot,
    /// `&`, the address of its operand.
    Address,
}

impl CUnOp {
    fn token(self) -> &'static str {
        match self {
            Self::Neg => "-",
            Self::Plus => "+",
            Self::Not => "!",
            Self::BitNot => "~",
            Self::Address => "&",
        }
    }

    /// The character an operand may not begin with bare after this operator,
    /// where the two would be read as one longer token: `--`, `++` or `&&`.
    fn glues_to(self) -> Option<char> {
        match self {
            Self::Neg => Some('-'),
            Self::Plus => Some('+'),
            Self::Address => Some('&'),
            Self::Not | Self::BitNot => None,
        }
    }
}

/// The binary operators the emitter writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CBinOp {
    Mul,
    Div,
    Rem,
    Add,
    Sub,
    Shl,
    Shr,
    Lt,
    Gt,
    Le,
    Ge,
    Eq,
    Ne,
    BitAnd,
    BitXor,
    BitOr,
    And,
    Or,
}

impl CBinOp {
    fn token(self) -> &'static str {
        match self {
            Self::Mul => "*",
            Self::Div => "/",
            Self::Rem => "%",
            Self::Add => "+",
            Self::Sub => "-",
            Self::Shl => "<<",
            Self::Shr => ">>",
            Self::Lt => "<",
            Self::Gt => ">",
            Self::Le => "<=",
            Self::Ge => ">=",
            Self::Eq => "==",
            Self::Ne => "!=",
            Self::BitAnd => "&",
            Self::BitXor => "^",
            Self::BitOr => "|",
            Self::And => "&&",
            Self::Or => "||",
        }
    }

    /// The subclause of clause 6.5 the operator is defined in, as a level.
    fn precedence(self) -> Precedence {
        match self {
            Self::Mul | Self::Div | Self::Rem => Precedence::Multiplicative,
            Self::Add | Self::Sub => Precedence::Additive,
            Self::Shl | Self::Shr => Precedence::Shift,
            Self::Lt | Self::Gt | Self::Le | Self::Ge => Precedence::Relational,
            Self::Eq | Self::Ne => Precedence::Equality,
            Self::BitAnd => Precedence::BitAnd,
            Self::BitXor => Precedence::BitXor,
            Self::BitOr => Precedence::BitOr,
            Self::And => Precedence::And,
            Self::Or => Precedence::Or,
        }
    }

    fn is_comparison(self) -> bool {
        matches!(
            self.precedence(),
            Precedence::Relational | Precedence::Equality
        )
    }

    /// Whether GCC or Clang flags `child`, bare, as an operand of this operator
    /// under `-Wparentheses`, which the generated C is built with as an error.
    /// Precedence reads every one of them as the document means it; the
    /// build does not take it.
    ///
    /// `a == 2 || b >= 1 && b <= 4` stopped a downstream build under
    /// `-Wall -Werror`, and `h * 5 + c & m` the C11 conformance build of
    /// `sce:std/hash/murmur3_32`.
    fn flags(self, child: CBinOp) -> bool {
        if self.is_comparison() && child.is_comparison() {
            return true;
        }
        let additive = matches!(child, Self::Add | Self::Sub);
        match self {
            Self::Or => child == Self::And,
            Self::BitAnd => additive || child.is_comparison(),
            Self::BitXor => additive || child.is_comparison() || child == Self::BitAnd,
            Self::BitOr => {
                additive || child.is_comparison() || matches!(child, Self::BitAnd | Self::BitXor)
            }
            Self::Shl | Self::Shr => additive,
            _ => false,
        }
    }

    /// Whether a `!` that is the left operand of this operator is flagged
    /// under `-Wlogical-not-parentheses`: GCC flags it before a comparison,
    /// Clang before a comparison and before `&`.
    fn flags_a_negated_left(self) -> bool {
        self.is_comparison() || self == Self::BitAnd
    }
}

/// How tightly an expression binds, loosest first: the subclauses of clause
/// 6.5 the emitter writes, and above them every expression no operator can
/// split (a name, a literal).
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
    Cast,
    Unary,
    Postfix,
    Primary,
}

/// Where a child stands in its parent, which is all the printer needs to
/// decide whether it is grouped.
#[derive(Debug, Clone, Copy)]
enum Slot {
    /// Between delimiters of the parent's own (an argument, an index, an
    /// initializer, the operand of `sizeof`, the middle of `?:`), or the whole
    /// expression.
    Enclosed,
    /// What a call calls, or what a field read or an index reads.
    Postfix,
    /// The operand of a prefix operator.
    Prefix(CUnOp),
    /// The value a cast converts.
    Cast,
    /// The left operand of a binary operator.
    Left(CBinOp),
    /// The right operand of a binary operator.
    Right(CBinOp),
    /// The condition of `?:`.
    Condition,
    /// The last operand of `?:`.
    Otherwise,
}

impl CExpr {
    /// Text that stands as one operand, written as it is.
    pub(crate) fn atom(text: impl Into<String>) -> Self {
        Self::Atom(text.into())
    }

    /// A call of `callee` with `args`.
    pub(crate) fn call(callee: CExpr, args: Vec<CExpr>) -> Self {
        Self::Call {
            callee: Box::new(callee),
            args,
        }
    }

    /// A call of the function or the macro `name`.
    pub(crate) fn path_call(name: impl Into<String>, args: Vec<CExpr>) -> Self {
        Self::call(Self::atom(name), args)
    }

    /// The member `member` of this value.
    pub(crate) fn field(self, member: impl Into<String>) -> Self {
        Self::Field {
            base: Box::new(self),
            member: member.into(),
        }
    }

    /// This value indexed at `index`.
    pub(crate) fn indexed(self, index: CExpr) -> Self {
        Self::Index {
            base: Box::new(self),
            index: Box::new(index),
        }
    }

    /// The compound literal of `ty` with `initializers`.
    pub(crate) fn compound(ty: impl Into<String>, initializers: Vec<CInit>) -> Self {
        Self::Compound {
            ty: ty.into(),
            initializers: Some(initializers),
        }
    }

    /// The object of `ty` with every member zero, `(ty){0}`.
    pub(crate) fn zeroed(ty: impl Into<String>) -> Self {
        Self::Compound {
            ty: ty.into(),
            initializers: None,
        }
    }

    /// The prefix operator `op` over this value.
    pub(crate) fn prefixed(self, op: CUnOp) -> Self {
        Self::Unary {
            op,
            operand: Box::new(self),
        }
    }

    /// The size of this value, `sizeof(value)`.
    pub(crate) fn size_of(self) -> Self {
        Self::Sizeof(Box::new(self))
    }

    /// This value converted to the type `ty`.
    pub(crate) fn cast(self, ty: impl Into<String>) -> Self {
        Self::Cast {
            ty: ty.into(),
            value: Box::new(self),
        }
    }

    /// `left op right`.
    pub(crate) fn binary(op: CBinOp, left: CExpr, right: CExpr) -> Self {
        Self::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    /// `condition ? then : otherwise`.
    pub(crate) fn conditional(condition: CExpr, then: CExpr, otherwise: CExpr) -> Self {
        Self::Conditional {
            condition: Box::new(condition),
            then: Box::new(then),
            otherwise: Box::new(otherwise),
        }
    }

    /// The name this value is, with `suffix` after it: the sibling a C
    /// struct holds beside a member (`raw` and `raw_len`), or beside a name. `None`
    /// for any value that is not a name or a member, which has no sibling.
    pub(crate) fn sibling(self, suffix: &str) -> Option<Self> {
        match self {
            Self::Atom(text) if is_a_name(&text) => Some(Self::Atom(text + suffix)),
            Self::Field { base, member } if is_a_name(&member) => Some(Self::Field {
                base,
                member: member + suffix,
            }),
            _ => None,
        }
    }

    /// This expression printed to stand where a fragment is spliced as it is
    /// ([`super::ExprKind::Raw`], a read a spelling of another layer writes
    /// after it): grouped unless it is one operand that no operator, cast or
    /// postfix read around it can take apart.
    pub(crate) fn as_operand(&self) -> String {
        let mut out = String::new();
        self.write(Slot::Postfix, &mut out);
        out
    }

    fn precedence(&self) -> Precedence {
        match self {
            Self::Conditional { .. } => Precedence::Conditional,
            Self::Binary { op, .. } => op.precedence(),
            Self::Cast { .. } => Precedence::Cast,
            Self::Unary { .. } | Self::Sizeof(_) => Precedence::Unary,
            Self::Call { .. } | Self::Field { .. } | Self::Index { .. } | Self::Compound { .. } => {
                Precedence::Postfix
            }
            Self::Atom(_) => Precedence::Primary,
        }
    }

    /// Whether this expression is grouped where it stands at `slot`.
    fn needs_parentheses(&self, slot: Slot) -> bool {
        let own = self.precedence();
        match slot {
            Slot::Enclosed => false,
            Slot::Postfix => own < Precedence::Postfix,
            Slot::Prefix(op) => {
                own < Precedence::Cast
                    || op.glues_to().is_some_and(|c| self.first_char() == Some(c))
            }
            Slot::Cast => own < Precedence::Cast,
            Slot::Left(op) => {
                own < op.precedence()
                    || self.is_flagged_under(op)
                    || (op.flags_a_negated_left()
                        && matches!(self, Self::Unary { op: CUnOp::Not, .. }))
            }
            Slot::Right(op) => own <= op.precedence() || self.is_flagged_under(op),
            Slot::Condition => own < Precedence::Or,
            Slot::Otherwise => own < Precedence::Conditional,
        }
    }

    /// Whether this expression is a binary operation `parent` flags as its
    /// operand ([`CBinOp::flags`]).
    fn is_flagged_under(&self, parent: CBinOp) -> bool {
        matches!(self, Self::Binary { op, .. } if parent.flags(*op))
    }

    /// The first character this expression is printed with, bare.
    fn first_char(&self) -> Option<char> {
        match self {
            Self::Atom(text) => text.chars().next(),
            Self::Unary { op, .. } => op.token().chars().next(),
            Self::Sizeof(_) => Some('s'),
            Self::Cast { .. } | Self::Compound { .. } => Some('('),
            Self::Call { callee: base, .. }
            | Self::Field { base, .. }
            | Self::Index { base, .. } => base.first_char_at(Slot::Postfix),
            Self::Binary { op, left, .. } => left.first_char_at(Slot::Left(*op)),
            Self::Conditional { condition, .. } => condition.first_char_at(Slot::Condition),
        }
    }

    /// The first character this expression is printed with at `slot`.
    fn first_char_at(&self, slot: Slot) -> Option<char> {
        if self.needs_parentheses(slot) {
            Some('(')
        } else {
            self.first_char()
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
            Self::Field { base, member } => {
                base.write(Slot::Postfix, out);
                out.push('.');
                out.push_str(member);
            }
            Self::Index { base, index } => {
                base.write(Slot::Postfix, out);
                out.push('[');
                index.write(Slot::Enclosed, out);
                out.push(']');
            }
            Self::Compound { ty, initializers } => {
                out.push('(');
                out.push_str(ty);
                out.push_str("){");
                match initializers {
                    None => out.push('0'),
                    Some(initializers) => {
                        out.push(' ');
                        for (i, initializer) in initializers.iter().enumerate() {
                            if i > 0 {
                                out.push_str(", ");
                            }
                            match initializer {
                                CInit::Value(value) => value.write(Slot::Enclosed, out),
                                CInit::Member { member, value } => {
                                    out.push('.');
                                    out.push_str(member);
                                    out.push_str(" = ");
                                    value.write(Slot::Enclosed, out);
                                }
                            }
                        }
                        out.push(' ');
                    }
                }
                out.push('}');
            }
            Self::Unary { op, operand } => {
                out.push_str(op.token());
                operand.write(Slot::Prefix(*op), out);
            }
            Self::Sizeof(value) => {
                out.push_str("sizeof(");
                value.write(Slot::Enclosed, out);
                out.push(')');
            }
            Self::Cast { ty, value } => {
                out.push('(');
                out.push_str(ty);
                out.push(')');
                value.write(Slot::Cast, out);
            }
            Self::Binary { op, left, right } => {
                left.write(Slot::Left(*op), out);
                out.push(' ');
                out.push_str(op.token());
                out.push(' ');
                right.write(Slot::Right(*op), out);
            }
            Self::Conditional {
                condition,
                then,
                otherwise,
            } => {
                condition.write(Slot::Condition, out);
                out.push_str(" ? ");
                then.write(Slot::Enclosed, out);
                out.push_str(" : ");
                otherwise.write(Slot::Otherwise, out);
            }
        }
    }
}

/// Whether `text` is one C identifier, or several joined by `.` and `->`
/// (a member path a rename wrote whole), which a suffix extends into the name
/// of a sibling.
fn is_a_name(text: &str) -> bool {
    !text.is_empty()
        && text
            .split("->")
            .flat_map(|part| part.split('.'))
            .all(|part| {
                part.chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                    && part.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            })
}

/// The whole expression, as the emitter hands it on.
impl fmt::Display for CExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = String::new();
        self.write(Slot::Enclosed, &mut out);
        f.write_str(&out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str) -> CExpr {
        CExpr::atom(text)
    }

    fn op(op: CBinOp, left: CExpr, right: CExpr) -> CExpr {
        CExpr::binary(op, left, right)
    }

    /// An operator groups an operand that binds more loosely than itself, and a
    /// right operand that binds as tightly, since every binary operator of C
    /// associates to the left; a cast and a prefix operator take a cast
    /// bare, and a postfix read groups everything below a postfix expression.
    #[test]
    fn an_operand_is_grouped_by_precedence_and_left_association() {
        let (a, b, c) = (at("a"), at("b"), at("c"));
        let sum = op(CBinOp::Add, a.clone(), b.clone());
        assert_eq!(
            op(CBinOp::Mul, sum.clone(), c.clone()).to_string(),
            "(a + b) * c"
        );
        assert_eq!(
            op(
                CBinOp::Add,
                op(CBinOp::Mul, a.clone(), b.clone()),
                c.clone()
            )
            .to_string(),
            "a * b + c"
        );
        assert_eq!(
            op(
                CBinOp::Sub,
                op(CBinOp::Sub, a.clone(), b.clone()),
                c.clone()
            )
            .to_string(),
            "a - b - c"
        );
        assert_eq!(
            op(
                CBinOp::Sub,
                a.clone(),
                op(CBinOp::Sub, b.clone(), c.clone())
            )
            .to_string(),
            "a - (b - c)"
        );
        assert_eq!(
            op(CBinOp::Or, op(CBinOp::Or, a.clone(), b.clone()), c.clone()).to_string(),
            "a || b || c"
        );
        assert_eq!(sum.clone().prefixed(CUnOp::Neg).to_string(), "-(a + b)");
        assert_eq!(sum.clone().cast("uint8_t").to_string(), "(uint8_t)(a + b)");
        assert_eq!(
            a.clone().field("len").cast("int64_t").to_string(),
            "(int64_t)a.len"
        );
        assert_eq!(
            a.clone()
                .prefixed(CUnOp::BitNot)
                .cast("uint8_t")
                .to_string(),
            "(uint8_t)~a"
        );
        assert_eq!(
            a.clone().cast("uint8_t").prefixed(CUnOp::Neg).to_string(),
            "-(uint8_t)a"
        );
        assert_eq!(
            a.clone().cast("uint8_t").field("f").to_string(),
            "((uint8_t)a).f"
        );
        assert_eq!(sum.clone().field("len").to_string(), "(a + b).len");
        assert_eq!(
            CExpr::path_call("f", vec![sum.clone()]).to_string(),
            "f(a + b)"
        );
        assert_eq!(
            op(CBinOp::Shr, a.clone().cast("uint8_t"), c.clone()).to_string(),
            "(uint8_t)a >> c"
        );
        assert_eq!(sum.size_of().to_string(), "sizeof(a + b)");
    }

    /// The condition of `?:` groups a conditional, its last operand takes one
    /// bare, and its middle operand is an expression; every binary operator
    /// groups a conditional.
    #[test]
    fn a_conditional_is_grouped_by_where_it_stands() {
        let choice = CExpr::conditional(at("g"), at("a"), at("b"));
        assert_eq!(
            CExpr::conditional(choice.clone(), at("1"), at("0")).to_string(),
            "(g ? a : b) ? 1 : 0"
        );
        assert_eq!(
            CExpr::conditional(at("c"), choice.clone(), choice.clone()).to_string(),
            "c ? g ? a : b : g ? a : b"
        );
        assert_eq!(
            op(CBinOp::Add, at("x"), choice.clone()).to_string(),
            "x + (g ? a : b)"
        );
        assert_eq!(
            CExpr::path_call("f", vec![choice.clone()]).to_string(),
            "f(g ? a : b)"
        );
        assert_eq!(choice.as_operand(), "(g ? a : b)");
    }

    /// The operands GCC and Clang flag under `-Wparentheses` are grouped on
    /// either side, and the ones they do not flag stand as precedence leaves
    /// them.
    #[test]
    fn an_operand_the_build_flags_is_grouped() {
        let (a, b, c) = (at("a"), at("b"), at("c"));
        let both = op(CBinOp::And, a.clone(), b.clone());
        assert_eq!(
            op(CBinOp::Or, c.clone(), both.clone()).to_string(),
            "c || (a && b)"
        );
        assert_eq!(
            op(CBinOp::Or, both.clone(), c.clone()).to_string(),
            "(a && b) || c"
        );
        let flagged = [
            (CBinOp::BitAnd, CBinOp::Add, "(a + b) & c"),
            (CBinOp::BitAnd, CBinOp::Lt, "(a < b) & c"),
            (CBinOp::BitXor, CBinOp::Sub, "(a - b) ^ c"),
            (CBinOp::BitXor, CBinOp::BitAnd, "(a & b) ^ c"),
            (CBinOp::BitOr, CBinOp::BitAnd, "(a & b) | c"),
            (CBinOp::BitOr, CBinOp::BitXor, "(a ^ b) | c"),
            (CBinOp::BitOr, CBinOp::Eq, "(a == b) | c"),
            (CBinOp::Shl, CBinOp::Add, "(a + b) << c"),
            (CBinOp::Shr, CBinOp::Sub, "(a - b) >> c"),
            (CBinOp::Eq, CBinOp::Eq, "(a == b) == c"),
            (CBinOp::Lt, CBinOp::Ne, "(a != b) < c"),
        ];
        for (parent, child, want) in flagged {
            let inner = op(child, a.clone(), b.clone());
            assert_eq!(op(parent, inner, c.clone()).to_string(), want);
        }
        assert_eq!(
            op(
                CBinOp::Shl,
                c.clone(),
                op(CBinOp::Add, a.clone(), b.clone())
            )
            .to_string(),
            "c << (a + b)"
        );
        assert_eq!(
            op(CBinOp::Lt, c.clone(), op(CBinOp::Eq, a.clone(), b.clone())).to_string(),
            "c < (a == b)"
        );
        assert_eq!(
            op(
                CBinOp::BitAnd,
                op(CBinOp::Mul, a.clone(), b.clone()),
                c.clone()
            )
            .to_string(),
            "a * b & c"
        );
        assert_eq!(
            op(
                CBinOp::BitOr,
                op(CBinOp::BitOr, a.clone(), b.clone()),
                c.clone()
            )
            .to_string(),
            "a | b | c"
        );
        assert_eq!(
            op(CBinOp::And, op(CBinOp::Eq, a.clone(), b.clone()), c.clone()).to_string(),
            "a == b && c"
        );
        let negated = a.clone().prefixed(CUnOp::Not);
        assert_eq!(
            op(CBinOp::Eq, negated.clone(), b.clone()).to_string(),
            "(!a) == b"
        );
        assert_eq!(
            op(CBinOp::BitAnd, negated.clone(), b.clone()).to_string(),
            "(!a) & b"
        );
        assert_eq!(
            op(CBinOp::Eq, b.clone(), negated.clone()).to_string(),
            "b == !a"
        );
        assert_eq!(op(CBinOp::And, negated, b).to_string(), "!a && b");
    }

    /// A prefix `-`, `+` or `&` before an operand that begins with the same
    /// character groups it, which C would otherwise read as one longer token.
    #[test]
    fn a_prefix_operator_never_glues_to_the_operand_after_it() {
        let a = at("a");
        assert_eq!(
            a.clone()
                .prefixed(CUnOp::Neg)
                .prefixed(CUnOp::Neg)
                .to_string(),
            "-(-a)"
        );
        assert_eq!(
            a.clone()
                .prefixed(CUnOp::Plus)
                .prefixed(CUnOp::Plus)
                .to_string(),
            "+(+a)"
        );
        assert_eq!(
            a.clone()
                .prefixed(CUnOp::Address)
                .prefixed(CUnOp::Address)
                .to_string(),
            "&(&a)"
        );
        assert_eq!(
            a.clone()
                .prefixed(CUnOp::Plus)
                .prefixed(CUnOp::Neg)
                .to_string(),
            "-+a"
        );
        assert_eq!(
            a.clone()
                .prefixed(CUnOp::Not)
                .prefixed(CUnOp::Not)
                .to_string(),
            "!!a"
        );
        assert_eq!(
            op(CBinOp::Sub, at("b"), a.clone().prefixed(CUnOp::Neg)).to_string(),
            "b - -a"
        );
        assert_eq!(
            a.clone()
                .prefixed(CUnOp::Neg)
                .cast("int8_t")
                .prefixed(CUnOp::Neg)
                .to_string(),
            "-(int8_t)-a"
        );
    }

    /// A compound literal is a postfix expression whose initializers stand bare,
    /// and `sizeof` is written over a parenthesized operand.
    #[test]
    fn a_compound_literal_and_sizeof_print_their_operands_bare() {
        let view = CExpr::compound(
            "sce_forge_bytes_view_t",
            vec![
                CInit::Value(at("p").cast("const uint8_t *")),
                CInit::Value(op(CBinOp::Add, at("n"), at("1"))),
            ],
        );
        assert_eq!(
            view.to_string(),
            "(sce_forge_bytes_view_t){ (const uint8_t *)p, n + 1 }"
        );
        assert_eq!(view.clone().field("len").to_string(), format!("{view}.len"));
        assert_eq!(
            CExpr::compound(
                "t",
                vec![CInit::Member {
                    member: "len".to_string(),
                    value: at("0")
                }]
            )
            .to_string(),
            "(t){ .len = 0 }"
        );
        assert_eq!(CExpr::zeroed("t").to_string(), "(t){0}");
        let xs = at("xs");
        assert_eq!(
            op(
                CBinOp::Div,
                xs.clone().size_of(),
                xs.indexed(at("0")).size_of()
            )
            .to_string(),
            "sizeof(xs) / sizeof(xs[0])"
        );
    }

    /// A name and a member have a sibling a suffix names; any other value has
    /// none.
    #[test]
    fn only_a_name_or_a_member_has_a_sibling() {
        assert_eq!(at("raw").sibling("_len"), Some(at("raw_len")));
        assert_eq!(
            at("sm->data.raw").sibling("_len"),
            Some(at("sm->data.raw_len"))
        );
        assert_eq!(
            at("e").field("pattern").sibling("_len"),
            Some(at("e").field("pattern_len"))
        );
        assert_eq!(at("(*raw)").sibling("_len"), None);
        assert_eq!(CExpr::path_call("f", vec![at("x")]).sibling("_len"), None);
        assert_eq!(at("xs").indexed(at("i")).sibling("_len"), None);
    }

    // ── The printer read back by the grammar of clause 6.5 ──────────────

    /// The type names the drawn trees cast to and build compound literals of,
    /// which the reader takes for a type where a parenthesis opens.
    const TYPES: [&str; 5] = ["uint8_t", "int64_t", "double", "const uint8_t *", "view_t"];

    #[derive(Debug, Clone, PartialEq)]
    enum Token {
        Word(String),
        Punct(&'static str),
    }

    /// The punctuators C reads, the longest first, so a glued `--` or `&&` is
    /// read as the one token it is (C11 6.4.6).
    const PUNCTUATORS: [&str; 35] = [
        "->", "++", "--", "<<", ">>", "<=", ">=", "==", "!=", "&&", "||", "(", ")", "[", "]", "{",
        "}", ".", ",", "?", ":", ";", "!", "~", "-", "+", "*", "/", "%", "<", ">", "&", "^", "|",
        "=",
    ];

    fn tokens(text: &str) -> Vec<Token> {
        let mut out = Vec::new();
        let mut rest = text;
        while let Some(c) = rest.chars().next() {
            if c.is_whitespace() {
                rest = &rest[c.len_utf8()..];
            } else if c.is_ascii_alphanumeric() || c == '_' || c == '"' {
                let end = rest
                    .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '"'))
                    .unwrap_or(rest.len());
                out.push(Token::Word(rest[..end].to_string()));
                rest = &rest[end..];
            } else {
                let punct = PUNCTUATORS
                    .iter()
                    .find(|p| rest.starts_with(**p))
                    .copied()
                    .unwrap_or_else(|| panic!("`{text}` holds a character C does not read"));
                out.push(Token::Punct(punct));
                rest = &rest[punct.len()..];
            }
        }
        out
    }

    /// A reader of the expressions the printer writes, one function per
    /// production of clause 6.5, which drops every parenthesis that only
    /// groups.
    struct Reader {
        tokens: Vec<Token>,
        at: usize,
        text: String,
    }

    /// The binary operators by the subclause that defines them, loosest first
    /// (6.5.14 down to 6.5.5).
    const LEVELS: [&[(&str, CBinOp)]; 10] = [
        &[("||", CBinOp::Or)],
        &[("&&", CBinOp::And)],
        &[("|", CBinOp::BitOr)],
        &[("^", CBinOp::BitXor)],
        &[("&", CBinOp::BitAnd)],
        &[("==", CBinOp::Eq), ("!=", CBinOp::Ne)],
        &[
            ("<", CBinOp::Lt),
            (">", CBinOp::Gt),
            ("<=", CBinOp::Le),
            (">=", CBinOp::Ge),
        ],
        &[("<<", CBinOp::Shl), (">>", CBinOp::Shr)],
        &[("+", CBinOp::Add), ("-", CBinOp::Sub)],
        &[("*", CBinOp::Mul), ("/", CBinOp::Div), ("%", CBinOp::Rem)],
    ];

    impl Reader {
        fn read(text: &str) -> CExpr {
            let mut reader = Reader {
                tokens: tokens(text),
                at: 0,
                text: text.to_string(),
            };
            let tree = reader.expression();
            assert_eq!(
                reader.at,
                reader.tokens.len(),
                "`{text}` has text after its end"
            );
            tree
        }

        fn peek(&self) -> Option<&Token> {
            self.tokens.get(self.at)
        }

        fn eat(&mut self, punct: &'static str) -> bool {
            if self.peek() == Some(&Token::Punct(punct)) {
                self.at += 1;
                true
            } else {
                false
            }
        }

        fn expect(&mut self, punct: &'static str) {
            assert!(
                self.eat(punct),
                "`{}`: no `{punct}` at token {}",
                self.text,
                self.at
            );
        }

        fn word(&mut self) -> String {
            match self.tokens.get(self.at) {
                Some(Token::Word(word)) => {
                    self.at += 1;
                    word.clone()
                }
                other => panic!("`{}`: a name expected, {other:?} found", self.text),
            }
        }

        /// 6.5.17, without the comma operator, which the printer never writes.
        fn expression(&mut self) -> CExpr {
            self.conditional()
        }

        /// 6.5.15.
        fn conditional(&mut self) -> CExpr {
            let condition = self.binary(0);
            if !self.eat("?") {
                return condition;
            }
            let then = self.expression();
            self.expect(":");
            let otherwise = self.conditional();
            CExpr::conditional(condition, then, otherwise)
        }

        /// 6.5.5 to 6.5.14.
        fn binary(&mut self, level: usize) -> CExpr {
            let Some(operators) = LEVELS.get(level) else {
                return self.cast();
            };
            let mut left = self.binary(level + 1);
            'operators: loop {
                for (token, op) in operators.iter() {
                    if self.eat(token) {
                        let right = self.binary(level + 1);
                        left = CExpr::binary(*op, left, right);
                        continue 'operators;
                    }
                }
                return left;
            }
        }

        /// Whether a type name follows the parenthesis at the reader's place.
        fn type_follows(&self) -> bool {
            self.peek() == Some(&Token::Punct("("))
                && matches!(self.tokens.get(self.at + 1), Some(Token::Word(w))
                    if TYPES.iter().any(|ty| ty.split(' ').next() == Some(w.as_str())))
        }

        /// A parenthesized type name, its words and `*`s joined by spaces.
        fn type_name(&mut self) -> String {
            self.expect("(");
            let mut parts = Vec::new();
            while !self.eat(")") {
                if self.eat("*") {
                    parts.push("*".to_string());
                } else {
                    parts.push(self.word());
                }
            }
            parts.join(" ")
        }

        /// 6.5.4, and the compound literal of 6.5.2.5 its parenthesis can open.
        fn cast(&mut self) -> CExpr {
            if self.type_follows() {
                let ty = self.type_name();
                if self.peek() == Some(&Token::Punct("{")) {
                    let literal = self.compound(ty);
                    return self.postfix_from(literal);
                }
                return self.cast().cast(ty);
            }
            self.unary()
        }

        /// 6.5.3.
        fn unary(&mut self) -> CExpr {
            if self.peek() == Some(&Token::Word("sizeof".to_string())) {
                self.at += 1;
                return self.unary().size_of();
            }
            for (token, op) in [
                ("-", CUnOp::Neg),
                ("+", CUnOp::Plus),
                ("!", CUnOp::Not),
                ("~", CUnOp::BitNot),
                ("&", CUnOp::Address),
            ] {
                if self.eat(token) {
                    return self.cast().prefixed(op);
                }
            }
            let primary = self.primary();
            self.postfix_from(primary)
        }

        /// The braces of a compound literal of `ty`.
        fn compound(&mut self, ty: String) -> CExpr {
            self.expect("{");
            if self.peek() == Some(&Token::Word("0".to_string()))
                && self.tokens.get(self.at + 1) == Some(&Token::Punct("}"))
            {
                self.at += 2;
                return CExpr::zeroed(ty);
            }
            let mut initializers = Vec::new();
            loop {
                if self.eat(".") {
                    let member = self.word();
                    self.expect("=");
                    initializers.push(CInit::Member {
                        member,
                        value: self.conditional(),
                    });
                } else {
                    initializers.push(CInit::Value(self.conditional()));
                }
                if !self.eat(",") {
                    break;
                }
            }
            self.expect("}");
            CExpr::compound(ty, initializers)
        }

        /// 6.5.2.
        fn postfix_from(&mut self, mut value: CExpr) -> CExpr {
            loop {
                if self.eat("(") {
                    let mut args = Vec::new();
                    if !self.eat(")") {
                        loop {
                            args.push(self.conditional());
                            if !self.eat(",") {
                                break;
                            }
                        }
                        self.expect(")");
                    }
                    value = CExpr::call(value, args);
                } else if self.eat("[") {
                    let index = self.expression();
                    self.expect("]");
                    value = value.indexed(index);
                } else if self.eat(".") {
                    let member = self.word();
                    value = value.field(member);
                } else {
                    return value;
                }
            }
        }

        /// 6.5.1.
        fn primary(&mut self) -> CExpr {
            if self.eat("(") {
                let inner = self.expression();
                self.expect(")");
                return inner;
            }
            match self.tokens.get(self.at) {
                Some(Token::Word(word)) => {
                    let word = word.clone();
                    self.at += 1;
                    CExpr::atom(word)
                }
                other => panic!("`{}`: an operand expected, {other:?} found", self.text),
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

    const OPERATORS: [CBinOp; 18] = [
        CBinOp::Mul,
        CBinOp::Div,
        CBinOp::Rem,
        CBinOp::Add,
        CBinOp::Sub,
        CBinOp::Shl,
        CBinOp::Shr,
        CBinOp::Lt,
        CBinOp::Gt,
        CBinOp::Le,
        CBinOp::Ge,
        CBinOp::Eq,
        CBinOp::Ne,
        CBinOp::BitAnd,
        CBinOp::BitXor,
        CBinOp::BitOr,
        CBinOp::And,
        CBinOp::Or,
    ];

    const PREFIXES: [CUnOp; 5] = [
        CUnOp::Neg,
        CUnOp::Plus,
        CUnOp::Not,
        CUnOp::BitNot,
        CUnOp::Address,
    ];

    /// The seed every drawing starts from.
    const SEED: u64 = 0x9E37_79B9_7F4A_7C15;

    /// How many trees a drawing holds.
    const TREES: usize = 4000;

    /// A tree of every node kind, at most `depth` deep, a cast and a prefix
    /// operator among the operands of every other kind.
    fn tree(draw: &mut Draw, depth: u32) -> CExpr {
        if depth == 0 || draw.below(6) == 0 {
            return at(draw.pick(&["a", "b", "x", "2", "7u", "sce_failure_"]));
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
                CExpr::call(callee, args)
            }
            1 => tree(draw, d).field(draw.pick(&["f", "len", "data"])),
            2 => {
                let base = tree(draw, d);
                base.indexed(tree(draw, d))
            }
            3 => {
                let ty = draw.pick(&["view_t", "int64_t"]);
                if draw.below(4) == 0 {
                    return CExpr::zeroed(ty);
                }
                let count = 1 + draw.below(2);
                let initializers = (0..count)
                    .map(|_| {
                        if draw.below(2) == 0 {
                            CInit::Value(tree(draw, d))
                        } else {
                            let member = draw.pick(&["len", "present"]).to_string();
                            CInit::Member {
                                member,
                                value: tree(draw, d),
                            }
                        }
                    })
                    .collect();
                CExpr::compound(ty, initializers)
            }
            4 | 5 => {
                let prefix = PREFIXES[draw.below(PREFIXES.len())];
                tree(draw, d).prefixed(prefix)
            }
            6 => tree(draw, d).size_of(),
            7 | 8 => {
                let value = tree(draw, d);
                value.cast(draw.pick(&TYPES))
            }
            9..=11 => {
                let operator = OPERATORS[draw.below(OPERATORS.len())];
                let left = tree(draw, d);
                let right = tree(draw, d);
                CExpr::binary(operator, left, right)
            }
            _ => {
                let condition = tree(draw, d);
                let then = tree(draw, d);
                let otherwise = tree(draw, d);
                CExpr::conditional(condition, then, otherwise)
            }
        }
    }

    /// Every tree the printer is handed is read back as itself by a reader of
    /// the productions of clause 6.5: as an expression, and spliced as one
    /// operand where a postfix read, a prefix operator and a cast could each
    /// take it apart. Every parenthesis the grammar needs is printed, and none
    /// the printer leaves out changes what is read.
    #[test]
    fn what_the_printer_writes_reads_back_as_the_tree_it_was_handed() {
        let mut draw = Draw(SEED);
        for _ in 0..TREES {
            let tree = tree(&mut draw, 5);
            let text = tree.to_string();
            assert_eq!(Reader::read(&text), tree, "`{text}` read as an expression");
            let operand = tree.as_operand();
            let spliced = format!("{operand}.m * (uint8_t)-{operand}");
            assert_eq!(
                Reader::read(&spliced),
                op(
                    CBinOp::Mul,
                    tree.clone().field("m"),
                    tree.prefixed(CUnOp::Neg).cast("uint8_t")
                ),
                "`{operand}` stands as one operand"
            );
        }
    }

    /// The kind of a node, as its variant is named.
    fn kind(tree: &CExpr) -> &'static str {
        match tree {
            CExpr::Atom(_) => "Atom",
            CExpr::Call { .. } => "Call",
            CExpr::Field { .. } => "Field",
            CExpr::Index { .. } => "Index",
            CExpr::Compound { .. } => "Compound",
            CExpr::Unary { .. } => "Unary",
            CExpr::Sizeof(_) => "Sizeof",
            CExpr::Cast { .. } => "Cast",
            CExpr::Binary { .. } => "Binary",
            CExpr::Conditional { .. } => "Conditional",
        }
    }

    /// The nodes directly inside `tree`.
    fn children(tree: &CExpr) -> Vec<&CExpr> {
        match tree {
            CExpr::Atom(_) => Vec::new(),
            CExpr::Call { callee, args } => std::iter::once(&**callee).chain(args).collect(),
            CExpr::Field { base, .. } => vec![&**base],
            CExpr::Index { base, index } => vec![&**base, &**index],
            CExpr::Compound { initializers, .. } => initializers
                .iter()
                .flatten()
                .map(|initializer| match initializer {
                    CInit::Value(value) | CInit::Member { value, .. } => value,
                })
                .collect(),
            CExpr::Unary { operand, .. } => vec![&**operand],
            CExpr::Sizeof(value) | CExpr::Cast { value, .. } => vec![&**value],
            CExpr::Binary { left, right, .. } => vec![&**left, &**right],
            CExpr::Conditional {
                condition,
                then,
                otherwise,
            } => vec![&**condition, &**then, &**otherwise],
        }
    }

    /// Every kind of node and every place a cast can stand in a parent is drawn,
    /// a prefix operator under a prefix operator among them, so the read-back
    /// above reaches each rule of the printer.
    #[test]
    fn the_trees_drawn_hold_every_kind_and_a_cast_in_every_parent() {
        let mut seen = std::collections::BTreeSet::new();
        let mut draw = Draw(SEED);
        for _ in 0..TREES {
            let mut stack = vec![tree(&mut draw, 5)];
            while let Some(node) = stack.pop() {
                seen.insert(kind(&node).to_string());
                for child in children(&node) {
                    if matches!(child, CExpr::Cast { .. } | CExpr::Unary { .. }) {
                        seen.insert(format!("{} in {}", kind(child), kind(&node)));
                    }
                    stack.push(child.clone());
                }
            }
        }
        for kind in [
            "Atom",
            "Call",
            "Field",
            "Index",
            "Compound",
            "Unary",
            "Sizeof",
            "Cast",
            "Binary",
            "Conditional",
            "Unary in Unary",
        ] {
            assert!(seen.contains(kind), "no {kind} was drawn: {seen:?}");
        }
        for parent in [
            "Call",
            "Field",
            "Index",
            "Compound",
            "Unary",
            "Sizeof",
            "Cast",
            "Binary",
            "Conditional",
        ] {
            assert!(
                seen.contains(&format!("Cast in {parent}")),
                "no cast was drawn in a {parent}: {seen:?}"
            );
        }
    }

    // ── The printer compiled ───────────────────────────────────────

    /// One printed form of each rule of the printer, compiled with the flags the
    /// generated C is built with (`-std=c11 -Wall -Wextra -Werror`): a
    /// parenthesis the grammar needs and left out is a syntax error, and an
    /// operand the build flags and left bare is a `-Wparentheses` error.
    #[test]
    fn what_the_printer_writes_compiles_where_the_generated_c_is_built() {
        let Some(cc) = crate::toolchain::require_any_or_skip(
            &["gcc", "clang", "cc"],
            "compile the C expression printer's forms under -Wall -Wextra -Werror",
        ) else {
            return;
        };
        let (a, b, c) = (at("a"), at("b"), at("c"));
        let both = op(
            CBinOp::And,
            op(CBinOp::Ge, b.clone(), at("1")),
            op(CBinOp::Le, b.clone(), at("4")),
        );
        let checked = CExpr::conditional(
            CExpr::path_call("sce_in", vec![at("r").prefixed(CUnOp::Address), a.clone()]),
            at("arr").indexed(a.clone()),
            at("0"),
        );
        let forms = [
            op(CBinOp::Or, op(CBinOp::Eq, a.clone(), at("2")), both.clone()),
            op(CBinOp::Or, both, op(CBinOp::Eq, a.clone(), at("2"))),
            op(
                CBinOp::BitOr,
                op(CBinOp::BitAnd, a.clone(), b.clone()),
                c.clone(),
            ),
            op(
                CBinOp::BitAnd,
                op(CBinOp::Add, a.clone(), b.clone()),
                c.clone(),
            ),
            op(
                CBinOp::BitXor,
                op(CBinOp::Lt, a.clone(), b.clone()),
                c.clone(),
            ),
            op(
                CBinOp::Shl,
                a.clone(),
                op(CBinOp::Add, b.clone(), c.clone()),
            ),
            op(CBinOp::Eq, op(CBinOp::Eq, a.clone(), b.clone()), c.clone()),
            op(CBinOp::Eq, a.clone().prefixed(CUnOp::Not), b.clone()),
            op(CBinOp::BitAnd, a.clone().prefixed(CUnOp::Not), b.clone()),
            a.clone().prefixed(CUnOp::Neg).prefixed(CUnOp::Neg),
            a.clone().prefixed(CUnOp::Plus).prefixed(CUnOp::Plus),
            a.clone()
                .prefixed(CUnOp::Neg)
                .cast("int8_t")
                .prefixed(CUnOp::Neg),
            op(CBinOp::Shl, a.clone().cast("uint16_t"), at("8")).cast("int16_t"),
            a.clone().prefixed(CUnOp::BitNot).cast("uint8_t"),
            CExpr::conditional(
                CExpr::conditional(c.clone(), a.clone(), b.clone()),
                at("1"),
                at("0"),
            ),
            CExpr::conditional(
                c.clone(),
                CExpr::conditional(a.clone(), b.clone(), c.clone()),
                CExpr::conditional(b.clone(), a.clone(), c.clone()),
            ),
            op(CBinOp::Add, checked.clone(), at("1")),
            CExpr::path_call("sce_id", vec![checked]),
            op(
                CBinOp::Div,
                at("arr").size_of(),
                at("arr").indexed(at("0")).size_of(),
            )
            .cast("int"),
            at("v").field("len").cast("int64_t").cast("int"),
            CExpr::compound(
                "struct sce_view",
                vec![
                    CInit::Value(at("v").field("data")),
                    CInit::Member {
                        member: "len".to_string(),
                        value: at("v").field("len"),
                    },
                ],
            )
            .field("len")
            .cast("int"),
            CExpr::zeroed("struct sce_view").field("len").cast("int"),
            CExpr::path_call("sce_id", vec![op(CBinOp::Add, a.clone(), b.clone())]),
            op(CBinOp::Sub, a.clone(), b.clone().prefixed(CUnOp::Neg)),
            op(CBinOp::Mul, op(CBinOp::Add, a.clone(), b), c)
                .cast("double")
                .cast("int"),
        ];
        let body: String = forms
            .iter()
            .map(|form| format!("    r += {form};\n"))
            .collect();
        let source = format!(
            "#include <stdint.h>\n#include <stddef.h>\n\
             struct sce_view {{ const uint8_t *data; size_t len; }};\n\
             static int sce_id(int v) {{ return v; }}\n\
             static int sce_in(const int *r, int a) {{ return *r < a; }}\n\
             int sce_probe(int a, int b, int c, struct sce_view v);\n\
             int sce_probe(int a, int b, int c, struct sce_view v) {{\n    \
             int r = 0;\n    int arr[4] = {{0, 1, 2, 3}};\n{body}    return r;\n}}\n"
        );
        let dir = std::env::temp_dir().join(format!("sce-build-c-tree-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a directory for the probe");
        let path = dir.join("probe.c");
        std::fs::write(&path, &source).expect("write the probe");
        let output = std::process::Command::new(&cc)
            .args(["-std=c11", "-Wall", "-Wextra", "-Werror", "-fsyntax-only"])
            .arg(&path)
            .output()
            .expect("run the C compiler");
        let _ = std::fs::remove_dir_all(&dir);
        assert!(
            output.status.success(),
            "the printed forms do not compile:\n{source}\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
