// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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

/// The bytes of code one JVM method holds: `code_length` is less than 65,536
/// (JVM Specification, section 4.7.3), and kotlinc refuses a method past it
/// with `MethodTooLargeException`.
pub const JVM_METHOD_CODE_BYTES: usize = 65_535;

/// The most items of a fixed shape one generated Kotlin method holds.
///
/// An item is one arm of a dispatch
/// (`tools/codegen/templates/kotlin/dispatch.kt.jinja2`): an `is` test or a
/// constant and one call or one constant; one entry of a table built at class
/// initialisation, weighed as
/// one plus the elements of the list it holds; or one statement of a
/// per-variable sequence (a variable's initialisation, its save, its restore).
/// What a document has more of than this is split into parts of at most this
/// many, each a method of its own, and the method that asks the parts makes at
/// most this many calls: every item names a class-file constant no other item
/// of the same method names, and a class holds at most 65,535 of them (JVM
/// Specification, section 4.1), so 65,535 / 256 = 256 parts at most.
///
/// Measured 2026-11-05, before the split: a large `sce-static` statechart's
/// machine did not compile, its `executeTransitionContent` past
/// [`JVM_METHOD_CODE_BYTES`].
pub const METHOD_ITEMS: usize = 256;

/// The bytes of JVM code one item compiles to at most.
///
/// An arm of a `when` over strings is its `lookupswitch` entry (8 bytes), the
/// subject loaded, the constant loaded and compared (`aload`, `ldc_w`,
/// `invokevirtual`, `ifeq`: 10), a call of the machine's own with at most
/// three arguments (`aload_0`, three loads of at most 2 bytes, `invokevirtual`:
/// 10) and the jump out (`goto_w`: 5) — 33 bytes. A table entry is the key
/// loaded, the value built, `invokeinterface put` and `pop`, under 40 bytes for
/// its own weight of one, each element of the list it holds weighed one item
/// more. The widest is a variable's initialisation: the engine's evaluation,
/// its write and the error its failure raises, about 60 bytes. 128 leaves room
/// above every one of them.
pub const ITEM_CODE_BYTES: usize = 128;

// One method's items take at most half of the limit, so the method's own
// prologue and the code kotlinc adds around a `when` fit in the rest.
const _: () = assert!(METHOD_ITEMS * ITEM_CODE_BYTES * 2 <= JVM_METHOD_CODE_BYTES + 1);

/// Items of the given weights, in order, packed into consecutive parts whose
/// weights sum to at most `budget`: each part is the positions of its items.
///
/// An item heavier than `budget` alone is a part of its own: the weight of one
/// item is what its own element writes, which no split of the dispatch makes
/// smaller. No item is left out, and the parts keep the items' order, so the
/// methods built from them run what one method would have run, in the same
/// order.
pub fn pack_parts(weights: &[usize], budget: usize) -> Vec<Vec<usize>> {
    let mut parts: Vec<Vec<usize>> = Vec::new();
    let mut current: Vec<usize> = Vec::new();
    let mut current_weight = 0usize;
    for (position, &weight) in weights.iter().enumerate() {
        if !current.is_empty() && current_weight + weight > budget {
            parts.push(std::mem::take(&mut current));
            current_weight = 0;
        }
        current.push(position);
        current_weight += weight;
    }
    if !current.is_empty() {
        parts.push(current);
    }
    parts
}

/// Where the Kotlin machine declares the constant of each transition the
/// microstep reads (`EnabledTransition`): in holder objects
/// (`TransitionsPart<n>`), each initialised by its own class initialiser.
///
/// They used to be in the machine's companion object, whose one class
/// initialiser grew with every transition of the document. A transition
/// weighs one item and one more for each target it names, which is what its
/// constant builds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionParts {
    /// Each holder: the state that owns each transition it declares, and the
    /// transition's position among that state's transitions.
    pub holders: Vec<Vec<(String, usize)>>,
    /// The holder of each transition, by its owning state, in the order of the
    /// state's transitions.
    pub holder_of: BTreeMap<String, Vec<usize>>,
}

/// [`TransitionParts`] for `model`: its transitions in the order of their
/// states' ids, each state's in document order, packed into holders of at most
/// [`METHOD_ITEMS`].
pub fn transition_parts(model: &SCXMLModel) -> TransitionParts {
    let mut keys: Vec<(String, usize)> = Vec::new();
    let mut weights: Vec<usize> = Vec::new();
    for (state_id, state) in &model.states {
        for (position, transition) in state.transitions.iter().enumerate() {
            keys.push((state_id.clone(), position));
            weights.push(1 + transition.targets.len());
        }
    }
    let mut holders: Vec<Vec<(String, usize)>> = Vec::new();
    let mut holder_of: BTreeMap<String, Vec<usize>> = model
        .states
        .iter()
        .map(|(id, state)| (id.clone(), vec![0; state.transitions.len()]))
        .collect();
    for (holder, part) in pack_parts(&weights, METHOD_ITEMS).into_iter().enumerate() {
        let mut members = Vec::with_capacity(part.len());
        for position in part {
            let (state_id, index) = &keys[position];
            if let Some(slots) = holder_of.get_mut(state_id) {
                slots[*index] = holder;
            }
            members.push((state_id.clone(), *index));
        }
        holders.push(members);
    }
    TransitionParts { holders, holder_of }
}

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

/// The class one event-tree node declares: its token, spelled by
/// [`crate::filters::to_kotlin_event_segment`].
pub fn event_tree_class_name(key: &str) -> String {
    if key.is_empty() {
        "Empty".to_string()
    } else {
        crate::filters::to_kotlin_event_segment(key)
    }
}

/// The member a node declares inside its own interface when it is both an
/// event and the prefix of others (`foo` beside `foo.zoo`, §scxml-3.12.1).
pub const EVENT_TREE_SELF_MEMBER: &str = "Self";

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

        let class_name = event_tree_class_name(key);

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
                lines.push(format!(
                    "{indent}    data object {EVENT_TREE_SELF_MEMBER} : {class_name}"
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
        format!("{class_name}.{EVENT_TREE_SELF_MEMBER}")
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
                let child_document = if si.inline_child.is_some() {
                    String::new()
                } else {
                    si.child_name.clone()
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
                    // `datamodel="sce-static"`: the statement that writes the id a
                    // start generates to the variable `idlocation` names, in
                    // place of the script engine's write (docs/adr/0045).
                    "native_idlocation": si.common.native_idlocation,
                    "finalize_content": si.finalize_content,
                    "state_id": state_id,
                    "child_needs_script_engine": si.child_needs_script_engine,
                    // The call that answers the host the child is built with,
                    // empty for a child that declares no act.
                    "child_host_call": si.child_host_call,
                    // Whether that child states a duty, so it is also handed
                    // the caller's statement that a simulated host is meant.
                    "child_declares_host_duties": si.child_declares_host_duties,
                    // The stem of the child's document, which the factory of
                    // children's Mesh routers is asked with (SCE_MESH.md, Child
                    // Sessions in Mesh): empty for a child written inline.
                    "child_document": child_document,
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
                    "namelist": hi.namelist,
                    "idlocation": hi.idlocation,
                    "native_idlocation": hi.common.native_idlocation,
                    "finalize_content": "",
                    "state_id": state_id,
                    "child_needs_script_engine": false,
                    "is_hybrid": true,
                    "srcexpr": hi.srcexpr.as_str(),
                    "contentexpr": hi.contentexpr.as_str(),
                    // `datamodel="sce-static"`: the `srcexpr` lowered to the
                    // string it computes, read from the machine's fields when
                    // the invocation starts, in place of a script engine's
                    // evaluation of the attribute (`srcexpr` is empty then).
                    "native_src": hi.native_src.as_str(),
                    "native_src_fails": hi.native_src_fails,
                    // §scxml-6.4 + SCE_ACCEPTED_SUBSET.md §2.13: the documents
                    // the value may choose between. Empty when none were
                    // declared, which is the build-time stub as before.
                    "candidates": hi.candidates,
                }));
            }
        }
    }

    // §scxml-6.4.1: `<invoke>` naming a processor this platform does not
    // implement. Unless the host declared the type, it carries no child
    // class and no done event — the deferred closure raises
    // `error.execution` and returns. Kotlin's
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
                    // document said to invoke, and with what. The `*expr`
                    // fields and `namelist` are evaluated when the invocation
                    // starts (§scxml-6.4.1); a list of fields spelled here
                    // rather than the whole struct serialized, so each one
                    // the template reads was chosen to reach it.
                    "src": ui.src.as_str(),
                    "srcexpr": ui.srcexpr.as_str(),
                    // The `srcexpr` of a `sce-static` document, lowered to the
                    // string it computes, which the template reads in its place.
                    "native_src": ui.native_src.as_str(),
                    "native_src_fails": ui.native_src_fails,
                    "namelist": ui.namelist.as_str(),
                    "content": ui.content.as_str(),
                    "contentexpr": ui.contentexpr.as_str(),
                    // The `<content expr>` of a `sce-static` document, lowered to
                    // the string it computes, which the template reads in its place.
                    "native_content": ui.native_content.as_str(),
                    "native_content_fails": ui.native_content_fails,
                    "idlocation": ui.base.idlocation.as_str(),
                    "params": serde_json::to_value(&ui.base.params).unwrap_or_default(),
                }));
            }
        }
    }

    invoke_entries
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every position lands in exactly one part, in order, and no part but one
    /// holding a single heavy item weighs more than the budget.
    #[test]
    fn parts_keep_every_item_in_order_within_the_budget() {
        let weights = [1, 1, 3, 2, 1, 5, 1, 1];
        let parts = pack_parts(&weights, 4);
        assert_eq!(
            parts,
            [vec![0, 1], vec![2], vec![3, 4], vec![5], vec![6, 7]]
        );
        let flat: Vec<usize> = parts.iter().flatten().copied().collect();
        assert_eq!(flat, (0..weights.len()).collect::<Vec<_>>());
        for part in &parts {
            let weight: usize = part.iter().map(|&i| weights[i]).sum();
            assert!(weight <= 4 || part.len() == 1, "{part:?} weighs {weight}");
        }
    }

    /// No items, no parts: a table with no entry declares no part to call.
    #[test]
    fn no_items_make_no_parts() {
        assert!(pack_parts(&[], METHOD_ITEMS).is_empty());
    }

    /// One more item than a method holds opens a second part, and the first
    /// holds exactly [`METHOD_ITEMS`].
    #[test]
    fn one_item_past_the_budget_opens_a_second_part() {
        let parts = pack_parts(&[1; METHOD_ITEMS + 1], METHOD_ITEMS);
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].len(), METHOD_ITEMS);
        assert_eq!(parts[1], [METHOD_ITEMS]);
    }

    /// A transition weighs one and one more per target, and each one's holder
    /// is recorded where its state's transitions are.
    #[test]
    fn transition_holders_are_found_where_their_state_is() {
        let mut model = SCXMLModel::default();
        let transition = |targets: usize| Transition {
            targets: (0..targets).map(|t| format!("t{t}")).collect(),
            ..Transition::default()
        };
        model.states.insert(
            "a".to_string(),
            State {
                id: "a".to_string(),
                transitions: (0..METHOD_ITEMS / 2).map(|_| transition(0)).collect(),
                ..State::default()
            },
        );
        model.states.insert(
            "b".to_string(),
            State {
                id: "b".to_string(),
                // Each weighs two, so the holder `a` began fills before these end.
                transitions: (0..METHOD_ITEMS / 2).map(|_| transition(1)).collect(),
                ..State::default()
            },
        );
        let parts = transition_parts(&model);
        assert_eq!(parts.holders.len(), 2);
        assert_eq!(parts.holder_of["a"], [0; METHOD_ITEMS / 2]);
        let b = &parts.holder_of["b"];
        assert_eq!(b[0], 0);
        assert_eq!(b[b.len() - 1], 1);
        for (holder, members) in parts.holders.iter().enumerate() {
            for (state, position) in members {
                assert_eq!(parts.holder_of[state][*position], holder);
            }
        }
    }
}
