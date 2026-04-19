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
    /// Reserved `discovery:` top-level key. Parsed as opaque `Value` so
    /// the parse-time validator can surface a spec-linked diagnostic
    /// instead of the generic `deny_unknown_fields` error. SCE Mesh §3.3
    /// is the invariant: transport-native routing is the source of truth
    /// for peer availability; SCE does not maintain a peer table, and
    /// the §2572 rejected list + §2574 rejection of `discovery.mode`
    /// both hold unconditionally. For per-binding runtime target
    /// selection use value-field placeholders (§14.4); for
    /// transport-level peer discovery configure external OEM config
    /// (zenoh.json5 scouting, vsomeip.json service-discovery).
    pub discovery: Option<serde_yaml_ng::Value>,
    /// Aggressive-distribution partition declarations (SCE_MESH.md §14
    /// "Partition resolution rules" + §16). A machine whose name does
    /// not appear in any partition's `contains:` runs monolithically on
    /// its device — the absence of a `partitions:` block is the normal
    /// single-process case. Absent ⇒ `None`; present ⇒ `Some(map)` with
    /// per-partition validation applied by [`parse_deploy_str`].
    ///
    /// The map type is [`PartitionMap`] rather than a raw `BTreeMap`
    /// because `serde_yaml_ng`'s typed map parse silently dedupes
    /// duplicate YAML keys (last-wins). `PartitionMap` installs a
    /// custom [`serde::Deserialize`] that rejects redeclarations via a
    /// sentinel-tagged error message — parse_deploy_str intercepts the
    /// sentinel and surfaces it as
    /// [`DeployError::PartitionDuplicateName`] (§14 rule 6).
    #[serde(default)]
    pub partitions: Option<PartitionMap>,
}

// SCE_MESH.md §14 rules 6-10 — partitions schema.
//
// The typed parse of a YAML mapping into `BTreeMap<String, T>` silently
// drops duplicate keys (last-wins). Rule 6 (partition names globally
// unique) therefore needs a dedicated detector; the other rules operate
// on the parsed [`PartitionDecl`] graph and live in their own
// validators below.

/// Sentinel token embedded in the custom serde error when
/// [`PartitionMap::deserialize`] observes a redeclaration. The token is
/// chosen to be unlikely to appear in authored text yet still visible
/// in the wrapped YAML error string, so [`parse_deploy_str`] can
/// promote the generic parse failure into the structured
/// [`DeployError::PartitionDuplicateName`] diagnostic.
const PARTITION_DUP_SENTINEL: &str = "__sce_partition_duplicate_name__";

/// Deserializer-backed `partitions:` map. Wraps a [`BTreeMap`] so
/// downstream validators can walk the entries in a deterministic order
/// (BTree ordering matches the source-free expectation of
/// CI-reproducible diagnostics).
#[derive(Debug, Clone, Default)]
pub struct PartitionMap(BTreeMap<String, PartitionDecl>);

impl PartitionMap {
    /// Iterate partitions in BTreeMap (lexicographic) order.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &PartitionDecl)> {
        self.0.iter()
    }

    /// Lookup a partition by name.
    pub fn get(&self, name: &str) -> Option<&PartitionDecl> {
        self.0.get(name)
    }

    /// True iff there are no partitions declared.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Partition count.
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl<'de> serde::Deserialize<'de> for PartitionMap {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct MapVisitor;
        impl<'de> serde::de::Visitor<'de> for MapVisitor {
            type Value = PartitionMap;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a mapping of partition name to PartitionDecl")
            }
            fn visit_map<A>(self, mut map: A) -> Result<PartitionMap, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut out: BTreeMap<String, PartitionDecl> = BTreeMap::new();
                while let Some(key) = map.next_key::<String>()? {
                    let value: PartitionDecl = map.next_value()?;
                    if out.insert(key.clone(), value).is_some() {
                        // Encode the collision as a serde custom error
                        // whose message embeds both the sentinel and the
                        // offending key. `parse_deploy_str` scans the
                        // wrapped YAML error for the sentinel and
                        // recovers the key verbatim.
                        return Err(<A::Error as serde::de::Error>::custom(format!(
                            "{PARTITION_DUP_SENTINEL}{key}"
                        )));
                    }
                }
                Ok(PartitionMap(out))
            }
        }
        deserializer.deserialize_map(MapVisitor)
    }
}

/// One partition entry under `partitions:`. A partition is the unit of
/// single-process execution for a machine's parallel regions and/or
/// invokes (SCE_MESH.md §14).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartitionDecl {
    /// Host device. Omitted ⇒ defaults to the first device declared in
    /// `topology:` at runtime resolution time (rule 7 still constrains
    /// the partition to one device).
    #[serde(default)]
    pub device: Option<String>,
    /// Machines whose pieces this partition hosts. Rule 9 requires
    /// every `contains:` entry to reference a machine in this list.
    pub machines: Vec<String>,
    /// Orthogonal units this partition runs.
    pub contains: PartitionContains,
    /// Transport used for inter-partition traffic within the same
    /// machine. Defaults handled at codegen time per SCE_MESH.md §14
    /// rule 4 (shm for single-device, custom_tcp otherwise).
    #[serde(default)]
    pub transport_binding: Option<String>,
    /// Per-partition parallel-final barrier timeout (SCE_MESH.md
    /// §16.5). `None` means "use the W3C normative default"
    /// (infinity). Only meaningful on partitions hosting the root of
    /// a `<parallel>`.
    #[serde(default)]
    pub barrier_timeout_ms: Option<u32>,
}

/// Orthogonal units assigned to a partition — parallel regions and
/// invokes, both of which are distribution axes per §16.3 + §14.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartitionContains {
    /// Child `<state>` IDs directly under a `<parallel>`.
    #[serde(default)]
    pub parallel_regions: Vec<PartitionUnitRef>,
    /// `<invoke>` IDs (including synthesized `__sce_synth_invoke__*`
    /// machines from §9.6.6).
    #[serde(default)]
    pub invokes: Vec<PartitionInvokeRef>,
}

/// A parallel-region unit reference — the (machine, region) pair is
/// the §14 rule 8 uniqueness key.
#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Hash)]
#[serde(deny_unknown_fields)]
pub struct PartitionUnitRef {
    /// SCXML machine name (deploy.yaml `machines.<name>` key).
    pub machine: String,
    /// `<state id>` of the region (direct child of `<parallel>`).
    pub region: String,
}

/// An invoke unit reference — the (machine, invoke) pair is the §14
/// rule 8 uniqueness key.
#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Hash)]
#[serde(deny_unknown_fields)]
pub struct PartitionInvokeRef {
    /// SCXML machine name hosting the invoke site.
    pub machine: String,
    /// `<invoke id>` of the invoke. May be a synthesized
    /// `<parent>__sce_synth_invoke__<id>` identifier per §9.6.6.
    pub invoke: String,
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
    /// custom_tcp device-shared listen endpoint (SCE_MESH.md §16.8.3).
    /// One TCP server per device on `127.0.0.1:<port>`; each binding's
    /// per-target `connect:` reaches another device's server. Omit for
    /// devices that only initiate connections.
    pub custom_tcp: Option<CustomTcpTransportConfig>,
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

/// custom_tcp device-shared listen endpoint (SCE_MESH.md §16.8.3).
///
/// IPv4 loopback only; the harness reference transport is local-only
/// by design. `listen:` is omitted when the device only acts as a TCP
/// client. `deny_unknown_fields` rejects typos like `lsten:` at parse
/// time so a missing server is surfaced as a deploy.yaml error rather
/// than as silent connection refusal at runtime.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CustomTcpTransportConfig {
    /// Server bind address in `host:port` form (e.g. `"127.0.0.1:9000"`).
    /// Codegen passes this verbatim to the generated server's
    /// `bind()` call. Hostnames other than `127.0.0.1` are accepted by
    /// the schema but the reference transport is documented as
    /// loopback-only; non-loopback hosts are an authoring concern, not
    /// a build-time validation.
    pub listen: Option<String>,
}

impl CustomTcpTransportConfig {
    /// True iff this device hosts a TCP listen socket. Single source
    /// for two converging gates: lib.rs's "pure-receiver still needs
    /// transport.h" early-return override and codegen.rs's "emit
    /// server field even with no client targets" template-context
    /// adjustment. Both call sites must agree, so the predicate lives
    /// here rather than being duplicated at each gate.
    pub fn hosts_server(&self) -> bool {
        self.listen.is_some()
    }
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

/// Ordering guarantee declared on a per-binding basis (SCE_MESH.md §10.6).
///
/// - `None` (default): the receiver sees envelopes in arrival order. This is
///   correct for transports that natively preserve per-sender FIFO (local,
///   shm, custom_tcp, SOME/IP over TCP) AND for authors who do not depend on
///   order across a UDP-backed route.
/// - `Required`: the route guarantees per-sender FIFO, either via the
///   transport's native order or via the runtime `OrderingBuffer` emitted
///   by the codegen. Topology rejects this value on transports whose
///   `ordering_representable` is `false` (CAN).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderingRequirement {
    #[default]
    None,
    Required,
}

impl OrderingRequirement {
    /// `true` when no runtime ordering action is requested. Used by the
    /// codegen and by `skip_serializing_if` so the default serializes out
    /// cleanly.
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

/// Default `gap_timeout_ms` applied when a machine omits the `ordering:`
/// section (SCE_MESH.md §10.6.1). 100 ms covers the Zenoh session-refresh
/// window and the SOME/IP retransmit envelope at 1 kHz sender rates. This
/// constant is the single source of truth — the C++ runtime no longer
/// hard-codes a fallback; every emitted router carries an explicit value.
pub const DEFAULT_GAP_TIMEOUT_MS: u64 = 100;

/// Default `tick_period_ms` applied when a machine omits the `ordering:`
/// section (SCE_MESH.md §10.6.1). One half of [`DEFAULT_GAP_TIMEOUT_MS`]
/// (Nyquist) so worst-case gap recovery latency is bounded by
/// `gap_timeout + tick_period`.
pub const DEFAULT_TICK_PERIOD_MS: u64 = 50;

/// Per-machine ordering buffer timings (SCE_MESH.md §10.6.1).
///
/// Both fields are required when the `ordering:` section is present —
/// no field-level default. Authors who want one knob accept both
/// [`DEFAULT_GAP_TIMEOUT_MS`] / [`DEFAULT_TICK_PERIOD_MS`] by omitting
/// the section entirely; partial overrides are rejected at parse time
/// to keep the deploy.yaml ↔ runtime mapping unambiguous.
///
/// The `ordering:` section configures HOW the per-machine ordering
/// buffer behaves (timing constants); it is independent of any
/// per-binding `ordering: required` declaration which controls
/// WHETHER the buffer activates for a given route.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OrderingTimings {
    /// Worst-case wait before fast-forwarding past a missing sequence
    /// number. SCE_MESH.md §10.6.4 — the receiver buffer drains contiguous
    /// envelopes and emits `ORDERING_GAP` on timeout.
    pub gap_timeout_ms: u64,
    /// Cadence at which the generated router drives `OrderingBuffer::tick`
    /// (SCE_MESH.md §10.6.4). Must be strictly less than
    /// [`Self::gap_timeout_ms`] so a single missed sequence is detected
    /// within `gap_timeout + tick_period`.
    pub tick_period_ms: u64,
}

impl OrderingTimings {
    /// Default timings applied when a machine omits the `ordering:` section.
    pub const fn default_const() -> Self {
        Self {
            gap_timeout_ms: DEFAULT_GAP_TIMEOUT_MS,
            tick_period_ms: DEFAULT_TICK_PERIOD_MS,
        }
    }

    /// Validate constraints common to every machine that declares an
    /// `ordering:` section. Returns the rejection reason without the
    /// machine name — the caller wraps this into
    /// [`DeployError::InvalidOrderingTimings`].
    fn validation_error(&self) -> Option<String> {
        if self.gap_timeout_ms == 0 {
            return Some("gap_timeout_ms must be greater than zero".to_string());
        }
        if self.tick_period_ms == 0 {
            return Some("tick_period_ms must be greater than zero".to_string());
        }
        if self.tick_period_ms >= self.gap_timeout_ms {
            return Some(format!(
                "tick_period_ms ({}) must be strictly less than gap_timeout_ms ({}) \
                 so a missed sequence is detected within `gap_timeout + tick_period`",
                self.tick_period_ms, self.gap_timeout_ms,
            ));
        }
        None
    }
}

impl Default for OrderingTimings {
    fn default() -> Self {
        Self::default_const()
    }
}

/// Minimum `lease_ms` accepted in a `liveliness:` section.
///
/// SCE Mesh §16.7 row 8 (`PEER_PARTITIONED`) couples peer-failure
/// detection latency to Zenoh's own keepalive cadence. Values below
/// this floor race the router's own internal heartbeat and generate
/// spurious DELETE/PUT churn, so parse-time rejection is preferred
/// over runtime misbehaviour. Matches the Nyquist-style floor the
/// plan memo locked.
pub const MIN_LIVELINESS_LEASE_MS: u64 = 100;

/// Minimum `query_timeout_ms` accepted in a `server:` section.
///
/// SCE Mesh §9.5 Zenoh server queryable timeout (gap Z2): values
/// below this floor are almost certainly typos — even a trivial
/// engine macrostep usually takes longer than 10 ms, so a sub-floor
/// value would cause every inbound query to time out before the
/// engine can respond. Parse-time rejection surfaces the mistake
/// at the offending deploy.yaml line rather than a silent runtime
/// cleanup cascade.
pub const MIN_SERVER_QUERY_TIMEOUT_MS: u64 = 10;

/// Per-machine Zenoh liveliness configuration (SCE Mesh §16.7 row 8).
///
/// Opt-in: absent section ⇒ no liveliness token declared, no
/// subscriber installed, zero generated code — matches the
/// [`OrderingTimings`] convention so authors pay only for what they
/// declare. Section present ⇒ the field is required and validated
/// (`lease_ms >= MIN_LIVELINESS_LEASE_MS`) at parse time so a bad
/// value cannot reach the generated router.
///
/// `lease_ms` is the keepalive cadence negotiated with the Zenoh
/// router — the application-side bound on DELETE-sample latency
/// when a peer drops. SCXML authors who need peer-failure detection
/// to raise `error.communication` (reason `PEER_PARTITIONED`)
/// within a bounded window declare this section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivelinessConfig {
    /// Keepalive cadence in milliseconds. Must be `>= MIN_LIVELINESS_LEASE_MS`.
    /// Worst-case PEER_PARTITIONED latency is `lease_ms` plus a small
    /// Zenoh-internal jitter (typically <50 ms on loopback).
    pub lease_ms: u64,
}

impl LivelinessConfig {
    /// Validate the constraint. Returns the rejection reason without
    /// the machine name — the caller wraps this into
    /// [`DeployError::InvalidLiveliness`].
    fn validation_error(&self) -> Option<String> {
        if self.lease_ms < MIN_LIVELINESS_LEASE_MS {
            return Some(format!(
                "lease_ms ({}) must be >= {} ms — values below this floor race \
                 Zenoh's own keepalive and generate spurious DELETE/PUT churn",
                self.lease_ms, MIN_LIVELINESS_LEASE_MS,
            ));
        }
        None
    }
}

/// Minimum `max_pending_per_target` accepted in an `outbound_buffer:`
/// section.
///
/// SCE Mesh §10.10 (`OutboundBuffer`): a buffer with capacity zero is
/// semantically equivalent to the pre-§10.10 "silently drop if not
/// ready" behaviour — it cannot hold anything. Rejecting zero at parse
/// time surfaces the mistake at the offending deploy.yaml line rather
/// than generating a router that compiles but cannot honour the §10.7
/// contract. Values of one or above are accepted regardless of
/// perceived "too small" judgement: a single-slot buffer is a
/// legitimate test-harness shape (one in-flight envelope during
/// readiness gating).
pub const MIN_OUTBOUND_BUFFER_MAX_PENDING: u32 = 1;

/// Per-machine outbound readiness-gated buffer (SCE Mesh §10.10).
///
/// Opt-in: absent section ⇒ no buffer emitted; every outbound send
/// goes straight to the transport and any pre-readiness send is
/// silently lost per the pre-§10.10 behaviour. Section present ⇒ the
/// generated router declares an [`OutboundBuffer`] per opt-in target
/// (SOME/IP targets and Zenoh PUT-pattern targets), wires the
/// transport's readiness callback, and drains buffered envelopes on
/// the 0→1 ready transition.
///
/// One knob today: `max_pending_per_target`. Other overflow policies
/// (`drop_oldest`, `max_age_ms`) are deferred — no consumer has
/// requested them yet (`feedback_verify_before_ship.md`). Validation
/// rejects `max_pending_per_target == 0` at parse time; see
/// [`MIN_OUTBOUND_BUFFER_MAX_PENDING`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OutboundBufferConfig {
    /// Maximum number of envelopes buffered per opt-in target before
    /// overflow raises `error.communication` with reason
    /// `BACKPRESSURE_DROP` (§16.7 row 9) and drops the newest. Must
    /// be `>= MIN_OUTBOUND_BUFFER_MAX_PENDING`.
    pub max_pending_per_target: u32,
}

impl OutboundBufferConfig {
    /// Validate the constraint. Returns the rejection reason without
    /// the machine name — the caller wraps this into
    /// [`DeployError::InvalidOutboundBuffer`].
    fn validation_error(&self) -> Option<String> {
        if self.max_pending_per_target < MIN_OUTBOUND_BUFFER_MAX_PENDING {
            return Some(format!(
                "max_pending_per_target ({}) must be >= {} — a zero-capacity \
                 buffer cannot hold any envelope, which is indistinguishable \
                 from the pre-§10.10 silent-drop behaviour; omit the section \
                 entirely to opt out of buffering instead",
                self.max_pending_per_target, MIN_OUTBOUND_BUFFER_MAX_PENDING,
            ));
        }
        None
    }
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
    /// Per-machine ordering buffer timings (SCE_MESH.md §10.6.1). Absent
    /// section ⇒ [`OrderingTimings::default_const`] (100 ms /
    /// 50 ms). Section present ⇒ both fields are required and validated
    /// (positive, Nyquist) at parse time. The values are emitted directly
    /// into the generated router; no fallback exists below the deploy
    /// layer.
    #[serde(default)]
    pub ordering: Option<OrderingTimings>,
    /// Per-machine Zenoh liveliness configuration (SCE Mesh §16.7 row 8).
    /// Absent section ⇒ no liveliness token declared and no subscriber
    /// installed; the generated router emits zero liveliness code.
    /// Section present ⇒ `lease_ms` is required and validated at parse
    /// time. Opt-in by design — see [`LivelinessConfig`].
    #[serde(default)]
    pub liveliness: Option<LivelinessConfig>,
    /// Per-machine outbound buffer for readiness-gated send paths
    /// (SCE Mesh §10.10). Absent section ⇒ no buffer emitted; outbound
    /// sends go straight to the transport and any pre-readiness send
    /// is silently lost (SOME/IP before `offer_service`, Zenoh PUT
    /// before any subscriber declares). Section present ⇒ opt-in
    /// targets route through `OutboundBuffer::admit`, the transport's
    /// native readiness primitive feeds `markReady` / `markNotReady`,
    /// and overflow raises `error.communication` with reason
    /// `BACKPRESSURE_DROP` (§16.7 row 9). See [`OutboundBufferConfig`].
    #[serde(default)]
    pub outbound_buffer: Option<OutboundBufferConfig>,
}

impl MachineConfig {
    /// Resolve the machine's ordering timings, filling defaults when the
    /// `ordering:` section is absent. Single source for downstream
    /// consumers (codegen, lib.rs) so the absent-section default lives
    /// in exactly one place.
    pub fn resolved_ordering_timings(&self) -> OrderingTimings {
        self.ordering.unwrap_or_else(OrderingTimings::default_const)
    }
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
    /// SCE Mesh §14.4 — server-side multi-instance pool.
    ///
    /// Accepted on transports whose registry entry sets
    /// [`crate::mesh::transport::TransportDescriptor::supports_multi_instance_server`]
    /// to `true` (SOME/IP today). For accepted transports the generated
    /// TransportRouter offers one instance per listed ID at `init()`
    /// and registers per-instance message handlers so inbound requests
    /// carry a peer-identifying `instance_id` at dispatch time. For
    /// non-supporting transports, declaring this field is a build-time
    /// hard error ([`DeployError::ServerPoolNotSupported`]); the
    /// silently-broken alternative — the field sinking into `extra`
    /// and vanishing — is impossible.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instances: Option<Vec<u16>>,

    /// Per-server Zenoh queryable response deadline (SCE Mesh §9.5, gap
    /// Z2).
    ///
    /// **Zenoh-only scope**: SOME/IP (and other non-zenoh) server-side
    /// response lifecycles use distinct transport-native state
    /// (`pending_server_requests_` for vsomeip), not the
    /// `pending_server_queries_` map that this knob targets. Parse-time
    /// validation rejects the knob on non-zenoh servers so a SOME/IP
    /// author cannot inadvertently ship a silent no-op. A future
    /// SOME/IP equivalent will land under its own gap memo
    /// (`mesh_someip_sd_gaps_roadmap.md`) with its own knob.
    ///
    /// Absent ⇒ no deadline armed per inbound query, matching the
    /// pre-Z2 behaviour where `pending_server_queries_` leaks any entry
    /// whose engine never emits the paired response. Present ⇒ each
    /// inbound query arms a scheduler entry that, on expiry, silently
    /// erases the stored `zenoh::Query`; the destructor lets the client
    /// observe the drop via the Z3 on_drop path
    /// (`RpcStatus::Unavailable`), so no new `error.communication` row
    /// is introduced. Validated at parse time
    /// (`query_timeout_ms >= MIN_SERVER_QUERY_TIMEOUT_MS` AND
    /// `transport == "zenoh"`).
    #[serde(default)]
    pub query_timeout_ms: Option<u64>,
    /// Transport-native passthrough (e.g. `protocol: tcp`).
    #[serde(flatten)]
    pub extra: HashMap<String, serde_yaml_ng::Value>,
}

impl ServerConfig {
    /// Validate the `query_timeout_ms` field. Returns the rejection
    /// reason without the machine name — the caller wraps this into
    /// [`DeployError::InvalidServerQueryTimeout`].
    ///
    /// Two rejection paths:
    ///   1. Value below [`MIN_SERVER_QUERY_TIMEOUT_MS`] — would race
    ///      engine macrostep latency and cause every query to time
    ///      out before a response is possible.
    ///   2. Non-zenoh transport — Z2 wires the scheduler to the
    ///      zenoh-specific `pending_server_queries_` map, so the knob
    ///      is a silent no-op on SOME/IP and other transports today.
    ///      Rejecting at parse time surfaces the mistake before it
    ///      reaches the generated router.
    fn query_timeout_validation_error(&self) -> Option<String> {
        let Some(ms) = self.query_timeout_ms else {
            return None;
        };
        if ms < MIN_SERVER_QUERY_TIMEOUT_MS {
            return Some(format!(
                "query_timeout_ms ({}) must be >= {} ms — values below this \
                 floor race typical engine macrostep latency and would cause \
                 every inbound query to time out before the engine can respond",
                ms, MIN_SERVER_QUERY_TIMEOUT_MS,
            ));
        }
        if self.transport != "zenoh" {
            return Some(format!(
                "query_timeout_ms is currently supported only on Zenoh servers \
                 (transport: zenoh); this server declares `transport: {}`. \
                 SOME/IP and other server-side response lifecycles are tracked \
                 separately (SCE Mesh §9.5 gap Z2 does not cover them yet). \
                 Remove the knob, or switch the server transport to zenoh",
                self.transport,
            ));
        }
        None
    }
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

    /// Per-binding ordering declaration (SCE_MESH.md §10.6). Default
    /// `None` keeps the legacy "engine sees arrival order" behavior;
    /// `Required` activates the runtime `OrderingBuffer` for transports
    /// whose `supplies_ordering` is `false`, and is a topology error for
    /// transports whose `ordering_representable` is `false` (CAN).
    #[serde(default)]
    pub ordering: OrderingRequirement,

    /// SCE Mesh §14.4 — bounded instance pool for a SOME/IP binding that
    /// carries a placeholder. vsomeip's
    /// `request_service(SERVICE, ANY_INSTANCE)` does not actually
    /// subscribe to every instance (treated as specific 0xFFFF), so
    /// codegen must emit one `request_service(SERVICE, i)` per declared
    /// instance at init(). Runtime placeholder values outside this list
    /// raise `error.invoke.<id>` with `RpcStatus::Unavailable`. Required
    /// alongside [`Self::instance_from`]; omitted for bindings without
    /// placeholders.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instances: Option<Vec<u16>>,

    /// SCE Mesh §14.4 — names the `<param>` whose runtime value feeds
    /// `message->set_instance(...)` on a SOME/IP pool binding. Unified
    /// with the Zenoh `{name}` KeyExpr mechanism: both describe "the
    /// binding references an author-named `<param>`". SOME/IP uses an
    /// explicit field (rather than a `{name}` embed) because the
    /// `instance_id` is a typed `uint16_t`, not a string. Required
    /// alongside [`Self::instances`] for SOME/IP pool bindings; rejected
    /// at parse time on non-SOME/IP transports (no set_instance() API)
    /// and when the binding also embeds a `{name}` placeholder (the two
    /// mechanisms are mutually exclusive per binding).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_from: Option<String>,

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
    let cfg: DeployConfig = serde_yaml_ng::from_str(content).map_err(|e| {
        let msg = e.to_string();
        // Promote the sentinel-tagged custom error emitted by
        // `PartitionMap::deserialize` into a structured diagnostic.
        // serde_yaml_ng wraps our `custom(...)` message with a YAML
        // location prefix, so the sentinel survives as a substring.
        if let Some(start) = msg.find(PARTITION_DUP_SENTINEL) {
            let after = &msg[start + PARTITION_DUP_SENTINEL.len()..];
            // Extract the key verbatim: YAML keys are arbitrary strings
            // but our sentinel was emitted at the tail of the message,
            // so read until the first whitespace, quote, or message
            // punctuation inserted by serde_yaml's wrapping layer.
            let name: String = after
                .chars()
                .take_while(|c| !c.is_whitespace() && *c != '"' && *c != ',')
                .collect();
            return DeployError::PartitionDuplicateName { name };
        }
        DeployError::Yaml(msg)
    })?;

    if let Some(v) = &cfg.version {
        if !SUPPORTED_VERSIONS.contains(&v.as_str()) {
            return Err(DeployError::UnsupportedVersion {
                found: v.clone(),
                supported: SUPPORTED_VERSIONS.to_vec(),
            });
        }
    }

    validate_machine_name_uniqueness(&cfg)?;
    validate_ordering_timings(&cfg)?;
    validate_liveliness(&cfg)?;
    validate_server_query_timeout(&cfg)?;
    validate_server_pool_rejection(&cfg)?;
    validate_pool_capability(&cfg)?;
    validate_outbound_buffer(&cfg)?;
    validate_discovery_not_supported(&cfg)?;
    validate_partitions_schema(&cfg)?;

    Ok(cfg)
}

/// SCE_MESH.md §14 rules 7-10 — structural checks on `partitions:`
/// that do not require SCXML cross-reference. Rule 6 (duplicate
/// partition names) is enforced at deserialization time via the
/// custom [`PartitionMap`] visitor; rules 1, 2, 5, 11 (coverage,
/// default-partition discipline, synthesized-invoke infix collision,
/// nested-parallel partitioning) require SCXML inspection and land
/// in a later phase. This validator is a no-op when `partitions:` is
/// absent.
fn validate_partitions_schema(cfg: &DeployConfig) -> Result<(), DeployError> {
    let Some(partitions) = &cfg.partitions else {
        return Ok(());
    };

    // Build a device lookup: machine_name → device_name. Device names
    // themselves live as HashMap keys in cfg.topology; the lookup is
    // only needed for rule 7 (multi-device detection).
    let mut machine_device: BTreeMap<&str, &str> = BTreeMap::new();
    for (device_name, device) in &cfg.topology {
        for machine_name in device.machines.keys() {
            machine_device.insert(machine_name.as_str(), device_name.as_str());
        }
    }

    // Rules 7, 9, 10 operate per-partition. Collect the (unit, partition)
    // pairs for rule 8 across all partitions so two partitions claiming
    // the same unit produce one diagnostic with both partition names.
    let mut unit_owners: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for (partition_name, decl) in partitions.iter() {
        // Rule 10 — empty partition. Checked first so it pre-empts the
        // rule-9 check (which would read no entries and pass vacuously).
        if decl.contains.parallel_regions.is_empty() && decl.contains.invokes.is_empty() {
            return Err(DeployError::PartitionEmpty {
                partition: partition_name.clone(),
            });
        }

        // Rule 7 — single device per partition. Resolve every machine in
        // `machines:` to its host device; if the set has cardinality > 1
        // the partition would span devices. Unknown machines are not
        // rejected here — topology-stage validation catches those with
        // a more precise diagnostic.
        let mut devices: BTreeMap<&str, ()> = BTreeMap::new();
        for machine in &decl.machines {
            if let Some(device) = machine_device.get(machine.as_str()) {
                devices.insert(*device, ());
            }
        }
        if devices.len() > 1 {
            let device_list: Vec<String> = devices.keys().map(|s| s.to_string()).collect();
            return Err(DeployError::PartitionMultiDevice {
                partition: partition_name.clone(),
                devices: device_list,
            });
        }

        // Rule 9 — every `contains:` entry must reference a machine in
        // the partition's own `machines:` list. Using a BTreeSet so
        // membership checks are O(log n) without allocation per
        // contained entry.
        let listed: std::collections::BTreeSet<&str> =
            decl.machines.iter().map(|s| s.as_str()).collect();
        for region in &decl.contains.parallel_regions {
            if !listed.contains(region.machine.as_str()) {
                return Err(DeployError::PartitionMachineNotListed {
                    partition: partition_name.clone(),
                    machine: region.machine.clone(),
                });
            }
            let key = format!("parallel_region:{}/{}", region.machine, region.region);
            unit_owners
                .entry(key)
                .or_default()
                .push(partition_name.clone());
        }
        for invoke in &decl.contains.invokes {
            if !listed.contains(invoke.machine.as_str()) {
                return Err(DeployError::PartitionMachineNotListed {
                    partition: partition_name.clone(),
                    machine: invoke.machine.clone(),
                });
            }
            let key = format!("invoke:{}/{}", invoke.machine, invoke.invoke);
            unit_owners
                .entry(key)
                .or_default()
                .push(partition_name.clone());
        }
    }

    // Rule 8 — each unit belongs to exactly one partition. Report the
    // first unit observed in more than one partition with all owners
    // named so authors fix the collision in one edit.
    for (unit, owners) in &unit_owners {
        if owners.len() > 1 {
            return Err(DeployError::PartitionUnitDuplicate {
                unit: unit.clone(),
                partitions: owners.clone(),
            });
        }
    }

    Ok(())
}

/// Reject any `discovery:` top-level block (SCE Mesh §3.3 + §2572 +
/// §2574). Parsed as opaque [`serde_yaml_ng::Value`] so an authored
/// `discovery:` key lands here rather than triggering the generic
/// `deny_unknown_fields` message; the validator produces a spec-linked
/// diagnostic that names the replacement mechanisms (§14.4 binding
/// value-field placeholders for per-binding runtime target selection,
/// external OEM config for transport-level peer discovery). `null` /
/// absent discovery values deserialise as `None` and pass through.
fn validate_discovery_not_supported(cfg: &DeployConfig) -> Result<(), DeployError> {
    let Some(value) = &cfg.discovery else {
        return Ok(());
    };
    Err(DeployError::DiscoveryNotSupported {
        content_kind: summarize_discovery_content(value),
    })
}

/// Render a short, deterministic description of the rejected
/// `discovery:` content. Used in both the `thiserror` message (so the
/// author sees what was rejected) and the diagnostic `key_fragments`
/// (so two different authored shapes get distinct fnv1a ids).
fn summarize_discovery_content(value: &serde_yaml_ng::Value) -> String {
    use serde_yaml_ng::Value;
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(_) => "scalar bool".to_string(),
        Value::Number(_) => "scalar number".to_string(),
        Value::String(_) => "scalar string".to_string(),
        Value::Sequence(_) => "sequence".to_string(),
        Value::Mapping(map) if map.is_empty() => "empty object".to_string(),
        Value::Mapping(map) => {
            let mut keys: Vec<String> = map
                .keys()
                .map(|k| match k {
                    Value::String(s) => s.clone(),
                    other => format!("{other:?}"),
                })
                .collect();
            keys.sort();
            format!("object with keys [{}]", keys.join(", "))
        }
        Value::Tagged(_) => "tagged value".to_string(),
    }
}

/// Walk every machine that declared an explicit `ordering:` section and
/// reject zero/Nyquist violations. Runs at parse time so the diagnostic
/// surfaces the offending deploy.yaml line rather than a deferred
/// runtime mis-tick.
fn validate_ordering_timings(cfg: &DeployConfig) -> Result<(), DeployError> {
    use std::collections::BTreeMap;
    // Walk in deterministic order (sorted by machine name) so the first
    // reported violation is stable across runs even though the
    // underlying topology map is a HashMap.
    let mut by_machine: BTreeMap<&str, &OrderingTimings> = BTreeMap::new();
    for device in cfg.topology.values() {
        for (machine_name, machine) in &device.machines {
            if let Some(t) = &machine.ordering {
                by_machine.insert(machine_name.as_str(), t);
            }
        }
    }
    for (machine, timings) in by_machine {
        if let Some(reason) = timings.validation_error() {
            return Err(DeployError::InvalidOrderingTimings {
                machine: machine.to_string(),
                reason,
            });
        }
    }
    Ok(())
}

/// Walk every machine that declared an explicit `liveliness:` section
/// and reject values below [`MIN_LIVELINESS_LEASE_MS`]. Runs at parse
/// time so the diagnostic surfaces the offending deploy.yaml line
/// rather than a deferred runtime misbehaviour.
fn validate_liveliness(cfg: &DeployConfig) -> Result<(), DeployError> {
    use std::collections::BTreeMap;
    let mut by_machine: BTreeMap<&str, &LivelinessConfig> = BTreeMap::new();
    for device in cfg.topology.values() {
        for (machine_name, machine) in &device.machines {
            if let Some(l) = &machine.liveliness {
                by_machine.insert(machine_name.as_str(), l);
            }
        }
    }
    for (machine, liveliness) in by_machine {
        if let Some(reason) = liveliness.validation_error() {
            return Err(DeployError::InvalidLiveliness {
                machine: machine.to_string(),
                reason,
            });
        }
    }
    Ok(())
}

// ── SCE Mesh §14.4 binding pool support ─────────────────────
//
// A binding value field may carry `{name}` tokens substituted at runtime
// from `<send>`/`<invoke>` `<param>` values. Detection is textual and
// transport-agnostic: any string value in `extra` is scanned. SOME/IP
// pool bindings additionally require an explicit `instances:` list
// because vsomeip's `request_service(SERVICE, ANY_INSTANCE)` does not
// subscribe to every instance.

/// Extract every `{name}` placeholder from a string value. Names are
/// `[A-Za-z_][A-Za-z0-9_]*`. Malformed braces (unbalanced, empty name,
/// invalid characters inside) return `Err` so the caller surfaces them
/// as a build-time diagnostic rather than silently accepting them.
pub(crate) fn extract_placeholders(s: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' {
            // Find the matching `}`.
            let start = i + 1;
            let mut end = start;
            while end < bytes.len() && bytes[end] != b'}' {
                end += 1;
            }
            if end >= bytes.len() {
                return Err(format!(
                    "unbalanced '{{' at byte {i} — every '{{' must have a matching '}}' \
                     within the same value"
                ));
            }
            if end == start {
                return Err(format!(
                    "empty placeholder `{{}}` at byte {i} — placeholder name cannot be empty"
                ));
            }
            let name = &s[start..end];
            let first = name.chars().next().unwrap();
            if !(first.is_ascii_alphabetic() || first == '_') {
                return Err(format!(
                    "placeholder '{{{name}}}' at byte {i} — name must start with an \
                     ASCII letter or underscore"
                ));
            }
            if !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
            {
                return Err(format!(
                    "placeholder '{{{name}}}' at byte {i} — name may only contain ASCII \
                     letters, digits, and underscores"
                ));
            }
            out.push(name.to_string());
            i = end + 1;
        } else if bytes[i] == b'}' {
            return Err(format!(
                "unmatched '}}' at byte {i} — every '}}' must be paired with an earlier '{{'"
            ));
        } else {
            i += 1;
        }
    }
    Ok(out)
}

/// True iff the binding carries any `{name}` placeholder in a string
/// `extra` value. Parse errors from [`extract_placeholders`] are also
/// signalled here — the caller surfaces them through the dedicated
/// diagnostic so an invalid placeholder never silently degrades to "no
/// pool behaviour".
fn binding_placeholder_names(
    binding: &BindingConfig,
) -> Result<Vec<String>, String> {
    let mut names = Vec::new();
    for value in binding.extra.values() {
        if let serde_yaml_ng::Value::String(s) = value {
            let placeholders = extract_placeholders(s)?;
            for name in placeholders {
                if !names.contains(&name) {
                    names.push(name);
                }
            }
        }
    }
    Ok(names)
}

/// SCE_MESH.md §14.4 — server-side multi-instance pool gating.
///
/// A machine declaring `server.instances:` is accepted iff its server
/// transport's registry flag `supports_multi_instance_server` is `true`
/// (SOME/IP today; see transport.rs for the exhaustive list). Any
/// other transport's entry rejects at parse time with the transport
/// name in the diagnostic, so an author who picks the wrong transport
/// for a pooled server learns at `deploy.yaml` parse rather than via a
/// silent codegen divergence. Unknown transports fall through to the
/// reject branch — an unregistered transport cannot promise the
/// peer-identity semantic the pool relies on.
fn validate_server_pool_rejection(cfg: &DeployConfig) -> Result<(), DeployError> {
    use std::collections::BTreeMap;
    // Deterministic sorted scan so the first reported violation is stable
    // across runs even though `topology` is a HashMap.
    let mut by_machine: BTreeMap<&str, &ServerConfig> = BTreeMap::new();
    for device in cfg.topology.values() {
        for (machine_name, machine) in &device.machines {
            if let Some(server) = &machine.server {
                by_machine.insert(machine_name.as_str(), server);
            }
        }
    }
    for (machine_name, server) in by_machine {
        if server.instances.is_none() {
            continue;
        }
        let supported = super::transport::lookup(&server.transport)
            .map(|d| d.supports_multi_instance_server)
            .unwrap_or(false);
        if !supported {
            return Err(DeployError::ServerPoolNotSupported {
                machine: machine_name.to_string(),
                transport: server.transport.clone(),
            });
        }
    }
    Ok(())
}

/// SCE_MESH.md §14.4 — a binding requesting a runtime pool may only
/// target a transport whose
/// [`crate::mesh::transport::TransportDescriptor::supports_pool`] is
/// `true`. A pool request is expressed via one of two transport-specific
/// mechanisms:
///   - **Zenoh**: `{name}` placeholder embedded in `key:`.
///   - **SOME/IP**: `instance_from: <param-name>` binding field, paired
///     with an explicit `instances:` list (vsomeip's `ANY_INSTANCE` is
///     not a wildcard; the finite instance set must be declared so
///     codegen can emit one `request_service` per member at `init()`).
///
/// The two mechanisms are mutually exclusive per binding — the author
/// chooses the mechanism by transport, not by field-packing style.
/// Bindings without pool requests pass through (pool support is purely
/// additive).
///
/// **Precondition.** This validator assumes [`validate_machine_name_uniqueness`]
/// has already run and succeeded. The internal `by_machine` map
/// collapses duplicate machine names silently (last-writer-wins on
/// `BTreeMap::insert`); without upstream uniqueness enforcement, the
/// diagnostic from this validator could point at the wrong device's
/// copy of a duplicate machine. The `debug_assert_eq!` below pins the
/// precondition explicitly — a debug build that reorders
/// `parse_deploy_str`'s validator chain will trip this assertion rather
/// than ship a misleading diagnostic.
fn validate_pool_capability(cfg: &DeployConfig) -> Result<(), DeployError> {
    use std::collections::BTreeMap;
    let mut by_machine: BTreeMap<&str, &MachineConfig> = BTreeMap::new();
    for device in cfg.topology.values() {
        for (machine_name, machine) in &device.machines {
            by_machine.insert(machine_name.as_str(), machine);
        }
    }
    // Hack-5 guard: the BTreeMap collapses duplicates silently. If the
    // caller pipeline forgot to run machine-name uniqueness first, the
    // collapsed count would be strictly less than the raw declaration
    // count and every downstream diagnostic in this function would
    // read from the last-seen copy instead of the real duplicate.
    let total_declarations: usize =
        cfg.topology.values().map(|d| d.machines.len()).sum();
    debug_assert_eq!(
        by_machine.len(),
        total_declarations,
        "validate_pool_capability was called before validate_machine_name_uniqueness: \
         {} machine declarations collapsed to {} unique names. Reorder parse_deploy_str \
         so duplicate-detection precedes pool validation.",
        total_declarations,
        by_machine.len(),
    );
    for (machine_name, machine) in by_machine {
        let mut sorted_bindings: Vec<(&TargetId, &BindingConfig)> =
            machine.bindings.iter().collect();
        sorted_bindings.sort_by_key(|(k, _)| k.as_str());
        for (binding_key, binding) in sorted_bindings {
            let placeholders = binding_placeholder_names(binding).map_err(|reason| {
                DeployError::PoolInvalidPlaceholder {
                    machine: machine_name.to_string(),
                    binding: binding_key.as_str().to_string(),
                    reason,
                }
            })?;
            let has_placeholder = !placeholders.is_empty();
            let has_instance_from = binding.instance_from.is_some();

            // Transport-specific mechanism constraints.
            //
            // `instance_from:` is a SOME/IP-only field because the
            // typed `uint16_t` instance_id is not a string carrier —
            // other transports have no target for the substituted
            // value. A non-SOME/IP binding declaring `instance_from:`
            // would sink into codegen silently; reject at parse.
            if has_instance_from && binding.transport != "someip" {
                return Err(DeployError::PoolNotSupportedByTransport {
                    machine: machine_name.to_string(),
                    binding: binding_key.as_str().to_string(),
                    transport: binding.transport.clone(),
                });
            }
            // Conversely, `{name}` placeholders have no wire surface
            // on SOME/IP: the only SOME/IP-side substitution target
            // (`set_instance`) is a typed integer, not a string
            // carrier. A SOME/IP binding declaring `{name}` is
            // therefore an author-facing mechanism mix-up; the
            // uniform answer is "use instance_from for SOME/IP".
            if has_placeholder && binding.transport == "someip" {
                return Err(DeployError::PoolInvalidPlaceholder {
                    machine: machine_name.to_string(),
                    binding: binding_key.as_str().to_string(),
                    reason:
                        "SOME/IP bindings express runtime instance selection via \
                         `instance_from: <param-name>`, not `{name}` placeholders \
                         — the instance_id is a typed uint16_t, not a string carrier"
                            .to_string(),
                });
            }

            let pool_requested = has_placeholder || has_instance_from;
            // SOME/IP `instances:` without a pool request still needs
            // the list to be non-empty if the author declared one,
            // but the transport-capability gate below only fires when
            // runtime substitution is actually requested.
            if !pool_requested {
                if let Some(list) = &binding.instances {
                    if list.is_empty() {
                        return Err(DeployError::PoolEmptyInstanceList {
                            machine: machine_name.to_string(),
                            binding: binding_key.as_str().to_string(),
                        });
                    }
                }
                continue;
            }
            // Transport capability gate.
            let descriptor =
                crate::mesh::transport::lookup(&binding.transport).ok_or_else(|| {
                    DeployError::PoolNotSupportedByTransport {
                        machine: machine_name.to_string(),
                        binding: binding_key.as_str().to_string(),
                        transport: binding.transport.clone(),
                    }
                })?;
            if !descriptor.supports_pool {
                return Err(DeployError::PoolNotSupportedByTransport {
                    machine: machine_name.to_string(),
                    binding: binding_key.as_str().to_string(),
                    transport: binding.transport.clone(),
                });
            }
            // SOME/IP-specific bounded-pool requirement: the
            // `instance_from:` / `instances:` pair goes together.
            if binding.transport == "someip" {
                match &binding.instances {
                    None => {
                        return Err(DeployError::PoolMissingInstanceList {
                            machine: machine_name.to_string(),
                            binding: binding_key.as_str().to_string(),
                        });
                    }
                    Some(list) if list.is_empty() => {
                        return Err(DeployError::PoolEmptyInstanceList {
                            machine: machine_name.to_string(),
                            binding: binding_key.as_str().to_string(),
                        });
                    }
                    Some(_) => {}
                }
            }
        }
    }
    Ok(())
}

/// Walk every machine that declared an explicit `outbound_buffer:`
/// section and reject capacity-zero values (SCE Mesh §10.10). Runs at
/// parse time so the diagnostic surfaces the offending deploy.yaml
/// line rather than generating a router whose buffer behaves
/// identically to the opt-out path.
fn validate_outbound_buffer(cfg: &DeployConfig) -> Result<(), DeployError> {
    use std::collections::BTreeMap;
    let mut by_machine: BTreeMap<&str, &OutboundBufferConfig> = BTreeMap::new();
    for device in cfg.topology.values() {
        for (machine_name, machine) in &device.machines {
            if let Some(b) = &machine.outbound_buffer {
                by_machine.insert(machine_name.as_str(), b);
            }
        }
    }
    for (machine, buffer) in by_machine {
        if let Some(reason) = buffer.validation_error() {
            return Err(DeployError::InvalidOutboundBuffer {
                machine: machine.to_string(),
                reason,
            });
        }
    }
    Ok(())
}

/// Walk every machine that declared an explicit `server.query_timeout_ms`
/// and reject values below [`MIN_SERVER_QUERY_TIMEOUT_MS`]. Runs at parse
/// time so the diagnostic surfaces the offending deploy.yaml line rather
/// than a silent runtime cleanup cascade when every inbound query times
/// out before the engine can respond.
fn validate_server_query_timeout(cfg: &DeployConfig) -> Result<(), DeployError> {
    use std::collections::BTreeMap;
    let mut by_machine: BTreeMap<&str, &ServerConfig> = BTreeMap::new();
    for device in cfg.topology.values() {
        for (machine_name, machine) in &device.machines {
            if let Some(s) = &machine.server {
                by_machine.insert(machine_name.as_str(), s);
            }
        }
    }
    for (machine, server) in by_machine {
        if let Some(reason) = server.query_timeout_validation_error() {
            return Err(DeployError::InvalidServerQueryTimeout {
                machine: machine.to_string(),
                reason,
            });
        }
    }
    Ok(())
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

    // ── custom_tcp transport config ──────────────────────────

    #[test]
    fn custom_tcp_listen_parsed() {
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    transports:
      custom_tcp:
        listen: "127.0.0.1:9000"
    machines:
      brake:
        source: brake.scxml
        bindings:
          "#motor":
            transport: custom_tcp
            connect: "127.0.0.1:9001"
"##;
        let cfg = parse_deploy_str(yaml).expect("parse");
        let tcp = cfg.topology["ecu1"]
            .transports
            .custom_tcp
            .as_ref()
            .expect("custom_tcp block");
        assert_eq!(tcp.listen.as_deref(), Some("127.0.0.1:9000"));
        // Per-binding `connect:` lands in BindingConfig.extra (serde flatten).
        let brake = &cfg.topology["ecu1"].machines["brake"];
        let motor_binding = &brake.bindings[&TargetId::new("#motor").unwrap()];
        assert_eq!(
            motor_binding.extra.get("connect").and_then(|v| v.as_str()),
            Some("127.0.0.1:9001")
        );
    }

    #[test]
    fn custom_tcp_listen_optional() {
        // Pure-client device omits listen.
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        bindings:
          "#motor":
            transport: custom_tcp
            connect: "127.0.0.1:9001"
"##;
        let cfg = parse_deploy_str(yaml).expect("parse");
        assert!(cfg.topology["ecu1"].transports.custom_tcp.is_none());
    }

    #[test]
    fn custom_tcp_unknown_session_field_rejected() {
        // Typo: `lsten` instead of `listen`.
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    transports:
      custom_tcp:
        lsten: "127.0.0.1:9000"
    machines:
      brake: { source: brake.scxml }
"##;
        assert!(matches!(parse_deploy_str(yaml), Err(DeployError::Yaml(_))));
    }

    // ── ordering (SCE_MESH.md §10.6) ──────────────────────────

    #[test]
    fn binding_ordering_required_parsed() {
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        bindings:
          "#motor":
            transport: zenoh
            key: motor/cmd
            ordering: required
"##;
        let cfg = parse_deploy_str(yaml).expect("parse");
        let brake = &cfg.topology["ecu1"].machines["brake"];
        let motor = &brake.bindings[&TargetId::new("#motor").unwrap()];
        assert_eq!(motor.ordering, OrderingRequirement::Required);
    }

    #[test]
    fn binding_ordering_absent_defaults_to_none() {
        // Same fixture as binding_ordering_required_parsed but without
        // the ordering key — must default to None, not fail parse.
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        bindings:
          "#motor":
            transport: zenoh
            key: motor/cmd
"##;
        let cfg = parse_deploy_str(yaml).expect("parse");
        let brake = &cfg.topology["ecu1"].machines["brake"];
        let motor = &brake.bindings[&TargetId::new("#motor").unwrap()];
        assert_eq!(motor.ordering, OrderingRequirement::None);
        assert!(motor.ordering.is_none());
    }

    #[test]
    fn binding_ordering_typo_rejected() {
        // `required` is the only non-default variant; a typo must fail
        // at parse time rather than silently falling through to the
        // `extra` map.
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        bindings:
          "#motor":
            transport: zenoh
            key: motor/cmd
            ordering: reqired
"##;
        let err = parse_deploy_str(yaml).unwrap_err();
        match err {
            DeployError::Yaml(msg) => {
                assert!(
                    msg.to_lowercase().contains("ordering")
                        || msg.to_lowercase().contains("variant")
                        || msg.to_lowercase().contains("required"),
                    "expected OrderingRequirement unknown-variant message, got: {msg}"
                );
            }
            other => panic!("expected DeployError::Yaml, got {other:?}"),
        }
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

    // ── ordering timings (SCE_MESH.md §10.6.1) ────────────────

    #[test]
    fn ordering_timings_absent_section_resolves_to_defaults() {
        // Machine without an `ordering:` section ⇒ defaults from the
        // single source (DEFAULT_GAP_TIMEOUT_MS / DEFAULT_TICK_PERIOD_MS).
        let yaml = r#"
version: "1.0"
topology:
  ecu1:
    machines:
      brake: { source: brake.scxml }
"#;
        let cfg = parse_deploy_str(yaml).expect("parse");
        let brake = &cfg.topology["ecu1"].machines["brake"];
        assert!(brake.ordering.is_none());
        let resolved = brake.resolved_ordering_timings();
        assert_eq!(resolved.gap_timeout_ms, DEFAULT_GAP_TIMEOUT_MS);
        assert_eq!(resolved.tick_period_ms, DEFAULT_TICK_PERIOD_MS);
    }

    #[test]
    fn ordering_timings_full_section_overrides_defaults() {
        let yaml = r#"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        ordering:
          gap_timeout_ms: 250
          tick_period_ms: 80
"#;
        let cfg = parse_deploy_str(yaml).expect("parse");
        let brake = &cfg.topology["ecu1"].machines["brake"];
        let timings = brake.ordering.expect("section present");
        assert_eq!(timings.gap_timeout_ms, 250);
        assert_eq!(timings.tick_period_ms, 80);
        // resolved_ordering_timings echoes the explicit value, not the
        // module default.
        let resolved = brake.resolved_ordering_timings();
        assert_eq!(resolved, timings);
    }

    #[test]
    fn ordering_timings_partial_section_rejected() {
        // Only `gap_timeout_ms:` is set — the schema requires both
        // because partial overrides leave the relationship between the
        // two values implicit.
        let yaml = r#"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        ordering:
          gap_timeout_ms: 250
"#;
        let err = parse_deploy_str(yaml).unwrap_err();
        match err {
            DeployError::Yaml(msg) => assert!(
                msg.to_lowercase().contains("tick_period_ms")
                    || msg.to_lowercase().contains("missing field"),
                "expected missing-field message about tick_period_ms; got: {msg}"
            ),
            other => panic!("expected DeployError::Yaml, got {other:?}"),
        }
    }

    #[test]
    fn ordering_timings_unknown_field_rejected() {
        // Typo: `tcik_period_ms` — deny_unknown_fields must fire.
        let yaml = r#"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        ordering:
          gap_timeout_ms: 250
          tcik_period_ms: 80
"#;
        let err = parse_deploy_str(yaml).unwrap_err();
        assert!(matches!(err, DeployError::Yaml(_)));
    }

    #[test]
    fn ordering_timings_zero_gap_rejected() {
        let yaml = r#"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        ordering:
          gap_timeout_ms: 0
          tick_period_ms: 1
"#;
        match parse_deploy_str(yaml) {
            Err(DeployError::InvalidOrderingTimings { machine, reason }) => {
                assert_eq!(machine, "brake");
                assert!(reason.contains("gap_timeout_ms"), "reason: {reason}");
            }
            other => panic!("expected InvalidOrderingTimings, got {other:?}"),
        }
    }

    #[test]
    fn ordering_timings_zero_tick_rejected() {
        let yaml = r#"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        ordering:
          gap_timeout_ms: 100
          tick_period_ms: 0
"#;
        match parse_deploy_str(yaml) {
            Err(DeployError::InvalidOrderingTimings { machine, reason }) => {
                assert_eq!(machine, "brake");
                assert!(reason.contains("tick_period_ms"), "reason: {reason}");
            }
            other => panic!("expected InvalidOrderingTimings, got {other:?}"),
        }
    }

    #[test]
    fn liveliness_section_absent_is_default_none() {
        let yaml = r#"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
"#;
        let cfg = parse_deploy_str(yaml).expect("parse");
        let machine = &cfg.topology["ecu1"].machines["brake"];
        assert!(
            machine.liveliness.is_none(),
            "absent section must deserialize as None (opt-in gate)"
        );
    }

    #[test]
    fn liveliness_section_present_parses() {
        let yaml = r#"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        liveliness:
          lease_ms: 2000
"#;
        let cfg = parse_deploy_str(yaml).expect("parse");
        let machine = &cfg.topology["ecu1"].machines["brake"];
        assert_eq!(
            machine.liveliness.unwrap().lease_ms,
            2000,
            "explicit section must propagate the lease_ms value"
        );
    }

    #[test]
    fn liveliness_lease_below_floor_rejected() {
        let yaml = r#"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        liveliness:
          lease_ms: 50
"#;
        match parse_deploy_str(yaml) {
            Err(DeployError::InvalidLiveliness { machine, reason }) => {
                assert_eq!(machine, "brake");
                assert!(reason.contains("lease_ms"), "reason: {reason}");
                assert!(reason.contains("100"), "reason must cite the floor: {reason}");
            }
            other => panic!("expected InvalidLiveliness, got {other:?}"),
        }
    }

    #[test]
    fn liveliness_unknown_field_rejected() {
        // Typo: `leese_ms` — deny_unknown_fields must fire.
        let yaml = r#"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        liveliness:
          leese_ms: 2000
"#;
        let err = parse_deploy_str(yaml).unwrap_err();
        assert!(matches!(err, DeployError::Yaml(_)));
    }

    #[test]
    fn outbound_buffer_section_absent_is_default_none() {
        let yaml = r#"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
"#;
        let cfg = parse_deploy_str(yaml).expect("parse");
        let machine = &cfg.topology["ecu1"].machines["brake"];
        assert!(
            machine.outbound_buffer.is_none(),
            "absent section must deserialize as None (opt-in gate — §10.10)"
        );
    }

    #[test]
    fn outbound_buffer_section_present_parses() {
        let yaml = r#"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        outbound_buffer:
          max_pending_per_target: 64
"#;
        let cfg = parse_deploy_str(yaml).expect("parse");
        let machine = &cfg.topology["ecu1"].machines["brake"];
        assert_eq!(
            machine.outbound_buffer.unwrap().max_pending_per_target,
            64,
            "explicit section must propagate the max_pending_per_target value"
        );
    }

    #[test]
    fn outbound_buffer_zero_capacity_rejected() {
        let yaml = r#"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        outbound_buffer:
          max_pending_per_target: 0
"#;
        match parse_deploy_str(yaml) {
            Err(DeployError::InvalidOutboundBuffer { machine, reason }) => {
                assert_eq!(machine, "brake");
                assert!(
                    reason.contains("max_pending_per_target"),
                    "reason: {reason}",
                );
                assert!(
                    reason.contains("1"),
                    "reason must cite the floor MIN_OUTBOUND_BUFFER_MAX_PENDING = 1: {reason}",
                );
            }
            other => panic!("expected InvalidOutboundBuffer, got {other:?}"),
        }
    }

    #[test]
    fn discovery_block_absent_is_ok() {
        // No `discovery:` key — validator must pass.
        let yaml = r#"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
"#;
        parse_deploy_str(yaml).expect("parse");
    }

    #[test]
    fn discovery_block_with_keys_rejected() {
        // Authored §4.3 example-shaped block — rejected per §3.3 / §2574.
        let yaml = r#"
version: "1.0"
discovery:
  mode: dynamic
  resolution:
    strategy: priority
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
"#;
        match parse_deploy_str(yaml) {
            Err(DeployError::DiscoveryNotSupported { content_kind }) => {
                assert_eq!(
                    content_kind, "object with keys [mode, resolution]",
                    "content_kind must enumerate observed top-level keys",
                );
            }
            other => panic!("expected DiscoveryNotSupported, got {other:?}"),
        }
    }

    #[test]
    fn discovery_empty_map_rejected() {
        // An empty `discovery: {}` map is still Some(Value::Mapping(_)); §3.3
        // rejects the existence of the block, not its contents, so the
        // validator fires here too. Covers the "author sketched the key
        // but did not fill it in" shape.
        let yaml = r#"
version: "1.0"
discovery: {}
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
"#;
        match parse_deploy_str(yaml) {
            Err(DeployError::DiscoveryNotSupported { content_kind }) => {
                assert_eq!(content_kind, "empty object");
            }
            other => panic!("expected DiscoveryNotSupported, got {other:?}"),
        }
    }

    #[test]
    fn discovery_null_treated_as_absent() {
        // `discovery: null` deserialises to `Option::None`, so the
        // validator leaves it alone. This is intentional — `null` is
        // indistinguishable from absence at the YAML level, and
        // rejecting both under one diagnostic would force authors to
        // delete the key in every downstream template they copy from.
        let yaml = r#"
version: "1.0"
discovery: ~
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
"#;
        parse_deploy_str(yaml).expect("null discovery must parse");
    }

    #[test]
    fn outbound_buffer_unknown_field_rejected() {
        // Typo: `max_pendng_per_target` — deny_unknown_fields must fire.
        let yaml = r#"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        outbound_buffer:
          max_pendng_per_target: 64
"#;
        let err = parse_deploy_str(yaml).unwrap_err();
        assert!(matches!(err, DeployError::Yaml(_)));
    }

    #[test]
    fn server_query_timeout_absent_is_default_none() {
        let yaml = r#"
version: "1.0"
topology:
  ecu1:
    machines:
      motor:
        source: motor.scxml
        server:
          transport: zenoh
          key: "sce/motor"
"#;
        let cfg = parse_deploy_str(yaml).expect("parse");
        let machine = &cfg.topology["ecu1"].machines["motor"];
        let server = machine.server.as_ref().expect("server");
        assert!(
            server.query_timeout_ms.is_none(),
            "absent knob must deserialize as None (opt-in gate — Z2)"
        );
    }

    #[test]
    fn server_query_timeout_present_parses() {
        let yaml = r#"
version: "1.0"
topology:
  ecu1:
    machines:
      motor:
        source: motor.scxml
        server:
          transport: zenoh
          key: "sce/motor"
          query_timeout_ms: 500
"#;
        let cfg = parse_deploy_str(yaml).expect("parse");
        let machine = &cfg.topology["ecu1"].machines["motor"];
        let server = machine.server.as_ref().expect("server");
        assert_eq!(
            server.query_timeout_ms,
            Some(500),
            "explicit knob must propagate the value verbatim"
        );
    }

    #[test]
    fn server_query_timeout_on_non_zenoh_rejected() {
        // SOME/IP server has no `pending_server_queries_` map — Z2 wires
        // the scheduler to a zenoh-specific structure, so the knob would
        // silently no-op on SOME/IP. Parse-time rejection prevents the
        // silent-hook pattern at the config layer.
        let yaml = r#"
version: "1.0"
topology:
  ecu1:
    machines:
      motor:
        source: motor.scxml
        server:
          transport: someip
          service: motor_control
          query_timeout_ms: 500
"#;
        match parse_deploy_str(yaml) {
            Err(DeployError::InvalidServerQueryTimeout { machine, reason }) => {
                assert_eq!(machine, "motor");
                assert!(
                    reason.contains("zenoh"),
                    "reason must cite the required transport: {reason}"
                );
                assert!(
                    reason.contains("someip"),
                    "reason must cite the declared transport: {reason}"
                );
            }
            other => panic!("expected InvalidServerQueryTimeout, got {other:?}"),
        }
    }

    #[test]
    fn server_query_timeout_below_floor_rejected() {
        let yaml = r#"
version: "1.0"
topology:
  ecu1:
    machines:
      motor:
        source: motor.scxml
        server:
          transport: zenoh
          key: "sce/motor"
          query_timeout_ms: 5
"#;
        match parse_deploy_str(yaml) {
            Err(DeployError::InvalidServerQueryTimeout { machine, reason }) => {
                assert_eq!(machine, "motor");
                assert!(
                    reason.contains("query_timeout_ms"),
                    "reason must cite the knob: {reason}"
                );
                assert!(
                    reason.contains("10"),
                    "reason must cite the floor: {reason}"
                );
            }
            other => panic!("expected InvalidServerQueryTimeout, got {other:?}"),
        }
    }

    #[test]
    fn ordering_timings_nyquist_violation_rejected() {
        // tick_period_ms == gap_timeout_ms — the buffer can drift up to
        // an entire window before tick observes the gap. Rejected so the
        // recovery latency bound `gap_timeout + tick_period` always
        // implies tick fires at least once during a single gap.
        let yaml = r#"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        ordering:
          gap_timeout_ms: 100
          tick_period_ms: 100
"#;
        match parse_deploy_str(yaml) {
            Err(DeployError::InvalidOrderingTimings { reason, .. }) => {
                assert!(reason.contains("strictly less"), "reason: {reason}");
            }
            other => panic!("expected InvalidOrderingTimings, got {other:?}"),
        }
    }

    // ── SCE Mesh §14.4 binding pool ──────────────────────────

    #[test]
    fn pool_placeholder_grammar_unbalanced_brace_rejected() {
        let reason = extract_placeholders("sce/player/{id").unwrap_err();
        assert!(reason.contains("unbalanced '{'"), "reason: {reason}");
    }

    #[test]
    fn pool_placeholder_grammar_empty_name_rejected() {
        let reason = extract_placeholders("sce/player/{}").unwrap_err();
        assert!(reason.contains("empty placeholder"), "reason: {reason}");
    }

    #[test]
    fn pool_placeholder_grammar_extracts_name_list() {
        let names = extract_placeholders("sce/{room}/{id}/chat").unwrap();
        assert_eq!(names, vec!["room".to_string(), "id".to_string()]);
    }

    #[test]
    fn pool_placeholder_on_shm_rejected() {
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        bindings:
          "#logger":
            transport: shm
            shm_channel: "ch-{id}"
"##;
        match parse_deploy_str(yaml) {
            Err(DeployError::PoolNotSupportedByTransport { machine, binding, transport }) => {
                assert_eq!(machine, "brake");
                assert_eq!(binding, "#logger");
                assert_eq!(transport, "shm");
            }
            other => panic!("expected PoolNotSupportedByTransport, got {other:?}"),
        }
    }

    #[test]
    fn pool_someip_instance_from_without_instances_rejected() {
        // `instance_from:` requested a pool but the matching
        // `instances:` list is missing; vsomeip cannot enumerate.
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        bindings:
          "#player":
            transport: someip
            service: player_service
            method: handle_request
            instance_from: id
"##;
        match parse_deploy_str(yaml) {
            Err(DeployError::PoolMissingInstanceList { machine, binding }) => {
                assert_eq!(machine, "brake");
                assert_eq!(binding, "#player");
            }
            other => panic!("expected PoolMissingInstanceList, got {other:?}"),
        }
    }

    #[test]
    fn pool_someip_empty_instances_list_rejected() {
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        bindings:
          "#player":
            transport: someip
            service: player_service
            method: handle_request
            instance_from: id
            instances: []
"##;
        match parse_deploy_str(yaml) {
            Err(DeployError::PoolEmptyInstanceList { machine, binding }) => {
                assert_eq!(machine, "brake");
                assert_eq!(binding, "#player");
            }
            other => panic!("expected PoolEmptyInstanceList, got {other:?}"),
        }
    }

    #[test]
    fn pool_someip_with_brace_placeholder_rejected() {
        // SOME/IP bindings express runtime instance selection via
        // `instance_from:`, not `{name}` placeholders — the
        // instance_id is a typed uint16_t, not a string carrier.
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        bindings:
          "#player":
            transport: someip
            service: player_service
            method: handle_request
            instances: [1, 2, 3]
            key: "unused-{id}"
"##;
        match parse_deploy_str(yaml) {
            Err(DeployError::PoolInvalidPlaceholder { machine, binding, reason }) => {
                assert_eq!(machine, "brake");
                assert_eq!(binding, "#player");
                assert!(
                    reason.contains("instance_from") && reason.contains("uint16"),
                    "reason should steer author to instance_from for SOME/IP; got: {reason}"
                );
            }
            other => panic!("expected PoolInvalidPlaceholder, got {other:?}"),
        }
    }

    #[test]
    fn pool_instance_from_on_non_someip_rejected() {
        // `instance_from:` has no wire surface outside SOME/IP.
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    transports:
      zenoh:
        mode: peer
    machines:
      brake:
        source: brake.scxml
        bindings:
          "#player":
            transport: zenoh
            key: "sce/player/x"
            instance_from: id
"##;
        match parse_deploy_str(yaml) {
            Err(DeployError::PoolNotSupportedByTransport { machine, binding, transport }) => {
                assert_eq!(machine, "brake");
                assert_eq!(binding, "#player");
                assert_eq!(transport, "zenoh");
            }
            other => panic!("expected PoolNotSupportedByTransport, got {other:?}"),
        }
    }

    #[test]
    fn pool_someip_with_instance_from_and_instances_accepted() {
        // The canonical SOME/IP client pool shape: `instance_from:`
        // names the <param>, `instances:` pre-declares the finite set.
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        bindings:
          "#player":
            transport: someip
            service: player_service
            method: handle_request
            instance_from: id
            instances: [1, 2, 3, 100, 101]
"##;
        let cfg = parse_deploy_str(yaml).expect("parse");
        let binding = cfg.topology["ecu1"].machines["brake"]
            .bindings
            .iter()
            .find(|(k, _)| k.as_str() == "#player")
            .expect("binding")
            .1;
        assert_eq!(binding.instance_from.as_deref(), Some("id"));
        assert_eq!(binding.instances.as_ref().unwrap().len(), 5);
    }

    #[test]
    fn server_pool_accepted_on_someip() {
        // SCE_MESH.md §14.4 (Gap 7): SOME/IP is the
        // sole transport whose native routing distinguishes inbound by
        // peer identity (instance_id in vsomeip's message header), so
        // it is the only transport for which multi-instance server pool
        // has a well-defined inbound dispatch shape. The parse layer
        // accepts `server.instances:` when the registry flag
        // `supports_multi_instance_server` is `true`; downstream stages
        // (topology carry + codegen per-instance emit) consume the list.
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    transports:
      someip:
        config: vsomeip.json
        application_name: motor_app
    machines:
      motor:
        source: motor.scxml
        server:
          transport: someip
          service: motor_service
          instances: [1, 2]
"##;
        let cfg = parse_deploy_str(yaml).expect("parse");
        let server = cfg.topology["ecu1"].machines["motor"]
            .server
            .as_ref()
            .expect("server");
        assert_eq!(server.instances.as_ref().map(|v| v.len()), Some(2));
    }

    #[test]
    fn server_pool_rejected_on_zenoh() {
        // `supports_multi_instance_server` is `false` for Zenoh — a
        // KeyExpr is not a peer identity, so multi-instance server
        // hosting has no transport-layer distinguisher. Diagnostic
        // names the offending transport so authors can read the
        // per-transport policy without consulting the spec.
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    transports:
      zenoh:
        mode: peer
    machines:
      motor:
        source: motor.scxml
        server:
          transport: zenoh
          key: "sce/motor"
          instances: [1, 2]
"##;
        match parse_deploy_str(yaml) {
            Err(DeployError::ServerPoolNotSupported { machine, transport }) => {
                assert_eq!(machine, "motor");
                assert_eq!(transport, "zenoh");
            }
            other => panic!("expected ServerPoolNotSupported with transport=zenoh, got {other:?}"),
        }
    }

    #[test]
    fn pool_zenoh_placeholder_accepted_at_parse_time() {
        // Zenoh `{id}` placeholder is accepted; the codegen-time
        // substitution threads through TargetContext.
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    transports:
      zenoh:
        mode: peer
    machines:
      brake:
        source: brake.scxml
        bindings:
          "#player":
            transport: zenoh
            key: "sce/player/{id}"
"##;
        let cfg = parse_deploy_str(yaml).expect("parse");
        let brake = &cfg.topology["ecu1"].machines["brake"];
        let binding = brake
            .bindings
            .iter()
            .find(|(k, _)| k.as_str() == "#player")
            .expect("binding")
            .1;
        assert_eq!(binding.transport, "zenoh");
    }

    #[test]
    fn pool_without_placeholder_unchanged_behaviour() {
        // A deploy.yaml without placeholders is accepted exactly as before
        // — the placeholder machinery is purely additive.
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    transports:
      zenoh:
        mode: peer
    machines:
      brake:
        source: brake.scxml
        bindings:
          "#motor":
            transport: zenoh
            key: "sce/motor"
"##;
        parse_deploy_str(yaml).expect("parse");
    }

    // SCE_MESH.md §14 rules 6-10 — partitions schema coverage. Each
    // test pins one rejection shape via its structured DeployError
    // variant so golden snapshots do not drift on message tweaks.

    #[test]
    fn partitions_happy_path_two_regions_one_device() {
        // Positive fixture: one machine, two parallel regions, two
        // partitions. No SCXML is consulted — rules 7-10 operate on
        // the deploy.yaml structure alone, so this exercises the happy
        // path for the schema without requiring an analyzer phase.
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        bindings: {}

partitions:
  brake_main:
    device: ecu1
    machines: [brake]
    contains:
      parallel_regions:
        - { machine: brake, region: watchdog }
  brake_worker:
    device: ecu1
    machines: [brake]
    contains:
      parallel_regions:
        - { machine: brake, region: executor }
"##;
        let cfg = parse_deploy_str(yaml).expect("parse");
        let parts = cfg.partitions.expect("partitions present");
        assert_eq!(parts.len(), 2);
        assert!(parts.get("brake_main").is_some());
        assert!(parts.get("brake_worker").is_some());
    }

    #[test]
    fn reject_duplicate_partition_name_rule6() {
        // Rule 6 — two `partitions.<name>:` entries with the same
        // name. BTreeMap typed parse would silently dedupe; the
        // custom PartitionMap visitor rejects via a sentinel-tagged
        // error that parse_deploy_str lifts to a structured diagnostic.
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        bindings: {}

partitions:
  brake_main:
    device: ecu1
    machines: [brake]
    contains:
      parallel_regions:
        - { machine: brake, region: watchdog }
  brake_main:
    device: ecu1
    machines: [brake]
    contains:
      parallel_regions:
        - { machine: brake, region: executor }
"##;
        match parse_deploy_str(yaml) {
            Err(DeployError::PartitionDuplicateName { name }) => {
                assert_eq!(name, "brake_main");
            }
            other => panic!("expected PartitionDuplicateName, got {other:?}"),
        }
    }

    #[test]
    fn reject_multi_device_partition_rule7() {
        // Rule 7 — the partition's `machines:` list spans more than
        // one device. A partition is one process on one device.
        let yaml = r##"
version: "1.0"
topology:
  ecu_a:
    machines:
      brake:
        source: brake.scxml
        bindings: {}
  ecu_b:
    machines:
      motor:
        source: motor.scxml
        bindings: {}

partitions:
  cross_part:
    machines: [brake, motor]
    contains:
      parallel_regions:
        - { machine: brake, region: watchdog }
        - { machine: motor, region: drive }
"##;
        match parse_deploy_str(yaml) {
            Err(DeployError::PartitionMultiDevice { partition, devices }) => {
                assert_eq!(partition, "cross_part");
                let mut ds = devices;
                ds.sort();
                assert_eq!(ds, vec!["ecu_a".to_string(), "ecu_b".to_string()]);
            }
            other => panic!("expected PartitionMultiDevice, got {other:?}"),
        }
    }

    #[test]
    fn reject_unit_in_two_partitions_rule8() {
        // Rule 8 — one `(machine, region)` unit listed under two
        // partitions. Each orthogonal unit belongs to exactly one
        // partition; analyzer never silently picks.
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        bindings: {}

partitions:
  part_a:
    device: ecu1
    machines: [brake]
    contains:
      parallel_regions:
        - { machine: brake, region: shared }
  part_b:
    device: ecu1
    machines: [brake]
    contains:
      parallel_regions:
        - { machine: brake, region: shared }
"##;
        match parse_deploy_str(yaml) {
            Err(DeployError::PartitionUnitDuplicate { unit, partitions }) => {
                assert_eq!(unit, "parallel_region:brake/shared");
                let mut ps = partitions;
                ps.sort();
                assert_eq!(ps, vec!["part_a".to_string(), "part_b".to_string()]);
            }
            other => panic!("expected PartitionUnitDuplicate, got {other:?}"),
        }
    }

    #[test]
    fn reject_contains_references_unlisted_machine_rule9() {
        // Rule 9 — `contains:` entry references a machine that the
        // partition's `machines:` does not list. A partition cannot
        // reach into another partition's address space.
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        bindings: {}
      motor:
        source: motor.scxml
        bindings: {}

partitions:
  brake_only:
    device: ecu1
    machines: [brake]
    contains:
      parallel_regions:
        - { machine: motor, region: drive }
"##;
        match parse_deploy_str(yaml) {
            Err(DeployError::PartitionMachineNotListed {
                partition,
                machine,
            }) => {
                assert_eq!(partition, "brake_only");
                assert_eq!(machine, "motor");
            }
            other => panic!("expected PartitionMachineNotListed, got {other:?}"),
        }
    }

    #[test]
    fn reject_empty_partition_rule10() {
        // Rule 10 — empty `contains:` block. An empty partition has
        // no runtime purpose and usually indicates a copy-paste error.
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        bindings: {}

partitions:
  empty_part:
    device: ecu1
    machines: [brake]
    contains:
      parallel_regions: []
      invokes: []
"##;
        match parse_deploy_str(yaml) {
            Err(DeployError::PartitionEmpty { partition }) => {
                assert_eq!(partition, "empty_part");
            }
            other => panic!("expected PartitionEmpty, got {other:?}"),
        }
    }

    #[test]
    fn partitions_absent_is_normal_monolith() {
        // Regression guard — the absence of `partitions:` must parse
        // identically to every in-tree deploy.yaml. No validator is
        // allowed to fire on `None`.
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        bindings: {}
"##;
        let cfg = parse_deploy_str(yaml).expect("parse");
        assert!(cfg.partitions.is_none());
    }
}
