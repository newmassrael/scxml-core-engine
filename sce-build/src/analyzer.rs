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
        } else if expr == "0" || (!expr.is_empty() && expr.chars().all(|c| c.is_ascii_digit())) {
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

    // Scan all actions
    let states: Vec<State> = model.states.values().cloned().collect();
    for state in &states {
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
    }

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
            let action_str = format!("{:?}", action);
            if action_str.contains("_event.sendid") {
                model.needs_event_sendid = true;
            }
            if action_str.contains("_event.origin") {
                model.needs_event_origin = true;
            }
            if action_str.contains("_event.invokeid") {
                model.needs_event_invokeid = true;
            }
        }
        "cancel" => {
            model.needs_event_scheduler = Some(true);
        }
        "assign" => {
            model.needs_assign_helper = Some(true);
        }
        "foreach" => {
            model.needs_foreach = Some(true);
        }
        "if" => {
            if !action.cond.is_empty() {
                model.needs_guard_helper = Some(true);
                if action.cond.contains("_event.type") {
                    model.needs_event_type = true;
                }
                if action.cond.contains("_event.data") {
                    model.needs_event_data = true;
                }
                if action.cond.contains("_event.name") {
                    model.needs_event_name = true;
                }
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
        let state = model.states.get(state_id).unwrap().clone();
        let mut updated_transitions = state.transitions.clone();

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
pub fn resolve_internal_transitions(model: &mut SCXMLModel) {
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
                while let Some(s) = model.states.get(&current) {
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
