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

use crate::model::{
    Action, DoneData, ElseIfBranch, Invoke, InvokeSessionCommon, MeshRpcTarget, SCXMLModel, State,
    Variable,
};

/// One distinct reason a document needs a runtime script engine.
///
/// Every variant corresponds to a previously-scattered write site that
/// flipped [`SCXMLModel::needs_script_engine`] from `true`; collecting
/// them here makes the set reviewable and lets tests pin exactly which
/// clause fires for a given fixture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NeedsScriptEngineCause {
    /// W3C SCXML 5.3 — `<data>` variable with a non-empty `expr`/`src`/`content`
    /// init requires evaluating the initializer at runtime.
    DatamodelVariableInit { var_id: String },
    /// W3C SCXML 5.8 — top-level `<script>` element, either inline text or
    /// loaded from a `src=` file. The script body is executed at document load.
    GlobalScript,
    /// `<script src="...">` appeared in the source but the parser had no
    /// filesystem access (WASM `parse_string`) so the body could not be
    /// loaded. The document still declares executable script; the flag
    /// stays honest even though `global_scripts` ends up empty.
    UnresolvedExternalScript,
    /// W3C SCXML 3.13 — `<transition cond="...">` evaluates a non-native
    /// ECMAScript guard. Native `cpp:` / `kt:` conditions are emitted
    /// inline and do **not** trigger this cause.
    TransitionGuard { source_state: String },
    /// W3C SCXML 6.2 — `<send namelist="...">` references datamodel
    /// identifiers that must be resolved at runtime.
    SendNamelist { state_id: String },
    /// W3C SCXML 6.2.4 — `<send><param expr="..."/>` whose expression is
    /// not a static string literal. Static literals are folded at build time.
    SendParamExpr { state_id: String, param_name: String },
    /// W3C SCXML 6.2 — `<send>` with `eventexpr` / `targetexpr` /
    /// `delayexpr` / `typeexpr` attributes, any of which is a runtime expression.
    SendDynamicAttr { state_id: String },
    /// W3C SCXML 4.2 — `<if cond="...">` evaluates a non-native guard expression.
    IfCondition { state_id: String },
    /// W3C SCXML 4.2 — `<elseif cond="...">` evaluates a non-native guard expression.
    ElseIfCondition { state_id: String },
    /// W3C SCXML 5.4 — `<assign>` modifies the datamodel at runtime; the
    /// assignment itself requires the engine regardless of `expr` complexity.
    AssignAction { state_id: String },
    /// W3C SCXML 4.2 — `<log expr="...">` with a non-empty expression needs
    /// the engine to evaluate the expression before logging.
    LogExpr { state_id: String },
    /// W3C SCXML 4.2 — inline `<script>` action (body text, no native
    /// `<cpp>`/`<kt>` child). Native script blocks are emitted as code.
    InlineScriptAction { state_id: String },
    /// W3C SCXML 6.2 — `<cancel sendidexpr="...">` needs the engine to
    /// evaluate the send id expression at runtime.
    CancelExpr { state_id: String },
    /// W3C SCXML 4.2 — `<foreach>` iterates over a runtime-evaluated array.
    ForeachAction { state_id: String },
    /// W3C SCXML 6.4 — hybrid `<invoke>` with `srcexpr` or `contentexpr`;
    /// the target is resolved at `<invoke>` entry by the script engine.
    HybridInvoke { invoke_id: String },
    /// W3C SCXML 6.4.1 — static `<invoke namelist="...">` reads datamodel
    /// variables to form the child's initial state at entry.
    StaticInvokeNamelist { invoke_id: String },
    /// SCE Mesh §9.5 — `<invoke type="sce:mesh-rpc">` with `srcexpr`
    /// target; the generated entry block calls `evaluateExpression`.
    MeshRpcSrcExpr { invoke_id: String },
    /// W3C SCXML 5.7 — `<donedata>` carries at least one `<param>` whose
    /// value must be evaluated when the final state is entered.
    DonedataParam { state_id: String },
    /// W3C SCXML 5.7 — `<donedata>` has `<content>` (inline text or `expr`)
    /// to be evaluated before the `done.state` event is raised.
    DonedataContent { state_id: String },
    /// W3C SCXML 6.4 — a static `<invoke>` targets a child SCXML whose
    /// own analyzer output declared `needs_script_engine = true`; the
    /// parent must carry an engine so the child can run.
    ChildInvokeNeedsScriptEngine { invoke_id: String },
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
        collect_state_causes(state_id, state, &mut causes);
    }
    causes
}

/// Convenience: `!analyze(model).is_empty()`. Avoids allocating the
/// full cause list when only the flag is needed.
pub fn requires_script_engine(model: &SCXMLModel) -> bool {
    // Inlined fast-path: stop at the first cause rather than building
    // the full vector. `analyze` is still the canonical spelling for
    // debug / test contexts that want the cause set.
    has_datamodel_cause(&model.variables)
        || has_global_script_cause(model)
        || model
            .states
            .iter()
            .any(|(state_id, state)| has_state_cause(state_id, state))
}

fn collect_datamodel_causes(variables: &[Variable], out: &mut Vec<NeedsScriptEngineCause>) {
    for var in variables {
        if variable_needs_engine(var) {
            out.push(NeedsScriptEngineCause::DatamodelVariableInit {
                var_id: var.id.clone(),
            });
        }
    }
}

fn has_datamodel_cause(variables: &[Variable]) -> bool {
    variables.iter().any(variable_needs_engine)
}

fn variable_needs_engine(var: &Variable) -> bool {
    // W3C SCXML 5.3: any initializer (expr/src/content) needs the engine
    // to evaluate at runtime. Tighter classification (int / string / bool
    // literal → static init) is orthogonal — it happens later in
    // [`crate::analyzer::classify_variables`] and doesn't affect this flag.
    !var.expr.is_empty() || !var.src.is_empty() || !var.content.is_empty()
}

fn collect_global_script_causes(model: &SCXMLModel, out: &mut Vec<NeedsScriptEngineCause>) {
    if !model.global_scripts.is_empty() {
        out.push(NeedsScriptEngineCause::GlobalScript);
    }
    if model.has_unresolved_external_script {
        out.push(NeedsScriptEngineCause::UnresolvedExternalScript);
    }
}

fn has_global_script_cause(model: &SCXMLModel) -> bool {
    !model.global_scripts.is_empty() || model.has_unresolved_external_script
}

fn collect_state_causes(state_id: &str, state: &State, out: &mut Vec<NeedsScriptEngineCause>) {
    for trans in &state.transitions {
        if transition_guard_needs_engine(trans) {
            out.push(NeedsScriptEngineCause::TransitionGuard {
                source_state: state_id.to_string(),
            });
        }
        for action in &trans.actions {
            collect_action_causes(state_id, action, out);
        }
    }
    for block in state.on_entry_blocks.iter().chain(state.on_exit_blocks.iter()) {
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
        collect_donedata_causes(state_id, dd, out);
    }
}

fn has_state_cause(state_id: &str, state: &State) -> bool {
    for trans in &state.transitions {
        if transition_guard_needs_engine(trans) {
            return true;
        }
        if trans.actions.iter().any(|a| action_needs_engine(state_id, a)) {
            return true;
        }
    }
    let entry_exit = state
        .on_entry_blocks
        .iter()
        .chain(state.on_exit_blocks.iter());
    for block in entry_exit {
        if block.iter().any(|a| action_needs_engine(state_id, a)) {
            return true;
        }
    }
    if state
        .initial_transition_actions
        .iter()
        .any(|a| action_needs_engine(state_id, a))
    {
        return true;
    }
    if state
        .initial_history_default_actions
        .iter()
        .any(|a| action_needs_engine(state_id, a))
    {
        return true;
    }
    if state.invokes.iter().any(invoke_needs_engine) {
        return true;
    }
    if let Some(dd) = &state.donedata {
        if donedata_needs_engine(dd) {
            return true;
        }
    }
    false
}

fn transition_guard_needs_engine(trans: &crate::model::Transition) -> bool {
    if trans.cond.is_empty() || trans.is_cpp_condition || trans.is_kt_condition {
        return false;
    }
    let (needs_se, _has_in) = crate::parser::check_expression_needs(&trans.cond);
    needs_se
}

fn collect_action_causes(
    state_id: &str,
    action: &Action,
    out: &mut Vec<NeedsScriptEngineCause>,
) {
    match action.action_type.as_str() {
        "send" => {
            if !action.namelist.is_empty() {
                out.push(NeedsScriptEngineCause::SendNamelist {
                    state_id: state_id.to_string(),
                });
            }
            for param in &action.params {
                if !param.expr.is_empty() && !param.is_static_literal {
                    out.push(NeedsScriptEngineCause::SendParamExpr {
                        state_id: state_id.to_string(),
                        param_name: param.name.clone(),
                    });
                }
            }
            if send_has_dynamic_attr(action) {
                out.push(NeedsScriptEngineCause::SendDynamicAttr {
                    state_id: state_id.to_string(),
                });
            }
        }
        "if" => {
            if if_cond_needs_engine(&action.cond, action) {
                out.push(NeedsScriptEngineCause::IfCondition {
                    state_id: state_id.to_string(),
                });
            }
            for branch in &action.elseif_branches {
                if elseif_needs_engine(branch) {
                    out.push(NeedsScriptEngineCause::ElseIfCondition {
                        state_id: state_id.to_string(),
                    });
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
            // W3C SCXML 5.4: `<assign>` is always engine-bound; even pure
            // location='var' expr='literal' routes through the assignment
            // helper which talks to the datamodel store.
            out.push(NeedsScriptEngineCause::AssignAction {
                state_id: state_id.to_string(),
            });
        }
        "log" => {
            if !action.expr.is_empty() {
                out.push(NeedsScriptEngineCause::LogExpr {
                    state_id: state_id.to_string(),
                });
            }
        }
        "script" => {
            // Inline `<script>` action. Native `<cpp>` / `<kt>` child
            // blocks are emitted as source code and set the respective
            // `is_*_function` flag — they do not require the engine.
            if !action.is_cpp_function && !action.is_kt_function {
                out.push(NeedsScriptEngineCause::InlineScriptAction {
                    state_id: state_id.to_string(),
                });
            }
        }
        "cancel" => {
            if !action.sendidexpr.is_empty() {
                out.push(NeedsScriptEngineCause::CancelExpr {
                    state_id: state_id.to_string(),
                });
            }
        }
        "foreach" => {
            out.push(NeedsScriptEngineCause::ForeachAction {
                state_id: state_id.to_string(),
            });
            for nested in &action.actions {
                collect_action_causes(state_id, nested, out);
            }
        }
        _ => {}
    }
}

fn action_needs_engine(state_id: &str, action: &Action) -> bool {
    match action.action_type.as_str() {
        "send" => {
            if !action.namelist.is_empty() {
                return true;
            }
            if action
                .params
                .iter()
                .any(|p| !p.expr.is_empty() && !p.is_static_literal)
            {
                return true;
            }
            send_has_dynamic_attr(action)
        }
        "if" => {
            if if_cond_needs_engine(&action.cond, action) {
                return true;
            }
            if action.elseif_branches.iter().any(elseif_needs_engine) {
                return true;
            }
            if action
                .then_actions
                .iter()
                .any(|a| action_needs_engine(state_id, a))
            {
                return true;
            }
            if action
                .else_actions
                .iter()
                .any(|a| action_needs_engine(state_id, a))
            {
                return true;
            }
            action
                .elseif_branches
                .iter()
                .any(|b| b.actions.iter().any(|a| action_needs_engine(state_id, a)))
        }
        "assign" => true,
        "log" => !action.expr.is_empty(),
        "script" => !action.is_cpp_function && !action.is_kt_function,
        "cancel" => !action.sendidexpr.is_empty(),
        "foreach" => true,
        _ => false,
    }
}

fn send_has_dynamic_attr(action: &Action) -> bool {
    !action.eventexpr.is_empty()
        || !action.targetexpr.is_empty()
        || !action.delayexpr.is_empty()
        || !action.typeexpr.is_empty()
}

fn if_cond_needs_engine(cond: &str, action: &Action) -> bool {
    // Mirrors `parse_if_action`: a native `<if>` condition is surfaced via
    // `action.cond_cpp` / `action.cond_kt`; if either is populated the raw
    // ECMAScript branch is not the one that gets emitted.
    if cond.is_empty() {
        return false;
    }
    if !action.cond_cpp.is_empty() || !action.cond_kt.is_empty() {
        return false;
    }
    let (needs_se, _has_in) = crate::parser::check_expression_needs(cond);
    needs_se
}

fn elseif_needs_engine(branch: &ElseIfBranch) -> bool {
    if branch.cond.is_empty() {
        return false;
    }
    if !branch.cond_cpp.is_empty() || !branch.cond_kt.is_empty() {
        return false;
    }
    let (needs_se, _has_in) = crate::parser::check_expression_needs(&branch.cond);
    needs_se
}

fn collect_invoke_causes(invoke: &Invoke, out: &mut Vec<NeedsScriptEngineCause>) {
    match invoke {
        Invoke::Hybrid(info) => {
            out.push(NeedsScriptEngineCause::HybridInvoke {
                invoke_id: info.common.base.invoke_id.clone(),
            });
            push_child_invoke_cause(&info.common, out);
        }
        Invoke::Scxml(info) => {
            if !info.namelist.is_empty() {
                out.push(NeedsScriptEngineCause::StaticInvokeNamelist {
                    invoke_id: info.common.base.invoke_id.clone(),
                });
            }
            push_child_invoke_cause(&info.common, out);
        }
        Invoke::MeshRpc(info) => {
            if matches!(&info.target, MeshRpcTarget::SrcExpr { .. }) {
                out.push(NeedsScriptEngineCause::MeshRpcSrcExpr {
                    invoke_id: info.base.invoke_id.clone(),
                });
            }
        }
    }
}

fn invoke_needs_engine(invoke: &Invoke) -> bool {
    match invoke {
        Invoke::Hybrid(_) => true,
        Invoke::Scxml(info) => !info.namelist.is_empty() || info.common.child_needs_script_engine,
        Invoke::MeshRpc(info) => matches!(&info.target, MeshRpcTarget::SrcExpr { .. }),
    }
}

fn push_child_invoke_cause(common: &InvokeSessionCommon, out: &mut Vec<NeedsScriptEngineCause>) {
    if common.child_needs_script_engine {
        out.push(NeedsScriptEngineCause::ChildInvokeNeedsScriptEngine {
            invoke_id: common.base.invoke_id.clone(),
        });
    }
}

fn collect_donedata_causes(
    state_id: &str,
    dd: &DoneData,
    out: &mut Vec<NeedsScriptEngineCause>,
) {
    if !dd.params.is_empty() {
        out.push(NeedsScriptEngineCause::DonedataParam {
            state_id: state_id.to_string(),
        });
    }
    if !dd.content.is_empty() || !dd.contentexpr.is_empty() {
        out.push(NeedsScriptEngineCause::DonedataContent {
            state_id: state_id.to_string(),
        });
    }
}

fn donedata_needs_engine(dd: &DoneData) -> bool {
    !dd.params.is_empty() || !dd.content.is_empty() || !dd.contentexpr.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SCXMLParser;

    fn parse(scxml: &str) -> SCXMLModel {
        SCXMLParser::new().parse_string(scxml, "test").unwrap()
    }

    fn single_cause(scxml: &str) -> NeedsScriptEngineCause {
        let model = parse(scxml);
        let causes = analyze(&model);
        assert_eq!(
            causes.len(),
            1,
            "expected exactly one cause, got {:?}",
            causes,
        );
        causes.into_iter().next().unwrap()
    }

    fn contains_cause<F>(scxml: &str, predicate: F)
    where
        F: Fn(&NeedsScriptEngineCause) -> bool,
    {
        let model = parse(scxml);
        let causes = analyze(&model);
        assert!(
            causes.iter().any(&predicate),
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
            NeedsScriptEngineCause::DatamodelVariableInit { var_id } if var_id == "x"
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
            NeedsScriptEngineCause::GlobalScript
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
            .any(|c| matches!(c, NeedsScriptEngineCause::UnresolvedExternalScript)));
    }

    #[test]
    fn transition_guard_triggers() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s"><transition event="e" cond="1 == 1" target="s"/></state>
        </scxml>"#;
        assert!(matches!(
            single_cause(scxml),
            NeedsScriptEngineCause::TransitionGuard { source_state } if source_state == "s"
        ));
    }

    #[test]
    fn send_namelist_triggers() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s"><onentry><send event="e" namelist="a b"/></onentry></state>
        </scxml>"#;
        contains_cause(scxml, |c| {
            matches!(c, NeedsScriptEngineCause::SendNamelist { state_id } if state_id == "s")
        });
    }

    #[test]
    fn send_param_expr_triggers() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s"><onentry>
                <send event="e"><param name="p" expr="1 + 2"/></send>
            </onentry></state>
        </scxml>"#;
        contains_cause(scxml, |c| matches!(
            c,
            NeedsScriptEngineCause::SendParamExpr { param_name, .. } if param_name == "p"
        ));
    }

    #[test]
    fn send_dynamic_attr_triggers() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s"><onentry><send eventexpr="'e'"/></onentry></state>
        </scxml>"#;
        contains_cause(scxml, |c| {
            matches!(c, NeedsScriptEngineCause::SendDynamicAttr { state_id } if state_id == "s")
        });
    }

    #[test]
    fn if_condition_triggers() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s"><onentry><if cond="1 == 1"/></onentry></state>
        </scxml>"#;
        contains_cause(scxml, |c| {
            matches!(c, NeedsScriptEngineCause::IfCondition { state_id } if state_id == "s")
        });
    }

    #[test]
    fn elseif_condition_triggers() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s"><onentry>
                <if cond="In('s')"><elseif cond="1 == 1"/></if>
            </onentry></state>
        </scxml>"#;
        contains_cause(scxml, |c| {
            matches!(c, NeedsScriptEngineCause::ElseIfCondition { state_id } if state_id == "s")
        });
    }

    #[test]
    fn assign_action_triggers() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s"><onentry><assign location="x" expr="1"/></onentry></state>
        </scxml>"#;
        contains_cause(scxml, |c| {
            matches!(c, NeedsScriptEngineCause::AssignAction { state_id } if state_id == "s")
        });
    }

    #[test]
    fn log_expr_triggers() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s"><onentry><log expr="1"/></onentry></state>
        </scxml>"#;
        contains_cause(scxml, |c| {
            matches!(c, NeedsScriptEngineCause::LogExpr { state_id } if state_id == "s")
        });
    }

    #[test]
    fn inline_script_action_triggers() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s"><onentry><script>var y = 2;</script></onentry></state>
        </scxml>"#;
        contains_cause(scxml, |c| {
            matches!(c, NeedsScriptEngineCause::InlineScriptAction { state_id } if state_id == "s")
        });
    }

    #[test]
    fn cancel_expr_triggers() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s"><onentry><cancel sendidexpr="'id'"/></onentry></state>
        </scxml>"#;
        contains_cause(scxml, |c| {
            matches!(c, NeedsScriptEngineCause::CancelExpr { state_id } if state_id == "s")
        });
    }

    #[test]
    fn foreach_action_triggers() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s"><onentry><foreach array="xs" item="x"/></onentry></state>
        </scxml>"#;
        contains_cause(scxml, |c| {
            matches!(c, NeedsScriptEngineCause::ForeachAction { state_id } if state_id == "s")
        });
    }

    #[test]
    fn hybrid_invoke_triggers() {
        let scxml = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s">
                <invoke id="h1" srcexpr="'child.scxml'"/>
            </state>
        </scxml>"##;
        contains_cause(scxml, |c| {
            matches!(c, NeedsScriptEngineCause::HybridInvoke { invoke_id } if invoke_id == "h1")
        });
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
        contains_cause(scxml, |c| matches!(
            c,
            NeedsScriptEngineCause::StaticInvokeNamelist { invoke_id } if invoke_id == "i1"
        ));
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
        contains_cause(scxml, |c| matches!(
            c,
            NeedsScriptEngineCause::MeshRpcSrcExpr { invoke_id } if invoke_id == "m1"
        ));
    }

    #[test]
    fn donedata_param_triggers() {
        let scxml = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s" initial="f"><final id="f"><donedata><param name="x" expr="1"/></donedata></final></state>
        </scxml>"##;
        contains_cause(scxml, |c| {
            matches!(c, NeedsScriptEngineCause::DonedataParam { state_id } if state_id == "f")
        });
    }

    #[test]
    fn donedata_content_triggers() {
        let scxml = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s" initial="f"><final id="f"><donedata><content expr="1"/></donedata></final></state>
        </scxml>"##;
        contains_cause(scxml, |c| {
            matches!(c, NeedsScriptEngineCause::DonedataContent { state_id } if state_id == "f")
        });
    }

    #[test]
    fn requires_script_engine_matches_analyze() {
        // Fast-path parity: `requires_script_engine` must return `true`
        // exactly when `analyze` returns a non-empty set. The contract
        // lets downstream pass through without allocating the vec.
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
            let fast = requires_script_engine(&model);
            let slow = !analyze(&model).is_empty();
            assert_eq!(fast, slow, "fast/slow parity broken for {scxml}");
            assert_eq!(fast, *expected, "unexpected flag for {scxml}");
        }
    }
}
