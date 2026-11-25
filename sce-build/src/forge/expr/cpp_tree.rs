// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The C++ a forge expression is written as, as a tree of the C++ expressions
//! it is made of, and the one printer that writes such a tree as text.
//!
//! The C++ emitter ([`super::emit_cpp`]) builds this tree; nothing is read back
//! out of text. A conversion is a `static_cast`, a call of its own, wherever it
//! comes from: a narrow shift made on its bits, an unsigned operand where the
//! join is signed, a length met by a signed context.
//!
//! The printer decides every parenthesis, at each parent, from the kind of the
//! child and the precedence and associativity C++17 gives its operators: the
//! grammar of ISO/IEC 14882:2017 clause 8 (Expressions), from the primary
//! expressions of `[expr.prim]` and the postfix expressions of `[expr.post]` to the
//! conditional operator of `[expr.cond]`, each binary operator associating to the
//! left. That is the textbook unparsing rule (Ramsey, "Unparsing Expressions
//! with Prefix and Postfix Operators", 1998). The rules of the grammar that a
//! precedence table does not say are the printer's too:
//!
//! * the condition of `?:` is a logical-or-expression and its last operand an
//!   assignment-expression (`[expr.cond]`): a conditional is grouped in the first
//!   and stands bare in the last, and the middle operand is an expression,
//!   which stands bare;
//! * a lambda is a primary expression (`[expr.prim.lambda]`) and its body's
//!   returned expression stands bare;
//! * two `[` in a row open an attribute and nothing else (`[dcl.attr.grammar]`),
//!   so an index that begins with a lambda's `[` is grouped;
//! * a prefix `-` or `+` before an operand that begins with the same character
//!   would be read as `--` or `++` (the longest token wins, `[lex.pptoken]`), so
//!   such an operand is grouped.
//!
//! The generated C++ is built with `-Wall -Wextra -Werror`, so the operands GCC and
//! Clang flag under `-Wparentheses` and `-Wlogical-not-parentheses` are grouped
//! where precedence alone would leave them bare (`CppBinOp::flags`), as the C
//! printer groups them ([`super::c_tree`]).
//!
//! Everything else stands bare. The tests hold the printer to the grammar by
//! reading what it prints back with a reader written from the productions of
//! clause 8 and comparing the tree read with the tree printed, and compile a
//! printed form of each rule with the flags the generated C++ is built with.

use std::fmt;

/// A C++ expression the emitter writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CppExpr {
    /// Text written as it is that stands as one operand: a literal, a name, a
    /// qualified or a templated name, or a fragment its writer made one
    /// ([`super::ExprKind::Raw`]).
    Atom(String),
    /// `callee(args)`, a `static_cast<T>(value)` among them.
    Call {
        callee: Box<CppExpr>,
        args: Vec<CppExpr>,
    },
    /// `base.member`.
    Field { base: Box<CppExpr>, member: String },
    /// `base[index]`.
    Index {
        base: Box<CppExpr>,
        index: Box<CppExpr>,
    },
    /// `[&] { return body; }`.
    Lambda { body: Box<CppExpr> },
    /// A prefix operator over its operand.
    Unary { op: CppUnOp, operand: Box<CppExpr> },
    /// `left op right`.
    Binary {
        op: CppBinOp,
        left: Box<CppExpr>,
        right: Box<CppExpr>,
    },
    /// `condition ? then : otherwise`.
    Conditional {
        condition: Box<CppExpr>,
        then: Box<CppExpr>,
        otherwise: Box<CppExpr>,
    },
}

/// The prefix operators the emitter writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CppUnOp {
    /// `-`
    Neg,
    /// `+`
    Plus,
    /// `!`
    Not,
    /// `~`
    BitNot,
    /// `*`, the value its operand points to or holds.
    Deref,
}

impl CppUnOp {
    fn token(self) -> &'static str {
        match self {
            Self::Neg => "-",
            Self::Plus => "+",
            Self::Not => "!",
            Self::BitNot => "~",
            Self::Deref => "*",
        }
    }

    /// The character an operand may not begin with bare after this operator,
    /// where the two would be read as one longer token: `--` or `++`.
    fn glues_to(self) -> Option<char> {
        match self {
            Self::Neg => Some('-'),
            Self::Plus => Some('+'),
            Self::Not | Self::BitNot | Self::Deref => None,
        }
    }
}

/// The binary operators the emitter writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CppBinOp {
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

impl CppBinOp {
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

    /// The subclause of clause 8 the operator is defined in, as a level.
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
    /// under `-Wparentheses`, which the generated C++ is built with as an
    /// error: a comparison under a comparison, `&&` under `||`, a sum or a
    /// comparison under a bitwise operator, `&` or `^` under `|` and `&` under
    /// `^`, and a sum under a shift. Precedence reads every one of them as the
    /// document means it; the build does not take it.
    fn flags(self, child: CppBinOp) -> bool {
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

/// How tightly an expression binds, loosest first: the subclauses of clause 8
/// the emitter writes, and above them every expression no operator can split
/// (a name, a literal, a lambda).
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
    Postfix,
    Primary,
}

/// Where a child stands in its parent, which is all the printer needs to
/// decide whether it is grouped.
#[derive(Debug, Clone, Copy)]
enum Slot {
    /// Between delimiters of the parent's own (an argument, the body of a
    /// lambda, the middle of `?:`), or the whole expression.
    Enclosed,
    /// Between the brackets of an index, where a `[` may not come first.
    Subscript,
    /// What a call calls, or what a field read or an index reads.
    Postfix,
    /// The operand of a prefix operator.
    Prefix(CppUnOp),
    /// The left operand of a binary operator.
    Left(CppBinOp),
    /// The right operand of a binary operator.
    Right(CppBinOp),
    /// The condition of `?:`.
    Condition,
    /// The last operand of `?:`.
    Otherwise,
}

impl CppExpr {
    /// Text that stands as one operand, written as it is.
    pub(crate) fn atom(text: impl Into<String>) -> Self {
        Self::Atom(text.into())
    }

    /// A call of `callee` with `args`.
    pub(crate) fn call(callee: CppExpr, args: Vec<CppExpr>) -> Self {
        Self::Call {
            callee: Box::new(callee),
            args,
        }
    }

    /// A call of the function, the type or the cast the name `name` spells.
    pub(crate) fn path_call(name: impl Into<String>, args: Vec<CppExpr>) -> Self {
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
    pub(crate) fn indexed(self, index: CppExpr) -> Self {
        Self::Index {
            base: Box::new(self),
            index: Box::new(index),
        }
    }

    /// The lambda that returns `body`, every name it reads taken by reference.
    pub(crate) fn lambda(body: CppExpr) -> Self {
        Self::Lambda {
            body: Box::new(body),
        }
    }

    /// The prefix operator `op` over this value.
    pub(crate) fn prefixed(self, op: CppUnOp) -> Self {
        Self::Unary {
            op,
            operand: Box::new(self),
        }
    }

    /// This value converted to the type `ty`: `static_cast<ty>(value)`.
    pub(crate) fn static_cast(self, ty: impl AsRef<str>) -> Self {
        Self::path_call(format!("static_cast<{}>", ty.as_ref()), vec![self])
    }

    /// `left op right`.
    pub(crate) fn binary(op: CppBinOp, left: CppExpr, right: CppExpr) -> Self {
        Self::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    /// `condition ? then : otherwise`.
    pub(crate) fn conditional(condition: CppExpr, then: CppExpr, otherwise: CppExpr) -> Self {
        Self::Conditional {
            condition: Box::new(condition),
            then: Box::new(then),
            otherwise: Box::new(otherwise),
        }
    }

    fn precedence(&self) -> Precedence {
        match self {
            Self::Conditional { .. } => Precedence::Conditional,
            Self::Binary { op, .. } => op.precedence(),
            Self::Unary { .. } => Precedence::Unary,
            Self::Call { .. } | Self::Field { .. } | Self::Index { .. } => Precedence::Postfix,
            Self::Atom(_) | Self::Lambda { .. } => Precedence::Primary,
        }
    }

    /// Whether this expression is grouped where it stands at `slot`.
    fn needs_parentheses(&self, slot: Slot) -> bool {
        let own = self.precedence();
        match slot {
            Slot::Enclosed => false,
            Slot::Subscript => self.first_char() == Some('['),
            Slot::Postfix => own < Precedence::Postfix,
            Slot::Prefix(op) => {
                own < Precedence::Unary
                    || op.glues_to().is_some_and(|c| self.first_char() == Some(c))
            }
            Slot::Left(op) => {
                own < op.precedence()
                    || self.is_flagged_under(op)
                    || (op.flags_a_negated_left()
                        && matches!(
                            self,
                            Self::Unary {
                                op: CppUnOp::Not,
                                ..
                            }
                        ))
            }
            Slot::Right(op) => own <= op.precedence() || self.is_flagged_under(op),
            Slot::Condition => own < Precedence::Or,
            Slot::Otherwise => own < Precedence::Conditional,
        }
    }

    /// Whether this expression is a binary operation `parent` flags as its
    /// operand ([`CppBinOp::flags`]).
    fn is_flagged_under(&self, parent: CppBinOp) -> bool {
        matches!(self, Self::Binary { op, .. } if parent.flags(*op))
    }

    /// The first character this expression is printed with, bare.
    fn first_char(&self) -> Option<char> {
        match self {
            Self::Atom(text) => text.chars().next(),
            Self::Lambda { .. } => Some('['),
            Self::Unary { op, .. } => op.token().chars().next(),
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
                index.write(Slot::Subscript, out);
                out.push(']');
            }
            Self::Lambda { body } => {
                out.push_str("[&] { return ");
                body.write(Slot::Enclosed, out);
                out.push_str("; }");
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

/// The whole expression, as the emitter hands it on.
impl fmt::Display for CppExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = String::new();
        self.write(Slot::Enclosed, &mut out);
        f.write_str(&out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str) -> CppExpr {
        CppExpr::atom(text)
    }

    fn op(op: CppBinOp, left: CppExpr, right: CppExpr) -> CppExpr {
        CppExpr::binary(op, left, right)
    }

    /// The text `tree` is printed as where one operand is spliced, which a
    /// postfix read, a prefix operator and a binary operator around it could
    /// each take apart.
    fn operand(tree: &CppExpr) -> String {
        let mut out = String::new();
        tree.write(Slot::Postfix, &mut out);
        out
    }

    /// An operator groups an operand that binds more loosely than itself, and a
    /// right operand that binds as tightly, since every binary operator of C++
    /// associates to the left; a prefix operator groups an operand below a
    /// unary expression, and a postfix read one below a postfix expression.
    #[test]
    fn an_operand_is_grouped_by_precedence_and_left_association() {
        let (a, b, c) = (at("a"), at("b"), at("c"));
        let sum = op(CppBinOp::Add, a.clone(), b.clone());
        assert_eq!(
            op(CppBinOp::Mul, sum.clone(), c.clone()).to_string(),
            "(a + b) * c"
        );
        assert_eq!(
            op(
                CppBinOp::Sub,
                op(CppBinOp::Sub, a.clone(), b.clone()),
                c.clone()
            )
            .to_string(),
            "a - b - c"
        );
        assert_eq!(
            op(
                CppBinOp::Sub,
                a.clone(),
                op(CppBinOp::Sub, b.clone(), c.clone())
            )
            .to_string(),
            "a - (b - c)"
        );
        assert_eq!(sum.clone().prefixed(CppUnOp::Neg).to_string(), "-(a + b)");
        assert_eq!(
            sum.clone().static_cast("int").to_string(),
            "static_cast<int>(a + b)"
        );
        assert_eq!(
            CppExpr::call(sum.clone().field("size"), Vec::new()).to_string(),
            "(a + b).size()"
        );
        assert_eq!(
            CppExpr::call(a.clone().field("size"), Vec::new()).to_string(),
            "a.size()"
        );
        let proven = a.clone().prefixed(CppUnOp::Deref);
        assert_eq!(proven.to_string(), "*a");
        assert_eq!(proven.clone().field("count").to_string(), "(*a).count");
        assert_eq!(op(CppBinOp::Add, proven, c).to_string(), "*a + c");
        assert_eq!(
            a.prefixed(CppUnOp::BitNot)
                .static_cast("std::uint8_t")
                .to_string(),
            "static_cast<std::uint8_t>(~a)"
        );
    }

    /// The condition of `?:` groups a conditional, its last operand takes one
    /// bare, and its middle operand is an expression; every binary operator
    /// groups a conditional.
    #[test]
    fn a_conditional_is_grouped_by_where_it_stands() {
        let choice = CppExpr::conditional(at("g"), at("a"), at("b"));
        assert_eq!(
            CppExpr::conditional(choice.clone(), at("1"), at("0")).to_string(),
            "(g ? a : b) ? 1 : 0"
        );
        assert_eq!(
            CppExpr::conditional(at("c"), choice.clone(), choice.clone()).to_string(),
            "c ? g ? a : b : g ? a : b"
        );
        assert_eq!(
            op(CppBinOp::Add, at("x"), choice.clone()).to_string(),
            "x + (g ? a : b)"
        );
        assert_eq!(operand(&choice), "(g ? a : b)");
    }

    /// The operands GCC and Clang flag under `-Wparentheses` are grouped on
    /// either side, and a `!` before a comparison or a `&`.
    #[test]
    fn an_operand_the_build_flags_is_grouped() {
        let (a, b, c) = (at("a"), at("b"), at("c"));
        let both = op(CppBinOp::And, a.clone(), b.clone());
        assert_eq!(
            op(CppBinOp::Or, c.clone(), both.clone()).to_string(),
            "c || (a && b)"
        );
        assert_eq!(
            op(CppBinOp::Or, both, c.clone()).to_string(),
            "(a && b) || c"
        );
        for (parent, child, want) in [
            (CppBinOp::BitAnd, CppBinOp::Add, "(a + b) & c"),
            (CppBinOp::BitXor, CppBinOp::BitAnd, "(a & b) ^ c"),
            (CppBinOp::BitOr, CppBinOp::BitXor, "(a ^ b) | c"),
            (CppBinOp::BitOr, CppBinOp::Lt, "(a < b) | c"),
            (CppBinOp::Shr, CppBinOp::Sub, "(a - b) >> c"),
            (CppBinOp::Eq, CppBinOp::Eq, "(a == b) == c"),
        ] {
            let inner = op(child, a.clone(), b.clone());
            assert_eq!(op(parent, inner, c.clone()).to_string(), want);
        }
        assert_eq!(
            op(
                CppBinOp::BitOr,
                op(CppBinOp::Shl, a.clone(), b.clone()),
                c.clone()
            )
            .to_string(),
            "a << b | c"
        );
        let negated = a.prefixed(CppUnOp::Not);
        assert_eq!(
            op(CppBinOp::Ne, negated.clone(), b.clone()).to_string(),
            "(!a) != b"
        );
        assert_eq!(op(CppBinOp::And, negated, b).to_string(), "!a && b");
    }

    /// A lambda is a primary expression whose body stands bare; as the first
    /// thing an index holds it is grouped, where `[[` would open an attribute.
    #[test]
    fn a_lambda_is_one_operand_and_never_follows_a_bracket() {
        let fallback = CppExpr::lambda(op(CppBinOp::Mul, at("h"), at("h")));
        assert_eq!(fallback.to_string(), "[&] { return h * h; }");
        assert_eq!(
            CppExpr::path_call("SCE::Forge::otherwise", vec![at("d"), fallback.clone()])
                .to_string(),
            "SCE::Forge::otherwise(d, [&] { return h * h; })"
        );
        let called = CppExpr::call(fallback, Vec::new());
        assert_eq!(called.to_string(), "[&] { return h * h; }()");
        assert_eq!(
            at("xs").indexed(called.clone()).to_string(),
            "xs[([&] { return h * h; }())]"
        );
        assert_eq!(
            at("xs")
                .indexed(op(CppBinOp::Add, at("i"), at("1")))
                .to_string(),
            "xs[i + 1]"
        );
    }

    /// A prefix `-` or `+` before an operand that begins with the same character
    /// groups it, which C++ would otherwise read as one longer token.
    #[test]
    fn a_prefix_operator_never_glues_to_the_operand_after_it() {
        let a = at("a");
        assert_eq!(
            a.clone()
                .prefixed(CppUnOp::Neg)
                .prefixed(CppUnOp::Neg)
                .to_string(),
            "-(-a)"
        );
        assert_eq!(
            a.clone()
                .prefixed(CppUnOp::Plus)
                .prefixed(CppUnOp::Plus)
                .to_string(),
            "+(+a)"
        );
        assert_eq!(
            a.clone()
                .prefixed(CppUnOp::Deref)
                .prefixed(CppUnOp::Deref)
                .to_string(),
            "**a"
        );
        assert_eq!(
            a.prefixed(CppUnOp::Neg)
                .static_cast("int")
                .prefixed(CppUnOp::Neg)
                .to_string(),
            "-static_cast<int>(-a)"
        );
    }

    // ── The printer read back by the grammar of clause 8 ─────────────

    #[derive(Debug, Clone, PartialEq)]
    enum Token {
        Word(String),
        Punct(&'static str),
    }

    /// The punctuators C++ reads, the longest first, so a glued `--` or `[[` is
    /// read as what it is (`[lex.pptoken]`).
    const PUNCTUATORS: [&str; 33] = [
        "::", "++", "--", "<<", ">>", "<=", ">=", "==", "!=", "&&", "||", "(", ")", "[", "]", "{",
        "}", ".", ",", "?", ":", ";", "!", "~", "-", "+", "*", "/", "%", "<", ">", "&", "^",
    ];

    fn tokens(text: &str) -> Vec<Token> {
        let mut out = Vec::new();
        let mut rest = text;
        while let Some(c) = rest.chars().next() {
            if c.is_whitespace() {
                rest = &rest[c.len_utf8()..];
            } else if c.is_ascii_alphanumeric() || c == '_' {
                let mut end = rest
                    .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == ':'))
                    .unwrap_or(rest.len());
                // A named cast is one name with its template argument.
                if rest[..end].ends_with("_cast") && rest[end..].starts_with('<') {
                    end += rest[end..].find('>').expect("a cast's type closes") + 1;
                }
                out.push(Token::Word(rest[..end].to_string()));
                rest = &rest[end..];
            } else if c == '|' {
                let punct = if rest.starts_with("||") { "||" } else { "|" };
                out.push(Token::Punct(punct));
                rest = &rest[punct.len()..];
            } else {
                let punct = PUNCTUATORS
                    .iter()
                    .find(|p| rest.starts_with(**p))
                    .copied()
                    .unwrap_or_else(|| panic!("`{text}` holds a character C++ does not read"));
                out.push(Token::Punct(punct));
                rest = &rest[punct.len()..];
            }
        }
        out
    }

    /// The binary operators by the subclause that defines them, loosest first
    /// (`[expr.log.or]` down to `[expr.mul]`).
    const LEVELS: [&[(&str, CppBinOp)]; 10] = [
        &[("||", CppBinOp::Or)],
        &[("&&", CppBinOp::And)],
        &[("|", CppBinOp::BitOr)],
        &[("^", CppBinOp::BitXor)],
        &[("&", CppBinOp::BitAnd)],
        &[("==", CppBinOp::Eq), ("!=", CppBinOp::Ne)],
        &[
            ("<", CppBinOp::Lt),
            (">", CppBinOp::Gt),
            ("<=", CppBinOp::Le),
            (">=", CppBinOp::Ge),
        ],
        &[("<<", CppBinOp::Shl), (">>", CppBinOp::Shr)],
        &[("+", CppBinOp::Add), ("-", CppBinOp::Sub)],
        &[
            ("*", CppBinOp::Mul),
            ("/", CppBinOp::Div),
            ("%", CppBinOp::Rem),
        ],
    ];

    /// A reader of the expressions the printer writes, one function per
    /// production of clause 8, which drops every parenthesis that only groups.
    struct Reader {
        tokens: Vec<Token>,
        at: usize,
        text: String,
    }

    impl Reader {
        fn read(text: &str) -> CppExpr {
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

        fn expect_word(&mut self, word: &str) {
            assert_eq!(
                self.peek(),
                Some(&Token::Word(word.to_string())),
                "`{}`: `{word}` expected at token {}",
                self.text,
                self.at
            );
            self.at += 1;
        }

        /// `[expr.comma]`, without the comma operator, which the printer never
        /// writes.
        fn expression(&mut self) -> CppExpr {
            self.conditional()
        }

        /// `[expr.cond]`.
        fn conditional(&mut self) -> CppExpr {
            let condition = self.binary(0);
            if !self.eat("?") {
                return condition;
            }
            let then = self.expression();
            self.expect(":");
            let otherwise = self.conditional();
            CppExpr::conditional(condition, then, otherwise)
        }

        /// `[expr.mul]` to `[expr.log.or]`.
        fn binary(&mut self, level: usize) -> CppExpr {
            let Some(operators) = LEVELS.get(level) else {
                return self.unary();
            };
            let mut left = self.binary(level + 1);
            'operators: loop {
                for (token, op) in operators.iter() {
                    if self.eat(token) {
                        let right = self.binary(level + 1);
                        left = CppExpr::binary(*op, left, right);
                        continue 'operators;
                    }
                }
                return left;
            }
        }

        /// `[expr.unary]`.
        fn unary(&mut self) -> CppExpr {
            for (token, op) in [
                ("-", CppUnOp::Neg),
                ("+", CppUnOp::Plus),
                ("!", CppUnOp::Not),
                ("~", CppUnOp::BitNot),
                ("*", CppUnOp::Deref),
            ] {
                if self.eat(token) {
                    return self.unary().prefixed(op);
                }
            }
            let primary = self.primary();
            self.postfix_from(primary)
        }

        /// `[expr.post]`.
        fn postfix_from(&mut self, mut value: CppExpr) -> CppExpr {
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
                    value = CppExpr::call(value, args);
                } else if self.eat("[") {
                    assert_ne!(
                        self.peek(),
                        Some(&Token::Punct("[")),
                        "`{}`: `[[` opens an attribute",
                        self.text
                    );
                    let index = self.expression();
                    self.expect("]");
                    value = value.indexed(index);
                } else if self.eat(".") {
                    match self.tokens.get(self.at) {
                        Some(Token::Word(member)) => {
                            value = value.field(member.clone());
                            self.at += 1;
                        }
                        other => panic!("`{}`: a member expected, {other:?} found", self.text),
                    }
                } else {
                    return value;
                }
            }
        }

        /// `[expr.prim]`, a lambda of `[expr.prim.lambda]` among them.
        fn primary(&mut self) -> CppExpr {
            if self.eat("(") {
                let inner = self.expression();
                self.expect(")");
                return inner;
            }
            if self.eat("[") {
                self.expect("&");
                self.expect("]");
                self.expect("{");
                self.expect_word("return");
                let body = self.expression();
                self.expect(";");
                self.expect("}");
                return CppExpr::lambda(body);
            }
            match self.tokens.get(self.at) {
                Some(Token::Word(word)) => {
                    let word = word.clone();
                    self.at += 1;
                    CppExpr::atom(word)
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

    const OPERATORS: [CppBinOp; 18] = [
        CppBinOp::Mul,
        CppBinOp::Div,
        CppBinOp::Rem,
        CppBinOp::Add,
        CppBinOp::Sub,
        CppBinOp::Shl,
        CppBinOp::Shr,
        CppBinOp::Lt,
        CppBinOp::Gt,
        CppBinOp::Le,
        CppBinOp::Ge,
        CppBinOp::Eq,
        CppBinOp::Ne,
        CppBinOp::BitAnd,
        CppBinOp::BitXor,
        CppBinOp::BitOr,
        CppBinOp::And,
        CppBinOp::Or,
    ];

    const PREFIXES: [CppUnOp; 5] = [
        CppUnOp::Neg,
        CppUnOp::Plus,
        CppUnOp::Not,
        CppUnOp::BitNot,
        CppUnOp::Deref,
    ];

    /// The seed every drawing starts from.
    const SEED: u64 = 0x9E37_79B9_7F4A_7C15;

    /// How many trees a drawing holds.
    const TREES: usize = 4000;

    /// A tree of every node kind, at most `depth` deep, a cast and a prefix
    /// operator among the operands of every other kind.
    fn tree(draw: &mut Draw, depth: u32) -> CppExpr {
        if depth == 0 || draw.below(6) == 0 {
            return at(draw.pick(&["a", "b", "x", "2", "7u", "SCE::Forge::v"]));
        }
        let d = depth - 1;
        match draw.below(13) {
            0 => {
                let callee = if draw.below(4) == 0 {
                    tree(draw, d)
                } else {
                    at("f")
                };
                let count = draw.below(3);
                let args = (0..count).map(|_| tree(draw, d)).collect();
                CppExpr::call(callee, args)
            }
            1 => tree(draw, d).field(draw.pick(&["f", "size"])),
            2 => {
                let base = tree(draw, d);
                base.indexed(tree(draw, d))
            }
            3 => CppExpr::lambda(tree(draw, d)),
            4 | 5 => {
                let prefix = PREFIXES[draw.below(PREFIXES.len())];
                tree(draw, d).prefixed(prefix)
            }
            6 | 7 => {
                let value = tree(draw, d);
                value.static_cast(draw.pick(&["std::uint8_t", "int", "double"]))
            }
            8..=10 => {
                let operator = OPERATORS[draw.below(OPERATORS.len())];
                let left = tree(draw, d);
                let right = tree(draw, d);
                CppExpr::binary(operator, left, right)
            }
            _ => {
                let condition = tree(draw, d);
                let then = tree(draw, d);
                let otherwise = tree(draw, d);
                CppExpr::conditional(condition, then, otherwise)
            }
        }
    }

    /// Every tree the printer is handed is read back as itself by a reader of
    /// the productions of clause 8: as an expression, and spliced as one operand
    /// where a postfix read, a prefix operator and a binary operator could each
    /// take it apart. Every parenthesis the grammar needs is printed, and none
    /// the printer leaves out changes what is read.
    #[test]
    fn what_the_printer_writes_reads_back_as_the_tree_it_was_handed() {
        let mut draw = Draw(SEED);
        for _ in 0..TREES {
            let tree = tree(&mut draw, 5);
            let text = tree.to_string();
            assert_eq!(Reader::read(&text), tree, "`{text}` read as an expression");
            let spliced = operand(&tree);
            assert_eq!(
                Reader::read(&format!("{spliced}.m * -{spliced}")),
                op(
                    CppBinOp::Mul,
                    tree.clone().field("m"),
                    tree.prefixed(CppUnOp::Neg)
                ),
                "`{spliced}` stands as one operand"
            );
        }
    }

    /// Whether `tree` is a `static_cast`.
    fn is_cast(tree: &CppExpr) -> bool {
        matches!(tree, CppExpr::Call { callee, .. }
            if matches!(&**callee, CppExpr::Atom(name) if name.starts_with("static_cast<")))
    }

    /// The kind of a node, as its variant is named, a `static_cast` its own.
    fn kind(tree: &CppExpr) -> &'static str {
        match tree {
            _ if is_cast(tree) => "Cast",
            CppExpr::Atom(_) => "Atom",
            CppExpr::Call { .. } => "Call",
            CppExpr::Field { .. } => "Field",
            CppExpr::Index { .. } => "Index",
            CppExpr::Lambda { .. } => "Lambda",
            CppExpr::Unary { .. } => "Unary",
            CppExpr::Binary { .. } => "Binary",
            CppExpr::Conditional { .. } => "Conditional",
        }
    }

    /// The nodes directly inside `tree`.
    fn children(tree: &CppExpr) -> Vec<&CppExpr> {
        match tree {
            CppExpr::Atom(_) => Vec::new(),
            CppExpr::Call { callee, args } => std::iter::once(&**callee).chain(args).collect(),
            CppExpr::Field { base, .. } => vec![&**base],
            CppExpr::Index { base, index } => vec![&**base, &**index],
            CppExpr::Lambda { body } => vec![&**body],
            CppExpr::Unary { operand, .. } => vec![&**operand],
            CppExpr::Binary { left, right, .. } => vec![&**left, &**right],
            CppExpr::Conditional {
                condition,
                then,
                otherwise,
            } => vec![&**condition, &**then, &**otherwise],
        }
    }

    /// Every kind of node and every place a prefix operator can stand in a
    /// parent is drawn, a lambda first in an index among them, so the read-back
    /// above reaches each rule of the printer.
    #[test]
    fn the_trees_drawn_hold_every_kind_and_a_prefix_in_every_parent() {
        let mut seen = std::collections::BTreeSet::new();
        let mut draw = Draw(SEED);
        for _ in 0..TREES {
            let mut stack = vec![tree(&mut draw, 5)];
            while let Some(node) = stack.pop() {
                seen.insert(kind(&node).to_string());
                for child in children(&node) {
                    if matches!(child, CppExpr::Unary { .. } | CppExpr::Lambda { .. })
                        || is_cast(child)
                    {
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
            "Lambda",
            "Unary",
            "Cast",
            "Binary",
            "Conditional",
            "Unary in Unary",
            "Lambda in Index",
        ] {
            assert!(seen.contains(kind), "no {kind} was drawn: {seen:?}");
        }
        for parent in [
            "Call",
            "Field",
            "Index",
            "Lambda",
            "Unary",
            "Cast",
            "Binary",
            "Conditional",
        ] {
            assert!(
                seen.contains(&format!("Unary in {parent}")),
                "no prefix operator was drawn in a {parent}: {seen:?}"
            );
        }
    }

    // ── The printer compiled ───────────────────────────────────────

    /// One printed form of each rule of the printer, compiled with the flags the
    /// generated C++ is built with (`-std=c++17 -Wall -Wextra -Werror`): a
    /// parenthesis the grammar needs and left out is a syntax error, and an
    /// operand the build flags and left bare is a `-Wparentheses` error.
    #[test]
    fn what_the_printer_writes_compiles_where_the_generated_cpp_is_built() {
        let Some(cxx) = crate::toolchain::require_any_or_skip(
            &["g++", "clang++"],
            "compile the C++ expression printer's forms under -Wall -Wextra -Werror",
        ) else {
            return;
        };
        let (a, b, c) = (at("a"), at("b"), at("c"));
        let both = op(
            CppBinOp::And,
            op(CppBinOp::Ge, b.clone(), at("1")),
            op(CppBinOp::Le, b.clone(), at("4")),
        );
        let held = at("ps").prefixed(CppUnOp::Deref);
        let forms = [
            op(
                CppBinOp::Or,
                op(CppBinOp::Eq, a.clone(), at("2")),
                both.clone(),
            ),
            op(CppBinOp::Or, both, op(CppBinOp::Eq, a.clone(), at("2"))),
            op(
                CppBinOp::BitOr,
                op(CppBinOp::BitAnd, a.clone(), b.clone()),
                c.clone(),
            ),
            op(
                CppBinOp::BitAnd,
                op(CppBinOp::Add, a.clone(), b.clone()),
                c.clone(),
            ),
            op(
                CppBinOp::BitXor,
                op(CppBinOp::Lt, a.clone(), b.clone()),
                c.clone(),
            ),
            op(
                CppBinOp::Shl,
                a.clone(),
                op(CppBinOp::Add, b.clone(), c.clone()),
            ),
            op(
                CppBinOp::Eq,
                op(CppBinOp::Eq, a.clone(), b.clone()),
                c.clone(),
            ),
            op(CppBinOp::Eq, a.clone().prefixed(CppUnOp::Not), b.clone()),
            op(
                CppBinOp::BitAnd,
                a.clone().prefixed(CppUnOp::Not),
                b.clone(),
            ),
            a.clone().prefixed(CppUnOp::Neg).prefixed(CppUnOp::Neg),
            a.clone().prefixed(CppUnOp::Plus).prefixed(CppUnOp::Plus),
            op(
                CppBinOp::Shl,
                a.clone().static_cast("std::uint16_t"),
                at("8"),
            )
            .static_cast("std::int16_t"),
            a.clone()
                .prefixed(CppUnOp::BitNot)
                .static_cast("std::uint8_t"),
            CppExpr::conditional(
                CppExpr::conditional(c.clone(), a.clone(), b.clone()),
                at("1"),
                at("0"),
            ),
            CppExpr::conditional(
                c.clone(),
                CppExpr::conditional(a.clone(), b.clone(), c.clone()),
                CppExpr::conditional(b.clone(), a.clone(), c.clone()),
            ),
            op(CppBinOp::Add, at("p").prefixed(CppUnOp::Deref), at("1")),
            CppExpr::call(held.clone().field("size"), Vec::new()),
            op(CppBinOp::Add, held.field("x"), at("1")),
            CppExpr::path_call(
                "sce_otherwise",
                vec![
                    a.clone(),
                    CppExpr::lambda(op(CppBinOp::Add, b.clone(), c.clone())),
                ],
            ),
            at("arr").indexed(CppExpr::call(CppExpr::lambda(at("0")), Vec::new())),
            CppExpr::call(at("s").field("size"), Vec::new()),
            op(
                CppBinOp::Add,
                CppExpr::conditional(c.clone(), a.clone(), b.clone()),
                at("1"),
            ),
            CppExpr::path_call("sce_id", vec![op(CppBinOp::Add, a.clone(), b.clone())]),
            op(CppBinOp::Sub, a, b.prefixed(CppUnOp::Neg)),
        ];
        let body: String = forms
            .iter()
            .map(|form| format!("    r += {form};\n"))
            .collect();
        let source = format!(
            "#include <cstdint>\n\
             struct SceS {{ int x; int size() const {{ return x; }} }};\n\
             template <typename F> static int sce_otherwise(int v, F f) {{ return v != 0 ? v : f(); }}\n\
             static int sce_id(int v) {{ return v; }}\n\
             int sce_probe(int a, int b, int c, const int *p, SceS s, const SceS *ps);\n\
             int sce_probe(int a, int b, int c, const int *p, SceS s, const SceS *ps) {{\n    \
             int r = 0;\n    int arr[4] = {{0, 1, 2, 3}};\n{body}    return r;\n}}\n"
        );
        let dir = std::env::temp_dir().join(format!("sce-build-cpp-tree-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a directory for the probe");
        let path = dir.join("probe.cpp");
        std::fs::write(&path, &source).expect("write the probe");
        let output = std::process::Command::new(&cxx)
            .args(["-std=c++17", "-Wall", "-Wextra", "-Werror", "-fsyntax-only"])
            .arg(&path)
            .output()
            .expect("run the C++ compiler");
        let _ = std::fs::remove_dir_all(&dir);
        assert!(
            output.status.success(),
            "the printed forms do not compile:\n{source}\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
