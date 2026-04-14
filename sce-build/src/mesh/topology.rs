// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh topology analyzer — collects <send> targets from an SCXML model,
// matches them against deploy.yaml bindings, and performs build-time validation.

use crate::mesh::deploy::{BindingConfig, DeployConfig};
use crate::mesh::error::TopologyError;
use crate::mesh::target::TargetId;
use crate::model::SCXMLModel;
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
        match pattern.someip_field() {
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
    /// Deduplicated external send targets (e.g. "#motor").
    pub targets: BTreeSet<TargetId>,
    /// Dynamic target warnings (`targetexpr` cannot be statically resolved).
    pub dynamic_warnings: Vec<TopologyWarning>,
    /// (target, event) pairs for event coverage validation.
    pub target_events: Vec<(TargetId, String)>,
    /// Per-action details for QoS and pattern validation.
    pub actions: Vec<SendActionDetail>,
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

    SendActionSummary {
        targets,
        dynamic_warnings,
        target_events,
        actions,
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
) -> Result<Vec<PartialTarget>, TopologyError> {
    if summary.targets.is_empty() {
        return Ok(Vec::new());
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

    for target in &summary.targets {
        match bindings.get(target) {
            Some(binding) => {
                partials.push(PartialTarget {
                    target: target.clone(),
                    events: events_map.remove(target).unwrap_or_default(),
                    transport: binding.transport.clone(),
                    extra: binding.extra.clone(),
                    event_patterns: pattern_map.remove(target).unwrap_or_default(),
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

        // SOME/IP per-event ID validation runs after `finalize_targets`
        // has fanned the per-binding resolution out into per-event entries.
    }

    Ok(partials)
}

/// Public pipeline entry: resolve, attach, validate in a single step so
/// external callers cannot observe (let alone construct) a half-built
/// [`ResolvedTarget`]. Replaces the sequence
/// `resolve_targets → attach_event_bindings → validate_someip_event_fields`
/// that previously required correct caller ordering.
pub fn build_resolved_targets(
    summary: &SendActionSummary,
    deploy: &DeployConfig,
    machine_name: &str,
    external: &super::external::ExternalResolution,
) -> Result<Vec<ResolvedTarget>, TopologyError> {
    let partials = resolve_partials(summary, deploy, machine_name)?;
    let resolved = finalize_targets(partials, machine_name, external)?;
    validate_someip_event_fields(&resolved, machine_name)?;
    Ok(resolved)
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
        let expected = pattern.someip_field();
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

        let state = build_transport_state(&pt, someip_resolved);

        resolved.push(ResolvedTarget {
            target: pt.target,
            events: pt.events,
            event_patterns: pt.event_patterns,
            state,
        });
    }

    Ok(resolved)
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
        let rx_model = parser.parse_string(&content, &receiver).map_err(|reason| {
            TopologyError::ReceiverSourceParse {
                machine: receiver.clone(),
                path: source_path.display().to_string(),
                reason,
            }
        })?;

        out.push((receiver, rx_model));
    }

    Ok(out)
}

/// Strict event coverage check for a single sender: every `<send event="Y"/>`
/// to a static target must have a matching `<transition event="Y"/>` in the
/// receiver.
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

    let mut findings = Vec::new();
    for (target, event) in &summary.target_events {
        if event.is_empty() {
            continue; // content-only <send>; no event name to match
        }
        let receiver_name = target.name();
        let rx_events = match receiver_events.get(receiver_name) {
            Some(s) => s,
            None => continue, // receiver model not supplied — out of scope
        };
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

// ── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::deploy::parse_deploy_str;
    use std::collections::HashMap;
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
}
