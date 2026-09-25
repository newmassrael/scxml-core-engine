// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Kotlin-specific analysis helpers — ports kotlin_generator.py.
//
// Contains only Kotlin-specific logic:
//   - Event tree (sealed interface hierarchy for sealed interfaces)
//   - Branch event detection (.Self suffix)
//   - Event tree rendering
//   - Invoke entries (serde_json output)
//
// Nothing here resolves a target to a leaf, routes a transition through its
// ancestors or pre-computes an entry order: the Kotlin runtime runs Appendix
// D's microstep (com.sce.runtime.Microstep) over the document's structure as
// written, which the templates take from the model directly.

use crate::model::*;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::sync::LazyLock;

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

                // §scxml-6.4: the child a hybrid `<invoke>` runs is the stub
                // codegen fixed at build time, the same one the other five AOT
                // backends spawn (docs/SCE_ACCEPTED_SUBSET.md §2.13). This used
                // to be empty because Kotlin resolved the evaluated string into
                // a document through `ScxmlRuntimeInterpreter` instead — an
                // interpreter fallback inside an AOT backend, and one that
                // could not ship: that class lives in this repository's Kotlin
                // TEST module, so every generated file carrying a hybrid invoke
                // imported a symbol a consumer's runtime does not have.
                let child_class = if hi.common.child_name.is_empty() {
                    String::new()
                } else {
                    crate::filters::to_pascal_case(hi.common.child_name.clone())
                };

                entries.push(serde_json::json!({
                    "invoke_id": invoke_id,
                    "child_name": hi.common.child_name,
                    "child_class": child_class,
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
                    // §scxml-6.4 + SCE_ACCEPTED_SUBSET.md §2.13: the documents
                    // the value may choose between. Empty when none were
                    // declared, which is the build-time stub as before.
                    "candidates": hi.candidates,
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
                    // §scxml-6.4.1: whether the host declared this `type`, so
                    // the entry template starts the invocation instead of
                    // refusing it. Carried per entry rather than derived from
                    // the model's declaration list for the reason
                    // `Action::send_type_host_served` is: the templates read
                    // the DECISION, not the list it was made from, so one
                    // backend cannot re-derive it differently from another.
                    "host_served": ui.host_served,
                    "invoke_type": ui.invoke_type.as_str(),
                    // Only meaningful for a host-served entry: what the
                    // document said to invoke, and with what.
                    "src": ui.src.as_str(),
                    "params": serde_json::to_value(&ui.base.params).unwrap_or_default(),
                }));
            }
        }
    }

    invoke_entries
}
