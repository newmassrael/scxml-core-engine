// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Which integer operations of an algorithm body can fail — the analysis the
// integer arithmetic contract (E12) rests on.
//
// Under that contract an integer `+ - *` or unary `-` whose result leaves its
// type, a `/ %` by zero, a signed `MIN / -1`, and a value stored where its
// type cannot hold it — a local, a record field, the returned value or a
// call's argument of a narrower integer type — are failures of the
// algorithm, not values: a wrapped result is a wrong answer delivered in
// silence, which is the one outcome the contract exists to end. Bitwise
// operations and shifts keep the declared width's two's-complement meaning
// and never fail.
//
// An algorithm that can reach such an operation must say so in its
// signature; one that cannot keeps an API that does not fail. Which is which
// is not left to a pattern the author has to match: this module proves it,
// by interval analysis over the body. Each integer value is tracked as the
// range it can hold — a parameter as its whole type, a local from its
// initialiser — and narrowed by the conditions that guard it (inside
// `while i < 8`, `i` is at most 7). A loop is iterated until its ranges stop
// growing; a range still growing after a few rounds is widened to its whole
// type, which the guarding condition then narrows again. What remains is an
// over-approximation: every hazard it reports is an operation the analysis
// could not prove safe, and every one it does not report is proven safe.
//
// It judges each expression against the same typed tree the emitters lower
// (`expr::resolve` + `expr::infer_types` over the algorithm's own scope), so
// the width an operation is judged at is the width it is emitted at.

use std::collections::BTreeMap;
use std::ops::Range;

use crate::attribute_spelling::AttributeSpelling;
use crate::forge::expr::{
    infer_types, judge_into, resolve, BinOp, Expected, ExprKind, TypedExpr, UnaryOp,
};
use crate::forge::model::{AlgorithmStmt, AlgorithmValueType};
use crate::forge::types::{InferredType, TypeCtx};

/// Why an operation can fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HazardKind {
    /// `+ - *` or unary `-` whose result can leave its type.
    Overflow,
    /// `/ %` whose divisor can be zero.
    DivideByZero,
    /// A signed `/ %` that can divide its type's minimum by `-1`, whose
    /// quotient its type cannot hold.
    MinDividedByMinusOne,
    /// A value stored in a place whose integer type cannot hold every value
    /// it can take — the place's type is the hazard's `ty`.
    DoesNotFit,
}

/// One operation the analysis could not prove safe.
#[derive(Debug, Clone)]
pub struct IntHazard {
    /// The expression it sits in, as written.
    pub expr: String,
    /// Where in `expr` the operation is written, when the parser knows.
    pub span: Option<Range<usize>>,
    /// The attribute `expr` was read from, so a refusal lands on the
    /// operation's own row and column.
    pub spelling: Option<AttributeSpelling>,
    pub kind: HazardKind,
    /// The type the operation computes in; for [`HazardKind::DoesNotFit`],
    /// the type of the place the value is stored in.
    pub ty: InferredType,
}

/// Every operation of `body` that can fail, given `params` (name and type),
/// `ret` (the declared return type) and `ctx`, the scope the algorithm's
/// expressions are typed in.
///
/// `None` when an expression of `body` is one the typed pipeline refuses: the
/// body is then not judged. That expression has a refusal of its own, raised
/// where it is lowered, and a guard the analysis cannot read would otherwise
/// have it report an operation the guard bounds — naming the wrong failure
/// (a misspelt bound read as an overflow).
pub fn hazards(
    params: &[(String, AlgorithmValueType)],
    ret: Option<&AlgorithmValueType>,
    body: &[AlgorithmStmt],
    ctx: &TypeCtx<'_>,
) -> Option<Vec<IntHazard>> {
    let mut analysis = Analysis {
        ctx,
        ret: ret
            .and_then(AlgorithmValueType::scalar)
            .map(InferredType::from_sce_type),
        found: Vec::new(),
        recording: true,
        untyped: std::cell::Cell::new(false),
    };
    let mut env = Env::default();
    for (name, ty) in params {
        if let Some(range) = scalar_range(ty) {
            env.vars.insert(name.clone(), range);
        }
    }
    analysis.block(body, &mut env);
    (!analysis.untyped.get()).then_some(analysis.found)
}

/// A closed integer range. Every width SCE declares fits in `i128` with room
/// for the product of two of its values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Interval {
    lo: i128,
    hi: i128,
}

impl Interval {
    fn point(v: i128) -> Self {
        Self { lo: v, hi: v }
    }

    fn join(self, other: Self) -> Self {
        Self {
            lo: self.lo.min(other.lo),
            hi: self.hi.max(other.hi),
        }
    }

    fn contains(self, v: i128) -> bool {
        self.lo <= v && v <= self.hi
    }

    fn within(self, outer: Self) -> bool {
        outer.lo <= self.lo && self.hi <= outer.hi
    }

    /// `self` limited to `outer` — the values an operation that did not fail
    /// can have produced.
    fn clamp(self, outer: Self) -> Self {
        Self {
            lo: self.lo.max(outer.lo),
            hi: self.hi.min(outer.hi),
        }
    }
}

/// The values an integer type holds; `None` for any other type.
fn type_range(ty: InferredType) -> Option<Interval> {
    ty.int_bounds().map(|(lo, hi)| Interval { lo, hi })
}

fn scalar_range(ty: &AlgorithmValueType) -> Option<Interval> {
    ty.scalar()
        .and_then(|t| type_range(InferredType::from_sce_type(t)))
}

/// The range each tracked name holds at one point of the body, or none at a
/// point no execution reaches.
#[derive(Debug, Clone, Default, PartialEq)]
struct Env {
    vars: BTreeMap<String, Interval>,
    /// Names a guard has shown to be non-zero. A range cannot say "any value
    /// but 0" of a signed type, and `if (b !== 0) … a / b` is the idiom a
    /// divisor is guarded by, so the fact is kept beside the range.
    nonzero: std::collections::BTreeSet<String>,
    unreachable: bool,
}

impl Env {
    fn dead() -> Self {
        Self {
            unreachable: true,
            ..Self::default()
        }
    }

    /// The ranges either of two paths can leave. A name only one path tracks
    /// is dropped: after the join it is known only by its type.
    fn join(&self, other: &Self) -> Self {
        if self.unreachable {
            return other.clone();
        }
        if other.unreachable {
            return self.clone();
        }
        let vars = self
            .vars
            .iter()
            .filter_map(|(k, a)| other.vars.get(k).map(|b| (k.clone(), a.join(*b))))
            .collect();
        Self {
            vars,
            nonzero: self.nonzero.intersection(&other.nonzero).cloned().collect(),
            unreachable: false,
        }
    }
}

struct Analysis<'c, 'a> {
    ctx: &'c TypeCtx<'a>,
    /// The declared return type, when it is a scalar — the place a returned
    /// value is stored in.
    ret: Option<InferredType>,
    found: Vec<IntHazard>,
    /// Off while a loop is iterated towards its fixed point, whose early
    /// rounds see narrower ranges than the loop really reaches; on for the
    /// one pass over the stable ranges, which is the one that reports.
    recording: bool,
    /// Whether an expression the typed pipeline refuses was met — the body is
    /// then not judged ([`hazards`]).
    untyped: std::cell::Cell<bool>,
}

/// Rounds a loop is iterated before its still-growing ranges are widened.
const ROUNDS_BEFORE_WIDENING: usize = 3;

/// Rounds a widened loop head is narrowed by, from its post-fixpoint. One
/// recovers a bound its body's guard imposes; a second lets that bound reach
/// a value computed from the first.
const ROUNDS_OF_NARROWING: usize = 2;

impl Analysis<'_, '_> {
    fn block(&mut self, stmts: &[AlgorithmStmt], env: &mut Env) {
        for stmt in stmts {
            if env.unreachable {
                return;
            }
            self.stmt(stmt, env);
        }
    }

    fn stmt(&mut self, stmt: &AlgorithmStmt, env: &mut Env) {
        match stmt {
            AlgorithmStmt::Var {
                name,
                sce_type,
                init,
                init_spelling,
                ..
            } => {
                let place = sce_type.scalar().map(InferredType::from_sce_type);
                let value = init
                    .as_deref()
                    .and_then(|e| self.stored(e, init_spelling.as_ref(), env, place));
                self.store(env, name, scalar_range(sce_type), value);
            }
            AlgorithmStmt::RecordVar { name, fields, .. } => {
                for field in fields {
                    let key = format!("{name}.{}", field.name);
                    let place = self.declared_type(&key);
                    let value = self.stored(&field.expr, field.expr_spelling.as_ref(), env, place);
                    let slot = self.declared_range(&key);
                    self.store(env, &key, slot, value);
                }
            }
            AlgorithmStmt::Assign {
                target,
                expr,
                expr_spelling,
                ..
            } => {
                let key = target.trim().to_string();
                let place = self.declared_type(&key);
                let value = self.stored(expr, expr_spelling.as_ref(), env, place);
                let slot = self.declared_range(&key);
                self.store(env, &key, slot, value);
            }
            AlgorithmStmt::Append {
                target,
                expr,
                expr_spelling,
                ..
            } => {
                // A `list<T>` element lands in `T`; a byte buffer takes a
                // `uint8` or a `bytes`, and a wider value is refused where
                // the append is lowered.
                let place = match self.declared_type(target.trim()) {
                    Some(InferredType::List(elem)) => Some(elem.element_type()),
                    _ => None,
                };
                self.stored(expr, expr_spelling.as_ref(), env, place);
            }
            AlgorithmStmt::Return {
                expr,
                expr_spelling,
            } => {
                if let Some(e) = expr {
                    self.stored(e, expr_spelling.as_ref(), env, self.ret);
                }
                *env = Env::dead();
            }
            AlgorithmStmt::Call { target, args, .. } => {
                // Each argument lands in its parameter, as an argument of the
                // expression form does ([`Self::eval`]).
                let params = self
                    .ctx
                    .funcs
                    .get(target.trim())
                    .filter(|sig| sig.host_only.is_none())
                    .map(|sig| sig.params.clone())
                    .unwrap_or_default();
                for (i, arg) in args.iter().enumerate() {
                    let place = params.get(i).copied();
                    self.stored(&arg.expr, arg.spelling.as_ref(), env, place);
                }
            }
            AlgorithmStmt::If {
                cond,
                cond_spelling,
                then_body,
                else_body,
            } => {
                let tree = self.typed(cond);
                if let Some(tree) = &tree {
                    self.eval(tree, cond, cond_spelling.as_ref(), env);
                }
                let mut then_env = self.narrowed(tree.as_ref(), env, true);
                self.block(then_body, &mut then_env);
                let mut else_env = self.narrowed(tree.as_ref(), env, false);
                if let Some(body) = else_body {
                    self.block(body, &mut else_env);
                }
                *env = then_env.join(&else_env);
            }
            AlgorithmStmt::While {
                cond,
                cond_spelling,
                body,
                ..
            } => {
                let tree = self.typed(cond);
                let head = self.fixed_point(env, |this, head| {
                    let mut inner = this.narrowed(tree.as_ref(), head, true);
                    this.block(body, &mut inner);
                    inner
                });
                // The reporting pass, over the ranges the loop really reaches.
                if let Some(tree) = &tree {
                    self.eval(tree, cond, cond_spelling.as_ref(), &head);
                }
                let mut inner = self.narrowed(tree.as_ref(), &head, true);
                self.block(body, &mut inner);
                *env = self.narrowed(tree.as_ref(), &head, false);
            }
            AlgorithmStmt::Foreach {
                item, source, body, ..
            } => {
                // The item holds any element of the source; how many there
                // are is the caller's, so the body runs any number of times.
                let item_range = self
                    .typed(&format!("{source}[0]"))
                    .and_then(|t| type_range(t.ty))
                    .or(Some(Interval { lo: 0, hi: 255 }));
                let head = self.fixed_point(env, |this, head| {
                    let mut inner = head.clone();
                    if let Some(r) = item_range {
                        inner.vars.insert(item.clone(), r);
                    }
                    this.block(body, &mut inner);
                    inner.vars.remove(item);
                    inner
                });
                let mut inner = head.clone();
                if let Some(r) = item_range {
                    inner.vars.insert(item.clone(), r);
                }
                self.block(body, &mut inner);
                inner.vars.remove(item);
                *env = head.join(&inner);
            }
        }
    }

    /// The loop-head ranges a loop whose one round is `round` reaches from
    /// `entry`: iterated without reporting, then widened.
    fn fixed_point(&mut self, entry: &Env, mut round: impl FnMut(&mut Self, &Env) -> Env) -> Env {
        let recording = std::mem::replace(&mut self.recording, false);
        let mut head = entry.clone();
        let mut rounds = 0;
        loop {
            let after = round(self, &head);
            let next = head.join(&after);
            if next == head {
                break;
            }
            rounds += 1;
            head = if rounds < ROUNDS_BEFORE_WIDENING {
                next
            } else {
                self.widened(&head, &next)
            };
        }
        // Narrowing: a widened head holds every value the loop can reach but
        // usually more — a counter guarded by `run < 254` was widened to its
        // whole type, and read AFTER the loop it would still be. `head` is a
        // post-fixpoint (one more round from it stays inside it), so a round
        // from it joined with the entry is inside it too and still holds
        // every reachable value; keeping it while it shrinks is sound. The
        // guard in the body is what brings the counter back to 0..254.
        for _ in 0..ROUNDS_OF_NARROWING {
            let refined = entry.join(&round(self, &head));
            if refined == head || head.join(&refined) != head {
                break;
            }
            head = refined;
        }
        self.recording = recording;
        head
    }

    /// `next` with every range that grew from `prev` widened to its whole
    /// type — the step that ends a loop whose ranges would grow for as many
    /// rounds as its type has values.
    fn widened(&self, prev: &Env, next: &Env) -> Env {
        let mut out = next.clone();
        for (name, range) in out.vars.iter_mut() {
            if prev.vars.get(name) != Some(range) {
                if let Some(full) = self.declared_range(name) {
                    *range = full;
                }
            }
        }
        out
    }

    /// Store `value` in `name`, whose declared type holds `slot`. A value the
    /// slot cannot hold was reported where it was computed ([`Self::stored`]);
    /// an execution that goes on stored one the slot holds.
    fn store(&self, env: &mut Env, name: &str, slot: Option<Interval>, value: Option<Interval>) {
        // A new value keeps no fact a guard proved of the old one.
        env.nonzero.remove(name);
        match (slot, value) {
            (Some(slot), Some(v)) if v.within(slot) => {
                env.vars.insert(name.to_string(), v);
            }
            (Some(slot), Some(v)) if v.lo <= slot.hi && slot.lo <= v.hi => {
                env.vars.insert(name.to_string(), v.clamp(slot));
            }
            (Some(slot), _) => {
                env.vars.insert(name.to_string(), slot);
            }
            (None, _) => {
                env.vars.remove(name);
            }
        }
    }

    /// The range `name`'s declared type holds, from the scope.
    fn declared_range(&self, name: &str) -> Option<Interval> {
        self.declared_type(name).and_then(type_range)
    }

    /// `name`'s declared type, from the scope.
    fn declared_type(&self, name: &str) -> Option<InferredType> {
        self.ctx.vars.get(name).copied()
    }

    /// Judge `expr`, stored in a place of type `place`, recording its hazards
    /// — a value `place` cannot hold among them — and return the range of its
    /// value when it is an integer.
    fn stored(
        &mut self,
        expr: &str,
        spelling: Option<&AttributeSpelling>,
        env: &Env,
        place: Option<InferredType>,
    ) -> Option<Interval> {
        let tree = self.typed(expr)?;
        let value = self.eval(&tree, expr, spelling, env);
        self.fits(&tree, expr, spelling, value, place);
        value
    }

    /// Record a [`HazardKind::DoesNotFit`] when `value`, the range of `node`,
    /// reaches past what `place` holds.
    fn fits(
        &mut self,
        node: &TypedExpr,
        expr: &str,
        spelling: Option<&AttributeSpelling>,
        value: Option<Interval>,
        place: Option<InferredType>,
    ) {
        let (Some(value), Some(place)) = (value, place) else {
            return;
        };
        let Some(slot) = type_range(place) else {
            return;
        };
        if !value.within(slot) {
            self.hazard_in(node, expr, spelling, HazardKind::DoesNotFit, place);
        }
    }

    /// `expr` as the typed tree the emitters lower, or `None` for one the
    /// typed pipeline refuses — that refusal is reported where it is lowered.
    ///
    /// Refused is what the validator path refuses before any backend lowers
    /// it ([`judge_into`]): a name nothing declares, and a literal the type it
    /// takes cannot hold. Either is a refusal of its own, and read as a value
    /// it would make the analysis report an operation it only failed to read
    /// (`reading + 300` is a literal out of `uint8`'s range, not an
    /// overflow).
    fn typed(&self, expr: &str) -> Option<TypedExpr> {
        let judged = judge_into(expr, self.ctx, Expected::Hint(InferredType::Unknown));
        let (Ok(_), Ok(mut tree)) = (judged, resolve(expr, self.ctx)) else {
            self.untyped.set(true);
            return None;
        };
        infer_types(&mut tree, self.ctx);
        Some(tree)
    }

    fn hazard(
        &mut self,
        node: &TypedExpr,
        expr: &str,
        spelling: Option<&AttributeSpelling>,
        kind: HazardKind,
    ) {
        self.hazard_in(node, expr, spelling, kind, node.ty);
    }

    /// Record `kind` at `node`, judged in `ty`.
    fn hazard_in(
        &mut self,
        node: &TypedExpr,
        expr: &str,
        spelling: Option<&AttributeSpelling>,
        kind: HazardKind,
        ty: InferredType,
    ) {
        if !self.recording {
            return;
        }
        let already = self
            .found
            .iter()
            .any(|h| h.expr == expr && h.span == node.span && h.kind == kind);
        if !already {
            self.found.push(IntHazard {
                expr: expr.to_string(),
                span: node.span.clone(),
                spelling: spelling.cloned(),
                kind,
                ty,
            });
        }
    }

    /// The range of `node`'s value when it is an integer, recording every
    /// hazard in it.
    fn eval(
        &mut self,
        node: &TypedExpr,
        expr: &str,
        spelling: Option<&AttributeSpelling>,
        env: &Env,
    ) -> Option<Interval> {
        let own = type_range(node.ty);
        match &node.kind {
            ExprKind::NumberLit(text) => integer_literal(text).or(own),
            ExprKind::Ident(name) | ExprKind::Raw(name) => env.vars.get(name).copied().or(own),
            ExprKind::Member { .. } => member_key(node)
                .and_then(|k| env.vars.get(&k).copied())
                .or(own),
            ExprKind::Unary { op, operand } => {
                let v = self.eval(operand, expr, spelling, env);
                match op {
                    UnaryOp::Neg => {
                        let (Some(v), Some(own)) = (v, own) else {
                            return own;
                        };
                        let r = Interval {
                            lo: -v.hi,
                            hi: -v.lo,
                        };
                        if !r.within(own) {
                            self.hazard(node, expr, spelling, HazardKind::Overflow);
                        }
                        Some(r.clamp(own))
                    }
                    UnaryOp::Pos => v.or(own),
                    UnaryOp::Not | UnaryOp::BitNot => own,
                }
            }
            ExprKind::Binary { op, left, right } => {
                let l = self.eval(left, expr, spelling, env);
                let r = self.eval(right, expr, spelling, env);
                let (Some(l), Some(r), Some(own)) = (l, r, own) else {
                    return own;
                };
                match op {
                    BinOp::Add | BinOp::Sub | BinOp::Mul => {
                        let result = arith(*op, l, r);
                        if !result.within(own) {
                            self.hazard(node, expr, spelling, HazardKind::Overflow);
                        }
                        Some(result.clamp(own))
                    }
                    BinOp::Div | BinOp::Mod => {
                        let guarded = name_of(right).is_some_and(|k| env.nonzero.contains(&k));
                        if r.contains(0) && !guarded {
                            self.hazard(node, expr, spelling, HazardKind::DivideByZero);
                        }
                        if own.lo < 0 && l.contains(own.lo) && r.contains(-1) {
                            self.hazard(node, expr, spelling, HazardKind::MinDividedByMinusOne);
                        }
                        Some(if *op == BinOp::Mod {
                            modulo_range(l, r).clamp(own)
                        } else {
                            quotient_range(l, r).map_or(own, |q| q.clamp(own))
                        })
                    }
                    // Bitwise operations and shifts keep the declared width's
                    // meaning and never fail; their value is the whole type.
                    _ => Some(own),
                }
            }
            // The value is one branch's, and each branch is reached only
            // where the test says so: `y >= 0 ? y : y - 399` subtracts from a
            // negative `y` alone. A branch no value reaches contributes
            // nothing — neither a range nor a hazard.
            ExprKind::Conditional {
                condition,
                consequent,
                alternate,
            } => {
                self.eval(condition, expr, spelling, env);
                let mut value: Option<Interval> = None;
                let mut every_branch_integer = true;
                for (branch, truth) in [(consequent, true), (alternate, false)] {
                    let branch_env = self.narrowed(Some(condition), env, truth);
                    if branch_env.unreachable {
                        continue;
                    }
                    match self.eval(branch, expr, spelling, &branch_env) {
                        Some(v) => value = Some(value.map_or(v, |acc| acc.join(v))),
                        None => every_branch_integer = false,
                    }
                }
                match (value, own) {
                    (Some(v), Some(own)) if every_branch_integer => Some(v.clamp(own)),
                    _ => own,
                }
            }
            ExprKind::Call { callee, args, .. } if matches!(&callee.kind, ExprKind::Ident(n) | ExprKind::Raw(n) if n == "len") =>
            {
                for arg in args {
                    self.eval(arg, expr, spelling, env);
                }
                // The length contract (SCE_FORGE.md §3.4.1): no sequence an
                // algorithm reads holds more than `u32::MAX` elements, on
                // every backend, whatever width its own length type has.
                let contract = Interval {
                    lo: 0,
                    hi: i128::from(u32::MAX),
                };
                Some(own.map_or(contract, |own| contract.clamp(own)))
            }
            // Each argument lands in its parameter's type, as a value lands
            // in a local's.
            ExprKind::Call { args, params, .. } if !params.is_empty() => {
                for (arg, param) in args.iter().zip(params) {
                    let value = self.eval(arg, expr, spelling, env);
                    self.fits(arg, expr, spelling, value, Some(*param));
                }
                own
            }
            _ => {
                for child in node.children() {
                    self.eval(child, expr, spelling, env);
                }
                own
            }
        }
    }

    /// `env` with the names `cond` compares narrowed to the ranges that make
    /// it `truth`, or a dead environment when no value can.
    fn narrowed(&mut self, cond: Option<&TypedExpr>, env: &Env, truth: bool) -> Env {
        let mut out = env.clone();
        if let Some(cond) = cond {
            let recording = std::mem::replace(&mut self.recording, false);
            self.narrow(cond, &mut out, truth);
            self.recording = recording;
        }
        out
    }

    fn narrow(&mut self, cond: &TypedExpr, env: &mut Env, truth: bool) {
        if env.unreachable {
            return;
        }
        match &cond.kind {
            ExprKind::Unary {
                op: UnaryOp::Not,
                operand,
            } => self.narrow(operand, env, !truth),
            ExprKind::Binary { op, left, right } => match (op, truth) {
                (BinOp::And, true) | (BinOp::Or, false) => {
                    self.narrow(left, env, truth);
                    self.narrow(right, env, truth);
                }
                (
                    BinOp::Lt
                    | BinOp::LtEq
                    | BinOp::Gt
                    | BinOp::GtEq
                    | BinOp::StrictEq
                    | BinOp::StrictNeq,
                    _,
                ) => {
                    let op = if truth { *op } else { negate(*op) };
                    // Narrow whichever side names a tracked value, against
                    // the range of the other.
                    if let (Some(key), Some(bound)) =
                        (name_of(left), self.eval(right, "", None, env))
                    {
                        restrict(env, &key, op, bound);
                    }
                    if let (Some(key), Some(bound)) =
                        (name_of(right), self.eval(left, "", None, env))
                    {
                        restrict(env, &key, flip(op), bound);
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
}

/// The exact range of `l op r` for `+ - *`.
fn arith(op: BinOp, l: Interval, r: Interval) -> Interval {
    match op {
        BinOp::Add => Interval {
            lo: l.lo + r.lo,
            hi: l.hi + r.hi,
        },
        BinOp::Sub => Interval {
            lo: l.lo - r.hi,
            hi: l.hi - r.lo,
        },
        _ => {
            let products = [l.lo * r.lo, l.lo * r.hi, l.hi * r.lo, l.hi * r.hi];
            Interval {
                lo: *products.iter().min().unwrap_or(&0),
                hi: *products.iter().max().unwrap_or(&0),
            }
        }
    }
}

/// The exact range of the truncated quotient `l / r`, or `None` when the
/// divisor's range holds zero.
///
/// With the divisor's sign fixed, the quotient moves monotonically with
/// each operand, so its extremes are at the corners. A divisor range that
/// holds zero (a guarded one, which may still hold both signs) has no such
/// corners, and the caller keeps the whole type.
fn quotient_range(l: Interval, r: Interval) -> Option<Interval> {
    if r.contains(0) {
        return None;
    }
    let quotients = [l.lo / r.lo, l.lo / r.hi, l.hi / r.lo, l.hi / r.hi];
    Some(Interval {
        lo: *quotients.iter().min()?,
        hi: *quotients.iter().max()?,
    })
}

/// A range the remainder `l % r` lies in: less than the largest divisor in
/// magnitude, with the dividend's sign (truncated division, as every backend
/// performs it).
fn modulo_range(l: Interval, r: Interval) -> Interval {
    let m = r.lo.abs().max(r.hi.abs()).max(1) - 1;
    let lo = if l.lo < 0 { -m.min(-l.lo) } else { 0 };
    let hi = if l.hi > 0 { m.min(l.hi) } else { 0 };
    Interval { lo, hi }
}

/// An integer literal's value; `None` for a real one.
fn integer_literal(text: &str) -> Option<Interval> {
    let t = text.replace('_', "");
    let v = if let Some(h) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        i128::from_str_radix(h, 16).ok()?
    } else if let Some(b) = t.strip_prefix("0b").or_else(|| t.strip_prefix("0B")) {
        i128::from_str_radix(b, 2).ok()?
    } else if let Some(o) = t.strip_prefix("0o").or_else(|| t.strip_prefix("0O")) {
        i128::from_str_radix(o, 8).ok()?
    } else {
        t.parse::<i128>().ok()?
    };
    Some(Interval::point(v))
}

/// The key a tracked value is stored under: a bare name, or `object.field`.
fn name_of(node: &TypedExpr) -> Option<String> {
    match &node.kind {
        ExprKind::Ident(n) | ExprKind::Raw(n) => Some(n.clone()),
        ExprKind::Member { .. } => member_key(node),
        _ => None,
    }
}

fn member_key(node: &TypedExpr) -> Option<String> {
    let ExprKind::Member { object, property } = &node.kind else {
        return None;
    };
    Some(format!("{}.{property}", name_of(object)?))
}

fn negate(op: BinOp) -> BinOp {
    match op {
        BinOp::Lt => BinOp::GtEq,
        BinOp::LtEq => BinOp::Gt,
        BinOp::Gt => BinOp::LtEq,
        BinOp::GtEq => BinOp::Lt,
        BinOp::StrictEq => BinOp::StrictNeq,
        BinOp::StrictNeq => BinOp::StrictEq,
        other => other,
    }
}

/// `a op b` restated as `b op' a`.
fn flip(op: BinOp) -> BinOp {
    match op {
        BinOp::Lt => BinOp::Gt,
        BinOp::LtEq => BinOp::GtEq,
        BinOp::Gt => BinOp::Lt,
        BinOp::GtEq => BinOp::LtEq,
        other => other,
    }
}

/// Narrow `key` to the values for which `key op bound` can hold.
fn restrict(env: &mut Env, key: &str, op: BinOp, bound: Interval) {
    if op == BinOp::StrictNeq && bound == Interval::point(0) {
        env.nonzero.insert(key.to_string());
    }
    let Some(cur) = env.vars.get(key).copied() else {
        return;
    };
    let next = match op {
        BinOp::Lt => Interval {
            lo: cur.lo,
            hi: cur.hi.min(bound.hi - 1),
        },
        BinOp::LtEq => Interval {
            lo: cur.lo,
            hi: cur.hi.min(bound.hi),
        },
        BinOp::Gt => Interval {
            lo: cur.lo.max(bound.lo + 1),
            hi: cur.hi,
        },
        BinOp::GtEq => Interval {
            lo: cur.lo.max(bound.lo),
            hi: cur.hi,
        },
        BinOp::StrictEq => cur.clamp(bound),
        // `!==` narrows only a bound equal to an end of the range.
        BinOp::StrictNeq if bound.lo == bound.hi => {
            if bound.lo == cur.lo {
                Interval {
                    lo: cur.lo + 1,
                    hi: cur.hi,
                }
            } else if bound.hi == cur.hi {
                Interval {
                    lo: cur.lo,
                    hi: cur.hi - 1,
                }
            } else {
                cur
            }
        }
        _ => cur,
    };
    if next.lo > next.hi {
        *env = Env::dead();
    } else {
        env.vars.insert(key.to_string(), next);
    }
}

impl HazardKind {
    /// What the operation can do, as the refusal says it.
    pub(crate) fn described(self) -> &'static str {
        match self {
            HazardKind::Overflow => "overflow",
            HazardKind::DivideByZero => "divide by zero",
            HazardKind::MinDividedByMinusOne => "divide the minimum by -1",
            HazardKind::DoesNotFit => "leave the type it is stored in",
        }
    }
}

/// The integer arithmetic contract, enforced (SCE_FORGE.md §3.4.1): an
/// algorithm that does not declare `may-fail` is refused at the first
/// operation the analysis cannot prove safe.
///
/// Judged once, before any backend renders, against the scope the renderer
/// lowers the body in ([`AlgorithmTypes`](crate::forge::generator::AlgorithmTypes)),
/// so every backend and `check` refuse the same documents. A `may-fail`
/// algorithm is not judged: every operation of its body is checked when it
/// runs, whatever the analysis could prove.
pub(crate) fn check(
    m: &crate::forge::model::AlgorithmModel,
    imports: &[crate::forge::generator::ImportContext],
    options: &crate::ForgeCompileOptions,
) -> Result<(), crate::forge::error::ForgeError> {
    use crate::forge::error::ValidationError;
    use crate::forge::expression_site::ExpressionSite;
    if m.signature.may_fail {
        return Ok(());
    }
    let types = crate::forge::generator::AlgorithmTypes::collect(m, imports, options)?;
    let ctx = types.type_ctx(m, imports);
    let params: Vec<(String, AlgorithmValueType)> = m
        .signature
        .params
        .iter()
        .map(|p| (p.name.clone(), p.sce_type.clone()))
        .collect();
    // A body the typed pipeline refuses somewhere is not judged here: its
    // refusal is raised where that expression is lowered.
    let Some(first) = hazards(&params, m.signature.return_type.as_ref(), &m.body, &ctx)
        .and_then(|found| found.into_iter().next())
    else {
        return Ok(());
    };
    let at = ExpressionSite::new(&first.expr, first.spelling.as_ref()).locate(first.span.clone());
    // The operation as written; the parsed text of it when the attribute
    // cannot say (text over several rows), and the whole expression when
    // the parser recorded no range.
    let trimmed = first.expr.trim();
    let operation = at.observed().unwrap_or_else(|| {
        first
            .span
            .clone()
            .and_then(|span| trimmed.get(span))
            .unwrap_or(trimmed)
            .to_string()
    });
    let refusal: crate::forge::error::ForgeError =
        ValidationError::AlgorithmUndeclaredIntegerFailure {
            algorithm: m.name.clone(),
            operation,
            hazard: first.kind.described().to_string(),
            ty: first.ty.describe(),
            observed: at.observed(),
        }
        .into();
    Err(at.place(refusal))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::model::SceType;

    fn u8t() -> AlgorithmValueType {
        AlgorithmValueType::Scalar(SceType::Uint8)
    }

    fn var(name: &str, ty: AlgorithmValueType, init: &str) -> AlgorithmStmt {
        AlgorithmStmt::Var {
            name: name.into(),
            name_spelling: None,
            sce_type: ty,
            init: Some(init.into()),
            init_spelling: None,
            capacity: None,
            capacity_spelling: None,
        }
    }

    fn assign(target: &str, expr: &str) -> AlgorithmStmt {
        AlgorithmStmt::Assign {
            target: target.into(),
            target_spelling: None,
            expr: expr.into(),
            expr_spelling: None,
        }
    }

    fn while_(cond: &str, max_iter: u32, body: Vec<AlgorithmStmt>) -> AlgorithmStmt {
        AlgorithmStmt::While {
            cond: cond.into(),
            cond_spelling: None,
            body,
            max_iter: Some(max_iter),
        }
    }

    fn if_(cond: &str, then_body: Vec<AlgorithmStmt>) -> AlgorithmStmt {
        AlgorithmStmt::If {
            cond: cond.into(),
            cond_spelling: None,
            then_body,
            else_body: None,
        }
    }

    fn ret(expr: &str) -> AlgorithmStmt {
        AlgorithmStmt::Return {
            expr: Some(expr.into()),
            expr_spelling: None,
        }
    }

    /// The hazards of `body`, every name in `names` typed as given.
    fn analyse(
        params: &[(&str, SceType)],
        locals: &[(&str, SceType)],
        body: &[AlgorithmStmt],
    ) -> Vec<(String, HazardKind)> {
        analyse_returning(params, locals, None, body)
    }

    /// [`analyse`] of an algorithm that declares it returns `ret`.
    fn analyse_returning(
        params: &[(&str, SceType)],
        locals: &[(&str, SceType)],
        ret: Option<SceType>,
        body: &[AlgorithmStmt],
    ) -> Vec<(String, HazardKind)> {
        let mut ctx = TypeCtx::new();
        for (n, t) in params.iter().chain(locals) {
            ctx.insert_var(n, InferredType::from_sce_type(t));
        }
        let params: Vec<(String, AlgorithmValueType)> = params
            .iter()
            .map(|(n, t)| (n.to_string(), AlgorithmValueType::Scalar(t.clone())))
            .collect();
        let ret = ret.map(AlgorithmValueType::Scalar);
        hazards(&params, ret.as_ref(), body, &ctx)
            .expect("every expression of a unit-test body types")
            .into_iter()
            .map(|h| (h.expr, h.kind))
            .collect()
    }

    /// A wider value stored in a narrower local is not wrapped into it: it
    /// is an operation that can fail, like an overflowing sum.
    #[test]
    fn a_value_its_local_cannot_hold_is_a_hazard() {
        let body = [
            var("v", AlgorithmValueType::Scalar(SceType::Int32), "x"),
            ret("v"),
        ];
        assert_eq!(
            analyse(&[("x", SceType::Int64)], &[("v", SceType::Int32)], &body),
            vec![("x".to_string(), HazardKind::DoesNotFit)]
        );
    }

    /// The range, not the type, decides: a remainder folded into 0..6 fits
    /// a uint8 whatever int64 it came from.
    #[test]
    fn a_value_proven_to_fit_a_narrower_local_is_not_a_hazard() {
        let body = [var("w", u8t(), "(x % 7 + 11) % 7"), ret("w")];
        assert!(analyse(&[("x", SceType::Int64)], &[("w", SceType::Uint8)], &body).is_empty());
    }

    /// The returned value lands in the declared return type.
    #[test]
    fn a_returned_value_its_return_type_cannot_hold_is_a_hazard() {
        let body = [ret("x")];
        assert_eq!(
            analyse_returning(&[("x", SceType::Int64)], &[], Some(SceType::Uint8), &body),
            vec![("x".to_string(), HazardKind::DoesNotFit)]
        );
        let guarded = [if_("x >= 0 && x <= 255", vec![ret("x")]), ret("0")];
        assert!(
            analyse_returning(
                &[("x", SceType::Int64)],
                &[],
                Some(SceType::Uint8),
                &guarded
            )
            .is_empty(),
            "a guard that bounds the value proves it fits"
        );
    }

    /// An assignment and a record field are places too.
    #[test]
    fn an_assigned_value_its_target_cannot_hold_is_a_hazard() {
        let body = [var("n", u8t(), "0"), assign("n", "x"), ret("n")];
        assert_eq!(
            analyse(&[("x", SceType::Uint16)], &[("n", SceType::Uint8)], &body),
            vec![("x".to_string(), HazardKind::DoesNotFit)]
        );
    }

    #[test]
    fn a_counter_bounded_by_its_loop_condition_cannot_overflow() {
        // `i < 8` keeps `i` at most 7 inside the body, so `i + 1` is at most
        // 8 — widening the loop head to the whole of uint8 does not lose it,
        // because the condition narrows it again.
        let body = [
            var("i", u8t(), "0"),
            while_("i < 8", 8, vec![assign("i", "i + 1")]),
            ret("i"),
        ];
        assert!(analyse(&[], &[("i", SceType::Uint8)], &body).is_empty());
    }

    #[test]
    fn a_conditional_is_as_wide_as_the_branches_its_test_lets_through() {
        // `y` holds an int32 widened to int64. `y - 399` is taken only when
        // `y` is negative, and the value is one branch's, so what follows
        // is judged against the union of the two — not the whole of int64.
        let i64t = AlgorithmValueType::Scalar(SceType::Int64);
        let body = [
            var("y", i64t.clone(), "year"),
            var("era", i64t.clone(), "(y >= 0 ? y : y - 399) / 400"),
            ret("era * 146097"),
        ];
        assert!(analyse(
            &[("year", SceType::Int32)],
            &[("y", SceType::Int64), ("era", SceType::Int64)],
            &body
        )
        .is_empty());
    }

    #[test]
    fn a_hazard_in_a_branch_its_test_excludes_is_not_reported() {
        // `x - 1` is taken only when `x` is above 0, where it cannot pass
        // below uint8's range.
        let body = [ret("x > 0 ? x - 1 : x")];
        assert!(analyse(&[("x", SceType::Uint8)], &[], &body).is_empty());
        // Unguarded, it can.
        let body = [ret("x >= 0 ? x - 1 : x")];
        assert_eq!(
            analyse(&[("x", SceType::Uint8)], &[], &body),
            vec![("x >= 0 ? x - 1 : x".to_string(), HazardKind::Overflow)]
        );
    }

    #[test]
    fn a_quotient_by_a_divisor_of_one_sign_keeps_its_bound() {
        // `x / 400` of an int32 stays within an int32 over 400, so a
        // product of it that the whole of int64 would overflow does not.
        let body = [ret("x / 400 * 146097")];
        assert!(analyse(&[("x", SceType::Int64)], &[], &body).len() == 1);
        let i64t = AlgorithmValueType::Scalar(SceType::Int64);
        let body = [var("w", i64t, "x"), ret("w / 400 * 146097")];
        assert!(analyse(&[("x", SceType::Int32)], &[("w", SceType::Int64)], &body).is_empty());
    }

    #[test]
    fn an_unbounded_increment_of_a_parameter_can_overflow() {
        // The HLC counter: `prev + 1` for any uint32 `prev` passes 2^32 - 1.
        let body = [ret("prev + 1")];
        assert_eq!(
            analyse(&[("prev", SceType::Uint32)], &[], &body),
            vec![("prev + 1".to_string(), HazardKind::Overflow)]
        );
    }

    #[test]
    fn a_body_with_an_expression_that_does_not_type_is_not_judged() {
        // `limt` is misspelt: the guard cannot be read, so `i + 1` would look
        // unbounded. The misspelling has its own refusal where the guard is
        // lowered; the analysis declines rather than name an overflow.
        let mut ctx = TypeCtx::new();
        ctx.insert_var("i", InferredType::from_sce_type(&SceType::Uint16));
        ctx.reject_unknown_identifiers = true;
        let body = [
            var("i", AlgorithmValueType::Scalar(SceType::Uint16), "0"),
            while_("i < limt", 8, vec![assign("i", "i + 1")]),
            ret("i"),
        ];
        assert!(hazards(&[], None, &body, &ctx).is_none());
    }

    #[test]
    fn a_counter_read_after_its_loop_keeps_the_bound_its_guard_imposes() {
        // Widening takes `i` to all of uint8; narrowing brings it back to
        // 0..200, the most the guarded increment reaches, so `i + 50` after
        // the loop is at most 250 — safe. Without narrowing it would be
        // reported, the COBS encoder's `run + 1` being the case that showed it.
        let body = [
            var("i", u8t(), "0"),
            while_("i < 200", 256, vec![assign("i", "i + 1")]),
            ret("i + 50"),
        ];
        assert!(analyse(&[], &[("i", SceType::Uint8)], &body).is_empty());
        // A bound the guard does not impose is still not assumed.
        let body = [
            var("i", u8t(), "0"),
            while_("i < 200", 256, vec![assign("i", "i + 1")]),
            ret("i + 56"),
        ];
        assert_eq!(
            analyse(&[], &[("i", SceType::Uint8)], &body),
            vec![("i + 56".to_string(), HazardKind::Overflow)]
        );
    }

    #[test]
    fn an_index_bounded_by_a_length_follows_the_length_contract() {
        // `i < len(a)` holds `i` below `u32::MAX` under the length contract,
        // so a uint32 index steps safely; a uint8 index still passes 255.
        let scan = |ty: SceType| {
            let body = [
                var("i", AlgorithmValueType::Scalar(ty.clone()), "0"),
                while_("i < len(a)", 64, vec![assign("i", "i + 1")]),
                ret("i"),
            ];
            analyse(&[("a", SceType::Bytes)], &[("i", ty)], &body)
        };
        assert!(scan(SceType::Uint32).is_empty());
        assert_eq!(
            scan(SceType::Uint8),
            vec![("i + 1".to_string(), HazardKind::Overflow)]
        );
    }

    #[test]
    fn a_guard_that_excludes_the_maximum_makes_the_increment_safe() {
        let body = [
            var("n", AlgorithmValueType::Scalar(SceType::Uint32), "0"),
            if_("prev < 4294967295", vec![assign("n", "prev + 1")]),
            ret("n"),
        ];
        assert!(analyse(
            &[("prev", SceType::Uint32)],
            &[("n", SceType::Uint32)],
            &body
        )
        .is_empty());
    }

    #[test]
    fn a_divisor_that_can_be_zero_is_a_hazard_and_a_nonzero_one_is_not() {
        let body = [ret("a / b")];
        assert_eq!(
            analyse(
                &[("a", SceType::Uint16), ("b", SceType::Uint16)],
                &[],
                &body
            ),
            vec![("a / b".to_string(), HazardKind::DivideByZero)]
        );
        let guarded = [if_("b !== 0", vec![ret("a / b")]), ret("0")];
        assert!(analyse(
            &[("a", SceType::Uint16), ("b", SceType::Uint16)],
            &[],
            &guarded
        )
        .is_empty());
    }

    #[test]
    fn a_signed_division_that_can_take_min_by_minus_one_is_a_hazard() {
        let body = [if_("b !== 0", vec![ret("a / b")]), ret("0")];
        assert_eq!(
            analyse(&[("a", SceType::Int32), ("b", SceType::Int32)], &[], &body),
            vec![("a / b".to_string(), HazardKind::MinDividedByMinusOne)]
        );
    }

    #[test]
    fn bitwise_operations_and_shifts_never_fail() {
        // CRC arithmetic: a shift that drops the high bit is the declared
        // width's meaning, not an overflow.
        let body = [ret("(crc << 1) ^ 4129")];
        assert!(analyse(&[("crc", SceType::Uint16)], &[], &body).is_empty());
    }

    #[test]
    fn an_accumulator_growing_each_round_is_widened_and_found() {
        // `sum` grows every round with no guard that bounds it: widening
        // takes it to the whole of uint16, and `sum + i` can then pass it.
        let body = [
            var("sum", AlgorithmValueType::Scalar(SceType::Uint16), "0"),
            var("i", u8t(), "0"),
            while_(
                "i < 200",
                200,
                vec![assign("sum", "sum + i"), assign("i", "i + 1")],
            ),
            ret("sum"),
        ];
        assert_eq!(
            analyse(
                &[],
                &[("sum", SceType::Uint16), ("i", SceType::Uint8)],
                &body
            ),
            vec![("sum + i".to_string(), HazardKind::Overflow)]
        );
    }

    #[test]
    fn a_negation_of_a_signed_minimum_is_a_hazard() {
        let body = [ret("-x")];
        assert_eq!(
            analyse(&[("x", SceType::Int8)], &[], &body),
            vec![("-x".to_string(), HazardKind::Overflow)]
        );
    }
}
