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

use crate::ecmascript::ExprError;
use crate::forge::error::SourceLocation;
use crate::model::{Action, DoneDataContent, Invoke, Param, SCXMLModel, State, Variable};

/// The three roles an authored expression can have, one per frontend
/// entry point.
///
/// The role is what decides which function lowers the source, so it is
/// also what decides the verdict: `to_lua_condition` applies ECMAScript
/// truthiness where `to_lua_value` does not, and `to_lua_script` admits
/// the statement grammar neither of the others does. A walker that
/// checked every site as a value would refuse legal `<script>` bodies.
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
        }
    }

    fn lower(self, source: &str) -> Result<String, ExprError> {
        match self {
            ExpressionRole::Value => crate::ecmascript::to_lua_value(source),
            ExpressionRole::Condition => crate::ecmascript::to_lua_condition(source),
            ExpressionRole::Script => crate::ecmascript::to_lua_script(source),
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

/// Every expression in `model` the ECMAScript frontend refuses, in
/// document order.
///
/// An empty vector means the whole document lowers — which is the answer
/// for every document in the W3C corpus except the two that write
/// `cond="return"` on purpose.
pub fn refusals(model: &SCXMLModel) -> Vec<RefusedExpression> {
    let mut out = Vec::new();

    for var in &model.variables {
        check_variable(var, &mut out);
    }
    for script in &model.global_scripts {
        check(
            ExpressionRole::Script,
            "<script>",
            &script.content,
            script.source_location.as_ref(),
            &mut out,
        );
    }

    let mut states: Vec<&State> = model.states.values().collect();
    states.sort_by_key(|s| s.document_order);
    for state in states {
        check_state(state, &mut out);
    }
    out
}

fn check_state(state: &State, out: &mut Vec<RefusedExpression>) {
    for var in &state.datamodel {
        check_variable(var, out);
    }
    for transition in &state.transitions {
        check_condition(
            "<transition cond>",
            &transition.cond,
            transition.source_location.as_ref(),
            out,
        );
        for action in &transition.actions {
            check_action(action, out);
        }
    }
    for block in state
        .on_entry_blocks
        .iter()
        .chain(state.on_exit_blocks.iter())
    {
        for action in block {
            check_action(action, out);
        }
    }
    for action in &state.initial_transition_actions {
        check_action(action, out);
    }
    for action in &state.initial_history_default_actions {
        check_action(action, out);
    }
    for invoke in &state.invokes {
        check_invoke(invoke, out);
    }
    if let Some(donedata) = &state.donedata {
        for param in &donedata.params {
            if let Some(expr) = &param.expr {
                check(
                    ExpressionRole::Value,
                    "<donedata><param expr>",
                    expr,
                    state.source_location.as_ref(),
                    out,
                );
            }
        }
        // Only the expression form reaches the frontend: a literal
        // `<content>` body is used as the value rather than re-evaluated,
        // and [`crate::parser`] is where that reading is made and cited.
        if let DoneDataContent::Expression(text) = &donedata.content {
            check(
                ExpressionRole::Value,
                "<donedata><content expr>",
                text,
                state.source_location.as_ref(),
                out,
            );
        }
    }
}

fn check_variable(var: &Variable, out: &mut Vec<RefusedExpression>) {
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
        out,
    );
}

fn check_invoke(invoke: &Invoke, out: &mut Vec<RefusedExpression>) {
    match invoke {
        Invoke::Scxml(info) => {
            check(
                ExpressionRole::Script,
                "<finalize>",
                &info.finalize_content,
                info.common.source_location.as_ref(),
                out,
            );
            check_params(
                &info.common.params,
                info.common.source_location.as_ref(),
                out,
            );
        }
        Invoke::Hybrid(info) => {
            check(
                ExpressionRole::Value,
                "<invoke srcexpr>",
                &info.srcexpr,
                info.common.source_location.as_ref(),
                out,
            );
            check(
                ExpressionRole::Value,
                "<invoke contentexpr>",
                &info.contentexpr,
                info.common.source_location.as_ref(),
                out,
            );
            check_params(
                &info.common.params,
                info.common.source_location.as_ref(),
                out,
            );
        }
        Invoke::MeshRpc(info) => {
            check_params(&info.base.params, info.base.source_location.as_ref(), out);
        }
        // An unsupported invoke type raises at entry and starts no
        // session — see [`crate::model::UnsupportedInvokeInfo`] — so
        // nothing under it is ever lowered and nothing can be refused.
        Invoke::Unsupported(_) => {}
    }
}

fn check_params(
    params: &[Param],
    fallback: Option<&SourceLocation>,
    out: &mut Vec<RefusedExpression>,
) {
    for param in params {
        // A `<param>` carries its own coordinate for exactly this: the
        // enclosing element's line does not contain the rejected value.
        let location = param.source_location.as_ref().or(fallback);
        check(
            ExpressionRole::Value,
            "<param expr>",
            &param.expr,
            location,
            out,
        );
    }
}

fn check_action(action: &Action, out: &mut Vec<RefusedExpression>) {
    let at = action.source_location.as_ref();
    match action.action_type.as_str() {
        "send" => {
            check(
                ExpressionRole::Value,
                "<send eventexpr>",
                &action.eventexpr,
                at,
                out,
            );
            check(
                ExpressionRole::Value,
                "<send targetexpr>",
                &action.targetexpr,
                at,
                out,
            );
            check(
                ExpressionRole::Value,
                "<send delayexpr>",
                &action.delayexpr,
                at,
                out,
            );
            check(
                ExpressionRole::Value,
                "<send contentexpr>",
                &action.contentexpr,
                at,
                out,
            );
            check_params(&action.params, at, out);
        }
        "assign" => check(
            ExpressionRole::Value,
            "<assign expr>",
            &action.expr,
            at,
            out,
        ),
        "log" => check(ExpressionRole::Value, "<log expr>", &action.expr, at, out),
        "cancel" => check(
            ExpressionRole::Value,
            "<cancel sendidexpr>",
            &action.sendidexpr,
            at,
            out,
        ),
        "foreach" => {
            check(
                ExpressionRole::Value,
                "<foreach array>",
                &action.array,
                at,
                out,
            );
            for nested in &action.actions {
                check_action(nested, out);
            }
        }
        "if" => {
            check_condition("<if cond>", &action.cond, at, out);
            for nested in &action.then_actions {
                check_action(nested, out);
            }
            for branch in &action.elseif_branches {
                check_condition("<elseif cond>", &branch.cond, at, out);
                for nested in &branch.actions {
                    check_action(nested, out);
                }
            }
            for nested in &action.else_actions {
                check_action(nested, out);
            }
        }
        // A `<cpp>` / `<kt>` child is emitted as source in that language
        // and never reaches the ECMAScript frontend, so the guard sits in
        // the arm pattern: an inline `<script>` is the only shape whose
        // body this frontend is asked to read.
        "script" if !action.is_cpp_function && !action.is_kt_function => {
            check(ExpressionRole::Script, "<script>", &action.content, at, out);
        }
        _ => {}
    }
}

/// A `cond`, skipping the spellings no backend lowers through the
/// frontend.
///
/// [`crate::parser::check_expression_needs`] is the single classifier for
/// that boundary — it answers `false` for a `cpp:` / `kt:` prefixed
/// condition and for a pure `In()` predicate, both of which are emitted
/// as native code. Asking it here rather than re-deciding keeps this
/// walker and the templates on one definition of "native".
fn check_condition(
    site: &str,
    cond: &str,
    location: Option<&SourceLocation>,
    out: &mut Vec<RefusedExpression>,
) {
    let (needs_engine, _has_in) = crate::parser::check_expression_needs(cond);
    if !needs_engine {
        return;
    }
    check(ExpressionRole::Condition, site, cond, location, out);
}

fn check(
    role: ExpressionRole,
    site: &str,
    source: &str,
    location: Option<&SourceLocation>,
    out: &mut Vec<RefusedExpression>,
) {
    // An absent attribute is not an expression. The filters answer the
    // same way — an empty `expr` lowers to nothing, an empty `cond` to
    // `true` — so neither can be refused.
    if source.is_empty() {
        return;
    }
    if let Err(error) = role.lower(source) {
        out.push(RefusedExpression {
            role,
            site: site.to_string(),
            source: source.to_string(),
            error,
            location: location.cloned(),
        });
    }
}
