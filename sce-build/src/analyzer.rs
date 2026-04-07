// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
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
    build_prefix_matching(model);

    // Named Context: set needs_nonstatic_method
    model.needs_nonstatic_method = model.needs_script_engine
        || !model.static_invokes.is_empty()
        || model.has_parent_communication
        || model.has_parallel_states
        || model.uses_in_predicate
        || !model.context_objects.is_empty();

    // Rust-specific analysis
    resolve_internal_transitions(model);
    model.scxml_base_path = compute_scxml_base_path(scxml_path);
}

/// W3C SCXML 5.3: Classify datamodel variables by type.
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
            model.needs_script_engine = true;
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
        for block in state.on_entry_blocks.iter().chain(state.on_exit_blocks.iter()) {
            for action in block {
                analyze_action(action, model);
            }
        }
        // W3C SCXML 3.3: Analyze initial transition actions
        for action in &state.initial_transition_actions {
            analyze_action(action, model);
        }
        // W3C SCXML 3.11: Analyze history default actions
        for action in &state.initial_history_default_actions {
            analyze_action(action, model);
        }
    }
    model.states = states;

    // W3C SCXML 5.2: Analyze document-level global scripts
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

    // Donedata with params or contentexpr requires script engine
    let donedata_needs_script = model.states.values().any(|state| {
        state.is_final
            && state.donedata.as_ref().map_or(false, |dd| {
                !dd.params.is_empty() || !dd.contentexpr.is_empty()
            })
    });
    if donedata_needs_script {
        model.needs_script_engine = true;
        apply_script_engine_implications(model);
    }

    // Child invoke script engine propagation
    if !model.needs_script_engine {
        let needs = model.states.values().any(|s| {
            s.static_invokes
                .iter()
                .any(|si| si.child_needs_script_engine)
        });
        if needs {
            model.needs_script_engine = true;
            apply_script_engine_implications(model);
        }
    }
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
        &action.expr, &action.target, &action.targetexpr,
        &action.cond, &action.content, &action.contentexpr,
        &action.eventexpr, &action.namelist, &action.typeexpr,
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
    if field.contains("_event.sendid") { model.needs_event_sendid = true; }
    if field.contains("_event.origintype") { model.needs_event_origintype = true; }
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
    if field.contains("_event.invokeid") { model.needs_event_invokeid = true; }
    if field.contains("_event.data") { model.needs_event_data = true; }
    if field.contains("_event.name") { model.needs_event_name = true; }
    if field.contains("_event.type") { model.needs_event_type = true; }
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
            // W3C SCXML C.2: BasicHTTP send detection
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

    if !model.static_invokes.is_empty() {
        model.events.insert("done.invoke".to_string());
        model.events.insert("cancel.invoke".to_string());
        model.events.insert("error.execution".to_string());
    }
}

/// W3C SCXML 3.12.1: Build prefix matching for event transitions.
fn build_prefix_matching(model: &mut SCXMLModel) {
    let all_events: Vec<String> = model.events.iter().cloned().collect();
    model.needs_event_matching_helper = false;

    let state_ids: Vec<String> = model.states.keys().cloned().collect();
    for state_id in &state_ids {
        // Take transitions out of the state to avoid cloning the entire state
        let mut updated_transitions = std::mem::take(
            &mut model.states.get_mut(state_id).unwrap().transitions,
        );

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

/// W3C SCXML 3.13: Resolve internal transition types.
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
    let parent = Path::new(scxml_path)
        .parent()
        .unwrap_or(Path::new("."));
    if let Ok(cwd) = std::env::current_dir() {
        if let Ok(rel) = parent.strip_prefix(&cwd) {
            return rel.to_string_lossy().to_string();
        }
    }
    parent.to_string_lossy().to_string()
}

/// Check if model can be statically code-generated.
pub fn can_generate_static(model: &SCXMLModel) -> bool {
    if model.document_rejected {
        return false;
    }
    if model.initial.is_empty() {
        return false;
    }
    let initial_states: Vec<&str> = model.initial.split_whitespace().collect();
    if initial_states.len() > 1 {
        initial_states
            .iter()
            .all(|s| model.states.contains_key(*s))
    } else {
        model.states.contains_key(&model.initial)
    }
}
