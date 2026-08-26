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
use crate::provenance::{RequirementId, SpecProvenance, UnresolvedMarker};

/// W3C SCXML default namespace URI (§scxml-3.5). Mirrors
/// [`crate::forge::model::SCE_NAMESPACE`] for the sce: extension axis —
/// used by the parser's element-dispatch helpers to filter children
/// by namespace as well as local name, closing the foreign-NS local-name
/// collision footgun previously documented in `SCE_FORGE.md` §3.1.
pub const SCXML_NAMESPACE: &str = "http://www.w3.org/2005/07/scxml";

/// §scxml-3.3: Transition element
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
    /// The boolean [`cond`](Self::cond) has at build time, when the
    /// frontend can decide it without the data model.
    ///
    /// `Some` is what makes a guard emittable with no script engine, so
    /// it and [`crate::parser::check_expression_needs`] answer one
    /// question and must not drift: a backend prints its own `true` /
    /// `false` from this, and printing the author's text instead is the
    /// defect the field exists to end — `cond="1"` reached Rust as
    /// `if 1 {`.
    pub cond_constant: Option<bool>,
    #[serde(rename = "type")]
    pub transition_type: String,
    pub actions: Vec<Action>,
    pub needs_string_matching: bool,
    pub matching_enum_values: Vec<String>,
    /// Original index within parent state's transition list. This index is
    /// unique only WITHIN the parent state, never machine-wide — see
    /// [`native_payload_guard`](Self::native_payload_guard) for why that
    /// distinction matters.
    pub transition_index: usize,
    /// EventSchema native lowering: the native typed `_event.data` guard expression
    /// for this transition, lowered at generate time from `cond` against the
    /// imported EventSchema (see `forge::generator::build_*_event_payload`).
    /// Empty when the guard stays on the script-engine / In() path or no
    /// schema applies.
    ///
    /// The guard is per-transition derived data, so its home is the
    /// transition that owns it — exactly like [`cond_cpp`](Self::cond_cpp).
    /// It is deliberately NOT a machine-global side table keyed by
    /// `transition_index`: that index is per-state, so two transitions in
    /// different states share a key and the last writer wins, silently
    /// miscompiling every colliding guard. Storing it here makes that
    /// collision unrepresentable.
    ///
    /// Transient by design: the language-agnostic parse model leaves it empty
    /// (skipped in serialized AST exports — the persistent IR stays neutral,
    /// per `build_rust_event_payload`'s render-context principle). Each
    /// single-language generate pass clones the model and populates it for
    /// that one language.
    ///
    /// Excluded from the stable AST export contract (`schemars(skip)`): it is
    /// a transient codegen artifact, never present on the parsed model that
    /// `--emit-ast` serializes, so it is not part of the language-agnostic IR
    /// schema.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(test, schemars(skip))]
    pub native_payload_guard: String,
    /// The canonical path of the
    /// state that OWNS this transition, paired with
    /// [`symbol_artifact`](Self::symbol_artifact) to form the identity
    /// half of the transition's mangled symbol.
    ///
    /// Stamped by `forge::symbol_mangling::stamp_symbol_attribution`
    /// from the same walk that builds the sourcemap, so the SCE-MAP
    /// marker a template emits and the sidecar row a consumer resolves
    /// it against cannot disagree. It rides on the transition rather
    /// than being recomputed at the render site because the Kotlin
    /// backend renders ancestor transitions under a descendant's arm —
    /// the arm is not the owner.
    ///
    /// Transient by design, exactly like
    /// [`native_payload_guard`](Self::native_payload_guard): the parse
    /// model leaves it empty and each generate pass stamps its own
    /// clone, so the `--emit-ast` export stays the language-agnostic IR.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(test, schemars(skip))]
    pub symbol_state_path: String,
    /// This transition's artifact
    /// label (`_transition_<idx>`, indexed within the owning state's
    /// transition list). See
    /// [`symbol_state_path`](Self::symbol_state_path).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(test, schemars(skip))]
    pub symbol_artifact: String,
    /// §scxml-3.11: History target if transition targets a history state
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history_target: Option<String>,
    /// §scxml-3.11: Resolved leaf target for history default (Kotlin backend)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history_leaf_target: Option<String>,
    /// Prefix matching events for Kotlin templates
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prefix_matching_events: Vec<String>,
    /// §scxml-3.13: True internal transition (target is descendant of source)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_true_internal: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub internal_source: Option<String>,
    /// SCE Protocol-Synthesis RFC §synth-5-O source traceability: post-preprocessor source
    /// position of the `<transition>` element, populated by
    /// [`crate::parser::SCXMLParser::parse_transition`] from
    /// `roxmltree::Document::text_pos_at`. Templates emit
    /// per-backend SCE-MAP markers above the transition's emitted
    /// handler function (Rust `// SCE-MAP:` + `#[doc]`, C/C++
    /// `#line`, Go `//line`, Kotlin `// SCE-MAP:`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_location: Option<SourceLocation>,
    /// `sce:req` requirement IDs attached to this transition. Sorted
    /// in document order (whitespace-tokenised from the attribute).
    /// Empty when the attribute is absent — preserves byte-identical
    /// output for inputs without traceability metadata.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub req: Vec<RequirementId>,
    /// `sce:provenance` spec-document anchors attached to this
    /// transition. Multiple allowed; element form lets one node
    /// anchor at multiple documents.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub provenance: Vec<SpecProvenance>,
    /// `sce:unresolved` placeholder markers attached to this
    /// transition. Detected by the parser, propagated as a comment
    /// by codegen, rejected by `--strict`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unresolved: Vec<UnresolvedMarker>,
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

    /// `true` when [`Self::send_type`] is a literal naming an Event I/O
    /// Processor this build has no delivery path for, so the emitted
    /// code raises `error.execution` at this site instead of sending
    /// (§scxml-6.2).
    ///
    /// Decided once, by
    /// [`crate::host_processor_analyzer::is_supported_send_type`], and
    /// read by every backend's send template. The templates used to
    /// re-derive it from a literal list of accepted URIs spelled inline
    /// — five copies of one set, which is five chances for a backend to
    /// disagree with the others about the same document. It is also the
    /// point a host-supplied processor has to flip, and a decision
    /// spelled five times has no such point.
    ///
    /// Says nothing about `typeexpr`. A runtime-resolved type has no
    /// build-time value, and each backend already decides for itself how
    /// a literal `type` beside a `typeexpr` is treated; this flag
    /// replaces the duplicated accepted-set literal without moving that
    /// boundary.
    #[serde(default)]
    pub send_type_unsupported: bool,

    /// `true` when [`Self::send_type`] names a processor the *host* has
    /// declared it serves (§scxml-6.2.5 makes the identifier extensible,
    /// so the set is open to the platform).
    ///
    /// Mutually exclusive with [`Self::send_type_unsupported`]: a
    /// declaration moves a site from one to the other, and a template
    /// branching on both cannot emit a refusal and a dispatch for one
    /// `<send>`. Set by
    /// [`crate::host_processor_analyzer::declare_host_processors`] after
    /// the parse, so every backend renders from one decision.
    #[serde(default)]
    pub send_type_host_served: bool,

    pub label: String,
    // if/elseif/else
    pub cond: String,

    pub cond_cpp: String,

    pub cond_kt: String,
    #[serde(default)]
    pub is_pure_in_predicate: bool,
    /// The boolean [`cond`](Self::cond) has at build time — the same
    /// field [`Transition::cond_constant`] carries, for the `<if>` a
    /// backend emits through the same arms.
    #[serde(default)]
    pub cond_constant: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub then_actions: Vec<Action>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub elseif_branches: Vec<ElseIfBranch>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub else_actions: Vec<Action>,
    // foreach
    pub array: String,

    pub item: String,

    pub index: String,
    /// foreach body / transition actions
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<Action>,
    // send params
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<Param>,
    // cancel
    pub sendid: String,

    pub sendidexpr: String,
    // send param static optimization
    #[serde(default)]
    pub is_static_literal: bool,

    pub static_value: String,
    // Named Context: native code actions
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub content_transformed: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub content_kt: String,
    #[serde(default)]
    pub is_cpp_function: bool,
    #[serde(default)]
    pub is_kt_function: bool,
    /// §scxml-G-7 — Custom Action Element `<sce:action name="...">`.
    /// The symbolic host-operation name this action dispatches to. When
    /// non-empty, [`action_type`](Self::action_type) is `"native_action"`
    /// and [`params`](Self::params) carries the positional `<sce:arg
    /// expr="...">` argument expressions. The codegen lowers each argument
    /// through the typed-expression pipeline (the same path guards use) and
    /// emits a direct call into a generated host-`Actions` trait method —
    /// no script engine. Empty for every standard W3C executable-content
    /// action.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub native_action_name: String,
    /// Codegen-internal: the fully-rendered backend call site for this
    /// `<sce:action>` (e.g. the Rust `self.actions.op(&ev.field);` wrapped
    /// in its payload-variant binding). Computed by the per-backend native
    /// lowering pass on the cloned codegen model and read by the
    /// `actions/native_action` template. Never set on the parsed model, so
    /// it is absent from the AST export and the serialized wire form.
    /// Excluded from the AST JSON Schema (`schemars(skip)`): a backend
    /// codegen scratch field, not part of the public AST contract.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(test, schemars(skip))]
    pub native_action_rendered: String,

    // SCE_MESH.md §13 — mesh metadata is not carried on individual
    // <send> actions. Communication pattern is inferred from event name
    // conventions (mesh::pattern), RPC reply pairing is inferred from
    // topology structure (mesh::topology::detect_rpc_pairs), and QoS is
    // a transport binding concern (deploy.yaml).
    /// SCE Protocol-Synthesis RFC §synth-5-O source traceability: post-preprocessor source
    /// position of the executable-content element this action
    /// represents (`<raise>` / `<send>` / `<assign>` / `<log>` /
    /// `<script>` / `<if>` / `<foreach>` / `<cancel>`). Populated by
    /// [`crate::parser::SCXMLParser::parse_executable_content_single`].
    /// Carries per-action attribution detail beyond the function-level
    /// SCE-MAP markers the templates emit; today this field is read by
    /// the pre-emit `validate_emission_provenance` walker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_location: Option<SourceLocation>,
    /// `sce:req` requirement IDs attached to this executable-content
    /// element. See [`Transition::req`] for the wire-format contract.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub req: Vec<RequirementId>,
    /// `sce:provenance` spec-document anchors attached to this
    /// executable-content element.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub provenance: Vec<SpecProvenance>,
    /// `sce:unresolved` placeholder markers attached to this
    /// executable-content element.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unresolved: Vec<UnresolvedMarker>,
}

/// W3C SCXML if/elseif branch
#[derive(Debug, Clone, Serialize, Default)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct ElseIfBranch {
    pub cond: String,
    pub cond_cpp: String,
    pub cond_kt: String,
    pub is_pure_in_predicate: bool,
    /// The boolean [`cond`](Self::cond) has at build time — see
    /// [`Transition::cond_constant`]. An `<elseif>` reaches the same
    /// arms its `<if>` does, so it needs the same answer.
    pub cond_constant: Option<bool>,
    pub actions: Vec<Action>,
}

/// §scxml-6.2.4: Send parameter
#[derive(Debug, Clone, Serialize, Default)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct Param {
    pub name: String,
    pub expr: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub location: String,
    pub is_static_literal: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub static_value: String,
    /// The `<param>` element's own position.
    ///
    /// Distinct from `location` above, which is the W3C
    /// `<param location="X"/>` attribute (a data-model expression).
    /// This is the coordinate a rejection of *this* param reports:
    /// SCE_ERROR_CONTRACT.md §2.2 has the consumer open `location.file`
    /// and edit at `location.line`, so a diagnostic naming a param's
    /// value has to point at the param, not at the enclosing `<send>`
    /// — that line does not contain the value being rejected.
    /// `Variable::source_location` exists for the same reason on the
    /// `<data>` side.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_location: Option<SourceLocation>,
}

/// §scxml-5.2: Datamodel variable
#[derive(Debug, Clone, Serialize, Default)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct Variable {
    pub id: String,
    pub expr: String,
    pub src: String,
    pub content: String,
    /// The `<data>` element's position, so a build that must stay
    /// pure-static can anchor a `datamodel-variable-init` script-engine
    /// cause on the line that costs it (SCE_ERROR_CONTRACT.md §10.4).
    pub source_location: Option<SourceLocation>,
    /// Classified type: int, string, bool, runtime
    #[serde(rename = "type")]
    pub var_type: String,
}

/// §scxml-3.11: History state information
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

/// §scxml-5.7: Done data for final states.
///
/// `content` is a [`DoneDataContent`] sum so that the parser decision —
/// `<content expr="...">` vs `<content>literal</content>` vs omitted — is
/// encoded in the type and survives into templates as a `{kind, text}`
/// object. Templates dispatch on `donedata.content.kind` rather than the
/// legacy truthy-check on two parallel strings.
#[derive(Debug, Clone, Serialize, Default)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct DoneData {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<DoneDataParam>,
    pub content: DoneDataContent,
}

/// §scxml-5.5: `<content>` body semantics.
///
/// - [`DoneDataContent::None`] — no `<content>` child (`{"kind":"none"}`).
/// - [`DoneDataContent::Expression`] — `<content expr="X"/>`, MUST be
///   evaluated against the active datamodel (`{"kind":"expression","text":"X"}`).
/// - [`DoneDataContent::InlineText`] — `<content>text</content>` under a
///   data model that has a value expression language
///   (`{"kind":"inline_text","text":"..."}`).
/// - [`DoneDataContent::Literal`] — `<content>inline text</content>` under
///   the Null data model; per §scxml-5.5 the children are used **as the
///   content value**, not re-evaluated as an expression. Literal means no
///   script engine required at runtime (`{"kind":"literal","text":"..."}`).
///
/// # Why inline text is not the `expr` attribute
///
/// Both used to be [`DoneDataContent::Expression`], and the rule that
/// belongs to only one of them was applied to both. `<content expr="X"/>`
/// says *evaluate this*, so text that cannot be evaluated is an error the
/// runtime reports rather than one the build refuses, and W3C test 344 is
/// in the suite to check it. Inline text says *this is the value*, and the
/// ECMAScript data model appendix gives it a second reading when the first
/// does not apply: content that is neither XML nor JSON **is a string**.
/// Collapsing the two made
/// `<content>inline payload</content>` generate an expression that could
/// only ever raise, so a document whose payload was ordinary prose reached
/// `error.execution` instead of carrying its prose.
///
/// [`Variable`] has taken the two readings apart since the inline-`<data>`
/// path moved its decision to generation time; this variant is what lets
/// `<donedata>` reach the same answer through the same filter.
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
    InlineText(String),
    Literal(String),
}

/// §scxml-5.7: Done data parameter.
///
/// `expr` and `location` are `Option<String>` because §scxml-6.2.4 mandates
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
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub cpp_type: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub cpp_include: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub kt_type: String,
}

/// An invoke declared on a state, keyed by kind.
///
/// §scxml-6.4 leaves `<invoke type="...">` open for implementation-defined
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
    Unsupported(UnsupportedInvokeInfo),
}

/// §scxml-6.4.1: an `<invoke>` whose `type` names no processor this
/// platform implements.
///
/// The spec defines this case rather than leaving it undefined — the
/// processor "MUST place error.execution on the internal event queue" —
/// so a document carrying one is *valid SCXML with defined meaning*, not
/// an author error the compiler may reject. That single raise at invoke
/// time is the whole observable: no child session starts, no
/// `done.invoke.<id>` ever fires, and there is nothing to cancel on state
/// exit. The struct therefore carries only the W3C identity and the
/// refused type string.
///
/// The variant exists so the case cannot be dropped silently. Before it,
/// `parse_invoke` returned `Ok(None)` for an unsupported type and the
/// `<invoke>` vanished from the model, leaving AOT with no observable at
/// all where the Interpreter raised one.
#[derive(Debug, Clone, Serialize, Default)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct UnsupportedInvokeInfo {
    #[serde(flatten)]
    pub base: InvokeBase,
    /// The `type` attribute verbatim. Reported in the raised
    /// `error.execution` payload so the author sees which URI was refused
    /// rather than a generic failure.
    pub invoke_type: String,
    /// `<invoke src="...">` verbatim, empty when the document named none.
    ///
    /// Inert for a refused invoke — nothing resolves it — but a host that
    /// RUNS the type needs it: `src` is how §scxml-6.4.1 lets the document
    /// say *what* to invoke, and dropping it would leave a host invoker
    /// able to receive the request and not the thing it names.
    #[serde(default)]
    pub src: String,
    /// `true` when the host has declared it serves
    /// [`Self::invoke_type`] (§scxml-6.4.1 leaves the set of invokable
    /// types to the platform, exactly as §scxml-6.2.5 does for `<send>`).
    ///
    /// The variant stays `Unsupported` rather than becoming a third kind:
    /// what a declaration changes is not the classification but the
    /// lowering. Both shapes ride the same deferred-invoke queue and the
    /// same entry chain; only the execute step differs — a raise for one,
    /// a dispatch into the host's invoker for the other. Set by
    /// [`crate::host_processor_analyzer::declare_host_processors`].
    #[serde(default)]
    pub host_served: bool,
}

/// Fields every invoke carries regardless of kind: the W3C identity
/// (`invoke_id`, `state_name`), the declared `<param>` payload, and the
/// optional `idlocation` attribute. Sits at the base of the invoke type
/// hierarchy and is flattened into each wrapping struct's serialisation.
///
/// `invoke_id` is preserved verbatim — including the leading underscore on
/// auto-generated ids — because §scxml-6.4.1 surfaces it in event names
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
    /// `sce:req` requirement IDs attached to this `<invoke>`. See
    /// [`Transition::req`] for the wire-format contract.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub req: Vec<RequirementId>,
    /// `sce:provenance` spec-document anchors attached to this
    /// `<invoke>`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub provenance: Vec<SpecProvenance>,
    /// `sce:unresolved` placeholder markers attached to this
    /// `<invoke>`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unresolved: Vec<UnresolvedMarker>,
    /// The `<invoke>` element's position, so an invoke-anchored
    /// script-engine cause can name the line (SCE_ERROR_CONTRACT.md §10.4).
    pub source_location: Option<SourceLocation>,
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
    /// §scxml-6.4: Use specific done.invoke.{id} event instead of generic done.invoke
    #[serde(default)]
    pub use_specific_event: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub child_datamodel_vars: Option<Vec<String>>,
    /// §scxml-6.4 (test226/240/241/243/244/245/276): the child SCXML
    /// has at least one `<send target="#_parent" event="..."/>`. Codegen
    /// uses this to gate parent_sm / parent_dispatch wiring at child
    /// spawn time so parent-routed events reach the parent's external
    /// queue. Mirrors the C++ reuse of `child_model.has_parent_communication`
    /// surfaced via the parsed child model — populated by
    /// `collect_child_to_parent_events` during invoke metadata
    /// resolution.
    #[serde(default)]
    pub child_has_send_to_parent: bool,
    /// §scxml-6.2 (test207): the child SCXML carries a non-empty
    /// `<send delay="...">` so its codegen emitted a scheduler queue +
    /// a `_tick` entry point. Parents that drive active children
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
/// §scxml-6.4 requires exactly one of `src` / `srcexpr`; this sum type
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

/// §scxml-6.4: Static invoke (`<invoke src="..."` or inline `<content>`).
///
/// Holds only the fields the static lifecycle actually uses. Hybrid-only
/// fields (`srcexpr`, `contentexpr`) do not appear here — the type system
/// guarantees `Invoke::Scxml` never carries them. Common metadata lives on
/// [`InvokeSessionCommon`] and is accessible via `Deref` so
/// `scxml_info.invoke_id` still reads naturally.
///
/// Inline `<content><scxml>` is resolved during parse (see
/// `SCXMLParser::parse_invoke`); the parser pre-parses the inline child
/// into [`Self::inline_child`] and rewrites `src` to the canonical
/// `#<synth_name>` Mesh peer reference (SCE_MESH.md §9.6.6 rules 1-2).
/// No disk side-effect: synth-invoke children no longer materialise as
/// sibling `.scxml` files in the parent's source directory; the §9.6.6
/// naming convention is enforced by codegen emit when applicable, not by
/// a parser write. External `src="file.scxml"` invokes keep
/// `inline_child = None` and resolve through disk at codegen time.
#[derive(Debug, Clone, Serialize, Default)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct ScxmlInvokeInfo {
    #[serde(flatten)]
    pub common: InvokeSessionCommon,
    pub finalize_content: String,
    pub src: String,
    pub namelist: String,
    /// Pre-parsed inline-`<content>` child model. `Some(model)` when the
    /// invoke carried `<content><scxml>…</scxml></content>` and the
    /// parser captured the child as a structured submodel; `None` for
    /// external `src="…"` invokes (codegen reads those from disk via
    /// [`InvokeSessionCommon::child_name`]). `#[serde(skip)]` keeps this
    /// field out of the Forge AST wire format and the JSON schema — it
    /// is parser↔codegen internal state, not part of the published IR.
    #[serde(skip)]
    #[cfg_attr(test, schemars(skip))]
    pub inline_child: Option<Box<SCXMLModel>>,
    /// Raw SCXML text the parser wrapped for [`Self::inline_child`]
    /// (`<?xml version="1.0"?>` prologue + `<scxml>…</scxml>` body
    /// with the W3C namespace stamped on the root). Co-populated with
    /// `inline_child` (both `Some` or both `None`) so codegen can
    /// re-materialise the synth SCXML next to the parent's `_sm.*`
    /// in `-o` when a downstream consumer needs it on disk:
    /// `--deploy` topology iteration (`inject_partition_context_for`
    /// parses every declared machine by name), CMake's stage-3 synth
    /// codegen (`tests/CMakeLists.txt:2442-2456`), and W3C
    /// `process_children_<N>.cmake` (`--as-child` per child SCXML).
    /// Re-emit lands in the caller-controlled `-o`, never in the
    /// parent's source directory — the bug-report pollution case stays
    /// closed.
    #[serde(skip)]
    #[cfg_attr(test, schemars(skip))]
    pub inline_child_xml: Option<String>,
    /// SCE_MESH.md §9.6 remote `<invoke type="scxml">`. When `src` is of the
    /// form `#<name>` and `<name>` matches a distinct mesh machine declared
    /// in `deploy.yaml`, this carries that machine name (without the leading
    /// `#`). C++ codegen branches on this field to emit the §9.6.2 wire-14
    /// `InvokeStart` dispatch, falling back to the §10.7.1
    /// `SESSION_F_TRANSPORT_UNAVAILABLE` raise when the deployment attaches
    /// no transport binding to that peer. `None` for local W3C invokes (inline
    /// `<content>` or file-relative `src`) and for non-mesh builds — those
    /// flow through the existing local-invoke path unchanged. Populated
    /// by [`crate::inject_partition_context_for`] from the deploy topology.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_mesh_target: Option<String>,
    /// SCE_MESH.md §9.6 cross-device transport — the author-declared
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

/// §scxml-6.4: Hybrid invoke (runtime `srcexpr`/`contentexpr`).
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

/// Top-level `<sce:driver href="..."/>` declaration on the SCXML
/// root. References an externally-authored driver header that the C11
/// backend `#include`s into the emitted `*_sm.c`. Author-time
/// reference only; SCE does not parse the driver header itself —
/// cross-TU signature verification is delegated to the C compiler.
///
/// Resolution happens at compile-model time: each `href` is joined
/// against `deploy.yaml`'s `platform.driver_root` (or the SCXML file's
/// parent directory as fallback), and an unresolved path surfaces
/// `mcu/driver-header-not-found`. Only the C11 backend consumes this
/// field for codegen emission.
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_path: Option<String>,
    /// 0-based document order within the SCXML root's `<sce:driver>`
    /// declarations. Lets diagnostics quote a stable index when the
    /// source file lacks reliable line numbers.
    pub document_order: u32,
    /// Source location of the `<sce:driver>` element (post-preprocessor
    /// row/col). `None` when the parser cannot reconstruct the position
    /// — diagnostics fall back to file-level anchoring in that case.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_location: Option<SourceLocation>,
}

/// Closed-set vocabulary for the cross-document session-FSM role
/// that a SCXML document declares via the top-level
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
/// cross-doc diagnostics.
///
/// `kind` values use the canonical session-FSM vocabulary
/// (`accept-side`), NOT the wire-side deploy vocabulary
/// (`listener`). The asymmetric pairing is hardcoded in the
/// orchestrator — each domain uses its native
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

/// `<sce:on-sample>` SCXML extension (SCE Protocol-Synthesis RFC §synth-5-E). Wraps
/// the RFC's subscriber callback contract ("the subscriber callback
/// receives `&Sample<'_>`; the slot returns to the pool when the
/// callback returns") into a state-level declaration that the codegen
/// template lowers to a per-state callback registration on the link's
/// RX path. Valid only inside `<state>` and `<parallel>`. `link`
/// (forge link kind artifact name) + `event` (SCXML event name
/// dispatched on Sample arrival) are both required; `callback` is an
/// optional `rust:` prefix-typed reference into the user's symbol
/// space.
#[derive(Debug, Clone, Serialize, Default)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct OnSampleNode {
    /// `link="X"` — forge link kind artifact name. Cross-reference
    /// resolution happens post-parse: the build pipeline looks each
    /// `link=` value up in the
    /// [`crate::forge::cross_doc_registry::SceCrossDocRegistry`]
    /// (see `validate_on_sample_links` in `parser.rs`).
    pub link: String,
    /// `event="X"` — SCXML event name raised when a Sample arrives.
    /// State-level `<transition event="X">` blocks dispatch normally
    /// per §scxml-5.10, with `_event.data` carrying the borrowed
    /// `&Sample<'_>` reference.
    pub event: String,
    /// `callback="rust:crate::path::fn"` — optional extern reference
    /// into the user's symbol space. When
    /// present, codegen emits `path(&sample)` at the dispatch site,
    /// forcing borrow-mode at the call boundary; rustc enforces the
    /// user's signature shape against the borrow contract. The
    /// language prefix today is `rust:` only; future axes (`c:`,
    /// `kotlin:`, …) reuse the same attribute via the same
    /// language-prefixed parsing. Absence (`None`) means codegen
    /// synthesizes a default dispatch shim — the bare
    /// `<sce:on-sample link/event>` shape.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub callback: Option<String>,
    /// 0-based document order within this state's `<sce:on-sample>`
    /// blocks. Lets diagnostics quote a stable per-state index even
    /// when the source file lacks reliable line numbers.
    pub document_order: u32,
}

/// §scxml-3.3: State element
#[derive(Debug, Clone, Serialize, Default)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct State {
    pub id: String,
    pub initial: String,
    pub initial_children: Vec<String>,
    pub is_final: bool,
    pub is_parallel: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
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
    /// Empty for states without sample subscriptions. Multiple blocks
    /// (one per link) are allowed per state, with a
    /// uniqueness validator on the `link` attribute.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub on_sample_blocks: Vec<OnSampleNode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub donedata: Option<DoneData>,
    pub document_order: u32,
    pub initial_transition_actions: Vec<Action>,
    pub initial_history_id: String,
    pub initial_history_default_target: String,
    pub initial_history_default_actions: Vec<Action>,
    /// SCE Protocol-Synthesis RFC §synth-5-O source traceability: post-preprocessor source
    /// position of the `<state>` / `<final>` / `<parallel>` element.
    /// Populated by [`crate::parser::SCXMLParser::parse_states`].
    /// Drives the per-state SCE-MAP marker that codegen templates
    /// emit above the on-entry / on-exit / transition-handler
    /// functions this state lowers to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_location: Option<SourceLocation>,
    /// `sce:req` requirement IDs attached to this state. See
    /// [`Transition::req`] for the wire-format contract.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub req: Vec<RequirementId>,
    /// `sce:provenance` spec-document anchors attached to this state.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub provenance: Vec<SpecProvenance>,
    /// `sce:unresolved` placeholder markers attached to this state.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unresolved: Vec<UnresolvedMarker>,
    /// `sce:unhandled` — the events this state deliberately does not
    /// handle, declared on the state the absence is true of.
    ///
    /// The declaration is what exempts a state from the
    /// NonExhaustiveEventHandling validator. It is deliberately not a
    /// property of the compound parent: the fact being asserted is
    /// "*this* state does not handle *this* event", so a sibling added
    /// later inherits no exemption and is judged on its own, and
    /// deleting the state deletes its exemptions with it.
    ///
    /// Both directions are checked, so the declaration cannot rot into
    /// prose: a state that declares an event it actually handles
    /// rejects via `scxml/contradictory-unhandled-declaration`, and one
    /// that declares an event which is not a gap under its parent
    /// rejects via `scxml/stale-unhandled-declaration`.
    ///
    /// Tokens are literal event names in declaration order. Wildcards
    /// are rejected at parse time — the gap set the declaration is
    /// checked against is always literal, and a second matching
    /// semantics here would let one attribute mean two things.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unhandled: Vec<String>,
}

/// What a model needs to translate its own recorded positions back
/// into the files an author wrote.
///
/// Held on the model rather than applied at parse time because the two
/// consumers of a position want different things and neither is wrong:
/// an SCE-MAP marker wants the artifact spelling (a basename, so a
/// generated tree does not bake in one machine's checkout), while a
/// diagnostic wants a path its reader can open, in the authored file —
/// which after expansion may not even be the document that was parsed.
/// Keeping the mapping lets each ask at the point of use.
#[derive(Debug, Clone, Default)]
pub struct AuthoredPositions {
    /// The expanded document the recorded rows index into.
    pub expanded: String,
    /// Expanded byte range → authored origin.
    pub map: crate::position_map::PositionMap,
}

impl AuthoredPositions {
    /// Resolve an expanded (row, col) to the authored file and
    /// position. Returns `None` when the mapping is the identity, so
    /// callers keep whatever spelling they already had rather than
    /// swapping in an equivalent one.
    pub fn resolve(&self, line: Option<u32>, col: Option<u32>) -> Option<(String, u32, u32)> {
        if self.map.is_identity() {
            return None;
        }
        let (line, col) = (line?, col.unwrap_or(1));
        let offset = crate::position_map::rowcol_to_offset(&self.expanded, line, col);
        let pos = self.map.lookup(offset);
        Some((pos.file.display().to_string(), pos.row, pos.col))
    }

    /// The `<sce:use>` that supplied substituted bytes on the expanded
    /// row `line`, if any.
    ///
    /// A rejection whose value was assembled by parameter substitution
    /// cannot be repaired where it is read: the authored row holds the
    /// template's `target="tick_{$n}"`, and rewriting that rewrites
    /// every expansion of the template, not the one that failed. The
    /// call site is the coordinate that distinguishes them, so it
    /// travels with the record.
    ///
    /// Row granularity rather than the exact value span: the recorded
    /// position is an element's, and an element occupying one row is
    /// the case the substitution rule (`{$name}` inside an attribute
    /// value) produces. A multi-row element would widen this to its
    /// first row, which under-reports rather than mis-reports.
    pub fn call_site_on(&self, line: Option<u32>) -> Option<(String, u32, u32)> {
        if self.map.is_identity() {
            return None;
        }
        let line = line?;
        let start = crate::position_map::rowcol_to_offset(&self.expanded, line, 1);
        let end = crate::position_map::rowcol_to_offset(&self.expanded, line + 1, 1);
        let end = if end <= start {
            self.expanded.len()
        } else {
            end
        };
        let pos = self.map.call_site_within(start, end)?;
        Some((pos.file.display().to_string(), pos.row, pos.col))
    }
}

/// The data model a document declares, restricted to what SCE supports.
///
/// §scxml-3.2 lists the attribute's valid values as `"null"`,
/// `"ecmascript"`, `"xpath"` "or other platform-defined values", and its
/// default as platform-specific. Appendix B adds the obligation: a conformant
/// processor MUST support the null data model and MAY support the others.
///
/// This enum is the set SCE supports, not the set the spec names. `"xpath"`
/// is a legal value SCE does not implement and SCE defines no values of its
/// own, so both are rejected where the attribute is read rather than being
/// carried into the model — an unsupported data model that reaches code
/// generation is a document silently evaluated in a language nobody
/// declared, which is the defect this type exists to make unrepresentable.
/// The two rejections stay distinct because their repairs differ: one is
/// "SCE has not implemented this yet", the other is "no processor defines
/// this at all".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub enum Datamodel {
    /// §scxml-B-1. An absent or empty data model: `In()` is the whole
    /// boolean expression language, and there is no location expression
    /// language, no value expression language, and no scripting language.
    Null,

    /// §scxml-B-2. Declaring this obliges the processor to support the
    /// third edition of ECMAScript.
    ///
    /// The platform-specific default §scxml-3.2 leaves to implementations.
    /// SCE picks ECMAScript because a document that omits the attribute
    /// and then writes `cond="x > 1"` is asking for a value expression
    /// language, and the Null data model has none — defaulting to Null
    /// would reject the common case on a technicality. The choice is made
    /// here, once, so the two engines cannot answer it differently.
    #[default]
    EcmaScript,
}

impl Datamodel {
    /// The attribute spelling, for diagnostics and the manifest.
    pub fn as_str(self) -> &'static str {
        match self {
            Datamodel::Null => "null",
            Datamodel::EcmaScript => "ecmascript",
        }
    }
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
    /// `<sce:import>` declarations on the statechart's `<scxml>` root.
    /// Each entry names a cross-doc dependency by `kind`, `src`, and
    /// `alias` so per-doc validators can compute per-statechart import
    /// visibility (rather than the legacy single global registry that
    /// merged every kind document in the build into one shared map).
    /// Today's primary consumer: per-machine EventSchema visibility
    /// for mesh cross-machine validation and receive/send-side
    /// schema selection on the per-statechart check passes. Empty for
    /// statecharts that declare no `<sce:import>` children — the
    /// schemaless-fallback path stays a no-op walk.
    ///
    /// Serialized into the manifest so downstream tooling (mesh
    /// codegen, AST export) can read the import graph without
    /// re-parsing the SCXML.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub forge_imports: Vec<crate::forge::model::ForgeImport>,
    /// EventSchema MCU native lowering —
    /// the statechart's `<sce:import kind="event-schema">` declarations
    /// resolved to their `EventSchemaModel`, keyed by SCXML event name
    /// (e.g. `"job.completed"`). Populated by [`crate::parser`] during
    /// the file-based parse (when a `base_dir` is available to follow
    /// each import's `src=` to the sibling schema document), *before*
    /// [`crate::script_engine_analyzer`] runs — so the engine-need
    /// analysis sees a transition guard's typed `_event.data.<field>`
    /// surface and can lower it natively instead of routing through the
    /// runtime script engine (which no_std MCU targets lack).
    ///
    /// Empty for statecharts that import no EventSchema, and for the
    /// in-memory `parse_string` path (WASM), which has no sibling files
    /// to resolve — those keep the dynamic `_event.data` String
    /// baseline. Purely an in-memory analysis/codegen aid recomputable
    /// from [`forge_imports`] + the sibling documents, so it is not
    /// serialized into the manifest.
    #[serde(skip)]
    pub imported_event_schemas:
        std::collections::BTreeMap<String, crate::forge::model::EventSchemaModel>,
    pub initial: String,
    pub initial_leaf: String,
    pub binding: String,
    /// The data model this document declares (§scxml-3.2 `datamodel`).
    ///
    /// Typed rather than the raw attribute string so a value SCE cannot
    /// honor has no representation that reaches code generation. See
    /// [`Datamodel`] for what the vocabulary is and why it has exactly
    /// these two members.
    pub datamodel: Datamodel,

    /// SCE Protocol-Synthesis RFC §synth-5-J-2 + §synth-5-L: per-document event-queue
    /// capacity declared via
    /// `<scxml sce:capacity="N">` on the root element. Drives the
    /// generated `EVENT_QUEUE_CAPACITY` bound that the heapless
    /// event-queue uses in `--no-std` emission. Unit: events (not
    /// bytes).
    ///
    /// Resolution rule: per-instance attribute
    /// wins; fallback to deploy.yaml
    /// `machines.<m>.scheduler.default_event_queue_capacity` when
    /// the attribute is absent. Both being absent is permitted
    /// (the emitted machine falls back to the runtime's default
    /// queue bound).
    ///
    /// Schema invariant enforced by the parser: present-but-malformed
    /// (non-numeric, zero, or u32-overflow) surfaces
    /// `validation/invalid-attribute` — no silent coercion.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_queue_capacity: Option<u32>,

    pub states: BTreeMap<String, State>,
    pub events: BTreeSet<String>,
    /// Derived external-ingress event set: the event descriptors that
    /// appear as `<transition event="...">` triggers, with engine-
    /// reserved families excluded (`error.*`, `done.invoke*`,
    /// `done.state*`, `cancel.invoke`, the wildcard sentinels
    /// `*`/`.*`/`_*`, and the eventless empty token).
    ///
    /// This is the contract a transport switchboard (e.g. a pub/sub
    /// key -> domain-event router) validates its injection targets
    /// against: an event raised via `Engine::raise_external_by_name`
    /// is accepted by this machine iff it matches a member per W3C
    /// SCXML 3.12.1 event-descriptor matching. Distinct from the
    /// kitchen-sink `events` set above, which also carries egress
    /// (`<send>`/`<raise>`) and engine-synthesized events. The
    /// reserved-family filter is SCE-owned (mirrors
    /// [`crate::analyzer::add_system_events`]) so downstream tooling
    /// validates against one published set instead of re-deriving the
    /// platform-event taxonomy. Populated in
    /// [`crate::analyzer::compute_external_ingress_events`].
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub external_ingress_events: BTreeSet<String>,
    /// Derived per-event typed-inject seam set: the event descriptors for
    /// which a `<Machine>Inject::raise_<event>(payload)` method is
    /// generated — i.e. events whose transition guard lowers to a native
    /// typed `_event.data.<field>` comparison (no script engine; the
    /// no_std MCU value path). A *subset* of
    /// [`Self::external_ingress_events`]: every member is an
    /// external-ingress event, but ingress events whose guards stay on the
    /// dynamic `_event.data` baseline are absent — an enum-typed schema
    /// field, a mixed datamodel/`In()`/function cond, or an event whose
    /// payload is read only by an `<assign>`/action rather than a
    /// transition guard.
    ///
    /// This is the contract a transport switchboard keys off to decide,
    /// per value binding, whether an event has a typed value path (call
    /// the generated `raise_<event>` inject seam) or only the schemaless
    /// signal path (`Engine::raise_external_by_name`) — so the switchboard
    /// never re-derives the native-lowering eligibility rule. Deliberately
    /// language-neutral: the Rust codegen identifiers
    /// (`raise_<event>` / `<Machine><Variant>Payload` / `<Machine>Inject`)
    /// are resolved from the published `to_snake_case` / `to_event_variant`
    /// filters, NOT carried in this IR, so backend naming never leaks into
    /// the cross-language contract. Populated in
    /// [`crate::analyzer::compute_typed_inject_events`] from the same
    /// [`crate::forge::event_schema_check::select_native_typed_guards`]
    /// selection the codegen payload builders consume.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub typed_inject_events: BTreeSet<String>,
    /// Every event name produced by an internal `<raise event="X">`
    /// anywhere in the document, captured at parse time
    /// ([`crate::parser`] — the single site every executable-content
    /// `<raise>` flows through, including `<finalize>`, nested
    /// `<if>`/`<foreach>`, initial/history transition content). This is
    /// the authoritative internal-signal set: the parser is the only
    /// stage that sees every `<raise>` before `<finalize>` bodies are
    /// stringified, so a downstream re-walk of the action tree would
    /// miss finalize-raised events. Build-time only (drives
    /// [`Self::externally_drivable_events`]); not part of any wire
    /// surface, hence `#[serde(skip)]`.
    #[serde(skip)]
    pub raised_events: BTreeSet<String>,
    /// Derived external-forgeability surface: the event descriptors an
    /// untrusted external party can legitimately drive this machine with
    /// — [`Self::external_ingress_events`] (non-reserved
    /// `<transition event>` triggers) minus [`Self::raised_events`]
    /// (internal `<raise>` signals), intersected with the concrete
    /// event-variant domain [`Self::events`] (so a prefix/wildcard
    /// descriptor like `foo.*`, which has no single variant, is not a
    /// member). This is the machine's trust boundary: an internally
    /// raised event is an owned internal signal, so forging it from
    /// outside is spoofing; a wildcard is a runtime-matching trigger,
    /// not a concrete forgeable event. A strict subset of
    /// [`Self::external_ingress_events`]. Populated last in
    /// [`crate::analyzer::compute_externally_drivable_events`] (after
    /// prefix-matching, so `events` is final). Consumer-agnostic
    /// structural fact; the Rust backend surfaces it as the
    /// `{Machine}Event::EXTERNALLY_DRIVABLE_EVENTS` associated const.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub externally_drivable_events: BTreeSet<String>,
    pub history_default_targets: BTreeMap<String, String>,
    pub history_states: BTreeMap<String, HistoryInfo>,

    // Feature flags
    pub has_dynamic_expressions: bool,
    pub has_parallel_states: bool,
    pub has_history_states: bool,
    pub has_event_metadata: bool,
    pub has_parent_communication: bool,
    /// SCE Protocol-Synthesis RFC §synth-5-E sample-callback codegen wire-up —
    /// sorted set of every `link` name referenced by any state's
    /// `<sce:on-sample link="X" .../>` block. Derived in
    /// [`crate::analyzer::analyze_model_features`] as the union over
    /// `state.on_sample_blocks`. Empty when the machine has no sample
    /// subscriptions (the common case); non-empty drives the Rust
    /// state_machine template's `deliver_link_X_sample` method
    /// emission (host-driven delivery, generic over `M:
    /// SampleMeta` so the codegen does not need to know the link's
    /// concrete metadata type at template-render time).
    #[serde(skip_serializing_if = "BTreeSet::is_empty", default)]
    pub on_sample_links: BTreeSet<String>,
    /// Every top-level `<sce:driver
    /// href="..."/>` reference on the document root, in document order.
    /// Each entry is an externally-authored driver header that the C11
    /// backend `#include`s into the emitted `*_sm.c` so cross-TU symbol
    /// resolution is handled by the C compiler.
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
    /// Set of session-FSM roles this SCXML document explicitly
    /// declares via top-level `<sce:session-role kind="..."/>`
    /// elements. Each variant appears at most once (parse-time
    /// uniqueness is enforced by
    /// [`crate::forge::error::ValidationError::ScxmlDuplicateSessionRoleDeclaration`]).
    ///
    /// Empty when the document declares no roles (the common case
    /// for non-session-FSM SCXML). Non-empty: drives the orchestrator's
    /// cross-doc listener-pair join — [`crate::resolve_listener_links`]
    /// reads this set to pair `role: listener` deploy links with
    /// machines declaring the matching session role.
    ///
    /// `BTreeSet` keeps iteration order deterministic so any diagnostic
    /// quoting the declared-roles list produces a stable string.
    #[serde(skip_serializing_if = "BTreeSet::is_empty", default)]
    pub declared_session_roles: BTreeSet<SessionRoleKind>,
    /// Section attribute payload
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
    /// §scxml-6.4 / test229: `true` iff any `<invoke>` in the document
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
    /// Every construct that forced [`Self::needs_script_engine`] to `true`
    /// — the analyzer's full finding, not just its boolean projection.
    ///
    /// Written in the same statement as the flag, from one
    /// [`crate::script_engine_analyzer::analyze`] traversal, so the
    /// invariant `needs_script_engine == !script_engine_causes.is_empty()`
    /// holds by construction. Recomputing the list downstream would
    /// re-derive it from a model that later passes may have changed, and a
    /// flag that disagreed with its own explanation is worse than no
    /// explanation. Pinned by
    /// `script_engine_analyzer::tests::model_flag_agrees_with_stored_causes`.
    ///
    /// Projected onto the `sce-codegen generate` stdout manifest as
    /// `script_engine_causes` (SCE_ERROR_CONTRACT.md §10.4) so a build that
    /// gates on a pure-static lowering can name what cost it.
    #[serde(skip)]
    pub script_engine_causes: Vec<crate::script_engine_analyzer::NeedsScriptEngineCause>,
    /// Every `<send>` / `<invoke>` site naming a processor type this
    /// build has no lowering path for.
    ///
    /// Sibling of [`Self::script_engine_causes`] and stored for the same
    /// reason: the traversal happens once, at parse, so the report and
    /// the emitted code describe the same model. The generated code
    /// refuses these sites at runtime whether or not anyone reads this
    /// list — what the list adds is that the refusal becomes knowable
    /// before the state is entered.
    ///
    /// Projected onto the `sce-codegen` stdout manifest as
    /// `host_processor_causes`, beside the `needs_host_processor`
    /// boolean it explains.
    #[serde(skip)]
    pub host_processor_causes: Vec<crate::host_processor_analyzer::HostProcessorCause>,
    /// Event I/O Processor `type` values this build's host has declared
    /// it serves, beyond the two §scxml-C-1 / §scxml-C-2 names.
    ///
    /// A build input rather than a document fact — the same SCXML
    /// compiled for a host that serves `x-sprag-host` and for one that
    /// does not is two different lowerings, and this is what separates
    /// them. Carried on the model rather than in a per-backend options
    /// struct so every generator reads the same declaration; templates
    /// see the *decision* on each `<send>`
    /// ([`Action::send_type_host_served`]) rather than re-deriving it
    /// from this list.
    #[serde(default)]
    pub host_processor_types: Vec<String>,
    /// `<invoke type="...">` values this build's host has declared it can
    /// run (§scxml-6.4.1 leaves the invokable set to the platform).
    ///
    /// A separate list from [`Self::host_processor_types`] because they
    /// are separate contracts: delivering an event is not the same
    /// capability as running an invoked process with a lifecycle, and one
    /// list would make declaring either silently claim both.
    #[serde(default)]
    pub host_invoker_types: Vec<String>,
    /// Whether any host-served `<send>` in this document carries a delay
    /// (§scxml-6.2.4 + §scxml-6.2.5).
    ///
    /// A delayed host-served send is performed from the scheduler drain
    /// rather than at the send site, so whatever holds the delayed-send
    /// queue must be able to hold the request itself. The backends with a
    /// heap say so in a variant and pay nothing when the document has
    /// none; the C11 backend's queue entry is a fixed-size struct, so it
    /// needs this answer at generation time to decide whether to emit the
    /// storage at all. Every backend can read it, so the decision is one
    /// fact rather than six re-derivations of it.
    ///
    /// Derived in [`crate::host_processor_analyzer::declare_host_surfaces`],
    /// from the same walk that marks each `<send>`
    /// ([`Action::send_type_host_served`]) — it cannot be known before the
    /// declaration is applied, because until then those sends are refusals.
    #[serde(default)]
    pub has_delayed_host_send: bool,
    /// The largest `<param>` count on any delayed host-served `<send>`.
    ///
    /// Zero when [`Self::has_delayed_host_send`] is false, and zero also
    /// for a document whose delayed host sends carry no parameters. Only
    /// the C11 backend reads it: its deferred entry owns the evaluated
    /// values (§scxml-6.2 evaluates a `<param>` at SEND time, not at fire
    /// time — tests 176 and 186), and a fixed-size struct has to be told
    /// how many to make room for.
    #[serde(default)]
    pub delayed_host_send_max_params: usize,
    /// Whether the document contains any `<cancel>` action (§scxml-6.3).
    ///
    /// Drives the Rust `StatePolicy::ScheduledSendId` selection: a cancel-free
    /// document emits the zero-size `ElidedSendId` (the no_std scheduler ring
    /// sheds its per-entry `send_id` string), a cancelling one emits the
    /// load-bearing `SceString`. Set in
    /// [`crate::analyzer::analyze_action`] from the same `<cancel>` walk that
    /// flags `needs_event_scheduler`.
    pub uses_cancel: bool,
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
    /// The `<data>` declarations that get a typed read accessor, one entry
    /// per name. The clause is cited where the rule lives, on
    /// `analyzer::readable_variables`, which computes this.
    ///
    /// Derived by [`crate::analyzer`] rather than filtered in each
    /// template, because the answer spans [`Self::variables`] and every
    /// state's own `<datamodel>` and has to reconcile them: the data model
    /// is one flat set of names, so a name declared at both depths is one
    /// variable and must yield one accessor. An emitter that walked the two
    /// lists in turn would define it twice, which is a compile failure in
    /// generated code — the kind that is loud but far from its cause. Six
    /// backends read this list; none of them repeats the reconciliation.
    pub readable_variables: Vec<Variable>,
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

    /// The coordinate system this model's recorded positions live in.
    ///
    /// Every `source_location` on this model names a row in the
    /// *expanded* document — the string `<xi:include>` and `<sce:use>`
    /// expansion produced, which exists only in memory. Validators
    /// that run after parsing (`analyzer::can_generate_static`,
    /// `scxml_references::validate`) report positions to consumers who
    /// open files, so they resolve through this before emitting: a row
    /// past the end of the authored file, or a row whose text does not
    /// contain the rejected value, is what an unresolved expanded
    /// coordinate looks like on the wire.
    ///
    /// `None` for models parsed from a string with no preprocessor
    /// pass, where expanded and authored coordinates coincide.
    #[serde(skip)]
    #[cfg_attr(test, schemars(skip))]
    pub authored_positions: Option<AuthoredPositions>,

    // Analysis helpers (set by analyzer)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub needs_transition_helper: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub needs_assign_helper: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub needs_foreach: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub needs_guard_helper: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub needs_send_helper: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub needs_event_data_helper: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub needs_donedata_helper: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub needs_namelist_helper: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub needs_event_type_helper: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub needs_event_scheduler: Option<bool>,
    /// [`Self::needs_event_scheduler_driving`] as a field, so a template can
    /// read the union without re-spelling its three terms.
    ///
    /// Set by the analyzer beside the other derived gates
    /// (`needs_parent_template`, `needs_nonstatic_method`). It answers a
    /// different question from [`Self::needs_event_scheduler`], which is about
    /// this machine owning a scheduler queue: a parent that schedules nothing
    /// itself still needs `tick()` to reach an invoked child's queue, so the
    /// two must not be conflated into one flag.
    #[serde(default)]
    pub needs_tick_driving: bool,
    /// §scxml-B-2: any reachable `<data>` content / `<data src=...>`
    /// loaded payload / `<send><content>` literal whose first non-WS
    /// character is `<` triggers the host-side XML DOM helper (C11
    /// backend only — cpp / Rust / Go / Kotlin handle the same surface
    /// through their own pipelines). Gates `lua_dom_register_metatable`
    /// emission in scriptengine.jinja2 and the XML branches in the
    /// var.content / send.content / event-promotion sites.
    ///
    /// Also true for every machine that carries a script engine, regardless of
    /// what its own text says: §scxml-B-2-8-1 decides an arriving
    /// `_event.data`'s reading from the PAYLOAD at run time, and a host may
    /// hand any scripted machine an XML document. The build-time scans above
    /// cannot see that, so before 2026-08-19 a machine whose text never
    /// mentioned `<` shipped without the helper and read an XML payload as a
    /// space-normalized string while its six siblings built a DOM.
    #[serde(default, skip_serializing_if = "Option::is_none")]
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
    /// `ScxmlInvokeChannel` for answering with wire-15 `InvokeStarted` on
    /// success or wire-20 `InvokeError` when the child cannot start.
    /// Empty when no peer invokes this machine.
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

    /// SCE_MESH.md §16.5 barrier-timeout runtime. Maps each
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

    /// SCE Protocol-Synthesis RFC §synth-5-O source traceability: post-preprocessor source
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
    /// True iff the state declares any [`Invoke::Unsupported`]
    /// (§scxml-6.4.1 unsupported `type`). Used by codegen to decide
    /// whether the state's entry chain must defer a pending invoke whose
    /// only effect is the spec-mandated `error.execution` raise.
    pub fn has_unsupported_invoke(&self) -> bool {
        self.invokes
            .iter()
            .any(|i| matches!(i, Invoke::Unsupported(_)))
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
    /// True iff any `<invoke type="scxml">` in the model carries a
    /// `namelist` attribute.
    ///
    /// §scxml-6.4.1 gives `namelist` on `<invoke>` the same reading it has
    /// on `<send>` — a list of data-model locations read in the invoking
    /// session — so a document carrying one needs the namelist helper
    /// declared whether or not it also sends anything. Only
    /// [`Invoke::Scxml`] can carry the attribute: the hybrid, mesh-rpc and
    /// unsupported variants have no namelist field.
    pub fn has_invoke_namelist(&self) -> bool {
        self.states.values().any(|s| {
            s.invokes.iter().any(|i| match i {
                Invoke::Scxml(si) => !si.namelist.is_empty(),
                _ => false,
            })
        })
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
    /// True iff a host must drive this machine with the runtime's
    /// `tick()` rather than `step()`.
    ///
    /// `tick()` carries two mechanisms `step()` does not: it drains the
    /// §scxml-6.2 delayed-send scheduler, and it ticks invoked child
    /// sessions. Either one alone makes it mandatory, which is why this
    /// is a union rather than the scheduler flag on its own — a parent
    /// that schedules nothing still reaches its child's queue only
    /// through `tick_children`.
    ///
    /// The two terms are the same conditions codegen already gates on:
    /// the analyzer sets [`Self::needs_event_scheduler`] from `<send
    /// delay>` / `<cancel>`, and the emitted policy declares
    /// `HAS_CHILD_TICK` for session-bearing invokes (`scxml` / `hybrid`
    /// — a mesh-rpc or unsupported-type invoke has no child engine to
    /// tick). Stated once here so the manifest's answer and the emitted
    /// code cannot disagree about which entry point the machine needs.
    pub fn needs_event_scheduler_driving(&self) -> bool {
        self.needs_event_scheduler.unwrap_or(false)
            || self.has_scxml_invoke()
            || self.has_hybrid_invoke()
    }
    /// True iff any state in the model declares an [`Invoke::Unsupported`]
    /// (§scxml-6.4.1 unsupported `type`). Drives the analyzer's
    /// `error.execution` event registration so `Event::Error_execution`
    /// resolves in the generated enum whether or not the author wrote a
    /// handler for it.
    pub fn has_unsupported_invoke(&self) -> bool {
        self.states.values().any(|s| s.has_unsupported_invoke())
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

    /// §scxml-3.3 / §scxml-3.4: Resolve state ID to leaf by following initial attrs
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

/// Every `model.<name>` a template reads is a name `SCXMLModel` declares.
///
/// The generator's root context object is this struct, and minijinja is
/// configured `Chainable` — an attribute the struct does not declare becomes
/// undefined, which is FALSY in a conditional and EMPTY when printed. So a
/// misspelt field does not fail; it silently takes the else branch. Measured
/// once already: a Go template read `trans.transition_type` where the model
/// declares `type`, and `type="internal"` never reached that engine.
///
/// `UndefinedBehavior::Strict` is the obvious cure and it does not work here.
/// This struct omits 91 optional fields from serialization
/// (`skip_serializing_if = "Option::is_none"`), so under Strict every read of a
/// currently-`None` field is an error indistinguishable from a typo — measured
/// 2026-08-26: flipping it made all six backends fail rendering at
/// `state_machine.jinja2:18`, on `model.needs_event_scheduler`, a real field
/// that happened to be `None`.
///
/// What IS decidable is the name set. `model` is the root context and always
/// this type, so `model.X` must be a declared name — no loop-binding inference
/// needed. The schema is the authority rather than a list kept by hand, and
/// `schema_for!` reports optional fields too, which is exactly what the Strict
/// route could not separate.
///
/// ⚠ SCOPE, stated because a partial gate reads like a total one: this checks
/// the `model.` base only. Templates also read `action.`, `state.`, `trans.`,
/// `invoke_info.` and a dozen more whose types come from loop bindings this does
/// not resolve — 576 distinct attribute names in all, of which 414 belong to
/// types declared outside this file. The original defect was on `trans.`, so it
/// would NOT have been caught here. `model.` is the largest single base (2238
/// accesses) and the only one whose type needs no inference.
#[cfg(test)]
mod model_attribute_names {
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};

    /// The one access the templates make that this struct does not declare.
    ///
    /// `model.scxml_author` has never existed in the Rust sources. Three license
    /// headers read it through `| default('[Author of input SCXML file]')`, so
    /// the placeholder is what ships: measured 2026-08-26, 388 generated files
    /// under version control carry that literal. It is listed rather than fixed
    /// because the repair is a choice — carry the input document's author
    /// through, or drop the access — and either one edits templates, which
    /// re-pins `template-hash` across every committed tree. Named here so a NEW
    /// unknown access still fails.
    const KNOWN_MISSING: &[&str] = &["scxml_author"];

    fn templates_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools/codegen/templates")
    }

    /// Every name a template may legitimately read off a model object.
    ///
    /// TWO derivations, unioned, because neither alone is the declared surface
    /// and finding that out cost a false accusation. `schema_for!` is
    /// authoritative for the schema but NOT for the struct: this file marks
    /// fields `#[cfg_attr(test, schemars(skip))]`, and five of them —
    /// `symbol_state_path`, `symbol_artifact`, `native_payload_guard`,
    /// `native_action_rendered` and their kin — are real fields templates read
    /// every render. A gate built on the schema alone reported those as typos.
    ///
    /// So the schema supplies one set, the source text supplies another, and
    /// `the_two_derivations_agree` below asserts the schema is a SUBSET of the
    /// source scan. That is what keeps the text half honest: a scan that
    /// silently stopped matching would drop below the schema and be caught,
    /// rather than quietly widening the allow-set.
    fn declared() -> BTreeSet<String> {
        let mut names = schema_names();
        names.extend(source_names());
        names.extend(PYCOMPAT_METHODS.iter().map(|m| (*m).to_string()));
        names
    }

    /// Method names `minijinja_contrib::pycompat` makes callable on strings,
    /// dicts and sequences. A template calling `state.id.startswith(...)` is
    /// reading a method, not a field.
    ///
    /// ⚠ This does widen the accepted set, so a misspelling that happens to
    /// equal one of these names would pass. That is the cost of the pycompat
    /// shim being enabled at all; the alternative is a gate that fires on every
    /// string operation in the tree.
    const PYCOMPAT_METHODS: &[&str] = &[
        "startswith",
        "endswith",
        "strip",
        "lstrip",
        "rstrip",
        "split",
        "splitlines",
        "upper",
        "lower",
        "replace",
        "items",
        "keys",
        "values",
        "get",
        "join",
        "count",
        "find",
        "title",
        "capitalize",
        "isdigit",
        "isalpha",
        "format",
    ];

    fn schema_names() -> BTreeSet<String> {
        let schema = schemars::schema_for!(super::SCXMLModel);
        let json = serde_json::to_value(&schema).expect("the schema serializes");
        let mut names = BTreeSet::new();
        collect_properties(&json, &mut names);
        names
    }

    /// The WIRE names of this file's fields, plus its public methods.
    ///
    /// ⚠⚠⚠ A renamed field contributes its rename and NOT its Rust identifier,
    /// and getting that backwards is not a detail — it is the whole defect. G1's
    /// recorded instance is `Transition`:
    ///
    ///     #[serde(rename = "type")]
    ///     pub transition_type: String,
    ///
    /// The template must write `trans.type`; a Go template wrote
    /// `trans.transition_type` and `type="internal"` never reached that engine.
    /// A first draft of this scan added both names, so restoring the original
    /// defect PASSED — measured. An accepted set containing the Rust identifier
    /// is an accepted set that blesses exactly the mistake being hunted.
    ///
    /// Serde is what the render context goes through, so serde's name is the
    /// only one a template can read. This is also why `schema_for!` had it right
    /// on its own: schemars follows the same renames.
    fn source_names() -> BTreeSet<String> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/model.rs");
        let src = std::fs::read_to_string(&path).expect("this file is readable");
        let mut names = BTreeSet::new();
        let mut pending_rename: Option<String> = None;
        for line in src.lines() {
            let t = line.trim();
            if let Some(at) = t.find("rename = \"") {
                let rest = &t[at + "rename = \"".len()..];
                if let Some(end) = rest.find('"') {
                    pending_rename = Some(rest[..end].to_string());
                }
                continue;
            }
            let Some(rest) = t.strip_prefix("pub ") else {
                // Attributes and doc comments sit between the rename and the
                // field, so only a non-attribute, non-`pub` line clears it.
                if !t.starts_with('#') && !t.starts_with("///") && !t.is_empty() {
                    pending_rename = None;
                }
                continue;
            };
            if let Some(name) = rest.strip_prefix("fn ") {
                let ident: String = name
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect();
                if !ident.is_empty() {
                    names.insert(ident);
                }
                pending_rename = None;
            } else if let Some((ident, _)) = rest.split_once(':') {
                let ident = ident.trim();
                if !ident.is_empty() && ident.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                {
                    match pending_rename.take() {
                        Some(wire) => names.insert(wire),
                        None => names.insert(ident.to_string()),
                    };
                }
            }
        }
        names
    }

    fn collect_properties(node: &serde_json::Value, out: &mut BTreeSet<String>) {
        match node {
            serde_json::Value::Object(map) => {
                if let Some(serde_json::Value::Object(props)) = map.get("properties") {
                    for key in props.keys() {
                        out.insert(key.clone());
                    }
                }
                for value in map.values() {
                    collect_properties(value, out);
                }
            }
            serde_json::Value::Array(items) => {
                for item in items {
                    collect_properties(item, out);
                }
            }
            _ => {}
        }
    }

    /// `model.<name>` occurrences inside Jinja delimiters, comments removed.
    ///
    /// Both halves matter. Outside `{{ }}` / `{% %}` a template is literal
    /// output, and the C++ it emits contains `->model.` sequences that are not
    /// template accesses at all; inside a `{# #}` comment the prose discusses
    /// field names it does not read. A scan skipping either would report names
    /// no render ever performs.
    fn accesses() -> BTreeSet<(String, String)> {
        let mut found = BTreeSet::new();
        let mut stack = vec![templates_dir()];
        while let Some(dir) = stack.pop() {
            let entries = std::fs::read_dir(&dir).expect("the template tree is readable");
            for entry in entries {
                let path = entry.expect("a directory entry").path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if path.extension().is_none_or(|e| e != "jinja2") {
                    continue;
                }
                let body = std::fs::read_to_string(&path).expect("a template is readable");
                let name = path
                    .file_name()
                    .expect("a file name")
                    .to_string_lossy()
                    .into_owned();
                let exprs = delimited(&strip_jinja_comments(&body));
                let derived = model_derived_bases(&exprs);
                for expr in &exprs {
                    for (base, attr) in attribute_accesses(expr) {
                        if derived.contains(&base) {
                            // The BASE is kept, not flattened to "model". A
                            // finding that says `model.symbol_artifact` when the
                            // template wrote `trans.symbol_artifact` sends the
                            // reader to the wrong struct — which is how the
                            // first version of this gate accused five real
                            // fields of being typos.
                            found.insert((format!("{base}.{attr}"), name.clone()));
                        }
                    }
                }
            }
        }
        found
    }

    /// The variables in one template that hold model data.
    ///
    /// `model` itself, plus every `{% for X in EXPR %}` whose EXPR is rooted at a
    /// variable already known to be model data — so `state` from
    /// `model.states`, then `trans` from `state.transitions`, and so on. Iterated
    /// to a fixed point because a template may bind the outer loop after the
    /// inner one in source order (an `{% include %}`d fragment does exactly
    /// that).
    ///
    /// This is what lets the check reach past `model.`. It is also what keeps it
    /// from drowning: templates read `action.`, `f.`, `field.`, `rule.`,
    /// `variant.` and a dozen other bases that are forge kinds, macro arguments
    /// or filter output, none of which this struct describes — 414 of the 576
    /// distinct attribute names in the tree belong to those. Checking a name
    /// against a schema that never claimed to hold it is how a gate produces
    /// noise and then gets an exception list long enough to hide a real typo.
    fn model_derived_bases(exprs: &[String]) -> BTreeSet<String> {
        let mut derived: BTreeSet<String> = BTreeSet::new();
        derived.insert("model".to_string());
        loop {
            let before = derived.len();
            for expr in exprs {
                for (bound, from) in for_bindings(expr) {
                    if derived.contains(&from) {
                        derived.insert(bound);
                    }
                }
            }
            if derived.len() == before {
                break;
            }
        }
        derived
    }

    /// `(bound variable, root of the iterated expression)` for one `{% for %}`.
    ///
    /// Handles the tuple form too: `{% for id, state in model.states.items() %}`
    /// binds the VALUE to the second name, which is the one carrying model data.
    fn for_bindings(expr: &str) -> Vec<(String, String)> {
        let trimmed = expr.trim_start().trim_start_matches('-').trim_start();
        let Some(rest) = trimmed.strip_prefix("for ") else {
            return Vec::new();
        };
        let Some((names, iterated)) = rest.split_once(" in ") else {
            return Vec::new();
        };
        // The iterated expression's root identifier: `state.transitions | foo`
        // is rooted at `state`.
        let root: String = iterated
            .trim()
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if root.is_empty() {
            return Vec::new();
        }
        let bound: Vec<&str> = names.split(',').map(str::trim).collect();
        // One name binds the item; two bind key and value, and the value is the
        // one that carries the model object.
        let carrier = bound.last().copied().unwrap_or_default();
        if carrier.is_empty()
            || !carrier
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Vec::new();
        }
        vec![(carrier.to_string(), root)]
    }

    /// Every `<base>.<attr>` in one delimited expression.
    fn attribute_accesses(expr: &str) -> Vec<(String, String)> {
        let mut out = Vec::new();
        let bytes = expr.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if !(bytes[i].is_ascii_alphabetic() || bytes[i] == b'_') {
                i += 1;
                continue;
            }
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            if i >= bytes.len() || bytes[i] != b'.' {
                continue;
            }
            let base = expr[start..i].to_string();
            i += 1;
            let attr_start = i;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            if i > attr_start {
                out.push((base, expr[attr_start..i].to_string()));
            }
        }
        out
    }

    fn strip_jinja_comments(body: &str) -> String {
        let mut out = String::with_capacity(body.len());
        let mut rest = body;
        while let Some(open) = rest.find("{#") {
            out.push_str(&rest[..open]);
            match rest[open..].find("#}") {
                Some(end) => {
                    out.push(' ');
                    rest = &rest[open + end + 2..];
                }
                None => return out,
            }
        }
        out.push_str(rest);
        out
    }

    /// The contents of every `{{ ... }}` and `{% ... %}` region.
    fn delimited(body: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut rest = body;
        loop {
            let curly = rest.find("{{");
            let block = rest.find("{%");
            let open = match (curly, block) {
                (None, None) => break,
                (Some(a), None) => a,
                (None, Some(b)) => b,
                (Some(a), Some(b)) => a.min(b),
            };
            let close = if rest[open..].starts_with("{{") {
                "}}"
            } else {
                "%}"
            };
            let after = &rest[open + 2..];
            match after.find(close) {
                Some(end) => {
                    out.push(after[..end].to_string());
                    rest = &after[end + close.len()..];
                }
                None => break,
            }
        }
        out
    }

    #[test]
    fn every_model_attribute_a_template_reads_is_declared() {
        let declared = declared();
        let accesses = accesses();

        // Floors, before any verdict. A scan of nothing violates nothing, and
        // both sides can go empty independently: the schema derive is behind
        // `cfg(test)` and the template tree is found by a relative path.
        assert!(
            declared.len() > 100,
            "the schema reported only {} property name(s); this gate has no \
             authority to check against",
            declared.len()
        );
        assert!(
            accesses.len() > 40,
            "only {} model attribute access(es) were found under {}; this gate \
             has no population and would pass on anything",
            accesses.len(),
            templates_dir().display()
        );

        let attr_of = |access: &str| -> String {
            access
                .rsplit_once('.')
                .map(|(_, a)| a.to_string())
                .unwrap_or_else(|| access.to_string())
        };

        let mut unknown: Vec<String> = accesses
            .iter()
            .filter(|(access, _)| !declared.contains(&attr_of(access)))
            .filter(|(access, _)| !KNOWN_MISSING.contains(&attr_of(access).as_str()))
            .map(|(access, file)| format!("{access} in {file}"))
            .collect();
        unknown.sort();
        unknown.dedup();

        let names: BTreeSet<String> = accesses.iter().map(|(a, _)| attr_of(a)).collect();
        let bases: BTreeSet<&str> = accesses
            .iter()
            .filter_map(|(a, _)| a.split_once('.').map(|(b, _)| b))
            .collect();
        println!(
            "{} distinct attribute name(s) on {} model-derived base(s), checked \
             against {} declared name(s); {} known-missing listed",
            names.len(),
            bases.len(),
            declared.len(),
            KNOWN_MISSING.len()
        );

        assert!(
            unknown.is_empty(),
            "a template reads an attribute the model does not declare on a \
             model-derived object. minijinja is `Chainable`, so it renders as \
             nothing and a condition on it silently takes the else branch — the \
             same shape that kept `type=\"internal\"` from reaching the Go \
             engine:\n  {}",
            unknown.join("\n  ")
        );
    }

    /// The two derivations overlap heavily, which is what keeps the text half
    /// of `declared()` trustworthy.
    ///
    /// The scan reads `pub <name>:` out of one file; if it ever stopped matching
    /// — a formatting change, a macro-generated struct — the accepted set would
    /// shrink toward nothing and every real field would start reading as a typo.
    /// A large intersection is what says it is still reading fields.
    ///
    /// ⚠ NOT containment, and the first draft of this test asserted containment
    /// and failed: 64 of the schema's names — `bit_offset`, `endian`,
    /// `tlv_chain_body_alias` and the rest of the forge vocabulary — belong to
    /// types declared in OTHER files that `SCXMLModel` reaches by `$ref`. The
    /// schema is transitive; this file is not the whole context surface. Saying
    /// so here so the next reader does not "fix" the scan to chase them.
    #[test]
    fn the_two_derivations_overlap() {
        let schema = schema_names();
        let source = source_names();
        assert!(
            schema.len() > 100 && source.len() > 100,
            "schema {} name(s), source scan {} name(s) — one of the two \
             derivations has collapsed",
            schema.len(),
            source.len()
        );
        let shared = schema.intersection(&source).count();
        assert!(
            shared > 100,
            "the two derivations share only {shared} name(s) out of {} schema \
             and {} scanned; the source scan is no longer reading this file's \
             fields, and the accepted set it feeds is nearly all schema",
            schema.len(),
            source.len()
        );
    }

    /// The listed exception is still missing, and still only one.
    ///
    /// Without this the list could outlive its reason: someone adds the field,
    /// the entry goes dead, and the next unknown access is waved through by a
    /// name that no longer means anything.
    #[test]
    fn the_known_missing_list_names_only_absent_fields() {
        let declared = declared();
        for name in KNOWN_MISSING {
            assert!(
                !declared.contains(*name),
                "`{name}` is now a declared field, so it must come off \
                 KNOWN_MISSING — an exception list naming present fields \
                 silently exempts real typos"
            );
        }
    }
}
