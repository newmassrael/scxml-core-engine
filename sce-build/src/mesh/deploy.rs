// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
//
// SCE Mesh deploy.yaml parser — topology, bindings, scheduler, QoS.
//
// Uses serde(flatten) on BindingConfig.extra so any transport-native
// configuration (DDS 22 QoS policies, SOME/IP service model, etc.)
// passes through to Jinja2 templates without schema knowledge.

use crate::mesh::error::DeployError;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Top-level deploy.yaml structure (SCE_MESH.md §14).
#[derive(Debug, Clone, Deserialize)]
pub struct DeployConfig {
    /// Schema version (e.g. "1.0").
    pub version: Option<String>,
    /// Scheduler configuration (Phase 2+).
    pub scheduler: Option<SchedulerConfig>,
    /// Device -> machine -> binding topology.
    pub topology: HashMap<String, DeviceConfig>,
    /// Discovery mode configuration (Phase 5).
    pub discovery: Option<serde_yaml_ng::Value>,
}

/// Scheduler configuration stub (Phase 2+ expansion).
#[derive(Debug, Clone, Deserialize)]
pub struct SchedulerConfig {
    /// Scheduler type: "tick", "event-driven", "cooperative".
    #[serde(rename = "type")]
    pub scheduler_type: Option<String>,
    /// Transport-native scheduler settings passed through to templates.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_yaml_ng::Value>,
}

/// Device-level configuration (one entry per device/ECU in the topology).
#[derive(Debug, Clone, Deserialize)]
pub struct DeviceConfig {
    /// Target platform (e.g. "linux-x86_64", "qnx-aarch64").
    pub platform: Option<String>,
    /// Build target triple.
    pub target: Option<String>,
    /// State machines deployed on this device.
    pub machines: HashMap<String, MachineConfig>,
}

/// Machine-level configuration (one state machine instance).
#[derive(Debug, Clone, Deserialize)]
pub struct MachineConfig {
    /// Target ID -> transport binding.
    /// Keys are SCXML `<send target="...">` values (e.g. "#motor").
    pub bindings: HashMap<String, BindingConfig>,
}

/// Transport binding for a single `<send>` target.
#[derive(Debug, Clone, Deserialize)]
pub struct BindingConfig {
    /// Transport type: "local", "shm", "someip", "dds", "zenoh", etc.
    pub transport: String,
    /// All transport-native settings pass through to Jinja2 templates.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_yaml_ng::Value>,
}

/// Parse a deploy.yaml file from disk.
pub fn parse_deploy(path: &Path) -> Result<DeployConfig, DeployError> {
    let content = std::fs::read_to_string(path).map_err(|e| DeployError::ReadFile {
        path: path.display().to_string(),
        source: e,
    })?;
    parse_deploy_str(&content)
}

/// Parse deploy.yaml from a string (filesystem-free, testable).
pub fn parse_deploy_str(content: &str) -> Result<DeployConfig, DeployError> {
    serde_yaml_ng::from_str(content).map_err(|e| DeployError::Yaml(e.to_string()))
}
