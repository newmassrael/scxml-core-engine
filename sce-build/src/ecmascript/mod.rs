// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
//! The W3C SCXML ECMAScript datamodel — parsed, then emitted as Lua.
//!
//! SCE runs the ECMAScript datamodel on a Lua interpreter, so every `cond`,
//! `expr` and `<script>` body in a `datamodel="ecmascript"` document has to
//! be translated. This module owns that translation end to end: tokenize
//! (sharing [`crate::forge::expr`]'s lexer), parse to an AST, emit Lua.
//!
//! # Why a parser
//!
//! What this replaced was a pipeline of about twenty-five string-rewriting
//! passes whose entry point could not fail — it returned `String`, so an
//! expression it did not recognise passed through unchanged and Lua then
//! ran it with a different meaning. Two consequences followed from that
//! signature, and both are the reason this module exists:
//!
//! * **Silence.** `Var1 && Var2` with `Var1 = 0` answers `false` in
//!   ECMAScript and `true` in the Lua the rewriter produced, because Lua's
//!   only falsy values are `nil` and `false`. Nothing reported anything;
//!   the guard simply took the other branch. The same held for `!`, for
//!   `? :`, and for every `==` between operands of different types.
//! * **No diagnosis.** A construct outside the accepted subset produced
//!   Lua that failed to compile at runtime, inside a generated string
//!   literal, with no reference back to the SCXML that wrote it.
//!
//! A parser answers both: the meaning of `&&` is decided on the AST where
//! the operand's runtime truthiness can be handed to `_scxml_truthy`, and
//! anything the grammar does not admit is an [`ExprError`] at codegen time
//! naming the construct.
//!
//! # Two dialects, one lexer
//!
//! SCE accepts two expression languages. Forge's Extended SCXML
//! ([`crate::forge::expr`]) is typed, forbids implicit coercion, and is a
//! single expression; W3C ECMAScript is untyped, coerces everywhere, and
//! admits statements. They share [`crate::forge::expr::tokenize_as`] and
//! nothing above it — a shared AST would force the six typed backend
//! emitters to contemplate array literals, `new` and function expressions
//! they can never receive, while a second lexer would let the two disagree
//! about string escapes and numeric forms, which is precisely where the
//! rewriter's bugs lived. `every_forge_expression_also_parses_as_ecmascript`
//! pins the containment the split relies on.

pub mod lua;
pub mod parser;

pub use crate::forge::error::ExprError;

/// An ECMAScript expression.
///
/// Untyped by construction: the W3C datamodel has no type annotations and
/// the Lua interpreter underneath has no static types either, so there is
/// no inference pass between this AST and the emitter.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Numeric literal, carried in its source spelling (`1`, `0x1f`,
    /// `1.5e3`). Lua 5.4 accepts every form ECMAScript writes here.
    Number(String),
    /// String literal with its ECMAScript escapes already decoded, so the
    /// emitter re-encodes for Lua from the actual characters rather than
    /// trying to make one escape grammar pass as another.
    Str(String),
    Bool(bool),
    /// `null` and `undefined` both land here. Lua has one empty value, and
    /// SCE binds `_NULL` and `_UNDEFINED` to it in every engine, so the two
    /// are indistinguishable downstream; see [`lua`] for what that costs.
    Nullish,
    Ident(String),
    /// `this` inside a constructor function. The emitter binds it to the
    /// table the constructor builds.
    This,
    Array(Vec<Expr>),
    Object(Vec<(String, Expr)>),
    Member {
        object: Box<Expr>,
        property: String,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    New {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
    },
    /// `++x` / `x--`. An expression with a side effect: it both stores and
    /// yields a value, which Lua has no expression form for. The emitter
    /// renders it as an immediately-called function.
    Update {
        op: UpdateOp,
        prefix: bool,
        target: Box<Expr>,
    },
    Binary {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    /// `&&` / `||`. Separate from [`Expr::Binary`] because these are the
    /// operators whose result is one of their *operands* rather than a
    /// boolean, which is what makes ECMAScript's falsy set observable.
    Logical {
        op: LogicalOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Conditional {
        condition: Box<Expr>,
        consequent: Box<Expr>,
        alternate: Box<Expr>,
    },
    Function {
        name: Option<String>,
        params: Vec<String>,
        body: Vec<Stmt>,
    },
    /// `x = v`, `x += v`. Also an expression in ECMAScript; the emitter
    /// renders it as a statement where the value is discarded and as an
    /// immediately-called function where it is not.
    Assign {
        /// `Some(op)` for a compound assignment, `None` for plain `=`.
        op: Option<BinOp>,
        target: Box<Expr>,
        value: Box<Expr>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Pos,
    Not,
    BitNot,
    TypeOf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateOp {
    Inc,
    Dec,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalOp {
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    StrictEq,
    StrictNeq,
    /// `==` / `!=`. Legal ECMAScript and legal here — unlike the Forge
    /// dialect, which rejects them because a typed language with no
    /// implicit coercion has nothing for them to mean.
    LooseEq,
    LooseNeq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    UShr,
    InstanceOf,
    In,
}

/// An ECMAScript statement. Reached from `<script>` bodies and from
/// function bodies — the two places the W3C datamodel admits more than a
/// single expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Expr(Expr),
    /// `var a = 1, b;` — one statement, possibly several bindings.
    VarDecl(Vec<(String, Option<Expr>)>),
    If {
        condition: Expr,
        consequent: Vec<Stmt>,
        alternate: Vec<Stmt>,
    },
    While {
        condition: Expr,
        body: Vec<Stmt>,
    },
    For {
        init: Option<Box<Stmt>>,
        test: Option<Expr>,
        update: Option<Expr>,
        body: Vec<Stmt>,
    },
    ForIn {
        /// `true` for `for (var k in o)`, `false` for `for (k in o)`. The
        /// emitter needs it to decide whether the loop variable is local.
        declares: bool,
        name: String,
        object: Expr,
        body: Vec<Stmt>,
    },
    Return(Option<Expr>),
    FunctionDecl {
        name: String,
        params: Vec<String>,
        body: Vec<Stmt>,
    },
    Break,
    Continue,
    /// A lone `;`. Kept as a node rather than dropped so a parse round-trip
    /// is total over what the grammar accepts.
    Empty,
}

/// Translate an ECMAScript expression used for its **value** — `<data
/// expr>`, `<assign expr>`, `<param expr>`, `<log expr>`, `<send>`'s
/// attribute expressions.
///
/// The result is a Lua expression, not a statement, so it can be spliced
/// into the `= (…)` and `return (…)` positions the generated code puts it
/// in.
pub fn to_lua_value(source: &str) -> Result<String, ExprError> {
    let ast = parser::parse_expression(source)?;
    lua::emit_value(&ast)
}

/// Translate an ECMAScript expression used as a **condition** —
/// `transition/@cond`, `<if>`/`<elseif>`.
///
/// The result is a Lua expression of type boolean: ECMAScript truthiness is
/// applied here rather than left to Lua's, which counts `0` and `""` as
/// true.
pub fn to_lua_condition(source: &str) -> Result<String, ExprError> {
    let ast = parser::parse_expression(source)?;
    lua::emit_condition(&ast)
}

/// Translate a `<script>` body: a statement list, emitted as a Lua chunk.
pub fn to_lua_script(source: &str) -> Result<String, ExprError> {
    let stmts = parser::parse_script(source)?;
    lua::emit_script(&stmts)
}

/// Translate an assignment target (`<assign location>`, `<foreach item>`).
///
/// A location is a restricted expression — an identifier or a member/index
/// path rooted at one — and this rejects anything else instead of emitting
/// Lua that would assign to a temporary.
pub fn to_lua_location(source: &str) -> Result<String, ExprError> {
    let ast = parser::parse_expression(source)?;
    lua::emit_location(&ast)
}
