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

use crate::mesh::error::{DeployError, PartitionTransportBindingFailure};
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
    /// SCE_MESH.md §16.3 — strict vs permissive distributability
    /// mode. `strict` fails the build on any R1/R2 violation;
    /// `permissive` (the absent-value default) auto-merges offending
    /// regions per §16.4 and records a [`crate::mesh::distributability::MergeNotice`].
    /// The knob is meaningful only when `partitions:` is present; an
    /// absent key means "permissive".
    #[serde(default)]
    pub distributability: Option<DistributabilityMode>,
}

/// SCE_MESH.md §16.3 strict/permissive toggle. Default is
/// [`DistributabilityMode::Permissive`] so authors who author a
/// partition plan that happens to violate R1/R2 still get a
/// minimum-merge build rather than a hard failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DistributabilityMode {
    Strict,
    Permissive,
}

impl Default for DistributabilityMode {
    fn default() -> Self {
        DistributabilityMode::Permissive
    }
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

    /// Construct a [`PartitionMap`] from an already-validated
    /// [`BTreeMap`]. Internal use only — the public deserialization
    /// path goes through the custom
    /// [`PartitionMap::deserialize`] visitor, which enforces rule-6
    /// uniqueness. Callers that build a map from post-merge state
    /// (§16.4 resolver) have already walked the original
    /// [`PartitionMap`] and therefore carry the rule-6 guarantee by
    /// construction; they merely rearrange entries.
    pub(crate) fn from_map(map: BTreeMap<String, PartitionDecl>) -> Self {
        PartitionMap(map)
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
    /// Distributed `<parallel>`s this partition claims as the root
    /// (SCE_MESH.md §14 rule 12, L2729-2735). Each entry names a
    /// `<parallel>` in one of the partition's listed machines; the
    /// rule-12 cross-reference validator (see
    /// [`crate::mesh::partitions::validate_parallel_root_designation`])
    /// enforces per-`<parallel>` uniqueness of claimants, rule-9 shape
    /// on `machine:`, and co-hosting of at least one region of the
    /// claimed parallel. A `<parallel>` whose regions live entirely in
    /// a single partition has that partition as implicit root; the
    /// field may be omitted in that case. Absent ⇒ `None` (no claims);
    /// empty list ⇒ `Some(vec![])` (treated identically to absent, but
    /// the authors can keep the key visible in source-controlled
    /// deploys if they prefer the explicit marker).
    #[serde(default)]
    pub hosts_parallel_roots: Option<Vec<HostsParallelRoot>>,
}

/// One entry under `partitions.<name>.hosts_parallel_roots:` — a
/// `(machine, parallel)` pair naming the `<parallel>` this partition
/// claims as the root (SCE_MESH.md §14 rule 12). `parallel` is the
/// `<parallel>` element's `id` attribute as authored in the SCXML
/// document for `machine`.
#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Hash)]
#[serde(deny_unknown_fields)]
pub struct HostsParallelRoot {
    /// SCXML machine name (deploy.yaml `machines.<name>` key). Must be
    /// one of the partition's `machines:` entries — rule 9 shape
    /// enforced by [`crate::mesh::partitions::validate_parallel_root_designation`].
    pub machine: String,
    /// `<parallel id>` in the named machine's SCXML document.
    pub parallel: String,
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
/// **Transport compat**: the declaring machine must also carry at
/// least one Zenoh binding or server; [`validate_liveliness`] enforces
/// this at parse time. The codegen template emits liveliness
/// primitives only when `"zenoh" in transport_types`, so a SomeIP-only
/// (or binding-less) machine declaring `liveliness:` without that gate
/// would compile but never raise the `error.communication` signal its
/// required handler awaits (`feedback_silently_broken_hooks`). SomeIP
/// per-partition liveness is tracked as a separate §16.9 E2/F
/// sub-landing — see SCE_MESH.md §16.4 for the deferral rationale.
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
    /// Author-pinned §9.6 SOMEIP scxml-invoke service ID (RFC F.X-1
    /// hybrid allocator). Optional — when present, the hybrid allocator
    /// reserves this ID for the machine and skips it during counter
    /// auto-assignment. Absent ⇒ counter auto-assigns from the lowest
    /// unreserved slot in lex order.
    ///
    /// **Range constraint**: must lie inside the §9.6 invoke sub-range
    /// `[0x8100, 0x817F]` — the upper half of the SCE-reserved 256-slot
    /// space is reserved for §16.4 region-liveness (RFC F.X-3).
    /// [`crate::mesh::transport::someip::assign_invoke_service_ids`]
    /// rejects out-of-range pins with
    /// [`DeployError::SomeipScxmlInvokeServiceIdPinOutOfRange`].
    ///
    /// **YAML grammar**: accepts either an integer literal (`33029` or
    /// YAML 1.1 hex `0x8105`) or a quoted hex string (`"0x8105"`). The
    /// string form is preferred for readability; both round-trip to the
    /// same `u16`.
    ///
    /// Use case: pin the IDs of long-lived participants whose Wireshark
    /// captures or cross-team contracts depend on a stable service ID.
    /// New auto-assigned participants will not shift pinned ones.
    #[serde(default, deserialize_with = "deserialize_someip_service_id")]
    pub someip_service_id: Option<u16>,
}

/// Custom deserializer for [`MachineConfig::someip_service_id`].
///
/// Accepts both YAML integer literals (`33029`, or YAML 1.1 hex `0x8105`
/// which parses as `Int(33029)`) and quoted hex strings (`"0x8105"`).
/// The latter form is preferred for readability — strings are unambiguous
/// across YAML versions whereas the `0x` prefix is YAML 1.1-only.
fn deserialize_someip_service_id<'de, D>(deserializer: D) -> Result<Option<u16>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum HexOrInt {
        Int(u16),
        Hex(String),
    }

    let opt = Option::<HexOrInt>::deserialize(deserializer)?;
    match opt {
        None => Ok(None),
        Some(HexOrInt::Int(n)) => Ok(Some(n)),
        Some(HexOrInt::Hex(s)) => {
            // Accept either `0x8105` or `0X8105`; reject anything else
            // so author typos like `8105` (decimal-looking but intended
            // as hex) surface at parse time.
            let trimmed = if let Some(rest) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X"))
            {
                rest
            } else {
                return Err(serde::de::Error::custom(format!(
                    "someip_service_id: hex string '{s}' must start with `0x` \
                     (e.g. `\"0x8105\"`); raw decimal integers are also accepted \
                     (e.g. `33029`) but bare hex strings without the prefix are \
                     rejected to avoid `0x8105` vs `8105` confusion"
                )));
            };
            u16::from_str_radix(trimmed, 16).map(Some).map_err(|e| {
                serde::de::Error::custom(format!(
                    "someip_service_id: cannot parse hex literal '{s}' as u16: {e} \
                     (expected `0x8100`-style hex inside [0x0000, 0xFFFF])"
                ))
            })
        }
    }
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
    validate_synth_invoke_infix(&cfg)?;
    validate_partitions_schema(&cfg)?;
    validate_someip_scxml_invoke_service_ids(&cfg)?;
    validate_someip_scxml_invoke_service_id_collisions(&cfg)?;

    Ok(cfg)
}

/// SCE Mesh §14 rule 5 — author-declared machine ids must not use the
/// reserved `__sce_synth_invoke__` infix. Synthesized children from
/// `<invoke type="scxml">` inline `<content>` (§9.6.6) are named
/// `<parent>__sce_synth_invoke__<id>`; a collision would silently
/// shadow or be shadowed by the synthesized peer at runtime, and the
/// partition coverage rules could not tell the two apart.
///
/// **Explicit override carve-out** (§9.6.6 rule 3): when the author
/// reassigns a synth machine to a different partition, they must also
/// add the synth to `topology.*.machines` so transport codegen can
/// emit the wire. Such an entry carries the reserved infix by
/// construction. It is admitted iff the name matches the synth shape
/// `<parent>__sce_synth_invoke__<id>` where `<parent>` is itself a
/// declared machine — so the entry is provably the projection of an
/// inline invoke that the author intended to distribute, not a typo.
/// Typos that merely contain the infix (no matching parent) continue
/// to fire `PartitionSynthInfixCollision` as before.
fn validate_synth_invoke_infix(cfg: &DeployConfig) -> Result<(), DeployError> {
    // Pre-compute the set of all declared machine ids once — the
    // carve-out below needs a membership check per infix-bearing name.
    let all_machines: std::collections::HashSet<&str> = cfg
        .topology
        .values()
        .flat_map(|d| d.machines.keys().map(String::as_str))
        .collect();
    for device in cfg.topology.values() {
        for name in device.machines.keys() {
            let Some((parent, _)) = name.split_once(SYNTH_INVOKE_INFIX) else {
                continue;  // no infix, no collision concern
            };
            if !parent.is_empty() && all_machines.contains(parent) {
                // Explicit override surface — author declares synth
                // under a non-parent partition, a sibling topology
                // entry for it is required so transport codegen can
                // emit channels. Admit.
                continue;
            }
            return Err(DeployError::PartitionSynthInfixCollision {
                machine: name.clone(),
            });
        }
    }
    Ok(())
}

/// The infix that names a machine synthesized from `<invoke type="scxml">`
/// inline `<content>` (SCE_MESH.md §14 rule 5 + §9.6.6). Single source
/// of truth so the parser, the validator, and any future synthesizer
/// agree on the reserved string.
pub const SYNTH_INVOKE_INFIX: &str = "__sce_synth_invoke__";

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

    // SCE_MESH.md §14.4 × §14 — SOME/IP server pool machines cannot be
    // partitioned. Pool semantics scope one router to N SOME/IP sessions
    // on a single process; partition semantics split one machine across
    // M OS processes. deploy.yaml defines neither a pool-of-partitions
    // nor a partition-of-pools, so the parser rejects the combination
    // at the machine-listing site instead of accepting a shape whose
    // runtime behaviour is undefined. Pre-build the pool-machine set so
    // the per-partition loop below is one BTreeSet::contains per listed
    // machine rather than a nested topology walk.
    let pool_machines: std::collections::BTreeSet<&str> = cfg
        .topology
        .values()
        .flat_map(|device| device.machines.iter())
        .filter_map(|(name, m)| {
            m.server
                .as_ref()
                .and_then(|s| s.instances.as_ref())
                .map(|_| name.as_str())
        })
        .collect();

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

        // §14 L2729-2730 — `transport_binding:` must name a transport
        // whose primary purpose is same-machine IPC. Unknown names and
        // known-but-incapable transports both fall here, with `reason`
        // telling the two shapes apart so the diagnostic is self-
        // explaining without the reader needing to cross-reference the
        // registry. Absent ⇒ skip (§14 L2730 defaults apply at codegen
        // time).
        if let Some(transport_name) = decl.transport_binding.as_deref() {
            match crate::mesh::transport::lookup(transport_name) {
                None => {
                    let known_names: Vec<String> = crate::mesh::transport::implemented_names()
                        .iter()
                        .map(|s| (*s).to_string())
                        .collect();
                    return Err(DeployError::PartitionTransportBindingUnsupported {
                        partition: partition_name.clone(),
                        transport: transport_name.to_string(),
                        failure: PartitionTransportBindingFailure::Unknown { known_names },
                    });
                }
                Some(desc) if !desc.supports_inter_partition_ipc => {
                    return Err(DeployError::PartitionTransportBindingUnsupported {
                        partition: partition_name.clone(),
                        transport: transport_name.to_string(),
                        failure: PartitionTransportBindingFailure::Incapable {
                            transport: transport_name.to_string(),
                        },
                    });
                }
                Some(_) => {}
            }
        }

        // §14 L2731-2732 — `barrier_timeout_ms:` is Option<u32>; `None`
        // / absent selects the W3C normative default of infinity. A
        // finite value of `0` would fire the §16.5 barrier before any
        // region can report `ParallelRegionDone`, unconditionally
        // raising `error.communication / PARALLEL_BARRIER_TIMEOUT` on
        // every `<parallel>` activation — the knob exists to bound
        // authentic hangs, not to convert barriers into errors. Authors
        // wanting "do not wait" must omit the key and rely on standard
        // SCXML transitions. Root-hosting-only semantics (spec
        // L2733-2735 "applies only to partitions hosting the root of a
        // `<parallel>`") is SCXML cross-reference scope (§16.5 runtime)
        // and is not enforced here — schema accept + range check only.
        if let Some(value) = decl.barrier_timeout_ms {
            if value == 0 {
                return Err(DeployError::PartitionBarrierTimeoutInvalid {
                    partition: partition_name.clone(),
                    value,
                    reason: "barrier_timeout_ms (0) would fire the §16.5 parallel-final \
                             barrier before any region can report ParallelRegionDone"
                        .to_string(),
                });
            }
        }

        // §14.4 × §14 pool-in-partition guard — reject before rule 7
        // so a pool machine listed across multiple devices surfaces the
        // spec-linked message instead of the generic multi-device one.
        // Iteration follows `machines:` author order, matching rule 7's
        // existing walk; a partition that lists multiple pool machines
        // names the first in source order, which is stable across
        // repeat parses of the same file.
        for machine in &decl.machines {
            if pool_machines.contains(machine.as_str()) {
                return Err(DeployError::PartitionPoolMachine {
                    machine: machine.clone(),
                    partition: partition_name.clone(),
                });
            }
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
/// and reject (a) values below [`MIN_LIVELINESS_LEASE_MS`] and (b)
/// machines whose transport set contains no Zenoh binding or server.
/// Runs at parse time so the diagnostic surfaces the offending
/// deploy.yaml line rather than a deferred runtime misbehaviour.
///
/// The transport-compat check closes the silent-broken window where a
/// SomeIP-only (or binding-less) machine declared `liveliness:` and
/// carried the `error.communication` handler required by
/// [`sce-build/src/generator.rs::reject_liveliness_without_handler`] —
/// the codegen template gates liveliness emission on
/// `"zenoh" in transport_types`, so such a machine previously compiled
/// with a handler that could never fire (`feedback_silently_broken_hooks`).
/// SomeIP per-partition liveness is tracked as a separate landing
/// (SCE_MESH.md §16.4 / §16.9 Session E2/F deferral) and requires a
/// distinct design round — per-partition application names, OEM
/// `vsomeip.json` coordination, and a §10.4 transport-contract
/// micro-revision — that Zenoh did not need.
fn validate_liveliness(cfg: &DeployConfig) -> Result<(), DeployError> {
    use std::collections::BTreeMap;
    let mut by_machine: BTreeMap<&str, (&LivelinessConfig, &MachineConfig)> = BTreeMap::new();
    for device in cfg.topology.values() {
        for (machine_name, machine) in &device.machines {
            if let Some(l) = &machine.liveliness {
                by_machine.insert(machine_name.as_str(), (l, machine));
            }
        }
    }
    for (machine_name, (liveliness, machine)) in by_machine {
        if let Some(reason) = liveliness.validation_error() {
            return Err(DeployError::InvalidLiveliness {
                machine: machine_name.to_string(),
                reason,
            });
        }
        if !machine_uses_zenoh_transport(machine) {
            return Err(DeployError::InvalidLiveliness {
                machine: machine_name.to_string(),
                reason: "machine has no Zenoh transport; `liveliness:` currently \
                         requires at least one `transport: zenoh` binding or server \
                         (SCE_MESH.md §16.4 / §16.7 rows 8 & 13). SomeIP per-partition \
                         liveness is deferred to a separate landing — add a Zenoh \
                         binding/server or drop `liveliness:`"
                    .to_string(),
            });
        }
    }
    Ok(())
}

/// Returns true when at least one of the machine's bindings or its
/// server declaration selects the Zenoh transport. Used by
/// [`validate_liveliness`] to gate `liveliness:` on transport
/// compatibility — see that function's doc comment for rationale.
fn machine_uses_zenoh_transport(machine: &MachineConfig) -> bool {
    machine.bindings.values().any(|b| b.transport == "zenoh")
        || machine
            .server
            .as_ref()
            .is_some_and(|s| s.transport == "zenoh")
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

/// SCE_MESH.md §9.6 Session 4c — reject deployments whose §9.6 SOME/IP
/// scxml-invoke participants hash to the same FNV-1a low-byte service ID
/// in the SCE-reserved range `[0x8100, 0x81FF]`.
///
/// **Why this validator exists.** The §9.6 dedicated
/// `<machine>_scxml_invoke_app_` registers a vsomeip service with ID
/// derived from `crate::mesh::transport::someip::service_id_for_machine`
/// (FNV-1a 32-bit, low byte ORed with the SCE-reserved base). The 256-ID
/// projection is a known MVP boundary documented at
/// `mesh::transport::someip::service_id_for_machine`. The birthday
/// paradox crosses 50% near 16 machines, so a real deployment can hit a
/// collision long before "256 §9.6 someip peers" is reached. Without
/// this check, the colliding `(application, service)` registration is
/// silently routed by vsomeip's routing manager to whichever
/// application registered the duplicate ID first, and the operator sees
/// no log signal — wire-14/15/16/17/18/19/20 envelopes go to the wrong
/// peer with no exception.
///
/// **Participant definition (structural, deploy.yaml-only).** A machine
/// `M` participates iff either:
/// * `M.bindings` contains a peer-shape entry `#X` whose
///   `transport == "someip"`, where `X` is itself declared in
///   `topology.*.machines`; OR
/// * `M` is named as the peer `X` in such an entry from any other
///   machine.
///
/// Internal targets (`#_parent`, `#_child`) are excluded — they never
/// register a service ID. Dangling `#X` (peer not declared anywhere) is
/// excluded too: an upstream validator will reject the dangling
/// reference, and double-counting it here would surface the wrong code
/// for the same root cause.
///
/// **Deliberate over-reach.** Pure deploy.yaml structure cannot tell
/// "this machine has a `<send>` target on someip" from "this machine
/// uses §9.6 `<invoke type=\"scxml\">` over someip" — both produce the
/// same `bindings["#X"].transport: someip` shape. Treating the former as
/// a participant is a *false-positive risk*: a machine with only OEM
/// `<send>` someip bindings does not actually register a §9.6 FNV
/// service ID at codegen, so its FNV hash slot is unused. Why we
/// accept this anyway:
/// 1. The cost of false-positive is operator-actionable (rename one
///    machine, re-FNV).
/// 2. The cost of false-negative is silent runtime mis-routing on a
///    deployed system — debugging the wrong-receiver behavior requires
///    on-device reproduction.
/// 3. Realistically a machine that uses someip for `<send>` will likely
///    also use it for `<invoke type="scxml">` if the latter is wired,
///    so the false-positive rate trends toward 0.
///
/// **Multi-domain debt.** Today every §9.6 someip participant in the
/// deploy is treated as one collision domain. SCE does not yet model
/// multi-OEM `vsomeip.json` `network:` boundaries that could in
/// principle host disjoint SCE-reserved ID spaces. When such federation
/// lands the validator must accept the per-domain shape; until then the
/// single-domain assumption is the conservative trade.
///
/// **Algorithm.** Walk the deploy, collect participants into a sorted
/// set, group by `service_id_for_machine`, return the first
/// collision-bearing group as `SomeipScxmlInvokeServiceIdCollision`.
/// Determinism: `BTreeMap` iteration + sorted machine list inside each
/// diagnostic group, so the byte-stable golden hash on the first
/// collision is reproducible across machines and runs. The validator
/// returns at the first colliding group (consistent with
/// `validate_scxml_invoke_transport`'s shape) — operators fix one
/// collision, re-run to surface the next.
/// SCE Mesh RFC F.X-1 — hybrid (counter + optional author-pin) §9.6 SOMEIP
/// scxml-invoke service ID validator. Replaces
/// [`validate_someip_scxml_invoke_service_id_collisions`]'s FNV-collision
/// shape with three operationally-distinct rejection codes:
///
/// 1. **Overflow** — participant count > 128 (the invoke sub-range ceiling
///    under subsystem range partitioning).
/// 2. **Pin out-of-range** — author-pinned `someip_service_id:` falls outside
///    the §9.6 invoke sub-range `[0x8100, 0x817F]` (the upper half of the
///    SCE-reserved range is reserved for §16.4 region-liveness).
/// 3. **Pin-vs-pin collision** — two or more machines pin the same value.
///
/// Pin-vs-auto collision is impossible by construction:
/// [`crate::mesh::transport::someip::assign_invoke_service_ids`]'s counter
/// skips slots already claimed by pins.
///
/// The participant projection mirrors
/// [`validate_someip_scxml_invoke_service_id_collisions`]: a machine is a
/// participant iff it (a) declares `bindings["#X"].transport: someip` for a
/// declared peer `X` (excluding internal targets and dangling references)
/// or (b) is named as the peer `X` in such a binding from another machine.
/// Same conservative single-domain assumption — multi-OEM `vsomeip.json`
/// `network:` boundaries are a separate landing.
fn validate_someip_scxml_invoke_service_ids(cfg: &DeployConfig) -> Result<(), DeployError> {
    use crate::mesh::transport::someip::{assign_invoke_service_ids, AssignInvokeServiceIdError};

    // Same participant projection as the legacy collision validator.
    // Sharing the projection between the two validators avoids a drift
    // window where one rejects a deploy the other accepts.
    let declared_machines: std::collections::HashSet<&str> = cfg
        .topology
        .values()
        .flat_map(|d| d.machines.keys().map(|k| k.as_str()))
        .collect();

    let mut participants: std::collections::BTreeSet<String> =
        std::collections::BTreeSet::new();
    for device in cfg.topology.values() {
        for (machine_name, machine_cfg) in &device.machines {
            for (target_id, binding) in &machine_cfg.bindings {
                if binding.transport != "someip" {
                    continue;
                }
                if target_id.is_internal() {
                    continue;
                }
                let peer = target_id.name();
                if !declared_machines.contains(peer) {
                    continue;
                }
                participants.insert(machine_name.clone());
                participants.insert(peer.to_string());
            }
        }
    }

    // Build the (machine_name → optional pin) map the assigner consumes.
    // A non-participant machine's `someip_service_id:` (if any) is ignored
    // — the field carries no meaning for machines that do not register a
    // §9.6 service. Surfacing a "pin on non-participant" rejection would
    // duplicate the upstream "binding refers to non-existent peer" check,
    // so silent ignore is the right shape (consistent with the participant
    // projection the legacy validator uses).
    let mut participants_with_pins: std::collections::BTreeMap<String, Option<u16>> =
        std::collections::BTreeMap::new();
    for name in &participants {
        let pin = cfg
            .topology
            .values()
            .find_map(|d| d.machines.get(name))
            .and_then(|m| m.someip_service_id);
        participants_with_pins.insert(name.clone(), pin);
    }

    // Run the assigner. Non-error → success; map each error variant to
    // the matching DeployError variant for diagnostic continuity.
    match assign_invoke_service_ids(&participants_with_pins) {
        Ok(_) => Ok(()),
        Err(AssignInvokeServiceIdError::Overflow {
            participant_count,
            ceiling,
        }) => Err(DeployError::SomeipScxmlInvokeServiceIdOverflow {
            participant_count,
            ceiling,
        }),
        Err(AssignInvokeServiceIdError::PinOutOfRange {
            machine,
            pinned_id,
            range_lo,
            range_hi,
        }) => Err(DeployError::SomeipScxmlInvokeServiceIdPinOutOfRange {
            machine,
            pinned_id,
            range_lo,
            range_hi,
        }),
        Err(AssignInvokeServiceIdError::PinCollision {
            machines,
            pinned_id,
        }) => Err(DeployError::SomeipScxmlInvokeServiceIdPinCollision {
            machines,
            pinned_id,
        }),
    }
}

fn validate_someip_scxml_invoke_service_id_collisions(
    cfg: &DeployConfig,
) -> Result<(), DeployError> {
    use crate::mesh::transport::someip::service_id_for_machine;

    // All declared machine names — used to filter out dangling `#X`
    // peer references (those produce a different diagnostic upstream
    // and double-counting them would attribute the wrong code).
    let declared_machines: std::collections::HashSet<&str> = cfg
        .topology
        .values()
        .flat_map(|d| d.machines.keys().map(|k| k.as_str()))
        .collect();

    // Participants ordered by name (BTreeSet → deterministic iteration
    // for the diagnostic id hash + machine list).
    let mut participants: std::collections::BTreeSet<String> =
        std::collections::BTreeSet::new();

    for device in cfg.topology.values() {
        for (machine_name, machine_cfg) in &device.machines {
            for (target_id, binding) in &machine_cfg.bindings {
                if binding.transport != "someip" {
                    continue;
                }
                if target_id.is_internal() {
                    // `#_parent` / `#_child` never register a service ID.
                    continue;
                }
                let peer = target_id.name();
                if !declared_machines.contains(peer) {
                    // Dangling reference — let the upstream validator
                    // surface the absence under its own code.
                    continue;
                }
                participants.insert(machine_name.clone());
                participants.insert(peer.to_string());
            }
        }
    }

    if participants.len() < 2 {
        // Single-participant set (or empty) cannot collide.
        return Ok(());
    }

    // Group by service ID. `BTreeMap` for deterministic iteration; the
    // first colliding service ID encountered (in numeric order) is the
    // one we surface so re-runs after a partial fix continue to make
    // forward progress through the violation set in the same order.
    let mut by_service_id: std::collections::BTreeMap<u16, Vec<String>> =
        std::collections::BTreeMap::new();
    for name in &participants {
        let svc = service_id_for_machine(name);
        by_service_id.entry(svc).or_default().push(name.clone());
    }
    for (service_id, machines) in by_service_id {
        if machines.len() >= 2 {
            // `participants` was sorted by `BTreeSet`, so per-group
            // order is already lex-sorted; no additional sort needed.
            return Err(DeployError::SomeipScxmlInvokeServiceIdCollision {
                service_id,
                machines,
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
        // A Zenoh binding is required by `validate_liveliness` transport-
        // compat check — the template emits liveliness code only when
        // `"zenoh" in transport_types`, so a machine without any Zenoh
        // binding/server cannot observe the signal it opts into.
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
            key: "sce/brake/motor/ping"
        liveliness:
          lease_ms: 2000
"##;
        let cfg = parse_deploy_str(yaml).expect("parse");
        let machine = &cfg.topology["ecu1"].machines["brake"];
        assert_eq!(
            machine.liveliness.unwrap().lease_ms,
            2000,
            "explicit section must propagate the lease_ms value"
        );
    }

    #[test]
    fn liveliness_someip_only_machine_rejected() {
        // SomeIP-only machine + `liveliness:` is the silent-broken case
        // that motivated this check: the codegen template gates
        // liveliness emission on `"zenoh" in transport_types`, so the
        // handler required by `reject_liveliness_without_handler` would
        // compile but never fire. Must reject at parse time with a
        // reason naming Zenoh specifically so the author can decide to
        // either add a Zenoh binding or drop the section.
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    transports:
      someip:
        config: vsomeip.json
        application_name: brake_app
    machines:
      brake:
        source: brake.scxml
        bindings:
          "#motor":
            transport: someip
            service: motor_control
            method: activate
        liveliness:
          lease_ms: 200
"##;
        match parse_deploy_str(yaml) {
            Err(DeployError::InvalidLiveliness { machine, reason }) => {
                assert_eq!(machine, "brake");
                assert!(
                    reason.contains("Zenoh"),
                    "reason must name Zenoh specifically so the fix is discoverable: {reason}"
                );
                assert!(
                    reason.contains("SomeIP") || reason.contains("deferred"),
                    "reason should mention SomeIP deferral so authors know this is not a bug: {reason}"
                );
            }
            other => panic!("expected InvalidLiveliness (transport-compat), got {other:?}"),
        }
    }

    #[test]
    fn liveliness_zenoh_server_accepted() {
        // Server-side Zenoh registration satisfies the transport-compat
        // check — the generated router hosts a liveliness token through
        // the device's Zenoh session regardless of whether the binding
        // axis (client) or the server axis selected Zenoh.
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    transports:
      zenoh:
        mode: peer
        listen: ["tcp/127.0.0.1:17460"]
    machines:
      brake:
        source: brake.scxml
        server:
          transport: zenoh
          key: "sce/brake/rpc"
        liveliness:
          lease_ms: 200
"##;
        let cfg = parse_deploy_str(yaml).expect("zenoh server must satisfy liveliness transport-compat");
        let machine = &cfg.topology["ecu1"].machines["brake"];
        assert!(
            machine.liveliness.is_some(),
            "liveliness section must propagate when zenoh server is declared"
        );
    }

    #[test]
    fn liveliness_machine_without_bindings_rejected() {
        // Edge case: a machine that declares `liveliness:` but has
        // neither bindings nor a server has no transport surface at all,
        // so the template emits zero liveliness code. The same
        // silent-broken shape as SomeIP-only — reject with the same
        // error variant to keep the diagnostic uniform.
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        liveliness:
          lease_ms: 200
"##;
        match parse_deploy_str(yaml) {
            Err(DeployError::InvalidLiveliness { machine, reason }) => {
                assert_eq!(machine, "brake");
                assert!(
                    reason.contains("Zenoh"),
                    "binding-less machine rejection must cite Zenoh requirement: {reason}"
                );
            }
            other => panic!("expected InvalidLiveliness (no bindings), got {other:?}"),
        }
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
    fn reject_pool_machine_listed_in_partition() {
        // SCE_MESH.md §14.4 × §14 — a SOME/IP pool machine declares
        // `server.instances:` (pool = one router, N sessions, one
        // process). The moment any partition's `machines:` lists the
        // pool, the author is requesting a per-partition split that
        // deploy.yaml does not define. The fixture pairs a pooled
        // `motor` (instances: [1, 2]) with a partition that both
        // lists and owns a unit from it; the parser must reject.
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    transports:
      someip:
        config: vsomeip.json
    machines:
      motor:
        source: motor.scxml
        bindings: {}
        server:
          transport: someip
          service: motor_svc
          instances: [1, 2]

partitions:
  motor_region_a:
    device: ecu1
    machines: [motor]
    contains:
      parallel_regions:
        - { machine: motor, region: drive }
"##;
        match parse_deploy_str(yaml) {
            Err(DeployError::PartitionPoolMachine { machine, partition }) => {
                assert_eq!(machine, "motor");
                assert_eq!(partition, "motor_region_a");
            }
            other => panic!("expected PartitionPoolMachine, got {other:?}"),
        }
    }

    #[test]
    fn pool_machine_without_partition_listing_passes() {
        // Regression guard — a SOME/IP pool machine that appears in no
        // partition must parse (§14.4 pool + absent partitioning is
        // the canonical single-process pool shape).
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    transports:
      someip:
        config: vsomeip.json
    machines:
      motor:
        source: motor.scxml
        bindings: {}
        server:
          transport: someip
          service: motor_svc
          instances: [1, 2]
      brake:
        source: brake.scxml
        bindings: {}

partitions:
  brake_default:
    device: ecu1
    machines: [brake]
    contains:
      parallel_regions:
        - { machine: brake, region: monitor }
"##;
        let cfg = parse_deploy_str(yaml).expect("parse");
        assert!(cfg.partitions.is_some());
        let motor = &cfg.topology["ecu1"].machines["motor"];
        assert!(
            motor.server.as_ref().and_then(|s| s.instances.as_ref()).is_some(),
            "motor must retain its pool declaration",
        );
    }

    #[test]
    fn non_pool_machine_in_partition_passes() {
        // Regression guard — a non-pool machine listed in a partition
        // must parse under the Phase A/A' rules alone; the Gap I
        // check is load-bearing only for pool machines.
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        bindings: {}

partitions:
  brake_part:
    device: ecu1
    machines: [brake]
    contains:
      parallel_regions:
        - { machine: brake, region: monitor }
"##;
        let cfg = parse_deploy_str(yaml).expect("parse");
        assert!(cfg.partitions.is_some());
    }

    #[test]
    fn accept_partition_transport_binding_shm() {
        // §14 L2729-2730 — `shm` is the canonical same-machine IPC
        // transport. Schema must accept it end-to-end without
        // diagnostic noise.
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
    transport_binding: shm
"##;
        let cfg = parse_deploy_str(yaml).expect("parse");
        let part = cfg.partitions.expect("partitions present");
        assert_eq!(
            part.get("brake_main")
                .unwrap()
                .transport_binding
                .as_deref(),
            Some("shm")
        );
    }

    #[test]
    fn accept_partition_transport_binding_custom_tcp() {
        // §14 L2730 "kind tcp/shm" — the `tcp` half is `custom_tcp`
        // (§16.8.3 reference transport).
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
    transport_binding: custom_tcp
"##;
        let cfg = parse_deploy_str(yaml).expect("parse");
        let part = cfg.partitions.expect("partitions present");
        assert_eq!(
            part.get("brake_main")
                .unwrap()
                .transport_binding
                .as_deref(),
            Some("custom_tcp")
        );
    }

    #[test]
    fn reject_partition_transport_binding_unknown() {
        // Unknown transport name — `iceoryx2` is not in the registry.
        // §14 L2729-2730 accepts only registry-known transports whose
        // `supports_inter_partition_ipc` is true.
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
    transport_binding: iceoryx2
"##;
        match parse_deploy_str(yaml) {
            Err(DeployError::PartitionTransportBindingUnsupported {
                partition,
                transport,
                failure,
            }) => {
                assert_eq!(partition, "brake_main");
                assert_eq!(transport, "iceoryx2");
                // Structural assertion: the unknown-name path must
                // carry the registry list so the message template can
                // interpolate it without a back-reference.
                match failure {
                    PartitionTransportBindingFailure::Unknown { known_names } => {
                        assert!(
                            !known_names.is_empty(),
                            "known_names must carry the registry list"
                        );
                    }
                    PartitionTransportBindingFailure::Incapable { .. } => {
                        panic!("unknown transport must take the Unknown arm, not Incapable");
                    }
                }
            }
            other => panic!("expected PartitionTransportBindingUnsupported, got {other:?}"),
        }
    }

    #[test]
    fn reject_partition_transport_binding_local() {
        // `local` is intra-process direct dispatch; it cannot cross
        // the OS process boundary `partitions:` defines. §14 L2729
        // requires same-machine IPC, not intra-process dispatch.
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
    transport_binding: local
"##;
        match parse_deploy_str(yaml) {
            Err(DeployError::PartitionTransportBindingUnsupported {
                partition,
                transport,
                failure,
            }) => {
                assert_eq!(partition, "brake_main");
                assert_eq!(transport, "local");
                // Structural assertion: the incapable-transport path
                // carries the same transport name the outer variant
                // holds, kept for Display byte-equivalence.
                match failure {
                    PartitionTransportBindingFailure::Incapable { transport: t } => {
                        assert_eq!(t, "local");
                    }
                    PartitionTransportBindingFailure::Unknown { .. } => {
                        panic!("registered-but-incapable transport must take the Incapable arm");
                    }
                }
            }
            other => panic!("expected PartitionTransportBindingUnsupported, got {other:?}"),
        }
    }

    #[test]
    fn reject_partition_transport_binding_someip() {
        // SOME/IP is inter-machine middleware routed through the
        // vsomeip daemon; not the direct same-machine IPC channel
        // §14 L2729 intends.
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
    transport_binding: someip
"##;
        match parse_deploy_str(yaml) {
            Err(DeployError::PartitionTransportBindingUnsupported {
                partition,
                transport,
                ..
            }) => {
                assert_eq!(partition, "brake_main");
                assert_eq!(transport, "someip");
            }
            other => panic!("expected PartitionTransportBindingUnsupported, got {other:?}"),
        }
    }

    #[test]
    fn reject_partition_transport_binding_zenoh() {
        // Zenoh is an inter-machine routing fabric; not the direct
        // same-machine IPC channel §14 L2729 intends.
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
    transport_binding: zenoh
"##;
        match parse_deploy_str(yaml) {
            Err(DeployError::PartitionTransportBindingUnsupported {
                partition,
                transport,
                ..
            }) => {
                assert_eq!(partition, "brake_main");
                assert_eq!(transport, "zenoh");
            }
            other => panic!("expected PartitionTransportBindingUnsupported, got {other:?}"),
        }
    }

    #[test]
    fn accept_partition_barrier_timeout_positive() {
        // §14 L2731-2732 — finite positive values are accepted; the
        // runtime consumer (§16.5) interprets them against the
        // partition's <parallel> root hosting status.
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
    barrier_timeout_ms: 5000
"##;
        let cfg = parse_deploy_str(yaml).expect("parse");
        let part = cfg.partitions.expect("partitions present");
        assert_eq!(
            part.get("brake_main").unwrap().barrier_timeout_ms,
            Some(5000)
        );
    }

    #[test]
    fn accept_partition_barrier_timeout_absent_is_infinity() {
        // Field omission ⇒ None ⇒ W3C normative default (infinity)
        // per §14 L2732. Regression guard: the knob must stay
        // optional, and absent must deserialize as None (not 0).
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
"##;
        let cfg = parse_deploy_str(yaml).expect("parse");
        let part = cfg.partitions.expect("partitions present");
        assert_eq!(part.get("brake_main").unwrap().barrier_timeout_ms, None);
    }

    #[test]
    fn reject_partition_barrier_timeout_zero() {
        // §16.5 barrier timeout: zero would fire before the first
        // region can report `ParallelRegionDone`, unconditionally
        // raising `error.communication / PARALLEL_BARRIER_TIMEOUT`.
        // The knob exists to bound hangs, not to convert every
        // barrier into an error.
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
    barrier_timeout_ms: 0
"##;
        match parse_deploy_str(yaml) {
            Err(DeployError::PartitionBarrierTimeoutInvalid {
                partition,
                value,
                reason,
            }) => {
                assert_eq!(partition, "brake_main");
                assert_eq!(value, 0);
                assert!(reason.contains("§16.5"));
            }
            other => panic!("expected PartitionBarrierTimeoutInvalid, got {other:?}"),
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

    // ── §9.6.6 rule 3 explicit override carve-out ─────────────

    /// Author overrides a synth machine's partition by adding it to
    /// `topology.*.machines` with a source that matches the parser's
    /// sibling emission. Because the bare stem before
    /// `__sce_synth_invoke__` is an actually-declared parent machine,
    /// `validate_synth_invoke_infix` treats the entry as an explicit
    /// override and admits the deploy.
    #[test]
    fn synth_infix_admitted_when_parent_is_declared() {
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      parent:
        source: parent.scxml
      parent__sce_synth_invoke__inv0:
        source: parent__sce_synth_invoke__inv0.scxml
"##;
        let cfg = parse_deploy_str(yaml).expect("explicit override must parse");
        assert!(cfg
            .topology
            .get("ecu1")
            .unwrap()
            .machines
            .contains_key("parent__sce_synth_invoke__inv0"));
    }

    /// Infix-bearing id with no matching parent is still a typo —
    /// rejection preserved so authors cannot accidentally shadow a
    /// future synth with a hand-authored name.
    #[test]
    fn synth_infix_rejected_when_no_matching_parent() {
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      typo__sce_synth_invoke__orphan:
        source: typo__sce_synth_invoke__orphan.scxml
"##;
        let err = parse_deploy_str(yaml).expect_err("infix without parent must reject");
        match err {
            crate::mesh::error::DeployError::PartitionSynthInfixCollision { machine } => {
                assert_eq!(machine, "typo__sce_synth_invoke__orphan");
            }
            other => panic!("expected PartitionSynthInfixCollision, got {other:?}"),
        }
    }

    // ── §9.6 Session 4c: SOME/IP service-ID collision validator ──

    /// Adversarial fixture: an actual colliding pair (computed at runtime
    /// via `find_colliding_pair`, not a hard-coded magic pair) wired into
    /// a deploy.yaml whose §9.6 someip participant set is exactly those
    /// two names. The validator must reject; the diagnostic carries the
    /// shared service ID and the colliding machine names sorted.
    ///
    /// "Computed at runtime" is the key property — the test does not pin
    /// any specific FNV output. If a future drift in the FNV constants
    /// changes which short alphanumerics collide, `find_colliding_pair`
    /// re-discovers a new colliding pair and the test still exercises
    /// the validator with a real collision. The drift is caught
    /// independently by the pinned-hash tests in `transport::someip`.
    #[test]
    fn someip_service_id_collision_rejected_via_adversarial_pair() {
        use crate::mesh::transport::someip::{find_colliding_pair, service_id_for_machine};
        let (a, b) = find_colliding_pair()
            .expect("FNV-1a low-byte projection must collide in 4-char alphanumeric");
        let expected_service_id = service_id_for_machine(&a);
        // Sanity — `find_colliding_pair`'s contract is "same service ID".
        assert_eq!(expected_service_id, service_id_for_machine(&b));

        // Wire `a` and `b` as someip §9.6 peers on separate ECUs. `a`
        // declares `bindings["#b"].transport: someip` so both names
        // enter the participant set under the validator's structural
        // walk; the SCXML side is irrelevant at this layer.
        let yaml = format!(
            r##"version: "1.0"
topology:
  ecu_alpha:
    machines:
      {a}:
        source: {a}.scxml
        bindings:
          "#{b}":
            transport: someip
  ecu_beta:
    machines:
      {b}:
        source: {b}.scxml
"##
        );

        let err = parse_deploy_str(&yaml)
            .expect_err("colliding §9.6 someip participant pair must be rejected");
        match err {
            crate::mesh::error::DeployError::SomeipScxmlInvokeServiceIdCollision {
                service_id,
                machines,
            } => {
                assert_eq!(service_id, expected_service_id);
                // Machine list is sorted (BTreeSet → BTreeMap iteration);
                // the lex-smaller name lands first regardless of which
                // side carried the binding declaration.
                let mut sorted = vec![a.clone(), b.clone()];
                sorted.sort();
                assert_eq!(machines, sorted);
            }
            other => panic!("expected SomeipScxmlInvokeServiceIdCollision, got {other:?}"),
        }
    }

    /// The 4-machine fixture set used across `tests/mesh` (parent /
    /// worker / motor / brake) is collision-free under the pinned FNV
    /// hashes documented in `transport::someip`. The validator must
    /// accept this set even though every machine declares an outbound
    /// `transport: someip` binding to one of the others — the §9.6
    /// participant union is `{parent, worker, motor, brake}` and the
    /// pinned hashes (0x81fd / 0x8157 / 0x8172 / 0x8130) are all
    /// distinct.
    ///
    /// This guards the validator's `< 2` early return is not
    /// over-applied: collision check fires when the participant set
    /// has size ≥ 2 (it does, size 4) and the per-service-ID grouping
    /// must produce no group of size ≥ 2.
    #[test]
    fn someip_collision_free_set_accepted() {
        let yaml = r##"
version: "1.0"
topology:
  ecu_a:
    machines:
      parent:
        source: parent.scxml
        bindings:
          "#worker":
            transport: someip
  ecu_b:
    machines:
      worker:
        source: worker.scxml
        bindings:
          "#motor":
            transport: someip
  ecu_c:
    machines:
      motor:
        source: motor.scxml
        bindings:
          "#brake":
            transport: someip
  ecu_d:
    machines:
      brake:
        source: brake.scxml
"##;
        let cfg = parse_deploy_str(yaml).expect("collision-free 4-machine set must accept");
        // Sanity: the participant set is exactly the four declared machines.
        assert_eq!(cfg.topology.len(), 4);
    }

    /// Single §9.6 someip participant (one machine references one peer)
    /// has participant-set size 2, so the validator does run; but the
    /// two pinned names parent/worker hash to distinct service IDs. To
    /// exercise the literal "single-participant short-circuit", we use
    /// a deploy with no someip bindings at all: the participant set is
    /// empty, the validator's `< 2` early return fires, and the
    /// deployment is accepted.
    #[test]
    fn no_someip_bindings_short_circuits_collision_check() {
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
      motor:
        source: motor.scxml
"##;
        parse_deploy_str(yaml)
            .expect("zero §9.6 someip participants must short-circuit collision check");
    }

    /// Mixed transports (zenoh + someip in the same deploy) must keep
    /// collision domains separate: only someip participants enter the
    /// FNV check, zenoh peers are ignored. Even if the zenoh-bound
    /// names happen to FNV-collide, the validator must accept because
    /// they never register a §9.6 someip service ID.
    #[test]
    fn zenoh_peers_excluded_from_someip_collision_domain() {
        use crate::mesh::transport::someip::{find_colliding_pair, service_id_for_machine};
        let (a, b) = find_colliding_pair()
            .expect("FNV-1a low-byte projection must collide in 4-char alphanumeric");
        // Sanity: pair really collides.
        assert_eq!(service_id_for_machine(&a), service_id_for_machine(&b));

        // Deploy `a`, `b` as zenoh peers (not someip). Plus one someip
        // participant (parent → worker over someip) which is
        // collision-free against itself / vacuously safe alone with
        // its single peer pair. The zenoh pair must NOT enter the
        // someip collision domain.
        let yaml = format!(
            r##"version: "1.0"
topology:
  ecu_a:
    machines:
      {a}:
        source: {a}.scxml
        bindings:
          "#{b}":
            transport: zenoh
      parent:
        source: parent.scxml
        bindings:
          "#worker":
            transport: someip
  ecu_b:
    machines:
      {b}:
        source: {b}.scxml
      worker:
        source: worker.scxml
"##
        );

        parse_deploy_str(&yaml)
            .expect("zenoh-bound colliding pair must not pollute someip collision domain");
    }

    // ── RFC F.X-1: hybrid (counter + author-pin) service ID validator ──

    #[test]
    fn someip_service_id_pin_yaml_string_form_parses() {
        // Quoted hex string form: explicit, YAML-version-independent.
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        someip_service_id: "0x8105"
        bindings:
          "#worker":
            transport: someip
      worker:
        source: worker.scxml
"##;
        let cfg = parse_deploy_str(yaml).expect("string-form pin must parse");
        let machine = &cfg.topology["ecu1"].machines["brake"];
        assert_eq!(machine.someip_service_id, Some(0x8105));
    }

    #[test]
    fn someip_service_id_pin_yaml_int_form_parses() {
        // Raw decimal integer — equivalent to the string form, accepted
        // for ergonomics where authors prefer numeric YAML values.
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        someip_service_id: 33029
        bindings:
          "#worker":
            transport: someip
      worker:
        source: worker.scxml
"##;
        let cfg = parse_deploy_str(yaml).expect("int-form pin must parse");
        let machine = &cfg.topology["ecu1"].machines["brake"];
        assert_eq!(machine.someip_service_id, Some(33029)); // 0x8105
    }

    #[test]
    fn someip_service_id_pin_yaml_bare_decimal_string_rejected() {
        // `"8105"` is ambiguous (could be intended as 8105 decimal or
        // 0x8105 hex). The deserializer rejects bare hex strings — author
        // must either use the integer form or the explicit `0x` prefix.
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        someip_service_id: "8105"
        bindings:
          "#worker":
            transport: someip
      worker:
        source: worker.scxml
"##;
        let result = parse_deploy_str(yaml);
        match result {
            Err(DeployError::Yaml(reason)) => {
                assert!(
                    reason.contains("must start with `0x`"),
                    "bare-string rejection must explain the prefix requirement: {reason}"
                );
            }
            other => panic!("expected Yaml error, got {other:?}"),
        }
    }

    #[test]
    fn someip_service_id_pin_in_range_accepted() {
        // Pin inside [0x8100, 0x817F] — must round-trip through the
        // hybrid validator without rejection.
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        someip_service_id: "0x817F"
        bindings:
          "#worker":
            transport: someip
      worker:
        source: worker.scxml
"##;
        parse_deploy_str(yaml).expect("ceiling-edge pin must be accepted");
    }

    #[test]
    fn someip_service_id_pin_above_range_rejected() {
        // 0x8180 is the F.X-3 region-liveness range floor — out-of-range
        // for invoke pins.
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        someip_service_id: "0x8180"
        bindings:
          "#worker":
            transport: someip
      worker:
        source: worker.scxml
"##;
        match parse_deploy_str(yaml) {
            Err(DeployError::SomeipScxmlInvokeServiceIdPinOutOfRange {
                machine,
                pinned_id,
                range_lo,
                range_hi,
            }) => {
                assert_eq!(machine, "brake");
                assert_eq!(pinned_id, 0x8180);
                assert_eq!(range_lo, 0x8100);
                assert_eq!(range_hi, 0x817F);
            }
            other => panic!("expected SomeipScxmlInvokeServiceIdPinOutOfRange, got {other:?}"),
        }
    }

    #[test]
    fn someip_service_id_pin_below_range_rejected() {
        // 0x80FF is one below the SCE-reserved range floor — collides
        // with OEM-owned [0x0000, 0x80FF] space.
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        someip_service_id: "0x80FF"
        bindings:
          "#worker":
            transport: someip
      worker:
        source: worker.scxml
"##;
        match parse_deploy_str(yaml) {
            Err(DeployError::SomeipScxmlInvokeServiceIdPinOutOfRange {
                machine,
                pinned_id,
                ..
            }) => {
                assert_eq!(machine, "brake");
                assert_eq!(pinned_id, 0x80FF);
            }
            other => panic!("expected SomeipScxmlInvokeServiceIdPinOutOfRange, got {other:?}"),
        }
    }

    #[test]
    fn someip_service_id_pin_collision_rejected() {
        // Two participating machines pin the same value — author error,
        // operator must repick one pin.
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        someip_service_id: "0x8105"
        bindings:
          "#worker":
            transport: someip
      worker:
        source: worker.scxml
        someip_service_id: "0x8105"
"##;
        match parse_deploy_str(yaml) {
            Err(DeployError::SomeipScxmlInvokeServiceIdPinCollision {
                machines,
                pinned_id,
            }) => {
                assert_eq!(pinned_id, 0x8105);
                // BTreeMap lex order: brake < worker.
                assert_eq!(machines, vec!["brake".to_string(), "worker".to_string()]);
            }
            other => panic!("expected SomeipScxmlInvokeServiceIdPinCollision, got {other:?}"),
        }
    }

    #[test]
    fn someip_service_id_pin_on_non_participant_silently_ignored() {
        // A machine with `someip_service_id:` but no SOMEIP binding is not
        // a §9.6 invoke participant. The pin carries no meaning and the
        // validator silently ignores it — matching the participant
        // projection of the legacy collision validator. The participant
        // projection (zero participants here) keeps the validator from
        // surfacing a "pin on non-participant" rejection that would
        // duplicate upstream binding-shape checks.
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        someip_service_id: "0x8180"
"##;
        // 0x8180 would be out-of-range for an invoke participant, but
        // brake is not a participant (no SOMEIP binding). Accept.
        parse_deploy_str(yaml).expect("non-participant pin must be silently ignored");
    }
}
