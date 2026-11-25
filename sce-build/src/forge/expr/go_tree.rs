// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Go a forge expression is written as, as a tree of the Go expressions it
//! is made of, and the one printer that writes such a tree as text.
//!
//! The Go emitter ([`super::emit_go`]) builds this tree and prints it once, where
//! the text leaves it; the Go walks over a composed value
//! ([`crate::forge::composed_lowering`]) build the expressions they read a held
//! value with too. A conversion, a call, a selector, an index and a function
//! literal are nodes of their own wherever they come from; nothing is read
//! back out of text.
//!
//! The printer decides every parenthesis, at each parent, from the kind of the
//! child and the precedence and associativity The Go Programming Language
//! Specification gives its operators ("Operators", "Operator precedence"): a
//! primary expression (an operand, a conversion, a selector, an index, a
//! slice, a call: "Primary expressions") binds tightest, then the unary
//! operators, then the five levels of the binary ones, `*  /  %  <<  >>  &`
//! above `+  -  |  ^` above `==  !=  <  <=  >  >=` above `&&` above `||`; binary
//! operators of one level associate from the left, comparisons among them.
//! This is the textbook unparsing rule (Ramsey, "Unparsing Expressions with
//! Prefix and Postfix Operators", 1998). The one rule of Go's grammar that
//! precedence does not see is the printer's too:
//!
//! * a token is the longest sequence of characters that forms one ("Tokens"),
//!   so two prefix operators written together can read as one token: `-` and
//!   `-` as the decrement `--`, `+` and `+` as the increment `++`, `&` and `&`
//!   as `&&`, `&` and `^` as `&^`. A prefix operator over one it would fuse
//!   with groups it: `-(-x)`.
//!
//! Everything else stands bare: a function literal is an operand, called or
//! not, so the function literal a conditional is written as is never grouped.
//! Two rules of the grammar are about the text of an atom, which the printer
//! writes as it is, and are its writer's: a conversion to a type that begins
//! with `*`, `<-` or `func` is grouped ("Conversions"), and so is a composite
//! literal of a type name between `if` and its block ("Composite literals").
//!
//! No Go parser is among the workspace's dependencies, so the tests hold the
//! printer to the grammar by reading what it prints back with a parser written
//! from the specification's productions and its own table of the binary
//! levels, apart from the printer's, and comparing the tree read with the tree
//! printed. The Go lanes compile what the emitter prints.

use std::fmt;

/// A Go expression the emitter writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GoExpr {
    /// Text written as it is that stands as one operand: a literal, a name, a
    /// qualified identifier, or a fragment its writer made one
    /// ([`super::ExprKind::Raw`]).
    Atom(String),
    /// `callee(args)`, a conversion `T(x)` among them.
    Call {
        callee: Box<GoExpr>,
        args: Vec<GoExpr>,
    },
    /// `base.member`.
    Selector { base: Box<GoExpr>, member: String },
    /// `base[index]`.
    Index {
        base: Box<GoExpr>,
        index: Box<GoExpr>,
    },
    /// `base[:]`, the whole of an array or a slice as a slice.
    Slice(Box<GoExpr>),
    /// A prefix operator over its operand.
    Unary { op: GoUnOp, operand: Box<GoExpr> },
    /// `left op right`.
    Binary {
        op: GoBinOp,
        left: Box<GoExpr>,
        right: Box<GoExpr>,
    },
    /// `func() ty { if condition { return then }; return otherwise }()`: the
    /// function literal a conditional is written as, called where it stands, so
    /// the branch not taken is never evaluated.
    Choice {
        ty: String,
        condition: Box<GoExpr>,
        then: Box<GoExpr>,
        otherwise: Box<GoExpr>,
    },
    /// `func() ty { return value }`: a function literal that answers `value` when
    /// it is called.
    Thunk { ty: String, value: Box<GoExpr> },
}

/// The prefix operators the emitter writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GoUnOp {
    /// `-`
    Neg,
    /// `+`
    Pos,
    /// `!`
    Not,
    /// `^`, the bitwise complement.
    Complement,
    /// `&`, the address of its operand.
    Address,
}

impl GoUnOp {
    fn token(self) -> &'static str {
        match self {
            Self::Neg => "-",
            Self::Pos => "+",
            Self::Not => "!",
            Self::Complement => "^",
            Self::Address => "&",
        }
    }

    /// Whether this operator written right before `inner` reads as one token of
    /// Go's: `--`, `++`, `&&` or `&^`.
    fn fuses_with(self, inner: GoUnOp) -> bool {
        matches!(
            (self, inner),
            (Self::Neg, Self::Neg)
                | (Self::Pos, Self::Pos)
                | (Self::Address, Self::Address | Self::Complement)
        )
    }
}

/// The binary operators the emitter writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GoBinOp {
    Mul,
    Div,
    Rem,
    Shl,
    Shr,
    BitAnd,
    Add,
    Sub,
    BitOr,
    BitXor,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

impl GoBinOp {
    fn token(self) -> &'static str {
        match self {
            Self::Mul => "*",
            Self::Div => "/",
            Self::Rem => "%",
            Self::Shl => "<<",
            Self::Shr => ">>",
            Self::BitAnd => "&",
            Self::Add => "+",
            Self::Sub => "-",
            Self::BitOr => "|",
            Self::BitXor => "^",
            Self::Eq => "==",
            Self::Ne => "!=",
            Self::Lt => "<",
            Self::Le => "<=",
            Self::Gt => ">",
            Self::Ge => ">=",
            Self::And => "&&",
            Self::Or => "||",
        }
    }

    /// The level of the specification's table the operator is on.
    fn precedence(self) -> Precedence {
        match self {
            Self::Mul | Self::Div | Self::Rem | Self::Shl | Self::Shr | Self::BitAnd => {
                Precedence::Product
            }
            Self::Add | Self::Sub | Self::BitOr | Self::BitXor => Precedence::Sum,
            Self::Eq | Self::Ne | Self::Lt | Self::Le | Self::Gt | Self::Ge => Precedence::Compare,
            Self::And => Precedence::And,
            Self::Or => Precedence::Or,
        }
    }
}

/// How tightly an expression binds, loosest first: the five levels of the
/// binary operators, a unary expression above them, and above that every
/// primary expression, which no operator can split.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Precedence {
    Or,
    And,
    Compare,
    Sum,
    Product,
    Unary,
    Primary,
}

/// Where a child stands in its parent, which is all the printer needs to
/// decide whether it is grouped.
#[derive(Debug, Clone, Copy)]
enum Slot {
    /// Between delimiters of the parent's own (an argument, an index, a
    /// condition, a `return`), or the whole expression.
    Enclosed,
    /// What a call calls, or what a selector, an index or a slice reads.
    Postfix,
    /// The operand of the prefix operator.
    Prefix(GoUnOp),
    /// The left operand of a binary operator.
    Left(GoBinOp),
    /// The right operand of a binary operator.
    Right(GoBinOp),
}

impl GoExpr {
    /// Text that stands as one operand, written as it is.
    pub(crate) fn atom(text: impl Into<String>) -> Self {
        Self::Atom(text.into())
    }

    /// A call of `callee` with `args`.
    pub(crate) fn call(callee: GoExpr, args: Vec<GoExpr>) -> Self {
        Self::Call {
            callee: Box::new(callee),
            args,
        }
    }

    /// A call of the function the qualified identifier `path` names.
    pub(crate) fn path_call(path: impl Into<String>, args: Vec<GoExpr>) -> Self {
        Self::call(Self::atom(path), args)
    }

    /// This value converted to the type `ty`: `ty(value)`.
    pub(crate) fn converted(self, ty: impl Into<String>) -> Self {
        Self::path_call(ty, vec![self])
    }

    /// The method `method` of this value called with `args`.
    pub(crate) fn method(self, method: impl Into<String>, args: Vec<GoExpr>) -> Self {
        Self::call(self.selector(method), args)
    }

    /// The field or method `member` of this value.
    pub(crate) fn selector(self, member: impl Into<String>) -> Self {
        Self::Selector {
            base: Box::new(self),
            member: member.into(),
        }
    }

    /// This value indexed at `index`.
    pub(crate) fn indexed(self, index: GoExpr) -> Self {
        Self::Index {
            base: Box::new(self),
            index: Box::new(index),
        }
    }

    /// The whole of this array or slice as a slice: `value[:]`.
    pub(crate) fn sliced(self) -> Self {
        Self::Slice(Box::new(self))
    }

    /// The prefix operator `op` over this value.
    pub(crate) fn prefixed(self, op: GoUnOp) -> Self {
        Self::Unary {
            op,
            operand: Box::new(self),
        }
    }

    /// `left op right`.
    pub(crate) fn binary(op: GoBinOp, left: GoExpr, right: GoExpr) -> Self {
        Self::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    /// The function literal of the result type `ty` that answers `then` where
    /// `condition` holds and `otherwise` where it does not, called where it stands.
    pub(crate) fn choice(
        ty: impl Into<String>,
        condition: GoExpr,
        then: GoExpr,
        otherwise: GoExpr,
    ) -> Self {
        Self::Choice {
            ty: ty.into(),
            condition: Box::new(condition),
            then: Box::new(then),
            otherwise: Box::new(otherwise),
        }
    }

    /// The function literal of the result type `ty` that answers `value`.
    pub(crate) fn thunk(ty: impl Into<String>, value: GoExpr) -> Self {
        Self::Thunk {
            ty: ty.into(),
            value: Box::new(value),
        }
    }

    /// This expression printed to stand where a fragment is spliced as it is
    /// (a held value a walk reads, [`super::ExprKind::Raw`]): grouped unless it is
    /// one operand that no operator or postfix read around it can take apart.
    pub(crate) fn as_operand(&self) -> String {
        let mut out = String::new();
        self.write(Slot::Postfix, &mut out);
        out
    }

    fn precedence(&self) -> Precedence {
        match self {
            Self::Binary { op, .. } => op.precedence(),
            Self::Unary { .. } => Precedence::Unary,
            Self::Atom(_)
            | Self::Call { .. }
            | Self::Selector { .. }
            | Self::Index { .. }
            | Self::Slice(_)
            | Self::Choice { .. }
            | Self::Thunk { .. } => Precedence::Primary,
        }
    }

    /// Whether this expression is grouped where it stands at `slot`.
    fn needs_parentheses(&self, slot: Slot) -> bool {
        let own = self.precedence();
        match slot {
            Slot::Enclosed => false,
            Slot::Postfix => own < Precedence::Primary,
            Slot::Prefix(op) => {
                own < Precedence::Unary
                    || matches!(self, Self::Unary { op: inner, .. } if op.fuses_with(*inner))
            }
            Slot::Left(op) => own < op.precedence(),
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
            Self::Selector { base, member } => {
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
            Self::Slice(base) => {
                base.write(Slot::Postfix, out);
                out.push_str("[:]");
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
            Self::Choice {
                ty,
                condition,
                then,
                otherwise,
            } => {
                out.push_str("func() ");
                out.push_str(ty);
                out.push_str(" { if ");
                condition.write(Slot::Enclosed, out);
                out.push_str(" { return ");
                then.write(Slot::Enclosed, out);
                out.push_str(" }; return ");
                otherwise.write(Slot::Enclosed, out);
                out.push_str(" }()");
            }
            Self::Thunk { ty, value } => {
                out.push_str("func() ");
                out.push_str(ty);
                out.push_str(" { return ");
                value.write(Slot::Enclosed, out);
                out.push_str(" }");
            }
        }
    }
}

/// The whole expression, as the emitter hands it on.
impl fmt::Display for GoExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = String::new();
        self.write(Slot::Enclosed, &mut out);
        f.write_str(&out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str) -> GoExpr {
        GoExpr::atom(text)
    }

    fn op(op: GoBinOp, left: GoExpr, right: GoExpr) -> GoExpr {
        GoExpr::binary(op, left, right)
    }

    /// An operator groups an operand that binds more loosely than itself, and a
    /// right operand that binds as tightly, since every binary operator of Go
    /// associates to the left; a prefix operator groups a binary operand, and a
    /// postfix read any operand that is no primary expression.
    #[test]
    fn an_operand_is_grouped_by_precedence_and_left_association() {
        let (a, b, c) = (at("a"), at("b"), at("c"));
        let sum = op(GoBinOp::Add, a.clone(), b.clone());
        assert_eq!(
            op(GoBinOp::Mul, sum.clone(), c.clone()).to_string(),
            "(a + b) * c"
        );
        assert_eq!(
            op(
                GoBinOp::Add,
                op(GoBinOp::Mul, a.clone(), b.clone()),
                c.clone()
            )
            .to_string(),
            "a * b + c"
        );
        assert_eq!(
            op(
                GoBinOp::Sub,
                op(GoBinOp::Sub, a.clone(), b.clone()),
                c.clone()
            )
            .to_string(),
            "a - b - c"
        );
        assert_eq!(
            op(
                GoBinOp::Sub,
                a.clone(),
                op(GoBinOp::Sub, b.clone(), c.clone())
            )
            .to_string(),
            "a - (b - c)"
        );
        // A shift and `&` bind as tightly as `*` in Go, and `|` and `^` as `+`.
        assert_eq!(
            op(
                GoBinOp::Add,
                a.clone(),
                op(GoBinOp::Shl, b.clone(), c.clone())
            )
            .to_string(),
            "a + b << c"
        );
        assert_eq!(
            op(
                GoBinOp::Shl,
                op(GoBinOp::BitOr, a.clone(), b.clone()),
                c.clone()
            )
            .to_string(),
            "(a | b) << c"
        );
        // A comparison associates as every binary operator does.
        let test = op(GoBinOp::Eq, a.clone(), b.clone());
        assert_eq!(
            op(GoBinOp::Eq, test.clone(), c.clone()).to_string(),
            "a == b == c"
        );
        assert_eq!(
            op(GoBinOp::Eq, c.clone(), test.clone()).to_string(),
            "c == (a == b)"
        );
        assert_eq!(
            op(
                GoBinOp::Or,
                a.clone(),
                op(GoBinOp::And, b.clone(), c.clone())
            )
            .to_string(),
            "a || b && c"
        );
        assert_eq!(sum.clone().prefixed(GoUnOp::Neg).to_string(), "-(a + b)");
        assert_eq!(
            a.clone().selector("f").prefixed(GoUnOp::Neg).to_string(),
            "-a.f"
        );
        assert_eq!(
            sum.clone().method("Get", Vec::new()).to_string(),
            "(a + b).Get()"
        );
        assert_eq!(
            a.clone()
                .prefixed(GoUnOp::Not)
                .method("Get", Vec::new())
                .to_string(),
            "(!a).Get()"
        );
        assert_eq!(sum.clone().converted("int64").to_string(), "int64(a + b)");
        assert_eq!(sum.clone().sliced().to_string(), "(a + b)[:]");
        assert_eq!(a.indexed(sum).to_string(), "a[a + b]");
    }

    /// Two prefix operators written together that would read as one token of
    /// Go's are kept apart by a pair; any other two stand together.
    #[test]
    fn a_prefix_operator_groups_one_it_would_fuse_with() {
        let x = at("x");
        let twice =
            |outer: GoUnOp, inner: GoUnOp| x.clone().prefixed(inner).prefixed(outer).to_string();
        assert_eq!(twice(GoUnOp::Neg, GoUnOp::Neg), "-(-x)");
        assert_eq!(twice(GoUnOp::Pos, GoUnOp::Pos), "+(+x)");
        assert_eq!(twice(GoUnOp::Address, GoUnOp::Address), "&(&x)");
        assert_eq!(twice(GoUnOp::Address, GoUnOp::Complement), "&(^x)");
        assert_eq!(twice(GoUnOp::Neg, GoUnOp::Pos), "-+x");
        assert_eq!(twice(GoUnOp::Not, GoUnOp::Not), "!!x");
        assert_eq!(twice(GoUnOp::Complement, GoUnOp::Complement), "^^x");
        // Beside a binary operator a space keeps them apart already.
        assert_eq!(
            op(GoBinOp::Sub, at("a"), x.clone().prefixed(GoUnOp::Neg)).to_string(),
            "a - -x"
        );
    }

    /// The function literal a conditional is written as is an operand: it is
    /// called, read and operated on bare.
    #[test]
    fn a_function_literal_stands_bare_wherever_it_is() {
        let choice = || GoExpr::choice("int64", at("c"), at("a"), at("b"));
        assert_eq!(
            choice().to_string(),
            "func() int64 { if c { return a }; return b }()"
        );
        assert_eq!(
            op(GoBinOp::Add, choice(), at("x")).to_string(),
            "func() int64 { if c { return a }; return b }() + x"
        );
        assert_eq!(
            choice().prefixed(GoUnOp::Neg).to_string(),
            "-func() int64 { if c { return a }; return b }()"
        );
        assert_eq!(
            choice().selector("f").to_string(),
            "func() int64 { if c { return a }; return b }().f"
        );
        assert_eq!(
            GoExpr::path_call(
                "optional.Otherwise",
                vec![
                    at("x"),
                    GoExpr::thunk("int64", op(GoBinOp::Add, at("a"), at("b")))
                ]
            )
            .to_string(),
            "optional.Otherwise(x, func() int64 { return a + b })"
        );
        assert_eq!(choice().as_operand(), choice().to_string());
        assert_eq!(op(GoBinOp::Add, at("a"), at("b")).as_operand(), "(a + b)");
    }

    // ── The printer read back by a parser of Go's productions ─────────

    /// A token of the text read: a name, a string literal, or an operator or a
    /// delimiter, matched longest first as Go's scanner does ("Tokens").
    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Token {
        Name(String),
        Text(String),
        Mark(&'static str),
    }

    /// Go's operators and punctuation ("Operators and punctuation"), every
    /// longer one before the shorter ones it begins with.
    const MARKS: [&str; 48] = [
        "<<=", ">>=", "&^=", "...", "&&", "||", "<-", "++", "--", "==", "!=", "<=", ">=", ":=",
        "<<", ">>", "&^", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "~", "+", "-", "*", "/",
        "%", "&", "|", "^", "<", ">", "=", "!", "(", ")", "[", "]", "{", "}", ",", ";", ".", ":",
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
            } else if first == '"' {
                let end = 1 + rest[1..].find('"').expect("a closed string literal");
                out.push(Token::Text(rest[..=end].to_string()));
                rest = &rest[end + 1..];
            } else {
                let mark = MARKS
                    .iter()
                    .find(|mark| rest.starts_with(**mark))
                    .unwrap_or_else(|| panic!("no token of Go's begins {rest:?}"));
                out.push(Token::Mark(mark));
                rest = &rest[mark.len()..];
            }
        }
        out
    }

    /// The binary operators and their levels, as the specification's table
    /// lists them ("Operator precedence").
    const LEVELS: [(&str, GoBinOp, u8); 18] = [
        ("*", GoBinOp::Mul, 5),
        ("/", GoBinOp::Div, 5),
        ("%", GoBinOp::Rem, 5),
        ("<<", GoBinOp::Shl, 5),
        (">>", GoBinOp::Shr, 5),
        ("&", GoBinOp::BitAnd, 5),
        ("+", GoBinOp::Add, 4),
        ("-", GoBinOp::Sub, 4),
        ("|", GoBinOp::BitOr, 4),
        ("^", GoBinOp::BitXor, 4),
        ("==", GoBinOp::Eq, 3),
        ("!=", GoBinOp::Ne, 3),
        ("<", GoBinOp::Lt, 3),
        ("<=", GoBinOp::Le, 3),
        (">", GoBinOp::Gt, 3),
        (">=", GoBinOp::Ge, 3),
        ("&&", GoBinOp::And, 2),
        ("||", GoBinOp::Or, 1),
    ];

    /// The unary operators ("unary_op") the printer writes.
    const UNARY: [(&str, GoUnOp); 5] = [
        ("-", GoUnOp::Neg),
        ("+", GoUnOp::Pos),
        ("!", GoUnOp::Not),
        ("^", GoUnOp::Complement),
        ("&", GoUnOp::Address),
    ];

    /// A reader of the expressions the printer writes, by the productions of
    /// "Expressions": `Expression = UnaryExpr | Expression binary_op Expression`
    /// with the levels of the table above, `UnaryExpr = PrimaryExpr | unary_op
    /// UnaryExpr`, and `PrimaryExpr` an operand, a function literal, or a primary
    /// expression followed by a selector, an index, a slice or arguments. Each
    /// pair of parentheses is dropped where it is read.
    struct Reader {
        tokens: Vec<Token>,
        at: usize,
    }

    impl Reader {
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

        fn expect(&mut self, mark: &str) {
            let token = self.take();
            assert_eq!(token, Token::Mark(mark_of(mark)), "{:?}", self.tokens);
        }

        fn expect_name(&mut self, name: &str) {
            let token = self.take();
            assert_eq!(token, Token::Name(name.to_string()), "{:?}", self.tokens);
        }

        fn at_mark(&self, mark: &str) -> bool {
            matches!(self.peek(), Some(Token::Mark(m)) if *m == mark)
        }

        fn whole(text: &str) -> GoExpr {
            let mut reader = Reader {
                tokens: tokens(text),
                at: 0,
            };
            let read = reader.expression(1);
            assert_eq!(
                reader.at,
                reader.tokens.len(),
                "{text:?} goes on after one expression"
            );
            read
        }

        fn expression(&mut self, least: u8) -> GoExpr {
            let mut left = self.unary();
            loop {
                let Some(Token::Mark(mark)) = self.peek() else {
                    return left;
                };
                let Some(&(_, op, level)) = LEVELS.iter().find(|(token, _, _)| token == mark)
                else {
                    return left;
                };
                if level < least {
                    return left;
                }
                self.at += 1;
                let right = self.expression(level + 1);
                left = GoExpr::binary(op, left, right);
            }
        }

        fn unary(&mut self) -> GoExpr {
            if let Some(Token::Mark(mark)) = self.peek() {
                if let Some(&(_, op)) = UNARY.iter().find(|(token, _)| token == mark) {
                    self.at += 1;
                    return self.unary().prefixed(op);
                }
            }
            self.primary()
        }

        fn primary(&mut self) -> GoExpr {
            let mut read = self.operand();
            loop {
                if self.at_mark(".") {
                    self.at += 1;
                    let Token::Name(member) = self.take() else {
                        panic!("a selector names no member: {:?}", self.tokens);
                    };
                    read = read.selector(member);
                } else if self.at_mark("(") {
                    self.at += 1;
                    let args = self.arguments();
                    read = GoExpr::call(read, args);
                } else if self.at_mark("[") {
                    self.at += 1;
                    if self.at_mark(":") {
                        self.at += 1;
                        self.expect("]");
                        read = read.sliced();
                    } else {
                        let index = self.expression(1);
                        self.expect("]");
                        read = read.indexed(index);
                    }
                } else {
                    return read;
                }
            }
        }

        fn arguments(&mut self) -> Vec<GoExpr> {
            let mut args = Vec::new();
            if self.at_mark(")") {
                self.at += 1;
                return args;
            }
            loop {
                args.push(self.expression(1));
                if self.at_mark(",") {
                    self.at += 1;
                } else {
                    self.expect(")");
                    return args;
                }
            }
        }

        fn operand(&mut self) -> GoExpr {
            match self.take() {
                Token::Name(name) if name == "func" => self.function_literal(),
                Token::Name(name) => GoExpr::Atom(name),
                Token::Text(text) => GoExpr::Atom(text),
                Token::Mark("(") => {
                    let inner = self.expression(1);
                    self.expect(")");
                    inner
                }
                token => panic!("{token:?} begins no operand: {:?}", self.tokens),
            }
        }

        /// `func() T { return v }`, or `func() T { if c { return a }; return b }`
        /// called at once.
        fn function_literal(&mut self) -> GoExpr {
            self.expect("(");
            self.expect(")");
            let Token::Name(ty) = self.take() else {
                panic!("a function literal names no result: {:?}", self.tokens);
            };
            self.expect("{");
            if matches!(self.peek(), Some(Token::Name(name)) if name == "if") {
                self.at += 1;
                let condition = self.expression(1);
                self.expect("{");
                self.expect_name("return");
                let then = self.expression(1);
                self.expect("}");
                self.expect(";");
                self.expect_name("return");
                let otherwise = self.expression(1);
                self.expect("}");
                self.expect("(");
                self.expect(")");
                return GoExpr::choice(ty, condition, then, otherwise);
            }
            self.expect_name("return");
            let value = self.expression(1);
            self.expect("}");
            GoExpr::thunk(ty, value)
        }
    }

    /// The `'static` spelling of a mark, as a token holds it.
    fn mark_of(mark: &str) -> &'static str {
        MARKS
            .iter()
            .find(|m| **m == mark)
            .copied()
            .unwrap_or_else(|| panic!("{mark:?} is no mark of Go's"))
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

    /// The seed every drawing starts from.
    const SEED: u64 = 0x9E37_79B9_7F4A_7C15;

    /// How many trees a drawing holds.
    const TREES: usize = 4000;

    /// A tree of every node kind, at most `depth` deep. A number is never drawn
    /// as an atom: a decimal literal followed by the `.` of a selector reads as a
    /// real one, and the emitter reads no member of a number.
    fn tree(draw: &mut Draw, depth: u32) -> GoExpr {
        if depth == 0 || draw.below(6) == 0 {
            return at(draw.pick(&["a", "b", "x", "nil", "\"s\""]));
        }
        let d = depth - 1;
        match draw.below(13) {
            0 => {
                let callee = if draw.below(4) == 0 {
                    tree(draw, d)
                } else {
                    at(draw.pick(&["f", "int64"]))
                };
                let count = draw.below(3);
                let args = (0..count).map(|_| tree(draw, d)).collect();
                GoExpr::call(callee, args)
            }
            1 => {
                let receiver = tree(draw, d);
                let method = draw.pick(&["Get", "M"]);
                let count = draw.below(3);
                let args = (0..count).map(|_| tree(draw, d)).collect();
                receiver.method(method, args)
            }
            2 => tree(draw, d).selector(draw.pick(&["f", "g"])),
            3 => {
                let base = tree(draw, d);
                if draw.below(3) == 0 {
                    base.sliced()
                } else {
                    base.indexed(tree(draw, d))
                }
            }
            4 | 5 => {
                let (_, prefix) = UNARY[draw.below(UNARY.len())];
                tree(draw, d).prefixed(prefix)
            }
            6..=9 => {
                let (_, operator, _) = LEVELS[draw.below(LEVELS.len())];
                let left = tree(draw, d);
                let right = tree(draw, d);
                GoExpr::binary(operator, left, right)
            }
            10 | 11 => {
                let ty = draw.pick(&["int64", "bool"]);
                let condition = tree(draw, d);
                let then = tree(draw, d);
                let otherwise = tree(draw, d);
                GoExpr::choice(ty, condition, then, otherwise)
            }
            _ => {
                let ty = draw.pick(&["int64", "bool"]);
                GoExpr::thunk(ty, tree(draw, d))
            }
        }
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
            let spliced = format!("{operand}.M() * -{operand}");
            assert_eq!(
                Reader::whole(&spliced),
                op(
                    GoBinOp::Mul,
                    tree.clone().method("M", Vec::new()),
                    tree.prefixed(GoUnOp::Neg)
                ),
                "{operand:?} stands as one operand"
            );
        }
    }

    /// The kind of a node, as its variant is named.
    fn kind(tree: &GoExpr) -> &'static str {
        match tree {
            GoExpr::Atom(_) => "Atom",
            GoExpr::Call { .. } => "Call",
            GoExpr::Selector { .. } => "Selector",
            GoExpr::Index { .. } => "Index",
            GoExpr::Slice(_) => "Slice",
            GoExpr::Unary { .. } => "Unary",
            GoExpr::Binary { .. } => "Binary",
            GoExpr::Choice { .. } => "Choice",
            GoExpr::Thunk { .. } => "Thunk",
        }
    }

    /// The nodes directly inside `tree`.
    fn children(tree: &GoExpr) -> Vec<&GoExpr> {
        match tree {
            GoExpr::Atom(_) => Vec::new(),
            GoExpr::Call { callee, args } => std::iter::once(&**callee).chain(args).collect(),
            GoExpr::Selector { base, .. } | GoExpr::Slice(base) => vec![&**base],
            GoExpr::Index { base, index } => vec![&**base, &**index],
            GoExpr::Unary { operand, .. } => vec![&**operand],
            GoExpr::Binary { left, right, .. } => vec![&**left, &**right],
            GoExpr::Choice {
                condition,
                then,
                otherwise,
                ..
            } => vec![&**condition, &**then, &**otherwise],
            GoExpr::Thunk { value, .. } => vec![&**value],
        }
    }

    /// Every kind of node is drawn, and a prefix operator and a binary one in
    /// every parent, two prefix operators that fuse among them, so the
    /// read-back above reaches each rule of the printer.
    #[test]
    fn the_trees_drawn_hold_every_kind_and_an_operator_in_every_parent() {
        let mut seen = std::collections::BTreeSet::new();
        let mut draw = Draw(SEED);
        for _ in 0..TREES {
            let mut stack = vec![tree(&mut draw, 5)];
            while let Some(node) = stack.pop() {
                seen.insert(kind(&node).to_string());
                if let GoExpr::Unary { op, operand } = &node {
                    if let GoExpr::Unary { op: inner, .. } = &**operand {
                        if op.fuses_with(*inner) {
                            seen.insert("a fused pair".to_string());
                        }
                    }
                }
                for child in children(&node) {
                    if matches!(child, GoExpr::Unary { .. } | GoExpr::Binary { .. }) {
                        seen.insert(format!("{} in {}", kind(child), kind(&node)));
                    }
                    stack.push(child.clone());
                }
            }
        }
        for kind in [
            "Atom",
            "Call",
            "Selector",
            "Index",
            "Slice",
            "Unary",
            "Binary",
            "Choice",
            "Thunk",
            "a fused pair",
        ] {
            assert!(seen.contains(kind), "no {kind} was drawn: {seen:?}");
        }
        for parent in [
            "Call", "Selector", "Index", "Slice", "Unary", "Binary", "Choice", "Thunk",
        ] {
            for child in ["Unary", "Binary"] {
                assert!(
                    seen.contains(&format!("{child} in {parent}")),
                    "no {child} was drawn in a {parent}: {seen:?}"
                );
            }
        }
    }
}
