// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Model analysis pipeline — ports base.py shared analysis methods.
// Language-agnostic SCXML model enrichment: variable classification,
// feature detection, event prefix matching, internal transition resolution.

use crate::model::*;
use std::path::Path;

/// Run the full analysis pipeline on a parsed model.
pub fn analyze(model: &mut SCXMLModel, scxml_path: &str) {
    classify_variables(model);
    analyze_model_features(model);
    add_system_events(model);
    compute_external_ingress_events(model);
    compute_typed_inject_events(model);
    build_prefix_matching(model);

    // SCE_MESH.md §9.6 codegen-shape seam. Initial value from SCXML-only
    // state — `is_remote_invoke_target` is still false here because this
    // pipeline stage has not yet read deploy.yaml. The mesh-inject stage
    // (`inject_partition_context_for`) recomputes the derived flag after
    // populating `is_remote_invoke_target` from `scxml_remote_inbound_peers`.
    // The value set here is what pure-SCXML consumers (conformance harness,
    // WASM online codegen without deploy) observe; deploy.yaml-aware builds
    // override it with the two-input flag. Single source of truth for the
    // "emit ParentStateMachine template" condition consumed by
    // state_machine.jinja2 / process_transition.jinja2 /
    // entry_exit_actions.jinja2 / actions/send.jinja2.
    model.needs_parent_template = model.has_parent_communication && !model.is_remote_invoke_target;

    // Named Context: set needs_nonstatic_method
    model.needs_nonstatic_method = model.needs_script_engine
        || model.has_scxml_invoke()
        || model.has_parent_communication
        || model.has_parallel_states
        || model.uses_in_predicate
        || !model.context_objects.is_empty();

    // `executeEntryActions` must be non-static whenever its switch body
    // emits a reference to `this` or requires engine-bound state that the
    // static form cannot reach. The set is derived rather than hand-maintained
    // in the template's conditional — new `this`-bearing features add one
    // clause here instead of threading another negation into a fragile
    // multi-line `{% if not A and not B and ... %}`.
    model.execute_entry_actions_needs_this = model.needs_script_engine
        || model.has_scxml_invoke()
        || model.has_mesh_rpc_invoke()
        || model.has_parent_communication
        || model.needs_event_scheduler.unwrap_or(false)
        || model.has_parallel_states
        || !model.context_objects.is_empty();

    // Rust-specific analysis
    resolve_internal_transitions(model);
    model.scxml_base_path = compute_scxml_base_path(scxml_path);
}

/// §scxml-5.3: Classify datamodel variables by type.
///
/// `needs_script_engine` is not set here — it is derived for the whole
/// model by [`crate::script_engine_analyzer`] at the end of parse.
/// Every variable with a non-empty initializer contributes a
/// [`crate::script_engine_analyzer::NeedsScriptEngineCause::DatamodelVariableInit`]
/// cause, which is a superset of the non-literal case this function used
/// to flag individually.
/// First non-whitespace character of `s`, or `None` if `s` is empty or
/// contains only whitespace. Mirrors cpp's
/// `content.find_first_not_of(" \t\r\n")` pattern in
/// `DataModelInitHelper::initializeVariable` and `LuaEngine::setCurrentEvent`.
fn first_non_ws(s: &str) -> Option<char> {
    s.chars().find(|c| !matches!(c, ' ' | '\t' | '\r' | '\n'))
}

fn classify_variables(model: &mut SCXMLModel) {
    for var in &mut model.variables {
        let expr = &var.expr;
        let content = &var.content;
        if expr.is_empty() && content.is_empty() {
            var.var_type = "runtime".to_string();
        } else if expr == "0"
            || (!expr.is_empty()
                && expr
                    .strip_prefix('-')
                    .unwrap_or(expr)
                    .chars()
                    .all(|c| c.is_ascii_digit())
                && !expr.strip_prefix('-').unwrap_or(expr).is_empty())
        {
            var.var_type = "int".to_string();
        } else if expr.starts_with('"') && expr.ends_with('"') {
            var.var_type = "string".to_string();
        } else if expr == "true" || expr == "false" {
            var.var_type = "bool".to_string();
        } else {
            var.var_type = "runtime".to_string();
        }
    }
}

/// Analyze model and set feature flags.
fn analyze_model_features(model: &mut SCXMLModel) {
    // Always true: every state machine needs transition handling
    model.needs_transition_helper = Some(true);
    model.needs_event_type_helper = Some(false);
    model.needs_assign_helper = Some(false);
    model.needs_foreach = Some(false);
    model.needs_guard_helper = Some(false);
    model.needs_send_helper = Some(false);
    model.needs_event_data_helper = Some(false);
    model.needs_donedata_helper = Some(false);
    model.needs_namelist_helper = Some(false);
    model.needs_dom_helper = Some(false);

    model.needs_event_name = false;
    model.needs_event_data = false;
    model.needs_event_type = false;
    model.needs_event_sendid = false;
    model.needs_event_origin = false;
    model.needs_event_origintype = false;
    model.needs_event_invokeid = false;
    model.needs_external_flag = false;

    // Temporarily take states out of model to avoid borrow conflict:
    // analyze_action needs &mut model (for events/flags) but never accesses model.states.
    let states = std::mem::take(&mut model.states);
    for state in states.values() {
        for trans in &state.transitions {
            if !trans.cond.is_empty() {
                model.needs_guard_helper = Some(true);
            }
            for action in &trans.actions {
                analyze_action(action, model);
            }
        }
        for block in state
            .on_entry_blocks
            .iter()
            .chain(state.on_exit_blocks.iter())
        {
            for action in block {
                analyze_action(action, model);
            }
        }
        // §scxml-3.3: Analyze initial transition actions
        for action in &state.initial_transition_actions {
            analyze_action(action, model);
        }
        // §scxml-3.11: Analyze history default actions
        for action in &state.initial_history_default_actions {
            analyze_action(action, model);
        }
    }
    model.states = states;

    // §scxml-5.2: Analyze document-level global scripts
    // Same pattern: take out to avoid borrow conflict with analyze_action(&mut model)
    let global_scripts = std::mem::take(&mut model.global_scripts);
    for action in &global_scripts {
        analyze_action(action, model);
    }
    model.global_scripts = global_scripts;

    // Script engine implies full event metadata
    if model.needs_script_engine {
        apply_script_engine_implications(model);
    }

    // Donedata detection
    for state in model.states.values() {
        if state.is_final && state.donedata.is_some() {
            model.needs_donedata_helper = Some(true);
            model.events.insert("error.execution".to_string());
            break;
        }
    }

    // watching-zenoh RFC §5.E B7-η' codegen wire-up: collect the
    // unique set of forge link names referenced by any
    // `<sce:on-sample link="X" .../>` block across all states.
    // Empty for documents without sample subscriptions; non-empty
    // drives the Rust state_machine template's per-link delivery
    // method emission. Sorted via BTreeSet so codegen output is
    // deterministic across runs.
    //
    // The event-name side of the same iteration registers each
    // `<sce:on-sample event="Y">` value into `model.events` so
    // backends emitting an enum + per-name dispatch (C11
    // `<machine>_event_t`) include the on-sample-only events
    // even when the SCXML lacks a `<transition event="Y">`
    // declaration locally — without this, the W2 C11 codegen's
    // `WATCHER_EVENT_SCOUT_TICK` reference would fail to compile
    // for documents that subscribe but don't react in the same
    // file. Rust's by-name lookup tolerates absence (silent
    // drop), but registering uniformly keeps the two backends
    // semantically aligned.
    for state in model.states.values() {
        for block in &state.on_sample_blocks {
            model.on_sample_links.insert(block.link.clone());
            model.events.insert(block.event.clone());
        }
    }

    // §scxml-B-2 (test557): inline `<data>` content whose first
    // non-whitespace character is `<` triggers the host-side XML DOM
    // helper. Mirrors cpp `DataModelInitHelper::initializeVariable`
    // first-char check (sce/src/common/DataModelInitHelper.cpp:103).
    // Both top-level (`model.variables`) and state-local datamodel
    // entries are inspected. The `<data src="...">` path defers content
    // detection to the template (the file is read by the
    // `read_data_src` filter at codegen time and the same first-char
    // check fires there); this analyzer pass only handles inline
    // content because every src=XML fixture in the current corpus
    // (test557) also carries an inline `<` sibling that triggers here.
    let needs_dom_from_inline = model
        .variables
        .iter()
        .any(|v| first_non_ws(&v.content) == Some('<'))
        || model.states.values().any(|state| {
            state
                .datamodel
                .iter()
                .any(|v| first_non_ws(&v.content) == Some('<'))
        });
    if needs_dom_from_inline {
        model.needs_dom_helper = Some(true);
    }

    // Donedata-param/content and child-invoke `needs_script_engine`
    // propagation used to flip the flag a second time here; both cases
    // are now folded into [`crate::script_engine_analyzer`]
    // ([`crate::script_engine_analyzer::NeedsScriptEngineCause::DonedataParam`] /
    // [`crate::script_engine_analyzer::NeedsScriptEngineCause::DonedataContent`] /
    // [`crate::script_engine_analyzer::NeedsScriptEngineCause::ChildInvokeNeedsScriptEngine`])
    // and applied during parse. `apply_script_engine_implications` above
    // has already fired once if the flag is set, so the downstream event
    // metadata is consistent with the pre-refactor behaviour.
}

fn apply_script_engine_implications(model: &mut SCXMLModel) {
    model.needs_event_name = true;
    model.needs_event_data = true;
    model.needs_event_type = true;
    model.needs_event_sendid = true;
    model.needs_event_origin = true;
    model.needs_event_origintype = true;
    model.needs_event_invokeid = true;
    model.needs_external_flag = true;
    model.events.insert("error.execution".to_string());
    model.needs_event_type_helper = Some(true);
    model.needs_assign_helper = Some(true);
    model.needs_foreach = Some(true);
    model.needs_guard_helper = Some(true);
}

/// Check specific string fields of an action for _event.* metadata patterns.
/// Uses direct field inspection instead of Debug format to avoid false positives
/// (e.g., `_event.origin` matching inside `_event.origintype`).
fn check_action_event_fields(action: &Action, model: &mut SCXMLModel) {
    let fields_to_check = [
        &action.expr,
        &action.target,
        &action.targetexpr,
        &action.cond,
        &action.content,
        &action.contentexpr,
        &action.eventexpr,
        &action.namelist,
        &action.typeexpr,
        &action.idlocation,
    ];
    for field in &fields_to_check {
        check_event_field(field, model);
    }
    for param in &action.params {
        check_event_field(&param.expr, model);
        check_event_field(&param.location, model);
    }
}

fn check_event_field(field: &str, model: &mut SCXMLModel) {
    if !field.contains("_event.") {
        return;
    }
    if field.contains("_event.sendid") {
        model.needs_event_sendid = true;
    }
    if field.contains("_event.origintype") {
        model.needs_event_origintype = true;
    }
    // Check _event.origin independently: exclude substring matches inside _event.origintype
    if field.contains("_event.origin") {
        // Set origin if field has _event.origin not followed by "type"
        let mut pos = 0;
        while let Some(idx) = field[pos..].find("_event.origin") {
            let abs_idx = pos + idx + "_event.origin".len();
            if abs_idx >= field.len() || !field[abs_idx..].starts_with("type") {
                model.needs_event_origin = true;
                break;
            }
            pos = abs_idx;
        }
    }
    if field.contains("_event.invokeid") {
        model.needs_event_invokeid = true;
    }
    if field.contains("_event.data") {
        model.needs_event_data = true;
    }
    if field.contains("_event.name") {
        model.needs_event_name = true;
    }
    if field.contains("_event.type") {
        model.needs_event_type = true;
    }
}

fn analyze_action(action: &Action, model: &mut SCXMLModel) {
    match action.action_type.as_str() {
        "send" => {
            model.needs_send_helper = Some(true);
            model.events.insert("error.execution".to_string());
            if !action.params.is_empty() {
                model.needs_event_data_helper = Some(true);
            }
            if !action.delay.is_empty() || !action.delayexpr.is_empty() {
                model.needs_event_scheduler = Some(true);
            }
            // §scxml-C-1 (test553): namelist evaluation needs the
            // declared-var set so undeclared names trigger error.execution
            // (cpp `NamelistHelper::evaluateNamelist` calls `hasVariable`
            // before reading; lua's silent-nil-for-undeclared semantic is
            // bridged via the same `_scxml_declared` table donedata uses).
            if !action.namelist.is_empty() {
                model.needs_namelist_helper = Some(true);
            }
            // §scxml-B-2 (test561): a `<send><content>` literal whose
            // first non-whitespace character is `<` is delivered to the
            // receiver as an XML DOM object, mirroring cpp
            // `LuaEngine::setCurrentEvent` first-char check
            // (LuaEngine.cpp:1145-1149). The flag gates the host-side DOM
            // helper register call in scriptengine.jinja2 and the XML
            // branch on the event-promotion site.
            if first_non_ws(&action.content) == Some('<') {
                model.needs_dom_helper = Some(true);
            }
            // §scxml-C-2: BasicHTTP send detection
            if action.send_type == "http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor"
                && (action.target.starts_with("http://")
                    || action.target.starts_with("https://")
                    || !action.targetexpr.is_empty())
            {
                model.needs_http_send = true;
            }
            // W3C SCXML: SCXMLEventProcessor external flag
            if action.send_type == "http://www.w3.org/TR/scxml/#SCXMLEventProcessor" {
                model.needs_external_flag = true;
            }
            check_action_event_fields(action, model);
        }
        "cancel" => {
            model.needs_event_scheduler = Some(true);
            // W3C SCXML 6.3: a `<cancel>` makes the scheduler's per-entry
            // `send_id` load-bearing, so the Rust backend must keep `SceString`
            // for `StatePolicy::ScheduledSendId` rather than eliding it.
            model.uses_cancel = true;
        }
        "assign" => {
            model.needs_assign_helper = Some(true);
            check_action_event_fields(action, model);
        }
        "log" => {
            check_action_event_fields(action, model);
        }
        "foreach" => {
            model.needs_foreach = Some(true);
            check_action_event_fields(action, model);
            for nested in &action.actions {
                analyze_action(nested, model);
            }
        }
        "if" => {
            if !action.cond.is_empty() {
                model.needs_guard_helper = Some(true);
            }
            check_action_event_fields(action, model);
            // Check elseif branch conditions for _event.* metadata
            for branch in &action.elseif_branches {
                check_event_field(&branch.cond, model);
            }
            for nested in &action.then_actions {
                analyze_action(nested, model);
            }
            for branch in &action.elseif_branches {
                for nested in &branch.actions {
                    analyze_action(nested, model);
                }
            }
            for nested in &action.else_actions {
                analyze_action(nested, model);
            }
        }
        _ => {}
    }
}

/// Add system-level events (wildcards, invoke events).
fn add_system_events(model: &mut SCXMLModel) {
    let has_wildcard = model.states.values().any(|state| {
        state
            .transitions
            .iter()
            .any(|t| t.event == "*" || t.event == ".*")
    });
    if has_wildcard {
        model.events.insert("Wildcard".to_string());
    }

    if model.has_scxml_invoke() {
        model.events.insert("done.invoke".to_string());
        model.events.insert("cancel.invoke".to_string());
        model.events.insert("error.execution".to_string());
    }
    // SCE_MESH.md §9.5: mesh-rpc invokes raise done.invoke.<id> on reply
    // and error.invoke.<id> on timeout or non-Ok status. cancel.invoke is
    // scxml-specific (§scxml-6.4 child-session cancel) and not raised for
    // mesh-rpc — cancellation erases the correlation entry silently.
    // error.execution (§scxml-6.4.1) is raised when `performMeshInvoke`
    // returns false — i.e. the document was rendered without a
    // TransportRouter installing the mesh-invoke callback. Same error
    // event scxml invokes emit on invocation failure; listing it here
    // keeps the generated `Event::Error_execution` enum value resolvable
    // whether or not the author wrote an explicit handler.
    if model.has_mesh_rpc_invoke() {
        model.events.insert("done.invoke".to_string());
        model.events.insert("error.invoke".to_string());
        model.events.insert("error.execution".to_string());
    }
}

/// Derive the external-ingress event set consumed by transport
/// switchboards (see [`SCXMLModel::external_ingress_events`]). Walks
/// every `<transition event="...">` trigger, splits the W3C SCXML
/// 3.12.1 space-separated descriptor list, and keeps the tokens that
/// are not engine-reserved. Runs before [`build_prefix_matching`] so it
/// reads the authored `transition.event` strings unmodified.
fn compute_external_ingress_events(model: &mut SCXMLModel) {
    let mut ingress = std::collections::BTreeSet::new();
    for state in model.states.values() {
        for transition in &state.transitions {
            for token in transition.event.split_whitespace() {
                if !is_reserved_ingress_event(token) {
                    ingress.insert(token.to_string());
                }
            }
        }
    }
    model.external_ingress_events = ingress;
}

/// Populate [`SCXMLModel::typed_inject_events`] — the per-event
/// typed-inject seam set (see that field's doc). Derived from the shared
/// [`crate::forge::event_schema_check::select_native_typed_guards`]
/// selection so the published set is byte-identical to the events for
/// which codegen emits a `<Machine>Inject::raise_<event>` method. Runs
/// before [`build_prefix_matching`] for the same reason
/// [`compute_external_ingress_events`] does: it reads the authored
/// `transition.event` (the key `imported_event_schemas` is indexed by)
/// before any prefix-matching enrichment. `imported_event_schemas` is
/// already resolved at parse time (the in-memory WASM path leaves it
/// empty, yielding an empty set — the schemaless baseline).
fn compute_typed_inject_events(model: &mut SCXMLModel) {
    model.typed_inject_events = crate::forge::event_schema_check::native_typed_inject_events(model);
}

/// True for event tokens a transport switchboard must not target:
/// engine-synthesized W3C platform events and the wildcard / eventless
/// sentinels. Mirrors the families [`add_system_events`] injects plus
/// the `<transition>` wildcard tokens, so the reserved-event taxonomy
/// has exactly one owner.
fn is_reserved_ingress_event(event: &str) -> bool {
    event.is_empty()
        || matches!(event, "*" | ".*" | "_*")
        || event.starts_with("error.")
        || event.starts_with("done.invoke")
        || event.starts_with("done.state")
        || event == "cancel.invoke"
}

/// §scxml-3.12.1: Build prefix matching for event transitions.
fn build_prefix_matching(model: &mut SCXMLModel) {
    let all_events: Vec<String> = model.events.iter().cloned().collect();
    model.needs_event_matching_helper = false;

    let state_ids: Vec<String> = model.states.keys().cloned().collect();
    for state_id in &state_ids {
        // Take transitions out of the state to avoid cloning the entire state
        let mut updated_transitions =
            std::mem::take(&mut model.states.get_mut(state_id).unwrap().transitions);

        for trans in &mut updated_transitions {
            if trans.event.is_empty() {
                continue;
            }

            // Wildcards and glob patterns need runtime matching
            if trans.event == "*"
                || trans.event == ".*"
                || trans.event == "_*"
                || trans.event.contains(".*")
                || trans.event.contains(' ')
            {
                trans.needs_string_matching = true;
                model.needs_event_matching_helper = true;

                // Compute prefix matching events
                let mut matching = std::collections::BTreeSet::new();
                for descriptor in trans.event.split_whitespace() {
                    if descriptor == "*" || descriptor == ".*" || descriptor == "_*" {
                        matching.extend(all_events.iter().cloned());
                    } else if let Some(base) = descriptor.strip_suffix(".*") {
                        for event in &all_events {
                            if event.starts_with(&format!("{base}.")) {
                                matching.insert(event.clone());
                            }
                        }
                    } else {
                        for event in &all_events {
                            if event == descriptor || event.starts_with(&format!("{descriptor}.")) {
                                matching.insert(event.clone());
                            }
                        }
                    }
                }
                trans.prefix_matching_events = matching.into_iter().collect();
                continue;
            }

            // Standard prefix matching
            let mut matching = std::collections::BTreeSet::new();
            for event in &all_events {
                if event == &trans.event || event.starts_with(&format!("{}.", trans.event)) {
                    matching.insert(event.clone());
                }
            }
            let sorted: Vec<String> = matching.into_iter().collect();
            trans.prefix_matching_events = sorted.clone();
            trans.matching_enum_values = sorted;
            trans.needs_string_matching = false;
        }

        model.states.get_mut(state_id).unwrap().transitions = updated_transitions;
    }
}

/// §scxml-3.13: Resolve internal transition types.
pub(crate) fn resolve_internal_transitions(model: &mut SCXMLModel) {
    let states_snapshot: Vec<(String, State)> = model
        .states
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    for (state_id, state) in &states_snapshot {
        let is_compound = !state.is_parallel
            && !state.is_final
            && model
                .states
                .values()
                .any(|s| s.parent.as_deref() == Some(state_id));

        let transitions = &state.transitions;
        let mut updates: Vec<(usize, String, bool, String)> = Vec::new();

        for (i, trans) in transitions.iter().enumerate() {
            if trans.transition_type == "internal" && !trans.target.is_empty() {
                let mut is_descendant = false;
                let mut current = trans.target.clone();
                let mut depth = 0;
                while let Some(s) = model.states.get(&current) {
                    if depth >= MAX_STATE_DEPTH {
                        break;
                    }
                    depth += 1;
                    if let Some(parent) = &s.parent {
                        if parent == state_id {
                            is_descendant = true;
                            break;
                        }
                        current = parent.clone();
                    } else {
                        break;
                    }
                }

                if !is_descendant || !is_compound {
                    updates.push((i, "external".to_string(), false, String::new()));
                } else {
                    updates.push((i, "internal".to_string(), true, state_id.clone()));
                }
            }
        }

        for (i, ttype, is_true_internal, source) in updates {
            let trans = &mut model.states.get_mut(state_id).unwrap().transitions[i];
            trans.transition_type = ttype;
            if is_true_internal {
                trans.is_true_internal = Some(true);
                trans.internal_source = Some(source);
            }
        }
    }
}

fn compute_scxml_base_path(scxml_path: &str) -> String {
    let parent = Path::new(scxml_path).parent().unwrap_or(Path::new("."));
    if let Ok(cwd) = std::env::current_dir() {
        if let Ok(rel) = parent.strip_prefix(&cwd) {
            return rel.to_string_lossy().to_string();
        }
    }
    parent.to_string_lossy().to_string()
}

/// Check if model can be statically code-generated.
///
/// Returns `Ok(())` when every precondition the static generator relies
/// on is met. The `Err` arm carries a typed [`ForgeError`] naming
/// *which* precondition failed.
///
/// RFC §W5 D3 refit splits the prior single-reason channel
/// (`ValidationError::DynamicFeatures` for all three reasons) into
/// stage-correct typed surfaces:
///
/// - **Top-level `<script>` rejected (§scxml-5.8)** →
///   [`ScxmlSemanticError::TopLevelScriptUnloaded`]
///   (`scxml/top-level-script-unloaded`). Hard semantic violation;
///   the Interpreter would also reject. Mis-classified prior to W5
///   as `validation/dynamic-features` — corrected here.
///
/// - **No initial-state attribute** (runtime default resolution
///   required) → [`ValidationError::DynamicFeatures`]
///   (`validation/dynamic-features`). Genuine "dynamic feature":
///   runtime CAN resolve via §3.3 default; static generator cannot.
///   Stays in DynamicFeatures (correctly classified).
///
/// - **Initial-state names undeclared state** →
///   [`ScxmlSemanticError::InitialStateUnknown`]
///   (`validation/invalid-reference`). Hard semantic violation.
///   Mis-classified prior to W5 — corrected here.
///
/// [`ForgeError`]: crate::forge::error::ForgeError
/// [`ScxmlSemanticError::TopLevelScriptUnloaded`]: crate::scxml_semantic::ScxmlSemanticError::TopLevelScriptUnloaded
/// [`ScxmlSemanticError::InitialStateUnknown`]: crate::scxml_semantic::ScxmlSemanticError::InitialStateUnknown
/// [`ValidationError::DynamicFeatures`]: crate::forge::error::ValidationError::DynamicFeatures
pub fn can_generate_static(model: &SCXMLModel) -> Result<(), crate::forge::error::ForgeError> {
    use crate::forge::error::{ForgeError, ValidationError};
    use crate::scxml_semantic::{InitialStateScope, ScxmlSemanticError};

    if model.document_rejected {
        // Analyzer path doesn't have the failing script's index/src
        // (parse_global_scripts sets `model.document_rejected = true`
        // without preserving the metadata). Both fields stay None;
        // C++ side captures detail at parser-throw time. The drift
        // test pins both surfaces emit the same wire code.
        return Err(ForgeError::Scxml(Box::new(
            ScxmlSemanticError::TopLevelScriptUnloaded {
                index: None,
                src: None,
            },
        )));
    }
    if model.initial.is_empty() {
        // Genuine dynamic-feature: runtime default resolution per
        // §scxml-3.3 picks the first child; static generator
        // has no equivalent fallback. The Interpreter would NOT
        // reject this document — it's a codegen-pipeline limitation,
        // not a semantic violation.
        return Err(ForgeError::Validation(Box::new(
            ValidationError::DynamicFeatures {
                name: model.name.clone(),
                reason: "no initial state (runtime default resolution required)".to_string(),
            },
        )));
    }
    let initial_states: Vec<&str> = model.initial.split_whitespace().collect();
    let all_known = if initial_states.len() > 1 {
        initial_states.iter().all(|s| model.states.contains_key(*s))
    } else {
        model.states.contains_key(&model.initial)
    };
    if !all_known {
        // Hard semantic violation: an Interpreter would also reject
        // because §3.3 cannot resolve a non-existent state id. The
        // available list feeds the structured `ReplaceOneOf` fix.
        let available: Vec<String> = model.states.keys().cloned().collect();
        return Err(ForgeError::Scxml(Box::new(
            ScxmlSemanticError::InitialStateUnknown {
                state_id: model.initial.clone(),
                scope: InitialStateScope::DocumentRoot,
                available,
            },
        )));
    }
    Ok(())
}

// ── Shared computed analysis (language-agnostic) ─────────────────
//
// These functions compute derived data structures from the parsed SCXML model.
// Used by language generators that need pre-computed context (e.g., Kotlin).
// C++/Rust templates access model fields directly and do not need these.

use std::collections::BTreeMap;

/// §scxml-3.13: Compute ancestor chains for transition routing.
///
/// Each state maps to an ordered list of ancestor state IDs (parent first).
pub fn compute_ancestor_chains(model: &SCXMLModel) -> BTreeMap<String, Vec<String>> {
    let mut ancestor_chains = BTreeMap::new();

    for (state_id, state) in &model.states {
        let mut chain = Vec::new();
        let mut current_id = state.parent.clone();
        while let Some(ref cid) = current_id {
            if !model.states.contains_key(cid) {
                break;
            }
            chain.push(cid.clone());
            current_id = model.states[cid].parent.clone();
        }
        ancestor_chains.insert(state_id.clone(), chain);
    }

    ancestor_chains
}

/// §scxml-3.3: Build parent map for state hierarchy.
pub fn compute_parent_map(model: &SCXMLModel) -> BTreeMap<String, String> {
    let mut parent_map = BTreeMap::new();

    for (state_id, state) in &model.states {
        if let Some(ref parent) = state.parent {
            if model.states.contains_key(parent) {
                parent_map.insert(state_id.clone(), parent.clone());
            }
        }
    }

    parent_map
}

/// §scxml-3.3 / §scxml-3.4: Build leaf map for compound/parallel state resolution.
pub fn compute_leaf_map(model: &SCXMLModel) -> BTreeMap<String, String> {
    let mut leaf_map = BTreeMap::new();

    for state_id in model.states.keys() {
        let leaf = model.resolve_to_leaf(state_id);
        if leaf != *state_id {
            leaf_map.insert(state_id.clone(), leaf);
        }
    }

    leaf_map
}

/// §scxml-3.2 / §scxml-3.4: Compute initial entry root.
pub fn compute_initial_entry_root(model: &SCXMLModel) -> String {
    let mut initial_entry_root = model.initial.clone();
    let mut current = model.initial.clone();

    while model.states.contains_key(&current) {
        let parent = &model.states[&current].parent;
        if let Some(ref pid) = parent {
            if model.states.contains_key(pid) {
                initial_entry_root = pid.clone();
                current = pid.clone();
            } else {
                break;
            }
        } else {
            break;
        }
    }

    initial_entry_root
}

/// Build a parent->children map for efficient descendant collection.
fn build_children_map(model: &SCXMLModel) -> BTreeMap<&str, Vec<&str>> {
    let mut children_map: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (state_id, state) in &model.states {
        if let Some(parent) = &state.parent {
            children_map
                .entry(parent.as_str())
                .or_default()
                .push(state_id.as_str());
        }
    }
    children_map
}

/// Collect all descendant state IDs using pre-built children map.
fn collect_descendants_fast(
    children_map: &BTreeMap<&str, Vec<&str>>,
    parent_id: &str,
    descendants: &mut Vec<String>,
) {
    if let Some(children) = children_map.get(parent_id) {
        for &child_id in children {
            descendants.push(child_id.to_string());
            collect_descendants_fast(children_map, child_id, descendants);
        }
    }
}

/// §scxml-3.4: Compute descendants for each parallel state.
pub fn compute_parallel_descendants(model: &SCXMLModel) -> BTreeMap<String, Vec<String>> {
    let children_map = build_children_map(model);
    let mut parallel_descendants = BTreeMap::new();

    for parallel_id in model.parallel_regions.keys() {
        let mut descendants = Vec::new();
        collect_descendants_fast(&children_map, parallel_id, &mut descendants);
        parallel_descendants.insert(parallel_id.clone(), descendants);
    }

    parallel_descendants
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::error::{ForgeError, ValidationError};
    use crate::scxml_semantic::{InitialStateScope, ScxmlSemanticError};

    fn empty_model() -> SCXMLModel {
        SCXMLModel {
            name: "probe".into(),
            ..Default::default()
        }
    }

    /// `external_ingress_events` keeps only non-reserved `<transition
    /// event>` triggers — the set a transport switchboard validates its
    /// injection targets against. Reserved W3C platform families
    /// (`error.*` / `done.invoke*` / `done.state*` / `cancel.invoke`),
    /// the wildcard sentinels, and the eventless token are excluded, and
    /// the §scxml-3.12.1 space-separated descriptor list is split per
    /// token so a reserved token never masks an app token sharing its
    /// transition.
    #[test]
    fn external_ingress_events_excludes_reserved_and_wildcard() {
        let mut model = empty_model();
        let state = State {
            transitions: vec![
                Transition {
                    event: "temp_update".into(),
                    ..Default::default()
                },
                Transition {
                    event: "error.execution".into(),
                    ..Default::default()
                },
                Transition {
                    event: "done.invoke.child".into(),
                    ..Default::default()
                },
                Transition {
                    event: "done.state.region".into(),
                    ..Default::default()
                },
                Transition {
                    event: "cancel.invoke".into(),
                    ..Default::default()
                },
                Transition {
                    event: "*".into(),
                    ..Default::default()
                },
                Transition {
                    event: String::new(),
                    ..Default::default()
                },
                // W3C 3.12.1 descriptor list: the app token survives, the
                // reserved token in the same list is dropped.
                Transition {
                    event: "humidity_update done.invoke.x".into(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        model.states.insert("s1".into(), state);

        compute_external_ingress_events(&mut model);

        let got: Vec<&str> = model
            .external_ingress_events
            .iter()
            .map(String::as_str)
            .collect();
        assert_eq!(got, vec!["humidity_update", "temp_update"]);
    }

    /// RFC §W5 D3 split, branch #2: "no initial attribute" stays
    /// classified as `ValidationError::DynamicFeatures` because the
    /// Interpreter CAN resolve via §3.3 default; only the static
    /// generator cannot. Genuine codegen limitation, not a semantic
    /// violation. Triggered at the analyzer level by setting
    /// `model.initial = ""` directly (`parser.rs` auto-defaults to
    /// the first child state, so this path is unreachable through
    /// normal parsing — pinning it here keeps the classification
    /// intact for any future caller that doesn't go through
    /// `parser.rs`).
    #[test]
    fn no_initial_attribute_keeps_dynamic_features() {
        let mut model = empty_model();
        model.initial = String::new();
        // states map non-empty so we don't hit document_rejected first
        model.states.insert("s1".into(), Default::default());

        let err = can_generate_static(&model)
            .expect_err("no initial attribute must surface a precondition failure");
        match err {
            ForgeError::Validation(boxed) => match *boxed {
                ValidationError::DynamicFeatures { name, reason } => {
                    assert_eq!(name, "probe");
                    assert_eq!(
                        reason,
                        "no initial state (runtime default resolution required)"
                    );
                }
                other => panic!(
                    "expected ValidationError::DynamicFeatures for no-initial path, got: {other:?}"
                ),
            },
            other => panic!(
                "expected ValidationError::DynamicFeatures for no-initial path, got: {other:?}"
            ),
        }
    }

    /// RFC §W5 D3 split, branch #3: "initial names undeclared state"
    /// is a hard semantic violation — the Interpreter would also
    /// reject. W5 corrects the prior mis-classification (was
    /// DynamicFeatures, now `ScxmlSemanticError::InitialStateUnknown`
    /// → `validation/invalid-reference`).
    #[test]
    fn undeclared_initial_routes_to_scxml_semantic() {
        let mut model = empty_model();
        model.initial = "nope".into();
        model.states.insert("s1".into(), Default::default());

        let err = can_generate_static(&model).expect_err("undeclared initial must reject");
        match err {
            ForgeError::Scxml(boxed) => match *boxed {
                ScxmlSemanticError::InitialStateUnknown {
                    state_id,
                    scope,
                    available,
                } => {
                    assert_eq!(state_id, "nope");
                    assert!(matches!(scope, InitialStateScope::DocumentRoot));
                    assert_eq!(available, vec!["s1".to_string()]);
                }
                other => panic!("expected ScxmlSemanticError::InitialStateUnknown, got: {other:?}"),
            },
            other => panic!("expected ScxmlSemanticError::InitialStateUnknown, got: {other:?}"),
        }
    }

    /// RFC §W5 D3 split, branch #1: top-level `<script>` rejected
    /// per §scxml-5.8 is a hard semantic violation, not a
    /// codegen limitation. `model.document_rejected = true` is the
    /// signal `parse_global_scripts` sets when it encounters a
    /// failing script.
    #[test]
    fn document_rejected_routes_to_top_level_script_unloaded() {
        let mut model = empty_model();
        model.document_rejected = true;

        let err =
            can_generate_static(&model).expect_err("document_rejected must surface a typed error");
        match err {
            ForgeError::Scxml(boxed) => match *boxed {
                ScxmlSemanticError::TopLevelScriptUnloaded { index, src } => {
                    // Analyzer path doesn't have the failing script's
                    // metadata — both fields stay None per RFC §W5 D2
                    // payload-asymmetry note.
                    assert!(index.is_none());
                    assert!(src.is_none());
                }
                other => {
                    panic!("expected ScxmlSemanticError::TopLevelScriptUnloaded, got: {other:?}")
                }
            },
            other => panic!("expected ScxmlSemanticError::TopLevelScriptUnloaded, got: {other:?}"),
        }
    }

    /// W4 D4 fold success criterion at the analyzer level: when the
    /// analyzer routes a precondition failure to a `ScxmlSemanticError`
    /// variant, the resulting wire `code` MUST match what the
    /// matching forge `ValidationError` variant would emit for the
    /// same conceptual failure. Locks the cross-document-type
    /// concept identity that motivates RFC §W5 D2 fold.
    #[test]
    fn analyzer_emitted_codes_obey_fold_invariant() {
        use crate::forge::diagnostic::ToDiagnostics;
        use crate::forge::model::ForgeKind;

        let mut model = empty_model();
        model.initial = "nope".into();
        model.states.insert("s1".into(), Default::default());
        let scxml_err = can_generate_static(&model).expect_err("undeclared");
        let scxml_diags = scxml_err.to_diagnostics();
        assert_eq!(scxml_diags.len(), 1);
        assert_eq!(scxml_diags[0].code.as_str(), "validation/invalid-reference");

        // Symmetric forge case: a forge document referencing an
        // undeclared symbol emits the same wire code.
        let forge_err: ForgeError = ValidationError::InvalidReference {
            kind: ForgeKind::Statechart,
            what: "transition target".into(),
            name: "nope".into(),
            available: "a, b".into(),
        }
        .into();
        let forge_diags = forge_err.to_diagnostics();
        assert_eq!(forge_diags[0].code.as_str(), scxml_diags[0].code.as_str());
    }

    /// W3C SCXML 6.3: a `<cancel>` nested inside executable content (`<if>`,
    /// and by the same recursion `<foreach>`) must still flag `uses_cancel`,
    /// so the Rust backend keeps the load-bearing `SceString` for
    /// `StatePolicy::ScheduledSendId` instead of eliding it to `ElidedSendId`.
    /// Were the analyzer walk to stop at top-level actions, a nested cancel
    /// would select `ElidedSendId` — whose `matches` always returns false —
    /// turning the cancel into a silent no-op (the delayed event would fire
    /// after it should have been cancelled). Guards the recursive descent in
    /// [`analyze_action`].
    #[test]
    fn nested_cancel_in_if_flags_uses_cancel() {
        let mut model = empty_model();
        let cancel = Action {
            action_type: "cancel".into(),
            sendid: "t1".into(),
            ..Default::default()
        };
        let if_action = Action {
            action_type: "if".into(),
            cond: "true".into(),
            then_actions: vec![cancel],
            ..Default::default()
        };
        let state = State {
            on_entry_blocks: vec![vec![if_action]],
            ..Default::default()
        };
        model.states.insert("s1".into(), state);

        analyze_model_features(&mut model);

        assert!(
            model.uses_cancel,
            "a <cancel> nested in <if> must set uses_cancel; otherwise \
             StatePolicy::ScheduledSendId wrongly elides to ElidedSendId and \
             the cancel silently no-ops"
        );
    }

    /// Inverse axis: a machine that uses the delayed-send scheduler but never
    /// `<cancel>`s keeps `uses_cancel` false, so the Rust backend elides the
    /// per-entry `send_id` (`ScheduledSendId = ElidedSendId`). Pins that
    /// "needs the scheduler" and "needs the cancel key" are distinct signals —
    /// only the latter blocks the no_std footprint elision.
    #[test]
    fn delayed_send_without_cancel_leaves_uses_cancel_false() {
        let mut model = empty_model();
        let send = Action {
            action_type: "send".into(),
            delay: "1s".into(),
            ..Default::default()
        };
        let state = State {
            on_entry_blocks: vec![vec![send]],
            ..Default::default()
        };
        model.states.insert("s1".into(), state);

        analyze_model_features(&mut model);

        assert_eq!(
            model.needs_event_scheduler,
            Some(true),
            "a delayed <send> must still flag the scheduler"
        );
        assert!(
            !model.uses_cancel,
            "no <cancel> means uses_cancel stays false so send_id elides"
        );
    }
}
