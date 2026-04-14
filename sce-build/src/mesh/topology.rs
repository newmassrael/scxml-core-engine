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

/// A resolved send target: SCXML <send> target matched to a deploy.yaml binding.
#[derive(Debug, Clone, Serialize)]
pub struct ResolvedTarget {
    /// The target ID from SCXML (e.g. "#motor").
    pub target: TargetId,
    /// Events sent to this target (for documentation/validation).
    pub events: Vec<String>,
    /// Transport type from deploy.yaml binding.
    pub transport: String,
    /// Transport-native configuration from deploy.yaml binding.
    pub extra: std::collections::HashMap<String, serde_yaml_ng::Value>,
    /// Per-event pattern metadata for codegen (pattern-aware send + RPC correlation).
    pub event_patterns: Vec<EventPatternInfo>,
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
        let Some(pattern) = action.pattern else {
            continue;
        };
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

/// Resolve SCXML send targets against deploy.yaml bindings for a specific machine.
///
/// Uses pre-collected targets and target_events from `SendActionSummary`
/// to avoid redundant model traversal. Pattern/pairing metadata is produced
/// by `analyze_event_pairs` and consumed here — this function itself is
/// responsible only for deploy.yaml binding resolution.
///
/// Returns an error if any SCXML target has no matching binding in deploy.yaml.
pub fn resolve_targets(
    summary: &SendActionSummary,
    deploy: &DeployConfig,
    machine_name: &str,
) -> Result<Vec<ResolvedTarget>, TopologyError> {
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
    let mut resolved = Vec::new();

    for target in &summary.targets {
        match bindings.get(target) {
            Some(binding) => {
                resolved.push(ResolvedTarget {
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
    for rt in &resolved {
        if let Some(desc) = super::transport::lookup(&rt.transport) {
            for &field in desc.required_binding_fields {
                if !rt.extra.contains_key(field) {
                    return Err(TopologyError::MissingBindingField {
                        machine: machine_name.to_string(),
                        target: rt.target.clone(),
                        transport: rt.transport.clone(),
                        field: field.to_string(),
                    });
                }
            }
        }

        // Transport-specific optional field validation. These fields are
        // optional (fall back to defaults) but must be well-formed when
        // present. SCE_MESH.md Section 7.5.
        if rt.transport == "shm" {
            validate_shm_extras(machine_name, rt)?;
        }

        // Pattern-specific field validation for someip: certain deploy.yaml
        // fields are required based on which communication patterns the SCXML
        // model uses. Without this, missing fields silently generate `return
        // false` in the template — a runtime failure instead of a build error.
        if rt.transport == "someip" {
            validate_someip_pattern_fields(machine_name, rt)?;
        }
    }

    Ok(resolved)
}

/// Validate optional shm binding fields:
///   - `shm_arena_bytes`   positive integer, must fit in u32 (offset/length
///                         fields in the wire layout use uint32_t)
///   - `shm_ring_capacity` positive power of two (EventQueueBridge
///                         requires power-of-two capacity)
fn validate_shm_extras(
    machine_name: &str,
    rt: &ResolvedTarget,
) -> Result<(), TopologyError> {
    let invalid = |field: &str, reason: String| TopologyError::InvalidBindingField {
        machine: machine_name.to_string(),
        target: rt.target.clone(),
        transport: rt.transport.clone(),
        field: field.to_string(),
        reason,
    };

    if let Some(v) = rt.extra.get("shm_arena_bytes") {
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

    if let Some(v) = rt.extra.get("shm_ring_capacity") {
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

/// Validate someip pattern-specific deploy.yaml fields.
///
/// While `service_id` and `instance_id` are always required (enforced by
/// `required_binding_fields`), other IDs depend on which communication
/// patterns the SCXML model uses for this target:
///   - FireForget / ServiceRequest / ServiceResponse → `method_id`
///   - Subscribe / Notification → `event_group_id` + `event_id`
///   - FieldGet → `getter_id`
///   - FieldSet → `setter_id`
fn validate_someip_pattern_fields(
    machine_name: &str,
    rt: &ResolvedTarget,
) -> Result<(), TopologyError> {
    use super::pattern::{
        WIRE_EVENT_NOTIFY, WIRE_EVENT_SUBSCRIBE, WIRE_FIELD_READ, WIRE_FIELD_WRITE,
        WIRE_FIRE_FORGET, WIRE_RPC_REPLY, WIRE_RPC_REQUEST,
    };
    for ep in &rt.event_patterns {
        let require = |field: &str| {
            if !rt.extra.contains_key(field) {
                Err(TopologyError::MissingBindingField {
                    machine: machine_name.to_string(),
                    target: rt.target.clone(),
                    transport: rt.transport.clone(),
                    field: field.to_string(),
                })
            } else {
                Ok(())
            }
        };

        match ep.pattern_kind_value {
            WIRE_FIRE_FORGET | WIRE_RPC_REQUEST | WIRE_RPC_REPLY => {
                require("method_id")?
            }
            WIRE_EVENT_SUBSCRIBE | WIRE_EVENT_NOTIFY => {
                require("event_group_id")?;
                require("event_id")?;
            }
            WIRE_FIELD_READ => require("getter_id")?,
            WIRE_FIELD_WRITE => require("setter_id")?,
            _ => {}
        }
    }
    Ok(())
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
// SCXML annotation. The earlier sce:qos intent/consistency validator was
// removed with the attribute in Session E1.


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
        vec![ResolvedTarget {
            target: TargetId::new("#motor").unwrap(),
            events: vec!["brake.activate".to_string()],
            transport: "local".to_string(),
            extra: HashMap::new(),
            event_patterns: Vec::new(),
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
        resolve_targets(&summary_for(&model), &deploy, "sender")
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
