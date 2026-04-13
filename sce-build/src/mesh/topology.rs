// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh topology analyzer — collects <send> targets from an SCXML model,
// matches them against deploy.yaml bindings, and performs build-time validation.

use crate::mesh::deploy::{BindingConfig, DeployConfig};
use crate::mesh::error::TopologyError;
use crate::model::SCXMLModel;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// A resolved send target: SCXML <send> target matched to a deploy.yaml binding.
#[derive(Debug, Clone, Serialize)]
pub struct ResolvedTarget {
    /// The target ID from SCXML (e.g. "#motor").
    pub target: String,
    /// Events sent to this target (for documentation/validation).
    pub events: Vec<String>,
    /// Transport type from deploy.yaml binding.
    pub transport: String,
    /// Transport-native configuration from deploy.yaml binding.
    pub extra: std::collections::HashMap<String, serde_yaml_ng::Value>,
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

/// Check if a target is a W3C internal target handled by the SM engine.
fn is_internal_target(target: &str) -> bool {
    target == "#_parent"
        || target == "#_child"
        || target == "#_internal"
        || target == "#_scxml_"
}

// ── Single-pass send action collection ──────────────────────

/// Details of a single `<send>` action, collected for downstream validators.
#[derive(Debug, Clone)]
pub struct SendActionDetail {
    /// The state containing the `<send>`.
    pub state: String,
    /// The `target` attribute (e.g. "#motor").
    pub target: String,
    /// The `event` attribute.
    pub event: String,
    /// The `sce:qos` attribute (e.g. "reliable").
    pub mesh_qos: String,
    /// The `sce:pattern` attribute (e.g. "request").
    pub mesh_pattern: String,
}

/// Pre-collected `<send>` action data from a single model traversal.
///
/// Created by `collect_send_summary()` and consumed by `resolve_targets()`,
/// `validate_qos_consistency()`, `validate_pattern_capability()`, and
/// `check_sender_event_coverage()`. Eliminates redundant model traversals.
#[derive(Debug)]
pub struct SendActionSummary {
    /// Deduplicated external send targets (e.g. "#motor").
    pub targets: BTreeSet<String>,
    /// Dynamic target warnings (`targetexpr` cannot be statically resolved).
    pub dynamic_warnings: Vec<TopologyWarning>,
    /// (target, event) pairs for event coverage validation.
    pub target_events: Vec<(String, String)>,
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

        // External static target
        if !action.target.is_empty() && !is_internal_target(&action.target) {
            targets.insert(action.target.clone());
            target_events.push((action.target.clone(), action.event.clone()));
        }

        // Per-action details for QoS and pattern validation
        actions.push(SendActionDetail {
            state: state_id.to_string(),
            target: action.target.clone(),
            event: action.event.clone(),
            mesh_qos: action.mesh_qos.clone(),
            mesh_pattern: action.mesh_pattern.clone(),
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
fn collect_target_events(model: &SCXMLModel) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    for_each_send_action(model, |_, action| {
        if !action.target.is_empty() && !is_internal_target(&action.target) {
            pairs.push((action.target.clone(), action.event.clone()));
        }
    });
    pairs
}

// ── Target resolution ────────────────────────────────────────

/// Resolve SCXML send targets against deploy.yaml bindings for a specific machine.
///
/// Uses pre-collected targets and target_events from `SendActionSummary`
/// to avoid redundant model traversal.
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
    let mut events_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for (target, event) in &summary.target_events {
        events_map
            .entry(target.clone())
            .or_default()
            .push(event.clone());
    }

    // Validate: every SCXML target must have a deploy.yaml binding
    let mut unresolved = Vec::new();
    let mut resolved = Vec::new();

    for target in &summary.targets {
        match bindings.get(target) {
            Some(binding) => {
                resolved.push(ResolvedTarget {
                    target: target.clone(),
                    events: events_map.remove(target).unwrap_or_default(),
                    transport: binding.transport.clone(),
                    extra: binding.extra.clone(),
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
) -> Result<&'a std::collections::HashMap<String, BindingConfig>, TopologyError> {
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
    pub target: String,
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
            let receiver_name = target.trim_start_matches('#');

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
        let receiver = w.target.trim_start_matches('#');
        deploy
            .topology
            .values()
            .any(|d| d.machines.contains_key(receiver))
    });

    warnings
}

// ── QoS consistency validation ──────────────────────────────

/// Build-time warning about QoS intent/config mismatch.
#[derive(Debug, Clone)]
pub struct QosWarning {
    /// The sender machine name.
    pub sender: String,
    /// The state containing the <send>.
    pub state: String,
    /// The target (e.g. "#motor").
    pub target: String,
    /// The sce:qos intent from SCXML (e.g. "reliable").
    pub qos_intent: String,
    /// The transport type from deploy.yaml (e.g. "someip").
    pub transport: String,
    /// Human-readable reason for the warning.
    pub reason: String,
}

impl std::fmt::Display for QosWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "machine '{}' state '{}': <send target=\"{}\" sce:qos=\"{}\"/> \
             bound to transport '{}' — {}",
            self.sender, self.state, self.target, self.qos_intent,
            self.transport, self.reason
        )
    }
}

/// Known transport reliability characteristics for QoS cross-reference.
/// Returns `None` for transports whose reliability depends on configuration
/// (protocol, QoS profile, etc.) — caller falls through to extra-config checks.
fn transport_reliability(transport: &str) -> Option<&'static str> {
    match transport {
        "local" => Some("reliable"),  // same-process direct call
        "shm" => Some("reliable"),    // shared memory with sequence numbers
        "someip" => None,             // depends on TCP/UDP config
        "dds" => None,                // depends on DDS QoS profile
        "zenoh" => None,              // depends on zenoh QoS profile
        _ => None,
    }
}

/// Validate QoS intent from SCXML `sce:qos` attributes against deploy.yaml
/// transport bindings.
///
/// Uses pre-collected action details from `SendActionSummary` to avoid
/// redundant model traversal.
pub fn validate_qos_consistency(
    summary: &SendActionSummary,
    deploy: &DeployConfig,
    machine_name: &str,
) -> Vec<QosWarning> {
    let mut warnings = Vec::new();

    let bindings = match find_machine_bindings(deploy, machine_name) {
        Ok(b) => b,
        Err(_) => return warnings, // Machine not in deploy.yaml — no validation
    };

    for action in &summary.actions {
        let qos = &action.mesh_qos;
        if qos.is_empty() || action.target.is_empty() || is_internal_target(&action.target) {
            continue;
        }

        if let Some(binding) = bindings.get(&action.target) {
            // Check known transport reliability against intent
            if let Some(transport_rel) = transport_reliability(&binding.transport) {
                let mismatch = match (qos.as_str(), transport_rel) {
                    ("reliable", "best-effort") => Some(
                        "transport provides best-effort delivery, \
                         but sender requires reliable. Consider TCP-based \
                         transport or application-level acknowledgment",
                    ),
                    ("best-effort", "reliable") => None, // OK: reliable transport satisfies best-effort
                    _ => None,
                };
                if let Some(reason) = mismatch {
                    warnings.push(QosWarning {
                        sender: machine_name.to_string(),
                        state: action.state.clone(),
                        target: action.target.clone(),
                        qos_intent: qos.clone(),
                        transport: binding.transport.clone(),
                        reason: reason.to_string(),
                    });
                }
            }

            // Check extra config for protocol-level hints
            if let Some(protocol) = binding.extra.get("protocol") {
                if let Some(proto_str) = protocol.as_str() {
                    if qos == "reliable" && proto_str.eq_ignore_ascii_case("udp") {
                        warnings.push(QosWarning {
                            sender: machine_name.to_string(),
                            state: action.state.clone(),
                            target: action.target.clone(),
                            qos_intent: qos.clone(),
                            transport: binding.transport.clone(),
                            reason: format!(
                                "transport protocol is UDP (best-effort) but \
                                 sce:qos=\"reliable\" requires guaranteed delivery"
                            ),
                        });
                    }
                }
            }
        }
    }

    warnings
}

// ── Pattern capability validation ────────────────────────────

/// Validate that SCXML communication patterns are supported by bound transports
/// (SCE_MESH.md Section 8.2).
///
/// Uses pre-collected action details from `SendActionSummary` to avoid
/// redundant model traversal.
///
/// Returns:
///   Ok(violations) — all `sce:pattern` values recognized; violations are transport mismatches
///   Err(UnrecognizedPattern) — an `sce:pattern` value is not recognized (likely typo, build error)
pub fn validate_pattern_capability(
    summary: &SendActionSummary,
    deploy: &DeployConfig,
    machine_name: &str,
) -> Result<Vec<super::pattern::PatternViolation>, TopologyError> {
    use super::pattern::{detect_pattern, PatternViolation};
    use super::transport;

    let bindings = match find_machine_bindings(deploy, machine_name) {
        Ok(b) => b,
        Err(_) => return Ok(Vec::new()), // Machine not in deploy.yaml — no validation
    };

    let mut violations = Vec::new();

    for action in &summary.actions {
        if action.target.is_empty() || is_internal_target(&action.target) || action.event.is_empty()
        {
            continue;
        }

        // Convert empty mesh_pattern to None (Rust-idiomatic Option API)
        let explicit = if action.mesh_pattern.is_empty() {
            None
        } else {
            Some(action.mesh_pattern.as_str())
        };

        let pattern = match detect_pattern(&action.event, explicit) {
            Ok(Some(p)) => p,
            Ok(None) => continue, // Application-specific event or explicit opt-out
            Err(unrecognized) => {
                return Err(TopologyError::UnrecognizedPattern {
                    sender: machine_name.to_string(),
                    state: action.state.clone(),
                    target: action.target.clone(),
                    event: action.event.clone(),
                    value: unrecognized,
                });
            }
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

    Ok(violations)
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
        let receiver = t.target.trim_start_matches('#').to_string();
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
        let receiver_name = target.trim_start_matches('#');
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
        let receiver = w.target.trim_start_matches('#');
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
            target: "#motor".to_string(),
            events: vec!["brake.activate".to_string()],
            transport: "local".to_string(),
            extra: HashMap::new(),
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

        let violations = validate_pattern_capability(&summary_for(&model), &deploy, "tester").unwrap();
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

        let violations = validate_pattern_capability(&summary_for(&model), &deploy, "tester").unwrap();
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

        let violations = validate_pattern_capability(&summary_for(&model), &deploy, "subscriber").unwrap();
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

            let violations = validate_pattern_capability(&summary_for(&model), &deploy, "sender").unwrap();
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

        let violations = validate_pattern_capability(&summary_for(&model), &deploy, "app").unwrap();
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

        let violations = validate_pattern_capability(&summary_for(&model), &deploy, "tester").unwrap();
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
        let violations = validate_pattern_capability(&summary_for(&model), &deploy, "tester").unwrap();
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
