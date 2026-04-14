// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh deploy.yaml parser — topology, device-level shared transport
// config, per-target bindings, scheduler.
//
// Schema shape (SCE_MESH.md §14):
//   topology.<device>.transports.<transport>   — device-shared session config
//   topology.<device>.machines.<name>.bindings — per-target binding config
//
// Device-shared transports (e.g. zenoh opens one Session per device, shared
// by all machines on that device) declare their session-level config under
// `transports:`. Per-target keys (e.g. zenoh `key:`, someip `service_id:`)
// stay on individual bindings. This mirrors the runtime semantics: there
// is exactly one session per device and many bindings per session.
//
// Per-target binding extras use `serde(flatten)` so transport-native
// settings pass through to Jinja2 templates without the parser needing
// to know every key.

use crate::mesh::error::DeployError;
use crate::mesh::target::TargetId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Top-level deploy.yaml structure.
///
/// `deny_unknown_fields` catches typos at parse time — e.g. `topolgy:`
/// instead of `topology:` would otherwise be silently ignored.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeployConfig {
    /// Schema version. The compiler rejects unknown versions to prevent
    /// silent misinterpretation when the schema evolves.
    pub version: Option<String>,
    /// Scheduler configuration (future expansion).
    pub scheduler: Option<SchedulerConfig>,
    /// Device → `DeviceConfig` map.
    pub topology: HashMap<String, DeviceConfig>,
    /// Discovery mode configuration (future expansion).
    pub discovery: Option<serde_yaml_ng::Value>,
}

/// Scheduler configuration stub (future expansion).
#[derive(Debug, Clone, Deserialize)]
pub struct SchedulerConfig {
    /// Scheduler type: "tick", "event-driven", "cooperative".
    #[serde(rename = "type")]
    pub scheduler_type: Option<String>,
    /// Scheduler-native settings passed through to templates.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_yaml_ng::Value>,
}

/// Device-level configuration (one entry per device/ECU in the topology).
///
/// `transports` is where device-shared session config lives. Fields outside
/// `transports` and `machines` are reserved for build-system metadata
/// (`platform`, `target`). `deny_unknown_fields` prevents silent typos like
/// `platfrom:` from being ignored.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct DeviceConfig {
    /// Target platform (e.g. "linux-x86_64", "qnx-aarch64").
    pub platform: Option<String>,
    /// Build target triple.
    pub target: Option<String>,
    /// Device-shared transport configuration — one entry per transport type
    /// that has session-level state (e.g. zenoh). Omit the block entirely
    /// when no device-shared transports are used.
    #[serde(default)]
    pub transports: TransportConfigs,
    /// State machines deployed on this device.
    pub machines: HashMap<String, MachineConfig>,
}

/// Device-level transport config block.
///
/// Each field is an entry for a transport that has device-shared state. A
/// transport that has no shared state (local, shm, someip) does not appear
/// here — its entire config is per-binding. `deny_unknown_fields` catches
/// typos in transport names (e.g. `zneoh:`) at parse time.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransportConfigs {
    /// Zenoh session config — applied to the single `zenoh::Session` shared
    /// by all zenoh bindings on this device.
    pub zenoh: Option<ZenohTransportConfig>,
}

/// Zenoh device-shared session configuration.
///
/// Corresponds to the fields of `zenoh::Config` that are session-level
/// (vs per-publisher/per-subscriber). Applied by the generated
/// `TransportRouter::init()` via `Config::insert_json5`. `deny_unknown_fields`
/// prevents silent typos like `conncet:` or `lsten:`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ZenohTransportConfig {
    /// Session mode: peer | client | router.
    pub mode: Option<ZenohMode>,
    /// Endpoint list the session will actively connect to.
    pub connect: Option<Vec<String>>,
    /// Endpoint list the session will listen on for incoming connections.
    pub listen: Option<Vec<String>>,
}

/// Zenoh session mode.
///
/// Typed at parse time so an invalid value (typo, wrong case) fails the
/// build rather than being silently forwarded to the runtime. `serde`
/// rejects any value outside the three variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ZenohMode {
    /// Peer-to-peer: each node publishes and forwards.
    Peer,
    /// Client: connects to a router/peer, does not forward.
    Client,
    /// Router: relays messages between peers/clients.
    Router,
}

impl std::fmt::Display for ZenohMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Peer => write!(f, "peer"),
            Self::Client => write!(f, "client"),
            Self::Router => write!(f, "router"),
        }
    }
}

/// Machine-level configuration (one state machine instance).
///
/// `deny_unknown_fields` catches typos like `soruce:` at parse time.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MachineConfig {
    /// SCXML source path, relative to the deploy.yaml file.
    /// Required for event coverage validation: every receiver referenced
    /// by a `<send target="#X"/>` must declare its source so the analyzer
    /// can read its `<transition event="...">` set.
    ///
    /// Trust boundary: deploy.yaml is part of the build configuration and
    /// is trusted. Absolute paths are rejected as a portability signal,
    /// not a security gate. `..` is permitted for legitimate cross-directory
    /// layouts (e.g., `../shared/motor.scxml`).
    pub source: String,
    /// Target ID → transport binding. Keys are SCXML `<send target="...">`
    /// values (e.g. "#motor"), typed as `TargetId` so the domain model stays
    /// stringly-free past the deploy.yaml boundary.
    #[serde(default)]
    pub bindings: HashMap<TargetId, BindingConfig>,
}

/// Transport binding for a single `<send>` target.
///
/// `extra` uses `serde(flatten)` for per-target transport-native keys
/// (e.g. zenoh `key:`, someip `service_id:`). Device-shared session keys
/// live on `DeviceConfig::transports`, not here.
#[derive(Debug, Clone, Deserialize)]
pub struct BindingConfig {
    /// Transport type: "local", "shm", "someip", "dds", "zenoh", etc.
    pub transport: String,
    /// Per-target transport-native settings passed through to templates.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_yaml_ng::Value>,
}

/// Schema versions this compiler understands.
pub const SUPPORTED_VERSIONS: &[&str] = &["1.0"];

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
    let cfg: DeployConfig =
        serde_yaml_ng::from_str(content).map_err(|e| DeployError::Yaml(e.to_string()))?;

    if let Some(v) = &cfg.version {
        if !SUPPORTED_VERSIONS.contains(&v.as_str()) {
            return Err(DeployError::UnsupportedVersion {
                found: v.clone(),
                supported: SUPPORTED_VERSIONS.to_vec(),
            });
        }
    }

    validate_machine_name_uniqueness(&cfg)?;

    Ok(cfg)
}

/// Ensure each machine name appears under at most one device.
///
/// Machine names are used globally (receiver lookup, SCXML `<send
/// target="#X"/>` resolution, generated C++ namespace `SCE::Generated::X`),
/// so duplicates across devices would cause ambiguous resolution at best
/// and silent code-collision at worst.
///
/// Uses `BTreeMap` for deterministic error messages — the first duplicate
/// reported is the alphabetically earliest machine name regardless of the
/// underlying `HashMap` iteration order.
fn validate_machine_name_uniqueness(cfg: &DeployConfig) -> Result<(), DeployError> {
    use std::collections::BTreeMap;
    let mut seen: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (device_name, device) in &cfg.topology {
        for machine_name in device.machines.keys() {
            seen.entry(machine_name.as_str())
                .or_default()
                .push(device_name.as_str());
        }
    }
    for (machine, devices) in seen {
        if devices.len() > 1 {
            let mut sorted: Vec<String> = devices.into_iter().map(String::from).collect();
            sorted.sort();
            return Err(DeployError::DuplicateMachine {
                machine: machine.to_string(),
                devices: sorted,
            });
        }
    }
    Ok(())
}

// ── Helpers for topology/codegen ────────────────────────────

impl DeployConfig {
    /// Find the device config that owns a given machine.
    ///
    /// Since devices are a flat `HashMap`, this is O(devices) — fine for
    /// any realistic deployment. Returns `None` if the machine is not
    /// declared on any device.
    pub fn device_for_machine(&self, machine_name: &str) -> Option<&DeviceConfig> {
        self.topology
            .values()
            .find(|d| d.machines.contains_key(machine_name))
    }
}

// ── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_version_accepted() {
        let yaml = r#"
version: "1.0"
topology:
  ecu1:
    machines:
      brake: { source: brake.scxml }
"#;
        let cfg = parse_deploy_str(yaml).expect("parse");
        assert_eq!(cfg.version.as_deref(), Some("1.0"));
    }

    #[test]
    fn missing_version_accepted() {
        let yaml = r#"
topology:
  ecu1:
    machines:
      brake: { source: brake.scxml }
"#;
        let cfg = parse_deploy_str(yaml).expect("parse");
        assert!(cfg.version.is_none());
    }

    #[test]
    fn unknown_version_rejected() {
        let yaml = r#"
version: "9.9"
topology:
  ecu1:
    machines:
      brake: { source: brake.scxml }
"#;
        match parse_deploy_str(yaml) {
            Err(DeployError::UnsupportedVersion { found, supported }) => {
                assert_eq!(found, "9.9");
                assert!(supported.contains(&"1.0"));
            }
            other => panic!("expected UnsupportedVersion, got {other:?}"),
        }
    }

    #[test]
    fn bindings_default_to_empty() {
        let yaml = r#"
version: "1.0"
topology:
  ecu1:
    machines:
      motor: { source: motor.scxml }
"#;
        let cfg = parse_deploy_str(yaml).expect("parse");
        let motor = &cfg.topology["ecu1"].machines["motor"];
        assert_eq!(motor.source, "motor.scxml");
        assert!(motor.bindings.is_empty());
    }

    #[test]
    fn device_transports_parsed() {
        let yaml = r#"
version: "1.0"
topology:
  ecu1:
    transports:
      zenoh:
        mode: peer
        connect:
          - "tcp/192.168.1.1:7447"
        listen:
          - "tcp/0.0.0.0:7447"
    machines:
      brake:
        source: brake.scxml
"#;
        let cfg = parse_deploy_str(yaml).expect("parse");
        let zenoh = cfg.topology["ecu1"]
            .transports
            .zenoh
            .as_ref()
            .expect("zenoh block");
        assert_eq!(zenoh.mode, Some(ZenohMode::Peer));
        assert_eq!(zenoh.connect.as_deref(), Some(&["tcp/192.168.1.1:7447".to_string()][..]));
        assert_eq!(zenoh.listen.as_deref(), Some(&["tcp/0.0.0.0:7447".to_string()][..]));
    }

    #[test]
    fn invalid_zenoh_mode_rejected_at_parse_time() {
        // Typo in mode — must be rejected before any topology validation.
        // serde rejects values outside the lowercase enum variants.
        let yaml = r#"
version: "1.0"
topology:
  ecu1:
    transports:
      zenoh:
        mode: pier
    machines:
      brake: { source: brake.scxml }
"#;
        let err = parse_deploy_str(yaml).unwrap_err();
        match err {
            DeployError::Yaml(msg) => assert!(msg.to_lowercase().contains("mode"), "msg: {msg}"),
            other => panic!("expected Yaml parse error, got {other:?}"),
        }
    }

    #[test]
    fn uppercase_zenoh_mode_rejected() {
        // rename_all = "lowercase" means "Peer" is not accepted.
        let yaml = r#"
version: "1.0"
topology:
  ecu1:
    transports:
      zenoh:
        mode: Peer
    machines:
      brake: { source: brake.scxml }
"#;
        assert!(matches!(parse_deploy_str(yaml), Err(DeployError::Yaml(_))));
    }

    #[test]
    fn device_without_transports_ok() {
        let yaml = r#"
version: "1.0"
topology:
  ecu1:
    machines:
      brake: { source: brake.scxml }
"#;
        let cfg = parse_deploy_str(yaml).expect("parse");
        assert!(cfg.topology["ecu1"].transports.zenoh.is_none());
    }

    #[test]
    fn device_for_machine_resolves() {
        let yaml = r#"
version: "1.0"
topology:
  ecu1:
    platform: linux-x86_64
    machines:
      brake: { source: brake.scxml }
  ecu2:
    platform: qnx-aarch64
    machines:
      motor: { source: motor.scxml }
"#;
        let cfg = parse_deploy_str(yaml).expect("parse");
        assert_eq!(
            cfg.device_for_machine("brake").unwrap().platform.as_deref(),
            Some("linux-x86_64")
        );
        assert_eq!(
            cfg.device_for_machine("motor").unwrap().platform.as_deref(),
            Some("qnx-aarch64")
        );
        assert!(cfg.device_for_machine("unknown").is_none());
    }

    // ── deny_unknown_fields ──────────────────────────────────

    #[test]
    fn top_level_unknown_field_rejected() {
        // Typo: `topolgy` instead of `topology`. Must fail at parse time,
        // not silently produce an empty topology.
        let yaml = r#"
version: "1.0"
topolgy:
  ecu1: { machines: { brake: { source: brake.scxml } } }
"#;
        assert!(matches!(parse_deploy_str(yaml), Err(DeployError::Yaml(_))));
    }

    #[test]
    fn device_unknown_field_rejected() {
        // Typo: `platfrom` instead of `platform`.
        let yaml = r#"
version: "1.0"
topology:
  ecu1:
    platfrom: linux-x86_64
    machines:
      brake: { source: brake.scxml }
"#;
        assert!(matches!(parse_deploy_str(yaml), Err(DeployError::Yaml(_))));
    }

    #[test]
    fn machine_unknown_field_rejected() {
        // Typo: `soruce` instead of `source`.
        let yaml = r#"
version: "1.0"
topology:
  ecu1:
    machines:
      brake: { soruce: brake.scxml }
"#;
        assert!(matches!(parse_deploy_str(yaml), Err(DeployError::Yaml(_))));
    }

    #[test]
    fn transports_unknown_transport_name_rejected() {
        // Typo: `zneoh` instead of `zenoh`. Without deny_unknown_fields the
        // entire block would be silently discarded.
        let yaml = r#"
version: "1.0"
topology:
  ecu1:
    transports:
      zneoh:
        mode: peer
    machines:
      brake: { source: brake.scxml }
"#;
        assert!(matches!(parse_deploy_str(yaml), Err(DeployError::Yaml(_))));
    }

    #[test]
    fn zenoh_config_unknown_field_rejected() {
        // Typo: `conncet` instead of `connect`.
        let yaml = r#"
version: "1.0"
topology:
  ecu1:
    transports:
      zenoh:
        mode: peer
        conncet: ["tcp/host:7447"]
    machines:
      brake: { source: brake.scxml }
"#;
        assert!(matches!(parse_deploy_str(yaml), Err(DeployError::Yaml(_))));
    }

    // ── Machine name uniqueness ──────────────────────────────

    #[test]
    fn duplicate_machine_across_devices_rejected() {
        let yaml = r#"
version: "1.0"
topology:
  ecu1:
    machines:
      brake: { source: brake.scxml }
  ecu2:
    machines:
      brake: { source: other_brake.scxml }
"#;
        match parse_deploy_str(yaml) {
            Err(DeployError::DuplicateMachine { machine, devices }) => {
                assert_eq!(machine, "brake");
                // Sorted for determinism
                assert_eq!(devices, vec!["ecu1".to_string(), "ecu2".to_string()]);
            }
            other => panic!("expected DuplicateMachine, got {other:?}"),
        }
    }

    #[test]
    fn duplicate_machine_reports_alphabetically_first() {
        // Both "brake" and "motor" are duplicated. The error must report
        // the alphabetically first name (deterministic).
        let yaml = r#"
version: "1.0"
topology:
  ecu1:
    machines:
      brake: { source: brake.scxml }
      motor: { source: motor.scxml }
  ecu2:
    machines:
      brake: { source: other_brake.scxml }
      motor: { source: other_motor.scxml }
"#;
        match parse_deploy_str(yaml) {
            Err(DeployError::DuplicateMachine { machine, .. }) => {
                assert_eq!(machine, "brake"); // < "motor" alphabetically
            }
            other => panic!("expected DuplicateMachine, got {other:?}"),
        }
    }

    #[test]
    fn same_machine_name_within_same_device_takes_last_wins() {
        // Known limitation: serde_yaml_ng silently keeps the last value for
        // duplicate keys within the same mapping (YAML 1.2 does not mandate
        // rejection). The resulting HashMap has one entry. Detecting this
        // would require a custom deserializer that intercepts each mapping
        // and records seen keys before they are folded into the HashMap.
        //
        // This test pins the current behavior so a future library upgrade
        // or custom deserializer that changes semantics is caught.
        let yaml = r#"
version: "1.0"
topology:
  ecu1:
    machines:
      brake: { source: a.scxml }
      brake: { source: b.scxml }
"#;
        let cfg = parse_deploy_str(yaml).expect("parse");
        let ecu1 = &cfg.topology["ecu1"];
        assert_eq!(ecu1.machines.len(), 1);
        assert_eq!(ecu1.machines["brake"].source, "b.scxml");
    }

    #[test]
    fn unique_machines_across_devices_accepted() {
        let yaml = r#"
version: "1.0"
topology:
  ecu1:
    machines:
      brake: { source: brake.scxml }
  ecu2:
    machines:
      motor: { source: motor.scxml }
"#;
        parse_deploy_str(yaml).expect("parse");
    }
}
