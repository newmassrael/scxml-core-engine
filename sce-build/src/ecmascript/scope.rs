// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The names a document's expressions are allowed to reach for.
//!
//! [`super::builtins`] answers what SCE's standard library carries, which
//! is enough to judge `words.map(...)` — the receiver's type does not
//! matter, the *method name* does. It is not enough to judge `Date.now()`
//! or `conut + 1`. Both are bare identifiers, and whether one names
//! something depends on the document: `Date` could be a `<data id="Date">`
//! and `conut` could be a `<script>`-declared variable. An expression
//! emitter sees one expression, so it cannot tell a misspelling from a
//! reference, and before this module it guessed *reference* every time —
//! `conut + 1` lowered to Lua `conut + 1`, generated on every backend, and
//! died at runtime performing arithmetic on a nil value.
//!
//! This type is the missing input. It is assembled once per document,
//! from the model, and handed to the frontend alongside the source, so the
//! question "does this name exist?" has an answer at generation time.
//!
//! # What counts as declared
//!
//! Everything that puts a name in the datamodel's global table, because
//! that table is what the generated Lua reads:
//!
//! * `<data id>`, at document level and inside any state.
//! * `<foreach item>` / `<foreach index>` — the loop writes them, and the
//!   body reads them from a *different* chunk, so they cannot be lexical.
//! * `<assign location>` and `<send idlocation>` naming a bare identifier.
//!   ECMAScript's sloppy mode creates a global on assignment and the
//!   emitted Lua does the same; a document that assigns before it reads is
//!   writing legal code for this datamodel.
//! * A `<script>` chunk's top-level `var` and `function` declarations.
//!   [`super::lua`] emits those without `local` precisely so a later
//!   `cond` can read them (W3C test 302), which makes them document scope
//!   rather than chunk scope.
//! * The system variables the specification reserves and the tables SCE
//!   installs — see [`super::builtins::INSTALLED_GLOBALS`].
//!
//! Names bound *inside* one expression — function parameters, a `var` in a
//! function body, a `for (var k in o)` — are not here. They are lexical,
//! [`super::resolve`] tracks them as it walks, and putting them in a
//! document-wide set would let one function's parameter silence a typo in
//! another.

use std::collections::BTreeSet;

use super::{Expr, Stmt};
use crate::model::{Action, Invoke, SCXMLModel, State};

/// How much of a document has been read into a scope.
///
/// A build-time caller always reads all of it — the model is on disk
/// before the first expression is lowered, so [`ScopeStage::Everything`]
/// is the only stage [`DocumentScope::from_model`] can mean. A caller
/// that lowers *while the document runs* does not have that: at the
/// moment the first `<transition cond>` is evaluated, a `<script>` further
/// down has not run and an `<assign>` further down has not written. Its
/// scope grows through these stages in order.
///
/// The variants are ordered by containment: each admits everything the
/// one before it does. That is what makes a difference between two of
/// them attributable — a site that lowers differently at
/// [`ScopeStage::WriteTargets`] than at [`ScopeStage::Everything`] does
/// so because of a `<script>` declaration and nothing else.
///
/// This exists because "who maintains the scope, and when" is the open
/// question against a run-time lowering surface, and a question asked in
/// prose gets answered in prose. Naming the stages lets it be counted:
/// see `sce-build/tests/scope_obligation.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScopeStage {
    /// Nothing of the document has been read — only what SCE installs.
    /// This is what a C surface with no scope handle would offer.
    Installed,
    /// Plus every `<data id>`, at document level and in any state. The
    /// specification's early binding puts all of them in the datamodel
    /// before the first macrostep, so a run-time caller can reach this
    /// stage from the model alone, before running anything.
    DataModel,
    /// Plus the names a `<assign location>`, `<send idlocation>` or
    /// `<foreach item>`/`<foreach index>` brings into existence by
    /// writing to it. These appear as the document *runs*.
    WriteTargets,
    /// Plus what a `<script>` or `<finalize>` body declares at its top
    /// level. These appear when that chunk executes, which is the latest
    /// any name in this datamodel can arrive.
    Everything,
}

/// The set of names a document's expressions may refer to.
#[derive(Debug, Clone, Default)]
pub struct DocumentScope {
    declared: BTreeSet<String>,
}

impl DocumentScope {
    /// The scope `model`'s own declarations produce, on top of the names
    /// SCE installs.
    pub fn from_model(model: &SCXMLModel) -> Self {
        Self::from_model_upto(model, ScopeStage::Everything)
    }

    /// The scope a caller has once it has read `model` as far as `stage`.
    ///
    /// [`Self::from_model`] is this at [`ScopeStage::Everything`], which
    /// is the only stage a build-time caller is ever in. The earlier
    /// stages exist for the caller that does not have the whole document
    /// yet, and for measuring how much that costs.
    pub fn from_model_upto(model: &SCXMLModel, stage: ScopeStage) -> Self {
        let mut scope = Self::installed();
        if stage == ScopeStage::Installed {
            return scope;
        }
        for var in &model.variables {
            scope.declare(&var.id);
        }
        if stage >= ScopeStage::Everything {
            for script in &model.global_scripts {
                scope.declare_chunk(&script.content);
            }
        }
        for state in model.states.values() {
            scope.absorb_state(state, stage);
        }
        scope
    }

    /// A scope carrying only what SCE installs — no document has been
    /// read. Used by callers that lower an expression standing on its
    /// own, and as the base every document scope grows from.
    pub fn installed() -> Self {
        Self {
            declared: super::builtins::INSTALLED_GLOBALS
                .iter()
                .map(|n| (*n).to_string())
                .collect(),
        }
    }

    /// [`Self::installed`] plus the named declarations, for a caller that
    /// knows its own environment without holding a model.
    pub fn declaring<S: AsRef<str>>(names: impl IntoIterator<Item = S>) -> Self {
        let mut scope = Self::installed();
        for name in names {
            scope.declare(name.as_ref());
        }
        scope
    }

    /// Record `name` as declared. A name that is not a plain identifier
    /// — `<data id>` accepts anything XML does — is kept verbatim,
    /// because that is how [`super::lua`] addresses it (`_ENV["…"]`).
    pub fn declare(&mut self, name: &str) {
        if !name.is_empty() {
            self.declared.insert(name.to_string());
        }
    }

    /// Record what a `<script>` chunk's top level declares.
    ///
    /// Only the top level: [`super::lua::emit_script`] emits a nested
    /// `var` as a Lua `local`, so nothing below the chunk's own statement
    /// list escapes into the datamodel. A chunk the parser refuses
    /// declares nothing and is left to the refusal it already earns —
    /// this is a name collector, not a second validator.
    pub fn declare_chunk(&mut self, source: &str) {
        if source.trim().is_empty() {
            return;
        }
        let Ok(stmts) = super::parser::parse_script(source) else {
            return;
        };
        for name in chunk_declarations(&stmts) {
            self.declare(&name);
        }
    }

    pub fn declares(&self, name: &str) -> bool {
        self.declared.contains(name)
    }

    /// The declared names close enough to `name` to be what the author
    /// meant, nearest first.
    ///
    /// This is the whole value of resolving names for a machine reader:
    /// `conut` is not merely undeclared, it is `count` with two letters
    /// swapped, and a diagnostic that says so can be repaired without
    /// reading the document. The distance bound scales with length so a
    /// two-letter name does not match every other two-letter name.
    pub fn candidates_for(&self, name: &str) -> Vec<String> {
        let budget = match name.chars().count() {
            0..=3 => 1,
            4..=7 => 2,
            _ => 3,
        };
        let mut scored: Vec<(usize, &String)> = self
            .declared
            .iter()
            .filter(|candidate| !candidate.starts_with('_'))
            .filter_map(|candidate| {
                let distance = edit_distance(name, candidate);
                (distance <= budget).then_some((distance, candidate))
            })
            .collect();
        scored.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(b.1)));
        scored
            .into_iter()
            .take(SUGGESTION_LIMIT)
            .map(|(_, name)| name.clone())
            .collect()
    }

    fn absorb_state(&mut self, state: &State, stage: ScopeStage) {
        for var in &state.datamodel {
            self.declare(&var.id);
        }
        if stage < ScopeStage::WriteTargets {
            return;
        }
        for transition in &state.transitions {
            for action in &transition.actions {
                self.absorb_action(action, stage);
            }
        }
        for block in state
            .on_entry_blocks
            .iter()
            .chain(state.on_exit_blocks.iter())
        {
            for action in block {
                self.absorb_action(action, stage);
            }
        }
        for action in &state.initial_transition_actions {
            self.absorb_action(action, stage);
        }
        for action in &state.initial_history_default_actions {
            self.absorb_action(action, stage);
        }
        if stage < ScopeStage::Everything {
            return;
        }
        for invoke in &state.invokes {
            // A `<finalize>` body is a chunk of this document, run in this
            // document's datamodel, so its top-level declarations are this
            // document's too.
            if let Invoke::Scxml(info) = invoke {
                self.declare_chunk(&info.finalize_content);
            }
        }
    }

    fn absorb_action(&mut self, action: &Action, stage: ScopeStage) {
        match action.action_type.as_str() {
            // The location an `<assign>` writes. A name that did not
            // exist before now does.
            "assign" => self.declare_location(&action.location),
            "send" => self.declare_location(&action.idlocation),
            "foreach" => {
                // `item` and `index` are declared by the loop and read by
                // its body, which is lowered as its own chunk.
                self.declare_location(&action.item);
                self.declare_location(&action.index);
            }
            "script"
                if stage >= ScopeStage::Everything
                    && !action.is_cpp_function
                    && !action.is_kt_function =>
            {
                self.declare_chunk(&action.content);
            }
            _ => {}
        }
        for nested in action
            .actions
            .iter()
            .chain(action.then_actions.iter())
            .chain(action.else_actions.iter())
        {
            self.absorb_action(nested, stage);
        }
        for branch in &action.elseif_branches {
            for nested in &branch.actions {
                self.absorb_action(nested, stage);
            }
        }
    }

    /// Declare a write target, when it names a variable rather than a
    /// path into one. `Var1.field` writes through `Var1`, which must
    /// already exist, so it declares nothing.
    fn declare_location(&mut self, location: &str) {
        let trimmed = location.trim();
        if trimmed.is_empty() || !is_plain_identifier(trimmed) {
            return;
        }
        self.declare(trimmed);
    }
}

/// At most this many suggestions ride a diagnostic. A repair the consumer
/// has to choose from twenty candidates is not a repair.
const SUGGESTION_LIMIT: usize = 3;

fn is_plain_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' || c == '$' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

/// The names a chunk's top level puts in the datamodel.
///
/// `var` and `function` at the chunk's own level, plus any assignment to
/// a name the chunk never declared — ECMAScript's implicit global, which
/// reaches nested statements and function bodies alike, so the walk goes
/// all the way down for assignments and stays at the top for declarations.
fn chunk_declarations(stmts: &[Stmt]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
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
            _ => {}
        }
    }
    for stmt in stmts {
        collect_assigned_names(stmt, &mut out);
    }
    out
}

fn collect_assigned_names(stmt: &Stmt, out: &mut BTreeSet<String>) {
    match stmt {
        Stmt::Expr(expr) | Stmt::Return(Some(expr)) => collect_assigned_in_expr(expr, out),
        Stmt::VarDecl(bindings) => {
            for (_, init) in bindings {
                if let Some(expr) = init {
                    collect_assigned_in_expr(expr, out);
                }
            }
        }
        Stmt::If {
            condition,
            consequent,
            alternate,
        } => {
            collect_assigned_in_expr(condition, out);
            for nested in consequent.iter().chain(alternate.iter()) {
                collect_assigned_names(nested, out);
            }
        }
        Stmt::While { condition, body } => {
            collect_assigned_in_expr(condition, out);
            for nested in body {
                collect_assigned_names(nested, out);
            }
        }
        Stmt::For {
            init,
            test,
            update,
            body,
        } => {
            if let Some(init) = init {
                collect_assigned_names(init, out);
            }
            for expr in test.iter().chain(update.iter()) {
                collect_assigned_in_expr(expr, out);
            }
            for nested in body {
                collect_assigned_names(nested, out);
            }
        }
        Stmt::ForIn { object, body, .. } => {
            collect_assigned_in_expr(object, out);
            for nested in body {
                collect_assigned_names(nested, out);
            }
        }
        Stmt::FunctionDecl { body, .. } => {
            for nested in body {
                collect_assigned_names(nested, out);
            }
        }
        Stmt::Return(None) | Stmt::Break | Stmt::Continue | Stmt::Empty => {}
    }
}

fn collect_assigned_in_expr(expr: &Expr, out: &mut BTreeSet<String>) {
    if let Expr::Assign { target, value, .. } = expr {
        if let Expr::Ident(name) = target.as_ref() {
            out.insert(name.clone());
        }
        collect_assigned_in_expr(value, out);
        return;
    }
    for child in children(expr) {
        collect_assigned_in_expr(child, out);
    }
}

/// The sub-expressions of `expr`, for walks that do not care which
/// position each one holds.
fn children(expr: &Expr) -> Vec<&Expr> {
    match expr {
        Expr::Number(_)
        | Expr::Str(_)
        | Expr::Bool(_)
        | Expr::Nullish
        | Expr::Ident(_)
        | Expr::This => Vec::new(),
        Expr::Array(items) => items.iter().collect(),
        Expr::Object(props) => props.iter().map(|(_, value)| value).collect(),
        Expr::Member { object, .. } => vec![object.as_ref()],
        Expr::Index { object, index } => vec![object.as_ref(), index.as_ref()],
        Expr::Call { callee, args } | Expr::New { callee, args } => {
            let mut out = vec![callee.as_ref()];
            out.extend(args.iter());
            out
        }
        Expr::Unary { operand, .. } => vec![operand.as_ref()],
        Expr::Update { target, .. } => vec![target.as_ref()],
        Expr::Binary { left, right, .. } | Expr::Logical { left, right, .. } => {
            vec![left.as_ref(), right.as_ref()]
        }
        Expr::Conditional {
            condition,
            consequent,
            alternate,
        } => vec![condition.as_ref(), consequent.as_ref(), alternate.as_ref()],
        // A function literal's body is statements, not expressions; the
        // callers that need it reach it through their own walk.
        Expr::Function { .. } => Vec::new(),
        Expr::Assign { target, value, .. } => vec![target.as_ref(), value.as_ref()],
    }
}

/// Levenshtein distance, bounded by nothing — the strings compared here
/// are identifiers, so the quadratic cost is over names, not documents.
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.is_empty() {
        return b.len();
    }
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    let mut current = vec![0usize; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        current[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let substitution = previous[j] + usize::from(ca != cb);
            current[j + 1] = substitution.min(previous[j + 1] + 1).min(current[j] + 1);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[b.len()]
}
