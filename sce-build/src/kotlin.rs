// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Kotlin-specific analysis helpers — ports kotlin_generator.py.
//
// Contains only Kotlin-specific logic:
//   - Event tree (sealed interface hierarchy for sealed interfaces)
//   - Branch event detection (.Self suffix)
//   - Event tree rendering
//   - Ancestor transition pre-computation (processEvent optimization)
//   - Invoke entries, effective transitions, deep initial entries (serde_json output)
//
// Generic analysis (ancestor chains, parent map, leaf map, initial entry root,
// parallel descendants) lives in analyzer.rs.

use crate::model::*;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::sync::LazyLock;

// Re-export shared analysis from analyzer for backward compatibility
pub use crate::analyzer::{
    compute_ancestor_chains, compute_initial_entry_root, compute_leaf_map,
    compute_parallel_descendants, compute_parent_map,
};

/// §scxml-3.12.1: Delimiter pattern for Kotlin PascalCase conversion (underscore/hyphen).
static RE_KT_DELIMITERS: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"[_\-]").unwrap());

/// §scxml-3.12.1: Build hierarchical event tree from flat dot-separated event names.
///
/// Each node is a JSON object where keys are event name parts and `_leaf` is a boolean
/// indicating whether this node represents a concrete event (not just a prefix).
///
/// Example:
///   {"play", "error.execution"} =>
///   { "play": {"_leaf": true}, "error": {"_leaf": false, "execution": {"_leaf": true}} }
pub fn build_event_tree(events: &BTreeSet<String>) -> serde_json::Value {
    let mut tree = serde_json::Map::new();

    for event_name in events {
        let parts: Vec<&str> = event_name.split('.').collect();
        let mut node = &mut tree;

        for (i, part) in parts.iter().enumerate() {
            if !node.contains_key(*part) {
                let mut new_node = serde_json::Map::new();
                new_node.insert("_leaf".to_string(), serde_json::Value::Bool(false));
                node.insert(part.to_string(), serde_json::Value::Object(new_node));
            }
            if i == parts.len() - 1 {
                if let Some(serde_json::Value::Object(ref mut child)) = node.get_mut(*part) {
                    child.insert("_leaf".to_string(), serde_json::Value::Bool(true));
                }
            }
            // Navigate into the child node for next iteration
            let child = node.get_mut(*part).unwrap();
            node = child.as_object_mut().unwrap();
        }
    }

    serde_json::Value::Object(tree)
}

/// Collect all leaf event names from the event tree (fully qualified dot paths).
///
/// Recursively walks the tree and returns events where `_leaf` is true.
pub fn collect_leaf_events(tree: &serde_json::Value, prefix: &str) -> Vec<String> {
    let mut leaves = Vec::new();
    let obj = match tree.as_object() {
        Some(o) => o,
        None => return leaves,
    };

    for (key, value) in obj {
        if key == "_leaf" {
            continue;
        }
        let full_name = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        if value
            .get("_leaf")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            leaves.push(full_name.clone());
        }
        leaves.extend(collect_leaf_events(value, &full_name));
    }

    leaves
}

/// §scxml-3.12.1: Collect events that are both leaf and branch (need `.Self` suffix).
///
/// Events like "foo" that also have children like "foo.zoo" require `.Self` suffix
/// when used as concrete event references (raise, send), because the event class
/// name alone refers to the sealed interface.
pub fn collect_branch_events(tree: &serde_json::Value, prefix: &str) -> HashSet<String> {
    let mut branch_events = HashSet::new();
    let obj = match tree.as_object() {
        Some(o) => o,
        None => return branch_events,
    };

    for (key, node) in obj {
        if key == "_leaf" {
            continue;
        }
        let full_name = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };

        let children: Vec<&String> = node
            .as_object()
            .map(|o| o.keys().filter(|k| k.as_str() != "_leaf").collect())
            .unwrap_or_default();

        let is_leaf = node.get("_leaf").and_then(|v| v.as_bool()).unwrap_or(false);

        if is_leaf && !children.is_empty() {
            branch_events.insert(full_name.clone());
        }
        branch_events.extend(collect_branch_events(node, &full_name));
    }

    branch_events
}

/// Render event tree as Kotlin sealed interface hierarchy code.
///
/// Recursive Rust function avoids Jinja2 macro recursion issues.
/// Produces nested `sealed interface` and `data object` declarations.
pub fn render_event_tree(tree: &serde_json::Value, parent_type: &str, indent: &str) -> String {
    let obj = match tree.as_object() {
        Some(o) => o,
        None => return String::new(),
    };

    let mut lines: Vec<String> = Vec::new();

    let mut sorted_keys: Vec<&String> = obj.keys().filter(|k| k.as_str() != "_leaf").collect();
    sorted_keys.sort();

    for key in sorted_keys {
        let node = &obj[key];

        // PascalCase: split on underscore/hyphen within each segment
        let class_name = if key.is_empty() {
            "Empty".to_string()
        } else {
            RE_KT_DELIMITERS
                .split(key)
                .map(|p| {
                    if p.is_empty() {
                        String::new()
                    } else {
                        crate::filters::capitalize_first(p)
                    }
                })
                .collect::<String>()
        };

        // Collect children (non-_leaf keys)
        let children: Vec<&String> = node
            .as_object()
            .map(|o| o.keys().filter(|k| k.as_str() != "_leaf").collect())
            .unwrap_or_default();

        if !children.is_empty() {
            // Branch node: sealed interface with nested children
            lines.push(format!(
                "{indent}sealed interface {class_name} : {parent_type} {{"
            ));
            let is_leaf = node.get("_leaf").and_then(|v| v.as_bool()).unwrap_or(false);
            if is_leaf {
                // Both a concrete event and a parent for prefix matching
                lines.push(format!("{indent}    data object Self : {class_name}"));
            }
            // Recurse into children
            let child_indent = format!("{indent}    ");
            let child_lines = render_event_tree(node, &class_name, &child_indent);
            if !child_lines.is_empty() {
                lines.push(child_lines);
            }
            lines.push(format!("{indent}}}"));
        } else {
            // Leaf node: data object
            lines.push(format!("{indent}data object {class_name} : {parent_type}"));
        }
    }

    lines.join("\n")
}

/// Convert event name to Kotlin class reference, appending `.Self` for branch events.
///
/// Events that are both concrete and branch nodes (e.g., "foo" with "foo.zoo")
/// need `.Self` suffix to reference the data object, not the sealed interface.
pub fn to_event_ref(event_name: &str, branch_events: &HashSet<String>) -> String {
    let class_name = crate::filters::to_event_class_name(event_name.to_string());
    if branch_events.contains(event_name) {
        format!("{class_name}.Self")
    } else {
        class_name
    }
}

/// §scxml-3.6: Compute deep initial entry order for space-separated initials.
///
/// When a compound state has `initial="target1 target2"` (deep descendant targets),
/// the codegen must enter ancestors along the path without triggering their
/// default initial child entry, then enter the actual targets with full onEntry.
pub fn compute_deep_initial_entries(
    model: &SCXMLModel,
) -> BTreeMap<String, Vec<serde_json::Value>> {
    let mut deep_initial_entries = BTreeMap::new();

    for (state_id, state) in &model.states {
        if state.initial_children.len() > 1 {
            let target_set: HashSet<&String> = state.initial_children.iter().collect();
            let mut all_path_states: HashSet<String> = HashSet::new();

            for target_id in &state.initial_children {
                let mut current = target_id.clone();
                let mut visited: HashSet<String> = HashSet::new();

                while current != *state_id && model.states.contains_key(&current) {
                    if visited.contains(&current) {
                        break;
                    }
                    visited.insert(current.clone());
                    all_path_states.insert(current.clone());
                    current = model.states[&current].parent.clone().unwrap_or_default();
                }
            }

            // Sort by document_order
            let mut path_vec: Vec<String> = all_path_states.into_iter().collect();
            path_vec.sort_by_key(|s| model.states[s].document_order);

            let entry_order: Vec<serde_json::Value> = path_vec
                .iter()
                .map(|sid| {
                    serde_json::json!({
                        "id": sid,
                        "is_target": target_set.contains(sid),
                    })
                })
                .collect();

            deep_initial_entries.insert(state_id.clone(), entry_order);
        }
    }

    deep_initial_entries
}

/// §scxml-6.4: Compute invoke entries for each state with language-agnostic data.
///
/// Returns invoke info per state. Each entry contains child_name (raw SCXML name);
/// language generators should post-process to add language-specific class/type names.
pub fn compute_invoke_entries(model: &SCXMLModel) -> BTreeMap<String, Vec<serde_json::Value>> {
    let mut invoke_entries: BTreeMap<String, Vec<serde_json::Value>> = BTreeMap::new();

    for (state_id, state) in &model.states {
        if state.has_scxml_invoke() {
            let mut entries = Vec::new();

            for si in state.iter_scxml_invokes() {
                let invoke_id = &si.invoke_id;
                let specific_done = if !invoke_id.is_empty() {
                    format!("done.invoke.{invoke_id}")
                } else {
                    String::new()
                };

                let done_event =
                    if !specific_done.is_empty() && model.events.contains(&specific_done) {
                        specific_done.clone()
                    } else {
                        "done.invoke".to_string()
                    };

                let has_done_event =
                    model.events.contains(&done_event) || model.events.contains("done.invoke");

                // Serialize params via serde
                let params_json = serde_json::to_value(&si.params).unwrap_or_default();

                // §scxml-6.4: child_class is PascalCase for Kotlin type-safe instantiation
                let child_class = if !si.child_name.is_empty() {
                    crate::filters::to_pascal_case(si.child_name.clone())
                } else {
                    String::new()
                };

                entries.push(serde_json::json!({
                    "invoke_id": invoke_id,
                    "child_name": si.child_name,
                    "child_class": child_class,
                    "autoforward": si.autoforward,
                    "done_event": done_event,
                    "has_done_event": has_done_event,
                    "params": params_json,
                    "namelist": si.namelist,
                    "idlocation": si.idlocation,
                    "finalize_content": si.finalize_content,
                    "state_id": state_id,
                    "child_needs_script_engine": si.child_needs_script_engine,
                }));
            }

            invoke_entries.insert(state_id.clone(), entries);
        }
    }

    // §scxml-6.4: Hybrid invoke support (srcexpr/contentexpr)
    for (state_id, state) in &model.states {
        if state.has_hybrid_invoke() {
            let entries = invoke_entries.entry(state_id.clone()).or_default();

            for hi in state.iter_hybrid_invokes() {
                let invoke_id = &hi.invoke_id;
                let specific_done = if !invoke_id.is_empty() {
                    format!("done.invoke.{invoke_id}")
                } else {
                    String::new()
                };

                let done_event =
                    if !specific_done.is_empty() && model.events.contains(&specific_done) {
                        specific_done.clone()
                    } else {
                        "done.invoke".to_string()
                    };

                let has_done_event =
                    model.events.contains(&done_event) || model.events.contains("done.invoke");

                let params_json = serde_json::to_value(&hi.params).unwrap_or_default();

                entries.push(serde_json::json!({
                    "invoke_id": invoke_id,
                    "child_name": "",
                    "autoforward": hi.autoforward,
                    "done_event": done_event,
                    "has_done_event": has_done_event,
                    "params": params_json,
                    "namelist": "",
                    "idlocation": hi.idlocation,
                    "finalize_content": "",
                    "state_id": state_id,
                    "child_needs_script_engine": false,
                    "is_hybrid": true,
                    "srcexpr": hi.srcexpr.as_str(),
                    "contentexpr": hi.contentexpr.as_str(),
                }));
            }
        }
    }

    // §scxml-6.4.1: `<invoke>` naming a processor this platform does not
    // implement. It carries no child class, no done event and no params —
    // the deferred closure raises `error.execution` and returns. Kotlin's
    // runtime-closure invoke shape means the "execute" step is the closure
    // body, so no separate execute-site arm is needed.
    for (state_id, state) in &model.states {
        if state.has_unsupported_invoke() {
            let entries = invoke_entries.entry(state_id.clone()).or_default();
            for ui in state.invokes.iter().filter_map(|i| match i {
                crate::model::Invoke::Unsupported(info) => Some(info),
                _ => None,
            }) {
                entries.push(serde_json::json!({
                    "invoke_id": ui.base.invoke_id.as_str(),
                    "state_id": state_id,
                    "is_unsupported": true,
                    "invoke_type": ui.invoke_type.as_str(),
                }));
            }
        }
    }

    invoke_entries
}

/// §scxml-3.13: Pre-compute ancestor transition maps for processEvent routing.
///
/// Eliminates inline ancestor scanning in process_event.kt.jinja2.
/// Returns two maps: ancestors with event-based transitions, and ancestors with
/// eventless (null) transitions.
pub fn compute_ancestors_with_transitions(
    model: &SCXMLModel,
    ancestor_chains: &BTreeMap<String, Vec<String>>,
) -> (BTreeMap<String, Vec<String>>, BTreeMap<String, Vec<String>>) {
    let mut event_map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut null_map: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for (state_id, chain) in ancestor_chains {
        let mut event_ancs = Vec::new();
        let mut null_ancs = Vec::new();

        for anc_id in chain {
            if let Some(anc_state) = model.states.get(anc_id) {
                let has_event = anc_state.transitions.iter().any(|t| !t.event.is_empty());
                let has_null = anc_state
                    .transitions
                    .iter()
                    .any(|t| t.event.is_empty() && !t.target.is_empty());
                if has_event {
                    event_ancs.push(anc_id.clone());
                }
                if has_null {
                    null_ancs.push(anc_id.clone());
                }
            }
        }

        event_map.insert(state_id.clone(), event_ancs);
        null_map.insert(state_id.clone(), null_ancs);
    }

    (event_map, null_map)
}

/// Whether `processEvent`'s `when (state)` still needs an `else` arm.
///
/// The template emits one branch per state that has event transitions of its
/// own or inherits an ancestor's, and skips `<parallel>` states entirely. The
/// `else` covers exactly what that leaves out. When it leaves nothing out the
/// `when` is exhaustive over the sealed state hierarchy, and Kotlin rejects a
/// redundant `else` under `-Werror` — which is how a document with no `<final>`
/// and a transition on every state failed to compile.
///
/// Takes the same `(model, ancestor_chains)` pair and the event map that
/// [`compute_ancestors_with_transitions`] returns, so the two answers cannot
/// drift: the predicate below is the negation of the template's branch
/// conditions, read off the same inputs the template reads.
pub fn process_event_needs_else(
    model: &SCXMLModel,
    ancestors_with_event_transitions: &BTreeMap<String, Vec<String>>,
) -> bool {
    model.states.iter().any(|(state_id, state)| {
        if state.is_parallel {
            // The template's `{% if not state.is_parallel %}` skips it, so the
            // `else` is the only arm that answers for this state.
            return true;
        }
        let has_own_event_transition = state.transitions.iter().any(|t| !t.event.is_empty());
        let inherits_one = ancestors_with_event_transitions
            .get(state_id)
            .is_some_and(|ancestors| !ancestors.is_empty());
        !has_own_event_transition && !inherits_one
    })
}

/// §scxml-3.13: Compute effective transitions (self + ancestors).
///
/// For each state, collects its own transitions followed by all ancestor transitions.
/// Serialized as JSON for template rendering.
pub fn compute_effective_transitions(
    model: &SCXMLModel,
    ancestor_chains: &BTreeMap<String, Vec<String>>,
) -> BTreeMap<String, serde_json::Value> {
    let mut effective_transitions = BTreeMap::new();

    for (state_id, state) in &model.states {
        let mut transitions: Vec<serde_json::Value> = state
            .transitions
            .iter()
            .map(|t| serde_json::to_value(t).unwrap_or_default())
            .collect();

        if let Some(chain) = ancestor_chains.get(state_id) {
            for anc_id in chain {
                if let Some(anc_state) = model.states.get(anc_id) {
                    for t in &anc_state.transitions {
                        transitions.push(serde_json::to_value(t).unwrap_or_default());
                    }
                }
            }
        }

        effective_transitions.insert(state_id.clone(), serde_json::Value::Array(transitions));
    }

    effective_transitions
}

/// Whether `executeTransitionActions`' `when (source)` still needs an `else` arm.
///
/// The twin of [`process_event_needs_else`], for the other `when` over the
/// sealed state hierarchy in the same file. That template emits a branch for
/// every state with at least one effective transition carrying actions, so the
/// `else` covers the states with none. A document where every state has one
/// leaves the `when` exhaustive, and Kotlin rejects a redundant `else` under
/// `-Werror`.
///
/// Reads `effective_transitions` — the same map the template iterates — so the
/// predicate cannot drift from the branch condition it is the negation of.
pub fn transition_actions_needs_else(
    effective_transitions: &BTreeMap<String, serde_json::Value>,
) -> bool {
    effective_transitions.values().any(|transitions| {
        !transitions
            .as_array()
            .is_some_and(|list| list.iter().any(transition_has_actions))
    })
}

/// Whether one serialized transition carries executable content.
///
/// Mirrors the template's `{% if trans.actions %}`, which is Jinja truthiness:
/// a missing key and an empty list both read as false.
fn transition_has_actions(transition: &serde_json::Value) -> bool {
    transition
        .get("actions")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|actions| !actions.is_empty())
}

/// §scxml-3.7.1: Generate Kotlin expression for parallel state completion check.
///
/// For a parallel state, checks that every child region has at least one active
/// final state. Returns a Kotlin boolean expression like:
///   `(activeStateIds.contains("f1")) && (activeStateIds.contains("f2"))`
pub fn to_parallel_complete_check(
    parallel_id: &str,
    parallel_regions: &BTreeMap<String, Vec<String>>,
    states: &BTreeMap<String, State>,
) -> String {
    let regions = match parallel_regions.get(parallel_id) {
        Some(r) => r,
        None => return "false".to_string(),
    };
    if regions.is_empty() {
        return "false".to_string();
    }
    let mut checks = Vec::new();
    for region_id in regions {
        let finals: Vec<&String> = states
            .iter()
            .filter(|(_, state)| {
                state.parent.as_deref() == Some(region_id.as_str()) && state.is_final
            })
            .map(|(id, _)| id)
            .collect();
        if finals.is_empty() {
            return "false".to_string();
        }
        let cond = finals
            .iter()
            .map(|f| format!("activeStateIds.contains(\"{f}\")"))
            .collect::<Vec<_>>()
            .join(" || ");
        checks.push(format!("({cond})"));
    }
    checks.join(" && ")
}
