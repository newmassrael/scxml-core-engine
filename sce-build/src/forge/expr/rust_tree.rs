// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Rust a forge expression is written as, as a tree of the Rust expressions
//! it is made of, and the one printer that writes such a tree as text.
//!
//! The Rust emitter ([`super::emit_rust`]) and the Rust spellings it calls
//! ([`crate::forge::rust_optional`], the union reads of
//! [`crate::forge::composed_lowering`]) build this tree; nothing is read back
//! out of text. A cast is a node of its own wherever it comes from: a coercion,
//! the widening of a length or a rounding, a shift made on its bits.
//!
//! The printer decides every parenthesis, at each parent, from the kind of the
//! child and the precedence and associativity the Rust Reference gives its
//! operators ("Expressions", "Expression precedence"): the textbook unparsing
//! rule (Ramsey, "Unparsing Expressions with Prefix and Postfix Operators",
//! 1998). The rules of Rust's grammar that precedence does not see are the
//! printer's too:
//!
//! * a comparison does not associate: one operand of another is grouped on
//!   either side;
//! * a type is never followed by `<` or `<<`: rustc reads either as the start
//!   of the type's generic arguments, so `a as u32 < b` does not parse. A left
//!   operand of `<` or `<<` that would end in a cast's type is grouped;
//! * an expression with a block (`if`, `match`, a block) that begins a
//!   statement ends it, so `if c { 1 } else { 2 } + 3` is a statement and a
//!   stray `+ 3` where a statement may begin, which a block's tail and a match
//!   arm are. Such an expression is grouped wherever it is the first operand of
//!   an operator, the value of a cast, or what a postfix reads; elsewhere,
//!   after an operator, it stands bare;
//! * `a.f(x)` calls the method `f`, so a field read that is called is grouped.
//!
//! Everything else stands bare. The tests hold the printer to the grammar by
//! reading what it prints back with `syn`, as an expression and as the tail of
//! a block, and comparing the tree read with the tree printed.

use std::fmt;

/// A Rust expression the emitter writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RustExpr {
    /// Text written as it is that stands as one operand: a literal, a path, a
    /// name, or a fragment its writer made one ([`super::ExprKind::Raw`]).
    Atom(String),
    /// `..`, the range of everything, which an index reads a whole slice by.
    FullRange,
    /// `callee(args)`.
    Call {
        callee: Box<RustExpr>,
        args: Vec<RustExpr>,
    },
    /// `name!(args)`.
    Macro { name: String, args: Vec<RustExpr> },
    /// `receiver.method(args)`; `method` carries its turbofish, if any.
    MethodCall {
        receiver: Box<RustExpr>,
        method: String,
        args: Vec<RustExpr>,
    },
    /// `base.member`.
    Field { base: Box<RustExpr>, member: String },
    /// `base[index]`.
    Index {
        base: Box<RustExpr>,
        index: Box<RustExpr>,
    },
    /// `value?`.
    Try(Box<RustExpr>),
    /// A prefix operator over its operand.
    Unary {
        op: RustUnOp,
        operand: Box<RustExpr>,
    },
    /// `value as ty`.
    Cast { value: Box<RustExpr>, ty: String },
    /// `left op right`.
    Binary {
        op: RustBinOp,
        left: Box<RustExpr>,
        right: Box<RustExpr>,
    },
    /// `if condition { then } else { otherwise }`.
    If {
        condition: Box<RustExpr>,
        then: Box<RustExpr>,
        otherwise: Box<RustExpr>,
    },
    /// `match scrutinee { arms }`, the last arm followed by a comma where
    /// `trailing_comma` says so.
    Match {
        scrutinee: Box<RustExpr>,
        arms: Vec<RustArm>,
        trailing_comma: bool,
    },
    /// `{ statements tail }`.
    Block {
        statements: Vec<RustStmt>,
        tail: Box<RustExpr>,
    },
}

/// One arm of a [`RustExpr::Match`]: `pattern => body`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustArm {
    pub(crate) pattern: String,
    pub(crate) body: RustExpr,
}

/// One statement of a [`RustExpr::Block`] before its tail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RustStmt {
    /// `let pattern = init;`
    Let { pattern: String, init: RustExpr },
    /// `value;`
    Expr(RustExpr),
}

/// The prefix operators the emitter writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RustUnOp {
    /// `-`
    Neg,
    /// `!`, the logical and the bitwise complement alike.
    Not,
    /// `&`
    Ref,
    /// `&mut`
    RefMut,
}

impl RustUnOp {
    fn token(self) -> &'static str {
        match self {
            Self::Neg => "-",
            Self::Not => "!",
            Self::Ref => "&",
            Self::RefMut => "&mut ",
        }
    }
}

/// The binary operators the emitter writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RustBinOp {
    Mul,
    Div,
    Rem,
    Add,
    Sub,
    Shl,
    Shr,
    BitAnd,
    BitXor,
    BitOr,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
}

impl RustBinOp {
    fn token(self) -> &'static str {
        match self {
            Self::Mul => "*",
            Self::Div => "/",
            Self::Rem => "%",
            Self::Add => "+",
            Self::Sub => "-",
            Self::Shl => "<<",
            Self::Shr => ">>",
            Self::BitAnd => "&",
            Self::BitXor => "^",
            Self::BitOr => "|",
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

    /// The row of the Rust Reference's table the operator is on.
    fn precedence(self) -> Precedence {
        match self {
            Self::Mul | Self::Div | Self::Rem => Precedence::Product,
            Self::Add | Self::Sub => Precedence::Sum,
            Self::Shl | Self::Shr => Precedence::Shift,
            Self::BitAnd => Precedence::BitAnd,
            Self::BitXor => Precedence::BitXor,
            Self::BitOr => Precedence::BitOr,
            Self::Eq | Self::Ne | Self::Lt | Self::Gt | Self::Le | Self::Ge => Precedence::Compare,
            Self::And => Precedence::And,
            Self::Or => Precedence::Or,
        }
    }

    /// Whether the operator, written after a type, is read as the start of the
    /// type's generic arguments: `<` and `<<`, and not `<=`.
    fn opens_generic_arguments(self) -> bool {
        matches!(self, Self::Lt | Self::Shl)
    }
}

/// How tightly an expression binds, loosest first: the rows of the Rust
/// Reference's precedence table the emitter writes, and above them every
/// expression no operator can split (a literal, a path, an expression with a
/// block).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Precedence {
    Range,
    Or,
    And,
    Compare,
    BitOr,
    BitXor,
    BitAnd,
    Shift,
    Sum,
    Product,
    Cast,
    Prefix,
    Postfix,
    Primary,
}

/// Where a child stands in its parent, which is all the printer needs to
/// decide whether it is grouped.
#[derive(Debug, Clone, Copy)]
enum Slot {
    /// Between delimiters of the parent's own (an argument, an index, a
    /// condition, an arm, a statement), or the whole expression.
    Enclosed,
    /// What a call calls.
    Callee,
    /// What a method call, a field read, an index or a `?` reads.
    Postfix,
    /// The operand of a prefix operator.
    Prefix,
    /// The value a cast converts.
    Cast,
    /// The left operand of a binary operator.
    Left(RustBinOp),
    /// The right operand of a binary operator.
    Right(RustBinOp),
}

impl RustExpr {
    /// Text that stands as one operand, written as it is.
    pub(crate) fn atom(text: impl Into<String>) -> Self {
        Self::Atom(text.into())
    }

    /// A call of `callee` with `args`. A field read called is a method call,
    /// as `a.f(x)` reads in Rust.
    pub(crate) fn call(callee: RustExpr, args: Vec<RustExpr>) -> Self {
        match callee {
            Self::Field { base, member } => Self::MethodCall {
                receiver: base,
                method: member,
                args,
            },
            callee => Self::Call {
                callee: Box::new(callee),
                args,
            },
        }
    }

    /// A call of the function the path `path` names.
    pub(crate) fn path_call(path: impl Into<String>, args: Vec<RustExpr>) -> Self {
        Self::call(Self::atom(path), args)
    }

    /// The macro `name` invoked with `args`.
    pub(crate) fn macro_call(name: impl Into<String>, args: Vec<RustExpr>) -> Self {
        Self::Macro {
            name: name.into(),
            args,
        }
    }

    /// The method `method` of this value called with `args`.
    pub(crate) fn method(self, method: impl Into<String>, args: Vec<RustExpr>) -> Self {
        Self::MethodCall {
            receiver: Box::new(self),
            method: method.into(),
            args,
        }
    }

    /// The field `member` of this value.
    pub(crate) fn field(self, member: impl Into<String>) -> Self {
        Self::Field {
            base: Box::new(self),
            member: member.into(),
        }
    }

    /// This value indexed at `index`.
    pub(crate) fn indexed(self, index: RustExpr) -> Self {
        Self::Index {
            base: Box::new(self),
            index: Box::new(index),
        }
    }

    /// This value with its failure handed on by `?`.
    pub(crate) fn tried(self) -> Self {
        Self::Try(Box::new(self))
    }

    /// This value converted by `as` to the type `ty`.
    pub(crate) fn cast(self, ty: impl Into<String>) -> Self {
        Self::Cast {
            value: Box::new(self),
            ty: ty.into(),
        }
    }

    /// The prefix operator `op` over this value.
    pub(crate) fn prefixed(self, op: RustUnOp) -> Self {
        Self::Unary {
            op,
            operand: Box::new(self),
        }
    }

    /// `left op right`.
    pub(crate) fn binary(op: RustBinOp, left: RustExpr, right: RustExpr) -> Self {
        Self::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    /// `if condition { then } else { otherwise }`.
    pub(crate) fn if_else(condition: RustExpr, then: RustExpr, otherwise: RustExpr) -> Self {
        Self::If {
            condition: Box::new(condition),
            then: Box::new(then),
            otherwise: Box::new(otherwise),
        }
    }

    /// `match scrutinee { arms }`.
    pub(crate) fn match_of(scrutinee: RustExpr, arms: Vec<RustArm>, trailing_comma: bool) -> Self {
        Self::Match {
            scrutinee: Box::new(scrutinee),
            arms,
            trailing_comma,
        }
    }

    /// `{ statements tail }`.
    pub(crate) fn block(statements: Vec<RustStmt>, tail: RustExpr) -> Self {
        Self::Block {
            statements,
            tail: Box::new(tail),
        }
    }

    /// The whole of this value borrowed as a slice: `&value[..]`.
    pub(crate) fn borrowed_whole(self) -> Self {
        self.indexed(Self::FullRange).prefixed(RustUnOp::Ref)
    }

    /// This expression printed to stand where a fragment is spliced as it is
    /// ([`super::ExprKind::Raw`]): grouped unless it is one operand that no
    /// operator, cast or postfix read around it can take apart.
    pub(crate) fn as_operand(&self) -> String {
        let mut out = String::new();
        self.write(Slot::Postfix, &mut out);
        out
    }

    fn precedence(&self) -> Precedence {
        match self {
            Self::FullRange => Precedence::Range,
            Self::Binary { op, .. } => op.precedence(),
            Self::Cast { .. } => Precedence::Cast,
            Self::Unary { .. } => Precedence::Prefix,
            Self::Call { .. }
            | Self::Macro { .. }
            | Self::MethodCall { .. }
            | Self::Field { .. }
            | Self::Index { .. }
            | Self::Try(_) => Precedence::Postfix,
            Self::Atom(_) | Self::If { .. } | Self::Match { .. } | Self::Block { .. } => {
                Precedence::Primary
            }
        }
    }

    /// An expression with a block, which ends a statement it begins.
    fn is_block_like(&self) -> bool {
        matches!(
            self,
            Self::If { .. } | Self::Match { .. } | Self::Block { .. }
        )
    }

    /// Whether this expression is grouped where it stands at `slot`.
    fn needs_parentheses(&self, slot: Slot) -> bool {
        let own = self.precedence();
        match slot {
            Slot::Enclosed => false,
            Slot::Callee => {
                own < Precedence::Postfix
                    || self.is_block_like()
                    || matches!(self, Self::Field { .. })
            }
            Slot::Postfix => own < Precedence::Postfix || self.is_block_like(),
            Slot::Cast => own < Precedence::Cast || self.is_block_like(),
            Slot::Prefix => own < Precedence::Prefix,
            Slot::Left(op) => {
                own < op.precedence()
                    || (own == Precedence::Compare && op.precedence() == Precedence::Compare)
                    || self.is_block_like()
                    || (op.opens_generic_arguments() && self.ends_in_a_type())
            }
            Slot::Right(op) => own <= op.precedence(),
        }
    }

    /// Whether the last token this expression is printed with, bare, is a
    /// cast's type.
    fn ends_in_a_type(&self) -> bool {
        match self {
            Self::Cast { .. } => true,
            Self::Binary { op, right, .. } => {
                !right.needs_parentheses(Slot::Right(*op)) && right.ends_in_a_type()
            }
            Self::Unary { operand, .. } => {
                !operand.needs_parentheses(Slot::Prefix) && operand.ends_in_a_type()
            }
            _ => false,
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
            Self::FullRange => out.push_str(".."),
            Self::Call { callee, args } => {
                callee.write(Slot::Callee, out);
                write_arguments(args, out);
            }
            Self::Macro { name, args } => {
                out.push_str(name);
                out.push('!');
                write_arguments(args, out);
            }
            Self::MethodCall {
                receiver,
                method,
                args,
            } => {
                receiver.write(Slot::Postfix, out);
                out.push('.');
                out.push_str(method);
                write_arguments(args, out);
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
            Self::Try(value) => {
                value.write(Slot::Postfix, out);
                out.push('?');
            }
            Self::Unary { op, operand } => {
                out.push_str(op.token());
                operand.write(Slot::Prefix, out);
            }
            Self::Cast { value, ty } => {
                value.write(Slot::Cast, out);
                out.push_str(" as ");
                out.push_str(ty);
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
                out.push_str("if ");
                condition.write(Slot::Enclosed, out);
                out.push_str(" { ");
                then.write(Slot::Enclosed, out);
                out.push_str(" } else { ");
                otherwise.write(Slot::Enclosed, out);
                out.push_str(" }");
            }
            Self::Match {
                scrutinee,
                arms,
                trailing_comma,
            } => {
                out.push_str("match ");
                scrutinee.write(Slot::Enclosed, out);
                out.push_str(" {");
                for (i, arm) in arms.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    out.push(' ');
                    out.push_str(&arm.pattern);
                    out.push_str(" => ");
                    arm.body.write(Slot::Enclosed, out);
                }
                if *trailing_comma {
                    out.push(',');
                }
                out.push_str(" }");
            }
            Self::Block { statements, tail } => {
                out.push_str("{ ");
                for statement in statements {
                    match statement {
                        RustStmt::Let { pattern, init } => {
                            out.push_str("let ");
                            out.push_str(pattern);
                            out.push_str(" = ");
                            init.write(Slot::Enclosed, out);
                        }
                        RustStmt::Expr(value) => value.write(Slot::Enclosed, out),
                    }
                    out.push_str("; ");
                }
                tail.write(Slot::Enclosed, out);
                out.push_str(" }");
            }
        }
    }
}

fn write_arguments(args: &[RustExpr], out: &mut String) {
    out.push('(');
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        arg.write(Slot::Enclosed, out);
    }
    out.push(')');
}

/// The whole expression, as the emitter hands it on.
impl fmt::Display for RustExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = String::new();
        self.write(Slot::Enclosed, &mut out);
        f.write_str(&out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::ToTokens;
    use syn::punctuated::Punctuated;

    fn at(text: &str) -> RustExpr {
        RustExpr::atom(text)
    }

    fn op(op: RustBinOp, left: RustExpr, right: RustExpr) -> RustExpr {
        RustExpr::binary(op, left, right)
    }

    fn choice() -> RustExpr {
        RustExpr::if_else(at("c"), at("1"), at("2"))
    }

    /// An operator groups an operand that binds more loosely than itself, and a
    /// right operand that binds as tightly, since every binary operator of Rust
    /// associates to the left; a prefix operator groups an operand that is no
    /// prefix or postfix expression, and a postfix read one that is no postfix
    /// expression.
    #[test]
    fn an_operand_is_grouped_by_precedence_and_left_association() {
        let (a, b, c) = (at("a"), at("b"), at("c"));
        let sum = op(RustBinOp::Add, a.clone(), b.clone());
        assert_eq!(
            op(RustBinOp::Mul, sum.clone(), c.clone()).to_string(),
            "(a + b) * c"
        );
        assert_eq!(
            op(
                RustBinOp::Add,
                op(RustBinOp::Mul, a.clone(), b.clone()),
                c.clone()
            )
            .to_string(),
            "a * b + c"
        );
        assert_eq!(
            op(
                RustBinOp::Sub,
                op(RustBinOp::Sub, a.clone(), b.clone()),
                c.clone()
            )
            .to_string(),
            "a - b - c"
        );
        assert_eq!(
            op(
                RustBinOp::Sub,
                a.clone(),
                op(RustBinOp::Sub, b.clone(), c.clone())
            )
            .to_string(),
            "a - (b - c)"
        );
        // `&` binds tighter than `==` in Rust, where C has it the other way.
        assert_eq!(
            op(
                RustBinOp::Eq,
                op(RustBinOp::BitAnd, a.clone(), b.clone()),
                c.clone()
            )
            .to_string(),
            "a & b == c"
        );
        assert_eq!(
            op(RustBinOp::Or, a.clone(), op(RustBinOp::And, b, c)).to_string(),
            "a || b && c"
        );
        assert_eq!(sum.clone().prefixed(RustUnOp::Neg).to_string(), "-(a + b)");
        assert_eq!(a.field("f").prefixed(RustUnOp::Neg).to_string(), "-a.f");
        assert_eq!(
            sum.clone().method("len", Vec::new()).to_string(),
            "(a + b).len()"
        );
        assert_eq!(
            RustExpr::path_call("f", vec![sum.clone()]).to_string(),
            "f(a + b)"
        );
        assert_eq!(sum.borrowed_whole().to_string(), "&(a + b)[..]");
    }

    /// A comparison does not associate in Rust: one under another is grouped on
    /// either side, where precedence alone would leave the left one bare.
    #[test]
    fn a_comparison_is_grouped_under_a_comparison() {
        let test = op(RustBinOp::Eq, at("a"), at("b"));
        assert_eq!(
            op(RustBinOp::Eq, test.clone(), at("c")).to_string(),
            "(a == b) == c"
        );
        assert_eq!(
            op(RustBinOp::Lt, at("c"), test.clone()).to_string(),
            "c < (a == b)"
        );
        assert_eq!(
            op(RustBinOp::And, test.clone(), test).to_string(),
            "a == b && a == b"
        );
    }

    /// A type followed by `<` or `<<` opens generic arguments, so a left operand of
    /// either that ends in a cast is grouped; before `<=`, and on the right, the
    /// cast stands bare, and a postfix read or a prefix operator groups a cast by
    /// precedence.
    #[test]
    fn a_cast_is_grouped_where_its_type_would_open_generic_arguments() {
        let widened = at("buf").method("len", Vec::new()).cast("u32");
        assert_eq!(
            op(RustBinOp::Lt, widened.clone(), at("n")).to_string(),
            "(buf.len() as u32) < n"
        );
        assert_eq!(
            op(RustBinOp::Shl, widened.clone(), at("n")).to_string(),
            "(buf.len() as u32) << n"
        );
        assert_eq!(
            op(RustBinOp::Le, widened.clone(), at("n")).to_string(),
            "buf.len() as u32 <= n"
        );
        assert_eq!(
            op(RustBinOp::Gt, at("n"), widened.clone()).to_string(),
            "n > buf.len() as u32"
        );
        assert_eq!(
            op(
                RustBinOp::Lt,
                op(RustBinOp::Add, at("a"), widened.clone()),
                at("n")
            )
            .to_string(),
            "(a + buf.len() as u32) < n"
        );
        assert_eq!(
            op(
                RustBinOp::Lt,
                op(RustBinOp::Add, widened.clone(), at("a")),
                at("n")
            )
            .to_string(),
            "buf.len() as u32 + a < n"
        );
        assert_eq!(
            widened.clone().method("f", Vec::new()).to_string(),
            "(buf.len() as u32).f()"
        );
        assert_eq!(
            widened.clone().field("f").to_string(),
            "(buf.len() as u32).f"
        );
        assert_eq!(widened.clone().tried().to_string(), "(buf.len() as u32)?");
        assert_eq!(
            widened.clone().prefixed(RustUnOp::Neg).to_string(),
            "-(buf.len() as u32)"
        );
        assert_eq!(widened.cast("u64").to_string(), "buf.len() as u32 as u64");
        assert_eq!(
            op(RustBinOp::Add, at("a"), at("b")).cast("u8").to_string(),
            "(a + b) as u8"
        );
        assert_eq!(
            at("a").prefixed(RustUnOp::Neg).cast("u8").to_string(),
            "-a as u8"
        );
    }

    /// An expression with a block ends a statement it begins, so it is grouped
    /// where it is the first operand of an operator, the value of a cast or what
    /// a postfix reads, and stands bare after an operator and between
    /// delimiters.
    #[test]
    fn an_expression_with_a_block_is_grouped_only_where_it_comes_first() {
        assert_eq!(choice().to_string(), "if c { 1 } else { 2 }");
        assert_eq!(
            op(RustBinOp::Add, choice(), at("a")).to_string(),
            "(if c { 1 } else { 2 }) + a"
        );
        assert_eq!(
            op(RustBinOp::Add, at("a"), choice()).to_string(),
            "a + if c { 1 } else { 2 }"
        );
        assert_eq!(
            choice().cast("u8").to_string(),
            "(if c { 1 } else { 2 }) as u8"
        );
        assert_eq!(
            choice().method("f", Vec::new()).to_string(),
            "(if c { 1 } else { 2 }).f()"
        );
        assert_eq!(
            choice().prefixed(RustUnOp::Neg).to_string(),
            "-if c { 1 } else { 2 }"
        );
        assert_eq!(
            RustExpr::path_call("Some", vec![choice()]).to_string(),
            "Some(if c { 1 } else { 2 })"
        );
        let read = RustExpr::match_of(
            at("h").prefixed(RustUnOp::Ref),
            vec![RustArm {
                pattern: "_".to_string(),
                body: at("1"),
            }],
            true,
        );
        assert_eq!(read.to_string(), "match &h { _ => 1, }");
        assert_eq!(read.as_operand(), "(match &h { _ => 1, })");
        assert_eq!(at("h").method("tag", Vec::new()).as_operand(), "h.tag()");
    }

    /// A call of a field read built from parts is the method call Rust reads
    /// `a.f(x)` as, and a field read that is called is grouped.
    #[test]
    fn a_called_field_is_a_method_call_or_grouped() {
        assert_eq!(
            RustExpr::call(at("a").field("f"), vec![at("x")]),
            at("a").method("f", vec![at("x")])
        );
        let field_call = RustExpr::Call {
            callee: Box::new(at("a").field("f")),
            args: vec![at("x")],
        };
        assert_eq!(field_call.to_string(), "(a.f)(x)");
    }

    // ── The printer read back by syn ───────────────────────────────

    /// The tree of the text syn read: the printer's text means what the printed
    /// tree does exactly where this reads it back to that tree, grouping
    /// dropped.
    fn read(expr: &syn::Expr) -> RustExpr {
        match expr {
            syn::Expr::Paren(e) => read(&e.expr),
            syn::Expr::Group(e) => read(&e.expr),
            syn::Expr::Lit(e) => RustExpr::Atom(e.lit.to_token_stream().to_string()),
            syn::Expr::Path(e) => RustExpr::Atom(e.path.to_token_stream().to_string()),
            syn::Expr::Range(e) if e.start.is_none() && e.end.is_none() => RustExpr::FullRange,
            syn::Expr::Call(e) => RustExpr::Call {
                callee: Box::new(read(&e.func)),
                args: e.args.iter().map(read).collect(),
            },
            syn::Expr::Macro(e) => {
                let args = e
                    .mac
                    .parse_body_with(Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated)
                    .expect("the arguments of a macro are expressions");
                RustExpr::Macro {
                    name: e.mac.path.to_token_stream().to_string(),
                    args: args.iter().map(read).collect(),
                }
            }
            syn::Expr::MethodCall(e) => RustExpr::MethodCall {
                receiver: Box::new(read(&e.receiver)),
                method: e.method.to_string(),
                args: e.args.iter().map(read).collect(),
            },
            syn::Expr::Field(e) => RustExpr::Field {
                base: Box::new(read(&e.base)),
                member: e.member.to_token_stream().to_string(),
            },
            syn::Expr::Index(e) => RustExpr::Index {
                base: Box::new(read(&e.expr)),
                index: Box::new(read(&e.index)),
            },
            syn::Expr::Try(e) => RustExpr::Try(Box::new(read(&e.expr))),
            syn::Expr::Unary(e) => RustExpr::Unary {
                op: match e.op {
                    syn::UnOp::Neg(_) => RustUnOp::Neg,
                    syn::UnOp::Not(_) => RustUnOp::Not,
                    _ => panic!(
                        "`{}` has an operator the printer never writes",
                        expr.to_token_stream()
                    ),
                },
                operand: Box::new(read(&e.expr)),
            },
            syn::Expr::Reference(e) => RustExpr::Unary {
                op: if e.mutability.is_some() {
                    RustUnOp::RefMut
                } else {
                    RustUnOp::Ref
                },
                operand: Box::new(read(&e.expr)),
            },
            syn::Expr::Cast(e) => RustExpr::Cast {
                value: Box::new(read(&e.expr)),
                ty: e.ty.to_token_stream().to_string(),
            },
            syn::Expr::Binary(e) => RustExpr::Binary {
                op: binary(&e.op),
                left: Box::new(read(&e.left)),
                right: Box::new(read(&e.right)),
            },
            syn::Expr::If(e) => {
                let Some((_, otherwise)) = &e.else_branch else {
                    panic!("`{}` has no else", expr.to_token_stream());
                };
                let syn::Expr::Block(otherwise) = &**otherwise else {
                    panic!("`{}` has an else that is no block", expr.to_token_stream());
                };
                RustExpr::If {
                    condition: Box::new(read(&e.cond)),
                    then: Box::new(tail(&e.then_branch)),
                    otherwise: Box::new(tail(&otherwise.block)),
                }
            }
            syn::Expr::Match(e) => RustExpr::Match {
                scrutinee: Box::new(read(&e.expr)),
                arms: e
                    .arms
                    .iter()
                    .map(|arm| RustArm {
                        pattern: arm.pat.to_token_stream().to_string(),
                        body: read(&arm.body),
                    })
                    .collect(),
                trailing_comma: e.arms.last().is_some_and(|arm| arm.comma.is_some()),
            },
            syn::Expr::Block(e) => block(&e.block),
            _ => panic!(
                "`{}` is no expression the printer writes",
                expr.to_token_stream()
            ),
        }
    }

    fn binary(op: &syn::BinOp) -> RustBinOp {
        match op {
            syn::BinOp::Mul(_) => RustBinOp::Mul,
            syn::BinOp::Div(_) => RustBinOp::Div,
            syn::BinOp::Rem(_) => RustBinOp::Rem,
            syn::BinOp::Add(_) => RustBinOp::Add,
            syn::BinOp::Sub(_) => RustBinOp::Sub,
            syn::BinOp::Shl(_) => RustBinOp::Shl,
            syn::BinOp::Shr(_) => RustBinOp::Shr,
            syn::BinOp::BitAnd(_) => RustBinOp::BitAnd,
            syn::BinOp::BitXor(_) => RustBinOp::BitXor,
            syn::BinOp::BitOr(_) => RustBinOp::BitOr,
            syn::BinOp::Eq(_) => RustBinOp::Eq,
            syn::BinOp::Ne(_) => RustBinOp::Ne,
            syn::BinOp::Lt(_) => RustBinOp::Lt,
            syn::BinOp::Gt(_) => RustBinOp::Gt,
            syn::BinOp::Le(_) => RustBinOp::Le,
            syn::BinOp::Ge(_) => RustBinOp::Ge,
            syn::BinOp::And(_) => RustBinOp::And,
            syn::BinOp::Or(_) => RustBinOp::Or,
            _ => panic!(
                "`{}` is no operator the printer writes",
                op.to_token_stream()
            ),
        }
    }

    /// The one expression a block holds: the branch of an `if`, or a block
    /// read where a statement begins.
    fn tail(block: &syn::Block) -> RustExpr {
        match block.stmts.as_slice() {
            [syn::Stmt::Expr(value, None)] => read(value),
            _ => panic!(
                "`{}` is no block of one expression",
                block.to_token_stream()
            ),
        }
    }

    fn block(block: &syn::Block) -> RustExpr {
        let Some((syn::Stmt::Expr(tail, None), statements)) = block.stmts.split_last() else {
            panic!("`{}` ends in no expression", block.to_token_stream());
        };
        RustExpr::Block {
            statements: statements
                .iter()
                .map(|statement| match statement {
                    syn::Stmt::Local(local) => RustStmt::Let {
                        pattern: local.pat.to_token_stream().to_string(),
                        init: read(&local.init.as_ref().expect("a let with a value").expr),
                    },
                    syn::Stmt::Expr(value, Some(_)) => RustStmt::Expr(read(value)),
                    _ => panic!(
                        "`{}` holds a statement the printer never writes",
                        block.to_token_stream()
                    ),
                })
                .collect(),
            tail: Box::new(read(tail)),
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

    const OPERATORS: [RustBinOp; 18] = [
        RustBinOp::Mul,
        RustBinOp::Div,
        RustBinOp::Rem,
        RustBinOp::Add,
        RustBinOp::Sub,
        RustBinOp::Shl,
        RustBinOp::Shr,
        RustBinOp::BitAnd,
        RustBinOp::BitXor,
        RustBinOp::BitOr,
        RustBinOp::Eq,
        RustBinOp::Ne,
        RustBinOp::Lt,
        RustBinOp::Gt,
        RustBinOp::Le,
        RustBinOp::Ge,
        RustBinOp::And,
        RustBinOp::Or,
    ];

    const PREFIXES: [RustUnOp; 4] = [
        RustUnOp::Neg,
        RustUnOp::Not,
        RustUnOp::Ref,
        RustUnOp::RefMut,
    ];

    /// The seed every drawing starts from.
    const SEED: u64 = 0x9E37_79B9_7F4A_7C15;

    /// How many trees a drawing holds.
    const TREES: usize = 4000;

    /// A tree of every node kind, at most `depth` deep, a cast and a prefix
    /// operator among the operands of every other kind. `blocks` says whether an
    /// expression with a block may be drawn, which a condition and a scrutinee
    /// are kept free of.
    fn tree(draw: &mut Draw, depth: u32, blocks: bool) -> RustExpr {
        if depth == 0 || draw.below(6) == 0 {
            return at(draw.pick(&["a", "b", "x", "None", "2u8", "7i32", "\"s\""]));
        }
        let d = depth - 1;
        let kinds = if blocks { 16 } else { 13 };
        match draw.below(kinds) {
            0 => {
                let callee = if draw.below(4) == 0 {
                    tree(draw, d, blocks)
                } else {
                    at("f")
                };
                let count = draw.below(3);
                let args = (0..count).map(|_| tree(draw, d, blocks)).collect();
                RustExpr::Call {
                    callee: Box::new(callee),
                    args,
                }
            }
            1 => {
                let receiver = tree(draw, d, blocks);
                let method = draw.pick(&["m", "len"]);
                let count = draw.below(3);
                let args = (0..count).map(|_| tree(draw, d, blocks)).collect();
                receiver.method(method, args)
            }
            2 => {
                let base = tree(draw, d, blocks);
                base.field(draw.pick(&["f", "g"]))
            }
            3 => {
                let base = tree(draw, d, blocks);
                let index = if draw.below(3) == 0 {
                    RustExpr::FullRange
                } else {
                    tree(draw, d, blocks)
                };
                base.indexed(index)
            }
            4 => tree(draw, d, blocks).tried(),
            5 => {
                let prefix = PREFIXES[draw.below(PREFIXES.len())];
                tree(draw, d, blocks).prefixed(prefix)
            }
            6 | 7 => {
                let value = tree(draw, d, blocks);
                value.cast(draw.pick(&["u8", "u32", "i64", "f64", "usize"]))
            }
            8..=11 => {
                let operator = OPERATORS[draw.below(OPERATORS.len())];
                let left = tree(draw, d, blocks);
                let right = tree(draw, d, blocks);
                RustExpr::binary(operator, left, right)
            }
            12 => {
                let count = draw.below(3);
                let mut args = vec![at("\"{}\"")];
                args.extend((0..count).map(|_| tree(draw, d, blocks)));
                RustExpr::macro_call("format", args)
            }
            13 => {
                let condition = tree(draw, d, false);
                let then = tree(draw, d, blocks);
                let otherwise = tree(draw, d, blocks);
                RustExpr::if_else(condition, then, otherwise)
            }
            14 => {
                let scrutinee = tree(draw, d, false);
                let count = 1 + draw.below(3);
                let arms = (0..count)
                    .map(|_| {
                        let pattern = draw.pick(&["_", "x", "y"]).to_string();
                        RustArm {
                            pattern,
                            body: tree(draw, d, blocks),
                        }
                    })
                    .collect();
                let trailing_comma = draw.below(2) == 0;
                RustExpr::match_of(scrutinee, arms, trailing_comma)
            }
            _ => {
                let count = draw.below(3);
                let statements = (0..count)
                    .map(|_| {
                        if draw.below(2) == 0 {
                            let pattern = draw.pick(&["x", "mut y"]).to_string();
                            RustStmt::Let {
                                pattern,
                                init: tree(draw, d, blocks),
                            }
                        } else {
                            let count = draw.below(3);
                            let args = (0..count).map(|_| tree(draw, d, false)).collect();
                            RustStmt::Expr(RustExpr::path_call("f", args).tried())
                        }
                    })
                    .collect();
                let tail = tree(draw, d, blocks);
                RustExpr::block(statements, tail)
            }
        }
    }

    /// Every tree the printer is handed is read back as itself by syn: as an
    /// expression, as the tail of a block, where a statement begins and an
    /// expression with a block ends it, and spliced as one operand where every
    /// operator, cast and postfix read could take it apart. Every parenthesis
    /// the grammar needs is printed, and none the printer leaves out changes
    /// what is read.
    #[test]
    fn what_the_printer_writes_reads_back_as_the_tree_it_was_handed() {
        let mut draw = Draw(SEED);
        for _ in 0..TREES {
            let tree = tree(&mut draw, 5, true);
            let text = tree.to_string();
            let expression = syn::parse_str::<syn::Expr>(&text)
                .unwrap_or_else(|e| panic!("`{text}` is no Rust expression: {e}"));
            assert_eq!(read(&expression), tree, "`{text}` read as an expression");
            let statement = syn::parse_str::<syn::Block>(&format!("{{ {text} }}"))
                .unwrap_or_else(|e| panic!("`{text}` is no block's tail: {e}"));
            assert_eq!(tail(&statement), tree, "`{text}` read as a block's tail");
            let operand = tree.as_operand();
            let spliced = format!("{{ {operand}.m() * -{operand} as u8 }}");
            let spliced = syn::parse_str::<syn::Block>(&spliced)
                .unwrap_or_else(|e| panic!("`{operand}` is no operand: {e}"));
            assert_eq!(
                tail(&spliced),
                op(
                    RustBinOp::Mul,
                    tree.clone().method("m", Vec::new()),
                    tree.prefixed(RustUnOp::Neg).cast("u8")
                ),
                "`{operand}` stands as one operand"
            );
        }
    }

    /// The kind of a node, as its variant is named.
    fn kind(tree: &RustExpr) -> &'static str {
        match tree {
            RustExpr::Atom(_) => "Atom",
            RustExpr::FullRange => "FullRange",
            RustExpr::Call { .. } => "Call",
            RustExpr::Macro { .. } => "Macro",
            RustExpr::MethodCall { .. } => "MethodCall",
            RustExpr::Field { .. } => "Field",
            RustExpr::Index { .. } => "Index",
            RustExpr::Try(_) => "Try",
            RustExpr::Unary { .. } => "Unary",
            RustExpr::Cast { .. } => "Cast",
            RustExpr::Binary { .. } => "Binary",
            RustExpr::If { .. } => "If",
            RustExpr::Match { .. } => "Match",
            RustExpr::Block { .. } => "Block",
        }
    }

    /// The nodes directly inside `tree`.
    fn children(tree: &RustExpr) -> Vec<&RustExpr> {
        match tree {
            RustExpr::Atom(_) | RustExpr::FullRange => Vec::new(),
            RustExpr::Call { callee, args } => std::iter::once(&**callee).chain(args).collect(),
            RustExpr::Macro { args, .. } => args.iter().collect(),
            RustExpr::MethodCall { receiver, args, .. } => {
                std::iter::once(&**receiver).chain(args).collect()
            }
            RustExpr::Field { base, .. } => vec![&**base],
            RustExpr::Index { base, index } => vec![&**base, &**index],
            RustExpr::Try(value) => vec![&**value],
            RustExpr::Unary { operand, .. } => vec![&**operand],
            RustExpr::Cast { value, .. } => vec![&**value],
            RustExpr::Binary { left, right, .. } => vec![&**left, &**right],
            RustExpr::If {
                condition,
                then,
                otherwise,
            } => vec![&**condition, &**then, &**otherwise],
            RustExpr::Match {
                scrutinee, arms, ..
            } => std::iter::once(&**scrutinee)
                .chain(arms.iter().map(|arm| &arm.body))
                .collect(),
            RustExpr::Block { statements, tail } => statements
                .iter()
                .map(|statement| match statement {
                    RustStmt::Let { init, .. } => init,
                    RustStmt::Expr(value) => value,
                })
                .chain(std::iter::once(&**tail))
                .collect(),
        }
    }

    /// Every kind of node and every place a cast can stand in a parent is drawn,
    /// a cast in a cast and a prefix operator in a prefix operator among them, so
    /// the read-back above reaches each rule of the printer.
    #[test]
    fn the_trees_drawn_hold_every_kind_and_a_cast_in_every_parent() {
        let mut seen = std::collections::BTreeSet::new();
        let mut draw = Draw(SEED);
        for _ in 0..TREES {
            let mut stack = vec![tree(&mut draw, 5, true)];
            while let Some(node) = stack.pop() {
                seen.insert(kind(&node).to_string());
                for child in children(&node) {
                    if matches!(child, RustExpr::Cast { .. } | RustExpr::Unary { .. }) {
                        seen.insert(format!("{} in {}", kind(child), kind(&node)));
                    }
                    stack.push(child.clone());
                }
            }
        }
        for kind in [
            "Atom",
            "FullRange",
            "Call",
            "Macro",
            "MethodCall",
            "Field",
            "Index",
            "Try",
            "Unary",
            "Cast",
            "Binary",
            "If",
            "Match",
            "Block",
            "Unary in Unary",
        ] {
            assert!(seen.contains(kind), "no {kind} was drawn: {seen:?}");
        }
        for parent in [
            "Call",
            "Macro",
            "MethodCall",
            "Field",
            "Index",
            "Try",
            "Unary",
            "Cast",
            "Binary",
            "If",
            "Match",
            "Block",
        ] {
            assert!(
                seen.contains(&format!("Cast in {parent}")),
                "no cast was drawn in a {parent}: {seen:?}"
            );
        }
    }
}
