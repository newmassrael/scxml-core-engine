// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

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
/// transport that has no shared state (local, shm) does not appear here —
/// its entire config is per-binding. SOME/IP and Zenoh appear here because
/// they carry device-shared identity (vsomeip application name, zenoh
/// session) and reference external OEM config files (SCE_MESH.md §13).
/// `deny_unknown_fields` catches typos in transport names (e.g. `zneoh:`)
/// at parse time.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransportConfigs {
    /// Zenoh session config — applied to the single `zenoh::Session` shared
    /// by all zenoh bindings on this device.
    pub zenoh: Option<ZenohTransportConfig>,
    /// SOME/IP device-shared config — references the OEM-supplied
    /// vsomeip.json (single source of truth for service/method IDs) and
    /// binds the generated runtime to a vsomeip application identity.
    pub someip: Option<SomeipTransportConfig>,
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
    /// Path (relative to deploy.yaml) to an external zenoh.json5 session
    /// config. `mode`/`connect`/`listen` above, when present, merge over
    /// this file at runtime. SCE_MESH.md §13 / §14.
    pub config: Option<PathBuf>,
}

/// SOME/IP device-shared configuration (SCE_MESH.md §13).
///
/// `config:` points to the OEM-supplied `vsomeip.json`; `application_name`
/// matches one of `applications[*].name` inside it. `deny_unknown_fields`
/// catches typos like `applicaton_name:` at parse time.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SomeipTransportConfig {
    /// Path (relative to deploy.yaml) to the external vsomeip.json.
    pub config: Option<PathBuf>,
    /// Matches `applications[*].name` in vsomeip.json.
    pub application_name: Option<String>,
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
    /// Machine-lifetime subscriptions (SCE_MESH.md §13). Subscribe on
    /// engine init, unsubscribe on engine shutdown. Each entry names an
    /// event to subscribe to and the source target that hosts the
    /// publisher.
    ///
    /// ```yaml
    /// subscriptions:
    ///   - event: event.notification.vehicle_speed
    ///     source: "#chassis"
    /// ```
    #[serde(default)]
    pub subscriptions: Vec<SubscriptionConfig>,
    /// Server-side transport registration (SCE_MESH.md §13 Session E).
    ///
    /// Declares that this machine acts as a transport-native server: it
    /// receives inbound RPC requests through the transport layer and
    /// sends responses back via the transport's reply mechanism (SOME/IP
    /// `create_response` + `app.send`, Zenoh `Query::reply`).
    ///
    /// Server detection is dual-source:
    ///   - SCXML model inference tells WHAT the machine serves (transitions
    ///     on `service.request.X` paired with sends of `service.response.X`)
    ///   - This section tells HOW: transport type, service identity, per-event
    ///     method IDs (SOME/IP) or key expression (Zenoh)
    ///
    /// ```yaml
    /// server:
    ///   transport: someip
    ///   service: motor_control
    ///   events:
    ///     "service.request.compute_force":
    ///       method: compute_force
    /// ```
    #[serde(default)]
    pub server: Option<ServerConfig>,
}

/// Server-side transport binding (SCE_MESH.md §13 Session E).
///
/// Mirrors [`BindingConfig`] shape but scoped to the server role: the
/// machine IS the target, not a client sending to one. Transport-specific
/// fields follow the same resolution pipeline as client bindings (SOME/IP
/// service names resolve against vsomeip.json, etc.).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerConfig {
    /// Transport type: "someip", "zenoh".
    pub transport: String,
    /// SOME/IP service name — resolved against vsomeip.json
    /// `services[*].name`, producing `service_id` + `instance_id`.
    #[serde(default)]
    pub service: Option<String>,
    /// Per-event binding table, keyed by SCXML event name. Same schema as
    /// [`BindingConfig::events`] — reuses [`EventBinding`] so the
    /// existing name-based resolution pipeline handles server events
    /// identically to client events.
    #[serde(default)]
    pub events: BTreeMap<String, EventBinding>,
    /// Zenoh key expression for queryable registration.
    #[serde(default)]
    pub key: Option<String>,
    /// Transport-native passthrough (e.g. `protocol: tcp`).
    #[serde(flatten)]
    pub extra: HashMap<String, serde_yaml_ng::Value>,
}

/// A single machine-lifetime subscription declaration (SCE_MESH.md §13).
///
/// Codegen emits subscribe on engine init and unsubscribe on engine
/// shutdown. The SCXML document is not touched — these subscriptions
/// exist only in deploy.yaml.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SubscriptionConfig {
    /// Event name to subscribe to (e.g. `event.notification.vehicle_speed`).
    /// Must match the `event.notification.*` or `event.subscribe.*` prefix
    /// convention; validated at topology resolution time.
    pub event: String,
    /// Source target that publishes the event (e.g. `"#chassis"`). Must
    /// have a matching entry in the machine's `bindings:` map so the
    /// transport layer knows which channel to subscribe on.
    pub source: String,
}

/// Per-event SOME/IP binding (SCE_MESH.md §14).
///
/// A binding declares one `EventBinding` per SCXML event routed to the
/// target, letting different events map to different methods or event
/// groups on the same target (e.g. `service.request.compute_force` vs
/// `service.request.release_force` both going to `#motor`). Each field
/// resolves against vsomeip.json at build time:
///
///   `method`      → `services[*].methods[*].name`      → `method_id`
///   `event_group` → `services[*].eventgroups[*].name`  → `event_group_id`
///                                                      + contained `event_id`
///   `getter`      → `services[*].methods[*].name`      → `getter_id`
///   `setter`      → `services[*].methods[*].name`      → `setter_id`
///
/// All fields are optional; which one is required depends on the event's
/// communication pattern (RPC → method, Notification → event_group, Field
/// access → getter/setter). Validation is done in topology, not here.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct EventBinding {
    /// Method name to resolve against vsomeip.json.
    #[serde(default)]
    pub method: Option<String>,
    /// Event group name to resolve against vsomeip.json.
    #[serde(default)]
    pub event_group: Option<String>,
    /// Field getter method name.
    #[serde(default)]
    pub getter: Option<String>,
    /// Field setter method name.
    #[serde(default)]
    pub setter: Option<String>,
}

impl EventBinding {
    pub fn is_empty(&self) -> bool {
        self.method.is_none()
            && self.event_group.is_none()
            && self.getter.is_none()
            && self.setter.is_none()
    }
}

/// Transport binding for a single `<send>` target.
///
/// Name-based fields (`service:`, `method:`, `event_group:`, `getter:`,
/// `setter:`, `events:`) reference entities in the external OEM config
/// (vsomeip.json); sce-build resolves them into numeric IDs at build
/// time (SCE_MESH.md §13, §14).
///
/// The per-event `events:` block is the canonical path — it lets different
/// SCXML events on the same target map to different methods or event groups.
/// The flat `method:` / `event_group:` / `getter:` / `setter:` fields at
/// binding level are sugar for "this one mapping applies to every event
/// on this target", useful for one-event-per-target deployments. `events:`
/// and the flat fields are mutually exclusive: declaring both is rejected
/// at resolution time so a reader cannot be confused about precedence.
///
/// `extra` uses `serde(flatten)` for per-target transport-native keys not
/// covered by the explicit fields (e.g. zenoh `key:`, someip `protocol:`,
/// shm arena/ring settings). The SOME/IP numeric-ID key names
/// (`service_id:`, `method_id:`, …) land in `extra` at parse time but are
/// reserved and rejected by the external-resolution stage
/// (`ExternalConfigError::ReservedSomeipIdKeys`) — numeric IDs come from
/// `transports.someip.config:` (vsomeip.json), referenced by name.
/// Device-shared session keys live on `DeviceConfig::transports`, not here.
#[derive(Debug, Clone, Deserialize)]
pub struct BindingConfig {
    /// Transport type: "local", "shm", "someip", "dds", "zenoh", etc.
    pub transport: String,

    /// SOME/IP service name — resolved against vsomeip.json
    /// `services[*].name`, producing `service_id` + `instance_id`. Applies
    /// at binding level because `service_id` / `instance_id` are per-target,
    /// not per-event.
    #[serde(default)]
    pub service: Option<String>,

    /// Sugar: single-method binding. Equivalent to an `events:` block where
    /// every SCXML event on this target uses the same method. Mutually
    /// exclusive with `events:`.
    #[serde(default)]
    pub method: Option<String>,

    /// Sugar: single event-group binding. Mutually exclusive with `events:`.
    #[serde(default)]
    pub event_group: Option<String>,

    /// Sugar: single getter. Mutually exclusive with `events:`.
    #[serde(default)]
    pub getter: Option<String>,

    /// Sugar: single setter. Mutually exclusive with `events:`.
    #[serde(default)]
    pub setter: Option<String>,

    /// Per-event binding table, keyed by SCXML event name (e.g.
    /// `"service.request.compute_force"`). `BTreeMap` gives deterministic
    /// codegen order. Empty iff the user relies on the flat sugar fields.
    #[serde(default)]
    pub events: BTreeMap<String, EventBinding>,

    /// Binding-level fallback for `<invoke type="sce:mesh-rpc">`
    /// deadline (SCE_MESH.md §9.5 precedence). Applied when a per-invoke
    /// `<param name="_mesh_deadline_ms">` is absent. A per-invoke value
    /// always overrides; if both are present with different values
    /// `sce-build` emits an informational notice (per-invoke override is
    /// expected usage). Absent here AND on the `<param>` ⇒ no deadline
    /// (the request can wait indefinitely for a reply).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline_ms: Option<u64>,

    /// Per-target transport-native settings passed through to templates
    /// (zenoh `key:`, someip `protocol:`, shm `shm_arena_bytes:`, etc.).
    /// Reserved SOME/IP ID key names are collected here at parse time but
    /// rejected by the external-resolution stage — see
    /// `ExternalConfigError::ReservedSomeipIdKeys`.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_yaml_ng::Value>,
}

impl BindingConfig {
    /// True iff any of the flat per-binding method/event_group/getter/setter
    /// fields is set (i.e. the single-event sugar path is in use).
    pub fn has_flat_event_fields(&self) -> bool {
        self.method.is_some()
            || self.event_group.is_some()
            || self.getter.is_some()
            || self.setter.is_some()
    }
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

    /// Find a machine name in deploy.yaml by matching the source filename.
    ///
    /// Fallback for when `model.name` (file stem) does not match the
    /// deploy.yaml key. This happens when the SCXML `name` attribute
    /// differs from the filename (e.g., `motor_someip_multi.scxml` has
    /// `<scxml name="motor">` and deploy.yaml uses `motor:` as the key).
    ///
    /// Returns the deploy.yaml machine key that has a `source:` ending
    /// with the given file stem + ".scxml". Returns `None` if no match.
    pub fn find_machine_name_by_source(&self, file_stem: &str) -> Option<String> {
        let source_suffix = format!("{file_stem}.scxml");
        for device in self.topology.values() {
            for (name, cfg) in &device.machines {
                if cfg.source == source_suffix || cfg.source.ends_with(&format!("/{source_suffix}")) {
                    return Some(name.clone());
                }
            }
        }
        None
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

    #[test]
    fn machine_subscriptions_parsed() {
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        subscriptions:
          - event: event.notification.vehicle_speed
            source: "#chassis"
          - event: event.notification.brake_pressure
            source: "#sensor"
      chassis: { source: chassis.scxml }
      sensor: { source: sensor.scxml }
"##;
        let cfg = parse_deploy_str(yaml).expect("parse");
        let brake = &cfg.topology["ecu1"].machines["brake"];
        assert_eq!(brake.subscriptions.len(), 2);
        assert_eq!(brake.subscriptions[0].event, "event.notification.vehicle_speed");
        assert_eq!(brake.subscriptions[0].source, "#chassis");
        assert_eq!(brake.subscriptions[1].event, "event.notification.brake_pressure");
        assert_eq!(brake.subscriptions[1].source, "#sensor");
    }

    #[test]
    fn machine_subscriptions_default_empty() {
        let yaml = r#"
version: "1.0"
topology:
  ecu1:
    machines:
      brake: { source: brake.scxml }
"#;
        let cfg = parse_deploy_str(yaml).expect("parse");
        let brake = &cfg.topology["ecu1"].machines["brake"];
        assert!(brake.subscriptions.is_empty());
    }
}
