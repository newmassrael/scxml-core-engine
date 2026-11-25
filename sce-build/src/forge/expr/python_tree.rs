// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Python a forge expression is written as, as a tree of the Python
//! expressions it is made of, and the one printer that writes such a tree as
//! text.
//!
//! The Python emitter ([`super::emit_python`]) and the expression-shaped
//! spellings of the composed lowering
//! ([`crate::forge::composed_lowering`]) build this tree; nothing is read
//! back out of text. A reduction of an integer to its width is the operators it
//! is made of, and a division between integers is the call of the `lambda`
//! that computes it.
//!
//! The printer decides every parenthesis, at each parent, from the kind of the
//! child and the precedence the Python Language Reference gives its operators
//! ("Expressions", "Operator precedence"): the textbook unparsing rule (Ramsey,
//! "Unparsing Expressions with Prefix and Postfix Operators", 1998). Every
//! binary operator Python has associates to the left. The rules of Python's
//! grammar that precedence does not see are the printer's too:
//!
//! * comparisons chain: `a < b < c` is `a < b and b < c`, never `(a < b) < c`
//!   (the grammar's `comparison` rule takes any number of operators), so a
//!   comparison that is an operand of a comparison is grouped on either side;
//! * `not` is a prefix below the comparisons (the `not_test` rule), so its
//!   operand is bare where it is a comparison and an operand of `-` or of any
//!   operator above it is grouped where it is a `not`;
//! * the body and the condition of a conditional expression are `or_test`s and
//!   its else branch an `expression` (the `conditional_expression` rule), so a
//!   conditional or a `lambda` is grouped in the first two and bare in the last,
//!   which is where it associates;
//! * the body of a `lambda` is an `expression` and stands bare;
//! * a decimal integer literal followed by `.` is read as a real
//!   (`1.real` is `1.` and a stray name, "Floating-point literals"), so a
//!   decimal integer read by an attribute is grouped.
//!
//! Everything else stands bare. The tests hold the printer to the grammar by
//! reading what it prints back with Python's own parser (`ast.parse`) and
//! comparing the tree read with the tree printed.

use std::fmt;

/// A Python expression the emitter writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PyExpr {
    /// Text written as it is that stands as one operand: a literal, a name, a
    /// dotted path, or a fragment its writer made one
    /// ([`super::ExprKind::Raw`]).
    Atom(String),
    /// A whole expression another writer made, whose shape the tree does not
    /// hold: it binds as loosely as any expression can, so it is grouped
    /// wherever a part of an expression is wanted and stands bare where a whole
    /// one is.
    Opaque(String),
    /// `callee(args)`.
    Call {
        callee: Box<PyExpr>,
        args: Vec<PyExpr>,
    },
    /// `base.name`.
    Attribute { base: Box<PyExpr>, name: String },
    /// `base[index]`.
    Subscript {
        base: Box<PyExpr>,
        index: Box<PyExpr>,
    },
    /// A prefix operator over its operand.
    Unary { op: PyUnOp, operand: Box<PyExpr> },
    /// `left op right`.
    Binary {
        op: PyBinOp,
        left: Box<PyExpr>,
        right: Box<PyExpr>,
    },
    /// `body if test else orelse`.
    Conditional {
        body: Box<PyExpr>,
        test: Box<PyExpr>,
        orelse: Box<PyExpr>,
    },
    /// `lambda params: body`.
    Lambda {
        params: Vec<String>,
        body: Box<PyExpr>,
    },
    /// `{key: value, ...}`.
    Dict(Vec<(PyExpr, PyExpr)>),
}

/// The prefix operators the emitter writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PyUnOp {
    /// `-`
    Neg,
    /// `+`
    Pos,
    /// `~`
    Invert,
    /// `not`
    Not,
}

impl PyUnOp {
    fn token(self) -> &'static str {
        match self {
            Self::Neg => "-",
            Self::Pos => "+",
            Self::Invert => "~",
            Self::Not => "not ",
        }
    }
}

/// The binary operators the emitter writes, the comparisons and the Boolean
/// operators among them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PyBinOp {
    Mul,
    Div,
    FloorDiv,
    Mod,
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
    Is,
    IsNot,
    And,
    Or,
}

impl PyBinOp {
    fn token(self) -> &'static str {
        match self {
            Self::Mul => "*",
            Self::Div => "/",
            Self::FloorDiv => "//",
            Self::Mod => "%",
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
            Self::Is => "is",
            Self::IsNot => "is not",
            Self::And => "and",
            Self::Or => "or",
        }
    }

    /// The row of the Python Language Reference's table the operator is on.
    fn precedence(self) -> Precedence {
        match self {
            Self::Mul | Self::Div | Self::FloorDiv | Self::Mod => Precedence::Product,
            Self::Add | Self::Sub => Precedence::Sum,
            Self::Shl | Self::Shr => Precedence::Shift,
            Self::BitAnd => Precedence::BitAnd,
            Self::BitXor => Precedence::BitXor,
            Self::BitOr => Precedence::BitOr,
            Self::Eq
            | Self::Ne
            | Self::Lt
            | Self::Gt
            | Self::Le
            | Self::Ge
            | Self::Is
            | Self::IsNot => Precedence::Compare,
            Self::And => Precedence::And,
            Self::Or => Precedence::Or,
        }
    }
}

/// How tightly an expression binds, loosest first: the rows of the Python
/// Language Reference's precedence table the emitter writes, and above them
/// every expression no operator can split (a literal, a name, a display).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Precedence {
    Lambda,
    Conditional,
    Or,
    And,
    Not,
    Compare,
    BitOr,
    BitXor,
    BitAnd,
    Shift,
    Sum,
    Product,
    Prefix,
    Postfix,
    Primary,
}

/// Where a child stands in its parent, which is all the printer needs to
/// decide whether it is grouped.
#[derive(Debug, Clone, Copy)]
enum Slot {
    /// Between delimiters of the parent's own (an argument, an index, a key or
    /// a value of a display), the body of a `lambda`, the else branch of a
    /// conditional, or the whole expression: where an `expression` stands.
    Enclosed,
    /// What a call calls or a subscript reads.
    Postfix,
    /// What an attribute is read of.
    Attribute,
    /// The operand of `-`, `+` or `~`.
    Prefix,
    /// The operand of `not`.
    Not,
    /// The body or the condition of a conditional, each an `or_test`.
    Conditional,
    /// The left operand of a binary operator.
    Left(PyBinOp),
    /// The right operand of a binary operator.
    Right(PyBinOp),
}

impl PyExpr {
    /// Text that stands as one operand, written as it is.
    pub(crate) fn atom(text: impl Into<String>) -> Self {
        Self::Atom(text.into())
    }

    /// A whole expression another writer made ([`PyExpr::Opaque`]).
    pub(crate) fn opaque(text: impl Into<String>) -> Self {
        Self::Opaque(text.into())
    }

    /// A call of `callee` with `args`.
    pub(crate) fn call(callee: PyExpr, args: Vec<PyExpr>) -> Self {
        Self::Call {
            callee: Box::new(callee),
            args,
        }
    }

    /// A call of the function the dotted path `path` names.
    pub(crate) fn path_call(path: impl Into<String>, args: Vec<PyExpr>) -> Self {
        Self::call(Self::atom(path), args)
    }

    /// The attribute `name` of this value.
    pub(crate) fn attribute(self, name: impl Into<String>) -> Self {
        Self::Attribute {
            base: Box::new(self),
            name: name.into(),
        }
    }

    /// The method `name` of this value called with `args`.
    pub(crate) fn method(self, name: impl Into<String>, args: Vec<PyExpr>) -> Self {
        Self::call(self.attribute(name), args)
    }

    /// This value subscripted by `index`.
    pub(crate) fn subscript(self, index: PyExpr) -> Self {
        Self::Subscript {
            base: Box::new(self),
            index: Box::new(index),
        }
    }

    /// The prefix operator `op` over this value.
    pub(crate) fn prefixed(self, op: PyUnOp) -> Self {
        Self::Unary {
            op,
            operand: Box::new(self),
        }
    }

    /// `left op right`.
    pub(crate) fn binary(op: PyBinOp, left: PyExpr, right: PyExpr) -> Self {
        Self::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    /// `body if test else orelse`.
    pub(crate) fn conditional(body: PyExpr, test: PyExpr, orelse: PyExpr) -> Self {
        Self::Conditional {
            body: Box::new(body),
            test: Box::new(test),
            orelse: Box::new(orelse),
        }
    }

    /// `lambda params: body`.
    pub(crate) fn lambda(params: &[&str], body: PyExpr) -> Self {
        Self::Lambda {
            params: params.iter().map(|param| (*param).to_string()).collect(),
            body: Box::new(body),
        }
    }

    /// `{key: value, ...}`.
    pub(crate) fn dict(entries: Vec<(PyExpr, PyExpr)>) -> Self {
        Self::Dict(entries)
    }

    fn precedence(&self) -> Precedence {
        match self {
            Self::Opaque(_) | Self::Lambda { .. } => Precedence::Lambda,
            Self::Conditional { .. } => Precedence::Conditional,
            Self::Binary { op, .. } => op.precedence(),
            Self::Unary {
                op: PyUnOp::Not, ..
            } => Precedence::Not,
            Self::Unary { .. } => Precedence::Prefix,
            Self::Call { .. } | Self::Attribute { .. } | Self::Subscript { .. } => {
                Precedence::Postfix
            }
            Self::Atom(_) | Self::Dict(_) => Precedence::Primary,
        }
    }

    /// Whether this is a decimal integer literal, which a `.` after it would
    /// make a real.
    fn is_decimal_integer(&self) -> bool {
        match self {
            Self::Atom(text) => {
                text.starts_with(|c: char| c.is_ascii_digit())
                    && text.chars().all(|c| c.is_ascii_digit() || c == '_')
            }
            _ => false,
        }
    }

    /// Whether this expression is grouped where it stands at `slot`.
    fn needs_parentheses(&self, slot: Slot) -> bool {
        let own = self.precedence();
        match slot {
            Slot::Enclosed => false,
            Slot::Postfix => own < Precedence::Postfix,
            Slot::Attribute => own < Precedence::Postfix || self.is_decimal_integer(),
            Slot::Prefix => own < Precedence::Prefix,
            Slot::Not => own < Precedence::Not,
            Slot::Conditional => own < Precedence::Or,
            Slot::Left(op) => {
                own < op.precedence()
                    || (own == Precedence::Compare && op.precedence() == Precedence::Compare)
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
            Self::Atom(text) | Self::Opaque(text) => out.push_str(text),
            Self::Call { callee, args } => {
                callee.write(Slot::Postfix, out);
                out.push('(');
                write_list(args, out);
                out.push(')');
            }
            Self::Attribute { base, name } => {
                base.write(Slot::Attribute, out);
                out.push('.');
                out.push_str(name);
            }
            Self::Subscript { base, index } => {
                base.write(Slot::Postfix, out);
                out.push('[');
                index.write(Slot::Enclosed, out);
                out.push(']');
            }
            Self::Unary { op, operand } => {
                out.push_str(op.token());
                let slot = if *op == PyUnOp::Not {
                    Slot::Not
                } else {
                    Slot::Prefix
                };
                operand.write(slot, out);
            }
            Self::Binary { op, left, right } => {
                left.write(Slot::Left(*op), out);
                out.push(' ');
                out.push_str(op.token());
                out.push(' ');
                right.write(Slot::Right(*op), out);
            }
            Self::Conditional { body, test, orelse } => {
                body.write(Slot::Conditional, out);
                out.push_str(" if ");
                test.write(Slot::Conditional, out);
                out.push_str(" else ");
                orelse.write(Slot::Enclosed, out);
            }
            Self::Lambda { params, body } => {
                out.push_str("lambda");
                if !params.is_empty() {
                    out.push(' ');
                    out.push_str(&params.join(", "));
                }
                out.push_str(": ");
                body.write(Slot::Enclosed, out);
            }
            Self::Dict(entries) => {
                out.push('{');
                for (i, (key, value)) in entries.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    key.write(Slot::Enclosed, out);
                    out.push_str(": ");
                    value.write(Slot::Enclosed, out);
                }
                out.push('}');
            }
        }
    }
}

fn write_list(items: &[PyExpr], out: &mut String) {
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        item.write(Slot::Enclosed, out);
    }
}

/// The whole expression, as the emitter hands it on.
impl fmt::Display for PyExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = String::new();
        self.write(Slot::Enclosed, &mut out);
        f.write_str(&out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str) -> PyExpr {
        PyExpr::atom(text)
    }

    fn op(op: PyBinOp, left: PyExpr, right: PyExpr) -> PyExpr {
        PyExpr::binary(op, left, right)
    }

    fn choice() -> PyExpr {
        PyExpr::conditional(at("a"), at("c"), at("b"))
    }

    /// An operator groups an operand that binds more loosely than itself, and a
    /// right operand that binds as tightly, every binary operator of Python
    /// associating to the left; a prefix operator groups an operand that is no
    /// prefix or postfix expression, and a postfix read one that is no postfix
    /// expression.
    #[test]
    fn an_operand_is_grouped_by_precedence_and_left_association() {
        let (a, b, c) = (at("a"), at("b"), at("c"));
        let sum = op(PyBinOp::Add, a.clone(), b.clone());
        assert_eq!(
            op(PyBinOp::Mul, sum.clone(), c.clone()).to_string(),
            "(a + b) * c"
        );
        assert_eq!(
            op(
                PyBinOp::Sub,
                op(PyBinOp::Sub, a.clone(), b.clone()),
                c.clone()
            )
            .to_string(),
            "a - b - c"
        );
        assert_eq!(
            op(
                PyBinOp::Sub,
                a.clone(),
                op(PyBinOp::Sub, b.clone(), c.clone())
            )
            .to_string(),
            "a - (b - c)"
        );
        // `&` binds looser than `+` and tighter than a comparison in Python.
        assert_eq!(
            op(PyBinOp::BitAnd, sum.clone(), at("0xFF")).to_string(),
            "a + b & 0xFF"
        );
        assert_eq!(
            op(
                PyBinOp::Eq,
                op(PyBinOp::BitAnd, a.clone(), b.clone()),
                c.clone()
            )
            .to_string(),
            "a & b == c"
        );
        assert_eq!(
            op(
                PyBinOp::Sub,
                op(PyBinOp::BitAnd, sum.clone(), at("0xFF")),
                at("0x80")
            )
            .to_string(),
            "(a + b & 0xFF) - 0x80"
        );
        assert_eq!(
            op(
                PyBinOp::Or,
                a.clone(),
                op(PyBinOp::And, b.clone(), c.clone())
            )
            .to_string(),
            "a or b and c"
        );
        assert_eq!(sum.clone().prefixed(PyUnOp::Neg).to_string(), "-(a + b)");
        assert_eq!(
            a.clone().attribute("f").prefixed(PyUnOp::Neg).to_string(),
            "-a.f"
        );
        assert_eq!(
            sum.clone().method("encode", Vec::new()).to_string(),
            "(a + b).encode()"
        );
        assert_eq!(
            PyExpr::path_call("len", vec![sum.clone()]).to_string(),
            "len(a + b)"
        );
        assert_eq!(sum.subscript(c).to_string(), "(a + b)[c]");
        assert_eq!(
            a.prefixed(PyUnOp::Neg).prefixed(PyUnOp::Neg).to_string(),
            "--a"
        );
    }

    /// Comparisons chain in Python, so one under another is grouped on either
    /// side, where precedence alone would leave the left one bare.
    #[test]
    fn a_comparison_is_grouped_under_a_comparison() {
        let test = op(PyBinOp::Ge, at("n"), at("0"));
        assert_eq!(
            op(PyBinOp::Eq, test.clone(), op(PyBinOp::Gt, at("d"), at("0"))).to_string(),
            "(n >= 0) == (d > 0)"
        );
        assert_eq!(
            op(PyBinOp::IsNot, at("x"), at("None")).to_string(),
            "x is not None"
        );
        assert_eq!(
            op(PyBinOp::And, test.clone(), test).to_string(),
            "n >= 0 and n >= 0"
        );
    }

    /// `not` sits below the comparisons: a comparison under it stands bare, and
    /// a `not` under a comparison, an arithmetic operator or `-` is grouped.
    #[test]
    fn not_is_a_prefix_below_the_comparisons() {
        let test = op(PyBinOp::Eq, at("a"), at("b"));
        assert_eq!(test.prefixed(PyUnOp::Not).to_string(), "not a == b");
        let negated = at("a").prefixed(PyUnOp::Not);
        assert_eq!(
            op(PyBinOp::Eq, negated.clone(), at("b")).to_string(),
            "(not a) == b"
        );
        assert_eq!(
            op(PyBinOp::And, negated.clone(), at("b")).to_string(),
            "not a and b"
        );
        assert_eq!(
            negated.clone().prefixed(PyUnOp::Neg).to_string(),
            "-(not a)"
        );
        assert_eq!(negated.prefixed(PyUnOp::Not).to_string(), "not not a");
        assert_eq!(
            op(PyBinOp::And, at("a"), at("b"))
                .prefixed(PyUnOp::Not)
                .to_string(),
            "not (a and b)"
        );
    }

    /// A conditional's body and condition are grouped where they are
    /// conditionals or `lambda`s; its else branch, a `lambda`'s body and an
    /// argument stand bare whatever they are.
    #[test]
    fn a_conditional_associates_through_its_else_branch() {
        assert_eq!(
            PyExpr::conditional(choice(), choice(), choice()).to_string(),
            "(a if c else b) if (a if c else b) else a if c else b"
        );
        assert_eq!(
            op(PyBinOp::Add, choice(), at("d")).to_string(),
            "(a if c else b) + d"
        );
        assert_eq!(choice().attribute("f").to_string(), "(a if c else b).f");
        assert_eq!(
            PyExpr::path_call(
                "sce_algorithm.otherwise",
                vec![at("x"), PyExpr::lambda(&[], choice())]
            )
            .to_string(),
            "sce_algorithm.otherwise(x, lambda: a if c else b)"
        );
        let divide = PyExpr::lambda(
            &["n", "d"],
            PyExpr::conditional(
                op(PyBinOp::FloorDiv, at("n"), at("d")),
                op(
                    PyBinOp::Eq,
                    op(PyBinOp::Ge, at("n"), at("0")),
                    op(PyBinOp::Gt, at("d"), at("0")),
                ),
                op(PyBinOp::FloorDiv, at("n").prefixed(PyUnOp::Neg), at("d")).prefixed(PyUnOp::Neg),
            ),
        );
        assert_eq!(
            PyExpr::call(divide, vec![at("a"), op(PyBinOp::Add, at("b"), at("c"))]).to_string(),
            "(lambda n, d: n // d if (n >= 0) == (d > 0) else -(-n // d))(a, b + c)"
        );
    }

    /// A decimal integer read by an attribute would read as a real, so it is
    /// grouped; a hexadecimal one, a name and a call are not.
    #[test]
    fn a_decimal_integer_read_by_an_attribute_is_grouped() {
        assert_eq!(at("2").attribute("real").to_string(), "(2).real");
        assert_eq!(at("0x10").attribute("real").to_string(), "0x10.real");
        assert_eq!(at("x").attribute("real").to_string(), "x.real");
        assert_eq!(at("2").subscript(at("0")).to_string(), "2[0]");
    }

    /// A fragment of unknown shape is grouped wherever a part is wanted and
    /// bare where a whole expression stands.
    #[test]
    fn an_opaque_fragment_is_grouped_wherever_a_part_is_wanted() {
        let fragment = PyExpr::opaque("None if v is None else v");
        assert_eq!(
            PyExpr::call(PyExpr::lambda(&["v"], fragment.clone()), vec![at("x.f")]).to_string(),
            "(lambda v: None if v is None else v)(x.f)"
        );
        assert_eq!(
            fragment.clone().attribute("f").to_string(),
            "(None if v is None else v).f"
        );
        assert_eq!(
            PyExpr::dict(vec![(at("'k'"), fragment)]).to_string(),
            "{'k': None if v is None else v}"
        );
    }

    // ── The printer read back by Python ────────────────────────────

    /// The canonical form of a tree, in the words the reader below writes the
    /// tree Python's parser read: the printer's text means what the printed
    /// tree does exactly where the two are equal, grouping dropped.
    fn canonical(tree: &PyExpr) -> String {
        match tree {
            PyExpr::Atom(text) | PyExpr::Opaque(text) => text.clone(),
            PyExpr::Call { callee, args } => {
                let mut parts = vec![canonical(callee)];
                parts.extend(args.iter().map(canonical));
                format!("(call {})", parts.join(" "))
            }
            PyExpr::Attribute { base, name } => format!("(. {} {name})", canonical(base)),
            PyExpr::Subscript { base, index } => {
                format!("([] {} {})", canonical(base), canonical(index))
            }
            PyExpr::Unary { op, operand } => {
                format!("({} {})", op.token().trim_end(), canonical(operand))
            }
            PyExpr::Binary { op, left, right } => {
                format!("({} {} {})", op.token(), canonical(left), canonical(right))
            }
            PyExpr::Conditional { body, test, orelse } => format!(
                "(if {} {} {})",
                canonical(body),
                canonical(test),
                canonical(orelse)
            ),
            PyExpr::Lambda { params, body } => {
                format!("(lambda [{}] {})", params.join(","), canonical(body))
            }
            PyExpr::Dict(entries) => {
                let parts: Vec<String> = entries
                    .iter()
                    .map(|(key, value)| format!("{} {}", canonical(key), canonical(value)))
                    .collect();
                format!("(dict {})", parts.join(" "))
            }
        }
    }

    /// Reads one Python expression a line with `ast.parse` and writes the tree
    /// it read in the words of [`canonical`], a chained comparison as the
    /// chain it is, which no printed tree is. Every line is read before any is
    /// written, so neither side waits on a full pipe.
    const READER: &str = r#"
import ast, sys
OPS = {
    ast.Add: '+', ast.Sub: '-', ast.Mult: '*', ast.Div: '/', ast.FloorDiv: '//',
    ast.Mod: '%', ast.LShift: '<<', ast.RShift: '>>', ast.BitAnd: '&',
    ast.BitXor: '^', ast.BitOr: '|', ast.Eq: '==', ast.NotEq: '!=', ast.Lt: '<',
    ast.Gt: '>', ast.LtE: '<=', ast.GtE: '>=', ast.Is: 'is', ast.IsNot: 'is not',
    ast.And: 'and', ast.Or: 'or', ast.USub: '-', ast.UAdd: '+', ast.Invert: '~',
    ast.Not: 'not',
}
def read(n):
    t = type(n)
    if t is ast.Name:
        return n.id
    if t is ast.Constant:
        return repr(n.value)
    if t is ast.BinOp:
        return '(%s %s %s)' % (OPS[type(n.op)], read(n.left), read(n.right))
    if t is ast.BoolOp:
        acc = read(n.values[0])
        for v in n.values[1:]:
            acc = '(%s %s %s)' % (OPS[type(n.op)], acc, read(v))
        return acc
    if t is ast.Compare:
        if len(n.ops) != 1:
            return '(chain %s)' % ' '.join(OPS[type(o)] for o in n.ops)
        return '(%s %s %s)' % (OPS[type(n.ops[0])], read(n.left), read(n.comparators[0]))
    if t is ast.UnaryOp:
        return '(%s %s)' % (OPS[type(n.op)], read(n.operand))
    if t is ast.IfExp:
        return '(if %s %s %s)' % (read(n.body), read(n.test), read(n.orelse))
    if t is ast.Lambda:
        return '(lambda [%s] %s)' % (','.join(a.arg for a in n.args.args), read(n.body))
    if t is ast.Call:
        return '(call %s)' % ' '.join([read(n.func)] + [read(a) for a in n.args])
    if t is ast.Attribute:
        return '(. %s %s)' % (read(n.value), n.attr)
    if t is ast.Subscript:
        s = n.slice
        if type(s).__name__ == 'Index':
            s = s.value
        return '([] %s %s)' % (read(n.value), read(s))
    if t is ast.Dict:
        return '(dict %s)' % ' '.join('%s %s' % (read(k), read(v)) for k, v in zip(n.keys, n.values))
    return '(unread %s)' % t.__name__
for line in sys.stdin.read().splitlines():
    try:
        print(read(ast.parse(line, mode='eval').body))
    except SyntaxError as e:
        print('(syntax-error %s)' % e.msg)
"#;

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

    const OPERATORS: [PyBinOp; 21] = [
        PyBinOp::Mul,
        PyBinOp::Div,
        PyBinOp::FloorDiv,
        PyBinOp::Mod,
        PyBinOp::Add,
        PyBinOp::Sub,
        PyBinOp::Shl,
        PyBinOp::Shr,
        PyBinOp::BitAnd,
        PyBinOp::BitXor,
        PyBinOp::BitOr,
        PyBinOp::Eq,
        PyBinOp::Ne,
        PyBinOp::Lt,
        PyBinOp::Gt,
        PyBinOp::Le,
        PyBinOp::Ge,
        PyBinOp::Is,
        PyBinOp::IsNot,
        PyBinOp::And,
        PyBinOp::Or,
    ];

    const PREFIXES: [PyUnOp; 4] = [PyUnOp::Neg, PyUnOp::Pos, PyUnOp::Invert, PyUnOp::Not];

    /// The seed every drawing starts from.
    const SEED: u64 = 0x9E37_79B9_7F4A_7C15;

    /// How many trees a drawing holds.
    const TREES: usize = 4000;

    /// A tree of every node kind but [`PyExpr::Opaque`], which Python reads as
    /// whatever its text is, at most `depth` deep; its leaves are written as
    /// Python's `repr` writes them back, a decimal integer among them.
    fn tree(draw: &mut Draw, depth: u32) -> PyExpr {
        if depth == 0 || draw.below(6) == 0 {
            return at(draw.pick(&["a", "b", "x", "None", "True", "2", "7", "'s'"]));
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
                PyExpr::call(callee, args)
            }
            1 => {
                let base = tree(draw, d);
                base.attribute(draw.pick(&["f", "real"]))
            }
            2 => {
                let base = tree(draw, d);
                base.subscript(tree(draw, d))
            }
            3 | 4 => {
                let prefix = PREFIXES[draw.below(PREFIXES.len())];
                tree(draw, d).prefixed(prefix)
            }
            5..=9 => {
                let operator = OPERATORS[draw.below(OPERATORS.len())];
                let left = tree(draw, d);
                let right = tree(draw, d);
                PyExpr::binary(operator, left, right)
            }
            10 | 11 => {
                let body = tree(draw, d);
                let test = tree(draw, d);
                let orelse = tree(draw, d);
                PyExpr::conditional(body, test, orelse)
            }
            12 => {
                let params: &[&str] = match draw.below(3) {
                    0 => &[],
                    1 => &["n"],
                    _ => &["n", "d"],
                };
                PyExpr::lambda(params, tree(draw, d))
            }
            _ => {
                let count = draw.below(3);
                let entries = (0..count).map(|_| (tree(draw, d), tree(draw, d))).collect();
                PyExpr::dict(entries)
            }
        }
    }

    /// Every tree the printer is handed is read back as itself by Python's own
    /// parser: every parenthesis the grammar needs is printed, and none the
    /// printer leaves out changes what is read. Skipped, by the toolchain's one
    /// rule, where no `python3` is found.
    #[test]
    fn what_the_printer_writes_reads_back_as_the_tree_it_was_handed() {
        use std::io::Write;
        use std::process::{Command, Stdio};
        let Some(python) = crate::toolchain::require_or_skip(
            "python3",
            "read the Python the expression printer writes back with Python's parser",
        ) else {
            return;
        };
        let mut draw = Draw(SEED);
        let trees: Vec<PyExpr> = (0..TREES).map(|_| tree(&mut draw, 5)).collect();
        let texts: Vec<String> = trees.iter().map(ToString::to_string).collect();
        let mut child = Command::new(python)
            .args(["-W", "ignore", "-c", READER])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("python3 starts");
        {
            let mut stdin = child.stdin.take().expect("python3 reads its input");
            for text in &texts {
                writeln!(stdin, "{text}").expect("python3 takes a line");
            }
        }
        let output = child.wait_with_output().expect("python3 ends");
        assert!(output.status.success(), "python3 read every line");
        let read = String::from_utf8(output.stdout).expect("python3 writes UTF-8");
        let read: Vec<&str> = read.lines().collect();
        assert_eq!(read.len(), trees.len(), "one reading for every tree");
        for ((tree, text), read) in trees.iter().zip(&texts).zip(read) {
            assert_eq!(read, canonical(tree), "`{text}` read back");
        }
    }

    /// The kind of a node, as its variant is named.
    fn kind(tree: &PyExpr) -> &'static str {
        match tree {
            PyExpr::Atom(_) => "Atom",
            PyExpr::Opaque(_) => "Opaque",
            PyExpr::Call { .. } => "Call",
            PyExpr::Attribute { .. } => "Attribute",
            PyExpr::Subscript { .. } => "Subscript",
            PyExpr::Unary {
                op: PyUnOp::Not, ..
            } => "Not",
            PyExpr::Unary { .. } => "Unary",
            PyExpr::Binary { op, .. } if op.precedence() == Precedence::Compare => "Compare",
            PyExpr::Binary { .. } => "Binary",
            PyExpr::Conditional { .. } => "Conditional",
            PyExpr::Lambda { .. } => "Lambda",
            PyExpr::Dict(_) => "Dict",
        }
    }

    /// The nodes directly inside `tree`.
    fn children(tree: &PyExpr) -> Vec<&PyExpr> {
        match tree {
            PyExpr::Atom(_) | PyExpr::Opaque(_) => Vec::new(),
            PyExpr::Call { callee, args } => std::iter::once(&**callee).chain(args).collect(),
            PyExpr::Attribute { base, .. } => vec![&**base],
            PyExpr::Subscript { base, index } => vec![&**base, &**index],
            PyExpr::Unary { operand, .. } => vec![&**operand],
            PyExpr::Binary { left, right, .. } => vec![&**left, &**right],
            PyExpr::Conditional { body, test, orelse } => vec![&**body, &**test, &**orelse],
            PyExpr::Lambda { body, .. } => vec![&**body],
            PyExpr::Dict(entries) => entries
                .iter()
                .flat_map(|(key, value)| [key, value])
                .collect(),
        }
    }

    /// Every kind of node is drawn, and a comparison, a `not`, a conditional
    /// and a `lambda` in every parent, so the read-back above reaches each rule
    /// of the printer; a decimal integer is drawn under an attribute.
    #[test]
    fn the_trees_drawn_hold_every_kind_in_every_parent() {
        let mut seen = std::collections::BTreeSet::new();
        let mut draw = Draw(SEED);
        for _ in 0..TREES {
            let mut stack = vec![tree(&mut draw, 5)];
            while let Some(node) = stack.pop() {
                seen.insert(kind(&node).to_string());
                for child in children(&node) {
                    let child_kind = kind(child);
                    if matches!(child_kind, "Compare" | "Not" | "Conditional" | "Lambda") {
                        seen.insert(format!("{child_kind} in {}", kind(&node)));
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
            "Attribute",
            "Subscript",
            "Not",
            "Unary",
            "Compare",
            "Binary",
            "Conditional",
            "Lambda",
            "Dict",
            "integer in Attribute",
        ] {
            assert!(seen.contains(kind), "no {kind} was drawn: {seen:?}");
        }
        for child in ["Compare", "Not", "Conditional", "Lambda"] {
            for parent in [
                "Call",
                "Attribute",
                "Subscript",
                "Not",
                "Unary",
                "Compare",
                "Binary",
                "Conditional",
                "Lambda",
                "Dict",
            ] {
                assert!(
                    seen.contains(&format!("{child} in {parent}")),
                    "no {child} was drawn in a {parent}: {seen:?}"
                );
            }
        }
    }
}
