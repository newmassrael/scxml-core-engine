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

/// W3C SCXML 5.7: Done data parameter.
///
/// `expr` and `location` are `Option<String>` because W3C SCXML 6.2.4 mandates
/// exactly one of them on each `<param>`. They are serialized as JSON `null`
/// when absent (no `skip_serializing_if`) so that minijinja templates can
/// distinguish "attribute omitted" (none) from "attribute present but empty"
/// (Some("")) via the canonical `is none` test — undefined-vs-null ambiguity
/// would otherwise mis-route the empty-location structural-error branch.
#[derive(Debug, Clone, Serialize, Default)]
pub struct DoneDataParam {
    pub name: String,
    pub expr: Option<String>,
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

/// An invoke declared on a state, keyed by kind.
///
/// W3C SCXML §6.4 leaves `<invoke type="...">` open for implementation-defined
/// values; SCE treats the three supported kinds as distinct lifecycles:
///
/// - [`Invoke::Scxml`] — static child SCXML session (`src` or inline `<content>`).
/// - [`Invoke::Hybrid`] — SCXML session whose target is resolved at runtime
///   (`srcexpr` / `contentexpr`).
/// - [`Invoke::MeshRpc`] — single request / single reply RPC over Mesh
///   (§9.5 `<invoke type="sce:mesh-rpc">`), no child session.
///
/// Each variant wraps a dedicated struct that holds only the fields its kind
/// actually needs — `finalize_content`/`src`/`namelist` never appear on a
/// hybrid invoke, `srcexpr`/`contentexpr` never appear on a static invoke.
/// The type system enforces these invariants; nothing at the call site has
/// to remember which fields apply to which variant.
///
/// Serialised with an internal `kind` tag so minijinja templates can dispatch
/// on `{% if invoke.kind == "Scxml" %}` without needing tuple-style access.
/// The inner structs use `#[serde(flatten)]` for `common`, so templates keep
/// reading `invoke.invoke_id` / `invoke.autoforward` at the top level.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind")]
pub enum Invoke {
    Scxml(ScxmlInvokeInfo),
    Hybrid(HybridInvokeInfo),
    MeshRpc(MeshRpcInvokeInfo),
}

/// Fields every invoke carries regardless of kind: the W3C identity
/// (`invoke_id`, `state_name`), the declared `<param>` payload, and the
/// optional `idlocation` attribute. Sits at the base of the invoke type
/// hierarchy and is flattened into each wrapping struct's serialisation.
///
/// `invoke_id` is preserved verbatim — including the leading underscore on
/// auto-generated ids — because W3C SCXML 6.4.1 surfaces it in event names
/// (`done.invoke._invoke_0`) and `_event.invokeid`. Codegen sites that need
/// an *identifier-safe* suffix for generated field/variable names must use
/// [`InvokeBase::field_suffix`] instead, otherwise concatenations like
/// `child_` + `_invoke_0` produce the double-underscore artifact
/// `child__invoke_0`.
#[derive(Debug, Clone, Serialize, Default)]
pub struct InvokeBase {
    pub invoke_id: String,
    /// Identifier suffix derived from [`Self::invoke_id`] by trimming the
    /// SCXML auto-id leading underscore. User-supplied ids round-trip
    /// unchanged. Used by Rust/C++ templates to compose field names like
    /// `child_{field_suffix}` without producing double underscores.
    pub field_suffix: String,
    pub state_name: String,
    pub params: Vec<Param>,
    pub idlocation: String,
}

/// Fields shared by W3C SCXML-session invokes (Scxml + Hybrid): child
/// session naming, autoforward, finalize metadata, child datamodel hints.
/// MeshRpc has no child session and therefore no `InvokeSessionCommon`.
#[derive(Debug, Clone, Serialize, Default)]
pub struct InvokeSessionCommon {
    #[serde(flatten)]
    pub base: InvokeBase,
    pub child_name: String,
    pub autoforward: bool,
    pub child_needs_script_engine: bool,
    /// W3C SCXML 6.4: Use specific done.invoke.{id} event instead of generic done.invoke
    #[serde(default)]
    pub use_specific_event: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub child_datamodel_vars: Option<Vec<String>>,
}

impl std::ops::Deref for InvokeSessionCommon {
    type Target = InvokeBase;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl std::ops::DerefMut for InvokeSessionCommon {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

/// SCE Mesh §9.5: short-lived RPC invoke (`<invoke type="sce:mesh-rpc">`).
///
/// Distinct from the SCXML-session kinds because mesh-rpc is a single
/// request / single reply RPC layered over W3C invoke lifecycle events — no
/// child session, no `<finalize>` stream, no autoforward. Only the truly
/// universal invoke fields ([`InvokeBase`]) are shared; `child_name` /
/// `autoforward` do not apply here and the type rejects setting them.
#[derive(Debug, Clone, Serialize, Default)]
pub struct MeshRpcInvokeInfo {
    #[serde(flatten)]
    pub base: InvokeBase,
    /// The raw `src` attribute (`#motor`). Topology resolves it to a target
    /// binding; at parse time we only store the literal so the validator can
    /// match against deploy.yaml.
    pub src: String,
    /// Value of the required `<param name="_mesh_event">`. This populates the
    /// envelope `type` field — never taken from any author-named `<param>`.
    pub mesh_event: String,
    /// Value of the optional `<param name="_mesh_deadline_ms">`, if present.
    /// `<param>` deadline overrides any deploy.yaml binding-level deadline
    /// (§9.5 precedence rule).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deadline_ms: Option<u64>,
}

impl std::ops::Deref for MeshRpcInvokeInfo {
    type Target = InvokeBase;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl std::ops::DerefMut for MeshRpcInvokeInfo {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

/// W3C SCXML 6.4: Static invoke (`<invoke src="..."` or inline `<content>`).
///
/// Holds only the fields the static lifecycle actually uses. Hybrid-only
/// fields (`srcexpr`, `contentexpr`) do not appear here — the type system
/// guarantees `Invoke::Scxml` never carries them. Common metadata lives on
/// [`InvokeSessionCommon`] and is accessible via `Deref` so
/// `scxml_info.invoke_id` still reads naturally.
///
/// Inline `<content><scxml>` is resolved to a concrete child SCXML file
/// during parse (see `SCXMLParser::parse_invoke`); by the time this struct
/// exists, `src` already points at the final path and `child_name` is set.
/// There is no "awaiting extraction" transient state.
#[derive(Debug, Clone, Serialize, Default)]
pub struct ScxmlInvokeInfo {
    #[serde(flatten)]
    pub common: InvokeSessionCommon,
    pub finalize_content: String,
    pub src: String,
    pub namelist: String,
}

/// W3C SCXML 6.4: Hybrid invoke (runtime `srcexpr`/`contentexpr`).
///
/// Scxml-only fields (`finalize_content`, `src`, `namelist`) do not appear
/// here; hybrid invokes never run a `<finalize>` block and resolve their
/// target at runtime, so the legacy flat struct's mixed fields are now
/// variant-scoped.
#[derive(Debug, Clone, Serialize, Default)]
pub struct HybridInvokeInfo {
    #[serde(flatten)]
    pub common: InvokeSessionCommon,
    /// Runtime expression that resolves to a child SCXML path.
    /// Empty if `contentexpr` is used instead.
    pub srcexpr: String,
    /// Runtime expression that produces inline SCXML content.
    /// Empty if `srcexpr` is used instead.
    pub contentexpr: String,
}

impl std::ops::Deref for ScxmlInvokeInfo {
    type Target = InvokeSessionCommon;
    fn deref(&self) -> &Self::Target {
        &self.common
    }
}

impl std::ops::DerefMut for ScxmlInvokeInfo {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.common
    }
}

impl std::ops::Deref for HybridInvokeInfo {
    type Target = InvokeSessionCommon;
    fn deref(&self) -> &Self::Target {
        &self.common
    }
}

impl std::ops::DerefMut for HybridInvokeInfo {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.common
    }
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
    /// Every `<invoke>` on this state in document order, typed by kind
    /// ([`Invoke::Scxml`] / [`Invoke::Hybrid`] / [`Invoke::MeshRpc`]).
    /// Templates dispatch on `invoke.kind`; Rust consumers pattern-match.
    pub invokes: Vec<Invoke>,
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
    /// The `name` attribute from `<scxml name="...">`. Distinct from
    /// `name` (which is the file stem). Used by server-side codegen to
    /// derive the self-target for injected response sends (e.g., the
    /// motor SCXML declares `name="motor"` and `#motor` is the
    /// conventional self-target for server response routing).
    /// Empty if the SCXML element has no name attribute.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub scxml_name: String,
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

    // Every `<invoke>` declared in the document, in insertion order. The
    // typed sum makes "exactly one kind per invoke" a structural invariant —
    // templates dispatch on `invoke.kind`, Rust consumers pattern-match.
    pub invokes: Vec<Invoke>,

    // Parallel regions
    pub parallel_regions: BTreeMap<String, Vec<String>>,

    // Named Context
    pub context_objects: Vec<ContextObject>,
    #[serde(skip)]
    pub context_object_ids: BTreeSet<String>,
    pub needs_nonstatic_method: bool,

    /// True iff the generated `executeEntryActions` body uses `this`.
    ///
    /// Drives the static/non-static decision in
    /// `tools/codegen/templates/entry_exit_actions.jinja2`. Each feature
    /// that emits a `this`-referencing expression into the body contributes
    /// to this predicate — adding a new feature means updating only this
    /// one spot rather than extending the template's conditional. Kept
    /// separate from `needs_nonstatic_method` (which gates lambda captures
    /// in parallel-region blocks) because the two sets of features overlap
    /// but are not identical — notably, mesh-rpc invokes synthesize a
    /// `reinterpret_cast<uintptr_t>(&engine)`-bearing block directly inside
    /// `executeEntryActions` but add no lambda captures, and scheduler
    /// state reached via `engine.scheduleEvent(...)` likewise touches
    /// `engine` (not `this`) but still leaks through the switch body.
    #[serde(default)]
    pub execute_entry_actions_needs_this: bool,

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

impl State {
    /// True iff the state declares any [`Invoke::Scxml`] (a W3C static
    /// SCXML-session invoke). Equivalent to iterating `invokes` and matching
    /// on the variant; kept as a method so call sites read at the intent
    /// level ("does this state spawn a child SCXML session?") rather than
    /// re-spelling the discriminator each time.
    pub fn has_scxml_invoke(&self) -> bool {
        self.invokes.iter().any(|i| matches!(i, Invoke::Scxml(_)))
    }
    /// True iff the state declares any [`Invoke::Hybrid`] (SCXML session
    /// whose target is resolved at runtime via `srcexpr`/`contentexpr`).
    pub fn has_hybrid_invoke(&self) -> bool {
        self.invokes.iter().any(|i| matches!(i, Invoke::Hybrid(_)))
    }
    /// True iff the state declares any [`Invoke::MeshRpc`] (SCE_MESH.md §9.5
    /// short-lived RPC invoke). Used by codegen to decide whether the state
    /// needs onentry / onexit hooks that fire into the generated transport
    /// router's `performMeshInvoke` / `performMeshCancel` callbacks.
    pub fn has_mesh_rpc_invoke(&self) -> bool {
        self.invokes.iter().any(|i| matches!(i, Invoke::MeshRpc(_)))
    }
    /// Iterate over the static-SCXML invoke payloads on this state.
    pub fn iter_scxml_invokes(&self) -> impl Iterator<Item = &ScxmlInvokeInfo> {
        self.invokes.iter().filter_map(|i| match i {
            Invoke::Scxml(info) => Some(info),
            _ => None,
        })
    }
    /// Iterate over the hybrid-SCXML invoke payloads on this state.
    pub fn iter_hybrid_invokes(&self) -> impl Iterator<Item = &HybridInvokeInfo> {
        self.invokes.iter().filter_map(|i| match i {
            Invoke::Hybrid(info) => Some(info),
            _ => None,
        })
    }
}

impl SCXMLModel {
    /// True iff any state declares a static [`Invoke::Scxml`]. The
    /// True iff any state in the model declares any `<invoke>` of any kind
    /// ([`Invoke::Scxml`], [`Invoke::Hybrid`], or [`Invoke::MeshRpc`]).
    /// Replaces the legacy `has_invoke: bool` field.
    pub fn has_invoke(&self) -> bool {
        self.states.values().any(|s| !s.invokes.is_empty())
    }
    /// True iff any state in the model declares a static [`Invoke::Scxml`].
    /// Computed from states — no cached flag duplicates the truth.
    pub fn has_scxml_invoke(&self) -> bool {
        self.states.values().any(|s| s.has_scxml_invoke())
    }
    /// True iff any state in the model declares a hybrid [`Invoke::Hybrid`].
    /// Replaces the legacy `has_hybrid_invoke: bool` field.
    pub fn has_hybrid_invoke(&self) -> bool {
        self.states.values().any(|s| s.has_hybrid_invoke())
    }
    /// True iff any state in the model declares a mesh-rpc [`Invoke::MeshRpc`].
    /// Computed from states — see [`State::has_mesh_rpc_invoke`].
    pub fn has_mesh_rpc_invoke(&self) -> bool {
        self.states.values().any(|s| s.has_mesh_rpc_invoke())
    }
    /// Iterate every static-SCXML invoke across all states in document order.
    /// Replaces the legacy flat `model.static_invokes` read by the codegen
    /// layer and sce_codegen binary.
    pub fn iter_scxml_invokes(&self) -> impl Iterator<Item = &ScxmlInvokeInfo> {
        let mut ordered: Vec<&State> = self.states.values().collect();
        ordered.sort_by_key(|s| s.document_order);
        ordered.into_iter().flat_map(|s| s.iter_scxml_invokes())
    }
    /// Iterate every hybrid-SCXML invoke across all states in document order.
    pub fn iter_hybrid_invokes(&self) -> impl Iterator<Item = &HybridInvokeInfo> {
        let mut ordered: Vec<&State> = self.states.values().collect();
        ordered.sort_by_key(|s| s.document_order);
        ordered.into_iter().flat_map(|s| s.iter_hybrid_invokes())
    }
    /// Rebuild the template-visible `invokes` field from the now-finalised
    /// per-state data. Called once at the end of parsing; subsequent
    /// mutations are not expected — state-level invokes are authoritative.
    pub fn refresh_invokes_view(&mut self) {
        let mut ordered: Vec<&State> = self.states.values().collect();
        ordered.sort_by_key(|s| s.document_order);
        self.invokes = ordered
            .into_iter()
            .flat_map(|s| s.invokes.iter().cloned())
            .collect();
    }

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
