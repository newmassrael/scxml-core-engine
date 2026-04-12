// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
//
// SCE Mesh topology analyzer — collects <send> targets from an SCXML model,
// matches them against deploy.yaml bindings, and performs build-time validation.

use crate::mesh::deploy::{BindingConfig, DeployConfig};
use crate::mesh::error::TopologyError;
use crate::model::SCXMLModel;
use serde::Serialize;
use std::collections::BTreeSet;

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
