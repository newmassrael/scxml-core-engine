// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! Exhaustive determination of [`SCXMLModel::needs_script_engine`].
//!
//! Before this module existed the flag was set from 22 scattered write
//! sites across [`crate::parser`] and [`crate::analyzer`]. Adding a new
//! action variant that needed a script engine meant grepping for the
//! flag and copy-pasting another `= true;` line at the new parse point —
//! the knowledge of *why* a document requires a script engine was
//! dispersed.
//!
//! This module is the single source of truth. It walks a fully-parsed
//! [`SCXMLModel`] and enumerates every distinct [`NeedsScriptEngineCause`]
//! that triggers the flag. Callers invoke [`requires_script_engine`] for
//! a boolean or [`analyze`] to inspect the cause list.
//!
//! The analyzer runs inside [`crate::parser::SCXMLParser::parse_impl`] after
//! `parse_states` finishes and before the `needs_nonstatic_method`
//! derivation — so every caller (top-level compile, child-invoke metadata
//! extraction, tests) sees a correctly-set flag without any parse-time
//! side effects.

use crate::forge::error::SourceLocation;
use crate::forge::model::EventSchemaModel;
use crate::model::{
    Action, DoneData, DoneDataContent, Invoke, InvokeSessionCommon, MeshRpcTarget, SCXMLModel,
    State, Variable,
};
use std::collections::BTreeMap;

/// One distinct reason a document needs a runtime script engine.
///
/// Every variant corresponds to a previously-scattered write site that
/// flipped [`SCXMLModel::needs_script_engine`] from `true`; collecting
/// them here makes the set reviewable and lets tests pin exactly which
/// clause fires for a given fixture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptEngineCauseKind {
    /// §scxml-5.3 — `<data>` variable with a non-empty `expr`/`src`/`content`
    /// init requires evaluating the initializer at runtime.
    DatamodelVariableInit { var_id: String },
    /// §scxml-5.8 — top-level `<script>` element, either inline text or
    /// loaded from a `src=` file. The script body is executed at document load.
    GlobalScript,
    /// `<script src="...">` appeared in the source but the parser had no
    /// filesystem access (WASM `parse_string`) so the body could not be
    /// loaded. The document still declares executable script; the flag
    /// stays honest even though `global_scripts` ends up empty.
    UnresolvedExternalScript,
    /// §scxml-3.13 — `<transition cond="...">` evaluates a non-native
    /// ECMAScript guard. Native `cpp:` / `kt:` conditions are emitted
    /// inline and do **not** trigger this cause.
    TransitionGuard { source_state: String },
    /// §scxml-6.2 — `<send namelist="...">` references datamodel
    /// identifiers that must be resolved at runtime.
    SendNamelist { state_id: String },
    /// §scxml-6.2.4 — `<send><param expr="..."/>` whose expression is
    /// not a static string literal. Static literals are folded at build time.
    SendParamExpr {
        state_id: String,
        param_name: String,
    },
    /// §scxml-6.2 — `<send>` with `eventexpr` / `targetexpr` /
    /// `delayexpr` / `typeexpr` / `contentexpr` / `idlocation` attributes:
    /// any of which forces runtime expression evaluation. `contentexpr`
    /// (W3C 5.10) and `idlocation` (W3C 6.2.4) entail datamodel reads/writes
    /// the script engine owns; without it the value has no carrier.
    SendDynamicAttr { state_id: String },
    /// §scxml-4.2 — `<if cond="...">` evaluates a non-native guard expression.
    IfCondition { state_id: String },
    /// §scxml-4.2 — `<elseif cond="...">` evaluates a non-native guard expression.
    ElseIfCondition { state_id: String },
    /// §scxml-5.4 — `<assign>` modifies the datamodel at runtime; the
    /// assignment itself requires the engine regardless of `expr` complexity.
    AssignAction { state_id: String },
    /// §scxml-4.2 — `<log expr="...">` with a non-empty expression needs
    /// the engine to evaluate the expression before logging.
    LogExpr { state_id: String },
    /// §scxml-5.8 — inline `<script>` action (body text, no native
    /// `<cpp>`/`<kt>` child). Native script blocks are emitted as code.
    InlineScriptAction { state_id: String },
    /// §scxml-6.2 — `<cancel sendidexpr="...">` needs the engine to
    /// evaluate the send id expression at runtime.
    CancelExpr { state_id: String },
    /// §scxml-4.6 — `<foreach>` iterates over a runtime-evaluated array.
    ForeachAction { state_id: String },
    /// §scxml-6.4 — hybrid `<invoke>` with `srcexpr` or `contentexpr`;
    /// the target is resolved at `<invoke>` entry by the script engine.
    HybridInvoke { invoke_id: String },
    /// §scxml-6.4.1 — static `<invoke namelist="...">` reads datamodel
    /// variables to form the child's initial state at entry.
    StaticInvokeNamelist { invoke_id: String },
    /// SCE Mesh §9.5 — `<invoke type="sce:mesh-rpc">` with `srcexpr`
    /// target; the generated entry block calls `evaluateExpression`.
    MeshRpcSrcExpr { invoke_id: String },
    /// §scxml-5.7 — `<donedata>` carries at least one `<param>` whose
    /// value must be evaluated when the final state is entered.
    DonedataParam { state_id: String },
    /// §scxml-5.7 — `<donedata>` has `<content expr="...">` whose
    /// expression must be evaluated before the `done.state` event is raised.
    /// `<content>literal</content>` does **not** trigger this cause: per
    /// §scxml-5.5 the children are used as the value directly (see
    /// [`DoneDataContent::Literal`]).
    DonedataContent { state_id: String },
    /// §scxml-6.4 — a static `<invoke>` targets a child SCXML whose
    /// own analyzer output declared `needs_script_engine = true`; the
    /// parent must carry an engine so the child can run.
    ChildInvokeNeedsScriptEngine { invoke_id: String },
}

/// One cause, anchored on the source it came from.
///
/// The kind says *what* forced the script engine in; the location says
/// *where*. Both are needed: a build that must stay pure-static fails on
/// the flag, and the author then has to find the construct that cost it.
///
/// `location` is the owning element's own [`SourceLocation`] — the same
/// anchor a diagnostic on that element would carry — so the degradation
/// report and the rejection report point at source the same way. It is
/// `None` only for a cause with no single element to blame (a document
/// whose `<script src=…>` the parser could not read).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NeedsScriptEngineCause {
    pub kind: ScriptEngineCauseKind,
    pub location: Option<SourceLocation>,
}

impl NeedsScriptEngineCause {
    fn new(kind: ScriptEngineCauseKind, location: Option<&SourceLocation>) -> Self {
        Self {
            kind,
            location: location.cloned(),
        }
    }
}

/// Wire projection of one [`NeedsScriptEngineCause`] — the shape the
/// `sce-codegen generate` stdout manifest carries in
/// `script_engine_causes` (SCE_ERROR_CONTRACT.md §10.1).
///
/// `needs_script_engine` on its own tells a consumer that a machine lost
/// its pure-static lowering but not *which* construct cost it. A build
/// that gates on the flag then fails with nothing to act on. These
/// records name the construct, so the gate can point at a line of SCXML.
///
/// The projection is deliberate rather than a `derive(Serialize)` on the
/// enum: `kind` is a stable kebab-case wire token that does not move when
/// the Rust variant is renamed, and the anchors are flattened to one
/// optional field per identifier kind so a consumer reads
/// `cause.state` / `cause.invoke` without matching on a nested union.
/// [`ScriptEngineCauseKind::to_wire`] is an exhaustive match, so a new
/// variant cannot be added without choosing its wire shape.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ScriptEngineCauseRecord {
    /// Stable kebab-case discriminator. Consumers dispatch on this and
    /// must tolerate unknown values (new constructs may be added).
    pub kind: &'static str,
    /// Owning state, for causes anchored to a state or a transition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    /// Datamodel variable id, for the `<data>` initializer cause.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub var: Option<String>,
    /// `<param name="…">`, for the send-param cause.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub param: Option<String>,
    /// `<invoke id="…">`, for the invoke-anchored causes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoke: Option<String>,
    /// Where in the source. Same `{file, line, col}` shape a diagnostic
    /// carries, so tooling anchors a degradation exactly as it anchors a
    /// rejection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<SourceLocation>,
}

impl ScriptEngineCauseRecord {
    fn new(kind: &'static str) -> Self {
        Self {
            kind,
            state: None,
            var: None,
            param: None,
            invoke: None,
            location: None,
        }
    }

    fn at_state(kind: &'static str, state_id: &str) -> Self {
        Self {
            state: Some(state_id.to_string()),
            ..Self::new(kind)
        }
    }

    fn at_invoke(kind: &'static str, invoke_id: &str) -> Self {
        Self {
            invoke: Some(invoke_id.to_string()),
            ..Self::new(kind)
        }
    }
}

impl NeedsScriptEngineCause {
    /// Project onto the manifest wire shape. Exhaustive by construction —
    /// adding a variant without a wire `kind` does not compile.
    pub fn to_wire(&self) -> ScriptEngineCauseRecord {
        ScriptEngineCauseRecord {
            location: self.location.clone(),
            ..self.kind.to_wire()
        }
    }
}

impl ScriptEngineCauseKind {
    fn to_wire(&self) -> ScriptEngineCauseRecord {
        use ScriptEngineCauseKind as C;
        match self {
            C::DatamodelVariableInit { var_id } => ScriptEngineCauseRecord {
                var: Some(var_id.clone()),
                ..ScriptEngineCauseRecord::new("datamodel-variable-init")
            },
            C::GlobalScript => ScriptEngineCauseRecord::new("global-script"),
            C::UnresolvedExternalScript => {
                ScriptEngineCauseRecord::new("unresolved-external-script")
            }
            // The cause a typed-guard author must be able to see: an
            // EventSchema guard that did not lower natively lands here,
            // indistinguishable in the flag alone from any other guard.
            C::TransitionGuard { source_state } => {
                ScriptEngineCauseRecord::at_state("transition-guard", source_state)
            }
            C::SendNamelist { state_id } => {
                ScriptEngineCauseRecord::at_state("send-namelist", state_id)
            }
            C::SendParamExpr {
                state_id,
                param_name,
            } => ScriptEngineCauseRecord {
                param: Some(param_name.clone()),
                ..ScriptEngineCauseRecord::at_state("send-param-expr", state_id)
            },
            C::SendDynamicAttr { state_id } => {
                ScriptEngineCauseRecord::at_state("send-dynamic-attr", state_id)
            }
            C::IfCondition { state_id } => {
                ScriptEngineCauseRecord::at_state("if-condition", state_id)
            }
            C::ElseIfCondition { state_id } => {
                ScriptEngineCauseRecord::at_state("elseif-condition", state_id)
            }
            C::AssignAction { state_id } => {
                ScriptEngineCauseRecord::at_state("assign-action", state_id)
            }
            C::LogExpr { state_id } => ScriptEngineCauseRecord::at_state("log-expr", state_id),
            C::InlineScriptAction { state_id } => {
                ScriptEngineCauseRecord::at_state("inline-script-action", state_id)
            }
            C::CancelExpr { state_id } => {
                ScriptEngineCauseRecord::at_state("cancel-expr", state_id)
            }
            C::ForeachAction { state_id } => {
                ScriptEngineCauseRecord::at_state("foreach-action", state_id)
            }
            C::HybridInvoke { invoke_id } => {
                ScriptEngineCauseRecord::at_invoke("hybrid-invoke", invoke_id)
            }
            C::StaticInvokeNamelist { invoke_id } => {
                ScriptEngineCauseRecord::at_invoke("static-invoke-namelist", invoke_id)
            }
            C::MeshRpcSrcExpr { invoke_id } => {
                ScriptEngineCauseRecord::at_invoke("mesh-rpc-srcexpr", invoke_id)
            }
            C::DonedataParam { state_id } => {
                ScriptEngineCauseRecord::at_state("donedata-param", state_id)
            }
            C::DonedataContent { state_id } => {
                ScriptEngineCauseRecord::at_state("donedata-content", state_id)
            }
            C::ChildInvokeNeedsScriptEngine { invoke_id } => {
                ScriptEngineCauseRecord::at_invoke("child-invoke-needs-script-engine", invoke_id)
            }
        }
    }
}

/// Walk `model` and return every distinct cause that would make the
/// document require a runtime script engine. The returned vector is
/// empty iff no such cause exists, i.e. [`requires_script_engine`]
/// would return `false`.
pub fn analyze(model: &SCXMLModel) -> Vec<NeedsScriptEngineCause> {
    let mut causes = Vec::new();
    collect_datamodel_causes(&model.variables, &mut causes);
    collect_global_script_causes(model, &mut causes);
    for (state_id, state) in &model.states {
        collect_state_causes(state_id, state, &model.imported_event_schemas, &mut causes);
    }
    causes
}

/// `true` iff `analyze(model)` would return any cause. Thin wrapper —
/// `analyze` is the single traversal, this spelling exists so callers
/// that only need the bool can read at the intent level without
/// spelling the `.is_empty()` check themselves.
pub fn requires_script_engine(model: &SCXMLModel) -> bool {
    !analyze(model).is_empty()
}

fn collect_datamodel_causes(variables: &[Variable], out: &mut Vec<NeedsScriptEngineCause>) {
    for var in variables {
        // §scxml-5.3: any initializer (expr/src/content) needs the
        // engine to evaluate at runtime. Tighter classification (int /
        // string / bool literal → static init) is orthogonal — it
        // happens later in [`crate::analyzer::classify_variables`] and
        // doesn't affect this flag.
        if !var.expr.is_empty() || !var.src.is_empty() || !var.content.is_empty() {
            out.push(NeedsScriptEngineCause::new(
                ScriptEngineCauseKind::DatamodelVariableInit {
                    var_id: var.id.clone(),
                },
                var.source_location.as_ref(),
            ));
        }
    }
}

fn collect_global_script_causes(model: &SCXMLModel, out: &mut Vec<NeedsScriptEngineCause>) {
    // Anchored on the document root: a top-level `<script>` is not a model
    // element of its own, so the root is the finest anchor that exists.
    if !model.global_scripts.is_empty() {
        out.push(NeedsScriptEngineCause::new(
            ScriptEngineCauseKind::GlobalScript,
            model.source_location.as_ref(),
        ));
    }
    if model.has_unresolved_external_script {
        out.push(NeedsScriptEngineCause::new(
            ScriptEngineCauseKind::UnresolvedExternalScript,
            model.source_location.as_ref(),
        ));
    }
}

fn collect_state_causes(
    state_id: &str,
    state: &State,
    schemas: &BTreeMap<String, EventSchemaModel>,
    out: &mut Vec<NeedsScriptEngineCause>,
) {
    for trans in &state.transitions {
        if transition_guard_needs_engine(trans, schemas) {
            out.push(NeedsScriptEngineCause::new(
                ScriptEngineCauseKind::TransitionGuard {
                    source_state: state_id.to_string(),
                },
                trans.source_location.as_ref(),
            ));
        }
        for action in &trans.actions {
            collect_action_causes(state_id, action, out);
        }
    }
    for block in state
        .on_entry_blocks
        .iter()
        .chain(state.on_exit_blocks.iter())
    {
        for action in block {
            collect_action_causes(state_id, action, out);
        }
    }
    for action in &state.initial_transition_actions {
        collect_action_causes(state_id, action, out);
    }
    for action in &state.initial_history_default_actions {
        collect_action_causes(state_id, action, out);
    }
    for invoke in &state.invokes {
        collect_invoke_causes(invoke, out);
    }
    if let Some(dd) = &state.donedata {
        collect_donedata_causes(state_id, state, dd, out);
    }
}

fn transition_guard_needs_engine(
    trans: &crate::model::Transition,
    schemas: &BTreeMap<String, EventSchemaModel>,
) -> bool {
    // `check_expression_needs` is the single classifier — it returns
    // `(false, _)` for `cpp:` / `kt:` prefixes and pure-In() predicates,
    // so no additional native-case guard is needed here. `is_cpp_condition`
    // / `is_kt_condition` remain on [`crate::model::Transition`] for
    // codegen template branching, not for flag decisions.
    let (needs_se, _has_in) = crate::parser::check_expression_needs(&trans.cond);
    if !needs_se {
        return false;
    }
    // NL→IR Item C1 Path A (EventSchema MCU native lowering, step 2) — a
    // guard whose triggering event carries an imported EventSchema and
    // whose whole expression lowers through the typed-expression pipeline
    // needs no script engine: codegen emits it as a native typed-payload
    // comparison (`_event.data.<field>` → the bound payload variant
    // field) instead of routing the ECMAScript string through the runtime
    // engine that no_std MCU targets lack. The lowerability verdict is
    // the single source of truth shared with the codegen path — see
    // [`crate::forge::event_schema_check::guard_is_native_lowerable`].
    // Events without an imported schema keep the dynamic `_event.data`
    // baseline (schemaless fallback DL-9').
    if let Some(schema) = schemas.get(&trans.event) {
        if crate::forge::event_schema_check::guard_is_native_lowerable(&trans.cond, schema) {
            return false;
        }
    }
    true
}

fn collect_action_causes(state_id: &str, action: &Action, out: &mut Vec<NeedsScriptEngineCause>) {
    match action.action_type.as_str() {
        "send" => {
            if !action.namelist.is_empty() {
                out.push(NeedsScriptEngineCause::new(
                    ScriptEngineCauseKind::SendNamelist {
                        state_id: state_id.to_string(),
                    },
                    action.source_location.as_ref(),
                ));
            }
            for param in &action.params {
                if !param.expr.is_empty() && !param.is_static_literal {
                    out.push(NeedsScriptEngineCause::new(
                        ScriptEngineCauseKind::SendParamExpr {
                            state_id: state_id.to_string(),
                            param_name: param.name.clone(),
                        },
                        action.source_location.as_ref(),
                    ));
                }
            }
            if send_has_dynamic_attr(action) {
                out.push(NeedsScriptEngineCause::new(
                    ScriptEngineCauseKind::SendDynamicAttr {
                        state_id: state_id.to_string(),
                    },
                    action.source_location.as_ref(),
                ));
            }
        }
        "if" => {
            if cond_needs_engine(&action.cond) {
                out.push(NeedsScriptEngineCause::new(
                    ScriptEngineCauseKind::IfCondition {
                        state_id: state_id.to_string(),
                    },
                    action.source_location.as_ref(),
                ));
            }
            for branch in &action.elseif_branches {
                if cond_needs_engine(&branch.cond) {
                    out.push(NeedsScriptEngineCause::new(
                        ScriptEngineCauseKind::ElseIfCondition {
                            state_id: state_id.to_string(),
                        },
                        action.source_location.as_ref(),
                    ));
                }
                for nested in &branch.actions {
                    collect_action_causes(state_id, nested, out);
                }
            }
            for nested in &action.then_actions {
                collect_action_causes(state_id, nested, out);
            }
            for nested in &action.else_actions {
                collect_action_causes(state_id, nested, out);
            }
        }
        "assign" => {
            // §scxml-5.4: `<assign>` is always engine-bound; even pure
            // location='var' expr='literal' routes through the assignment
            // helper which talks to the datamodel store.
            out.push(NeedsScriptEngineCause::new(
                ScriptEngineCauseKind::AssignAction {
                    state_id: state_id.to_string(),
                },
                action.source_location.as_ref(),
            ));
        }
        "log" => {
            if !action.expr.is_empty() {
                out.push(NeedsScriptEngineCause::new(
                    ScriptEngineCauseKind::LogExpr {
                        state_id: state_id.to_string(),
                    },
                    action.source_location.as_ref(),
                ));
            }
        }
        "script" => {
            // Inline `<script>` action. Native `<cpp>` / `<kt>` child
            // blocks are emitted as source code and set the respective
            // `is_*_function` flag — they do not require the engine.
            if !action.is_cpp_function && !action.is_kt_function {
                out.push(NeedsScriptEngineCause::new(
                    ScriptEngineCauseKind::InlineScriptAction {
                        state_id: state_id.to_string(),
                    },
                    action.source_location.as_ref(),
                ));
            }
        }
        "cancel" => {
            if !action.sendidexpr.is_empty() {
                out.push(NeedsScriptEngineCause::new(
                    ScriptEngineCauseKind::CancelExpr {
                        state_id: state_id.to_string(),
                    },
                    action.source_location.as_ref(),
                ));
            }
        }
        "foreach" => {
            out.push(NeedsScriptEngineCause::new(
                ScriptEngineCauseKind::ForeachAction {
                    state_id: state_id.to_string(),
                },
                action.source_location.as_ref(),
            ));
            for nested in &action.actions {
                collect_action_causes(state_id, nested, out);
            }
        }
        "native_action" => {
            // §scxml-G-7 — `<sce:action>` Custom Action Element. A
            // native host-trait dispatch never needs a runtime engine:
            // codegen lowers each `<sce:arg>` through the typed-expression
            // pipeline and emits a direct call into the generated `Actions`
            // trait. An argument that cannot be statically lowered is
            // rejected at codegen time by the `expression/*` machinery — it
            // is never an engine cause. Listed explicitly (rather than left
            // to the `_` arm) so this guarantee is pinned by
            // `native_action_is_not_a_script_engine_cause`.
        }
        _ => {}
    }
}

fn send_has_dynamic_attr(action: &Action) -> bool {
    !action.eventexpr.is_empty()
        || !action.targetexpr.is_empty()
        || !action.delayexpr.is_empty()
        || !action.typeexpr.is_empty()
        || !action.contentexpr.is_empty()
        || !action.idlocation.is_empty()
}

fn cond_needs_engine(cond: &str) -> bool {
    // `check_expression_needs` is the single classifier. It returns
    // `(false, _)` for `cpp:` / `kt:` prefixes and pure-In() predicates
    // alike; no separate `cond_cpp` / `cond_kt` inspection is needed.
    let (needs_se, _has_in) = crate::parser::check_expression_needs(cond);
    needs_se
}

fn collect_invoke_causes(invoke: &Invoke, out: &mut Vec<NeedsScriptEngineCause>) {
    match invoke {
        Invoke::Hybrid(info) => {
            out.push(NeedsScriptEngineCause::new(
                ScriptEngineCauseKind::HybridInvoke {
                    invoke_id: info.common.base.invoke_id.clone(),
                },
                info.common.base.source_location.as_ref(),
            ));
            push_child_invoke_cause(&info.common, out);
        }
        Invoke::Scxml(info) => {
            if !info.namelist.is_empty() {
                out.push(NeedsScriptEngineCause::new(
                    ScriptEngineCauseKind::StaticInvokeNamelist {
                        invoke_id: info.common.base.invoke_id.clone(),
                    },
                    info.common.base.source_location.as_ref(),
                ));
            }
            push_child_invoke_cause(&info.common, out);
        }
        Invoke::MeshRpc(info) => {
            if matches!(&info.target, MeshRpcTarget::SrcExpr { .. }) {
                out.push(NeedsScriptEngineCause::new(
                    ScriptEngineCauseKind::MeshRpcSrcExpr {
                        invoke_id: info.base.invoke_id.clone(),
                    },
                    info.base.source_location.as_ref(),
                ));
            }
        }
        // §scxml-6.4.1: the whole lowering is one `error.execution` raise
        // with a compile-time-constant message. Nothing is evaluated, so
        // an unsupported invoke never pulls in a script engine.
        Invoke::Unsupported(_) => {}
    }
}

fn push_child_invoke_cause(common: &InvokeSessionCommon, out: &mut Vec<NeedsScriptEngineCause>) {
    if common.child_needs_script_engine {
        out.push(NeedsScriptEngineCause::new(
            ScriptEngineCauseKind::ChildInvokeNeedsScriptEngine {
                invoke_id: common.base.invoke_id.clone(),
            },
            common.base.source_location.as_ref(),
        ));
    }
}

// `<donedata>` is not a located model element of its own, so its causes
// anchor on the `<final>` state that owns it — the finest anchor available.
fn collect_donedata_causes(
    state_id: &str,
    state: &State,
    dd: &DoneData,
    out: &mut Vec<NeedsScriptEngineCause>,
) {
    if !dd.params.is_empty() {
        out.push(NeedsScriptEngineCause::new(
            ScriptEngineCauseKind::DonedataParam {
                state_id: state_id.to_string(),
            },
            state.source_location.as_ref(),
        ));
    }
    // Only `<content expr="...">` forces a script engine. Literal bodies
    // are emitted as string constants by the codegen literal path and by
    // the interpreter's `DoneDataHelper::emitContentLiteral`, so no
    // evaluation is required.
    if matches!(dd.content, DoneDataContent::Expression(_)) {
        out.push(NeedsScriptEngineCause::new(
            ScriptEngineCauseKind::DonedataContent {
                state_id: state_id.to_string(),
            },
            state.source_location.as_ref(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SCXMLParser;

    fn parse(scxml: &str) -> SCXMLModel {
        SCXMLParser::new().parse_string(scxml, "test").unwrap()
    }

    /// The kind of the sole cause. Tests that assert *which* construct
    /// fired read the kind; the location is asserted separately by
    /// `causes_are_anchored_on_their_source`.
    fn single_cause(scxml: &str) -> ScriptEngineCauseKind {
        let model = parse(scxml);
        let causes = analyze(&model);
        assert_eq!(
            causes.len(),
            1,
            "expected exactly one cause, got {:?}",
            causes,
        );
        causes.into_iter().next().unwrap().kind
    }

    fn contains_cause<F>(scxml: &str, predicate: F)
    where
        F: Fn(&ScriptEngineCauseKind) -> bool,
    {
        let model = parse(scxml);
        let causes = analyze(&model);
        assert!(
            causes.iter().any(|c| predicate(&c.kind)),
            "expected a matching cause, got {:?}",
            causes,
        );
    }

    #[test]
    fn empty_document_has_no_causes() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s"/>
        </scxml>"#;
        let model = parse(scxml);
        assert!(analyze(&model).is_empty());
        assert!(!requires_script_engine(&model));
    }

    #[test]
    fn datamodel_variable_init_triggers() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s" datamodel="ecmascript">
            <datamodel><data id="x" expr="1"/></datamodel>
            <state id="s"/>
        </scxml>"#;
        assert!(matches!(
            single_cause(scxml),
            ScriptEngineCauseKind::DatamodelVariableInit { var_id } if var_id == "x"
        ));
    }

    #[test]
    fn global_script_triggers() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <script>var x = 1;</script>
            <state id="s"/>
        </scxml>"#;
        assert!(matches!(
            single_cause(scxml),
            ScriptEngineCauseKind::GlobalScript
        ));
    }

    #[test]
    fn unresolved_external_script_triggers() {
        // parse_string has no base_dir, so the external src cannot be
        // loaded; the analyzer surfaces the document fact via the
        // `has_unresolved_external_script` model field.
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <script src="unresolvable.js"/>
            <state id="s"/>
        </scxml>"#;
        let model = parse(scxml);
        assert!(model.has_unresolved_external_script);
        let causes = analyze(&model);
        assert!(causes
            .iter()
            .any(|c| matches!(c.kind, ScriptEngineCauseKind::UnresolvedExternalScript)));
    }

    #[test]
    fn transition_guard_triggers() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s"><transition event="e" cond="1 == 1" target="s"/></state>
        </scxml>"#;
        assert!(matches!(
            single_cause(scxml),
            ScriptEngineCauseKind::TransitionGuard { source_state } if source_state == "s"
        ));
    }

    #[test]
    fn send_namelist_triggers() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s"><onentry><send event="e" namelist="a b"/></onentry></state>
        </scxml>"#;
        contains_cause(
            scxml,
            |c| matches!(c, ScriptEngineCauseKind::SendNamelist { state_id } if state_id == "s"),
        );
    }

    #[test]
    fn send_param_expr_triggers() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s"><onentry>
                <send event="e"><param name="p" expr="1 + 2"/></send>
            </onentry></state>
        </scxml>"#;
        contains_cause(scxml, |c| {
            matches!(
                c,
                ScriptEngineCauseKind::SendParamExpr { param_name, .. } if param_name == "p"
            )
        });
    }

    #[test]
    fn send_dynamic_attr_triggers() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s"><onentry><send eventexpr="'e'"/></onentry></state>
        </scxml>"#;
        contains_cause(
            scxml,
            |c| matches!(c, ScriptEngineCauseKind::SendDynamicAttr { state_id } if state_id == "s"),
        );
    }

    #[test]
    fn if_condition_triggers() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s"><onentry><if cond="1 == 1"/></onentry></state>
        </scxml>"#;
        contains_cause(
            scxml,
            |c| matches!(c, ScriptEngineCauseKind::IfCondition { state_id } if state_id == "s"),
        );
    }

    #[test]
    fn elseif_condition_triggers() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s"><onentry>
                <if cond="In('s')"><elseif cond="1 == 1"/></if>
            </onentry></state>
        </scxml>"#;
        contains_cause(
            scxml,
            |c| matches!(c, ScriptEngineCauseKind::ElseIfCondition { state_id } if state_id == "s"),
        );
    }

    #[test]
    fn assign_action_triggers() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s"><onentry><assign location="x" expr="1"/></onentry></state>
        </scxml>"#;
        contains_cause(
            scxml,
            |c| matches!(c, ScriptEngineCauseKind::AssignAction { state_id } if state_id == "s"),
        );
    }

    #[test]
    fn log_expr_triggers() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s"><onentry><log expr="1"/></onentry></state>
        </scxml>"#;
        contains_cause(
            scxml,
            |c| matches!(c, ScriptEngineCauseKind::LogExpr { state_id } if state_id == "s"),
        );
    }

    #[test]
    fn inline_script_action_triggers() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s"><onentry><script>var y = 2;</script></onentry></state>
        </scxml>"#;
        contains_cause(
            scxml,
            |c| matches!(c, ScriptEngineCauseKind::InlineScriptAction { state_id } if state_id == "s"),
        );
    }

    #[test]
    fn native_action_is_not_a_script_engine_cause() {
        // §scxml-G-7: a `<sce:action>` Custom Action Element dispatches
        // to a host trait method; it never needs a runtime engine even with
        // typed-payload arguments. The whole document must analyze clean.
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" initial="s">
            <state id="s">
                <transition event="e" target="s">
                    <sce:action name="do_effect">
                        <sce:arg expr="_event.data.payload"/>
                    </sce:action>
                </transition>
            </state>
        </scxml>"#;
        let model = parse(scxml);
        // Guard against a false positive: a *dropped* action also yields no
        // causes. Assert the action actually parsed onto the transition.
        let trans = &model.states.get("s").unwrap().transitions[0];
        assert_eq!(trans.actions.len(), 1, "the <sce:action> must parse");
        assert_eq!(trans.actions[0].action_type, "native_action");
        let causes = analyze(&model);
        assert!(
            causes.is_empty(),
            "<sce:action> must not require a script engine, got {:?}",
            causes,
        );
        assert!(!requires_script_engine(&model));
    }

    #[test]
    fn cancel_expr_triggers() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s"><onentry><cancel sendidexpr="'id'"/></onentry></state>
        </scxml>"#;
        contains_cause(
            scxml,
            |c| matches!(c, ScriptEngineCauseKind::CancelExpr { state_id } if state_id == "s"),
        );
    }

    #[test]
    fn foreach_action_triggers() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s"><onentry><foreach array="xs" item="x"/></onentry></state>
        </scxml>"#;
        contains_cause(
            scxml,
            |c| matches!(c, ScriptEngineCauseKind::ForeachAction { state_id } if state_id == "s"),
        );
    }

    #[test]
    fn hybrid_invoke_triggers() {
        let scxml = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s">
                <invoke id="h1" srcexpr="'child.scxml'"/>
            </state>
        </scxml>"##;
        contains_cause(
            scxml,
            |c| matches!(c, ScriptEngineCauseKind::HybridInvoke { invoke_id } if invoke_id == "h1"),
        );
    }

    #[test]
    fn static_invoke_namelist_triggers() {
        // Uses `content` inline SCXML to avoid filesystem resolution in
        // parse_string. Namelist is the cause under test.
        let scxml = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <datamodel><data id="a"/></datamodel>
            <state id="s">
                <invoke id="i1" namelist="a">
                    <content><scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="c"><state id="c"/></scxml></content>
                </invoke>
            </state>
        </scxml>"##;
        contains_cause(scxml, |c| {
            matches!(
                c,
                ScriptEngineCauseKind::StaticInvokeNamelist { invoke_id } if invoke_id == "i1"
            )
        });
    }

    #[test]
    fn mesh_rpc_srcexpr_triggers() {
        let scxml = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s">
                <invoke id="m1" type="sce:mesh-rpc" srcexpr="'#motor'">
                    <param name="_mesh_event" expr="'service.request.ping'"/>
                </invoke>
            </state>
        </scxml>"##;
        contains_cause(scxml, |c| {
            matches!(
                c,
                ScriptEngineCauseKind::MeshRpcSrcExpr { invoke_id } if invoke_id == "m1"
            )
        });
    }

    #[test]
    fn donedata_param_triggers() {
        let scxml = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s" initial="f"><final id="f"><donedata><param name="x" expr="1"/></donedata></final></state>
        </scxml>"##;
        contains_cause(
            scxml,
            |c| matches!(c, ScriptEngineCauseKind::DonedataParam { state_id } if state_id == "f"),
        );
    }

    #[test]
    fn donedata_content_expression_triggers() {
        // §scxml-5.5: `<content expr="...">` MUST be evaluated against the
        // datamodel — so the script engine is required.
        let scxml = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s" initial="f"><final id="f"><donedata><content expr="1"/></donedata></final></state>
        </scxml>"##;
        contains_cause(
            scxml,
            |c| matches!(c, ScriptEngineCauseKind::DonedataContent { state_id } if state_id == "f"),
        );
    }

    #[test]
    fn donedata_content_literal_null_datamodel_does_not_trigger() {
        // §scxml-5.5 + `datamodel="null"`: inline text is the content value
        // verbatim — no evaluation, no script engine. This is the
        // native-only path (`cpp:` / `kt:` documents, tc8-harness verdict
        // payloads). The same XML under the ECMAScript datamodel would
        // route through the Expression variant (Appendix B.2.2 JSON parse),
        // exercised in `donedata_content_literal_ecmascript_triggers`.
        let scxml = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="null" initial="s">
            <state id="s" initial="f">
                <final id="f"><donedata><content>{"verdict":"pass"}</content></donedata></final>
            </state>
        </scxml>"##;
        let model = parse(scxml);
        let causes = analyze(&model);
        assert!(
            causes.is_empty(),
            "literal <content> under null datamodel must not force a script engine, got {:?}",
            causes,
        );
        assert!(!requires_script_engine(&model));
    }

    #[test]
    fn donedata_content_literal_ecmascript_triggers() {
        // W3C ECMAScript Appendix B.2.2: inline text is parsed as JSON by
        // the datamodel — SCE routes that through the script engine by
        // tagging the content as Expression. This pins the contract that
        // ECMAScript-datamodel documents keep the evaluated path.
        let scxml = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="s">
            <state id="s" initial="f">
                <final id="f"><donedata><content>21</content></donedata></final>
            </state>
        </scxml>"##;
        contains_cause(
            scxml,
            |c| matches!(c, ScriptEngineCauseKind::DonedataContent { state_id } if state_id == "f"),
        );
    }

    #[test]
    fn requires_script_engine_is_analyze_is_not_empty() {
        // `requires_script_engine` is a one-line wrapper over
        // `!analyze(model).is_empty()`. Pin the wrapper contract so that
        // a future optimisation that reintroduces a divergent fast path
        // fails here.
        let docs: &[(&str, bool)] = &[
            (
                r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s"><state id="s"/></scxml>"#,
                false,
            ),
            (
                r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s"><state id="s"><onentry><log expr="1"/></onentry></state></scxml>"#,
                true,
            ),
        ];
        for (scxml, expected) in docs {
            let model = parse(scxml);
            assert_eq!(
                requires_script_engine(&model),
                !analyze(&model).is_empty(),
                "requires_script_engine diverged from analyze for {scxml}",
            );
            assert_eq!(
                requires_script_engine(&model),
                *expected,
                "unexpected flag for {scxml}"
            );
        }
    }

    /// The parser stores the cause list on the model in the same statement
    /// that sets the flag, so a machine that needs the engine always names
    /// at least one cause and a pure-static machine names none. A consumer
    /// gating on the flag can therefore always act on the list — and no
    /// later pass can leave the two disagreeing, because neither is
    /// recomputed.
    #[test]
    fn model_flag_agrees_with_stored_causes() {
        let docs: &[&str] = &[
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s"><state id="s"/></scxml>"#,
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s"><state id="s"><onentry><log expr="1"/></onentry></state></scxml>"#,
        ];
        for scxml in docs {
            let model = parse(scxml);
            assert_eq!(
                model.needs_script_engine,
                !model.script_engine_causes.is_empty(),
                "stored flag and stored causes disagree for {scxml}",
            );
            assert_eq!(
                model.script_engine_causes,
                analyze(&model),
                "stored causes diverged from the analyzer for {scxml}",
            );
        }
    }

    /// Every cause projects to a non-empty, stable kebab-case wire token.
    /// `to_wire` is an exhaustive match, so a new variant cannot reach the
    /// manifest without one being chosen — this pins the token *shape*,
    /// which is what a consumer dispatches on.
    #[test]
    fn wire_kinds_are_kebab_case_tokens() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s"
                              datamodel="ecmascript">
              <datamodel><data id="v" expr="0"/></datamodel>
              <state id="s">
                <onentry><log expr="1"/><assign location="v" expr="1"/></onentry>
                <transition event="e" cond="v === 1" target="t"/>
              </state>
              <state id="t"/>
            </scxml>"#;
        let model = parse(scxml);
        let causes: Vec<ScriptEngineCauseRecord> = model
            .script_engine_causes
            .iter()
            .map(|c| c.to_wire())
            .collect();
        assert!(!causes.is_empty(), "fixture must need the script engine");
        for c in &causes {
            assert!(
                !c.kind.is_empty() && c.kind.bytes().all(|b| b.is_ascii_lowercase() || b == b'-'),
                "wire kind must be a kebab-case token: {:?}",
                c.kind
            );
        }
        // The transition guard is anchored on its owning state, which is
        // what lets a build gate point at a line of SCXML.
        let guard = causes
            .iter()
            .find(|c| c.kind == "transition-guard")
            .expect("guard cause");
        assert_eq!(guard.state.as_deref(), Some("s"));
    }

    /// A cause must point at source. `needs_script_engine` fails a
    /// pure-static build; the record is what tells the author where to
    /// look, so an unanchored cause is only half a report. Every cause
    /// with an element to blame carries that element's own line — the same
    /// anchor a diagnostic on it would carry.
    #[test]
    fn causes_are_anchored_on_their_source() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s"
                              datamodel="ecmascript">
              <datamodel><data id="v" expr="0"/></datamodel>
              <state id="s">
                <transition event="e" cond="v === 1" target="t"/>
              </state>
              <state id="t"/>
            </scxml>"#;
        let model = parse(scxml);
        for cause in &model.script_engine_causes {
            let loc = cause
                .location
                .as_ref()
                .unwrap_or_else(|| panic!("cause has no source anchor: {:?}", cause.kind));
            assert!(
                loc.line.is_some(),
                "cause anchor carries no line: {:?}",
                cause
            );
        }
        // The `<data>` and the `<transition>` sit on different lines, so a
        // cause list that anchored everything on the document root would
        // collapse them — pinning distinctness keeps the anchors real.
        let lines: std::collections::BTreeSet<Option<u32>> = model
            .script_engine_causes
            .iter()
            .map(|c| c.location.as_ref().and_then(|l| l.line))
            .collect();
        assert_eq!(
            lines.len(),
            2,
            "each cause must carry its own element's line, got {:?}",
            model.script_engine_causes
        );
    }
}
