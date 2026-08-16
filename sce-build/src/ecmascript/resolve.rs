// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Resolving the identifiers an expression reads.
//!
//! [`super::lua`] decides *shape*: what Lua an AST node becomes. This
//! decides *names*: whether the node is allowed to name what it names.
//! The two are separate walks on purpose. Shape is answerable from the
//! expression alone, which is why the emitter takes nothing but an AST;
//! a name is answerable only with the document's declarations and the
//! bindings the expression itself introduces, which is state the emitter
//! would have to thread through forty functions to reach a question it
//! never asks.
//!
//! # What is refused
//!
//! A free identifier that nothing declares, in one of two readings:
//!
//! * A **standard global SCE does not install** — `Date`, `parseInt`,
//!   `isNaN`. ECMA-262 defines it, this datamodel does not have it, and
//!   the answer is [`super::builtins`]'s: name the vocabulary that does
//!   exist. Same rule as `words.map(...)`, one AST node up.
//! * **Anything else** — `conut`, `helepr`, `undefinedHelper`. Nothing in
//!   the language or the document gives it a meaning, which in practice
//!   means it is a misspelling, and the document's own declarations are
//!   where the correction comes from.
//!
//! Before this walk both lowered verbatim and reached Lua as globals,
//! where reading an unset global yields `nil` and the arithmetic or the
//! call that followed died with a Lua message naming neither the SCXML
//! nor the identifier.
//!
//! # Why hoisting is modelled
//!
//! `var` in ECMAScript binds for the whole enclosing function regardless
//! of where it is written, so
//!
//! ```text
//! function f() { total = total + 1; var total; }
//! ```
//!
//! declares `total` and a walk that bound names in source order would
//! call the first mention undeclared. [`bindings_hoisted_into`] collects a
//! function body's `var` and `function` declarations before the body is
//! walked, and stops at a nested function boundary — which is exactly
//! where ECMAScript stops.

use std::collections::BTreeSet;

use super::builtins;
use super::scope::DocumentScope;
use super::{BinOp, Expr, Stmt};
use crate::forge::error::ExprError;

/// Resolve every identifier an expression reads.
pub fn expression(expr: &Expr, scope: &DocumentScope) -> Result<(), ExprError> {
    let mut frames = Frames::new(scope);
    frames.expression(expr)
}

/// Resolve every identifier a statement list reads.
///
/// The chunk's own top level is a binding frame: a `<script>`'s `var` is
/// emitted as a datamodel global, so it binds here as well as in the
/// [`DocumentScope`] the model built.
pub fn script(stmts: &[Stmt], scope: &DocumentScope) -> Result<(), ExprError> {
    let mut frames = Frames::new(scope);
    frames.enter(bindings_hoisted_into(stmts));
    for stmt in stmts {
        frames.statement(stmt)?;
    }
    Ok(())
}

struct Frames<'a> {
    scope: &'a DocumentScope,
    lexical: Vec<BTreeSet<String>>,
}

impl<'a> Frames<'a> {
    fn new(scope: &'a DocumentScope) -> Self {
        Self {
            scope,
            lexical: Vec::new(),
        }
    }

    fn enter(&mut self, names: BTreeSet<String>) {
        self.lexical.push(names);
    }

    fn leave(&mut self) {
        self.lexical.pop();
    }

    fn bind(&mut self, name: &str) {
        if let Some(frame) = self.lexical.last_mut() {
            frame.insert(name.to_string());
        }
    }

    fn bound(&self, name: &str) -> bool {
        self.lexical.iter().any(|frame| frame.contains(name))
    }

    /// A name read for its value.
    fn read(&self, name: &str) -> Result<(), ExprError> {
        if self.bound(name) || self.scope.declares(name) {
            return Ok(());
        }
        if let Some(refusal) = builtins::unsupported_global(name) {
            return Err(refusal);
        }
        Err(ExprError::UnknownIdentifier {
            name: name.to_string(),
            candidates: self.scope.candidates_for(name),
        })
    }

    fn expression(&mut self, expr: &Expr) -> Result<(), ExprError> {
        match expr {
            Expr::Number(_) | Expr::Str(_) | Expr::Bool(_) | Expr::Nullish | Expr::This => Ok(()),
            Expr::Ident(name) => self.read(name),
            Expr::Array(items) => {
                for item in items {
                    self.expression(item)?;
                }
                Ok(())
            }
            Expr::Object(props) => {
                // A property key is a name in the object, not in the
                // datamodel: `{ Date: 1 }` declares nothing and reaches
                // for nothing.
                for (_, value) in props {
                    self.expression(value)?;
                }
                Ok(())
            }
            // Likewise a member's property: `x.length` asks `x` for a
            // field, and `length` is answered by the receiver.
            Expr::Member { object, .. } => self.expression(object),
            Expr::Index { object, index } => {
                self.expression(object)?;
                self.expression(index)
            }
            Expr::Call { callee, args } | Expr::New { callee, args } => {
                self.expression(callee)?;
                for arg in args {
                    self.expression(arg)?;
                }
                Ok(())
            }
            Expr::Unary { operand, op } => {
                // §ecma-262-11.4.3: `typeof` on an undeclared name is the
                // one read that does not throw — it answers `"undefined"`.
                // It is how a document asks whether something exists, so
                // refusing it would refuse the question.
                if matches!(op, super::UnaryOp::TypeOf)
                    && matches!(operand.as_ref(), Expr::Ident(_))
                {
                    return Ok(());
                }
                self.expression(operand)
            }
            // `x++` reads before it writes.
            Expr::Update { target, .. } => self.expression(target),
            Expr::Binary { op, left, right } => {
                self.expression(left)?;
                // `x instanceof Array` names the one constructor this
                // datamodel represents, and [`super::lua`] consumes the
                // name rather than reading it. Every other spelling is
                // already refused there as a construct.
                if matches!(op, BinOp::InstanceOf) {
                    return Ok(());
                }
                self.expression(right)
            }
            Expr::Logical { left, right, .. } => {
                self.expression(left)?;
                self.expression(right)
            }
            Expr::Conditional {
                condition,
                consequent,
                alternate,
            } => {
                self.expression(condition)?;
                self.expression(consequent)?;
                self.expression(alternate)
            }
            Expr::Function { name, params, body } => {
                let mut frame = bindings_hoisted_into(body);
                frame.extend(params.iter().cloned());
                // A named function expression can call itself.
                if let Some(name) = name {
                    frame.insert(name.clone());
                }
                self.enter(frame);
                let result = self.statements(body);
                self.leave();
                result
            }
            Expr::Assign { op, target, value } => {
                match target.as_ref() {
                    // §ecma-262-10.2.1: assigning to a name nothing
                    // declares creates it. A compound assignment reads
                    // first, so only a plain `=` gets that licence.
                    Expr::Ident(name) => {
                        if op.is_some() {
                            self.read(name)?;
                        } else {
                            self.bind(name);
                        }
                    }
                    other => self.expression(other)?,
                }
                self.expression(value)
            }
        }
    }

    fn statements(&mut self, stmts: &[Stmt]) -> Result<(), ExprError> {
        for stmt in stmts {
            self.statement(stmt)?;
        }
        Ok(())
    }

    fn statement(&mut self, stmt: &Stmt) -> Result<(), ExprError> {
        match stmt {
            Stmt::Empty | Stmt::Break | Stmt::Continue => Ok(()),
            Stmt::Expr(expr) => self.expression(expr),
            Stmt::VarDecl(bindings) => {
                for (name, init) in bindings {
                    self.bind(name);
                    if let Some(expr) = init {
                        self.expression(expr)?;
                    }
                }
                Ok(())
            }
            Stmt::If {
                condition,
                consequent,
                alternate,
            } => {
                self.expression(condition)?;
                self.statements(consequent)?;
                self.statements(alternate)
            }
            Stmt::While { condition, body } => {
                self.expression(condition)?;
                self.statements(body)
            }
            Stmt::For {
                init,
                test,
                update,
                body,
            } => {
                if let Some(init) = init {
                    self.statement(init)?;
                }
                if let Some(test) = test {
                    self.expression(test)?;
                }
                if let Some(update) = update {
                    self.expression(update)?;
                }
                self.statements(body)
            }
            Stmt::ForIn {
                name, object, body, ..
            } => {
                // The loop variable is written by the loop, whether or not
                // the source spelled `var` — [`super::lua`] emits it as the
                // Lua loop variable either way.
                self.bind(name);
                self.expression(object)?;
                self.statements(body)
            }
            Stmt::Return(expr) => match expr {
                Some(expr) => self.expression(expr),
                None => Ok(()),
            },
            Stmt::FunctionDecl { name, params, body } => {
                let mut frame = bindings_hoisted_into(body);
                frame.extend(params.iter().cloned());
                frame.insert(name.clone());
                self.enter(frame);
                let result = self.statements(body);
                self.leave();
                result
            }
        }
    }
}

/// The names a statement list binds for the whole of its enclosing
/// function — ECMA-262's `var` and function-declaration hoisting.
///
/// Descends through blocks, loops and branches, because `var` is not
/// block-scoped, and stops at a nested function, because that is a new
/// variable environment.
fn bindings_hoisted_into(stmts: &[Stmt]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    hoist(stmts, &mut out);
    out
}

fn hoist(stmts: &[Stmt], out: &mut BTreeSet<String>) {
    for stmt in stmts {
        match stmt {
            Stmt::VarDecl(bindings) => {
                for (name, _) in bindings {
                    out.insert(name.clone());
                }
            }
            Stmt::FunctionDecl { name, .. } => {
                out.insert(name.clone());
            }
            Stmt::If {
                consequent,
                alternate,
                ..
            } => {
                hoist(consequent, out);
                hoist(alternate, out);
            }
            Stmt::While { body, .. } => hoist(body, out),
            Stmt::For { init, body, .. } => {
                if let Some(init) = init {
                    hoist(std::slice::from_ref(init.as_ref()), out);
                }
                hoist(body, out);
            }
            Stmt::ForIn {
                declares,
                name,
                body,
                ..
            } => {
                if *declares {
                    out.insert(name.clone());
                }
                hoist(body, out);
            }
            Stmt::Expr(_) | Stmt::Return(_) | Stmt::Break | Stmt::Continue | Stmt::Empty => {}
        }
    }
}
