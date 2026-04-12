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

// ── Public analysis functions ────────────────────────────────

/// Collect all external `<send>` targets from an SCXML model.
///
/// Filters out internal targets (#_parent, #_child, #_internal) and
/// empty targets (same-machine events). Returns deduplicated target IDs.
pub fn collect_send_targets(model: &SCXMLModel) -> BTreeSet<String> {
    let mut targets = BTreeSet::new();
    for_each_send_action(model, |_, action| {
        if !action.target.is_empty() && !is_internal_target(&action.target) {
            targets.insert(action.target.clone());
        }
    });
    targets
}

/// Detect `<send targetexpr="...">` actions that cannot be statically resolved.
/// Returns warnings for each occurrence (emitted to stderr by the caller).
pub fn collect_dynamic_target_warnings(model: &SCXMLModel) -> Vec<TopologyWarning> {
    let mut warnings = Vec::new();
    for_each_send_action(model, |state_id, action| {
        if !action.targetexpr.is_empty() {
            warnings.push(TopologyWarning {
                state: state_id.to_string(),
                targetexpr: action.targetexpr.clone(),
            });
        }
    });
    warnings
}

/// Collect (target, event) pairs from an SCXML model for documentation/validation.
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
/// `machine_name` is the SCXML document name (`<scxml name="...">`), which
/// must match a machine key in the deploy.yaml topology.
///
/// Returns an error if any SCXML target has no matching binding in deploy.yaml.
pub fn resolve_targets(
    model: &SCXMLModel,
    deploy: &DeployConfig,
    machine_name: &str,
) -> Result<Vec<ResolvedTarget>, TopologyError> {
    let send_targets = collect_send_targets(model);
    if send_targets.is_empty() {
        return Ok(Vec::new());
    }

    // Find the machine's bindings in deploy.yaml topology
    let bindings = find_machine_bindings(deploy, machine_name)?;

    // Collect events per target for documentation
    let target_events = collect_target_events(model);
    let mut events_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for (target, event) in &target_events {
        events_map
            .entry(target.clone())
            .or_default()
            .push(event.clone());
    }

    // Validate: every SCXML target must have a deploy.yaml binding
    let mut unresolved = Vec::new();
    let mut resolved = Vec::new();

    for target in &send_targets {
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

    Ok(resolved)
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
/// Cross-references each `<send sce:qos="reliable">` against the bound
/// transport's known reliability characteristics. Returns warnings for
/// mismatches (e.g., "reliable" intent on a best-effort transport).
pub fn validate_qos_consistency(
    model: &SCXMLModel,
    deploy: &DeployConfig,
    machine_name: &str,
) -> Vec<QosWarning> {
    let mut warnings = Vec::new();

    let bindings = match find_machine_bindings(deploy, machine_name) {
        Ok(b) => b,
        Err(_) => return warnings, // Machine not in deploy.yaml — no validation
    };

    for_each_send_action(model, |state_id, action| {
        let qos = &action.mesh_qos;
        if qos.is_empty() || action.target.is_empty() || is_internal_target(&action.target) {
            return;
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
                        state: state_id.to_string(),
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
                            state: state_id.to_string(),
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
    });

    warnings
}
