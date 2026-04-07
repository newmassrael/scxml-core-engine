// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
//
// Kotlin-specific analysis helpers — ports kotlin_generator.py and base.py
// shared analysis methods for Kotlin code generation.
//
// These functions compute derived data structures from the parsed SCXML model
// that the Kotlin template rendering pipeline needs:
//   - Event tree (sealed interface hierarchy)
//   - Ancestor chains, parent map, leaf map
//   - Parallel descendants, deep initial entries
//   - Invoke entries, effective transitions

use crate::model::*;
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet, HashSet};

/// W3C SCXML 3.12.1: Build hierarchical event tree from flat dot-separated event names.
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
        if value.get("_leaf").and_then(|v| v.as_bool()).unwrap_or(false) {
            leaves.push(full_name.clone());
        }
        leaves.extend(collect_leaf_events(value, &full_name));
    }

    leaves
}

/// W3C SCXML 3.12.1: Collect events that are both leaf and branch (need `.Self` suffix).
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

        let is_leaf = node
            .get("_leaf")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

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

    let re = Regex::new(r"[_\-]").unwrap();
    let mut lines: Vec<String> = Vec::new();

    let mut sorted_keys: Vec<&String> = obj.keys().filter(|k| k.as_str() != "_leaf").collect();
    sorted_keys.sort();

    for key in sorted_keys {
        let node = &obj[key];

        // PascalCase: split on underscore/hyphen within each segment
        let class_name = if key.is_empty() {
            "Empty".to_string()
        } else {
            let sub_parts: Vec<&str> = re.split(key).collect();
            sub_parts
                .iter()
                .map(|p| {
                    if p.is_empty() {
                        String::new()
                    } else {
                        let mut chars = p.chars();
                        let first = chars.next().unwrap().to_uppercase().to_string();
                        first + chars.as_str()
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
            let is_leaf = node
                .get("_leaf")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if is_leaf {
                // Both a concrete event and a parent for prefix matching
                lines.push(format!(
                    "{indent}    data object Self : {class_name}"
                ));
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
            lines.push(format!(
                "{indent}data object {class_name} : {parent_type}"
            ));
        }
    }

    lines.join("\n")
}

/// W3C SCXML 5.2: Return Kotlin default value for a datamodel variable.
///
/// Maps SCXML variable types to Kotlin literal defaults:
///   int    -> expr or "0"
///   string -> expr (if quoted) or "\"\""
///   bool   -> expr (if "true"/"false") or "false"
///   other  -> "null"
pub fn to_kotlin_default(var: &serde_json::Value) -> String {
    let var_type = var
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("runtime");
    let expr = var
        .get("expr")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    match var_type {
        "int" => {
            if !expr.is_empty() {
                expr.to_string()
            } else {
                "0".to_string()
            }
        }
        "string" => {
            if expr.starts_with('"') && expr.ends_with('"') {
                expr.to_string()
            } else {
                "\"\"".to_string()
            }
        }
        "bool" => {
            if expr == "true" || expr == "false" {
                expr.to_string()
            } else {
                "false".to_string()
            }
        }
        _ => "null".to_string(),
    }
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

/// W3C SCXML 3.13: Compute ancestor chains for transition routing.
///
/// Each state maps to an ordered list of ancestor state IDs (parent first).
/// Used by processEvent to check ancestor transitions when self returns Ignored.
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

/// W3C SCXML 3.3: Build parent map for state hierarchy.
///
/// Maps state_id to parent_id for states with a parent that exists in the model.
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

/// W3C SCXML 3.3/3.4: Build leaf map for compound/parallel state resolution.
///
/// Maps non-leaf states to their initial leaf states by following `initial` attributes.
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

/// W3C SCXML 3.2/3.4: Compute initial entry root.
///
/// Walks up from the resolved leaf initial state to find the top-level
/// ancestor. Used by enterInitialConfiguration() in generated code.
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

/// Collect all descendant state IDs of a parent state (recursive helper).
fn collect_descendants(model: &SCXMLModel, parent_id: &str, descendants: &mut Vec<String>) {
    for (state_id, state) in &model.states {
        if state.parent.as_deref() == Some(parent_id) {
            descendants.push(state_id.clone());
            collect_descendants(model, state_id, descendants);
        }
    }
}

/// W3C SCXML 3.4: Compute descendants for each parallel state.
///
/// Used by onExit to collect and exit active descendants.
pub fn compute_parallel_descendants(model: &SCXMLModel) -> BTreeMap<String, Vec<String>> {
    let mut parallel_descendants = BTreeMap::new();

    for parallel_id in model.parallel_regions.keys() {
        let mut descendants = Vec::new();
        collect_descendants(model, parallel_id, &mut descendants);
        parallel_descendants.insert(parallel_id.clone(), descendants);
    }

    parallel_descendants
}

/// W3C SCXML 3.6: Compute deep initial entry order for space-separated initials.
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
                    current = model.states[&current]
                        .parent
                        .clone()
                        .unwrap_or_default();
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

/// W3C SCXML 6.4: Compute invoke entries for each state with language-agnostic data.
///
/// Returns invoke info per state. Each entry contains child_name (raw SCXML name);
/// language generators should post-process to add language-specific class/type names.
pub fn compute_invoke_entries(model: &SCXMLModel) -> BTreeMap<String, Vec<serde_json::Value>> {
    let mut invoke_entries: BTreeMap<String, Vec<serde_json::Value>> = BTreeMap::new();

    for (state_id, state) in &model.states {
        if !state.static_invokes.is_empty() {
            let mut entries = Vec::new();

            for si in &state.static_invokes {
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

                let has_done_event = model.events.contains(&done_event)
                    || model.events.contains("done.invoke");

                // Serialize params via serde
                let params_json = serde_json::to_value(&si.params).unwrap_or_default();

                // W3C SCXML 6.4: child_class is PascalCase for Kotlin type-safe instantiation
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

    // W3C SCXML 6.4: Hybrid invoke support (srcexpr/contentexpr)
    for (state_id, state) in &model.states {
        if !state.hybrid_invokes.is_empty() {
            let entries = invoke_entries.entry(state_id.clone()).or_default();

            for hi in &state.hybrid_invokes {
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

                let has_done_event = model.events.contains(&done_event)
                    || model.events.contains("done.invoke");

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
                    "srcexpr": hi.srcexpr.as_deref().unwrap_or(""),
                    "contentexpr": hi.contentexpr.as_deref().unwrap_or(""),
                }));
            }
        }
    }

    invoke_entries
}

/// W3C SCXML 3.13: Compute effective transitions (self + ancestors).
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
                        transitions
                            .push(serde_json::to_value(t).unwrap_or_default());
                    }
                }
            }
        }

        effective_transitions.insert(
            state_id.clone(),
            serde_json::Value::Array(transitions),
        );
    }

    effective_transitions
}

/// W3C SCXML 3.7.1: Generate Kotlin expression for parallel state completion check.
///
/// For a parallel state, checks that every child region has at least one active
/// final state. Returns a Kotlin boolean expression like:
///   `(activeStateIds.contains("f1")) && (activeStateIds.contains("f2"))`
pub fn to_parallel_complete_check(
    parallel_id: &str,
    parallel_regions: &BTreeMap<String, Vec<String>>,
    states: &BTreeMap<String, State>,
    _machine_name: &str,
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
