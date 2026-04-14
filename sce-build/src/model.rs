// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Data structures for SCXML model representation.
// Ports Python scxml_parser.py dataclasses to Rust structs with serde Serialize
// for minijinja template rendering.

use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

use crate::forge::model::InlineKind;

/// W3C SCXML 3.3: Transition element
#[derive(Debug, Clone, Serialize, Default)]
pub struct Transition {
    pub event: String,
    pub target: String,
    pub cond: String,
    pub cond_cpp: String,
    pub cond_cpp_transformed: String,
    pub is_pure_in_predicate: bool,
    pub is_cpp_condition: bool,
    pub cond_kt: String,
    pub is_kt_condition: bool,
    #[serde(rename = "type")]
    pub transition_type: String,
    pub actions: Vec<Action>,
    pub needs_string_matching: bool,
    pub matching_enum_values: Vec<String>,
    /// Original index within parent state's transition list
    pub transition_index: usize,
    /// W3C SCXML 3.11: History target if transition targets a history state
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history_target: Option<String>,
    /// W3C SCXML 3.11: Resolved leaf target for history default (Kotlin Phase 1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history_leaf_target: Option<String>,
    /// Prefix matching events for Kotlin templates
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub prefix_matching_events: Vec<String>,
    /// W3C SCXML 3.13: True internal transition (target is descendant of source)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_true_internal: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_source: Option<String>,
}

/// W3C SCXML executable content action
#[derive(Debug, Clone, Serialize, Default)]
pub struct Action {
    #[serde(rename = "type")]
    pub action_type: String,
    
    pub event: String,
    
    pub location: String,
    
    pub expr: String,
    
    pub content: String,
    
    pub target: String,
    
    pub targetexpr: String,
    
    pub send_type: String,
    
    pub delay: String,
    
    pub delayexpr: String,
    pub delay_ms: i64,
    
    pub id: String,
    
    pub auto_send_id: String,
    
    pub idlocation: String,
    
    pub namelist: String,
    
    pub contentexpr: String,
    
    pub eventexpr: String,
    
    pub typeexpr: String,
    
    pub label: String,
    // if/elseif/else
    
    pub cond: String,
    
    pub cond_cpp: String,
    
    pub cond_kt: String,
    #[serde(default)]
    pub is_pure_in_predicate: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub then_actions: Vec<Action>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub elseif_branches: Vec<ElseIfBranch>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub else_actions: Vec<Action>,
    // foreach
    
    pub array: String,
    
    pub item: String,
    
    pub index: String,
    /// foreach body / transition actions
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<Action>,
    // send params
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<Param>,
    // cancel
    
    pub sendid: String,
    
    pub sendidexpr: String,
    // send param static optimization
    #[serde(default)]
    pub is_static_literal: bool,

    pub static_value: String,
    // Named Context: native code actions
    #[serde(skip_serializing_if = "String::is_empty")]
    pub content_transformed: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub content_kt: String,
    #[serde(default)]
    pub is_cpp_function: bool,
    #[serde(default)]
    pub is_kt_function: bool,

    // SCE_MESH.md §13 — mesh metadata is not carried on individual
    // <send> actions. Communication pattern is inferred from event name
    // conventions (mesh::pattern), RPC reply pairing is inferred from
    // topology structure (mesh::topology::detect_rpc_pairs), and QoS is
    // a transport binding concern (deploy.yaml).
}

/// W3C SCXML if/elseif branch
#[derive(Debug, Clone, Serialize, Default)]
pub struct ElseIfBranch {
    pub cond: String,
    pub cond_cpp: String,
    pub cond_kt: String,
    pub is_pure_in_predicate: bool,
    pub actions: Vec<Action>,
}

/// W3C SCXML 6.2.4: Send parameter
#[derive(Debug, Clone, Serialize, Default)]
pub struct Param {
    pub name: String,
    pub expr: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub location: String,
    pub is_static_literal: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub static_value: String,
}

/// W3C SCXML 5.2: Datamodel variable
#[derive(Debug, Clone, Serialize, Default)]
pub struct Variable {
    pub id: String,
    pub expr: String,
    pub src: String,
    pub content: String,
    /// Classified type: int, string, bool, runtime
    #[serde(rename = "type")]
    pub var_type: String,
}

/// W3C SCXML 3.11: History state information
#[derive(Debug, Clone, Serialize, Default)]
pub struct HistoryInfo {
    pub parent: String,
    #[serde(rename = "type")]
    pub history_type: String,
    pub default_target: String,
    pub leaf_target: String,
    pub default_actions: Vec<Action>,
}

/// W3C SCXML 5.7: Done data for final states
#[derive(Debug, Clone, Serialize, Default)]
pub struct DoneData {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<DoneDataParam>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub content: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub contentexpr: String,
}

/// W3C SCXML 5.7: Done data parameter
#[derive(Debug, Clone, Serialize, Default)]
pub struct DoneDataParam {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
}

/// Named Context object declaration
#[derive(Debug, Clone, Serialize, Default)]
pub struct ContextObject {
    pub id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub cpp_type: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub cpp_include: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub kt_type: String,
}

/// W3C SCXML 6.4: Static invoke information
#[derive(Debug, Clone, Serialize, Default)]
pub struct InvokeInfo {
    pub invoke_id: String,
    pub child_name: String,
    pub state_name: String,
    pub autoforward: bool,
    pub finalize_content: String,
    pub src: String,
    pub params: Vec<Param>,
    pub idlocation: String,
    pub namelist: String,
    pub child_needs_script_engine: bool,
    /// W3C SCXML 6.4: Use specific done.invoke.{id} event instead of generic done.invoke
    #[serde(default)]
    pub use_specific_event: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub child_datamodel_vars: Option<Vec<String>>,
    // Hybrid invoke fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub srcexpr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contentexpr: Option<String>,
}

/// W3C SCXML 3.3: State element
#[derive(Debug, Clone, Serialize, Default)]
pub struct State {
    pub id: String,
    pub initial: String,
    pub initial_children: Vec<String>,
    pub is_final: bool,
    pub is_parallel: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    pub transitions: Vec<Transition>,
    pub on_entry_blocks: Vec<Vec<Action>>,
    pub on_exit_blocks: Vec<Vec<Action>>,
    pub datamodel: Vec<Variable>,
    pub invokes: Vec<serde_json::Value>,
    pub static_invokes: Vec<InvokeInfo>,
    pub hybrid_invokes: Vec<InvokeInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub donedata: Option<DoneData>,
    pub document_order: u32,
    pub initial_transition_actions: Vec<Action>,
    pub initial_history_id: String,
    pub initial_history_default_target: String,
    pub initial_history_default_actions: Vec<Action>,
}

/// W3C SCXML: Complete state machine model
#[derive(Debug, Clone, Serialize, Default)]
pub struct SCXMLModel {
    pub name: String,
    pub initial: String,
    pub initial_leaf: String,
    pub binding: String,
    pub datamodel_type: String,

    pub states: BTreeMap<String, State>,
    pub events: BTreeSet<String>,
    pub history_default_targets: BTreeMap<String, String>,
    pub history_states: BTreeMap<String, HistoryInfo>,

    // Feature flags
    pub has_dynamic_expressions: bool,
    pub has_parallel_states: bool,
    pub has_history_states: bool,
    pub has_invoke: bool,
    pub has_hybrid_invoke: bool,
    pub has_event_metadata: bool,
    pub has_parent_communication: bool,
    pub has_child_communication: bool,
    pub needs_http_send: bool,
    pub needs_script_engine: bool,
    pub uses_in_predicate: bool,
    pub has_transition_actions: bool,
    pub has_entry_actions: bool,
    pub has_exit_actions: bool,
    pub has_hierarchy: bool,
    pub needs_event_matching_helper: bool,
    pub document_rejected: bool,

    // Event metadata flags
    pub needs_event_name: bool,
    pub needs_event_data: bool,
    pub needs_event_type: bool,
    pub needs_event_sendid: bool,
    pub needs_event_origin: bool,
    pub needs_event_origintype: bool,
    pub needs_event_invokeid: bool,
    pub needs_external_flag: bool,

    // Variables
    pub variables: Vec<Variable>,
    pub global_scripts: Vec<Action>,

    // Invoke
    pub static_invokes: Vec<InvokeInfo>,
    pub hybrid_invokes: Vec<InvokeInfo>,

    // Parallel regions
    pub parallel_regions: BTreeMap<String, Vec<String>>,

    // Named Context
    pub context_objects: Vec<ContextObject>,
    #[serde(skip)]
    pub context_object_ids: BTreeSet<String>,
    pub needs_nonstatic_method: bool,

    // SCE Forge: Inline kind declarations from <data sce:kind="..."> elements.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub inline_kinds: Vec<InlineKind>,

    // Path info
    #[serde(skip_serializing_if = "String::is_empty")]
    pub scxml_source_path: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub scxml_base_path: String,

    // Analysis helpers (set by analyzer)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub needs_transition_helper: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub needs_assign_helper: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub needs_foreach: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub needs_guard_helper: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub needs_send_helper: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub needs_event_data_helper: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub needs_donedata_helper: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub needs_event_type_helper: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub needs_event_scheduler: Option<bool>,
}

/// Maximum recursion depth for resolving nested initial states to leaf states.
pub(crate) const MAX_STATE_DEPTH: usize = 20;

impl SCXMLModel {
    /// W3C SCXML 3.3/3.4: Resolve state ID to leaf by following initial attrs
    pub fn resolve_to_leaf(&self, state_id: &str) -> String {
        let mut current = state_id.to_string();
        for _ in 0..MAX_STATE_DEPTH {
            let state = match self.states.get(&current) {
                Some(s) => s,
                None => return current,
            };
            if !state.initial.is_empty() && self.states.contains_key(&state.initial) {
                current = state.initial.clone();
            } else if state.is_parallel {
                let first_child = self
                    .states
                    .iter()
                    .filter(|(_, s)| s.parent.as_deref() == Some(&current))
                    .min_by_key(|(_, s)| s.document_order);
                match first_child {
                    Some((child_id, _)) => current = child_id.clone(),
                    None => break,
                }
            } else {
                break;
            }
        }
        current
    }
}
