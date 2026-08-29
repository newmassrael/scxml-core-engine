// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Which authored expressions the ECMAScript frontend refuses.
//!
//! [`crate::filters`] lowers every `expr`, every `cond` and every
//! `<script>` body through [`crate::ecmascript`], and a refusal there is
//! emitted as Lua that raises when the engine evaluates it: the spec
//! requires an unevaluable `cond` to raise `error.execution` and read as
//! false, so refusing at generation time would make W3C tests 309 and 344
//! ungeneratable rather than conformant. [`crate::filters`] carries the
//! clause and the reasoning; this module only reports what it decided.
//!
//! That answer is right about the artifact and wrong about the author.
//! The verdict is reached at generation time and, before this module,
//! reached nobody: `check` answered `status: "ok"`, `generate` exited `0`
//! with artifacts listed, and the parser's message survived only as a
//! string literal inside the generated source. The one reader who saw it
//! was whoever ran the machine and read `error.execution`.
//!
//! This walker reaches the same verdict on the same entry points — it
//! calls [`crate::ecmascript::to_lua_value`],
//! [`crate::ecmascript::to_lua_condition`] and
//! [`crate::ecmascript::to_lua_script`], the functions the filters call —
//! and anchors each refusal on the element that wrote the expression, so
//! it can be reported as the `expression/*` diagnostic the wire contract
//! already carries. `SCE_ERROR_CONTRACT.md` §4 gives the `expression`
//! stage exactly this role ("Stateless-subset rejections, ECMAScript
//! unsupported constructs"); nothing new is added to the wire.
//!
//! # What binds this walker to the filters
//!
//! Two walks over one model can drift, and a checker that has drifted
//! from the rewriter it describes is worse than no checker. The binding
//! is measured rather than argued: every refusal the filters make is
//! observable in the generated artifact as the raise they emit, so
//! `sce-build/tests/ecmascript_acceptance_parity.rs` generates the whole
//! fixture corpus and asserts that the set of raises in the artifacts and
//! the set this module reports are the same set. A site this walker
//! forgets, or one it invents, reds that test.

use crate::ecmascript::{DocumentScope, ExprError};
use crate::forge::error::SourceLocation;
use crate::model::{Action, DoneDataContent, Invoke, Param, SCXMLModel, State, Variable};

/// The four roles an authored expression can have, one per frontend
/// entry point.
///
/// The role is what decides which function lowers the source, so it is
/// also what decides the verdict: `to_lua_condition` applies ECMAScript
/// truthiness where `to_lua_value` does not, `to_lua_script` admits the
/// statement grammar neither of the others does, and `to_lua_location`
/// admits only what can be assigned to. A walker that checked every site
/// as a value would refuse legal `<script>` bodies and accept
/// `<assign location="1 + 1"/>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpressionRole {
    /// `<data expr>`, `<assign expr>`, `<param expr>`, `<log expr>`,
    /// `<send>`'s attribute expressions — evaluated for what they yield.
    Value,
    /// `transition/@cond`, `<if>`, `<elseif>` — evaluated under
    /// ECMAScript truthiness.
    Condition,
    /// A `<script>` body or `<finalize>` body — a statement list.
    Script,
    /// `<assign location>`, `<send idlocation>`, `<foreach item>`,
    /// `<foreach index>` — the places a document *writes*, which the
    /// data model restricts to an identifier or a member path rooted at
    /// one. Unlike the three above, the source is not resolved against
    /// the document's declarations: writing is how this datamodel's
    /// globals come into existence.
    Location,
}

impl ExpressionRole {
    /// The word the raise emitted by [`crate::filters`] uses for this
    /// role. The parity test joins on it, so the two spellings are one
    /// definition rather than two literals that happen to agree.
    pub fn wire_word(self) -> &'static str {
        match self {
            ExpressionRole::Value => "expr",
            ExpressionRole::Condition => "cond",
            ExpressionRole::Script => "script",
            ExpressionRole::Location => "location",
        }
    }

    fn lower(self, source: &str, scope: &DocumentScope) -> Result<String, ExprError> {
        match self {
            ExpressionRole::Value => crate::ecmascript::to_lua_value(source, scope),
            ExpressionRole::Condition => crate::ecmascript::to_lua_condition(source, scope),
            ExpressionRole::Script => crate::ecmascript::to_lua_script(source, scope),
            ExpressionRole::Location => crate::ecmascript::to_lua_location(source),
        }
    }
}

/// One authored expression the frontend refused, anchored on its source.
#[derive(Debug)]
pub struct RefusedExpression {
    pub role: ExpressionRole,
    /// Author-facing name of the attribute or element that carried it —
    /// `<assign expr>`, `<transition cond>`. The message needs it because
    /// a line can hold more than one expression.
    pub site: String,
    /// The expression as the author wrote it.
    pub source: String,
    pub error: ExprError,
    /// The owning element's own coordinate, when the parser recorded one.
    pub location: Option<SourceLocation>,
}

/// The one-line message, in the shape the raise inside the generated
/// artifact already uses so a reader who has met one recognises the
/// other.
///
/// This is also the record's `message` field: `SingleDiagnostic`'s
/// assembly reads `self.to_string()`, which is why the site and the
/// authored source belong in `Display` rather than in a bespoke
/// formatter. A line can carry more than one expression — `<send
/// eventexpr targetexpr>` carries two — so `location` alone does not
/// name which one was refused.
impl std::fmt::Display for RefusedExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} is not valid ECMAScript: {}: {}",
            self.site, self.source, self.error
        )
    }
}

/// One authored expression, named by where it stands, before anything has
/// been asked of it.
///
/// [`sites`] answers *where the expressions are*; [`refusals`] answers
/// *which of them the frontend rejects*. They are separate because the
/// first question has more than one caller: a probe that lowers the same
/// site under two different scopes is asking about the scope rather than
/// about acceptance, and it must not re-implement the walk to do so — the
/// walk reaches fourteen call sites, and a second copy of it would drift
/// from this one without saying so.
#[derive(Debug, Clone)]
pub struct ExpressionSite {
    pub role: ExpressionRole,
    /// Author-facing name of the attribute or element that carried it —
    /// `<assign expr>`, `<transition cond>`.
    pub site: String,
    /// The expression as the author wrote it.
    pub source: String,
    /// The owning element's own coordinate, when the parser recorded one.
    pub location: Option<SourceLocation>,
}

impl ExpressionSite {
    /// Lower this site's source against `scope`, in its own role.
    ///
    /// The role decides which of the four frontend entry points runs, so a
    /// caller cannot pick one and stay correct — `<assign location>` is
    /// not a value and a `<script>` body is not a condition. This is the
    /// only supported way to lower a site.
    pub fn lower(&self, scope: &DocumentScope) -> Result<String, ExprError> {
        self.role.lower(&self.source, scope)
    }
}

/// Every expression `model` carries that the frontend is asked to lower,
/// in document order.
///
/// This is the population, not a verdict. Sites the pipeline never routes
/// through the frontend are absent — an absent attribute, a `cpp:`/`kt:`
/// native guard, a pure `In()` predicate — because including them would
/// inflate every count taken over this walk with expressions no engine
/// ever sees.
pub fn sites(model: &SCXMLModel) -> Vec<ExpressionSite> {
    let mut into = Collector {
        document_needs_script_engine: model.needs_script_engine,
        out: Vec::new(),
    };

    for var in &model.variables {
        check_variable(var, &mut into);
    }
    for script in &model.global_scripts {
        check(
            ExpressionRole::Script,
            "<script>",
            &script.content,
            script.source_location.as_ref(),
            &mut into,
        );
    }

    let mut states: Vec<&State> = model.states.values().collect();
    states.sort_by_key(|s| s.document_order);
    for state in states {
        check_state(state, &mut into);
    }
    into.out
}

/// Every expression in `model` the ECMAScript frontend refuses, in
/// document order.
///
/// An empty vector means the whole document lowers — which is the answer
/// for every document in the W3C corpus except the two that write
/// `cond="return"` on purpose.
pub fn refusals(model: &SCXMLModel) -> Vec<RefusedExpression> {
    // One scope for the document, assembled before any expression is
    // lowered. A `<data>` declared in the last state is in scope for the
    // first state's `cond`: the datamodel is one table, and early
    // binding puts every declaration in it before the first macrostep.
    // Building it per expression would make the verdict depend on
    // document order, which nothing in the spec licenses.
    let scope = DocumentScope::from_model(model);
    sites(model)
        .into_iter()
        .filter_map(|site| {
            site.lower(&scope).err().map(|error| RefusedExpression {
                role: site.role,
                site: site.site,
                source: site.source,
                error,
                location: site.location,
            })
        })
        .collect()
}

/// The walk's accumulator: the sites found so far, and the one fact about
/// the document that a site's inclusion can depend on.
///
/// It carries no scope. The walk used to lower as it went, which tied the
/// population to whichever scope it was judged against and left no way to
/// ask the same walk a second question. Deciding *whether a site is one*
/// and deciding *what it lowers to* are different jobs, and only the first
/// belongs here.
struct Collector {
    /// Whether the templates will route this document's guards through
    /// the frontend at all — see [`check_condition`].
    document_needs_script_engine: bool,
    out: Vec<ExpressionSite>,
}

fn check_state(state: &State, into: &mut Collector) {
    for var in &state.datamodel {
        check_variable(var, into);
    }
    for transition in &state.transitions {
        check_condition(
            "<transition cond>",
            &transition.cond,
            transition.source_location.as_ref(),
            into,
        );
        for action in &transition.actions {
            check_action(action, into);
        }
    }
    for block in state
        .on_entry_blocks
        .iter()
        .chain(state.on_exit_blocks.iter())
    {
        for action in block {
            check_action(action, into);
        }
    }
    for action in &state.initial_transition_actions {
        check_action(action, into);
    }
    for action in &state.initial_history_default_actions {
        check_action(action, into);
    }
    for invoke in &state.invokes {
        check_invoke(invoke, into);
    }
    if let Some(donedata) = &state.donedata {
        for param in &donedata.params {
            if let Some(expr) = &param.expr {
                check(
                    ExpressionRole::Value,
                    "<donedata><param expr>",
                    expr,
                    state.source_location.as_ref(),
                    into,
                );
            }
            // The `location` half, for the reason [`check_params`] gives:
            // it is read, through the same value seam.
            if let Some(location) = &param.location {
                check(
                    ExpressionRole::Value,
                    "<donedata><param location>",
                    location,
                    state.source_location.as_ref(),
                    into,
                );
            }
        }
        // Only the `expr` form can be refused, for the reason
        // [`check_variable`] gives below: `expr` promises an expression,
        // while an inline body has "not an expression" among its legal
        // readings and reaches `filters::to_lua_data_content`, which takes
        // it. A `DoneDataContent::InlineText` therefore has no refusal to
        // report, and a Null-data-model `Literal` never reaches the
        // frontend at all.
        if let DoneDataContent::Expression(text) = &donedata.content {
            check(
                ExpressionRole::Value,
                "<donedata><content expr>",
                text,
                state.source_location.as_ref(),
                into,
            );
        }
    }
}

fn check_variable(var: &Variable, into: &mut Collector) {
    // `<data>`'s inline body is not checked here. The spec makes "not an
    // expression" a legal reading of it, and `filters::to_lua_data_content`
    // — which carries that clause — takes that reading: a body the
    // frontend cannot parse *is a string*, whitespace-normalized. Only
    // `expr` promises to be an expression, so only `expr` can be refused.
    check(
        ExpressionRole::Value,
        "<data expr>",
        &var.expr,
        var.source_location.as_ref(),
        into,
    );
}

fn check_invoke(invoke: &Invoke, into: &mut Collector) {
    match invoke {
        Invoke::Scxml(info) => {
            check(
                ExpressionRole::Script,
                "<finalize>",
                &info.finalize_content,
                info.common.source_location.as_ref(),
                into,
            );
            check_params(
                &info.common.params,
                info.common.source_location.as_ref(),
                into,
            );
        }
        Invoke::Hybrid(info) => {
            check(
                ExpressionRole::Value,
                "<invoke srcexpr>",
                &info.srcexpr,
                info.common.source_location.as_ref(),
                into,
            );
            check(
                ExpressionRole::Value,
                "<invoke contentexpr>",
                &info.contentexpr,
                info.common.source_location.as_ref(),
                into,
            );
            check_params(
                &info.common.params,
                info.common.source_location.as_ref(),
                into,
            );
        }
        Invoke::MeshRpc(info) => {
            check_params(&info.base.params, info.base.source_location.as_ref(), into);
        }
        // An unsupported invoke type raises at entry and starts no
        // session — see [`crate::model::UnsupportedInvokeInfo`] — so
        // nothing under it is ever lowered and nothing can be refused.
        Invoke::Unsupported(_) => {}
    }
}

fn check_params(params: &[Param], fallback: Option<&SourceLocation>, into: &mut Collector) {
    for param in params {
        // A `<param>` carries its own coordinate for exactly this: the
        // enclosing element's line does not contain the rejected value.
        let location = param.source_location.as_ref().or(fallback);
        check(
            ExpressionRole::Value,
            "<param expr>",
            &param.expr,
            location,
            into,
        );
        // `location` is the other half of the same element and is
        // *read* — the value at that location becomes the payload
        // field. Every backend lowers it through the same value seam
        // `expr` goes through, which is why it belongs here and not with
        // the write targets [`crate::ecmascript::to_lua_location`] serves.
        // W3C test 343 is the fixture: `<param location="foo"/>` naming
        // nothing is the illegal `<param>` whose `error.execution` the
        // test waits for.
        check(
            ExpressionRole::Value,
            "<param location>",
            &param.location,
            location,
            into,
        );
    }
}

fn check_action(action: &Action, into: &mut Collector) {
    let at = action.source_location.as_ref();
    match action.action_type.as_str() {
        "send" => {
            check(
                ExpressionRole::Value,
                "<send eventexpr>",
                &action.eventexpr,
                at,
                into,
            );
            check(
                ExpressionRole::Value,
                "<send targetexpr>",
                &action.targetexpr,
                at,
                into,
            );
            check(
                ExpressionRole::Value,
                "<send delayexpr>",
                &action.delayexpr,
                at,
                into,
            );
            check(
                ExpressionRole::Value,
                "<send contentexpr>",
                &action.contentexpr,
                at,
                into,
            );
            check(
                ExpressionRole::Location,
                "<send idlocation>",
                &action.idlocation,
                at,
                into,
            );
            check_params(&action.params, at, into);
        }
        "assign" => {
            check(
                ExpressionRole::Value,
                "<assign expr>",
                &action.expr,
                at,
                into,
            );
            check(
                ExpressionRole::Location,
                "<assign location>",
                &action.location,
                at,
                into,
            );
        }
        "log" => check(ExpressionRole::Value, "<log expr>", &action.expr, at, into),
        "cancel" => check(
            ExpressionRole::Value,
            "<cancel sendidexpr>",
            &action.sendidexpr,
            at,
            into,
        ),
        "foreach" => {
            check(
                ExpressionRole::Value,
                "<foreach array>",
                &action.array,
                at,
                into,
            );
            // The two iteration variables are written once per turn, so
            // they are targets rather than reads — §scxml-4.6 types both
            // as location expressions.
            check(
                ExpressionRole::Location,
                "<foreach item>",
                &action.item,
                at,
                into,
            );
            check(
                ExpressionRole::Location,
                "<foreach index>",
                &action.index,
                at,
                into,
            );
            for nested in &action.actions {
                check_action(nested, into);
            }
        }
        "if" => {
            check_condition("<if cond>", &action.cond, at, into);
            for nested in &action.then_actions {
                check_action(nested, into);
            }
            for branch in &action.elseif_branches {
                check_condition("<elseif cond>", &branch.cond, at, into);
                for nested in &branch.actions {
                    check_action(nested, into);
                }
            }
            for nested in &action.else_actions {
                check_action(nested, into);
            }
        }
        // A `<cpp>` / `<kt>` child is emitted as source in that language
        // and never reaches the ECMAScript frontend, so the guard sits in
        // the arm pattern: an inline `<script>` is the only shape whose
        // body this frontend is asked to read.
        "script" if !action.is_cpp_function && !action.is_kt_function => {
            check(
                ExpressionRole::Script,
                "<script>",
                &action.content,
                at,
                into,
            );
        }
        _ => {}
    }
}

/// A `cond`, skipping the spellings no backend lowers through the
/// frontend.
///
/// Two questions, and the second is about the document rather than the
/// guard. A `cpp:` / `kt:` prefixed condition and a pure `In()`
/// predicate are emitted as native code by every backend, so
/// [`crate::parser::check_expression_needs`] answers the first. But a
/// guard that is *not* one of those reaches the frontend only when the
/// document needs a script engine at all: the templates branch on
/// `model.needs_script_engine`, and a document without one emits every
/// remaining guard verbatim in the target language.
///
/// Asking only the first question is what this used to do, and it was
/// wrong in both directions for a bare call like `cond="shouldCool()"`:
/// the classifier calls it native — it has no operator, no literal and
/// no reserved word — while a document whose `<script>` bodies already
/// forced a script engine lowers it through `to_lua_guard` like any
/// other. The walker reported nothing and the artifact carried a
/// refusal.
fn check_condition(
    site: &str,
    cond: &str,
    location: Option<&SourceLocation>,
    into: &mut Collector,
) {
    let (needs_engine, _has_in) = crate::parser::check_expression_needs(cond);
    if !needs_engine && !into.document_needs_script_engine {
        return;
    }
    if cond.starts_with("cpp:") || cond.starts_with("kt:") {
        return;
    }
    if crate::parser::is_pure_in_predicate(cond) {
        return;
    }
    check(ExpressionRole::Condition, site, cond, location, into);
}

fn check(
    role: ExpressionRole,
    site: &str,
    source: &str,
    location: Option<&SourceLocation>,
    into: &mut Collector,
) {
    // An absent attribute is not an expression. The filters answer the
    // same way — an empty `expr` lowers to nothing, an empty `cond` to
    // `true` — so neither can be refused.
    if source.is_empty() {
        return;
    }
    into.out.push(ExpressionSite {
        role,
        site: site.to_string(),
        source: source.to_string(),
        location: location.cloned(),
    });
}
