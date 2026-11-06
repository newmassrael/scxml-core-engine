// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Where a value that may be absent is proven present (docs/adr/0055,
//! section 3): the one analysis a `sce-static` statechart's judge and an
//! algorithm's body walk both read, so the two cannot disagree about what a
//! test proves, how far a fact reaches, or what ends one.
//!
//! A **subject** is a path the build can follow — a variable, a field of a
//! record, a field of the payload (`_event.data.due`), a field of a loop's
//! record item, an algorithm's parameter or local or a field of one, each
//! followed by `.value.<field>` through an optional record already proven. An
//! element read by its index (`xs[i].due`) and the value of a call are no
//! subjects: an index may change between a test and a read, and a call answers
//! anew each time. A subject is proven present by a test on it and by nothing
//! else: `s !== null` (or `null !== s`) where it is true, `s === null` (or
//! `null === s`) where it is false.
//!
//! What this module decides, and each walker applies where its own tree has
//! the construct:
//!
//! - within one expression ([`proves`]): the right operand of `a && b` is read
//!   where `a` is true, of `a || b` where `a` is false, and the branches of
//!   `c ? t : e` where `c` is true and false; `!c` exchanges the two. The
//!   expression typer reads this ([`crate::forge::expr`]).
//! - an `<if>` / `<elseif>` / `<else>` chain, and an algorithm's `<sce:if>` /
//!   `<sce:else>` ([`Chain`]): each condition is read where every earlier one
//!   is false, each block where its own is true too, the `else` where all are
//!   false; nothing a chain proves reaches past its end.
//! - a loop ([`loop_entry`]): a fact holds in its body only if nothing in the
//!   body ends it, since a second turn follows the end of the first; an
//!   `<sce:while>`'s condition proves its facts at the start of each turn.
//! - an algorithm's `<sce:require>` (the statements after it are read where its
//!   condition is true) and an `<sce:if>` with no `<sce:else>` whose block
//!   returns on every path (the statements after it are read where its
//!   condition is false) — [`always_returns`].
//! - a transition's `cond`, which proves a field of its payload in the
//!   transition's content and nothing else ([`Facts::of_payload`]): between the
//!   two the microstep runs other content, which may write any variable.
//!
//! What ends a fact ([`Facts::after`]): a write of the subject, of a path below
//! it, or of a path above it (`r` for `r.f`, `r.value` for `r.value.f`),
//! whatever value it writes, and a statement that may write anything (a native
//! `<script>`). Re-assigning a present value proves nothing: only a test does.
//!
//! The same analysis keeps a second kind of fact, which case of a union a
//! subject is in (docs/adr/0092, section 6.1): a test `s.<tag> === E.v` (or
//! `E.v === s.<tag>`) proves the path `s.<tag>` one of `{E.v}` where it holds
//! and not `E.v` where it does not, `!==` the other way round ([`CaseFact`]).
//! Such a fact is kept under the path of the tag read, and this module does not
//! know which paths are tags: the judge, which types the subject, asks the fact
//! of the tag path of a union subject ([`Facts::cases_of`]) and of no other.
//! Its constructs and its invalidations are the ones above, with one addition:
//! where `a || b` is true and each side proves one path one of a set of
//! variants, the path is one of the two sets together (the cases a field
//! common to them is read under, section 6.1 item 2).

use std::collections::{BTreeMap, BTreeSet};

use crate::forge::expr::{
    dotted_path, judge_optional_use, parse_to_ast, BinOp, Expected, ExprKind, TypedExpr, UnaryOp,
    OPTIONAL_VALUE,
};
use crate::forge::expression_site::ExpressionSite;
use crate::forge::model::AlgorithmStmt;
use crate::forge::types::{InferredType, OptionalElem, TypeCtx};
use crate::model::Action;

/// The path every field of a transition's payload is read under.
const PAYLOAD: &str = crate::forge::event_schema_check::EVENT_DATA_PATH;

/// The facts proven at one point of a walk: the subjects proven present, and
/// for each tag path a test named, the variants it may be ([`CaseFact`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Facts {
    present: BTreeSet<String>,
    cases: BTreeMap<String, CaseFact>,
}

/// A variant a test names, `E.v`: the alias of the enum as written, and the
/// variant's name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Variant {
    pub alias: String,
    pub name: String,
}

/// What the tests on one tag path prove about it (docs/adr/0092, section 6.1):
/// the variants it is one of, where a test said which (`None` where none
/// did), and the variants it is not.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CaseFact {
    pub within: Option<BTreeSet<Variant>>,
    pub not: BTreeSet<Variant>,
}

impl CaseFact {
    /// Both facts hold: the variants it is one of are those both allow, and the
    /// variants it is not are those either excludes.
    fn and(&self, other: &CaseFact) -> CaseFact {
        CaseFact {
            within: match (&self.within, &other.within) {
                (Some(a), Some(b)) => Some(a.intersection(b).cloned().collect()),
                (Some(a), None) | (None, Some(a)) => Some(a.clone()),
                (None, None) => None,
            },
            not: self.not.union(&other.not).cloned().collect(),
        }
    }

    /// One of the two facts holds: the variants it may be one of are either's,
    /// and the variants it is not are those both exclude. `None` where nothing
    /// is left that both say.
    fn or(&self, other: &CaseFact) -> Option<CaseFact> {
        let fact = CaseFact {
            within: match (&self.within, &other.within) {
                (Some(a), Some(b)) => Some(a.union(b).cloned().collect()),
                _ => None,
            },
            not: self.not.intersection(&other.not).cloned().collect(),
        };
        (fact.within.is_some() || !fact.not.is_empty()).then_some(fact)
    }

    /// Whether the variant `name` of the enum the build names `alias` may be
    /// the tag's value: a test naming another enum's variant is no test of this
    /// tag, and the comparison it makes is refused where the enum is judged.
    pub fn admits(&self, alias: &str, name: &str) -> bool {
        let named = |v: &Variant| v.alias == alias && v.name == name;
        let within = match &self.within {
            Some(within) => within.iter().any(named),
            None => true,
        };
        within && !self.not.iter().any(named)
    }
}

impl Facts {
    /// No subject proven: where every block starts.
    pub fn none() -> Self {
        Self::default()
    }

    /// Whether `subject` is proven present here.
    pub fn holds(&self, subject: &str) -> bool {
        self.present.contains(subject)
    }

    /// What the tests that hold here prove about the tag read at `tag_path`
    /// (`when.kind`), when one does (docs/adr/0092, section 6.1).
    pub fn cases_of(&self, tag_path: &str) -> Option<&CaseFact> {
        self.cases.get(tag_path)
    }

    /// Whether nothing is proven here.
    pub fn is_empty(&self) -> bool {
        self.present.is_empty() && self.cases.is_empty()
    }

    /// These facts and `other`'s: both hold.
    pub fn and(&self, other: &Facts) -> Facts {
        let mut cases = self.cases.clone();
        for (path, fact) in &other.cases {
            let joined = match cases.get(path) {
                Some(held) => held.and(fact),
                None => fact.clone(),
            };
            cases.insert(path.clone(), joined);
        }
        Facts {
            present: self.present.union(&other.present).cloned().collect(),
            cases,
        }
    }

    /// Either these facts or `other`'s hold: a fact about which cases a tag
    /// path is in that both state, widened to either's cases. No subject is
    /// proven present by a disjunction (docs/adr/0055, section 3).
    fn or(&self, other: &Facts) -> Facts {
        Facts {
            present: BTreeSet::new(),
            cases: self
                .cases
                .iter()
                .filter_map(|(path, fact)| {
                    let theirs = other.cases.get(path)?;
                    fact.or(theirs).map(|joined| (path.clone(), joined))
                })
                .collect(),
        }
    }

    /// The facts that hold after what `writes` may write ran: a write ends a
    /// fact about its path, a path below it and a path above it, and a
    /// statement that may write anything ends them all. A fact about a tag is
    /// ended by a write of the union that holds it (a path above it), and not
    /// by a write of a field of its case, which leaves the case as it was.
    pub fn after(&self, writes: &Writes) -> Facts {
        if writes.anything {
            return Facts::none();
        }
        let kept = |subject: &String| !writes.paths.iter().any(|path| ends(path, subject));
        Facts {
            present: self.present.iter().filter(|s| kept(s)).cloned().collect(),
            cases: self
                .cases
                .iter()
                .filter(|(path, _)| kept(path))
                .map(|(path, fact)| (path.clone(), fact.clone()))
                .collect(),
        }
    }

    /// The facts of a transition's `cond` that reach its executable content:
    /// those about a field of the payload, which is the same value from the
    /// `cond` to the end of the transition. A fact about a variable does not
    /// reach it: the exit actions and the other transitions of the microstep
    /// run between the two, and any of them may write the variable.
    pub fn of_payload(&self) -> Facts {
        let prefix = format!("{PAYLOAD}.");
        let payload = |subject: &String| subject.starts_with(&prefix);
        Facts {
            present: self
                .present
                .iter()
                .filter(|s| payload(s))
                .cloned()
                .collect(),
            cases: self
                .cases
                .iter()
                .filter(|(path, _)| payload(path))
                .map(|(path, fact)| (path.clone(), fact.clone()))
                .collect(),
        }
    }

    fn insert(&mut self, subject: String) {
        self.present.insert(subject);
    }

    /// The tag path `path` is `variant` (`is`), or is not.
    fn insert_case(&mut self, path: String, variant: Variant, is: bool) {
        let fact = if is {
            CaseFact {
                within: Some(BTreeSet::from([variant])),
                not: BTreeSet::new(),
            }
        } else {
            CaseFact {
                within: None,
                not: BTreeSet::from([variant]),
            }
        };
        let joined = match self.cases.get(&path) {
            Some(held) => held.and(&fact),
            None => fact,
        };
        self.cases.insert(path, joined);
    }
}

/// Whether a write of `path` ends a fact about `subject`: `path` is the
/// subject, a path below it, or a path above it — but for a path below the
/// present value of the subject (`r.value.f` for `r`, an optional record), which
/// changes what the record holds and not whether it is present: a field written
/// under a test of `r` leaves `r` proven, so that a second field may be written
/// after the first.
fn ends(path: &str, subject: &str) -> bool {
    let below = |longer: &str, shorter: &str| {
        longer
            .strip_prefix(shorter)
            .is_some_and(|rest| rest.starts_with('.'))
    };
    let within_present_value = path
        .strip_prefix(subject)
        .and_then(|rest| rest.strip_prefix('.'))
        .and_then(|rest| rest.strip_prefix(OPTIONAL_VALUE))
        .is_some_and(|rest| rest.starts_with('.'));
    if within_present_value {
        return false;
    }
    path == subject || below(path, subject) || below(subject, path)
}

/// The optional record whose present value's field `target` names, when it is
/// written `<record>.value.<field>` (`pick.value.count` gives `pick`): the shape
/// of a write to a field of an optional record (docs/adr/0055), which stands only
/// where a test proves the record present. `None` for any other location.
pub(crate) fn present_record_of(target: &str) -> Option<&str> {
    let marker = format!(".{OPTIONAL_VALUE}.");
    let target = target.trim();
    let (holder, field) = target.split_once(marker.as_str())?;
    (!holder.is_empty()
        && !holder.contains('.')
        && !field.is_empty()
        && !field.contains(['.', '[']))
    .then_some(holder)
}

/// What a statement, or a block of them, may write: each location written, and
/// whether it may write anything at all.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Writes {
    paths: Vec<String>,
    anything: bool,
}

impl Writes {
    /// A write of the location `path`, as written (`count`, `item.due`).
    pub fn write(&mut self, path: &str) {
        let path = path.trim();
        if !path.is_empty() {
            self.paths.push(path.to_string());
        }
    }

    /// A statement that may write anything — a native `<script>`, whose
    /// effect no build reads.
    pub fn anything(&mut self) {
        self.anything = true;
    }

    /// Everything `statement` may write, the blocks it holds included.
    pub fn of<S: Statement>(statement: &S) -> Self {
        let mut writes = Self::default();
        writes.collect(statement);
        writes
    }

    /// Everything the statements of `block` may write, at any depth.
    pub fn of_block<S: Statement>(block: &[S]) -> Self {
        let mut writes = Self::default();
        for statement in block {
            writes.collect(statement);
        }
        writes
    }

    fn collect<S: Statement>(&mut self, statement: &S) {
        statement.writes(self);
        for block in statement.blocks() {
            for nested in block {
                self.collect(nested);
            }
        }
    }
}

/// A statement as this analysis reads it: what it writes itself, and the
/// blocks it holds. One implementation per tree a walker reads, here beside
/// the rules, so what ends a fact is said once for each.
pub trait Statement: Sized {
    /// Each location this statement writes itself — not what a block it holds
    /// writes.
    fn writes(&self, into: &mut Writes);
    /// The blocks this statement holds.
    fn blocks(&self) -> Vec<&[Self]>;
}

/// A `sce-static` statechart's executable content: an `<assign>`, an
/// `<sce:append>` and an `<sce:clear>` write their location, a `<send>` that
/// names an `idlocation` writes the id there, and a `<script>` (a native block,
/// the one this data model admits) may write anything. An `<invoke>`'s
/// `idlocation` is written when the invocation starts, at the end of a
/// macrostep, after every block that could hold a fact has run, so it ends
/// none: each block starts with no fact about a variable.
impl Statement for Action {
    fn writes(&self, into: &mut Writes) {
        match self.action_type.as_str() {
            // A `<sce:render>` replaces its location whole (docs/adr/0087).
            "assign" | "sce_append" | "sce_clear" | "sce_render" => into.write(&self.location),
            // A lift writes its refusal on every run, and its location on a
            // text that is read (docs/adr/0087, section 2.1): both may change.
            "sce_lift" => {
                into.write(&self.location);
                into.write(&self.lift_refusal);
            }
            "send" => into.write(&self.idlocation),
            "script" => into.anything(),
            _ => {}
        }
    }

    fn blocks(&self) -> Vec<&[Self]> {
        self.nested_blocks()
            .into_iter()
            .map(|block| block.actions)
            .collect()
    }
}

/// An algorithm's body: an `<sce:assign>` writes its target (a record or a
/// list an algorithm hands back whole, taken into the local that holds it,
/// among them), an `<sce:append>` its buffer, and a local's declaration its
/// name.
impl Statement for AlgorithmStmt {
    fn writes(&self, into: &mut Writes) {
        match self {
            AlgorithmStmt::Assign { target, .. } | AlgorithmStmt::Append { target, .. } => {
                into.write(target)
            }
            AlgorithmStmt::Var { name, .. }
            | AlgorithmStmt::RecordVar { name, .. }
            | AlgorithmStmt::RecordFromCall { name, .. } => into.write(name),
            AlgorithmStmt::If { .. }
            | AlgorithmStmt::While { .. }
            | AlgorithmStmt::Foreach { .. }
            | AlgorithmStmt::Return { .. }
            | AlgorithmStmt::Call { .. }
            | AlgorithmStmt::Require { .. } => {}
        }
    }

    fn blocks(&self) -> Vec<&[Self]> {
        match self {
            AlgorithmStmt::If {
                then_body,
                else_body,
                ..
            } => std::iter::once(then_body.as_slice())
                .chain(else_body.as_deref())
                .collect(),
            AlgorithmStmt::While { body, .. } | AlgorithmStmt::Foreach { body, .. } => {
                vec![body.as_slice()]
            }
            AlgorithmStmt::Var { .. }
            | AlgorithmStmt::RecordVar { .. }
            | AlgorithmStmt::RecordFromCall { .. }
            | AlgorithmStmt::Assign { .. }
            | AlgorithmStmt::Append { .. }
            | AlgorithmStmt::Return { .. }
            | AlgorithmStmt::Call { .. }
            | AlgorithmStmt::Require { .. } => Vec::new(),
        }
    }
}

/// The subject `node` names, or `None` for what no test can prove: an element
/// read by its index, the value of a call, anything that is not a chain of
/// names.
pub(crate) fn subject(node: &TypedExpr) -> Option<String> {
    dotted_path(node)
}

/// What `cond` proves where it is `when`: the subjects a test in it names
/// present.
pub(crate) fn proves(cond: &TypedExpr, when: bool) -> Facts {
    let mut facts = Facts::none();
    collect_proofs(cond, when, &mut facts);
    facts
}

/// [`proves`] of a condition as written. One that does not parse proves
/// nothing; refusing it is the judge's.
pub fn proves_written(cond: &str, when: bool) -> Facts {
    parse_to_ast(cond.trim()).map_or_else(|_| Facts::none(), |ast| proves(&ast, when))
}

fn collect_proofs(node: &TypedExpr, when: bool, into: &mut Facts) {
    match &node.kind {
        ExprKind::Binary {
            op: op @ (BinOp::StrictEq | BinOp::StrictNeq),
            left,
            right,
        } => {
            // `s !== null` proves `s` where it is true, `s === null` where it is
            // false.
            let present_when = *op == BinOp::StrictNeq;
            if when == present_when {
                if let Some(tested) = tested(left, right).or_else(|| tested(right, left)) {
                    into.insert(tested);
                }
            }
            // `s.<tag> === E.v` proves the tag `E.v` where it is true and not
            // `E.v` where it is false; `!==` the other way round
            // (docs/adr/0092, section 6.1). Either side may hold the variant.
            let is = when == (*op == BinOp::StrictEq);
            for (path, variant) in [tag_test(left, right), tag_test(right, left)]
                .into_iter()
                .flatten()
            {
                into.insert_case(path, variant, is);
            }
        }
        ExprKind::Unary {
            op: UnaryOp::Not,
            operand,
        } => collect_proofs(operand, !when, into),
        // Where `a && b` is true both are; where `a || b` is false neither is.
        ExprKind::Binary {
            op: BinOp::And,
            left,
            right,
        } if when => {
            collect_proofs(left, true, into);
            collect_proofs(right, true, into);
        }
        ExprKind::Binary {
            op: BinOp::Or,
            left,
            right,
        } if !when => {
            collect_proofs(left, false, into);
            collect_proofs(right, false, into);
        }
        // Where `a || b` is true one of the two is: a tag path each side proves
        // one of a set of variants is one of the two sets (docs/adr/0092,
        // section 6.1 item 2), and nothing else is proven.
        ExprKind::Binary {
            op: BinOp::Or,
            left,
            right,
        } => {
            let either = proves(left, true).or(&proves(right, true));
            *into = into.and(&either);
        }
        // Where `a && b` is false one of the two is false: the same widening,
        // of what each side proves where it is false.
        ExprKind::Binary {
            op: BinOp::And,
            left,
            right,
        } => {
            let either = proves(left, false).or(&proves(right, false));
            *into = into.and(&either);
        }
        _ => {}
    }
}

/// The subject `side` names when `other` is `null`.
fn tested(side: &TypedExpr, other: &TypedExpr) -> Option<String> {
    matches!(other.kind, ExprKind::NullLit)
        .then(|| subject(side))
        .flatten()
}

/// The tag path `side` names and the variant `other` writes, when `side` is a
/// member read of a subject (`when.kind`, a path of two names or more) and
/// `other` is `E.v`, a variant of an enum as written: what a test of a union's
/// tag compares (docs/adr/0092, section 6.1). Whether the path is a tag is the
/// judge's to say; a comparison of two such paths is recorded both ways, and
/// the one that names no tag is never asked for.
fn tag_test(side: &TypedExpr, other: &TypedExpr) -> Option<(String, Variant)> {
    let path = subject(side).filter(|path| path.contains('.'))?;
    let ExprKind::Member { object, property } = &other.kind else {
        return None;
    };
    let ExprKind::Ident(alias) = &object.kind else {
        return None;
    };
    Some((
        path,
        Variant {
            alias: alias.clone(),
            name: property.clone(),
        },
    ))
}

/// The facts each part of an `<if>` / `<elseif>` / `<else>` chain is read with,
/// given those at its entry and its conditions in order: each condition where
/// every earlier one is false, each block where its own is true besides, and the
/// `else` where every one is false.
#[derive(Debug, Clone)]
pub struct Chain {
    conds: Vec<Facts>,
    blocks: Vec<Facts>,
    otherwise: Facts,
}

impl Chain {
    /// The chain of `conds` (as written) entered with `entry`.
    pub fn of(entry: &Facts, conds: &[&str]) -> Self {
        let mut falsified = entry.clone();
        let mut chain = Chain {
            conds: Vec::with_capacity(conds.len()),
            blocks: Vec::with_capacity(conds.len()),
            otherwise: Facts::none(),
        };
        for cond in conds {
            chain.conds.push(falsified.clone());
            chain
                .blocks
                .push(falsified.and(&proves_written(cond, true)));
            falsified = falsified.and(&proves_written(cond, false));
        }
        chain.otherwise = falsified;
        chain
    }

    /// The facts the `k`th condition is read with.
    pub fn cond(&self, k: usize) -> Facts {
        self.conds.get(k).cloned().unwrap_or_default()
    }

    /// The facts the `k`th condition's block is read with.
    pub fn block(&self, k: usize) -> Facts {
        self.blocks.get(k).cloned().unwrap_or_default()
    }

    /// The facts the `else` block is read with.
    pub fn otherwise(&self) -> Facts {
        self.otherwise.clone()
    }
}

/// The facts a loop's body starts each turn with, entered with `entry`: those
/// nothing in the body ends, since a turn follows the end of the one before.
pub fn loop_entry<S: Statement>(entry: &Facts, body: &[S]) -> Facts {
    entry.after(&Writes::of_block(body))
}

/// Whether `block` returns on every path: its last statement is an
/// `<sce:return>`, or an `<sce:if>` with an `<sce:else>` whose two blocks each
/// return on every path.
pub fn always_returns(block: &[AlgorithmStmt]) -> bool {
    match block.last() {
        Some(AlgorithmStmt::Return { .. }) => true,
        Some(AlgorithmStmt::If {
            then_body,
            else_body: Some(else_body),
            ..
        }) => always_returns(then_body) && always_returns(else_body),
        _ => false,
    }
}

/// The facts the statements after `statement` are read with, in the block that
/// holds it, entered with `before`: what it writes ends a fact; an
/// `<sce:require>` proves what its condition does where it is true, since the
/// algorithm has failed otherwise; an `<sce:if>` with no `<sce:else>` whose block
/// returns on every path proves what its condition does where it is false. The
/// one sequencing rule of an algorithm's body.
pub fn after_algorithm_statement(before: &Facts, statement: &AlgorithmStmt) -> Facts {
    let after = before.after(&Writes::of(statement));
    match statement {
        AlgorithmStmt::Require { cond, .. } => after.and(&proves_written(cond, true)),
        AlgorithmStmt::If {
            cond,
            then_body,
            else_body: None,
            ..
        } if always_returns(then_body) => before.and(&proves_written(cond, false)),
        _ => after,
    }
}

/// Every expression of the algorithm `m`'s body judged by the rules of an
/// optional value (docs/adr/0055) with the subjects proven present where it
/// stands: the sequencing of [`after_algorithm_statement`], an `<sce:if>`'s
/// [`Chain`], and a loop's [`loop_entry`], an `<sce:while>`'s condition proving
/// its facts at the start of each turn. `imports` are the algorithm's own,
/// enriched. Judged once, before any backend renders the body, so the verdict
/// does not depend on the backend; a body that holds no optional value and
/// returns none has nothing to judge here.
pub(crate) fn check_algorithm(
    m: &crate::forge::model::AlgorithmModel,
    imports: &[crate::forge::generator::ImportContext],
    options: &crate::ForgeCompileOptions,
) -> Result<(), crate::forge::error::ForgeError> {
    let types = crate::forge::generator::AlgorithmTypes::collect(m, imports, options)?;
    let returns_optional = m
        .signature
        .return_type
        .as_ref()
        .is_some_and(|ty| ty.optional_elem().is_some());
    if !types.holds_optional() && !returns_optional {
        return Ok(());
    }
    let ctx = types.type_ctx(m, imports);
    let returns_return = m.signature.return_type.as_ref();
    let walk = AlgorithmWalk {
        ctx: &ctx,
        returns: returns_return.map_or(InferredType::Unknown, value_type),
        returns_alias: returns_return
            .and_then(crate::forge::model::AlgorithmValueType::optional_elem)
            .and_then(crate::forge::model::AlgorithmValueType::record_alias)
            .map(str::to_string),
        records: types.record_aliases(),
        optional_records: types.optional_record_aliases(),
    };
    walk.block(&m.body, &Facts::none())
}

/// The type a place declared `ty` takes a value of: a scalar's, an optional
/// value's, and `Unknown` for a list, a record and an enum, which the lattice
/// names no type for.
fn value_type(ty: &crate::forge::model::AlgorithmValueType) -> InferredType {
    if let Some(elem) = ty.optional_elem() {
        return InferredType::Optional(OptionalElem::of_value(elem));
    }
    ty.scalar()
        .map_or(InferredType::Unknown, InferredType::from_sce_type)
}

/// The walk of one algorithm's body ([`check_algorithm`]).
struct AlgorithmWalk<'w, 'c> {
    ctx: &'w TypeCtx<'c>,
    /// The type the algorithm returns.
    returns: InferredType,
    /// The alias of the schema of the optional record the algorithm returns, when
    /// it returns one (docs/adr/0055, step 9).
    returns_alias: Option<String>,
    /// Each record parameter, local and loop item by name, with the alias of the
    /// schema that types it.
    records: Vec<(String, String)>,
    /// Each optional record parameter and local by name, with the alias of the
    /// schema that types it.
    optional_records: Vec<(String, String)>,
}

impl AlgorithmWalk<'_, '_> {
    /// The alias of the schema of the optional record the local or parameter
    /// `name` is, if it is one.
    fn optional_record_alias(&self, name: &str) -> Option<&str> {
        self.optional_records
            .iter()
            .find(|(held, _)| held == name)
            .map(|(_, alias)| alias.as_str())
    }

    /// The refusal of `expr`, written at `spelling`, when it names a record of
    /// another schema than the optional record of `alias` it is put into: a
    /// record is put into an optional record by its name, and is of its schema
    /// (docs/adr/0055, step 9). A value that is no record's name is the judge's.
    fn record_fits(
        &self,
        expr: &str,
        spelling: Option<&crate::attribute_spelling::AttributeSpelling>,
        alias: Option<&str>,
    ) -> Result<(), crate::forge::error::ForgeError> {
        let name = expr.trim();
        let Some(alias) = alias else {
            return Ok(());
        };
        let held = self
            .records
            .iter()
            .chain(self.optional_records.iter())
            .find(|(held, _)| held == name)
            .map(|(_, held)| held.as_str());
        match held {
            Some(held) if held != alias => {
                let refusal = crate::forge::error::ExprError::UnsupportedConstruct {
                    construct: format!(
                        "`{name}`, a record of `{held}`, where an optional record of `{alias}` \
                         is declared"
                    ),
                    observed: Some(name.to_string()),
                }
                .at(None);
                Err(ExpressionSite::new(expr, spelling).place(refusal))
            }
            _ => Ok(()),
        }
    }

    /// `body`, entered with `entry` proven.
    fn block(
        &self,
        body: &[AlgorithmStmt],
        entry: &Facts,
    ) -> Result<(), crate::forge::error::ForgeError> {
        let mut proven = entry.clone();
        for statement in body {
            self.statement(statement, &proven)?;
            proven = after_algorithm_statement(&proven, statement);
        }
        Ok(())
    }

    fn statement(
        &self,
        statement: &AlgorithmStmt,
        proven: &Facts,
    ) -> Result<(), crate::forge::error::ForgeError> {
        let hint = Expected::Hint(InferredType::Unknown);
        match statement {
            AlgorithmStmt::Var {
                sce_type,
                init: Some(init),
                init_spelling,
                ..
            } => {
                // An optional record starts absent: a record is built by its
                // `<sce:set>`s, which a local of this type has none of
                // (docs/adr/0055, step 9).
                if sce_type
                    .optional_elem()
                    .is_some_and(|elem| elem.record_alias().is_some())
                    && !crate::forge::static_datamodel::is_null_literal(init)
                {
                    let refusal = crate::forge::error::ExprError::UnsupportedConstruct {
                        construct: format!(
                            "the initial value `{}` of a local of type {}, which starts absent \
                             (`init=\"null\"`): a record is put into it by name where the \
                             algorithm runs",
                            init.trim(),
                            sce_type.as_attr()
                        ),
                        observed: Some(init.trim().to_string()),
                    }
                    .at(None);
                    return Err(ExpressionSite::new(init, init_spelling.as_ref()).place(refusal));
                }
                self.judge(
                    init,
                    init_spelling.as_ref(),
                    Expected::Slot(value_type(sce_type)),
                    proven,
                )
            }
            AlgorithmStmt::Var { init: None, .. } => Ok(()),
            AlgorithmStmt::RecordVar { name, fields, .. } => {
                for field in fields {
                    let slot = self.ctx.lookup_var(&format!("{name}.{}", field.name));
                    self.judge(
                        &field.expr,
                        field.expr_spelling.as_ref(),
                        Expected::Slot(slot),
                        proven,
                    )?;
                }
                Ok(())
            }
            AlgorithmStmt::RecordFromCall {
                init,
                init_spelling,
                ..
            } => self.judge(init, init_spelling.as_ref(), hint, proven),
            AlgorithmStmt::Assign {
                target,
                target_spelling,
                expr,
                expr_spelling,
            } => {
                let target = target.trim();
                // An optional value is written whole: `null` makes it absent and
                // a value present. Its `.value` is a read, and no place to write.
                if let Some(holder) = crate::forge::static_enum::present_value_of(target)
                    .filter(|holder| self.ctx.lookup_var(holder.trim()).is_optional())
                {
                    let at = ExpressionSite::new(target, target_spelling.as_ref())
                        .locate(Some(0..target.len()));
                    let refusal: crate::forge::error::ForgeError =
                        crate::forge::error::ExprError::UnsupportedConstruct {
                            construct: format!(
                                "an assignment to `{target}`, the value of an optional value \
                                 (an optional value is assigned whole: <sce:assign \
                                 target=\"{}\"> writes `null` or a value)",
                                holder.trim()
                            ),
                            observed: at.observed(),
                        }
                        .into();
                    return Err(at.place(refusal));
                }
                // A field of the value of an optional record is written only
                // where a test proves the record present, as it is read
                // (docs/adr/0055, step 9): there is no field to write in an
                // absent record, and no write that makes one.
                if let Some(holder) = present_record_of(target)
                    .filter(|holder| self.optional_record_alias(holder).is_some())
                {
                    if !proven.holds(holder) {
                        let at = ExpressionSite::new(target, target_spelling.as_ref())
                            .locate(Some(0..target.len()));
                        let refusal: crate::forge::error::ForgeError =
                            crate::forge::error::ExprError::UnsupportedConstruct {
                                construct: format!(
                                    "a write to `{target}`, a field of the value of the optional \
                                     record `{holder}`, where no test proves `{holder}` present \
                                     (test it first: `{holder} !== null` around the write)"
                                ),
                                observed: at.observed(),
                            }
                            .into();
                        return Err(at.place(refusal));
                    }
                }
                // A record put whole into an optional record is of its schema.
                self.record_fits(
                    expr,
                    expr_spelling.as_ref(),
                    self.optional_record_alias(target),
                )?;
                let slot = self.ctx.lookup_var(target);
                self.judge(expr, expr_spelling.as_ref(), Expected::Slot(slot), proven)
            }
            AlgorithmStmt::Append {
                expr,
                expr_spelling,
                ..
            } => self.judge(expr, expr_spelling.as_ref(), hint, proven),
            AlgorithmStmt::If {
                cond,
                cond_spelling,
                then_body,
                else_body,
            } => {
                let chain = Chain::of(proven, &[cond.as_str()]);
                self.judge(
                    cond,
                    cond_spelling.as_ref(),
                    Expected::Slot(InferredType::Bool),
                    &chain.cond(0),
                )?;
                self.block(then_body, &chain.block(0))?;
                match else_body {
                    Some(else_body) => self.block(else_body, &chain.otherwise()),
                    None => Ok(()),
                }
            }
            AlgorithmStmt::While {
                cond,
                cond_spelling,
                body,
                ..
            } => {
                let turn = loop_entry(proven, body);
                self.judge(
                    cond,
                    cond_spelling.as_ref(),
                    Expected::Slot(InferredType::Bool),
                    &turn,
                )?;
                self.block(body, &turn.and(&proves_written(cond, true)))
            }
            AlgorithmStmt::Foreach { body, .. } => self.block(body, &loop_entry(proven, body)),
            AlgorithmStmt::Return {
                expr: Some(expr),
                expr_spelling,
            } => {
                self.record_fits(expr, expr_spelling.as_ref(), self.returns_alias.as_deref())?;
                self.judge(
                    expr,
                    expr_spelling.as_ref(),
                    Expected::Slot(self.returns),
                    proven,
                )
            }
            AlgorithmStmt::Return { expr: None, .. } => Ok(()),
            AlgorithmStmt::Call { args, .. } => {
                for arg in args {
                    self.judge(&arg.expr, arg.spelling.as_ref(), hint, proven)?;
                }
                Ok(())
            }
            AlgorithmStmt::Require {
                cond,
                cond_spelling,
            } => self.judge(
                cond,
                cond_spelling.as_ref(),
                Expected::Slot(InferredType::Bool),
                proven,
            ),
        }
    }

    /// `expr`, written at `spelling`, judged where `proven` holds.
    fn judge(
        &self,
        expr: &str,
        spelling: Option<&crate::attribute_spelling::AttributeSpelling>,
        expected: Expected,
        proven: &Facts,
    ) -> Result<(), crate::forge::error::ForgeError> {
        let ctx = self.ctx.proving(proven.clone());
        judge_optional_use(expr, &ctx, expected)
            .map_err(|refusal| ExpressionSite::new(expr, spelling).place(refusal))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(subjects: &[&str]) -> Facts {
        Facts {
            present: subjects.iter().map(|s| s.to_string()).collect(),
            cases: BTreeMap::new(),
        }
    }

    fn variant(alias: &str, name: &str) -> Variant {
        Variant {
            alias: alias.into(),
            name: name.into(),
        }
    }

    /// The cases the tag path `path` may be in where `cond` is `when`, read for
    /// the enum `K` and the variants `a`, `b` and `c`: those [`CaseFact::admits`]
    /// lets through, every one where no test names the path.
    fn left(cond: &str, when: bool, path: &str) -> Vec<&'static str> {
        let facts = proves_written(cond, when);
        ["a", "b", "c"]
            .into_iter()
            .filter(|name| !matches!(facts.cases_of(path), Some(fact) if !fact.admits("K", name)))
            .collect()
    }

    #[test]
    fn a_test_of_a_tag_proves_its_case_where_it_holds_and_not_where_it_fails() {
        assert_eq!(left("s.kind === K.a", true, "s.kind"), ["a"]);
        assert_eq!(left("K.a === s.kind", true, "s.kind"), ["a"]);
        assert_eq!(left("s.kind === K.a", false, "s.kind"), ["b", "c"]);
        assert_eq!(left("s.kind !== K.a", true, "s.kind"), ["b", "c"]);
        assert_eq!(left("s.kind !== K.a", false, "s.kind"), ["a"]);
        assert_eq!(left("!(s.kind === K.a)", true, "s.kind"), ["b", "c"]);
        // A conjunction narrows by both; a test of another path proves nothing
        // of this one.
        assert_eq!(
            left("s.kind !== K.a && s.kind !== K.b", true, "s.kind"),
            ["c"]
        );
        assert_eq!(left("t.kind === K.a", true, "s.kind"), ["a", "b", "c"]);
        // A disjunction that is true leaves either side's cases.
        assert_eq!(
            left("s.kind === K.a || s.kind === K.b", true, "s.kind"),
            ["a", "b"]
        );
        assert_eq!(
            left("s.kind === K.a || s.kind === K.b", false, "s.kind"),
            ["c"]
        );
        // One side of a true disjunction that says nothing of the path leaves
        // every case.
        assert_eq!(
            left("s.kind === K.a || x !== null", true, "s.kind"),
            ["a", "b", "c"]
        );
        // A conjunction that is false is one of two false sides.
        assert_eq!(
            left("s.kind !== K.a && s.kind !== K.b", false, "s.kind"),
            ["a", "b"]
        );
        // A variant of another enum is no variant of this tag.
        assert_eq!(left("s.kind === L.a", true, "s.kind"), Vec::<&str>::new());
        // A disjunction proves no subject present.
        assert_eq!(
            proves_written("a !== null || b !== null", true),
            Facts::none()
        );
        // An element read by its index is no subject.
        assert!(proves_written("xs[0].kind === K.a", true)
            .cases_of("xs[0].kind")
            .is_none());
    }

    #[test]
    fn a_fact_about_a_tag_ends_with_a_write_of_its_union_and_not_of_its_field() {
        let mut held = Facts::none();
        held.insert_case("s.kind".into(), variant("K", "a"), true);
        assert_eq!(held.after(&writing(&["s.count"])), held);
        assert_eq!(held.after(&writing(&["s"])), Facts::none());
        // A chain's else is read where every test failed.
        let chain = Chain::of(&Facts::none(), &["s.kind === K.a", "s.kind === K.b"]);
        let otherwise = chain.otherwise();
        let fact = otherwise.cases_of("s.kind").expect("a fact about the tag");
        assert!(!fact.admits("K", "a") && !fact.admits("K", "b") && fact.admits("K", "c"));
        let second = chain.block(1);
        let fact = second.cases_of("s.kind").expect("a fact about the tag");
        assert!(!fact.admits("K", "a") && fact.admits("K", "b") && !fact.admits("K", "c"));
    }

    fn writing(paths: &[&str]) -> Writes {
        let mut writes = Writes::default();
        for path in paths {
            writes.write(path);
        }
        writes
    }

    #[test]
    fn a_test_proves_its_subject_where_it_says_present() {
        assert_eq!(proves_written("due !== null", true), facts(&["due"]));
        assert_eq!(proves_written("null !== due", true), facts(&["due"]));
        assert_eq!(proves_written("due !== null", false), Facts::none());
        assert_eq!(proves_written("due === null", false), facts(&["due"]));
        assert_eq!(proves_written("due === null", true), Facts::none());
        assert_eq!(proves_written("!(due === null)", true), facts(&["due"]));
        assert_eq!(
            proves_written("(a !== null) && (b.c !== null)", true),
            facts(&["a", "b.c"])
        );
        assert_eq!(
            proves_written("a !== null && b !== null", false),
            Facts::none()
        );
        assert_eq!(
            proves_written("a === null || b === null", false),
            facts(&["a", "b"])
        );
        // An element read by its index and a call are no subjects.
        assert_eq!(proves_written("xs[0].due !== null", true), Facts::none());
        assert_eq!(proves_written("f(x) !== null", true), Facts::none());
    }

    #[test]
    fn a_write_ends_a_fact_about_its_path_below_and_above() {
        let held = facts(&["r.due", "r.value.f", "x"]);
        assert_eq!(held.after(&writing(&["r.due"])), facts(&["r.value.f", "x"]));
        assert_eq!(held.after(&writing(&["r"])), facts(&["x"]));
        assert_eq!(held.after(&writing(&["r.value"])), facts(&["r.due", "x"]));
        assert_eq!(
            held.after(&writing(&["x.f"])),
            facts(&["r.due", "r.value.f"])
        );
        // A path that only begins with the same letters is another path.
        assert_eq!(held.after(&writing(&["xy"])), held);
        let mut anything = Writes::default();
        anything.anything();
        assert_eq!(held.after(&anything), Facts::none());
    }

    #[test]
    fn a_chain_reads_each_branch_where_the_earlier_ones_are_false() {
        let chain = Chain::of(&Facts::none(), &["a === null", "b !== null"]);
        assert_eq!(chain.cond(0), Facts::none());
        assert_eq!(chain.block(0), Facts::none());
        assert_eq!(chain.cond(1), facts(&["a"]));
        assert_eq!(chain.block(1), facts(&["a", "b"]));
        assert_eq!(chain.otherwise(), facts(&["a"]));
    }

    #[test]
    fn only_a_payload_fact_reaches_a_transitions_content() {
        let proven = facts(&["_event.data.due", "due", "_event.datum"]);
        assert_eq!(proven.of_payload(), facts(&["_event.data.due"]));
    }

    fn action(action_type: &str, location: &str, idlocation: &str) -> Action {
        Action {
            action_type: action_type.to_string(),
            location: location.to_string(),
            idlocation: idlocation.to_string(),
            expr: "make()".to_string(),
            ..Default::default()
        }
    }

    fn assign(target: &str) -> AlgorithmStmt {
        AlgorithmStmt::Assign {
            target: target.to_string(),
            target_spelling: None,
            expr: "0".to_string(),
            expr_spelling: None,
        }
    }

    fn returning() -> AlgorithmStmt {
        AlgorithmStmt::Return {
            expr: Some("0".to_string()),
            expr_spelling: None,
        }
    }

    fn if_then(cond: &str, then_body: Vec<AlgorithmStmt>) -> AlgorithmStmt {
        AlgorithmStmt::If {
            cond: cond.to_string(),
            cond_spelling: None,
            then_body,
            else_body: None,
        }
    }

    #[test]
    fn what_executable_content_writes_ends_a_fact() {
        let held = facts(&["rec.due", "sid"]);
        // An assignment of a whole record, a call's among them, to its holder.
        assert_eq!(
            held.after(&Writes::of(&action("assign", "rec", ""))),
            facts(&["sid"])
        );
        assert_eq!(
            held.after(&Writes::of(&action("send", "", "sid"))),
            facts(&["rec.due"])
        );
        assert_eq!(
            held.after(&Writes::of(&action("script", "", ""))),
            Facts::none()
        );
        // A block's writes are its nested statements' too.
        let nested = Action::if_then("x !== null", vec![action("assign", "rec.due", "")]);
        assert_eq!(held.after(&Writes::of(&nested)), facts(&["sid"]));
        // A log writes nothing.
        assert_eq!(held.after(&Writes::of(&action("log", "", ""))), held);
    }

    #[test]
    fn a_record_an_algorithm_takes_from_a_call_ends_a_fact_about_it() {
        let taken = AlgorithmStmt::RecordFromCall {
            name: "r".to_string(),
            name_spelling: None,
            alias: "Due".to_string(),
            type_spelling: None,
            init: "make()".to_string(),
            init_spelling: None,
        };
        let held = facts(&["r.due", "item.due"]);
        assert_eq!(
            after_algorithm_statement(&held, &taken),
            facts(&["item.due"])
        );
        assert_eq!(
            after_algorithm_statement(&held, &assign("r")),
            facts(&["item.due"])
        );
    }

    #[test]
    fn a_guard_and_an_early_return_prove_what_follows() {
        let require = AlgorithmStmt::Require {
            cond: "item.due !== null".to_string(),
            cond_spelling: None,
        };
        assert_eq!(
            after_algorithm_statement(&Facts::none(), &require),
            facts(&["item.due"])
        );
        let early = if_then("item.due === null", vec![returning()]);
        assert_eq!(
            after_algorithm_statement(&Facts::none(), &early),
            facts(&["item.due"])
        );
        // A block that does not return on every path proves nothing after it.
        let falls_through = if_then("item.due === null", vec![assign("n")]);
        assert_eq!(
            after_algorithm_statement(&Facts::none(), &falls_through),
            Facts::none()
        );
        assert!(always_returns(&[AlgorithmStmt::If {
            cond: "c".to_string(),
            cond_spelling: None,
            then_body: vec![returning()],
            else_body: Some(vec![returning()]),
        }]));
        assert!(!always_returns(&[if_then("c", vec![returning()])]));
    }

    #[test]
    fn a_loop_keeps_a_fact_only_if_its_body_ends_none() {
        let held = facts(&["item.due", "r.due"]);
        assert_eq!(loop_entry(&held, &[assign("n")]), held);
        assert_eq!(
            loop_entry(&held, &[if_then("c", vec![assign("r.due")])]),
            facts(&["item.due"])
        );
    }

    #[test]
    fn a_write_below_the_present_value_of_a_record_ends_no_fact_about_the_record() {
        // A field written under a test of `pick` changes what the record holds and
        // not whether it is present, so a second field may be written after the
        // first.
        let held = facts(&["pick", "pick.value.due", "other"]);
        assert_eq!(held.after(&writing(&["pick.value.count"])), held);
        // It ends a fact about that field, and the record itself written ends every
        // fact about it and below it.
        assert_eq!(
            held.after(&writing(&["pick.value.due"])),
            facts(&["pick", "other"])
        );
        assert_eq!(held.after(&writing(&["pick"])), facts(&["other"]));
        // A path that only begins with the same letters is another path.
        assert_eq!(held.after(&writing(&["picked.value.count"])), held);
    }

    #[test]
    fn a_field_written_through_an_optional_record_names_its_holder() {
        assert_eq!(present_record_of("pick.value.count"), Some("pick"));
        assert_eq!(present_record_of(" pick.value.count "), Some("pick"));
        assert_eq!(present_record_of("pick.value"), None);
        assert_eq!(present_record_of("pick.count"), None);
        assert_eq!(present_record_of("pick.value.a.b"), None);
        assert_eq!(present_record_of("a.b.value.c"), None);
        assert_eq!(present_record_of("pick.value.a[0]"), None);
    }
}
