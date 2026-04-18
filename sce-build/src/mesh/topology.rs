// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh topology analyzer — collects <send> targets from an SCXML model,
// matches them against deploy.yaml bindings, and performs build-time validation.

use crate::mesh::deploy::{BindingConfig, DeployConfig};
use crate::mesh::error::TopologyError;
use crate::mesh::target::TargetId;
use crate::model::{Invoke, Param, SCXMLModel};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Per-event pattern metadata, detected at build time from the event name
/// prefix convention (`pattern::detect_pattern`). Used by codegen to generate
/// pattern-aware send logic (wireTo → PatternKind) and RPC correlation tables.
/// SCE_MESH.md §13 path B: no longer carries sce:* attribute provenance.
#[derive(Debug, Clone, Serialize)]
pub struct EventPatternInfo {
    /// SCXML event name (e.g. "service.request.brake_status").
    pub event: String,
    /// PatternKind wire value for C++ enum (e.g. 2 for RpcRequest).
    /// This is the only pattern representation the code generator needs —
    /// the symbolic form is recoverable from the event name and is not
    /// serialized into generated code.
    pub pattern_kind_value: u16,
    /// Paired reply event, inferred by convention for RPC requests
    /// (`service.request.X` → `service.response.X`). `None` for non-RPC
    /// patterns or when no convention match exists — empty-string sentinels
    /// are not used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_event: Option<String>,
}

/// Per-event resolved SOME/IP numeric IDs (SCE_MESH.md §14).
///
/// Tagged enum: each event binds to exactly one SOME/IP resource family
/// (RPC method, event group, field getter, or field setter) per the event's
/// communication pattern. Variant is the dispatch — no probing of which
/// `Option` happens to be populated, and impossible states (an event with
/// both `method_id` and `getter_id`) are unrepresentable.
///
/// Produced from:
///   1. Per-event `events:` block in deploy.yaml (spec canonical) — one
///      variant per entry, chosen by which `EventBinding` field is set.
///   2. Flat per-binding sugar projected through
///      [`CommunicationPattern::someip_field`] — see
///      [`BindingDefaultIds::project_to`].
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "field_kind", rename_all = "snake_case")]
pub enum SomeipEventIds {
    /// RPC / FireForget — `services[*].methods[*]` slot.
    Method { method_id: u16 },
    /// Subscribe / Notification — `services[*].eventgroups[*]` slot plus
    /// the single event ID it contains (the current template expects
    /// exactly one event per group; multi-event groups are rejected at
    /// resolution time).
    EventGroup { event_group_id: u16, event_id: u16 },
    /// Field read — `services[*].methods[*]` slot used as getter.
    Getter { getter_id: u16 },
    /// Field write — `services[*].methods[*]` slot used as setter.
    Setter { setter_id: u16 },
}

impl SomeipEventIds {
    /// The SOME/IP field kind this entry represents. Used by codegen and
    /// validation to dispatch uniformly (`match SomeipFieldKind { … }`)
    /// instead of probing which variant they see.
    pub fn field_kind(&self) -> super::pattern::SomeipFieldKind {
        use super::pattern::SomeipFieldKind;
        match self {
            Self::Method { .. } => SomeipFieldKind::Method,
            Self::EventGroup { .. } => SomeipFieldKind::EventGroup,
            Self::Getter { .. } => SomeipFieldKind::Getter,
            Self::Setter { .. } => SomeipFieldKind::Setter,
        }
    }
}

/// Binding-level defaults: optional IDs gathered from flat sugar
/// (`method:` / `event_group:` / `getter:` / `setter:` on `BindingConfig`).
/// A single binding may set more than one family at once — flat sugar is
/// "apply these defaults to all events whose pattern wants this family".
///
/// `project_to(pattern)` fans a default entry out into a per-event
/// [`SomeipEventIds`]; callers consult
/// [`CommunicationPattern::someip_field`] to decide which fields are
/// relevant to each event's pattern.
#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct BindingDefaultIds {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method_id: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_group_id: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub getter_id: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub setter_id: Option<u16>,
}

/// Binding-level SOME/IP service identity (SCE_MESH.md §14). Populated
/// by name-based resolution of `service:` against vsomeip.json in
/// `external.rs`; inline `service_id:`/`instance_id:` are rejected
/// outright (see [`ExternalConfigError::ReservedSomeipIdKeys`]).
///
/// Held as a typed field on [`ResolvedTarget`] rather than probed out of
/// the untyped `extra` map at codegen time; this keeps `extra` reserved
/// for genuinely opaque transport-native passthrough keys.
///
/// [`ExternalConfigError::ReservedSomeipIdKeys`]:
///     crate::mesh::error::ExternalConfigError::ReservedSomeipIdKeys
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct SomeipServiceIds {
    pub service_id: u16,
    pub instance_id: u16,
}

impl BindingDefaultIds {
    pub fn is_empty(&self) -> bool {
        self.method_id.is_none()
            && self.event_group_id.is_none()
            && self.event_id.is_none()
            && self.getter_id.is_none()
            && self.setter_id.is_none()
    }

    /// Project the default into a per-event value for a specific pattern,
    /// using [`CommunicationPattern::someip_field`] as SSOT for which
    /// family the pattern needs. Returns `None` if the pattern's required
    /// family is not populated on this default — caller treats that as
    /// "no defaults to attach for this event".
    pub fn project_to(&self, pattern: super::pattern::CommunicationPattern) -> Option<SomeipEventIds> {
        use super::pattern::SomeipFieldKind;
        // A pattern with no dedicated SOME/IP slot (currently only
        // FieldNotify, which piggybacks on the getter/setter reply path)
        // never consumes a per-binding default. Binding-level flat sugar
        // is only meaningful for request/subscribe/field-access patterns.
        match pattern.someip_field()? {
            SomeipFieldKind::Method => self.method_id.map(|id| SomeipEventIds::Method { method_id: id }),
            SomeipFieldKind::EventGroup => match (self.event_group_id, self.event_id) {
                (Some(group), Some(event)) => {
                    Some(SomeipEventIds::EventGroup { event_group_id: group, event_id: event })
                }
                _ => None,
            },
            SomeipFieldKind::Getter => self.getter_id.map(|id| SomeipEventIds::Getter { getter_id: id }),
            SomeipFieldKind::Setter => self.setter_id.map(|id| SomeipEventIds::Setter { setter_id: id }),
        }
    }
}

/// SCE Mesh §9.5: a single `<invoke type="sce:mesh-rpc">` attached to
/// a resolved target. Collected from `SCXMLModel.invokes` in
/// [`collect_send_summary`] and attached to the matching
/// [`ResolvedTarget`] in [`finalize_targets`].
///
/// The data duplicates a subset of [`crate::model::MeshRpcInvokeInfo`]
/// but deliberately lives in the topology layer: the parser artifact
/// describes what the author wrote, while this struct describes what
/// codegen needs per target (invoke methods on `TransportRouter`,
/// state-entry/exit hooks). Keeping them distinct lets topology skip
/// fields like `idlocation` that are irrelevant to the wire and also
/// keeps parser output independent of codegen shape.
#[derive(Debug, Clone, Serialize)]
pub struct MeshRpcInvokeSite {
    /// Enclosing SCXML state — the parent that runs `<invoke>` on
    /// entry. Matches `Invoke::MeshRpc.base.state_name`.
    pub state_name: String,
    /// SCXML invoke id (possibly auto-generated like `_invoke_0`).
    /// Surfaces as `_event.invokeid` / `done.invoke.<id>` /
    /// `error.invoke.<id>` on the parent engine.
    pub invoke_id: String,
    /// Identifier-safe suffix for generated field / method names.
    /// Equal to `invoke_id` with the W3C auto-id leading underscore
    /// stripped, mirroring `InvokeBase::field_suffix` so codegen sites
    /// compose `invoke_<suffix>` / `cancel_<suffix>` cleanly.
    pub field_suffix: String,
    /// Value of the required `<param name="_mesh_event">`. Populates
    /// the outbound envelope's `type` field per §9.5 wire mapping.
    pub mesh_event: String,
    /// Value of the optional `<param name="_mesh_deadline_ms">`.
    /// `None` means no per-invoke deadline; the deploy.yaml binding
    /// deadline (if any) applies, otherwise no timeout.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deadline_ms: Option<u64>,
    /// Author payload — the `<param>` entries that remained after the
    /// reserved `_mesh_*` names were stripped by
    /// `parse_mesh_rpc_invoke`.
    pub params: Vec<Param>,
}

/// Per-target transport-specific state. Variant identity replaces the
/// `transport: String` runtime tag — a reader can no longer reach for
/// `someip_service` on a non-someip target because the field does not
/// exist on those variants.
///
/// Three sources are merged into the variant payloads at construction
/// (`finalize_targets` + `build_transport_state`):
///   1. `transport:` from the deploy.yaml binding selects the variant.
///   2. Validated typed fields (e.g. `Shm::arena_bytes`) come from
///      `validate_shm_extras_partial` having already proven the source
///      yaml value parses cleanly.
///   3. `extra: HashMap<...>` carries genuinely opaque transport-native
///      passthrough keys (e.g. someip `protocol:`) — reserved-name keys
///      that aliased typed fields are stripped before this lands here.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TransportState {
    /// Same-process direct call. No transport state besides the target
    /// identity carried by `ResolvedTarget`.
    Local,
    /// Shared-memory cross-process channel. Capacity tunables are typed
    /// here so the template never re-validates them.
    Shm {
        /// `None` falls back to `SCE::Mesh::SHM_DEFAULT_ARENA_BYTES`.
        #[serde(skip_serializing_if = "Option::is_none")]
        arena_bytes: Option<u32>,
        /// `None` falls back to `SCE::Mesh::SHM_DEFAULT_RING_CAPACITY`.
        #[serde(skip_serializing_if = "Option::is_none")]
        ring_capacity: Option<u32>,
    },
    /// SOME/IP via vsomeip. Identity + per-event ID map are inseparable.
    Someip {
        /// Service / instance numeric IDs resolved from vsomeip.json.
        service: SomeipServiceIds,
        /// Per-event resolved IDs. Topology guarantees one entry per
        /// `event_patterns` entry — `validate_someip_pattern_fields`
        /// rejects builds where this invariant is violated.
        event_bindings: BTreeMap<String, SomeipEventIds>,
        /// Non-typed passthrough (e.g. `protocol: udp/tcp`). Reserved
        /// SOME/IP ID keys are stripped upstream. Always serialised as
        /// an object so consumers can probe `extra.<key>` with `is
        /// defined` even on empty bindings.
        extra: std::collections::HashMap<String, serde_yaml_ng::Value>,
    },
    /// Zenoh pub/sub. The key expression is mandatory.
    Zenoh {
        /// Zenoh key expression (`brake/cmd`, etc).
        key: String,
        /// Non-typed passthrough. Always serialised (see Someip::extra).
        extra: std::collections::HashMap<String, serde_yaml_ng::Value>,
    },
    /// custom_tcp reference transport (SCE_MESH.md §16.8.3). One TCP
    /// client per binding connecting to `connect`. The device's optional
    /// listen endpoint is device-shared state and lives on
    /// [`crate::mesh::deploy::CustomTcpTransportConfig`], not here.
    CustomTcp {
        /// Server endpoint to dial (`host:port`). Validated as present
        /// upstream by `required_binding_fields = ["connect"]`.
        connect: String,
        /// Non-typed passthrough; always serialised so consumers can probe
        /// `extra.<key>` with `is defined` even on empty bindings.
        extra: std::collections::HashMap<String, serde_yaml_ng::Value>,
    },
    /// Recognised in the registry but not yet implemented (dds, can, …).
    /// Codegen rejects with `UnsupportedTransport` before this variant
    /// reaches the template — the variant exists so the sum type
    /// covers every entry in `transport::lookup`.
    Unimplemented {
        /// The deploy.yaml `transport:` string, retained verbatim for
        /// diagnostics emitted by codegen.
        transport_name: String,
    },
}

impl TransportState {
    /// Stable string label identifying the transport variant. Used by
    /// diagnostics that need to refer to the transport by name (e.g.
    /// `transport::lookup` queries, error messages).
    pub fn transport_name(&self) -> &str {
        match self {
            Self::Local => "local",
            Self::Shm { .. } => "shm",
            Self::Someip { .. } => "someip",
            Self::Zenoh { .. } => "zenoh",
            Self::CustomTcp { .. } => "custom_tcp",
            Self::Unimplemented { transport_name } => transport_name,
        }
    }
}

/// A resolved send target: SCXML <send> target matched to a deploy.yaml binding.
///
/// Transport-specific data lives on `state` ([`TransportState`]). The
/// runtime tag that used to live on a separate `transport: String` field
/// is the variant identity itself — readers cannot probe SOME/IP fields
/// on a non-SOME/IP target because those fields do not exist on those
/// variants.
#[derive(Debug, Clone, Serialize)]
pub struct ResolvedTarget {
    /// The target ID from SCXML (e.g. "#motor").
    pub target: TargetId,
    /// Events sent to this target (for documentation/validation).
    pub events: Vec<String>,
    /// Per-event pattern metadata for codegen (pattern-aware send + RPC correlation).
    pub event_patterns: Vec<EventPatternInfo>,
    /// Tagged transport state — variant identity is the dispatch key for
    /// codegen and validators. Replaces the historical
    /// `transport: String` + `someip_service: Option<...>` +
    /// `event_bindings: BTreeMap<...>` + `extra: HashMap<...>` quartet
    /// whose populated/empty correlation was a runtime invariant.
    pub state: TransportState,
    /// `<invoke type="sce:mesh-rpc">` sites whose `src="#<target>"`
    /// resolves to this target. Empty for targets that only receive
    /// `<send>` traffic. Populated by [`finalize_targets`] from the
    /// summary collected in [`collect_send_summary`].
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub invoke_sites: Vec<MeshRpcInvokeSite>,
    /// SCE_MESH.md §10.6 per-binding ordering declaration. Copied
    /// verbatim from the binding's `ordering:` key; default is
    /// [`OrderingRequirement::None`]. Codegen consumes this alongside
    /// the registry-level `supplies_ordering` to decide whether the
    /// generated mesh-send-callback must stamp `env.sequence_no` and
    /// whether the receiver path must route through `admitOrdered`.
    #[serde(default, skip_serializing_if = "crate::mesh::deploy::OrderingRequirement::is_none")]
    pub ordering: crate::mesh::deploy::OrderingRequirement,
    /// SCE_MESH.md §14.4 — runtime pool substitution plan, or `None` if
    /// this target has no placeholder bindings. The typed sum type
    /// (`Zenoh { placeholders }` vs `Someip { instance_from, instances }`)
    /// replaces the historical pattern of consulting `pt.extra` +
    /// `pt.instance_from` + `pt.instances` at every codegen / validator
    /// site; each consumer dispatches by variant and cannot observe an
    /// ambiguous half-built pool.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pool_plan: Option<PoolPlan>,
}

/// SCE_MESH.md §14.4 — resolved binding pool plan for runtime target
/// substitution. Transport-specific because each transport exposes a
/// different substitution surface (Zenoh uses `{name}` embeds inside a
/// `key:` string; SOME/IP uses a typed `uint16_t` instance selector).
/// Tagged serialisation (`{"kind":"zenoh", "placeholders":[...]}`)
/// keeps the minijinja template branch unambiguous.
///
/// Call sites dispatch by variant; every field a consumer reads is
/// populated by construction, so there is no "pool requested but
/// half-wired" middle state like the old `placeholder_names` + `instances`
/// pair would permit.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PoolPlan {
    /// Zenoh open pool: `key:` carries one or more `{name}` placeholder
    /// tokens; codegen emits `zenoh::KeyExpr(std::string(prefix) + ...)`
    /// assembling the runtime address from named `<param>` values.
    Zenoh {
        /// Placeholder identifiers in declaration order (sorted). Each
        /// must be supplied as a `<param name="<id>">` on every using
        /// invoke / send site — enforced by
        /// [`validate_pool_param_names`].
        placeholders: Vec<String>,
    },
    /// SOME/IP bounded pool: `instance_from: <param-name>` names the
    /// `<param>` whose runtime value feeds `message->set_instance(...)`,
    /// validated against the finite `instances:` list before dispatch.
    Someip {
        /// `<param>` name whose value becomes the instance_id at send
        /// time. Enforced present on every using invoke / send site by
        /// [`validate_pool_param_names`].
        instance_from: String,
        /// Finite instance set pre-registered at init via
        /// `request_service(SERVICE, i)`. Runtime values outside the
        /// list fail fast with `RpcStatus::Unavailable`.
        instances: Vec<u16>,
    },
}

impl PoolPlan {
    /// The set of `<param>` names that every using invoke / send site
    /// must supply for this pool to resolve. Consumed by
    /// [`validate_pool_param_names`]; a missing name is a build error.
    pub fn required_param_names(&self) -> Vec<&str> {
        match self {
            Self::Zenoh { placeholders } => {
                placeholders.iter().map(String::as_str).collect()
            }
            Self::Someip { instance_from, .. } => vec![instance_from.as_str()],
        }
    }
}

/// Build-time warning about dynamic targets that cannot be statically resolved.
#[derive(Debug, Clone)]
pub struct TopologyWarning {
    /// State in which the dynamic target was found.
    pub state: String,
    /// The targetexpr attribute value.
    pub targetexpr: String,
}

impl std::fmt::Display for TopologyWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "state '{}': <send targetexpr=\"{}\"> cannot be statically resolved. \
             This target will not appear in generated transport routing code",
            self.state, self.targetexpr
        )
    }
}

/// Informational notice emitted when SCE_MESH.md §9.5 deadline precedence
/// silently overrides a deploy.yaml binding-level value with a per-invoke
/// `<param name="_mesh_deadline_ms">`. The override itself is the spec's
/// expected usage (per-invoke wins); the notice surfaces the divergence
/// so the deploy.yaml author can see whether the binding-level fallback
/// is still meaningful for this target. Not an error — printed to stderr,
/// not propagated to MeshError.
#[derive(Debug, Clone)]
pub struct DeadlineOverrideNotice {
    /// State that hosts the `<invoke>` whose `<param>` won.
    pub state: String,
    /// Resolved mesh target the invoke fires against (e.g. `#motor`).
    pub target: TargetId,
    /// SCXML invoke id (parser-assigned, e.g. `_invoke_0`).
    pub invoke_id: String,
    /// Per-invoke `<param name="_mesh_deadline_ms">` value (the winner).
    pub param_value: u64,
    /// deploy.yaml `bindings.<target>.deadline_ms` value (the fallback).
    pub binding_value: u64,
}

impl std::fmt::Display for DeadlineOverrideNotice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "state '{}', invoke '{}' targeting {}: per-invoke \
             _mesh_deadline_ms={}ms overrides deploy.yaml \
             binding deadline_ms={}ms (per SCE_MESH.md §9.5 precedence; \
             remove either value if the override is unintended)",
            self.state,
            self.invoke_id,
            self.target.as_str(),
            self.param_value,
            self.binding_value
        )
    }
}

/// Result of [`resolve_partials`] / [`build_resolved_targets`]: the
/// payload (targets) plus any informational notices the resolution
/// stage produced. Named struct rather than a tuple so call sites read
/// `outcome.targets` / `outcome.deadline_overrides` instead of
/// positional `.0` / `.1` — keeps intent explicit when the second
/// vector is forwarded several layers up to the CLI.
#[derive(Debug, Clone, Default)]
pub struct TargetResolution {
    /// Per-target resolved binding state, in deterministic
    /// `summary.targets` (BTreeSet) iteration order.
    pub targets: Vec<ResolvedTarget>,
    /// SCE_MESH.md §9.5 deadline-precedence notices emitted when a
    /// per-invoke value silently overrode a binding-level fallback.
    /// Empty in the common case where the two agree (or only one is
    /// present).
    pub deadline_overrides: Vec<DeadlineOverrideNotice>,
}

// ── Common action traversal ──────────────────────────────────

/// Visit every `<send>` action in the model, providing state ID and action ref.
/// Single traversal point for entry/exit blocks, transitions, and initial actions.
fn for_each_send_action<F>(model: &SCXMLModel, mut visitor: F)
where
    F: FnMut(&str, &crate::model::Action),
{
    for (state_id, state) in &model.states {
        let mut visit_actions = |actions: &[crate::model::Action]| {
            for action in actions {
                if action.action_type == "send" {
                    visitor(state_id, action);
                }
            }
        };
        for block in &state.on_entry_blocks {
            visit_actions(block);
        }
        for block in &state.on_exit_blocks {
            visit_actions(block);
        }
        for transition in &state.transitions {
            visit_actions(&transition.actions);
        }
        visit_actions(&state.initial_transition_actions);
    }
}

// W3C internal targets (#_parent, #_child, …) are identified by
// `TargetId::is_internal()` — the previous free helper `is_internal_target`
// was removed when the newtype migration centralised target semantics.

// ── Server role detection (SCE_MESH.md §13 Session E) ───────

/// A confirmed server RPC pair: the machine transitions on a
/// `service.request.X` event AND produces a matching `service.response.X`
/// somewhere in its actions (send or raise). This pairing is the build-time
/// evidence that the machine is an RPC server for `X`.
#[derive(Debug, Clone, Serialize)]
pub struct ServerRpcPair {
    /// Inbound request event (e.g. `"service.request.compute_force"`).
    pub request_event: String,
    /// Outbound response event (e.g. `"service.response.compute_force"`).
    pub response_event: String,
}

/// Which field access operation a confirmed server pair handles. SOME/IP
/// codegen dispatches on this to pick between `SOMEIP_SERVER_GETTER_*` and
/// `SOMEIP_SERVER_SETTER_*` method constants; the transport reply path
/// (Zenoh `Query::reply`, SOME/IP `create_response`) is identical across
/// both kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldAccessKind {
    Getter,
    Setter,
}

/// A confirmed server field access pair: the machine transitions on a
/// `field.get.X` (or `field.set.X`) event AND produces a matching
/// `field.notify.X` somewhere in its actions (send or raise). Structurally
/// parallel to [`ServerRpcPair`] — both patterns share the
/// request/response injection + transport reply path (SCE_MESH.md §8.3).
#[derive(Debug, Clone, Serialize)]
pub struct ServerFieldAccessPair {
    /// Inbound request event (e.g. `"field.get.vehicle_speed"`).
    pub request_event: String,
    /// Outbound response event (e.g. `"field.notify.vehicle_speed"`).
    pub response_event: String,
    /// Whether this pair handles a field read (Getter) or write (Setter).
    pub kind: FieldAccessKind,
}

/// A server-side eventgroup notification event: the machine publishes
/// this event spontaneously (without a preceding getter/setter request)
/// via the transport's native pub/sub mechanism — SOME/IP
/// `offer_event` + `notify`, Zenoh `session.put`.
///
/// SCE_MESH.md §8.1: eventgroup notifications are declared in the
/// `server.events` block of deploy.yaml with an `event_group:` binding.
/// The SCXML model raises the event from any transition; the build-time
/// `inject_server_response_sends` appends a synthetic `<send>` so the
/// transport's publish path fires.
#[derive(Debug, Clone, Serialize)]
pub struct ServerEventgroupEvent {
    /// Event name to publish (e.g. `"field.notify.vehicle_speed"`).
    pub event: String,
}

/// Resolved server-side binding — the transport-specific state needed to
/// register the machine as a transport-native server. Produced by
/// [`resolve_server_binding`] and consumed by codegen to emit
/// `offer_service` / `declare_queryable` + response handlers.
#[derive(Debug, Clone, Serialize)]
pub struct ServerBinding {
    /// Confirmed RPC pairs detected from the SCXML model.
    pub rpc_pairs: Vec<ServerRpcPair>,
    /// `service.fire_forget.X` events the server must accept from
    /// clients. Detected by [`detect_server_fire_forget_events`] —
    /// inbound-only, no paired response event. SCE_MESH.md §8.3 claims
    /// FireForget is realized end-to-end, which requires the server
    /// transport to install a receive handler
    /// (SOME/IP `register_message_handler` with MT_REQUEST and no
    /// response, Zenoh `declare_subscriber` on the server key).
    pub fire_forget_events: Vec<String>,
    /// Confirmed field access pairs detected from the SCXML model.
    /// SCE_MESH.md §8.3: `field.get.X`/`field.set.X` requests are paired
    /// with `field.notify.X` replies by convention. Codegen emits a
    /// server handler that forwards the request to the engine and stashes
    /// the transport-native request handle; the paired
    /// `<raise event="field.notify.X">` flows through
    /// `inject_server_response_sends` + `handleServerResponse` to reply.
    pub field_access_pairs: Vec<ServerFieldAccessPair>,
    /// Server-initiated eventgroup notification events (SCE_MESH.md §8.1).
    /// These events are published spontaneously (not in response to a
    /// request) via SOME/IP `offer_event` + `notify` or Zenoh
    /// `session.put`. Declared in deploy.yaml `server.events` with
    /// `event_group:` binding. Coexists with the FieldNotify piggyback
    /// reply path — the mesh send callback discriminates at runtime by
    /// `correlation_id` presence.
    pub eventgroup_events: Vec<ServerEventgroupEvent>,
    /// Transport-specific state (Someip or Zenoh variant) carrying the
    /// server's identity (service IDs / key expression).
    pub state: TransportState,
    /// Per-event pattern metadata for the inbound request events.
    pub event_patterns: Vec<EventPatternInfo>,
    /// SCE Mesh §9.5 gap Z2: per-server Zenoh queryable response
    /// deadline. Propagated verbatim from [`super::deploy::ServerConfig`].
    /// `None` ⇒ no deadline armed (pre-Z2 behaviour); `Some` ⇒ codegen
    /// arms a `MeshDeadlineScheduler` entry at every
    /// `pending_server_queries_` insert and cancels it at
    /// `handleServerResponse`.
    ///
    /// **Zenoh-only scope**: this field only has meaning when the
    /// resolved `state` is [`TransportState::Zenoh`]. Parse-time
    /// validation in [`super::deploy::ServerConfig`] rejects the
    /// knob on non-zenoh transports, so by the time codegen reads
    /// this field the transport invariant holds. SOME/IP server-side
    /// response lifecycles are tracked separately under
    /// `mesh_someip_sd_gaps_roadmap.md` and will land under their
    /// own knob rather than overloading this one.
    pub query_timeout_ms: Option<u64>,
}

/// Detect server RPC pairs from an SCXML model.
///
/// SCE_MESH.md §13 Session E: a machine is a confirmed RPC server for
/// suffix `X` iff:
///   1. It has a `<transition event="service.request.X">` in any state.
///   2. Somewhere in its actions (send or raise across all states), it
///      produces `service.response.X`.
///
/// The detection is static and model-wide — no control-flow reachability
/// analysis is performed. This is conservative: if the response event
/// exists ANYWHERE in the model, the pair is confirmed. A future session
/// may tighten this to reachability from the request-handling state.
/// Collect every send/raise action's `<pattern>.X` suffix across the model.
///
/// Shared scanner for server-role response detection: the server raises a
/// response event (`service.response.X`, `field.notify.X`) from any state,
/// and the detector needs a model-wide suffix set to pair against inbound
/// request transitions. Only non-empty suffixes are returned (the bare
/// pattern prefix carries no suffix identity to match).
fn collect_response_suffixes(
    model: &SCXMLModel,
    pattern: super::pattern::CommunicationPattern,
) -> std::collections::HashSet<String> {
    let mut suffixes: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut scan = |actions: &[crate::model::Action]| {
        for action in actions {
            if action.action_type != "send" && action.action_type != "raise" {
                continue;
            }
            if let Some(suffix) = pattern.match_suffix(&action.event) {
                if !suffix.is_empty() {
                    suffixes.insert(suffix.to_string());
                }
            }
        }
    };
    for state in model.states.values() {
        for block in &state.on_entry_blocks {
            scan(block);
        }
        for block in &state.on_exit_blocks {
            scan(block);
        }
        for transition in &state.transitions {
            scan(&transition.actions);
        }
        scan(&state.initial_transition_actions);
    }
    suffixes
}

pub fn detect_server_pairs(model: &SCXMLModel) -> Vec<ServerRpcPair> {
    use super::pattern::CommunicationPattern;

    // Step 1: collect all service.request.* events from transitions
    let mut request_suffixes: Vec<(String, String)> = Vec::new(); // (suffix, full_event)
    for state in model.states.values() {
        for transition in &state.transitions {
            if let Some(suffix) = CommunicationPattern::ServiceRequest.match_suffix(&transition.event) {
                if !suffix.is_empty() {
                    request_suffixes.push((suffix.to_string(), transition.event.clone()));
                }
            }
        }
    }
    if request_suffixes.is_empty() {
        return Vec::new();
    }

    // Step 2: collect all service.response.* events from sends and raises
    let response_suffixes =
        collect_response_suffixes(model, CommunicationPattern::ServiceResponse);

    // Step 3: pair request suffixes with confirmed response suffixes
    let mut pairs = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for (suffix, request_event) in &request_suffixes {
        if response_suffixes.contains(suffix) && seen.insert(suffix.clone()) {
            let response_event = format!(
                "{}.{}",
                CommunicationPattern::ServiceResponse.prefix_str(),
                suffix
            );
            pairs.push(ServerRpcPair {
                request_event: request_event.clone(),
                response_event,
            });
        }
    }
    pairs
}

/// Detect `service.fire_forget.X` events the server-role machine must
/// accept from clients.
///
/// SCE_MESH.md §8.3: `service.fire_forget` is realized end-to-end across
/// transports. When a machine is declared as a server in deploy.yaml AND
/// has a `<transition event="service.fire_forget.X">`, the transport
/// layer must install an inbound handler (SOME/IP
/// `register_message_handler` with `MT_REQUEST` and no response, Zenoh
/// `declare_subscriber` on the server key).
///
/// Detection mirrors [`detect_server_pairs`] — static, model-wide — but
/// requires no outbound pairing because FireForget is one-way by
/// definition.
pub fn detect_server_fire_forget_events(model: &SCXMLModel) -> Vec<String> {
    use super::pattern::CommunicationPattern;
    let mut events = std::collections::BTreeSet::new();
    for state in model.states.values() {
        for transition in &state.transitions {
            if let Some(suffix) =
                CommunicationPattern::FireForget.match_suffix(&transition.event)
            {
                if !suffix.is_empty() {
                    events.insert(transition.event.clone());
                }
            }
        }
    }
    events.into_iter().collect()
}

/// Detect server FieldAccess pairs from an SCXML model.
///
/// SCE_MESH.md §8.3: a machine is a confirmed FieldAccess server for
/// suffix `X` iff:
///   1. It has a `<transition event="field.get.X">` or
///      `<transition event="field.set.X">` in any state.
///   2. Somewhere in its actions (send or raise across all states), it
///      produces `field.notify.X`.
///
/// Detection mirrors [`detect_server_pairs`] — static, model-wide. The
/// conservative pairing rule (the response event must exist ANYWHERE in
/// the model, not only reachable from the handling state) matches the
/// RPC case; tightening to reachability is a future refinement.
///
/// Getter and setter roles are disambiguated by the request event prefix
/// so codegen can pick the `SOMEIP_SERVER_GETTER_*` vs
/// `SOMEIP_SERVER_SETTER_*` method constants. A suffix that happens to
/// appear in both FieldGet and FieldSet transitions yields two distinct
/// pairs (one Getter, one Setter) — that is the authored contract.
pub fn detect_server_field_access_pairs(
    model: &SCXMLModel,
) -> Vec<ServerFieldAccessPair> {
    use super::pattern::CommunicationPattern;

    // Step 1: collect (kind, suffix, full_event) for every field.get/field.set
    // transition in the model.
    let mut requests: Vec<(FieldAccessKind, String, String)> = Vec::new();
    for state in model.states.values() {
        for transition in &state.transitions {
            if let Some(suffix) = CommunicationPattern::FieldGet.match_suffix(&transition.event) {
                if !suffix.is_empty() {
                    requests.push((
                        FieldAccessKind::Getter,
                        suffix.to_string(),
                        transition.event.clone(),
                    ));
                }
            } else if let Some(suffix) =
                CommunicationPattern::FieldSet.match_suffix(&transition.event)
            {
                if !suffix.is_empty() {
                    requests.push((
                        FieldAccessKind::Setter,
                        suffix.to_string(),
                        transition.event.clone(),
                    ));
                }
            }
        }
    }
    if requests.is_empty() {
        return Vec::new();
    }

    // Step 2: collect all field.notify.* suffixes emitted as send or raise
    // action events anywhere in the model.
    let response_suffixes =
        collect_response_suffixes(model, CommunicationPattern::FieldNotify);

    // Step 3: pair each (kind, suffix) with a confirmed field.notify suffix.
    // The dedup key is (kind, suffix) so a suffix that appears as both a
    // getter and a setter produces two distinct pairs without dropping one.
    let mut pairs = Vec::new();
    let mut seen: std::collections::HashSet<(FieldAccessKind, String)> =
        std::collections::HashSet::new();
    for (kind, suffix, request_event) in &requests {
        if !response_suffixes.contains(suffix) {
            continue;
        }
        let key = (*kind, suffix.clone());
        if !seen.insert(key) {
            continue;
        }
        let response_event = format!(
            "{}.{}",
            CommunicationPattern::FieldNotify.prefix_str(),
            suffix
        );
        pairs.push(ServerFieldAccessPair {
            request_event: request_event.clone(),
            response_event,
            kind: *kind,
        });
    }
    pairs
}

/// Detect server-side eventgroup notification events from deploy.yaml.
///
/// SCE_MESH.md §8.1: the `server.events` block in deploy.yaml declares
/// events the machine can publish spontaneously (without a preceding
/// request) via the transport's native pub/sub mechanism. Detection is
/// transport-aware:
///
///   - **SOME/IP**: requires `event_group:` binding (name-resolved to
///     `event_group_id` + `event_id` via vsomeip.json). The `event_group`
///     field is the declaration — without it, the event is not offered.
///   - **Zenoh**: has no eventgroup concept. Instead, any `field.notify.*`
///     or `event.notification.*` event declared in `server.events` is
///     recognized as an eventgroup publish event. The event name prefix
///     is the declaration; no `event_group:` field required.
///
/// Events that also appear in field_access_pairs (e.g.
/// `field.notify.vehicle_speed` with both a getter and an eventgroup
/// binding) are included — both paths coexist at runtime, discriminated
/// by `correlation_id` presence.
pub fn detect_server_eventgroup_events(
    server_cfg: &super::deploy::ServerConfig,
) -> Vec<ServerEventgroupEvent> {
    use super::pattern::CommunicationPattern;
    let is_publish_event = |event_name: &str| -> bool {
        CommunicationPattern::FieldNotify
            .match_suffix(event_name)
            .is_some()
            || CommunicationPattern::Notification
                .match_suffix(event_name)
                .is_some()
    };

    server_cfg
        .events
        .iter()
        .filter(|(event_name, binding)| {
            if server_cfg.transport == "someip" {
                // SOME/IP: event_group binding required (resolves to IDs).
                binding.event_group.is_some()
            } else {
                // Zenoh and others: event name prefix is the declaration.
                is_publish_event(event_name)
            }
        })
        .map(|(event_name, _binding)| ServerEventgroupEvent {
            event: event_name.clone(),
        })
        .collect()
}

/// Inject synthetic `<send>` actions for server response events.
///
/// SCE_MESH.md §13 Session E / §8.3: the SCXML spec says server machines
/// use `<raise event="service.response.X">` or `<raise event="field.notify.X">`
/// for responses. `<raise>` puts events in the internal queue only — the
/// mesh send callback does not fire. This function injects a synthetic
/// `<send>` action alongside each qualifying raise, so the transport
/// layer receives the response event through the mesh send callback.
///
/// The injected send targets `#{scxml_name}` (self-target). The topology
/// pipeline exempts this target from binding resolution and coverage
/// validation (it is a server-internal route, not a cross-machine send).
///
/// `response_events` is the full set of event names that should be
/// intercepted — the caller merges RPC response events and FieldAccess
/// notify events (and any future server-response kind) into a single set
/// so this function owns one responsibility: "after any raise of a known
/// response event, append a synthetic self-send".
///
/// Follows the same two-pass pattern as [`inject_auto_subscriptions`]:
/// collect candidates (immutable), then inject (mutable).
///
/// Must be called BEFORE `collect_send_summary` so the synthetic sends
/// are visible to pattern detection.
pub fn inject_server_response_sends(
    model: &mut crate::model::SCXMLModel,
    response_events: &std::collections::HashSet<String>,
) -> Vec<String> {
    if response_events.is_empty() {
        return Vec::new();
    }

    // The self-target for injected sends. Uses the SCXML name attribute
    // (e.g., "motor") so the target is `#motor`. Falls back to model.name
    // (file stem) if no SCXML name is declared.
    let scxml_name = if model.scxml_name.is_empty() {
        &model.name
    } else {
        &model.scxml_name
    };
    let self_target = format!("#{scxml_name}");

    // Adapt the caller's `HashSet<String>` to the `&str` probe used below.
    let response_events: std::collections::HashSet<&str> =
        response_events.iter().map(String::as_str).collect();

    // Pass 1: collect (state_id, block_index, action_index) for qualifying raises.
    struct Candidate {
        state_id: String,
        block_kind: BlockKind,
        block_idx: usize,
        action_idx: usize,
        event: String,
    }
    #[derive(Clone, Copy)]
    enum BlockKind { OnEntry, OnExit, Transition(usize) }

    // Idempotency guard: a raise is only a candidate if no synthetic
    // send with the same event and self-target already follows it. This
    // allows the function to be called multiple times (pre-SM and inside
    // compile_mesh_transport) without inserting duplicate sends.
    let already_followed_by_send = |actions: &[crate::model::Action], ai: usize, event: &str| -> bool {
        if ai + 1 < actions.len() {
            let next = &actions[ai + 1];
            next.action_type == "send" && next.event == event && next.target == self_target
        } else {
            false
        }
    };

    let mut candidates: Vec<Candidate> = Vec::new();

    for (state_id, state) in &model.states {
        for (bi, block) in state.on_entry_blocks.iter().enumerate() {
            for (ai, action) in block.iter().enumerate() {
                if action.action_type == "raise" && response_events.contains(action.event.as_str())
                    && !already_followed_by_send(block, ai, &action.event)
                {
                    candidates.push(Candidate {
                        state_id: state_id.clone(),
                        block_kind: BlockKind::OnEntry,
                        block_idx: bi,
                        action_idx: ai,
                        event: action.event.clone(),
                    });
                }
            }
        }
        for (bi, block) in state.on_exit_blocks.iter().enumerate() {
            for (ai, action) in block.iter().enumerate() {
                if action.action_type == "raise" && response_events.contains(action.event.as_str())
                    && !already_followed_by_send(block, ai, &action.event)
                {
                    candidates.push(Candidate {
                        state_id: state_id.clone(),
                        block_kind: BlockKind::OnExit,
                        block_idx: bi,
                        action_idx: ai,
                        event: action.event.clone(),
                    });
                }
            }
        }
        for (ti, transition) in state.transitions.iter().enumerate() {
            for (ai, action) in transition.actions.iter().enumerate() {
                if action.action_type == "raise" && response_events.contains(action.event.as_str())
                    && !already_followed_by_send(&transition.actions, ai, &action.event)
                {
                    candidates.push(Candidate {
                        state_id: state_id.clone(),
                        block_kind: BlockKind::Transition(ti),
                        block_idx: 0,
                        action_idx: ai,
                        event: action.event.clone(),
                    });
                }
            }
        }
    }

    // Pass 2: inject synthetic sends AFTER each qualifying raise.
    // Process in reverse order so insertion indices remain valid.
    let mut injected = Vec::new();
    candidates.sort_by(|a, b| {
        a.state_id.cmp(&b.state_id)
            .then(b.action_idx.cmp(&a.action_idx)) // reverse action index
    });

    for c in &candidates {
        let state = model.states.get_mut(&c.state_id).expect(
            "state must exist — collected from the same model",
        );

        let mut send_action = crate::model::Action::default();
        send_action.action_type = "send".to_string();
        send_action.event = c.event.clone();
        send_action.target = self_target.clone();

        let insert_pos = c.action_idx + 1;
        match c.block_kind {
            BlockKind::OnEntry => {
                state.on_entry_blocks[c.block_idx].insert(insert_pos, send_action);
            }
            BlockKind::OnExit => {
                state.on_exit_blocks[c.block_idx].insert(insert_pos, send_action);
            }
            BlockKind::Transition(ti) => {
                state.transitions[ti].actions.insert(insert_pos, send_action);
            }
        }
        injected.push(c.event.clone());
    }

    injected
}

/// Resolve a deploy.yaml `server:` section into a [`ServerBinding`].
///
/// Applies the same external-config resolution pipeline as client bindings:
/// SOME/IP service names resolve against vsomeip.json, per-event method
/// names resolve to numeric IDs. The result is a fully typed transport
/// state ready for codegen.
pub fn resolve_server_binding(
    server_cfg: &super::deploy::ServerConfig,
    pairs: &[ServerRpcPair],
    fire_forget_events: &[String],
    field_access_pairs: &[ServerFieldAccessPair],
    eventgroup_events: &[ServerEventgroupEvent],
    machine_name: &str,
    external: &super::external::ExternalResolution,
) -> Result<ServerBinding, TopologyError> {
    use super::pattern::CommunicationPattern;

    // Build event patterns from RPC pairs + FireForget events + FieldAccess
    // pairs (all inbound request events). The response/notify events
    // themselves do not appear in `event_patterns` — they flow through
    // `handleServerResponse`, not the inbound receive path.
    let mut event_patterns: Vec<EventPatternInfo> = pairs
        .iter()
        .map(|pair| EventPatternInfo {
            event: pair.request_event.clone(),
            pattern_kind_value: CommunicationPattern::ServiceRequest.wire_value(),
            reply_event: Some(pair.response_event.clone()),
        })
        .collect();
    for event in fire_forget_events {
        event_patterns.push(EventPatternInfo {
            event: event.clone(),
            pattern_kind_value: CommunicationPattern::FireForget.wire_value(),
            reply_event: None,
        });
    }
    for pair in field_access_pairs {
        let pattern = match pair.kind {
            FieldAccessKind::Getter => CommunicationPattern::FieldGet,
            FieldAccessKind::Setter => CommunicationPattern::FieldSet,
        };
        event_patterns.push(EventPatternInfo {
            event: pair.request_event.clone(),
            pattern_kind_value: pattern.wire_value(),
            reply_event: Some(pair.response_event.clone()),
        });
    }

    // Build transport state from server config
    let state = build_server_transport_state(server_cfg, machine_name, external)?;

    // SOME/IP requires deploy.yaml to declare a method/getter/setter
    // binding for every inbound event so the external resolver produces
    // a numeric ID. Zenoh shares the single server key across every event
    // (subscriber/queryable callback dispatches by `env.type`), so it
    // needs no per-event binding.
    if server_cfg.transport == "someip" {
        for event in fire_forget_events {
            let has_method_binding = match &state {
                TransportState::Someip { event_bindings, .. } => matches!(
                    event_bindings.get(event),
                    Some(SomeipEventIds::Method { .. })
                ),
                _ => false,
            };
            if !has_method_binding {
                let server_target =
                    TargetId::new("#_server").expect("static target ID");
                return Err(TopologyError::MissingBindingField {
                    machine: machine_name.to_string(),
                    target: server_target,
                    transport: "someip".to_string(),
                    field: format!(
                        "server.events.\"{event}\".method (required for FireForget server handler)"
                    ),
                });
            }
        }
        for pair in field_access_pairs {
            let (expected, field_label) = match pair.kind {
                FieldAccessKind::Getter => ("getter", "getter"),
                FieldAccessKind::Setter => ("setter", "setter"),
            };
            let has_binding = match &state {
                TransportState::Someip { event_bindings, .. } => match event_bindings
                    .get(&pair.request_event)
                {
                    Some(SomeipEventIds::Getter { .. }) if expected == "getter" => true,
                    Some(SomeipEventIds::Setter { .. }) if expected == "setter" => true,
                    _ => false,
                },
                _ => false,
            };
            if !has_binding {
                let server_target =
                    TargetId::new("#_server").expect("static target ID");
                return Err(TopologyError::MissingBindingField {
                    machine: machine_name.to_string(),
                    target: server_target,
                    transport: "someip".to_string(),
                    field: format!(
                        "server.events.\"{}\".{field_label} (required for FieldAccess server handler)",
                        pair.request_event
                    ),
                });
            }
        }
        // Eventgroup events require an event_group binding so the external
        // resolver produces event_group_id + event_id.
        for eg in eventgroup_events {
            let has_eventgroup_binding = match &state {
                TransportState::Someip { event_bindings, .. } => matches!(
                    event_bindings.get(&eg.event),
                    Some(SomeipEventIds::EventGroup { .. })
                ),
                _ => false,
            };
            if !has_eventgroup_binding {
                let server_target =
                    TargetId::new("#_server").expect("static target ID");
                return Err(TopologyError::MissingBindingField {
                    machine: machine_name.to_string(),
                    target: server_target,
                    transport: "someip".to_string(),
                    field: format!(
                        "server.events.\"{}\".event_group (required for eventgroup notification)",
                        eg.event
                    ),
                });
            }
        }
    }

    Ok(ServerBinding {
        rpc_pairs: pairs.to_vec(),
        fire_forget_events: fire_forget_events.to_vec(),
        field_access_pairs: field_access_pairs.to_vec(),
        eventgroup_events: eventgroup_events.to_vec(),
        state,
        event_patterns,
        query_timeout_ms: server_cfg.query_timeout_ms,
    })
}

/// Build the server-side [`TransportState`] from a `ServerConfig`.
///
/// SOME/IP: service IDs and per-event method IDs come from
/// `ExternalResolution.server_bindings` (already resolved in external.rs).
/// Zenoh: key expression is taken directly from deploy.yaml.
fn build_server_transport_state(
    server_cfg: &super::deploy::ServerConfig,
    machine_name: &str,
    external: &super::external::ExternalResolution,
) -> Result<TransportState, TopologyError> {
    let server_target = TargetId::new("#_server").expect("static target ID");

    match server_cfg.transport.as_str() {
        "someip" => {
            // Service identity + per-event IDs already resolved by
            // external.rs into `server_bindings[machine_name]`.
            let resolution = external.server_bindings.get(machine_name).ok_or_else(|| {
                TopologyError::MissingBindingField {
                    machine: machine_name.to_string(),
                    target: server_target.clone(),
                    transport: "someip".to_string(),
                    field: "service (required for server-side SOME/IP)".to_string(),
                }
            })?;

            Ok(TransportState::Someip {
                service: resolution.service_ids,
                event_bindings: resolution.by_event.clone(),
                extra: server_cfg.extra.clone(),
            })
        }
        "zenoh" => {
            let key = server_cfg.key.clone().ok_or_else(|| {
                TopologyError::MissingBindingField {
                    machine: machine_name.to_string(),
                    target: server_target,
                    transport: "zenoh".to_string(),
                    field: "key (required for server-side Zenoh queryable)".to_string(),
                }
            })?;
            Ok(TransportState::Zenoh {
                key,
                extra: server_cfg.extra.clone(),
            })
        }
        other => Err(TopologyError::MissingBindingField {
            machine: machine_name.to_string(),
            target: server_target,
            transport: other.to_string(),
            field: "server transport must be 'someip' or 'zenoh'".to_string(),
        }),
    }
}

// ── Single-pass send action collection ──────────────────────

/// Details of a single `<send>` action, collected for downstream validators.
/// SCE_MESH.md §13 path B: no longer carries sce:* attribute values — pattern
/// and reply pairing are derived from the event name by the analyzer.
///
/// Only actions with a non-empty `target` attribute are captured; empty-target
/// (e.g. `targetexpr`-only) sends produce `TopologyWarning` instead.
#[derive(Debug, Clone)]
pub struct SendActionDetail {
    /// The state containing the `<send>`.
    pub state: String,
    /// The `target` attribute (e.g. "#motor"), non-empty by construction.
    pub target: TargetId,
    /// The `event` attribute.
    pub event: String,
    /// Pattern detected at summary-collection time from the event name.
    /// `None` for application-specific events (no reserved prefix match).
    /// Caching here avoids re-running `detect_pattern` in every consumer.
    pub pattern: Option<super::pattern::CommunicationPattern>,
}

/// Pre-collected `<send>` action data from a single model traversal.
///
/// Created by `collect_send_summary()` and consumed by `resolve_targets()`,
/// `validate_pattern_capability()`, and `check_sender_event_coverage()`.
/// Eliminates redundant model traversals.
#[derive(Debug)]
pub struct SendActionSummary {
    /// Deduplicated external targets from both `<send target="#X">`
    /// and `<invoke type="sce:mesh-rpc" src="#X">`. Unified in one
    /// set so the deploy.yaml resolution pipeline handles every
    /// cross-machine interaction through a single path; a target
    /// reached only by mesh-rpc invokes is just as "external" as one
    /// reached only by `<send>`.
    pub targets: BTreeSet<TargetId>,
    /// Dynamic target warnings (`targetexpr` cannot be statically resolved).
    pub dynamic_warnings: Vec<TopologyWarning>,
    /// (target, event) pairs for event coverage validation.
    pub target_events: Vec<(TargetId, String)>,
    /// Per-action details for QoS and pattern validation.
    pub actions: Vec<SendActionDetail>,
    /// Mesh-RPC invoke sites keyed by the target their `src="#X"`
    /// resolves to. Consumed by [`resolve_partials`] — each partial
    /// target pops its bucket and carries the sites through to
    /// [`ResolvedTarget::invoke_sites`]. Absent targets yield an
    /// empty site list (i.e. a pure-`<send>` target).
    pub invoke_sites_by_target: BTreeMap<TargetId, Vec<MeshRpcInvokeSite>>,
}

/// Collect all `<send>` action data from an SCXML model in a single pass.
///
/// Replaces five separate `for_each_send_action` traversals with one.
/// Each downstream validator reads from the summary instead of re-traversing.
pub fn collect_send_summary(model: &SCXMLModel) -> SendActionSummary {
    let mut targets = BTreeSet::new();
    let mut dynamic_warnings = Vec::new();
    let mut target_events = Vec::new();
    let mut actions = Vec::new();

    for_each_send_action(model, |state_id, action| {
        // Dynamic target warning (targetexpr present)
        if !action.targetexpr.is_empty() {
            dynamic_warnings.push(TopologyWarning {
                state: state_id.to_string(),
                targetexpr: action.targetexpr.clone(),
            });
        }

        // Empty-target sends (e.g. targetexpr-only) produce a warning above
        // but contribute no SendActionDetail — the downstream validators
        // skip them anyway.
        let Some(tid) = TargetId::new(&action.target) else {
            return;
        };

        // External (non-internal) static targets feed deploy.yaml resolution
        // and event-coverage analysis.
        if !tid.is_internal() {
            targets.insert(tid.clone());
            target_events.push((tid.clone(), action.event.clone()));
        }

        // Per-action details for pattern validation (includes internal
        // targets; validators filter them via TargetId::is_internal()).
        actions.push(SendActionDetail {
            state: state_id.to_string(),
            target: tid,
            event: action.event.clone(),
            pattern: super::pattern::detect_pattern(&action.event),
        });
    });

    // SCE Mesh §9.5: collect `<invoke type="sce:mesh-rpc">` sites.
    // Each invoke's `src="#target"` becomes an external target that
    // goes through the same deploy.yaml resolution as `<send>`
    // targets — the transport selects the RPC wire (SOME/IP method
    // call, Zenoh get/reply) and the deadline binding supplies any
    // fallback for missing per-invoke deadlines (§9.5 precedence).
    let mut invoke_sites_by_target: BTreeMap<TargetId, Vec<MeshRpcInvokeSite>> = BTreeMap::new();
    for invoke in &model.invokes {
        let Invoke::MeshRpc(info) = invoke else {
            continue;
        };
        // Only the `Src` variant contributes a build-time-resolvable
        // target. `SrcExpr` is evaluated at `<invoke>` entry; its target
        // cannot be enumerated at build time and is looked up against
        // the existing static topology at runtime — a miss raises
        // `error.invoke.<id>` with `RpcStatus::Unavailable` (§9.5).
        let Some(src_literal) = info.target.src_literal() else {
            continue;
        };
        let Some(tid) = TargetId::new(src_literal) else {
            // `src` malformed — pure-parse problem that should have
            // been flagged earlier. Skip rather than invent a
            // diagnostic in the wrong layer.
            continue;
        };
        if tid.is_internal() {
            // `src="#_internal"` or sibling — not a mesh target. Skip;
            // W3C semantics already cover the internal delivery.
            continue;
        }
        targets.insert(tid.clone());
        invoke_sites_by_target
            .entry(tid)
            .or_default()
            .push(MeshRpcInvokeSite {
                state_name: info.base.state_name.clone(),
                invoke_id: info.base.invoke_id.clone(),
                field_suffix: info.base.field_suffix.clone(),
                mesh_event: info.mesh_event.clone(),
                deadline_ms: info.deadline_ms,
                params: info.base.params.clone(),
            });
    }

    SendActionSummary {
        targets,
        dynamic_warnings,
        target_events,
        actions,
        invoke_sites_by_target,
    }
}

/// Collect (target, event) pairs from an SCXML model.
///
/// Used by `validate_event_coverage` (multi-model API). The per-model
/// pipeline uses `SendActionSummary.target_events` instead.
fn collect_target_events(model: &SCXMLModel) -> Vec<(TargetId, String)> {
    let mut pairs = Vec::new();
    for_each_send_action(model, |_, action| {
        if let Some(tid) = TargetId::new(&action.target) {
            if !tid.is_internal() {
                pairs.push((tid, action.event.clone()));
            }
        }
    });
    pairs
}

// ── Target resolution ────────────────────────────────────────

/// Analyze `<send>` actions and produce per-target event/pattern/reply
/// metadata by pure inference (no deploy.yaml involvement).
///
/// SCE_MESH.md §13 path B decomposition: this pass owns pattern detection
/// and topology-inferred RPC pairing. `resolve_targets` consumes the result
/// and is responsible only for matching targets to deploy.yaml bindings.
/// Separating the two enforces single responsibility: pairing is a property
/// of the SCXML event vocabulary, binding is a property of deployment.
///
/// Returns a map `target → Vec<EventPatternInfo>` deduplicated by event name.
pub fn analyze_event_pairs(
    summary: &SendActionSummary,
) -> std::collections::HashMap<TargetId, Vec<EventPatternInfo>> {
    let mut pattern_map: std::collections::HashMap<TargetId, Vec<EventPatternInfo>> =
        std::collections::HashMap::new();

    for action in &summary.actions {
        if action.target.is_internal() || action.event.is_empty() {
            continue;
        }
        // Spec §14: "everything else → FireForget" — events that don't match
        // a reserved prefix default to FireForget rather than dropping out
        // of the pattern table. Falling out would leave such events with
        // no per-event metadata for codegen to emit, so app-level events
        // (e.g. `brake.activate`) would silently route to no method_id.
        let pattern = action
            .pattern
            .unwrap_or(super::pattern::CommunicationPattern::FireForget);
        let entry = pattern_map.entry(action.target.clone()).or_default();
        if entry.iter().any(|e| e.event == action.event) {
            continue; // Same event to same target — already captured.
        }
        entry.push(EventPatternInfo {
            event: action.event.clone(),
            pattern_kind_value: pattern.wire_value(),
            reply_event: pattern.infer_reply_event(&action.event),
        });
    }

    // SCE_MESH.md §9.5: <invoke type="sce:mesh-rpc"> contributes its
    // `_mesh_event` as an RpcRequest event on the target binding. Without
    // this, a SOME/IP binding targeted only by mesh-rpc invokes has an
    // empty `event_patterns` → no per-event method_id constant is emitted
    // → the §14.4 pool dispatch in invokeMeshRpc finds no method match
    // and silently returns RpcStatus::Unavailable for every invoke. The
    // Zenoh path is independent (ZENOH_KEY_<target> is per-target, not
    // per-event), but the SOME/IP per-event ID resolution is driven off
    // this map so the entry must land here too.
    for (target, sites) in &summary.invoke_sites_by_target {
        for site in sites {
            if site.mesh_event.is_empty() {
                continue;
            }
            let entry = pattern_map.entry(target.clone()).or_default();
            if entry.iter().any(|e| e.event == site.mesh_event) {
                continue;
            }
            let pattern = super::pattern::CommunicationPattern::ServiceRequest;
            entry.push(EventPatternInfo {
                event: site.mesh_event.clone(),
                pattern_kind_value: pattern.wire_value(),
                reply_event: pattern.infer_reply_event(&site.mesh_event),
            });
        }
    }

    pattern_map
}

/// Intermediate target produced by [`resolve_partials`] before the
/// external-config stage has filled in per-event SOME/IP IDs. Internal
/// to topology — it exists precisely to keep `ResolvedTarget` free of the
/// half-built `event_bindings: BTreeMap::new()` state. External callers
/// only ever see `ResolvedTarget`, produced by [`build_resolved_targets`].
#[derive(Debug, Clone)]
pub(crate) struct PartialTarget {
    pub target: TargetId,
    pub events: Vec<String>,
    pub transport: String,
    pub extra: std::collections::HashMap<String, serde_yaml_ng::Value>,
    pub event_patterns: Vec<EventPatternInfo>,
    /// Mesh-RPC invoke sites targeting this binding. Carried through
    /// to the final [`ResolvedTarget`] unchanged — no external-config
    /// stage touches invokes (per-invoke deadlines / event names are
    /// authoritative from `<param>`, no vsomeip.json lookup needed).
    pub invoke_sites: Vec<MeshRpcInvokeSite>,
    /// SCE_MESH.md §10.6 per-binding ordering declaration, copied
    /// verbatim from the `BindingConfig`. Carried through to
    /// [`ResolvedTarget::ordering`] so codegen can compute the
    /// per-binding `needs_ordering` decision without revisiting
    /// deploy.yaml.
    pub ordering: crate::mesh::deploy::OrderingRequirement,
    /// SCE_MESH.md §14.4 — raw `BindingConfig.instance_from` copied
    /// for SOME/IP pool resolution in [`finalize_targets`]. `None`
    /// means no SOME/IP pool is requested at this binding; the exact
    /// SOME/IP validation (transport match, instances pairing) has
    /// already run at parse time in `validate_pool_capability` so
    /// `Some(_)` here implies a SOME/IP binding.
    pub instance_from: Option<String>,
    /// SCE_MESH.md §14.4 — raw `BindingConfig.instances` copied for
    /// SOME/IP pool resolution. `None` / empty means no bounded pool.
    pub instances: Option<Vec<u16>>,
}

/// Resolve SCXML send targets against deploy.yaml bindings for a specific
/// machine. Pre-external stage — produces [`PartialTarget`]s that carry
/// every field independent of vsomeip.json resolution. [`finalize_targets`]
/// then attaches per-event SOME/IP IDs and produces the public
/// [`ResolvedTarget`]s.
///
/// Uses pre-collected targets and target_events from `SendActionSummary`
/// to avoid redundant model traversal. Pattern/pairing metadata is produced
/// by `analyze_event_pairs` and consumed here.
///
/// Returns an error if any SCXML target has no matching binding in deploy.yaml.
pub(crate) fn resolve_partials(
    summary: &SendActionSummary,
    deploy: &DeployConfig,
    machine_name: &str,
) -> Result<(Vec<PartialTarget>, Vec<DeadlineOverrideNotice>), TopologyError> {
    if summary.targets.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }

    // Find the machine's bindings in deploy.yaml topology
    let bindings = find_machine_bindings(deploy, machine_name)?;

    // Build events-per-target map from summary
    let mut events_map: std::collections::HashMap<TargetId, Vec<String>> =
        std::collections::HashMap::new();
    for (target, event) in &summary.target_events {
        events_map
            .entry(target.clone())
            .or_default()
            .push(event.clone());
    }

    // Pattern metadata + RPC reply pairing — pure event-vocabulary inference.
    let mut pattern_map = analyze_event_pairs(summary);

    // Validate: every SCXML target must have a deploy.yaml binding
    let mut unresolved: Vec<TargetId> = Vec::new();
    let mut partials = Vec::new();
    let mut deadline_overrides: Vec<DeadlineOverrideNotice> = Vec::new();

    for target in &summary.targets {
        match bindings.get(target) {
            Some(binding) => {
                // SCE_MESH.md §9.5 deadline precedence:
                //   1. per-invoke <param name="_mesh_deadline_ms">    (authoritative)
                //   2. deploy.yaml bindings.<target>.deadline_ms      (fallback)
                //   3. absent ⇒ no deadline
                // Apply (2) when (1) is None; emit a notice when both
                // are present with different values so the deploy author
                // can see the override took effect.
                let raw_sites = summary
                    .invoke_sites_by_target
                    .get(target)
                    .cloned()
                    .unwrap_or_default();
                let merged_sites = raw_sites
                    .into_iter()
                    .map(|mut site| {
                        match (site.deadline_ms, binding.deadline_ms) {
                            (Some(per), Some(bind)) if per != bind => {
                                deadline_overrides.push(DeadlineOverrideNotice {
                                    state: site.state_name.clone(),
                                    target: target.clone(),
                                    invoke_id: site.invoke_id.clone(),
                                    param_value: per,
                                    binding_value: bind,
                                });
                            }
                            (None, Some(bind)) => {
                                site.deadline_ms = Some(bind);
                            }
                            _ => {}
                        }
                        site
                    })
                    .collect();
                partials.push(PartialTarget {
                    target: target.clone(),
                    events: events_map.remove(target).unwrap_or_default(),
                    transport: binding.transport.clone(),
                    extra: binding.extra.clone(),
                    event_patterns: pattern_map.remove(target).unwrap_or_default(),
                    invoke_sites: merged_sites,
                    ordering: binding.ordering,
                    instance_from: binding.instance_from.clone(),
                    instances: binding.instances.clone(),
                });
            }
            None => {
                unresolved.push(target.clone());
            }
        }
    }

    if !unresolved.is_empty() {
        return Err(TopologyError::UnresolvedTargets {
            machine: machine_name.to_string(),
            targets: unresolved,
        });
    }

    // Validate: each binding provides all fields required by its transport.
    // Without this, missing fields would only surface as C++ #error directives
    // in the generated template — a two-stage failure.
    for pt in &partials {
        if let Some(desc) = super::transport::lookup(&pt.transport) {
            for &field in desc.required_binding_fields {
                if !pt.extra.contains_key(field) {
                    return Err(TopologyError::MissingBindingField {
                        machine: machine_name.to_string(),
                        target: pt.target.clone(),
                        transport: pt.transport.clone(),
                        field: field.to_string(),
                    });
                }
            }
        }

        // Transport-specific optional field validation. These fields are
        // optional (fall back to defaults) but must be well-formed when
        // present. SCE_MESH.md Section 7.5.
        if pt.transport == "shm" {
            validate_shm_extras_partial(machine_name, pt)?;
        }

        // SCE_MESH.md §10.6.2: `ordering: required` on a transport whose
        // broadcast semantics leave no per-(sender, receiver) sequence
        // domain is structurally unrepairable by a runtime buffer.
        // Reject at topology time with an actionable diagnostic rather
        // than deferring to a silently-incorrect OrderingBuffer that
        // cannot distinguish "skipped for me" from "not destined for me".
        if pt.ordering == crate::mesh::deploy::OrderingRequirement::Required {
            if let Some(desc) = super::transport::lookup(&pt.transport) {
                if !desc.ordering_representable {
                    return Err(TopologyError::OrderingCannotBeGuaranteed {
                        machine: machine_name.to_string(),
                        target: pt.target.clone(),
                        transport: pt.transport.clone(),
                    });
                }
            }
        }

        // SOME/IP per-event ID validation runs after `finalize_targets`
        // has fanned the per-binding resolution out into per-event entries.
    }

    Ok((partials, deadline_overrides))
}

/// Public pipeline entry: resolve, attach, validate in a single step so
/// external callers cannot observe (let alone construct) a half-built
/// [`ResolvedTarget`]. Replaces the sequence
/// `resolve_targets → attach_event_bindings → validate_someip_event_fields`
/// that previously required correct caller ordering.
///
/// Returns both the resolved targets and any informational notices the
/// resolution stage produced (currently: deadline overrides per
/// SCE_MESH.md §9.5). Notices are non-fatal and exposed for the CLI to
/// surface to the operator; consumers that only care about the targets
/// can pattern-match `(resolved, _)`.
pub fn build_resolved_targets(
    summary: &SendActionSummary,
    deploy: &DeployConfig,
    machine_name: &str,
    external: &super::external::ExternalResolution,
) -> Result<TargetResolution, TopologyError> {
    let (partials, deadline_overrides) = resolve_partials(summary, deploy, machine_name)?;
    let resolved = finalize_targets(partials, machine_name, external)?;
    validate_someip_event_fields(&resolved, machine_name)?;
    validate_pool_param_names(&resolved, machine_name)?;
    Ok(TargetResolution {
        targets: resolved,
        deadline_overrides,
    })
}

/// SCE_MESH.md §14.4 — cross-reference pool placeholder / instance_from
/// names against every `<invoke type="sce:mesh-rpc">` site targeting a
/// pooled binding. A pool binding that no using invoke supplies
/// substitution values for is a silently-broken deployment (every
/// dispatch would miss at runtime with `RpcStatus::Unavailable`).
/// Detecting at build time pinpoints the (state, invoke_id, missing
/// names) triple so the author can fix one invoke site instead of
/// diagnosing runtime misfires.
///
/// Scope: only `<invoke type="sce:mesh-rpc">` sites are cross-referenced
/// here. `<send>` does not carry `<param>` metadata through
/// [`SendActionSummary`], and the pool dispatch path in `invokeMeshRpc`
/// is the only emission site that consumes pool plans, so there is no
/// other call-site kind to verify. If `<send>` later grows a param-
/// propagating path, extend [`SendActionSummary`] and add a parallel
/// cross-reference branch here.
fn validate_pool_param_names(
    resolved: &[ResolvedTarget],
    machine_name: &str,
) -> Result<(), TopologyError> {
    for rt in resolved {
        let Some(plan) = &rt.pool_plan else {
            continue;
        };
        let required: Vec<&str> = plan.required_param_names();
        for site in &rt.invoke_sites {
            let supplied: std::collections::BTreeSet<&str> = site
                .params
                .iter()
                .map(|p| p.name.as_str())
                .collect();
            let missing: Vec<String> = required
                .iter()
                .filter(|name| !supplied.contains(*name))
                .map(|s| s.to_string())
                .collect();
            if !missing.is_empty() {
                return Err(TopologyError::PoolParamNameMissing {
                    machine: machine_name.to_string(),
                    target: rt.target.clone(),
                    state: site.state_name.clone(),
                    invoke_id: site.invoke_id.clone(),
                    missing,
                });
            }
        }
    }
    Ok(())
}

/// Validate SOME/IP per-event field presence.
///
/// Runs after `finalize_targets`, so it can read the populated
/// `TransportState::Someip { event_bindings, .. }` variant directly.
/// Each SCXML event must have the IDs its communication pattern
/// requires, otherwise the template would silently emit `return false`
/// for that event at runtime.
pub(crate) fn validate_someip_event_fields(
    resolved: &[ResolvedTarget],
    machine_name: &str,
) -> Result<(), TopologyError> {
    for rt in resolved {
        if let TransportState::Someip { event_bindings, .. } = &rt.state {
            validate_someip_pattern_fields(machine_name, rt, event_bindings)?;
        }
    }
    Ok(())
}

/// Validate optional shm binding fields (partial stage — runs before
/// external IDs are attached, so there is no `ResolvedTarget` yet):
///   - `shm_arena_bytes`   positive integer, must fit in u32 (offset/length
///                         fields in the wire layout use uint32_t)
///   - `shm_ring_capacity` positive power of two (EventQueueBridge
///                         requires power-of-two capacity)
fn validate_shm_extras_partial(
    machine_name: &str,
    pt: &PartialTarget,
) -> Result<(), TopologyError> {
    let invalid = |field: &str, reason: String| TopologyError::InvalidBindingField {
        machine: machine_name.to_string(),
        target: pt.target.clone(),
        transport: pt.transport.clone(),
        field: field.to_string(),
        reason,
    };

    if let Some(v) = pt.extra.get("shm_arena_bytes") {
        let n = v.as_u64().ok_or_else(|| {
            invalid(
                "shm_arena_bytes",
                format!("must be a positive integer (got {})", render_yaml_value(v)),
            )
        })?;
        if n == 0 {
            return Err(invalid("shm_arena_bytes", "must be greater than zero".into()));
        }
        if n > u32::MAX as u64 {
            return Err(invalid(
                "shm_arena_bytes",
                format!(
                    "exceeds u32 max ({n} > {}); arena offsets use uint32_t",
                    u32::MAX
                ),
            ));
        }
    }

    if let Some(v) = pt.extra.get("shm_ring_capacity") {
        let n = v.as_u64().ok_or_else(|| {
            invalid(
                "shm_ring_capacity",
                format!("must be a positive integer (got {})", render_yaml_value(v)),
            )
        })?;
        if n == 0 || (n & (n - 1)) != 0 {
            return Err(invalid(
                "shm_ring_capacity",
                format!("must be a power of two (got {n})"),
            ));
        }
    }

    Ok(())
}

/// Validate someip pattern-specific per-event IDs.
///
/// `service_id` / `instance_id` are binding-level and guaranteed present
/// by `resolve_someip_ids`. The per-event resource ID is pattern-dependent;
/// [`super::pattern::CommunicationPattern::someip_field`] is the SSOT. This
/// validator checks that the expected [`SomeipFieldKind`] appears as a
/// matching [`SomeipEventIds`] variant in the SOME/IP variant's
/// `event_bindings` map (passed in by the caller after destructuring
/// the variant).
fn validate_someip_pattern_fields(
    machine_name: &str,
    rt: &ResolvedTarget,
    event_bindings: &BTreeMap<String, SomeipEventIds>,
) -> Result<(), TopologyError> {
    for ep in &rt.event_patterns {
        // Recover the detected pattern from the cached wire value; pattern
        // is what knows which field kind is required.
        let Some(pattern) = super::pattern::CommunicationPattern::from_wire(ep.pattern_kind_value)
        else {
            // Unknown wire value — upstream produced a pattern we don't
            // recognize. Skip rather than fail: the catalogue is the SSOT
            // and a missing arm is a pattern-layer bug, not an event bug.
            continue;
        };
        let Some(expected) = pattern.someip_field() else {
            // Pattern has no dedicated SOME/IP slot (FieldNotify rides the
            // getter/setter reply path) — nothing to validate on the
            // per-event binding map.
            continue;
        };
        let actual_kind = event_bindings.get(&ep.event).map(|ids| ids.field_kind());
        if actual_kind != Some(expected) {
            return Err(TopologyError::MissingBindingField {
                machine: machine_name.to_string(),
                target: rt.target.clone(),
                transport: rt.state.transport_name().to_string(),
                field: format!("{} (event \"{}\")", field_kind_yaml_name(expected), ep.event),
            });
        }
    }
    Ok(())
}

/// Map a SOME/IP field kind back to the deploy.yaml identifier the operator
/// would use to declare it. Kept close to validation so diagnostics match
/// the user-facing vocabulary exactly.
fn field_kind_yaml_name(kind: super::pattern::SomeipFieldKind) -> &'static str {
    use super::pattern::SomeipFieldKind;
    match kind {
        SomeipFieldKind::Method => "method_id",
        SomeipFieldKind::EventGroup => "event_group_id",
        SomeipFieldKind::Getter => "getter_id",
        SomeipFieldKind::Setter => "setter_id",
    }
}

/// Fan the per-binding external resolution into per-event SOME/IP IDs
/// and construct the public [`ResolvedTarget`]s.
///
/// Three sources are merged into `target.event_bindings`:
///   1. Per-event `events:` entries from deploy.yaml — copied verbatim.
///   2. Flat sugar (`method:` / `event_group:` / ...) — one entry per
///      SCXML event whose pattern matches the field set.
///
/// Per-event entries always win on conflict — they are the most specific
/// declaration. `EventBindingUnused` is reported when an `events:` entry
/// names an SCXML event that is never actually `<send>`-ed to this
/// target (likely a typo).
///
/// Consumes `partials` by value: `ResolvedTarget` has only one construction
/// path (this function), so the "empty `event_bindings` BTreeMap" transient
/// state is never observable — callers either see a half-built
/// `PartialTarget` (internal) or a fully populated `ResolvedTarget` (public).
pub(crate) fn finalize_targets(
    partials: Vec<PartialTarget>,
    machine_name: &str,
    external: &super::external::ExternalResolution,
) -> Result<Vec<ResolvedTarget>, TopologyError> {
    let mut resolved = Vec::with_capacity(partials.len());

    for pt in partials {
        let someip_resolved = if pt.transport == "someip" {
            Some(resolve_someip_ids(&pt, machine_name, external)?)
        } else {
            None
        };

        let pool_plan = derive_pool_plan(&pt);
        let state = build_transport_state(&pt, someip_resolved);

        resolved.push(ResolvedTarget {
            target: pt.target,
            events: pt.events,
            event_patterns: pt.event_patterns,
            state,
            invoke_sites: pt.invoke_sites,
            ordering: pt.ordering,
            pool_plan,
        });
    }

    Ok(resolved)
}

/// Compute the typed [`PoolPlan`] for a binding, or `None` if no pool
/// substitution is requested. `validate_pool_capability` (deploy.rs)
/// has already enforced the transport / field pairing (Zenoh uses
/// `{name}` in `extra`; SOME/IP uses `instance_from` + `instances`),
/// so the match here is exhaustive over the remaining valid shapes.
/// Invalid combinations are parse-time errors and do not reach this
/// function.
fn derive_pool_plan(pt: &PartialTarget) -> Option<PoolPlan> {
    match pt.transport.as_str() {
        "zenoh" => {
            // Zenoh placeholders live inside string `extra` values
            // (today: `key:`). Collect every `{name}` in insertion
            // order, de-duplicate, sort — the template's
            // substitution plan is position-independent.
            let mut names: std::collections::BTreeSet<String> =
                std::collections::BTreeSet::new();
            for value in pt.extra.values() {
                if let Some(s) = value.as_str() {
                    if let Ok(ids) =
                        crate::mesh::deploy::extract_placeholders(s)
                    {
                        names.extend(ids);
                    }
                }
            }
            if names.is_empty() {
                None
            } else {
                Some(PoolPlan::Zenoh {
                    placeholders: names.into_iter().collect(),
                })
            }
        }
        "someip" => {
            // SOME/IP bounded pool: `instance_from` + `instances`
            // pair gated by `validate_pool_capability`. Either both
            // present or both absent by that point.
            match (pt.instance_from.as_ref(), pt.instances.as_ref()) {
                (Some(instance_from), Some(list)) if !list.is_empty() => {
                    Some(PoolPlan::Someip {
                        instance_from: instance_from.clone(),
                        instances: list.clone(),
                    })
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// Construct the [`TransportState`] variant for a partial target.
///
/// SHM capacity tunables are extracted via `as_u64` after
/// [`validate_shm_extras_partial`] has already confirmed they fit in
/// `u32`; the cast is therefore lossless. SOME/IP identity and per-event
/// IDs are produced by the caller via [`resolve_someip_ids`] and passed
/// in as a single tuple — non-`None` iff `pt.transport == "someip"`.
/// Zenoh `key` is required by the registry
/// (`required_binding_fields = ["key"]`) so its presence is guaranteed
/// upstream; non-string values render as the empty string here, which
/// downstream codegen will surface as a build-time mismatch.
fn build_transport_state(
    pt: &PartialTarget,
    someip_resolved: Option<(SomeipServiceIds, BTreeMap<String, SomeipEventIds>)>,
) -> TransportState {
    match pt.transport.as_str() {
        "local" => TransportState::Local,
        "shm" => TransportState::Shm {
            arena_bytes: pt.extra.get("shm_arena_bytes").and_then(|v| v.as_u64()).map(|n| n as u32),
            ring_capacity: pt.extra.get("shm_ring_capacity").and_then(|v| v.as_u64()).map(|n| n as u32),
        },
        "someip" => {
            // `someip_resolved` is `Some` for every someip target after
            // `resolve_someip_ids` runs (typed `PerBindingResolution`
            // invariant). The `expect` pins that contract; reaching
            // `None` here would be an upstream bug.
            let (service, event_bindings) = someip_resolved.expect(
                "build_transport_state: someip target must have resolved service identity",
            );
            TransportState::Someip {
                service,
                event_bindings,
                // `pt.extra` already excludes typed BindingConfig fields
                // (`service`, `method`, `events`, …) and reserved ID keys
                // (rejected upstream). Whatever lands here is opaque
                // passthrough such as `protocol: udp/tcp`.
                extra: pt.extra.clone(),
            }
        }
        "zenoh" => {
            let key = pt
                .extra
                .get("key")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let extra = pt
                .extra
                .iter()
                .filter(|(k, _)| k.as_str() != "key")
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            TransportState::Zenoh { key, extra }
        }
        "custom_tcp" => {
            // `connect` presence is enforced by `required_binding_fields`
            // (`["connect"]`) before we reach this point — empty string here
            // would signal that the registry's required-fields list and this
            // arm have drifted apart.
            let connect = pt
                .extra
                .get("connect")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let extra = pt
                .extra
                .iter()
                .filter(|(k, _)| k.as_str() != "connect")
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            TransportState::CustomTcp { connect, extra }
        }
        other => TransportState::Unimplemented {
            transport_name: other.to_string(),
        },
    }
}

/// Per-target someip resolution: service identity + per-event IDs.
///
/// Invariants established by `external::resolve_external_bindings`:
///  * Every SOME/IP target that reached this stage has an entry in
///    `external.bindings` — targets without `service:` or `events:` are
///    rejected upstream as `NamedReferenceWithoutConfig`; targets whose
///    `service:` fails to resolve are filtered out and surface as
///    `UnresolvedNames`.
///  * `PerBindingResolution.service_ids` is typed (non-Option) — if the
///    binding reaches topology the service identity is always concrete.
///
/// The two defensive `Err(MissingBindingField)` branches this function
/// used to emit are therefore unreachable; `unreachable!` pins the
/// invariant in the type system rather than hiding it behind a
/// convincing-looking error path.
fn resolve_someip_ids(
    pt: &PartialTarget,
    machine_name: &str,
    external: &super::external::ExternalResolution,
) -> Result<(SomeipServiceIds, BTreeMap<String, SomeipEventIds>), TopologyError> {
    let key = (machine_name.to_string(), pt.target.clone());
    let per_binding = external.bindings.get(&key).unwrap_or_else(|| {
        unreachable!(
            "someip target '{}' for machine '{machine_name}' has no ExternalResolution entry; \
             external::resolve_external_bindings should have rejected it upstream",
            pt.target.as_str(),
        )
    });

    let someip_service = per_binding.service_ids;
    let default_ids = &per_binding.default;
    let mut event_bindings: BTreeMap<String, SomeipEventIds> = BTreeMap::new();

    // Fan out per-event entries.
    for ep in &pt.event_patterns {
        // Per-event explicit entry wins.
        if let Some(by_event) = per_binding.by_event.get(&ep.event) {
            event_bindings.insert(ep.event.clone(), by_event.clone());
            continue;
        }
        // Otherwise project the binding-level default through the event's
        // pattern. `project_to` consults the pattern's `someip_field` SSOT
        // and only yields Some when the fields the pattern wants are
        // populated on the default. Events whose pattern has no matching
        // default ID simply have no per-event entry —
        // `validate_someip_event_fields` will flag the gap.
        let Some(pattern) = super::pattern::CommunicationPattern::from_wire(ep.pattern_kind_value)
        else {
            continue;
        };
        if let Some(ids) = default_ids.project_to(pattern) {
            event_bindings.insert(ep.event.clone(), ids);
        }
    }

    // Auto-symmetry binding derivation: an EventUnsubscribe event generated
    // by inject_auto_subscriptions always shares the same event_group as its
    // paired EventSubscribe (event.subscribe.X ↔ event.unsubscribe.X). When
    // the unsubscribe event has no explicit per-event entry and no matching
    // default, clone the subscribe event's resolved binding. This fulfils the
    // auto-symmetry contract ("lifecycle symmetry comes from codegen") at the
    // deploy.yaml level — the author declares the subscribe binding once,
    // the unsubscribe binding is derived.
    for ep in &pt.event_patterns {
        if event_bindings.contains_key(&ep.event) {
            continue; // already resolved
        }
        let Some(pattern) = super::pattern::CommunicationPattern::from_wire(ep.pattern_kind_value)
        else {
            continue;
        };
        if pattern != super::pattern::CommunicationPattern::Unsubscribe {
            continue;
        }
        // Derive the paired subscribe event name:
        // "event.unsubscribe.X" → "event.subscribe.X"
        let sub_prefix = super::pattern::CommunicationPattern::Subscribe.prefix_str();
        let unsub_prefix = super::pattern::CommunicationPattern::Unsubscribe.prefix_str();
        let suffix = match ep.event.strip_prefix(unsub_prefix) {
            Some(s) => s,
            None => continue,
        };
        let subscribe_event = format!("{sub_prefix}{suffix}");
        if let Some(ids) = event_bindings.get(&subscribe_event) {
            event_bindings.insert(ep.event.clone(), ids.clone());
        }
    }

    // Detect unused per-event entries (likely typos).
    let actual_events: BTreeSet<&str> =
        pt.event_patterns.iter().map(|ep| ep.event.as_str()).collect();
    for declared in per_binding.by_event.keys() {
        if !actual_events.contains(declared.as_str()) {
            return Err(TopologyError::EventBindingUnused {
                machine: machine_name.to_string(),
                target: pt.target.clone(),
                event: declared.clone(),
            });
        }
    }

    Ok((someip_service, event_bindings))
}

/// Render a YAML value compactly for diagnostic messages.
/// Uses serde_yaml_ng's built-in serialization, falling back to Debug
/// if serialization fails (should not happen for valid input).
fn render_yaml_value(v: &serde_yaml_ng::Value) -> String {
    serde_yaml_ng::to_string(v)
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| format!("{v:?}"))
}

/// Find bindings for a machine across all devices in the topology.
///
/// Phase 1: returns the first match (single-device assumption).
/// Phase 2+: when the same machine can be deployed on multiple devices,
/// this must be extended to accept a device qualifier or return all matches.
fn find_machine_bindings<'a>(
    deploy: &'a DeployConfig,
    machine_name: &str,
) -> Result<&'a std::collections::HashMap<TargetId, BindingConfig>, TopologyError> {
    for device in deploy.topology.values() {
        if let Some(machine) = device.machines.get(machine_name) {
            return Ok(&machine.bindings);
        }
    }
    Err(TopologyError::MachineNotFound {
        machine: machine_name.to_string(),
        available: deploy
            .topology
            .values()
            .flat_map(|d| d.machines.keys())
            .cloned()
            .collect(),
    })
}

// ── Event coverage validation ───────────────────────────────

/// Build-time warning about an event sent to a target that has no matching
/// transition in the receiver.
#[derive(Debug, Clone)]
pub struct EventCoverageWarning {
    /// The sender machine name.
    pub sender: String,
    /// The receiver target (e.g. "#motor").
    pub target: TargetId,
    /// The event name that has no matching transition in the receiver.
    pub event: String,
}

impl std::fmt::Display for EventCoverageWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "machine '{}': <send target=\"{}\" event=\"{}\"/> has no matching \
             <transition event=\"...\"/> in the receiver. The event may be \
             silently dropped at runtime",
            self.sender, self.target, self.event
        )
    }
}

/// Collect all transition event names from an SCXML model.
///
/// Returns the set of event names that the model has `<transition event="...">`
/// handlers for. Wildcard (`*`) events are represented as the literal `"*"`.
fn collect_transition_events(model: &SCXMLModel) -> BTreeSet<String> {
    let mut events = BTreeSet::new();
    for state in model.states.values() {
        for transition in &state.transitions {
            if !transition.event.is_empty() {
                // W3C SCXML 3.12.1: space-separated event descriptors
                for descriptor in transition.event.split_whitespace() {
                    events.insert(descriptor.to_string());
                }
            }
        }
    }
    events
}

/// Check whether a sent event name matches any event descriptor in the
/// receiver's transition set.
///
/// W3C SCXML 3.12.1 matching rules:
///   - Exact match: "brake.activate" matches "brake.activate"
///   - Prefix match: "brake.activate" matches "brake" or "brake.*"
///   - Wildcard: receiver has "*" → matches everything
fn event_matches_any(sent_event: &str, receiver_events: &BTreeSet<String>) -> bool {
    if receiver_events.contains("*") {
        return true;
    }
    if receiver_events.contains(sent_event) {
        return true;
    }
    // W3C SCXML prefix matching: "brake.activate" matches descriptor "brake"
    // Zero-allocation: compare via byte-level prefix check + '.' separator
    for descriptor in receiver_events {
        let desc = descriptor.trim_end_matches(".*");
        if desc.is_empty() {
            continue;
        }
        if sent_event == desc {
            return true;
        }
        if sent_event.len() > desc.len()
            && sent_event.starts_with(desc)
            && sent_event.as_bytes()[desc.len()] == b'.'
        {
            return true;
        }
    }
    false
}

/// Validate event coverage across multiple SCXML models in a deployment.
///
/// For each `<send target="#X" event="Y">` in a sender model, checks whether
/// the receiver model `X` has at least one `<transition event="...">` that
/// matches `Y`. Returns warnings (not errors) because receivers may use
/// wildcard events or handle events in deeply nested states not visible
/// to this static analysis.
///
/// `models` maps machine name → parsed SCXML model. Only machines present
/// in `deploy` are cross-referenced.
pub fn validate_event_coverage(
    models: &[(&str, &SCXMLModel)],
    deploy: &DeployConfig,
) -> Vec<EventCoverageWarning> {
    let mut warnings = Vec::new();

    // Index: machine_name → transition events
    let mut receiver_events: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    for &(name, model) in models {
        receiver_events.insert(name, collect_transition_events(model));
    }

    for &(sender_name, sender_model) in models {
        let target_events = collect_target_events(sender_model);

        for (target, event) in &target_events {
            if event.is_empty() {
                continue; // No event name — content-only send
            }
            // SCE_MESH.md §13: subscribe/unsubscribe events are transport-
            // layer lifecycle actions, not receiver-engine events.
            if let Some(p) = super::pattern::detect_pattern(event) {
                if p == super::pattern::CommunicationPattern::Subscribe
                    || p == super::pattern::CommunicationPattern::Unsubscribe
                {
                    continue;
                }
            }

            // Resolve target to machine name: "#motor" → "motor"
            let receiver_name = target.name();

            // Only validate if receiver model is available
            if let Some(rx_events) = receiver_events.get(receiver_name) {
                if !event_matches_any(event, rx_events) {
                    warnings.push(EventCoverageWarning {
                        sender: sender_name.to_string(),
                        target: target.clone(),
                        event: event.clone(),
                    });
                }
            }
        }
    }

    // Second filter: suppress warnings for receivers whose model was provided
    // (via `models`) but that aren't actually deployed in this topology.
    // The first filter (receiver_events.get) already skips receivers with no
    // model — this catches the case where a model exists but isn't in deploy.yaml.
    warnings.retain(|w| {
        let receiver = w.target.name();
        deploy
            .topology
            .values()
            .any(|d| d.machines.contains_key(receiver))
    });

    warnings
}

// SCE_MESH.md §13 path B: QoS is a transport binding concern (declared in
// deploy.yaml or external config such as vsomeip.json), not a per-<send>
// SCXML annotation — so there is no `sce:qos` consistency validator here.


// ── Pattern capability validation ────────────────────────────

/// Validate that SCXML communication patterns are supported by bound transports
/// (SCE_MESH.md Section 8.2).
///
/// Uses pre-collected action details from `SendActionSummary` to avoid
/// redundant model traversal. SCE_MESH.md §13 path B: patterns are inferred
/// from event names only — never fails on author-provided input, so the
/// return type is a plain `Vec` of transport-mismatch violations.
pub fn validate_pattern_capability(
    summary: &SendActionSummary,
    deploy: &DeployConfig,
    machine_name: &str,
) -> Vec<super::pattern::PatternViolation> {
    use super::pattern::PatternViolation;
    use super::transport;

    let bindings = match find_machine_bindings(deploy, machine_name) {
        Ok(b) => b,
        Err(_) => return Vec::new(), // Machine not in deploy.yaml — no validation
    };

    let mut violations = Vec::new();

    for action in &summary.actions {
        if action.target.is_internal() || action.event.is_empty() {
            continue;
        }

        // SCE_MESH.md §13 path B: pattern is cached on SendActionDetail by
        // collect_send_summary (inferred from event name). Application-specific
        // events have no pattern — skip capability check.
        let Some(pattern) = action.pattern else {
            continue;
        };

        let binding = match bindings.get(&action.target) {
            Some(b) => b,
            None => continue, // Unresolved target — caught by resolve_targets()
        };

        let required = pattern.required_capability();
        if !transport::supports(&binding.transport, required) {
            violations.push(PatternViolation {
                state: action.state.clone(),
                target: action.target.clone(),
                event: action.event.clone(),
                pattern,
                required,
                transport: binding.transport.clone(),
            });
        }
    }

    violations
}

// ── Build-time event coverage enforcement ───────────────────

/// Load each receiver machine's SCXML model by reading the `source:` path
/// declared in deploy.yaml (resolved relative to `deploy_dir`).
///
/// Each unique receiver target (`#motor` → "motor") is loaded once. Errors
/// cover three distinct failure modes so diagnostics point at the right
/// line in deploy.yaml: receiver not declared, source unreadable, source
/// unparseable.
pub fn load_receiver_models(
    resolved: &[ResolvedTarget],
    deploy: &DeployConfig,
    deploy_dir: &Path,
    sender_name: &str,
) -> Result<Vec<(String, SCXMLModel)>, TopologyError> {
    // BTreeSet: dedup + deterministic iteration order across platforms.
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut out = Vec::new();

    for t in resolved {
        let receiver = t.target.name().to_string();
        if !seen.insert(receiver.clone()) {
            continue;
        }

        let machine_cfg = deploy
            .topology
            .values()
            .find_map(|d| d.machines.get(&receiver))
            .ok_or_else(|| TopologyError::ReceiverNotDeclared {
                sender: sender_name.to_string(),
                target: t.target.clone(),
                receiver: receiver.clone(),
            })?;

        // Portability: absolute source paths break deploy descriptors across
        // checkouts and build roots. Reject early with a distinct diagnostic.
        if Path::new(&machine_cfg.source).is_absolute() {
            return Err(TopologyError::AbsoluteSourcePath {
                machine: receiver.clone(),
                path: machine_cfg.source.clone(),
            });
        }

        let source_path = deploy_dir.join(&machine_cfg.source);
        let content = std::fs::read_to_string(&source_path).map_err(|e| {
            TopologyError::ReceiverSourceRead {
                machine: receiver.clone(),
                path: source_path.display().to_string(),
                source: e,
            }
        })?;

        let mut parser = crate::parser::SCXMLParser::new();
        let rx_model = parser.parse_string(&content, &receiver).map_err(|e| {
            // Flatten the typed parser error into the mesh-topology
            // diagnostic channel; adding a typed `source: ForgeError`
            // field here would require a mesh::error shape change and
            // is out of scope for the parser-typing refactor. The
            // machine + path context already tells operators which
            // receiver file to inspect.
            TopologyError::ReceiverSourceParse {
                machine: receiver.clone(),
                path: source_path.display().to_string(),
                reason: e.to_string(),
            }
        })?;

        out.push((receiver, rx_model));
    }

    Ok(out)
}

/// Strict event coverage check for a single sender: every `<send event="Y"/>`
/// to a static target must have a matching `<transition event="Y"/>` in the
/// receiver — with the §9.5 exemption that RpcReply events targeting a
/// receiver that declares `<invoke type="sce:mesh-rpc">` are consumed by the
/// correlation table before reaching SCXML, so the reply event name will
/// never appear on a literal transition and must not be flagged.
///
/// Sender-scoped by construction: only the sender's target_events are
/// iterated, so the cost is proportional to the sender's `<send>` count
/// regardless of how many other machines are in the deployment.
pub fn check_sender_event_coverage(
    sender_name: &str,
    summary: &SendActionSummary,
    receiver_models: &[(String, SCXMLModel)],
    deploy: &DeployConfig,
) -> Vec<EventCoverageWarning> {
    // Index receivers' transition event sets once.
    let receiver_events: BTreeMap<&str, BTreeSet<String>> = receiver_models
        .iter()
        .map(|(name, m)| (name.as_str(), collect_transition_events(m)))
        .collect();

    // Collect each receiver's §9.5 reply-event exemption set: for every
    // `<invoke type="sce:mesh-rpc">` the receiver hosts, the paired reply
    // event (derived via `CommunicationPattern::infer_reply_event`) is
    // delivered through InvokeCorrelation, not a literal transition.
    let receiver_rpc_replies: BTreeMap<&str, BTreeSet<String>> = receiver_models
        .iter()
        .map(|(name, m)| {
            let replies = m
                .states
                .values()
                .flat_map(|s| s.invokes.iter())
                .filter_map(|i| match i {
                    Invoke::MeshRpc(info) => Some(info),
                    _ => None,
                })
                .filter_map(|info| {
                    super::pattern::detect_pattern(&info.mesh_event)
                        .and_then(|p| p.infer_reply_event(&info.mesh_event))
                })
                .collect::<BTreeSet<String>>();
            (name.as_str(), replies)
        })
        .collect();

    let mut findings = Vec::new();
    for (target, event) in &summary.target_events {
        if event.is_empty() {
            continue; // content-only <send>; no event name to match
        }
        // SCE_MESH.md §13: subscribe and unsubscribe events are transport-
        // layer lifecycle actions consumed by the sender's own router, not
        // by the receiver engine. Skip coverage check for both patterns.
        if let Some(p) = super::pattern::detect_pattern(event) {
            if p == super::pattern::CommunicationPattern::Subscribe
                || p == super::pattern::CommunicationPattern::Unsubscribe
            {
                continue;
            }
        }
        let receiver_name = target.name();
        let rx_events = match receiver_events.get(receiver_name) {
            Some(s) => s,
            None => continue, // receiver model not supplied — out of scope
        };
        if let Some(rpc_replies) = receiver_rpc_replies.get(receiver_name) {
            if rpc_replies.contains(event) {
                continue; // §9.5 reply consumed by correlation, not SCXML
            }
        }
        if !event_matches_any(event, rx_events) {
            findings.push(EventCoverageWarning {
                sender: sender_name.to_string(),
                target: target.clone(),
                event: event.clone(),
            });
        }
    }

    // Mirror validate_event_coverage: only report for receivers that are
    // actually deployed. A supplied-but-undeployed receiver would indicate
    // an inconsistent caller, not a real runtime violation.
    findings.retain(|w| {
        let receiver = w.target.name();
        deploy
            .topology
            .values()
            .any(|d| d.machines.contains_key(receiver))
    });

    findings
}

// ── Subscription auto-symmetry (SCE_MESH.md §13) ────────────

/// A single auto-symmetry site: an `<onentry>` `<send event="event.subscribe.X">`
/// that qualifies for automatic `<onexit>` unsubscribe generation.
#[derive(Debug, Clone)]
pub struct AutoSubscription {
    /// State containing the qualifying `<onentry>` subscribe.
    pub state_id: String,
    /// Target of the subscribe send (e.g. `#bus`).
    pub target: TargetId,
    /// The original subscribe event name (`event.subscribe.X`).
    pub subscribe_event: String,
    /// The synthesized unsubscribe event name (`event.unsubscribe.X`).
    pub unsubscribe_event: String,
}

/// Lint notice emitted when a subscribe send does not qualify for
/// auto-symmetry. Not a build error — the document still compiles —
/// but subscription lifecycle becomes the author's responsibility.
#[derive(Debug, Clone)]
pub struct SubscriptionLintNotice {
    /// State containing the ineligible subscribe.
    pub state_id: String,
    /// The subscribe event name.
    pub event: String,
    /// Why auto-symmetry was suppressed.
    pub reason: SubscriptionLintReason,
}

/// Reason auto-symmetry was suppressed for a subscribe send.
#[derive(Debug, Clone)]
pub enum SubscriptionLintReason {
    /// The `<send>` is nested inside `<if>`, `<foreach>`, or other
    /// conditional/iterative executable content (not a direct child of
    /// `<onentry>`).
    NestedInConditional,
    /// The state's `<onexit>` already contains a manual
    /// `<send event="event.unsubscribe.X">` for the same topic.
    ManualUnsubscribePresent,
    /// Duplicate subscribe for the same event in the same `<onentry>` block.
    DuplicateSubscribe,
}

impl std::fmt::Display for SubscriptionLintNotice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let reason = match &self.reason {
            SubscriptionLintReason::NestedInConditional =>
                "subscribe is nested inside <if>/<foreach> (not a direct child of <onentry>)",
            SubscriptionLintReason::ManualUnsubscribePresent =>
                "state already has a manual <onexit> unsubscribe for the same event",
            SubscriptionLintReason::DuplicateSubscribe =>
                "duplicate subscribe for the same event in the same <onentry> block",
        };
        write!(
            f,
            "state '{}': <send event=\"{}\"> does not qualify for auto-symmetry \
             unsubscribe generation ({}). Write an explicit <onexit> unsubscribe \
             if subscription lifecycle management is intended.",
            self.state_id, self.event, reason
        )
    }
}

/// Detect subscribe sends eligible for auto-symmetry and inject
/// synthetic unsubscribe sends into the model's `<onexit>` blocks.
///
/// SCE_MESH.md §13 auto-symmetry rules:
///   1. Direct child of `<onentry>` (not nested in `<if>`, `<foreach>`)
///   2. No manual `<send event="event.unsubscribe.X">` in `<onexit>`
///   3. Not a duplicate subscribe in the same `<onentry>` block
///
/// Returns qualifying sites + lint notices for suppressed ones. The
/// model is mutated in place: synthetic `Action { action_type: "send",
/// event: "event.unsubscribe.X", target: ... }` entries are appended
/// to the state's first `on_exit_blocks` entry (created if absent).
///
/// Must be called BEFORE `collect_send_summary` so the synthetic
/// sends are visible to pattern detection and event coverage.
pub fn inject_auto_subscriptions(
    model: &mut crate::model::SCXMLModel,
) -> (Vec<AutoSubscription>, Vec<SubscriptionLintNotice>) {
    let mut sites = Vec::new();
    let mut notices = Vec::new();

    // Collect subscription candidates by scanning all states.
    // Two-pass: first collect (immutable read), then inject (mutable write).
    struct Candidate {
        state_id: String,
        target: TargetId,
        subscribe_event: String,
        unsubscribe_event: String,
    }
    let mut candidates: Vec<Candidate> = Vec::new();

    for (state_id, state) in &model.states {
        // Collect existing manual unsubscribe event names in onexit.
        // Used to suppress auto-symmetry (rule 3: manual takes precedence).
        let manual_unsubscribes: std::collections::HashSet<String> = state
            .on_exit_blocks
            .iter()
            .flat_map(|block| block.iter())
            .filter(|a| a.action_type == "send")
            .filter(|a| {
                super::pattern::detect_pattern(&a.event)
                    == Some(super::pattern::CommunicationPattern::Unsubscribe)
            })
            .map(|a| a.event.clone())
            .collect();

        // Also scan nested actions for subscribe patterns that don't
        // qualify (for lint notices).
        let mut seen_subscribes_in_block: std::collections::HashSet<String> =
            std::collections::HashSet::new();

        for block in &state.on_entry_blocks {
            seen_subscribes_in_block.clear();

            // Scan nested actions for non-qualifying subscribe sends.
            for action in block {
                if action.action_type == "if" || action.action_type == "foreach" {
                    collect_nested_subscribe_notices(
                        state_id,
                        action,
                        &mut notices,
                    );
                }
            }

            // Direct children: check eligibility.
            for action in block {
                if action.action_type != "send" {
                    continue;
                }
                if super::pattern::detect_pattern(&action.event)
                    != Some(super::pattern::CommunicationPattern::Subscribe)
                {
                    continue;
                }

                let Some(unsub_event) =
                    super::pattern::subscribe_to_unsubscribe(&action.event)
                else {
                    continue;
                };

                // Rule 3: duplicate subscribe in same block
                if !seen_subscribes_in_block.insert(action.event.clone()) {
                    notices.push(SubscriptionLintNotice {
                        state_id: state_id.clone(),
                        event: action.event.clone(),
                        reason: SubscriptionLintReason::DuplicateSubscribe,
                    });
                    continue;
                }

                // Rule 2: manual unsubscribe already present
                if manual_unsubscribes.contains(&unsub_event) {
                    notices.push(SubscriptionLintNotice {
                        state_id: state_id.clone(),
                        event: action.event.clone(),
                        reason: SubscriptionLintReason::ManualUnsubscribePresent,
                    });
                    continue;
                }

                // Eligible — collect candidate.
                let Some(tid) = TargetId::new(&action.target) else {
                    continue;
                };
                candidates.push(Candidate {
                    state_id: state_id.clone(),
                    target: tid,
                    subscribe_event: action.event.clone(),
                    unsubscribe_event: unsub_event,
                });
            }
        }
    }

    // Inject synthetic unsubscribe sends into the model.
    for c in &candidates {
        let state = model.states.get_mut(&c.state_id).expect(
            "state must exist — collected from the same model",
        );

        // Build synthetic send action.
        let mut unsub_action = crate::model::Action::default();
        unsub_action.action_type = "send".to_string();
        unsub_action.event = c.unsubscribe_event.clone();
        unsub_action.target = c.target.as_str().to_string();

        // Append to the first onexit block; create one if absent.
        if state.on_exit_blocks.is_empty() {
            state.on_exit_blocks.push(Vec::new());
        }
        state.on_exit_blocks[0].push(unsub_action);

        sites.push(AutoSubscription {
            state_id: c.state_id.clone(),
            target: c.target.clone(),
            subscribe_event: c.subscribe_event.clone(),
            unsubscribe_event: c.unsubscribe_event.clone(),
        });
    }

    (sites, notices)
}

/// Recursively scan nested actions (inside `<if>`, `<foreach>`) for
/// subscribe sends and emit lint notices for each.
fn collect_nested_subscribe_notices(
    state_id: &str,
    action: &crate::model::Action,
    notices: &mut Vec<SubscriptionLintNotice>,
) {
    // Flat list of all action sequences inside this conditional/iterative.
    let sequences: Vec<&[crate::model::Action]> = {
        let mut seqs: Vec<&[crate::model::Action]> = Vec::new();
        seqs.push(&action.then_actions);
        for branch in &action.elseif_branches {
            seqs.push(&branch.actions);
        }
        seqs.push(&action.else_actions);
        seqs.push(&action.actions); // <foreach> body
        seqs
    };

    for seq in sequences {
        for a in seq {
            if a.action_type == "send"
                && super::pattern::detect_pattern(&a.event)
                    == Some(super::pattern::CommunicationPattern::Subscribe)
            {
                notices.push(SubscriptionLintNotice {
                    state_id: state_id.to_string(),
                    event: a.event.clone(),
                    reason: SubscriptionLintReason::NestedInConditional,
                });
            }
            if a.action_type == "if" || a.action_type == "foreach" {
                collect_nested_subscribe_notices(state_id, a, notices);
            }
        }
    }
}

// ── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::deploy::parse_deploy_str;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    /// Scratch directory with RAII cleanup. The dir is created on construction
    /// and removed on drop so /tmp doesn't accumulate `sce_mesh_topology_*`
    /// entries across test runs. `Deref<Target = Path>` lets callers use it
    /// transparently with `fs::write(dir.join(...), ...)` etc.
    struct TempDir(std::path::PathBuf);

    impl TempDir {
        fn new(label: &str) -> Self {
            let n = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "sce_mesh_topology_{}_{}_{}",
                label,
                std::process::id(),
                n
            ));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    impl std::ops::Deref for TempDir {
        type Target = std::path::Path;
        fn deref(&self) -> &std::path::Path {
            &self.0
        }
    }

    const BRAKE_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="null" name="brake" initial="idle">
    <state id="idle">
        <transition event="brake.press" target="braking"/>
    </state>
    <state id="braking">
        <onentry>
            <send target="#motor" event="brake.activate"/>
        </onentry>
        <transition event="brake.release" target="idle"/>
    </state>
</scxml>"##;

    const BRAKE_BAD_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="null" name="brake" initial="idle">
    <state id="idle">
        <transition event="brake.press" target="braking"/>
    </state>
    <state id="braking">
        <onentry>
            <send target="#motor" event="brake.typo"/>
        </onentry>
    </state>
</scxml>"##;

    const MOTOR_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="null" name="motor" initial="running">
    <state id="running">
        <transition event="brake.activate" target="stopped"/>
    </state>
    <state id="stopped"/>
</scxml>"##;

    fn parse_model(content: &str, name: &str) -> SCXMLModel {
        let mut p = crate::parser::SCXMLParser::new();
        p.parse_string(content, name).expect("parse")
    }

    fn summary_for(model: &SCXMLModel) -> SendActionSummary {
        collect_send_summary(model)
    }

    fn resolved_motor_target() -> Vec<ResolvedTarget> {
        // Non-someip target — `TransportState::Local` carries no
        // SOME/IP fields, so impossible-state probing is unrepresentable.
        vec![ResolvedTarget {
            target: TargetId::new("#motor").unwrap(),
            events: vec!["brake.activate".to_string()],
            event_patterns: Vec::new(),
            state: TransportState::Local,
            invoke_sites: Vec::new(),
            ordering: crate::mesh::deploy::OrderingRequirement::None,
            pool_plan: None,
        }]
    }

    // ── load_receiver_models: happy path ─────────────────────

    #[test]
    fn load_receiver_models_reads_source() {
        let dir = TempDir::new("happy");
        fs::write(dir.join("motor.scxml"), MOTOR_SCXML).unwrap();
        let deploy = parse_deploy_str(
            r##"version: "1.0"
topology:
  ecu1:
    machines:
      brake: { source: brake.scxml, bindings: { "#motor": { transport: local } } }
      motor: { source: motor.scxml }
"##,
        )
        .unwrap();

        let resolved = resolved_motor_target();
        let loaded = load_receiver_models(&resolved, &deploy, &dir, "brake").expect("load");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].0, "motor");
        assert!(!loaded[0].1.states.is_empty());
    }

    // ── ReceiverNotDeclared ──────────────────────────────────

    #[test]
    fn receiver_not_declared_errors() {
        let dir = TempDir::new("notdecl");
        let deploy = parse_deploy_str(
            r##"version: "1.0"
topology:
  ecu1:
    machines:
      brake: { source: brake.scxml, bindings: { "#motor": { transport: local } } }
"##,
        )
        .unwrap();

        let resolved = resolved_motor_target();
        let err = load_receiver_models(&resolved, &deploy, &dir, "brake").unwrap_err();
        match err {
            TopologyError::ReceiverNotDeclared { sender, target, receiver } => {
                assert_eq!(sender, "brake");
                assert_eq!(target, "#motor");
                assert_eq!(receiver, "motor");
            }
            other => panic!("expected ReceiverNotDeclared, got {other:?}"),
        }
    }

    // ── ReceiverSourceRead (missing file) ────────────────────

    #[test]
    fn receiver_source_missing_file_errors() {
        let dir = TempDir::new("missingfile");
        // Note: motor.scxml is NOT written to scratch dir.
        let deploy = parse_deploy_str(
            r##"version: "1.0"
topology:
  ecu1:
    machines:
      brake: { source: brake.scxml, bindings: { "#motor": { transport: local } } }
      motor: { source: motor.scxml }
"##,
        )
        .unwrap();

        let resolved = resolved_motor_target();
        let err = load_receiver_models(&resolved, &deploy, &dir, "brake").unwrap_err();
        match err {
            TopologyError::ReceiverSourceRead { machine, path, .. } => {
                assert_eq!(machine, "motor");
                assert!(path.ends_with("motor.scxml"), "path was {path}");
            }
            other => panic!("expected ReceiverSourceRead, got {other:?}"),
        }
    }

    // ── ReceiverSourceParse (bad XML) ────────────────────────

    #[test]
    fn receiver_source_bad_xml_errors() {
        let dir = TempDir::new("badxml");
        fs::write(dir.join("motor.scxml"), "<<<not-xml>>>").unwrap();
        let deploy = parse_deploy_str(
            r##"version: "1.0"
topology:
  ecu1:
    machines:
      brake: { source: brake.scxml, bindings: { "#motor": { transport: local } } }
      motor: { source: motor.scxml }
"##,
        )
        .unwrap();

        let resolved = resolved_motor_target();
        let err = load_receiver_models(&resolved, &deploy, &dir, "brake").unwrap_err();
        match err {
            TopologyError::ReceiverSourceParse { machine, .. } => {
                assert_eq!(machine, "motor");
            }
            other => panic!("expected ReceiverSourceParse, got {other:?}"),
        }
    }

    // ── AbsoluteSourcePath ───────────────────────────────────

    #[test]
    fn absolute_source_path_rejected() {
        let dir = TempDir::new("absolute");
        let deploy = parse_deploy_str(
            r##"version: "1.0"
topology:
  ecu1:
    machines:
      brake: { source: brake.scxml, bindings: { "#motor": { transport: local } } }
      motor: { source: /etc/motor.scxml }
"##,
        )
        .unwrap();

        let resolved = resolved_motor_target();
        let err = load_receiver_models(&resolved, &deploy, &dir, "brake").unwrap_err();
        match err {
            TopologyError::AbsoluteSourcePath { machine, path } => {
                assert_eq!(machine, "motor");
                assert_eq!(path, "/etc/motor.scxml");
            }
            other => panic!("expected AbsoluteSourcePath, got {other:?}"),
        }
    }

    // ── check_sender_event_coverage: positive + negative ─────

    #[test]
    fn check_sender_event_coverage_ok_when_matched() {
        let deploy = parse_deploy_str(
            r##"version: "1.0"
topology:
  ecu1:
    machines:
      brake: { source: brake.scxml, bindings: { "#motor": { transport: local } } }
      motor: { source: motor.scxml }
"##,
        )
        .unwrap();
        let brake = parse_model(BRAKE_SCXML, "brake");
        let motor = parse_model(MOTOR_SCXML, "motor");
        let receivers = vec![("motor".to_string(), motor)];

        let summary = summary_for(&brake);
        let findings = check_sender_event_coverage("brake", &summary, &receivers, &deploy);
        assert!(findings.is_empty(), "expected no findings, got {findings:?}");
    }

    #[test]
    fn check_sender_event_coverage_exempts_mesh_rpc_reply() {
        // §9.5 scenario: brake hosts <invoke type="sce:mesh-rpc"> with
        // mesh_event="service.request.compute_force"; motor replies with
        // "service.response.compute_force" targeting #brake. Brake has NO
        // literal transition for the reply event (it rides the correlation
        // table), so the coverage check must exempt it.
        const BRAKE_INVOKE_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="null" name="brake" initial="idle">
    <state id="idle">
        <transition event="go" target="computing"/>
    </state>
    <state id="computing">
        <invoke type="sce:mesh-rpc" src="#motor">
            <param name="_mesh_event" expr="'service.request.compute_force'"/>
        </invoke>
        <transition event="done.invoke.*" target="ok"/>
    </state>
    <final id="ok"/>
</scxml>"##;
        const MOTOR_REPLIES_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="null" name="motor" initial="ready">
    <state id="ready">
        <transition event="service.request.compute_force" target="replying"/>
    </state>
    <state id="replying">
        <onentry>
            <send target="#brake" event="service.response.compute_force"/>
        </onentry>
    </state>
</scxml>"##;
        let deploy = parse_deploy_str(
            r##"version: "1.0"
topology:
  ecu1:
    machines:
      brake: { source: brake.scxml, bindings: { "#motor": { transport: local } } }
      motor: { source: motor.scxml, bindings: { "#brake": { transport: local } } }
"##,
        )
        .unwrap();
        let motor = parse_model(MOTOR_REPLIES_SCXML, "motor");
        let brake = parse_model(BRAKE_INVOKE_SCXML, "brake");
        let receivers = vec![("brake".to_string(), brake)];

        let summary = summary_for(&motor);
        let findings = check_sender_event_coverage("motor", &summary, &receivers, &deploy);
        assert!(
            findings.is_empty(),
            "expected RpcReply exemption, got {findings:?}"
        );
    }

    #[test]
    fn check_sender_event_coverage_reports_mismatch() {
        let deploy = parse_deploy_str(
            r##"version: "1.0"
topology:
  ecu1:
    machines:
      brake: { source: brake.scxml, bindings: { "#motor": { transport: local } } }
      motor: { source: motor.scxml }
"##,
        )
        .unwrap();
        let brake = parse_model(BRAKE_BAD_SCXML, "brake");
        let motor = parse_model(MOTOR_SCXML, "motor");
        let receivers = vec![("motor".to_string(), motor)];

        let summary = summary_for(&brake);
        let findings = check_sender_event_coverage("brake", &summary, &receivers, &deploy);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].event, "brake.typo");
        assert_eq!(findings[0].target, "#motor");
        assert_eq!(findings[0].sender, "brake");
    }

    // ── validate_pattern_capability ─────────────────────────

    /// SCXML with communication pattern events for pattern validation tests.
    const PATTERN_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="null" name="tester" initial="idle">
    <state id="idle">
        <onentry>
            <send target="#service" event="service.request.get_status"/>
        </onentry>
        <transition event="done" target="idle"/>
    </state>
</scxml>"##;

    /// SCXML with fire-and-forget pattern (universally supported).
    const FIRE_FORGET_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="null" name="sender" initial="s">
    <state id="s">
        <onentry>
            <send target="#receiver" event="service.fire_forget.motor_cmd"/>
        </onentry>
    </state>
</scxml>"##;

    /// SCXML with subscribe pattern.
    const SUBSCRIBE_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="null" name="subscriber" initial="s">
    <state id="s">
        <onentry>
            <send target="#broker" event="event.subscribe.speed_updates"/>
        </onentry>
    </state>
</scxml>"##;

    /// SCXML with application-specific events (no pattern — no validation).
    const APP_EVENT_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="null" name="app" initial="s">
    <state id="s">
        <onentry>
            <send target="#motor" event="brake.activate"/>
        </onentry>
    </state>
</scxml>"##;

    #[test]
    fn pattern_validation_passes_on_capable_transport() {
        let deploy = parse_deploy_str(
            r##"version: "1.0"
topology:
  ecu1:
    machines:
      tester: { source: tester.scxml, bindings: { "#service": { transport: someip } } }
      service: { source: service.scxml }
"##,
        )
        .unwrap();
        let model = parse_model(PATTERN_SCXML, "tester");

        let violations = validate_pattern_capability(&summary_for(&model), &deploy, "tester");
        assert!(violations.is_empty(), "expected no violations, got {violations:?}");
    }

    #[test]
    fn pattern_validation_detects_request_on_shm() {
        // shm does not support request/reply — service.request should fail.
        let deploy = parse_deploy_str(
            r##"version: "1.0"
topology:
  ecu1:
    machines:
      tester: { source: tester.scxml, bindings: { "#service": { transport: shm } } }
      service: { source: service.scxml }
"##,
        )
        .unwrap();
        let model = parse_model(PATTERN_SCXML, "tester");

        let violations = validate_pattern_capability(&summary_for(&model), &deploy, "tester");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].event, "service.request.get_status");
        assert_eq!(violations[0].transport, "shm");
        assert_eq!(
            violations[0].pattern,
            super::super::pattern::CommunicationPattern::ServiceRequest
        );
    }

    #[test]
    fn pattern_validation_detects_subscribe_on_can() {
        // CAN does not support pub/sub — event.subscribe should fail.
        let deploy = parse_deploy_str(
            r##"version: "1.0"
topology:
  ecu1:
    machines:
      subscriber: { source: sub.scxml, bindings: { "#broker": { transport: can } } }
      broker: { source: broker.scxml }
"##,
        )
        .unwrap();
        let model = parse_model(SUBSCRIBE_SCXML, "subscriber");

        let violations = validate_pattern_capability(&summary_for(&model), &deploy, "subscriber");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].event, "event.subscribe.speed_updates");
        assert_eq!(violations[0].transport, "can");
    }

    #[test]
    fn pattern_validation_fire_forget_passes_everywhere() {
        // fire_forget is supported by all known transports.
        for transport in &["local", "shm", "someip", "dds", "zenoh", "can"] {
            let yaml = format!(
                r##"version: "1.0"
topology:
  ecu1:
    machines:
      sender: {{ source: s.scxml, bindings: {{ "#receiver": {{ transport: {transport} }} }} }}
      receiver: {{ source: r.scxml }}
"##
            );
            let deploy = parse_deploy_str(&yaml).unwrap();
            let model = parse_model(FIRE_FORGET_SCXML, "sender");

            let violations = validate_pattern_capability(&summary_for(&model), &deploy, "sender");
            assert!(
                violations.is_empty(),
                "fire_forget should pass on {transport}, got {violations:?}"
            );
        }
    }

    #[test]
    fn pattern_validation_skips_application_events() {
        // Application-specific events (brake.activate) have no pattern — no validation.
        let deploy = parse_deploy_str(
            r##"version: "1.0"
topology:
  ecu1:
    machines:
      app: { source: app.scxml, bindings: { "#motor": { transport: can } } }
      motor: { source: motor.scxml }
"##,
        )
        .unwrap();
        let model = parse_model(APP_EVENT_SCXML, "app");

        let violations = validate_pattern_capability(&summary_for(&model), &deploy, "app");
        assert!(violations.is_empty(), "app events should not be validated");
    }

    #[test]
    fn pattern_validation_skips_unknown_transport() {
        // Unknown transports get conservative pass (no validation).
        let deploy = parse_deploy_str(
            r##"version: "1.0"
topology:
  ecu1:
    machines:
      tester: { source: t.scxml, bindings: { "#service": { transport: custom_ipc } } }
      service: { source: s.scxml }
"##,
        )
        .unwrap();
        let model = parse_model(PATTERN_SCXML, "tester");

        let violations = validate_pattern_capability(&summary_for(&model), &deploy, "tester");
        assert!(violations.is_empty(), "unknown transport should pass");
    }

    #[test]
    fn pattern_validation_skips_machine_not_in_deploy() {
        let deploy = parse_deploy_str(
            r##"version: "1.0"
topology:
  ecu1:
    machines:
      other: { source: other.scxml }
"##,
        )
        .unwrap();
        let model = parse_model(PATTERN_SCXML, "tester");

        // Machine "tester" not in deploy.yaml — no validation.
        let violations = validate_pattern_capability(&summary_for(&model), &deploy, "tester");
        assert!(violations.is_empty());
    }

    // ── shm_* binding field validation ──────────────────────────

    const SHM_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" name="sender" initial="s">
  <state id="s">
    <onentry><send target="#receiver" event="e"/></onentry>
  </state>
</scxml>"##;

    fn resolve_for_shm(yaml: &str) -> Result<Vec<ResolvedTarget>, TopologyError> {
        let deploy = parse_deploy_str(yaml).unwrap();
        let model = parse_model(SHM_SCXML, "sender");
        let external = super::super::external::ExternalResolution::default();
        build_resolved_targets(&summary_for(&model), &deploy, "sender", &external)
            .map(|r| r.targets)
    }

    #[test]
    fn shm_no_extras_accepts_defaults() {
        let yaml = r##"version: "1.0"
topology:
  ecu1:
    machines:
      sender: { source: s.scxml, bindings: { "#receiver": { transport: shm } } }
      receiver: { source: r.scxml }
"##;
        resolve_for_shm(yaml).expect("defaults should be accepted");
    }

    #[test]
    fn shm_valid_arena_and_ring_accepted() {
        let yaml = r##"version: "1.0"
topology:
  ecu1:
    machines:
      sender:
        source: s.scxml
        bindings:
          "#receiver":
            transport: shm
            shm_arena_bytes: 131072
            shm_ring_capacity: 512
      receiver: { source: r.scxml }
"##;
        resolve_for_shm(yaml).expect("valid shm extras should be accepted");
    }

    #[test]
    fn shm_zero_arena_rejected() {
        let yaml = r##"version: "1.0"
topology:
  ecu1:
    machines:
      sender:
        source: s.scxml
        bindings:
          "#receiver":
            transport: shm
            shm_arena_bytes: 0
      receiver: { source: r.scxml }
"##;
        let err = resolve_for_shm(yaml).unwrap_err();
        match err {
            TopologyError::InvalidBindingField { field, reason, .. } => {
                assert_eq!(field, "shm_arena_bytes");
                assert!(reason.contains("greater than zero"), "reason: {reason}");
            }
            other => panic!("expected InvalidBindingField, got {other:?}"),
        }
    }

    #[test]
    fn shm_ring_capacity_must_be_power_of_two() {
        let yaml = r##"version: "1.0"
topology:
  ecu1:
    machines:
      sender:
        source: s.scxml
        bindings:
          "#receiver":
            transport: shm
            shm_ring_capacity: 300
      receiver: { source: r.scxml }
"##;
        let err = resolve_for_shm(yaml).unwrap_err();
        match err {
            TopologyError::InvalidBindingField { field, reason, .. } => {
                assert_eq!(field, "shm_ring_capacity");
                assert!(reason.contains("power of two"), "reason: {reason}");
            }
            other => panic!("expected InvalidBindingField, got {other:?}"),
        }
    }

    #[test]
    fn shm_ring_capacity_zero_rejected() {
        let yaml = r##"version: "1.0"
topology:
  ecu1:
    machines:
      sender:
        source: s.scxml
        bindings:
          "#receiver":
            transport: shm
            shm_ring_capacity: 0
      receiver: { source: r.scxml }
"##;
        let err = resolve_for_shm(yaml).unwrap_err();
        match err {
            TopologyError::InvalidBindingField { field, .. } => {
                assert_eq!(field, "shm_ring_capacity");
            }
            other => panic!("expected InvalidBindingField, got {other:?}"),
        }
    }

    #[test]
    fn shm_non_integer_arena_rejected() {
        let yaml = r##"version: "1.0"
topology:
  ecu1:
    machines:
      sender:
        source: s.scxml
        bindings:
          "#receiver":
            transport: shm
            shm_arena_bytes: "not-a-number"
      receiver: { source: r.scxml }
"##;
        let err = resolve_for_shm(yaml).unwrap_err();
        match err {
            TopologyError::InvalidBindingField { field, reason, .. } => {
                assert_eq!(field, "shm_arena_bytes");
                // Message should include the offending value for diagnostics.
                assert!(reason.contains("positive integer"), "reason: {reason}");
                assert!(reason.contains("not-a-number"), "reason: {reason}");
            }
            other => panic!("expected InvalidBindingField, got {other:?}"),
        }
    }

    #[test]
    fn shm_arena_exceeds_u32_rejected() {
        // 2^32 = 4294967296 > u32::MAX (4294967295) — out of range.
        let yaml = r##"version: "1.0"
topology:
  ecu1:
    machines:
      sender:
        source: s.scxml
        bindings:
          "#receiver":
            transport: shm
            shm_arena_bytes: 4294967296
      receiver: { source: r.scxml }
"##;
        let err = resolve_for_shm(yaml).unwrap_err();
        match err {
            TopologyError::InvalidBindingField { field, reason, .. } => {
                assert_eq!(field, "shm_arena_bytes");
                assert!(reason.contains("exceeds u32"), "reason: {reason}");
            }
            other => panic!("expected InvalidBindingField, got {other:?}"),
        }
    }

    #[test]
    fn shm_negative_arena_rejected() {
        // Negative values cannot coerce to u64; error message should
        // include the offending value for diagnostics.
        let yaml = r##"version: "1.0"
topology:
  ecu1:
    machines:
      sender:
        source: s.scxml
        bindings:
          "#receiver":
            transport: shm
            shm_arena_bytes: -1
      receiver: { source: r.scxml }
"##;
        let err = resolve_for_shm(yaml).unwrap_err();
        match err {
            TopologyError::InvalidBindingField { field, reason, .. } => {
                assert_eq!(field, "shm_arena_bytes");
                assert!(reason.contains("-1"), "reason should include value: {reason}");
            }
            other => panic!("expected InvalidBindingField, got {other:?}"),
        }
    }

    // ── SCE_MESH.md §9.5: per-invoke vs binding-level deadline ─────

    /// Build a sender model with one `<invoke type="sce:mesh-rpc">`
    /// targeting `#motor`, optionally embedding a per-invoke deadline.
    fn mesh_rpc_sender_scxml(per_invoke_deadline: Option<u64>) -> String {
        let dl_param = match per_invoke_deadline {
            Some(ms) => format!("\n        <param name=\"_mesh_deadline_ms\" expr=\"{ms}\"/>"),
            None => String::new(),
        };
        format!(
            r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       datamodel="null" name="brake" initial="idle">
  <state id="idle">
    <invoke type="sce:mesh-rpc" src="#motor">
      <param name="_mesh_event" expr="'service.request.compute_force'"/>{dl_param}
    </invoke>
  </state>
</scxml>"##
        )
    }

    fn deploy_with_motor_binding(deadline_yaml_line: &str) -> String {
        format!(
            r##"version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        bindings:
          "#motor":
            transport: local{deadline_yaml_line}
      motor: {{ source: motor.scxml }}
"##
        )
    }

    fn resolve_for_mesh_rpc(
        per_invoke: Option<u64>,
        binding_yaml: &str,
    ) -> TargetResolution {
        let model = parse_model(&mesh_rpc_sender_scxml(per_invoke), "brake");
        let deploy = parse_deploy_str(&deploy_with_motor_binding(binding_yaml)).unwrap();
        let external = super::super::external::ExternalResolution::default();
        build_resolved_targets(&summary_for(&model), &deploy, "brake", &external)
            .expect("resolve")
    }

    #[test]
    fn deadline_per_invoke_only_no_notice_no_inheritance() {
        let res = resolve_for_mesh_rpc(Some(50), "");
        assert!(res.deadline_overrides.is_empty(), "no binding-level deadline → no notice");
        let site = &res.targets[0].invoke_sites[0];
        assert_eq!(site.deadline_ms, Some(50));
    }

    #[test]
    fn deadline_binding_only_inherits_to_site() {
        let res = resolve_for_mesh_rpc(None, "\n            deadline_ms: 75");
        assert!(res.deadline_overrides.is_empty(), "per-invoke absent → no notice");
        let site = &res.targets[0].invoke_sites[0];
        assert_eq!(site.deadline_ms, Some(75), "binding fallback should populate site");
    }

    #[test]
    fn deadline_both_set_equal_no_notice() {
        let res = resolve_for_mesh_rpc(Some(40), "\n            deadline_ms: 40");
        assert!(res.deadline_overrides.is_empty(), "equal values → silent");
        assert_eq!(res.targets[0].invoke_sites[0].deadline_ms, Some(40));
    }

    #[test]
    fn deadline_both_set_diverge_emits_notice_and_per_invoke_wins() {
        let res = resolve_for_mesh_rpc(Some(50), "\n            deadline_ms: 200");
        assert_eq!(res.deadline_overrides.len(), 1, "diverging values → exactly one notice");
        let n = &res.deadline_overrides[0];
        assert_eq!(n.param_value, 50);
        assert_eq!(n.binding_value, 200);
        assert_eq!(n.invoke_id, "_invoke_0");
        assert_eq!(n.target.as_str(), "#motor");
        // §9.5: per-invoke is authoritative.
        assert_eq!(res.targets[0].invoke_sites[0].deadline_ms, Some(50));
        // Display message must name both values so a deploy author can
        // diff intent vs effect without re-running with --verbose.
        let msg = format!("{n}");
        assert!(msg.contains("50ms"), "notice should name per-invoke value: {msg}");
        assert!(msg.contains("200ms"), "notice should name binding value: {msg}");
    }

    #[test]
    fn deadline_both_absent_remains_none() {
        let res = resolve_for_mesh_rpc(None, "");
        assert!(res.deadline_overrides.is_empty());
        assert_eq!(res.targets[0].invoke_sites[0].deadline_ms, None);
    }

    // ── inject_auto_subscriptions ─────────────────────────────

    /// SCXML with a qualifying onentry subscribe (direct child, no manual unsubscribe).
    const AUTO_SUB_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="null" name="sub_test" initial="monitoring">
    <state id="monitoring">
        <onentry>
            <send target="#bus" event="event.subscribe.brake_status"/>
        </onentry>
        <transition event="event.notification.brake_status" target="monitoring"/>
    </state>
</scxml>"##;

    /// SCXML with a subscribe nested inside <if> (should lint, not auto-generate).
    const NESTED_SUB_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="null" name="nested_test" initial="s">
    <state id="s">
        <onentry>
            <if cond="true">
                <send target="#bus" event="event.subscribe.speed"/>
            </if>
        </onentry>
    </state>
</scxml>"##;

    /// SCXML with a manual unsubscribe in onexit (should suppress auto-generation).
    const MANUAL_UNSUB_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="null" name="manual_test" initial="s">
    <state id="s">
        <onentry>
            <send target="#bus" event="event.subscribe.brake_status"/>
        </onentry>
        <onexit>
            <send target="#bus" event="event.unsubscribe.brake_status"/>
        </onexit>
    </state>
</scxml>"##;

    #[test]
    fn auto_subscription_injects_unsubscribe_in_onexit() {
        let mut model = parse_model(AUTO_SUB_SCXML, "sub_test");
        let (sites, notices) = inject_auto_subscriptions(&mut model);

        assert_eq!(sites.len(), 1, "one qualifying site");
        assert_eq!(sites[0].state_id, "monitoring");
        assert_eq!(sites[0].subscribe_event, "event.subscribe.brake_status");
        assert_eq!(sites[0].unsubscribe_event, "event.unsubscribe.brake_status");
        assert_eq!(sites[0].target, "#bus");
        assert!(notices.is_empty(), "no lint notices for qualifying site");

        // Verify the model was mutated: onexit block should have the synthetic send.
        let state = model.states.get("monitoring").expect("state exists");
        assert!(!state.on_exit_blocks.is_empty(), "onexit block created");
        let exit_block = &state.on_exit_blocks[0];
        let unsub = exit_block.iter().find(|a| {
            a.action_type == "send" && a.event == "event.unsubscribe.brake_status"
        });
        assert!(unsub.is_some(), "synthetic unsubscribe send injected");
        let unsub = unsub.unwrap();
        assert_eq!(unsub.target, "#bus");
    }

    #[test]
    fn nested_subscribe_emits_lint_notice() {
        let mut model = parse_model(NESTED_SUB_SCXML, "nested_test");
        let (sites, notices) = inject_auto_subscriptions(&mut model);

        assert!(sites.is_empty(), "nested subscribe should not auto-generate");
        assert_eq!(notices.len(), 1, "one lint notice");
        assert_eq!(notices[0].event, "event.subscribe.speed");
        assert!(matches!(
            notices[0].reason,
            SubscriptionLintReason::NestedInConditional
        ));
    }

    #[test]
    fn manual_unsubscribe_suppresses_auto_generation() {
        let mut model = parse_model(MANUAL_UNSUB_SCXML, "manual_test");
        let (sites, notices) = inject_auto_subscriptions(&mut model);

        assert!(sites.is_empty(), "manual unsubscribe suppresses auto-generation");
        assert_eq!(notices.len(), 1, "one lint notice for manual suppression");
        assert!(matches!(
            notices[0].reason,
            SubscriptionLintReason::ManualUnsubscribePresent
        ));
    }

    #[test]
    fn auto_subscription_detected_by_collect_send_summary() {
        let mut model = parse_model(AUTO_SUB_SCXML, "sub_test");
        inject_auto_subscriptions(&mut model);

        // After injection, collect_send_summary should see both subscribe and unsubscribe.
        let summary = collect_send_summary(&model);
        let events: Vec<&str> = summary.actions.iter().map(|a| a.event.as_str()).collect();
        assert!(events.contains(&"event.subscribe.brake_status"), "subscribe detected");
        assert!(events.contains(&"event.unsubscribe.brake_status"), "unsubscribe detected");
    }

    #[test]
    fn subscribe_events_exempt_from_coverage() {
        let deploy = parse_deploy_str(
            r##"version: "1.0"
topology:
  ecu1:
    machines:
      sub_test: { source: sub_test.scxml, bindings: { "#bus": { transport: local } } }
      bus: { source: bus.scxml }
"##,
        )
        .unwrap();
        let mut model = parse_model(AUTO_SUB_SCXML, "sub_test");
        inject_auto_subscriptions(&mut model);

        let bus_scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="null" name="bus" initial="idle">
    <state id="idle"/>
</scxml>"##;
        let bus_model = parse_model(bus_scxml, "bus");
        let receivers = vec![("bus".to_string(), bus_model)];

        let summary = collect_send_summary(&model);
        let findings = check_sender_event_coverage("sub_test", &summary, &receivers, &deploy);
        assert!(
            findings.is_empty(),
            "subscribe/unsubscribe events should be exempt from coverage: {findings:?}"
        );
    }

    // ── §10.6 ordering validation ────────────────────────────

    #[test]
    fn zenoh_with_ordering_required_passes_topology() {
        // Zenoh: supplies_ordering=false, ordering_representable=true.
        // `ordering: required` is valid — codegen will emit the runtime
        // buffer — and must traverse topology without error.
        let yaml = r##"version: "1.0"
topology:
  ecu1:
    machines:
      sender:
        source: s.scxml
        bindings:
          "#receiver":
            transport: zenoh
            key: receiver/key
            ordering: required
      receiver: { source: r.scxml }
"##;
        let deploy = parse_deploy_str(yaml).unwrap();
        let model = parse_model(SHM_SCXML, "sender");
        let external = super::super::external::ExternalResolution::default();
        let resolved = build_resolved_targets(
            &summary_for(&model),
            &deploy,
            "sender",
            &external,
        )
        .expect("zenoh + ordering:required is valid");
        assert_eq!(resolved.targets.len(), 1);
        assert_eq!(
            resolved.targets[0].ordering,
            crate::mesh::deploy::OrderingRequirement::Required,
            "ordering must survive the topology→resolved pipeline"
        );
    }

    #[test]
    fn zenoh_with_ordering_none_passes_topology() {
        // Default path: ordering key absent → OrderingRequirement::None.
        // No runtime buffer emitted; arrival order contract.
        let yaml = r##"version: "1.0"
topology:
  ecu1:
    machines:
      sender:
        source: s.scxml
        bindings:
          "#receiver":
            transport: zenoh
            key: receiver/key
      receiver: { source: r.scxml }
"##;
        let deploy = parse_deploy_str(yaml).unwrap();
        let model = parse_model(SHM_SCXML, "sender");
        let external = super::super::external::ExternalResolution::default();
        let resolved = build_resolved_targets(
            &summary_for(&model),
            &deploy,
            "sender",
            &external,
        )
        .expect("zenoh without ordering key is valid");
        assert_eq!(
            resolved.targets[0].ordering,
            crate::mesh::deploy::OrderingRequirement::None
        );
    }

    #[test]
    fn can_with_ordering_required_rejected_at_topology() {
        // CAN: ordering_representable=false (broadcast bus). A binding
        // that asks for runtime-reconstructed order is structurally
        // unrepairable — topology must reject before codegen reaches
        // the `implemented=false` check. The resulting diagnostic
        // points at the CAN binding and enumerates the two repair
        // paths in the error message.
        let yaml = r##"version: "1.0"
topology:
  ecu1:
    machines:
      sender:
        source: s.scxml
        bindings:
          "#receiver":
            transport: can
            ordering: required
      receiver: { source: r.scxml }
"##;
        let deploy = parse_deploy_str(yaml).unwrap();
        let model = parse_model(SHM_SCXML, "sender");
        let external = super::super::external::ExternalResolution::default();
        let err = build_resolved_targets(
            &summary_for(&model),
            &deploy,
            "sender",
            &external,
        )
        .expect_err("CAN + ordering:required must be rejected");
        match err {
            TopologyError::OrderingCannotBeGuaranteed { machine, target, transport } => {
                assert_eq!(machine, "sender");
                assert_eq!(target.as_str(), "#receiver");
                assert_eq!(transport, "can");
            }
            other => panic!("expected OrderingCannotBeGuaranteed, got {other:?}"),
        }
    }

    #[test]
    fn can_with_ordering_none_passes_ordering_check() {
        // CAN with default ordering (None) bypasses the §10.6 check
        // entirely — no runtime reconstruction is requested. The
        // build still fails later at codegen because CAN has
        // implemented=false, but the topology-stage ordering check
        // must not be the failure point.
        let yaml = r##"version: "1.0"
topology:
  ecu1:
    machines:
      sender:
        source: s.scxml
        bindings:
          "#receiver":
            transport: can
      receiver: { source: r.scxml }
"##;
        let deploy = parse_deploy_str(yaml).unwrap();
        let model = parse_model(SHM_SCXML, "sender");
        let external = super::super::external::ExternalResolution::default();
        // Topology succeeds — the ordering check is the only one we
        // care about here. Downstream codegen would reject CAN for
        // the separate implemented=false reason.
        let resolved = build_resolved_targets(
            &summary_for(&model),
            &deploy,
            "sender",
            &external,
        )
        .expect("CAN with ordering:none must clear the §10.6 check");
        assert_eq!(
            resolved.targets[0].ordering,
            crate::mesh::deploy::OrderingRequirement::None
        );
    }
}
