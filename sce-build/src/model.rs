// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! Data structures for SCXML model representation.
//! Ports Python scxml_parser.py dataclasses to Rust structs with serde Serialize
//! for minijinja template rendering.
//!
//! # `Option<T>` template-field convention
//!
//! Template-consumed `Option<T>` fields take one of two shapes, keyed on how
//! the consuming jinja2 template guards the field:
//!
//! - Guarded by `is not none` / `is none` — the field **must not** carry
//!   `#[serde(skip_serializing_if = "Option::is_none")]`. The field serializes
//!   to JSON `null` when absent. `is not none` correctly fails on `null` but
//!   spuriously succeeds on `undefined` (minijinja's lax semantics), so
//!   skipping serialization would emit the guarded block unconditionally.
//!   Canonical examples: [`DoneDataParam::expr`] / [`DoneDataParam::location`],
//!   [`crate::mesh::topology::MeshRpcInvokeSite::deadline_ms`], and
//!   [`MeshRpcInvokeInfo::deadline_ms`].
//!
//! - Guarded by truthy (`{% if x %}`), subfield access
//!   (`{% if x and x.foo %}`), or `| default(y)` — the field **must** carry
//!   `#[serde(skip_serializing_if = "Option::is_none")]`. These guards depend
//!   on `undefined` semantics; serializing a literal `null` would evaluate
//!   truthy, bypass `default()`, or feed through to downstream rendering.
//!
//! The convention is enforced by `sce-build/tests/option_serde_convention.rs`,
//! which walks every jinja2 template for `.x is [not] none` probes and fails
//! when a Rust `Option<x>` field in a non-wire-format file carries
//! `skip_serializing_if = "Option::is_none"`.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::forge::error::SourceLocation;
use crate::forge::model::InlineKind;

/// W3C SCXML default namespace URI (W3C SCXML §3.5). Mirrors
/// [`crate::forge::model::SCE_NAMESPACE`] for the sce: extension axis —
/// used by the parser's element-dispatch helpers to filter children
/// by namespace as well as local name, closing the foreign-NS local-name
/// collision footgun previously documented in `SCE_FORGE.md` §3.1.
pub const SCXML_NAMESPACE: &str = "http://www.w3.org/2005/07/scxml";

/// W3C SCXML 3.3: Transition element
#[derive(Debug, Clone, Serialize, Default)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
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
    /// Watching-zenoh RFC §5.O Atomic 0: post-preprocessor source
    /// position of the `<transition>` element, populated by
    /// [`crate::parser::SCXMLParser::parse_transition`] from
    /// `roxmltree::Document::text_pos_at`. Templates emit
    /// per-backend SCE-MAP markers above the transition's emitted
    /// handler function (Rust `// SCE-MAP:` + `#[doc]`, C/C++
    /// `#line`, Go `//line`, Kotlin `// SCE-MAP:`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_location: Option<SourceLocation>,
}

/// W3C SCXML executable content action
#[derive(Debug, Clone, Serialize, Default)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
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
    /// Watching-zenoh RFC §5.O Atomic 0: post-preprocessor source
    /// position of the executable-content element this action
    /// represents (`<raise>` / `<send>` / `<assign>` / `<log>` /
    /// `<script>` / `<if>` / `<foreach>` / `<cancel>`). Populated by
    /// [`crate::parser::SCXMLParser::parse_executable_content_single`].
    /// Used by §5.O Atomic 1 per-action attribution (Atomic 0 only
    /// emits function-level markers, so this field is read by the
    /// pre-emit `validate_emission_provenance` walker today).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_location: Option<SourceLocation>,
}

/// W3C SCXML if/elseif branch
#[derive(Debug, Clone, Serialize, Default)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct ElseIfBranch {
    pub cond: String,
    pub cond_cpp: String,
    pub cond_kt: String,
    pub is_pure_in_predicate: bool,
    pub actions: Vec<Action>,
}

/// W3C SCXML 6.2.4: Send parameter
#[derive(Debug, Clone, Serialize, Default)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
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
#[cfg_attr(test, derive(schemars::JsonSchema))]
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
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct HistoryInfo {
    pub parent: String,
    #[serde(rename = "type")]
    pub history_type: String,
    pub default_target: String,
    pub leaf_target: String,
    pub default_actions: Vec<Action>,
}

/// W3C SCXML 5.7: Done data for final states.
///
/// `content` is a [`DoneDataContent`] sum so that the parser decision —
/// `<content expr="...">` vs `<content>literal</content>` vs omitted — is
/// encoded in the type and survives into templates as a `{kind, text}`
/// object. Templates dispatch on `donedata.content.kind` rather than the
/// legacy truthy-check on two parallel strings.
#[derive(Debug, Clone, Serialize, Default)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct DoneData {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<DoneDataParam>,
    pub content: DoneDataContent,
}

/// W3C SCXML 5.5: `<content>` body semantics.
///
/// - [`DoneDataContent::None`] — no `<content>` child (`{"kind":"none"}`).
/// - [`DoneDataContent::Expression`] — `<content expr="X"/>`, MUST be
///   evaluated against the active datamodel (`{"kind":"expression","text":"X"}`).
/// - [`DoneDataContent::Literal`] — `<content>inline text</content>`; per
///   W3C SCXML 5.5 the children are used **as the content value**, not
///   re-evaluated as an expression. Literal means no script engine required
///   at runtime (`{"kind":"literal","text":"..."}`).
///
/// Serialized via adjacent tagging (`tag = "kind", content = "text"`) so
/// minijinja consumers match on `donedata.content.kind` and read the payload
/// from `.text` when present.
#[derive(Debug, Clone, Serialize, Default)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(tag = "kind", content = "text", rename_all = "snake_case")]
pub enum DoneDataContent {
    #[default]
    None,
    Expression(String),
    Literal(String),
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
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct DoneDataParam {
    pub name: String,
    pub expr: Option<String>,
    pub location: Option<String>,
}

/// Named Context object declaration
#[derive(Debug, Clone, Serialize, Default)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
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
#[cfg_attr(test, derive(schemars::JsonSchema))]
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
#[cfg_attr(test, derive(schemars::JsonSchema))]
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
#[cfg_attr(test, derive(schemars::JsonSchema))]
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
    /// W3C SCXML 6.4 (test226/240/241/243/244/245/276): the child SCXML
    /// has at least one `<send target="#_parent" event="..."/>`. Codegen
    /// uses this to gate parent_sm / parent_dispatch wiring at child
    /// spawn time so parent-routed events reach the parent's external
    /// queue. Mirrors the C++ reuse of `child_model.has_parent_communication`
    /// surfaced via the parsed child model — populated by
    /// `collect_child_to_parent_events` during invoke metadata
    /// resolution.
    #[serde(default)]
    pub child_has_send_to_parent: bool,
    /// W3C SCXML 6.2 (test207): the child SCXML carries a non-empty
    /// `<send delay="...">` so its codegen emitted a scheduler queue
    /// + a `_tick` entry point. Parents that drive active children
    /// from `_drive_active_children` need to call the child's `_tick`
    /// (not just `_step`) so scheduled child events promote onto the
    /// child's internal queue and drive its macrostep on the same
    /// outer-step iteration. Top-level fixtures know about this
    /// derived from the child SCXML at parse time, mirroring how
    /// `child_needs_script_engine` is populated.
    #[serde(default)]
    pub child_needs_event_scheduler: bool,
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

/// SCE Mesh §9.5: target selector for a `<invoke type="sce:mesh-rpc">`.
///
/// W3C SCXML §6.4 requires exactly one of `src` / `srcexpr`; this sum type
/// makes "both empty" and "both set" structurally impossible so parser
/// consumers no longer carry that invariant as runtime discipline.
///
/// `#[serde(untagged)]` with struct-named variants serialises to a flat
/// key — either `{"src": "#motor"}` or `{"srcexpr": "'#motor_' + id"}` —
/// not a nested `{"target": {"Src": ...}}` shape. This keeps minijinja
/// templates' flat attribute access (`{{ invoke_info.src }}`) working for
/// the literal case and lets them branch on `{% if invoke_info.srcexpr %}`
/// for runtime evaluation, mirroring every other SCXML dual-attribute
/// pair (`content` / `contentexpr`, `type` / `typeexpr`). Variants are
/// distinguished by field name, so untagged deserialisation has no
/// string-shape ambiguity to resolve.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum MeshRpcTarget {
    /// Static `src="#<machine_name>"`. Topology resolves it to a
    /// deploy.yaml binding at build time.
    Src { src: String },
    /// Runtime `srcexpr="<datamodel expression>"`. Evaluated at
    /// `<invoke>` entry; result must be a `#<machine_name>` string
    /// matching a static binding (no peer discovery implied — §3.3).
    SrcExpr { srcexpr: String },
}

impl Default for MeshRpcTarget {
    fn default() -> Self {
        Self::Src { src: String::new() }
    }
}

impl MeshRpcTarget {
    /// The static `#<machine_name>` literal, if this is the [`Self::Src`]
    /// variant. `None` for [`Self::SrcExpr`] because the resolution of a
    /// runtime expression cannot be surfaced at build time — topology
    /// enumeration must skip such invokes.
    pub fn src_literal(&self) -> Option<&str> {
        match self {
            Self::Src { src } => Some(src.as_str()),
            Self::SrcExpr { .. } => None,
        }
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
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct MeshRpcInvokeInfo {
    #[serde(flatten)]
    pub base: InvokeBase,
    /// Target selector (§9.5). [`MeshRpcTarget`] enforces exactly-one
    /// between `src` and `srcexpr` at the type level; the `#[serde(flatten)]`
    /// on an untagged sum produces a flat `src`/`srcexpr` key in the
    /// rendered template context, preserving pre-existing byte goldens
    /// for every fixture that uses `src`.
    #[serde(flatten)]
    pub target: MeshRpcTarget,
    /// Value of the required `<param name="_mesh_event">`. This populates the
    /// envelope `type` field — never taken from any author-named `<param>`.
    pub mesh_event: String,
    /// Value of the optional `<param name="_mesh_deadline_ms">`, if present.
    /// `<param>` deadline overrides any deploy.yaml binding-level deadline
    /// (§9.5 precedence rule).
    ///
    /// Serializes to JSON `null` when absent (no `skip_serializing_if`),
    /// aligning with [`crate::model::DoneDataParam`] so minijinja
    /// templates can use the canonical `is not none` test without
    /// tripping over `undefined` vs `null` ambiguity.
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

/// SCE_MESH.md §9.6.2 wire-14/20 peer entry — a machine name paired with
/// the author-selected deploy.yaml transport for the scxml-remote invoke
/// channel. `transport` is `None` for same-device cross-partition peers
/// that take the implicit shm fallback (today's only wired codegen path);
/// `Some(<name>)` carries a cross-device declaration resolved from
/// `machines.<parent>.bindings["#<peer>"].transport`. Session 1 of the
/// cross-device roll-out records the field on the model; Session 2 adds
/// `connect_endpoint` so custom_tcp codegen has the peer's listen address
/// available for the per-peer `CustomTcp::Client` ctor args.
///
/// `connect_endpoint` mirrors `transport`: it is `Some` only when the peer
/// actually needs an endpoint (currently custom_tcp only) AND the peer's
/// device has `transports.custom_tcp.listen` declared in deploy.yaml; all
/// other paths (shm, local, missing device config) keep it `None`. Stored
/// unquoted — the template emits the quoted C++ string literal.
#[derive(Debug, Clone, Serialize, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct ScxmlRemotePeerBinding {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transport: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connect_endpoint: Option<String>,
}

impl ScxmlRemotePeerBinding {
    /// Construct a peer entry with no author-declared transport — the
    /// same-device shm implicit-fallback shape. Keeps call sites tidy
    /// where the transport is always absent (today's classifier callers
    /// that operate on same-device data).
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            transport: None,
            connect_endpoint: None,
        }
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
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct ScxmlInvokeInfo {
    #[serde(flatten)]
    pub common: InvokeSessionCommon,
    pub finalize_content: String,
    pub src: String,
    pub namelist: String,
    /// SCE_MESH.md §9.6 remote `<invoke type="scxml">`. When `src` is of the
    /// form `#<name>` and `<name>` matches a distinct mesh machine declared
    /// in `deploy.yaml`, this carries that machine name (without the leading
    /// `#`). C++ codegen branches on this field to emit the §10.7.1
    /// `SESSION_F_NOT_IMPLEMENTED` scaffold until Session F lands the
    /// wire patterns 14-20 runtime. `None` for local W3C invokes (inline
    /// `<content>` or file-relative `src`) and for non-mesh builds — those
    /// flow through the existing local-invoke path unchanged. Populated
    /// by [`crate::inject_partition_context_for`] from the deploy topology.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_mesh_target: Option<String>,
    /// SCE_MESH.md §9.6 L1393 cross-device transport — the author-declared
    /// transport on `machines.<parent>.bindings["#<peer>"]` when the peer
    /// is cross-device. `None` when `remote_mesh_target` is also `None`
    /// (not a remote invoke) OR the invoke is cross-partition same-device
    /// (implicit shm fallback, today's only wired codegen path). The
    /// Session 1 cross-device validator rejects cross-device declarations
    /// whose transport is not yet wired by Session 2 C++ dispatch, so
    /// this field is only a diagnostic payload until Session 2 lands.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_mesh_transport: Option<String>,
}

/// W3C SCXML 6.4: Hybrid invoke (runtime `srcexpr`/`contentexpr`).
///
/// Scxml-only fields (`finalize_content`, `src`, `namelist`) do not appear
/// here; hybrid invokes never run a `<finalize>` block and resolve their
/// target at runtime, so the legacy flat struct's mixed fields are now
/// variant-scoped.
#[derive(Debug, Clone, Serialize, Default)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
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

/// watching-zenoh RFC §5.E B7-η' — `<sce:on-sample>` SCXML extension.
/// Wraps the spec line 1269 subscriber callback contract ("the
/// subscriber callback receives `&Sample<'_>`; the slot returns to the
/// pool when the callback returns") into a state-level declaration
/// that the η' codegen template lowers to a per-state callback
/// registration on the link's RX path.
///
/// Q-OnSample-1 (Y) parser-AST extension; Q-OnSample-2 (a) valid only
/// inside `<state>` and `<parallel>`; Q-OnSample-3 v1 attributes are
/// Watching-zenoh RFC §5.2 Round F-α — top-level `<sce:driver
/// href="..."/>` declaration on the SCXML root. References an
/// externally-authored driver header that the C11 backend `#include`s
/// into the emitted `*_sm.c`. Author-time reference only; SCE does not
/// parse the driver header itself (Q-Round-F-D2 lock — cross-TU
/// signature verification is delegated to the C compiler).
///
/// Resolution happens at compile-model time: each `href` is joined
/// against `deploy.yaml`'s `platform.driver_root` (or the SCXML file's
/// parent directory as fallback), and an unresolved path surfaces
/// `mcu/driver-header-not-found`. Only the C11 backend consumes this
/// field for codegen emission (Q-Round-F-D5 / D6 locks).
#[derive(Debug, Clone, Serialize, Default)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct DriverRef {
    /// `href="..."` — author-written path to the driver header. May be
    /// absolute or relative to the resolved root directory. Stored
    /// verbatim (no canonicalisation) so the original string can appear
    /// in `actual` payloads of `mcu/driver-header-not-found`.
    pub href: String,
    /// Post-resolution absolute path of the driver header, populated by
    /// the compile-model-time resolver (`crate::lib`-level entry points).
    /// `None` immediately after parser exit; `Some(...)` once the
    /// resolver has confirmed the file exists. Codegen consumes this
    /// field — not `href` — when emitting `#include` lines so the
    /// emitted path matches what was actually resolved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_path: Option<String>,
    /// 0-based document order within the SCXML root's `<sce:driver>`
    /// declarations. Lets diagnostics quote a stable index when the
    /// source file lacks reliable line numbers.
    pub document_order: u32,
    /// Source location of the `<sce:driver>` element (post-preprocessor
    /// row/col). `None` when the parser cannot reconstruct the position
    /// — diagnostics fall back to file-level anchoring in that case.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_location: Option<SourceLocation>,
}

/// Axis-3 inversion — closed-set vocabulary for the cross-document
/// session-FSM role that a SCXML document declares via the top-level
/// `<sce:session-role kind="..."/>` extension.
///
/// v1 ships a single variant ([`AcceptSide`]) covering the
/// canonical session-FSM accept-side state machine (`docs/session-fsm.md`
/// §2.6 — `Accepting`, `Accepting.AwaitingInitSyn`,
/// `Accepting.SentInitAck`, etc.). Future variants (initiator-side,
/// broker-side, …) extend this enum in lockstep with their codegen-
/// side semantics; declaring any unknown `kind` value at parse time
/// fires [`crate::forge::error::ValidationError::ScxmlUnknownSessionRoleKind`]
/// with the current vocabulary list embedded in the message.
///
/// The orchestrator (`crate::resolve_listener_links`) joins by named
/// role only: a deploy.yaml link with `role: listener` pairs with the
/// machine's source SCXML iff that SCXML declares
/// `<sce:session-role kind="accept-side"/>`. Partial-claim cases
/// (either side declares the role but not the other) fire typed
/// cross-doc diagnostics — see RFC Axis-3 Q-A7.
///
/// `kind` values use the canonical session-FSM vocabulary
/// (`accept-side`), NOT the wire-side deploy vocabulary
/// (`listener`). The asymmetric pairing is hardcoded in the
/// orchestrator per RFC Axis-3 Q-A6 (a) — each domain uses its native
/// term; the orchestrator knows the pairing table.
///
/// [`AcceptSide`]: SessionRoleKind::AcceptSide
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum SessionRoleKind {
    /// Implements the canonical session-FSM accept-side state machine
    /// (`docs/session-fsm.md` §2.6). Pairs 1:1 with a deploy.yaml link
    /// configured as `role: listener` + `trust_class: session_arming`.
    AcceptSide,
}

impl SessionRoleKind {
    /// Wire-canonical kind string used in `<sce:session-role kind="...">`
    /// XML and in diagnostic `actual` / `expected` payloads. Stable
    /// across Rust edition / `Debug`-impl changes.
    pub fn as_str(self) -> &'static str {
        match self {
            SessionRoleKind::AcceptSide => "accept-side",
        }
    }

    /// Parse a `kind` attribute value into a [`SessionRoleKind`]. Returns
    /// `None` for any value outside the v1 closed set; the parser uses
    /// this to materialize the
    /// [`crate::forge::error::ValidationError::ScxmlUnknownSessionRoleKind`]
    /// diagnostic with the v1 vocabulary list.
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "accept-side" => Some(SessionRoleKind::AcceptSide),
            _ => None,
        }
    }

    /// Canonical sorted list of every variant's wire-string form, used
    /// by [`Self::parse`] failure paths to populate the diagnostic
    /// vocabulary list. Kept as a `'static` slice so the diagnostic's
    /// `expected` payload can borrow without allocation.
    pub fn all_wire_names() -> &'static [&'static str] {
        &["accept-side"]
    }
}

/// `link` (forge link kind artifact name) + `event` (SCXML event name
/// dispatched on Sample arrival), both required. B7-η' Atomic A2 adds
/// optional `callback` attribute (Q-Callback-1 Option α + Q-Callback-2
/// `rust:` prefix-typed reference into the user's symbol space).
#[derive(Debug, Clone, Serialize, Default)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct OnSampleNode {
    /// `link="X"` — forge link kind artifact name. Cross-reference
    /// resolution against the build's [`crate::forge::pool_registry`]-style
    /// `SceCrossDocRegistry` is the Atomic B follow-on; structural
    /// validators in this atomic do not consult the registry.
    pub link: String,
    /// `event="X"` — SCXML event name raised when a Sample arrives.
    /// State-level `<transition event="X">` blocks dispatch normally
    /// per W3C SCXML §5.10, with `_event.data` carrying the borrowed
    /// `&Sample<'_>` reference.
    pub event: String,
    /// `callback="rust:crate::path::fn"` — optional extern reference
    /// into the user's symbol space (Q-Callback-1 Option α). When
    /// present, codegen emits `path(&sample)` at the dispatch site,
    /// forcing borrow-mode at the call boundary; rustc enforces the
    /// user's signature shape against the borrow contract. The
    /// language prefix today is `rust:` only; future axes (`c:`,
    /// `kotlin:`, …) reuse the same attribute via Q-Callback-2's
    /// language-typed parsing. Absence (`None`) means codegen
    /// synthesizes a default dispatch shim — backwards-compat with
    /// the landed Atomic A/B `<sce:on-sample link/event>` shape
    /// (Q-Callback-5).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback: Option<String>,
    /// 0-based document order within this state's `<sce:on-sample>`
    /// blocks. Lets diagnostics quote a stable per-state index even
    /// when the source file lacks reliable line numbers.
    pub document_order: u32,
}

/// W3C SCXML 3.3: State element
#[derive(Debug, Clone, Serialize, Default)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
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
    /// `<sce:on-sample>` declarations on this state, in document order.
    /// Empty for states without sample subscriptions. Q-OnSample-5 (a)
    /// allows multiple blocks (one per link) per state with a
    /// uniqueness validator on the `link` attribute.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub on_sample_blocks: Vec<OnSampleNode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub donedata: Option<DoneData>,
    pub document_order: u32,
    pub initial_transition_actions: Vec<Action>,
    pub initial_history_id: String,
    pub initial_history_default_target: String,
    pub initial_history_default_actions: Vec<Action>,
    /// Watching-zenoh RFC §5.O Atomic 0: post-preprocessor source
    /// position of the `<state>` / `<final>` / `<parallel>` element.
    /// Populated by [`crate::parser::SCXMLParser::parse_states`].
    /// Drives the per-state SCE-MAP marker that codegen templates
    /// emit above the on-entry / on-exit / transition-handler
    /// functions this state lowers to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_location: Option<SourceLocation>,
}

/// W3C SCXML: Complete state machine model
#[derive(Debug, Clone, Serialize, Default)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
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

    /// Watching-zenoh RFC §5.J.2 + §5.L (Q-RustNoStd-7 (a), C3 Atomic
    /// B-γ1): per-document event-queue capacity declared via
    /// `<scxml sce:capacity="N">` on the root element. Drives the
    /// `heapless::spsc::Queue<E, N>` capacity literal that Atomic
    /// B-γ2's runtime port consumes when emitting the no_std event
    /// queue. Unit: events (not bytes).
    ///
    /// Resolution rule (Q-RustNoStd-7 (a)): per-instance attribute
    /// wins; fallback to deploy.yaml
    /// `machines.<m>.scheduler.default_event_queue_capacity` when
    /// the attribute is absent. Both being absent is permitted today
    /// (`--no-std` codegen still works against std runtime); the
    /// capacity becomes load-bearing when B-γ2's heapless adoption
    /// lands.
    ///
    /// Schema invariant enforced by the parser: present-but-malformed
    /// (non-numeric, zero, or u32-overflow) surfaces
    /// `validation/invalid-attribute` — no silent coercion.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_queue_capacity: Option<u32>,

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
    /// watching-zenoh RFC §5.E B7-η' codegen wire-up — sorted set of
    /// every `link` name referenced by any state's
    /// `<sce:on-sample link="X" .../>` block. Derived in
    /// [`crate::analyzer::analyze_model_features`] as the union over
    /// `state.on_sample_blocks`. Empty when the machine has no sample
    /// subscriptions (the common case); non-empty drives the Rust
    /// state_machine template's `deliver_link_X_sample` method
    /// emission per Q-Wire-9 (host-driven delivery, generic over `M:
    /// SampleMeta` so the codegen does not need to know the link's
    /// concrete metadata type at template-render time).
    #[serde(skip_serializing_if = "BTreeSet::is_empty", default)]
    pub on_sample_links: BTreeSet<String>,
    /// Watching-zenoh RFC §5.2 Round F-α — every top-level `<sce:driver
    /// href="..."/>` reference on the document root, in document order.
    /// Each entry is an externally-authored driver header that the C11
    /// backend `#include`s into the emitted `*_sm.c` so cross-TU symbol
    /// resolution is handled by the C compiler (Q-Round-F-D2 lock).
    ///
    /// Empty when the machine declares no driver dependencies (the
    /// common case for non-MCU / hosted backends). Non-empty: each
    /// `href` is resolved at compile-model time against
    /// `deploy.yaml`'s `platform.driver_root` (or the SCXML file's
    /// parent directory as fallback); unresolved paths surface
    /// `mcu/driver-header-not-found`. Only the C11 backend consumes
    /// this field; other backends emit the references unchanged into
    /// their codegen context (no `#include` semantics translate
    /// outside C).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub driver_refs: Vec<DriverRef>,
    /// Axis-3 inversion (RFC `claudedocs/rfc-axis3-listener-role-declarations.md`)
    /// — set of session-FSM roles this SCXML document explicitly
    /// declares via top-level `<sce:session-role kind="..."/>`
    /// elements. Each variant appears at most once (parse-time
    /// uniqueness is enforced by
    /// [`crate::forge::error::ValidationError::ScxmlDuplicateSessionRoleDeclaration`]).
    ///
    /// Empty when the document declares no roles (the common case
    /// for non-session-FSM SCXML). Non-empty: drives the orchestrator's
    /// cross-doc listener-pair join — see
    /// [`crate::resolve_listener_links`] for Phase B's consumer wire-up.
    /// During Phase A (foundation) the orchestrator does NOT yet read
    /// this field; the existing `accepting_substate_present` walker
    /// is unchanged. Phase B switches the join over to this set.
    ///
    /// `BTreeSet` keeps iteration order deterministic so any diagnostic
    /// quoting the declared-roles list produces a stable string.
    #[serde(skip_serializing_if = "BTreeSet::is_empty", default)]
    pub declared_session_roles: BTreeSet<SessionRoleKind>,
    /// Watching-zenoh RFC §5.2 Round F-α-2 — section attribute payload
    /// for the C11 backend's `SCE_SM_FN` macro emission. Set by
    /// [`crate::compile_scxml_lang_typed_with_section`] when
    /// `deploy.yaml`'s `platform.c11_section_attribute.class` is
    /// present; `None` everywhere else (the parser never touches this
    /// field, the SCXML document does not encode the section choice).
    ///
    /// When `Some("<name>")`, every C11 statechart function definition
    /// receives a `SCE_SM_FN` prefix that the emitted macro expands to
    /// `__attribute__((section("<name>")))`. When `None`, the macro
    /// expands to nothing — function definitions stay textually
    /// `SCE_SM_FN <ret> name(...)` so a downstream sce-rust-runtime
    /// linker change can later swap the macro definition without
    /// re-emitting the per-function source.
    ///
    /// Non-C11 backends ignore this field (the section-attribute
    /// reject `mcu/section-attribute-on-non-mcu-target` fires at the
    /// orchestrator pass before any non-C11 codegen runs).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub c11_section_attribute_class: Option<String>,
    /// W3C SCXML 6.4 / test229: `true` iff any `<invoke>` in the document
    /// carries `autoforward="true"`. Drives codegen of the
    /// `forward_to_autoforward_children` helper + its call site in the
    /// external-dequeue branch of `process_event_queues`. Set in
    /// [`crate::parser::SCXMLParser::parse_invoke`] alongside the per-invoke
    /// `autoforward` field so the helper is only emitted when at least one
    /// invoke needs it (rest of the corpus stays byte-identical).
    pub has_autoforward_invoke: bool,
    /// SCE_MESH.md §9.6 — codegen-shape seam. `true` when the machine
    /// must emit the non-templated child shape (no `ParentStateMachine`
    /// template param, no `parent_` pointer field, `#_parent` routed
    /// through `performMeshSend`). Derived by [`crate::analyzer::analyze`]
    /// as `has_parent_communication && !is_remote_invoke_target`. The
    /// two shapes converge once §9.6 adopts a mesh-callback shim for
    /// local invokes; until then this flag keeps W3C local-invoke tests
    /// (test233/338) on the existing template shape while the mesh
    /// worker emits the default-constructible shape required by
    /// `ChildSessionAdapter<Engine>` (see §9.6 child session lifecycle).
    pub needs_parent_template: bool,
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
    /// True iff a `<script src="...">` element appeared in the source
    /// document but could not be loaded because the parser had no
    /// filesystem access (WASM-style `parse_string`). The script is not
    /// pushed to [`Self::global_scripts`] — its contents are unknown —
    /// so this is the only trace that the document carries executable
    /// script. [`crate::script_engine_analyzer`] reads it as the
    /// [`crate::script_engine_analyzer::NeedsScriptEngineCause::UnresolvedExternalScript`]
    /// cause. Not serialised: template consumers read `global_scripts`
    /// (which is empty in this case) and `needs_script_engine`.
    #[serde(skip)]
    pub has_unresolved_external_script: bool,

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
    //
    // `default` keeps the AST-export v1 schema consistent with the
    // wire form: schemars marks a field with `skip_serializing_if`
    // but no `default` as required, which contradicts the actual
    // emit behaviour (empty Vec is skipped). Adding `default` aligns
    // the schema with the wire — matches the convention every other
    // skip-when-empty field on `SCXMLModel` already follows.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inline_kinds: Vec<InlineKind>,

    // Path info — environment-specific paths populated by
    // `resolve_source_path` (lib.rs) and `compute_scxml_base_path`
    // (analyzer.rs). Marked `default` for the same schema-alignment
    // reason as `inline_kinds`: the AST-export path emits
    // post-analyzer + pre-deploy-mutation, so `scxml_source_path`
    // (set in the codegen-prep stage) is empty there and the schema
    // treats it as optional.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub scxml_source_path: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
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
    pub needs_namelist_helper: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub needs_event_type_helper: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub needs_event_scheduler: Option<bool>,
    /// W3C SCXML B.2: any reachable `<data>` content / `<data src=...>`
    /// loaded payload / `<send><content>` literal whose first non-WS
    /// character is `<` triggers the host-side XML DOM helper (C11
    /// backend only — cpp / Rust / Go / Kotlin handle the same surface
    /// through their own pipelines). Gates `lua_dom_register_metatable`
    /// emission in scriptengine.jinja2 and the XML branches in the
    /// var.content / send.content / event-promotion sites.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub needs_dom_helper: Option<bool>,

    /// SCE_MESH.md §14 rule 12: set to `true` when this machine is
    /// listed under any `partitions.<name>.machines:` entry in a
    /// `--deploy` deploy.yaml. Gates the delegation from
    /// `entry_exit_actions.jinja2`'s inline `<parallel>`-final branch
    /// to `mesh/cpp/parallel_final.jinja2`. The delegate's body now
    /// also inspects [`Self::partition_parallel_roles`] to select
    /// between single-partition fallback, root-branch, and non-root
    /// branch emission — so a partition-listed machine built without a
    /// `--partition` argument still produces P0-compatible code
    /// (role map empty ⇒ every parallel falls through to the
    /// single-partition branch).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub partition_context_present: bool,

    /// SCE_MESH.md §14 rule 12: per-`<parallel>` role assignment for
    /// the partition currently being codegen'd, keyed by `<parallel>`
    /// element id. Populated by
    /// [`crate::inject_partition_context_flag`] **only** when a
    /// `--partition <name>` argument is supplied — a partitioned
    /// machine built without a partition argument keeps this map
    /// empty, and the `mesh/cpp/parallel_final.jinja2` template falls
    /// through to the single-partition branch (preserving P0 output
    /// for fixtures that exercise the scaffolding hook alone). A
    /// `<parallel>` id absent from the map renders as single-partition
    /// regardless of whether other parallels in the same machine are
    /// distributed.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub partition_parallel_roles: std::collections::BTreeMap<String, PartitionRole>,

    /// SCE_MESH.md §16.5 wire-21 outbound routes for the partition
    /// currently being codegen'd. Maps each `<parallel>` whose role
    /// here is [`PartitionRole::NonRoot`] to the **destination
    /// partition name** — the partition that claims the parallel's
    /// root via `partitions.<root>.hosts_parallel_roots:`. Codegen
    /// uses this to pick the outbound shm channel inside
    /// `sendParallelRegionDone(parallel_id, ...)`. Empty when the
    /// partition is pure-Root, has no NonRoot parallels, or codegen
    /// runs without `--partition`.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub partition_wire21_outbound_routes: std::collections::BTreeMap<String, String>,

    /// SCE_MESH.md §16.5 wire-21 inbound source partitions for the
    /// partition currently being codegen'd. Sorted, deduplicated list
    /// of NonRoot partitions whose region-final entries forward
    /// `ParallelRegionDone` envelopes here (this partition Roots one
    /// or more parallels they host). Codegen materializes one inbound
    /// shm channel per source for [`crate::inject_partition_context_for`]
    /// to thread through. Empty when this partition Roots no
    /// distributed `<parallel>`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub partition_wire21_inbound_sources: Vec<String>,

    /// The current partition's name as supplied to `--partition`.
    /// `None` when codegen runs without `--partition`. Codegen reads
    /// this to disambiguate channel name constants for shared
    /// templates (e.g. the wire-21 channel name encodes both source
    /// and destination partition names).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partition_self_name: Option<String>,

    /// SCE_MESH.md §9.6.2 wire-14 outbound peers — deploy.yaml machine
    /// names referenced by any `<invoke type="scxml" src="#peer">` in
    /// this machine's SCXML that the classifier (`classify_remote_scxml_invokes`)
    /// marked with `remote_mesh_target`. Sorted by `.name`, deduplicated.
    /// Each entry also carries the author-declared `transport:` from the
    /// parent's `bindings["#<peer>"]` entry (cross-device case) or `None`
    /// (same-device implicit shm). Codegen materializes one outbound
    /// `ScxmlInvokeChannel` (shm) per entry for the §9.6.2 `InvokeStart`
    /// envelope, paired with an inbound `ScxmlInvokeChannel` for the
    /// peer's wire-20 `InvokeError` reply. Empty when the machine issues
    /// no remote SCXML invokes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scxml_remote_outbound_peers: Vec<ScxmlRemotePeerBinding>,

    /// SCE_MESH.md §9.6.2 wire-14 inbound peers — deploy.yaml machine
    /// names of other machines whose SCXML issues a remote `<invoke
    /// type="scxml" src="#<this>">`, with `<this>` equal to the machine
    /// currently being codegen'd. Sorted by `.name`, deduplicated.
    /// The `transport:` field mirrors the parent's `bindings["#<this>"]`
    /// entry (cross-device case) or `None` (same-device implicit shm);
    /// the symmetry is required so the worker-side codegen can emit the
    /// matching transport channel when Session 2 extends past shm.
    /// Codegen materializes one inbound `ScxmlInvokeChannel` per entry
    /// for receiving wire-14 `InvokeStart`, paired with an outbound
    /// `ScxmlInvokeChannel` for answering with wire-20 `InvokeError`
    /// carrying `SESSION_F_NOT_IMPLEMENTED` until the child-session
    /// runtime lands. Empty when no peer invokes this machine.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scxml_remote_inbound_peers: Vec<ScxmlRemotePeerBinding>,

    /// SCE_MESH.md §9.6 — `true` when this machine serves as a remote
    /// `<invoke type="scxml" src="#<this>">` target for at least one
    /// sibling machine in the deploy.yaml topology (equivalently, when
    /// [`Self::scxml_remote_inbound_peers`] is non-empty). Derived in
    /// [`crate::collect_scxml_remote_peers`] and consumed by
    /// [`crate::analyzer::analyze`] to compute
    /// [`Self::needs_parent_template`] and by `actions/send.jinja2` to
    /// route `<send target="#_parent">` through
    /// [`StaticExecutionEngine::performMeshSend`] instead of the
    /// local `parent_` pointer path. `ChildSessionAdapter<Engine>`
    /// (§9.6 child session lifecycle) default-constructs the engine,
    /// which requires the non-templated shape whenever a machine is
    /// used as an `<invoke>` worker.
    pub is_remote_invoke_target: bool,

    /// SCE_MESH.md §16.5 L3500 barrier-timeout runtime. Maps each
    /// `<parallel>` id **Rooted by the current partition** to the
    /// deploy-declared `partitions.<root>.barrier_timeout_ms:` value.
    /// An entry absent from the map is the W3C normative infinity
    /// (no finite timer armed at first region completion); an entry
    /// present ⇒ the generated Root SM installs `TimerHooks` on the
    /// matching `ParallelCompletionTracker`.
    ///
    /// Populated by [`crate::inject_partition_context_for`] whenever
    /// the selected partition declares `barrier_timeout_ms:` AND
    /// claims at least one root here. NonRoot partitions never see an
    /// entry (they hold no tracker). Empty in every non-partitioned
    /// build.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub partition_barrier_timeouts: std::collections::BTreeMap<String, u32>,

    /// SCE_MESH.md §16.4 / §16.7 liveness opt-in. `true` when the deploy
    /// declares a `liveliness:` block on the machine being codegen'd,
    /// regardless of whether this is a partition binary. Populated by
    /// [`crate::inject_partition_context_for`] from `deploy.yaml`.
    ///
    /// The flag drives only the codegen-side observability gate
    /// [`crate::generator::reject_liveliness_without_handler`]
    /// (`feedback_silently_broken_hooks`): a machine declaring
    /// `liveliness:` without an `error.communication` handler in its
    /// SCXML is rejected at codegen because the row 8 `PEER_PARTITIONED`
    /// raise (on any machine with `liveliness:`) and the row 13
    /// `REGION_PARTITIONED` raise (on a partitioned machine) both flow
    /// through `error.communication` and would be silently discarded.
    /// Transport emission is a different signal: per-partition row-13
    /// token wiring is keyed on [`Self::partition_self_name`], and
    /// machine-level row-8 token wiring lives in the mesh transport
    /// codegen that reads `deploy.yaml`'s `liveliness:` directly.
    ///
    /// `false` when the machine declares no `liveliness:` section.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub machine_liveliness_opt_in: bool,

    /// Watching-zenoh RFC §5.O Atomic 0: post-preprocessor source
    /// position of the `<scxml>` root element. Populated by
    /// [`crate::parser::SCXMLParser::parse_impl`]. Drives the
    /// per-backend SCE-MAP marker above the generated state machine's
    /// top-level definition (`impl <SmName>` / `class <SmName>` /
    /// `struct <SmName>` / `fn main` for procedure-test wrappers).
    /// XInclude / sce:template expanded content gets the position in
    /// the *included* source via the preprocessor coordinate map.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_location: Option<SourceLocation>,
}

/// SCE_MESH.md §14 rule 12 — partition's role for a specific
/// `<parallel>`. A machine's partitions each hold one of these per
/// `<parallel>` the machine declares. Only surfaced when codegen is
/// invoked with `--partition <name>`; absent (map-missing) implies
/// single-partition fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub enum PartitionRole {
    /// This partition claims the `<parallel>` root via
    /// `hosts_parallel_roots:`. Owns the §16.5
    /// `ParallelCompletionTracker` and raises `done.state.<parallel_id>`
    /// into the SM's local external queue on threshold.
    Root,
    /// This partition hosts one or more regions of the `<parallel>`
    /// but is not the root. Emits wire-21 `ParallelRegionDone`
    /// envelopes on local region-final entry; holds no tracker.
    NonRoot,
    /// All regions of this `<parallel>` live in this same partition —
    /// equivalent to a single-process parallel. The template uses the
    /// legacy `ParallelCompletionHelper` path (P0 body).
    SinglePartition,
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
