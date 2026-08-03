// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh deploy.yaml parser — topology, device-level shared transport
// config, per-target bindings, scheduler.
//
// Schema shape (SCE_MESH.md §mesh-14):
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

use crate::forge::model::LinkClass;
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
    /// Device → `DeviceConfig` map.
    pub topology: HashMap<String, DeviceConfig>,
    /// Reserved `discovery:` top-level key. Parsed as opaque `Value` so
    /// the parse-time validator can surface a spec-linked diagnostic
    /// instead of the generic `deny_unknown_fields` error. SCE Mesh §mesh-3.3
    /// is the invariant: transport-native routing is the source of truth
    /// for peer availability; SCE does not maintain a peer table, and
    /// the §mesh-13 rejected list — which rejects SCE-maintained peer tables
    /// and a `discovery.mode: static | dynamic` deploy switch — holds
    /// unconditionally. For per-binding runtime target
    /// selection use value-field placeholders (§mesh-14.4); for
    /// transport-level peer discovery configure external OEM config
    /// (zenoh.json5 scouting, vsomeip.json service-discovery).
    pub discovery: Option<serde_yaml_ng::Value>,
    /// Aggressive-distribution partition declarations (SCE_MESH.md §mesh-14
    /// "Partition resolution rules" + §mesh-16). A machine whose name does
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
    /// [`DeployError::PartitionDuplicateName`] (§mesh-14 rule 6).
    #[serde(default)]
    pub partitions: Option<PartitionMap>,
    /// SCE_MESH.md §mesh-16.3 — strict vs permissive distributability
    /// mode. `strict` fails the build on any R1/R2 violation;
    /// `permissive` (the absent-value default) auto-merges offending
    /// regions per §mesh-16.4 and records a [`crate::mesh::distributability::MergeNotice`].
    /// The knob is meaningful only when `partitions:` is present; an
    /// absent key means "permissive".
    #[serde(default)]
    pub distributability: Option<DistributabilityMode>,
    /// SCE Protocol-Synthesis RFC §synth-5-I lines 1761-1764 — target-plugin path
    /// pointer for `<sce:extern>` whitelist extension: a path-pointed
    /// YAML file (loaded via
    /// [`crate::forge::target_plugin::parse_target_plugin_yaml`]),
    /// single plugin per deploy. Plugin entries
    /// extend the §synth-5-I baseline registry; baseline-shadowing
    /// surfaces as `extern/target-plugin-symbol-conflict` at plugin
    /// load time.
    ///
    /// Absent ⇒ baseline-only registry (the deploy-unaware default).
    /// Consumer-gated plugin-extension axes ride through the same
    /// field's reserved keys (`linker_flavor`,
    /// `fuzz_coverage_transport`); the plugin file itself accepts
    /// these forward-compat slots so today's sce-build can load a
    /// plugin authored for a later extension without a schema bump.
    #[serde(default)]
    pub extern_symbols: Option<ExternSymbolsConfig>,

    /// Variant-default overlay — consumer-shaped default
    /// arm choice for `<sce:variant>` peek-byte dispatch.
    ///
    /// SCE-side SCXMLs declare wire-spec invariants only — the bit
    /// positions and MID values of each codec's header are wire facts
    /// shared by every consumer. But the *choice* of which arm a
    /// freshly-constructed `Default::default()` instance dispatches to
    /// is a per-consumer convention: a zenoh client may default a
    /// request to query (0x03), a zenoh router may default to push
    /// (0x1d), and neither choice contradicts the wire spec.
    ///
    /// `variant_defaults` carries this per-codec convention out of the
    /// SCXML and into the deploy overlay. Map keys are codec names
    /// (matching the SCXML root `name="..."`); values are the chosen
    /// arm's discriminator value (matching a declared `<sce:arm
    /// value="X"/>`).
    ///
    /// Resolution order at codegen time:
    ///   1. If `variant_defaults` names this codec, that arm wins.
    ///   2. Otherwise the SCXML's `<sce:arm default="true"/>` marker
    ///      wins (Atomic α-γ legacy path — unchanged).
    ///   3. Otherwise `codec/variant-no-default-arm` fires.
    ///
    /// Codec names listed here that do not exist in the doc set fire
    /// `codec/variant-default-overlay-codec-not-found` at deploy
    /// validation time. Absent ⇒ legacy path only, all existing
    /// fixtures and consumers compile unchanged.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub variant_defaults: BTreeMap<String, u64>,
}

/// `extern_symbols:` block in deploy.yaml. Today carries one
/// field; `ordering_default` (spec line 1851 and the cross-core inbox
/// companion `worker/inbox-ordering-*` family) is consumer-gated.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternSymbolsConfig {
    /// Path (deploy-relative or absolute) to the target plugin YAML
    /// file extending the §synth-5-I whitelist. Spec line 1761-1762
    /// verbatim: `extern_symbols.target_plugin: <path>`.
    pub target_plugin: Option<PathBuf>,
}

/// SCE_MESH.md §mesh-16.3 strict/permissive toggle. Default is
/// [`DistributabilityMode::Permissive`] so authors who author a
/// partition plan that happens to violate R1/R2 still get a
/// minimum-merge build rather than a hard failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum DistributabilityMode {
    Strict,
    #[default]
    Permissive,
}

// SCE_MESH.md §mesh-14 rules 6-10 — partitions schema.
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
    /// (§mesh-16.4 resolver) have already walked the original
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
/// invokes (SCE_MESH.md §mesh-14).
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
    /// machine. Defaults handled at codegen time per SCE_MESH.md §mesh-14
    /// rule 4 (shm for single-device, custom_tcp otherwise).
    #[serde(default)]
    pub transport_binding: Option<String>,
    /// Per-partition parallel-final barrier timeout (SCE_MESH.md
    /// §mesh-16.5). `None` means "use the W3C normative default"
    /// (infinity). Only meaningful on partitions hosting the root of
    /// a `<parallel>`.
    #[serde(default)]
    pub barrier_timeout_ms: Option<u32>,
    /// Distributed `<parallel>`s this partition claims as the root
    /// (SCE_MESH.md §mesh-14 rule 12, L2729-2735). Each entry names a
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
    /// Author-pinned vsomeip `service_t` for this partition's §mesh-16.4
    /// region-partition liveness service (RFC F.X-3 D3). Optional —
    /// when `None`, the F.X-3 assigner
    /// ([`crate::mesh::transport::someip::assign_liveness_service_ids`])
    /// auto-assigns the lowest unreserved slot in lex order from
    /// [`SCXML_LIVENESS_SERVICE_BASE`]
    /// (`crate::mesh::transport::someip::SCXML_LIVENESS_SERVICE_BASE`).
    /// Pinned values are validated to fall inside the F.X-3 sub-range
    /// `[0x8180, 0x81FF]` and to be unique across partitions; pin-vs-auto
    /// collision is impossible by construction (counter skips reserved
    /// slots).
    ///
    /// Use case: pin the IDs of long-lived partitions whose Wireshark
    /// captures or cross-team contracts depend on a stable service ID.
    /// New auto-assigned partitions will not shift pinned ones.
    ///
    /// Lives on the partition (not the machine) because the participant
    /// key `<machine>__P__<partition>` is partition-grained — two
    /// partitions of the same machine need distinct service IDs.
    #[serde(default, deserialize_with = "deserialize_someip_liveness_service_id")]
    pub someip_liveness_service_id: Option<u16>,
}

/// Custom deserializer for [`PartitionDecl::someip_liveness_service_id`].
/// Parallel to [`deserialize_someip_service_id`] for §mesh-16.4 region-liveness
/// pins (RFC F.X-3). Accepts both YAML integer literals and quoted hex
/// strings; bare hex strings without `0x` prefix are rejected.
fn deserialize_someip_liveness_service_id<'de, D>(deserializer: D) -> Result<Option<u16>, D::Error>
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
            let trimmed = if let Some(rest) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X"))
            {
                rest
            } else {
                return Err(serde::de::Error::custom(format!(
                    "someip_liveness_service_id: hex string '{s}' must start with `0x` \
                     (e.g. `\"0x8185\"`); raw decimal integers are also accepted \
                     (e.g. `33157`) but bare hex strings without the prefix are \
                     rejected to avoid `0x8185` vs `8185` confusion"
                )));
            };
            u16::from_str_radix(trimmed, 16).map(Some).map_err(|e| {
                serde::de::Error::custom(format!(
                    "someip_liveness_service_id: cannot parse hex literal '{s}' as u16: {e} \
                     (expected `0x8180`-style hex inside [0x0000, 0xFFFF])"
                ))
            })
        }
    }
}

/// One entry under `partitions.<name>.hosts_parallel_roots:` — a
/// `(machine, parallel)` pair naming the `<parallel>` this partition
/// claims as the root (SCE_MESH.md §mesh-14 rule 12). `parallel` is the
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
/// invokes, both of which are distribution axes per §mesh-16.3 + §mesh-14.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartitionContains {
    /// Child `<state>` IDs directly under a `<parallel>`.
    #[serde(default)]
    pub parallel_regions: Vec<PartitionUnitRef>,
    /// `<invoke>` IDs (including synthesized `__sce_synth_invoke__*`
    /// machines from §mesh-9.6.6).
    #[serde(default)]
    pub invokes: Vec<PartitionInvokeRef>,
}

/// A parallel-region unit reference — the (machine, region) pair is
/// the §mesh-14 rule 8 uniqueness key.
#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Hash)]
#[serde(deny_unknown_fields)]
pub struct PartitionUnitRef {
    /// SCXML machine name (deploy.yaml `machines.<name>` key).
    pub machine: String,
    /// `<state id>` of the region (direct child of `<parallel>`).
    pub region: String,
}

/// An invoke unit reference — the (machine, invoke) pair is the §mesh-14
/// rule 8 uniqueness key.
#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Hash)]
#[serde(deny_unknown_fields)]
pub struct PartitionInvokeRef {
    /// SCXML machine name hosting the invoke site.
    pub machine: String,
    /// `<invoke id>` of the invoke. May be a synthesized
    /// `<parent>__sce_synth_invoke__<id>` identifier per §mesh-9.6.6.
    pub invoke: String,
}

/// Per-machine platform classification (SCE Mesh §mesh-14, SCE Protocol-Synthesis
/// RFC §synth-5-K). The class axis chooses between MCU-class targets (small,
/// bare-metal / RTOS, no general-purpose OS) and AP-class targets
/// (Linux/QNX/macOS/FreeBSD/Windows). The class gates downstream
/// codegen-matrix decisions (e.g. only `class: mcu` admits the C11
/// backend's MCU-only kinds — see RFC §synth-5-J-4 / §synth-5-J-5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PlatformClass {
    /// Application processor: Linux / QNX / macOS / FreeBSD / Windows host.
    Ap,
    /// Microcontroller-class target: bare-metal or RTOS, no general-purpose OS.
    Mcu,
}

/// Per-machine OS axis (SCE Mesh §mesh-14, SCE Protocol-Synthesis RFC §synth-5-K).
///
/// Authored values are gated against `class` by
/// [`validate_platform_class_os_consistency`]: when `class: mcu`, only
/// `bare_metal` / `rtos` are admitted; when `class: ap`, only the
/// general-purpose OS values are admitted. The split mirrors the RFC §synth-7
/// rollout (bare_metal / MCU is the foundation target; linux / qnx land
/// with §synth-7 items D.1 / D.2; the remaining AP slots are reserved for
/// items E.1-E.3) without hard-coding rollout order into the schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OsKind {
    BareMetal,
    Rtos,
    Linux,
    Qnx,
    Macos,
    Freebsd,
    Windows,
}

impl OsKind {
    /// Human-readable token used in diagnostics. Matches the YAML serde rename.
    pub fn as_str(self) -> &'static str {
        match self {
            OsKind::BareMetal => "bare_metal",
            OsKind::Rtos => "rtos",
            OsKind::Linux => "linux",
            OsKind::Qnx => "qnx",
            OsKind::Macos => "macos",
            OsKind::Freebsd => "freebsd",
            OsKind::Windows => "windows",
        }
    }
}

impl PlatformClass {
    pub fn as_str(self) -> &'static str {
        match self {
            PlatformClass::Ap => "ap",
            PlatformClass::Mcu => "mcu",
        }
    }

    /// Returns `true` iff `os` is admissible under this class.
    /// Single source of truth for [`validate_platform_class_os_consistency`].
    pub fn admits_os(self, os: OsKind) -> bool {
        match self {
            PlatformClass::Mcu => matches!(os, OsKind::BareMetal | OsKind::Rtos),
            PlatformClass::Ap => matches!(
                os,
                OsKind::Linux | OsKind::Qnx | OsKind::Macos | OsKind::Freebsd | OsKind::Windows
            ),
        }
    }
}

/// Per-machine platform descriptor (SCE Mesh §mesh-14, SCE Protocol-Synthesis RFC
/// §synth-5-K). Captures the target's class/OS plus cache and core-count
/// invariants the codegen-matrix walker (RFC §synth-5-J-4 / §synth-5-J-5) and the
/// §synth-5-E cache-policy validator consume.
///
/// Field-specific numeric checks (e.g. `dcache_line_size` power-of-2 or
/// `has_speculative_prefetch` REQUIRED when `has_dcache=true`) are not
/// enforced here at schema time — each lands alongside its codegen
/// consumer per the RFC §synth-7 sequence. The single
/// invariant validated at parse time is `class` ↔ `os` consistency
/// (`validate_platform_class_os_consistency`), which is intrinsic to
/// the schema rather than a downstream codegen rule.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformConfig {
    /// Target class. Required when the section is present.
    pub class: PlatformClass,
    /// Target OS. Required when the section is present.
    pub os: OsKind,
    /// `true` when the target core has a data cache. Drives §synth-5-E
    /// cache-maintenance emission (cache invalidate before DMA-RX,
    /// flush after DMA-TX). Optional at parse time; consumer-specific
    /// follow-up rules (e.g. `has_speculative_prefetch` REQUIRED when
    /// `has_dcache=true` on M7+ class cores) land with the codegen
    /// consumer per RFC §synth-5-K.
    #[serde(default)]
    pub has_dcache: Option<bool>,
    /// Cache-line granularity in bytes (e.g. 32, 64). Consumed by
    /// §synth-5-E cache-maintenance emission. Optional at parse time.
    #[serde(default)]
    pub dcache_line_size: Option<u32>,
    /// `true` for cores with speculative load / hardware prefetcher
    /// (Cortex-M7, M85, A-class). Drives §synth-5-E pre-DMA-RX invalidate
    /// emission. Optional at parse time.
    #[serde(default)]
    pub has_speculative_prefetch: Option<bool>,
    /// Number of cores; enables cross-core ordering checks in later
    /// phases. Optional at parse time.
    #[serde(default)]
    pub core_count: Option<u32>,
    /// Core clock frequency in MHz (SCE Protocol-Synthesis RFC §synth-5-K line 2185).
    /// Drives the stage-copy WCET formula (`expected_p99_bytes ×
    /// memcpy_cycles_per_byte / clock_freq_mhz`) gated by
    /// `reassembly/stage-copy-wcet-exceeds-slot-budget` (RFC §synth-5-M line
    /// 2995, §synth-5-M reassembly consumer) and the §synth-5-B aggregate WCET
    /// roll-up. Optional at parse time; the reassembly cross-doc
    /// validators require it when a reassembly-variant buffer pool is
    /// bound to a link on this machine.
    #[serde(default)]
    pub clock_freq_mhz: Option<u32>,
    /// Per-target memcpy cost in cycles-per-byte (SCE Protocol-Synthesis RFC
    /// §synth-5-K line 2188-2192). Architecture defaults per spec:
    /// M0/M0+ = 4.0, M3/M4 = 2.0, M7 = 1.0, A-class = 0.5. Used by the
    /// §synth-5-M `reassembly/stage-copy-wcet-exceeds-slot-budget` consumer
    /// alongside `clock_freq_mhz`. Optional at parse time;
    /// consumer-side validators raise when missing AND a
    /// reassembly-variant pool is bound.
    #[serde(default)]
    pub memcpy_cycles_per_byte: Option<f32>,
    /// Per-byte VLE decode cost (SCE Protocol-Synthesis RFC §synth-5-K line 2193-2200).
    /// Architecture defaults per spec: M0/M0+ = 12.0, M3/M4 = 8.0,
    /// M7 = 6.0, A-class = 3.0. REQUIRED at the §synth-5-B aggregate WCET
    /// consumer when any codec on the deploy contains a `vle_*` field
    /// AND `scheduler.kind=cooperative`. Optional at parse time
    /// (presence enforced by §synth-5-B consumer when load-bearing).
    #[serde(default)]
    pub vle_decode_cycles_per_byte: Option<f32>,
    /// Fixed cost per TLV chain entry in microseconds (SCE Protocol-Synthesis
    /// RFC §synth-5-K line 2201-2208). id-byte + length VLE + dispatch.
    /// Architecture defaults per spec: M0/M0+ = 1.5, M3/M4 = 0.8,
    /// M7 = 0.5, A-class = 0.2. REQUIRED at §synth-5-B aggregate WCET when
    /// any codec on the deploy contains a `tlv-chain` AND
    /// `scheduler.kind=cooperative`. Optional at parse time.
    #[serde(default)]
    pub tlv_chain_per_entry_overhead_us: Option<f32>,
    /// SCE Protocol-Synthesis RFC §synth-5-O — escalation flag for the
    /// `traceability/symbol-name-exceeds-c-identifier-limit`
    /// diagnostic. Default `None` = warn-only (the sourcemap still
    /// emits, the long identifier still ships to downstream compilers
    /// that allow >31 chars). `Some(true)` = hard-error: codegen
    /// refuses to write any artifact when any mangled symbol exceeds
    /// the C99 §5.2.4.1 external-identifier limit. Authors needing
    /// strict ANSI-C portability set this; targets with relaxed
    /// linkers (modern GCC/Clang/MSVC all allow much longer) leave
    /// the default.
    #[serde(default)]
    pub strict_c99_identifiers: Option<bool>,
    /// SCE Protocol-Synthesis RFC §5.2 — search root for `<sce:driver
    /// href="..."/>` resolution. When set, `href` values are resolved
    /// relative to this directory; otherwise resolution falls back to
    /// the SCXML file's parent directory. Optional at parse time;
    /// consumed by the driver-header resolver at compile-model time
    /// alongside `mcu/driver-header-not-found`.
    #[serde(default)]
    pub driver_root: Option<String>,
    /// SCE Protocol-Synthesis RFC §5.2 — C11-backend-only linker
    /// section attribute injection. When `class` is set, every emitted
    /// statechart function definition is prefixed with
    /// `__attribute__((section("<class>")))`. When `driver` is set,
    /// the same is intended for emitted driver-glue functions
    /// (consumer-gated; only the `class` half is emitted today).
    /// Non-C11 backends reject this section with
    /// `mcu/section-attribute-on-non-mcu-target` (the same non-MCU
    /// reject pattern as `<sce:extern>`). Only the GCC / Clang / Keil
    /// common `__attribute__((section("...")))` syntax is emitted;
    /// IAR's `@".name"` placement syntax is consumer-gated.
    #[serde(default)]
    pub c11_section_attribute: Option<C11SectionAttribute>,
}

/// SCE Protocol-Synthesis RFC §5.2 — C11 linker section attribute
/// injection knobs. Set on `machines.<n>.platform.c11_section_attribute`
/// in `deploy.yaml`. `class` controls statechart function placement;
/// `driver` is reserved for the driver-glue half — parsed but
/// not yet consumed by codegen (consumer-gated).
///
/// Only the GCC / Clang / Keil common `__attribute__((section("...")))`
/// syntax is emitted. IAR's `@".name"` placement syntax is
/// consumer-gated. Non-C11 backends (cpp / rust / kotlin / go / python)
/// raise `mcu/section-attribute-on-non-mcu-target` when this section is
/// present, matching the `<sce:extern>` non-MCU reject pattern.
#[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct C11SectionAttribute {
    /// Section name for statechart class functions. When set, every
    /// emitted statechart function definition is prefixed with
    /// `__attribute__((section("<class>")))`. Examples: `.app_code`,
    /// `.text.statechart`.
    #[serde(default)]
    pub class: Option<String>,
    /// Section name for driver-glue functions. Parsed today but
    /// its codegen consumer (driver-glue emission) is consumer-gated.
    /// Reserved.
    #[serde(default)]
    pub driver: Option<String>,
}

/// Trust-class enum for `machines.<n>.links.<name>.domain_attrs.trust_class`
/// (SCE Protocol-Synthesis RFC §synth-5-K line 2265 + §synth-5-M line 2716-2732). Three
/// values determine what traffic the link may carry and whether
/// reassembly pools may bind to it:
/// - `untrusted` — Scout / Hello only (small, never fragmented). Pool
///   binding **forbidden**.
/// - `session_arming` — INIT / OPEN handshake (small). Pool binding
///   **forbidden**.
/// - `established_session` — Frame / data plane (may fragment). Pool
///   binding **required** for reassembly.
///
/// The parser carries the enum; the reassembly validators (`reassembly/
/// untrusted-link-binding` + `reassembly/trust-class-missing-on-fragmenting-link`)
/// consume it at cross-doc resolution time. No
/// default; required when `domain_attrs` is declared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustClass {
    /// Scout / Hello traffic only. Reassembly-pool binding raises
    /// `reassembly/untrusted-link-binding` (RFC §synth-5-M line 2964).
    Untrusted,
    /// INIT / OPEN handshake traffic. Anti-flood fields apply.
    /// Reassembly-pool binding raises `reassembly/untrusted-link-binding`.
    SessionArming,
    /// Post-handshake Frame / data plane traffic. ONLY trust class
    /// eligible for reassembly-pool binding (RFC §synth-5-M line 2731).
    EstablishedSession,
}

impl TrustClass {
    /// Snake-case wire label matching the `#[serde(rename_all = "snake_case")]`
    /// rendition. Used by diagnostic message text + `actual` payload so
    /// the wire form stays stable across Rust edition / `Debug`-impl
    /// changes. The reassembly cross-doc validator emits this
    /// in the `actual` field of `reassembly/untrusted-link-binding`.
    pub fn as_str(self) -> &'static str {
        match self {
            TrustClass::Untrusted => "untrusted",
            TrustClass::SessionArming => "session_arming",
            TrustClass::EstablishedSession => "established_session",
        }
    }
}

/// Explicit cross-document listener-role declaration for a
/// deploy.yaml link.
/// Decouples the historic implicit "trust_class: session_arming =
/// listener" claim into an explicit named-role contract that pairs
/// with the SCXML-side `<sce:session-role kind="..."/>` declaration.
///
/// Top-level peer of `bind`, `driver`, `mtu_bytes`, `domain_attrs`
/// (NOT nested under `domain_attrs` so the
/// role and trust-tier remain conceptually distinct fields).
///
/// Cardinality: optional. Default `None` = "this link does not
/// participate in a listener-pair role" (the common case for plain
/// `established_session` links and for non-session-FSM machines).
/// The parser captures the field; `crate::resolve_listener_links`
/// joins it with the SCXML-side declaration and
/// `crate::validate_cross_doc_listener_roles` enforces the
/// partial-claim contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkRole {
    /// Pre-handshake listener half of the session-FSM accept-side
    /// pair. Pairs with a machine SCXML that declares
    /// `<sce:session-role kind="accept-side"/>`. Per the
    /// role/trust-class matrix, a
    /// `Listener` role on a link with `trust_class != session_arming`
    /// fires the typed validator
    /// `link/role-listener-with-non-session-arming-trust-class`.
    Listener,
    /// Initiator-side wire role. v1 is forward-compat — declaring it
    /// captures the intent on the model but the orchestrator silently
    /// passes it (no codegen path consumes the variant yet). Lands
    /// when a reachable consumer requires initiator-side sibling
    /// synthesis.
    Initiator,
}

impl LinkRole {
    /// Snake-case wire label. Used in diagnostic `actual` / `expected`
    /// payloads (the listener-role partial-claim codes) and in
    /// deploy.yaml deserialization.
    pub fn as_str(self) -> &'static str {
        match self {
            LinkRole::Listener => "listener",
            LinkRole::Initiator => "initiator",
        }
    }
}

/// RX-dispatch policy for `machines.<n>.links.<name>.rx_dispatch`
/// (SCE Protocol-Synthesis RFC §synth-5-K line 2254-2262).
///
/// - `isr_to_pool` — RX-complete IRQ immediately re-arms next slot
///   from descriptor ring (wire-rate absorption). Required when
///   `burst_pps` declared.
/// - `worker_tick` — RX only progresses on cooperative tick (simpler,
///   lower wire-rate ceiling).
///
/// Conditional default per spec line 2261: `IsrToPool`
/// when `burst_pps` declared, `WorkerTick` otherwise. Applied at the
/// field-resolver layer (post-parse), not parser-tier — same pattern
/// as the `WorkerPlacementConfig` populator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RxDispatch {
    /// IRQ-driven RX, descriptor ring re-arm. Wire-rate absorption.
    IsrToPool,
    /// Cooperative-tick-driven RX. Lower wire-rate ceiling, simpler
    /// driver. Default when `burst_pps` is absent.
    WorkerTick,
}

/// Machine-wide stage-copy policy enum (SCE Protocol-Synthesis RFC §synth-5-K
/// lines 2351-2369). Drives the policy promotion of
/// `reassembly/expected-fragmentation-rate-high` (warning under
/// `warn`) to `pool/stage-copy-policy-error` (hard error under
/// `error` / `forbid`), and gates the per-link `<sce:accept-stage-
/// copy-rate>` opt-out (still allowed under `warn` / `error`,
/// rejected outright under `forbid` via `pool/stage-copy-accept-
/// rejected-under-forbid`).
///
/// Default = `Warn` (spec line 2351 default). The default applies
/// only when both `pool_defaults` and `pool_defaults.stage_copy_policy`
/// are present in declaration syntax but treated as their literal
/// default; absence of `pool_defaults` entirely keeps the validator's
/// pre-`pool_defaults` behavior unchanged via [`MachineConfig::resolved_stage_copy_policy`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StageCopyPolicy {
    /// Spec line 2352-2357: default. §synth-5-M / ARCHITECTURE §9.3
    /// stage-copy-rate gate emits `reassembly/expected-fragmentation-
    /// rate-high` as a warning; the per-link
    /// `<sce:accept-stage-copy-rate>` opt-out suppresses it.
    Warn,
    /// Spec line 2358-2361: warning is promoted to hard error
    /// (`pool/stage-copy-policy-error`). The per-link opt-out is
    /// still honored.
    Error,
    /// Spec line 2362-2367: same promotion as `Error` AND
    /// `<sce:accept-stage-copy-rate>` itself is rejected via
    /// `pool/stage-copy-accept-rejected-under-forbid`. For
    /// safety-critical deploys (medical / automotive / aerospace)
    /// where any stage-copy is unacceptable.
    Forbid,
}

impl StageCopyPolicy {
    /// Wire-format label matching the `#[serde(rename_all = "snake_case")]`
    /// rendition. Used by diagnostic payload + Fix::ReplaceOneOf
    /// candidate list of `deploy/stage-copy-policy-unknown` so the
    /// closed-set the spec names is the exact set the diagnostic
    /// surfaces.
    pub fn as_str(self) -> &'static str {
        match self {
            StageCopyPolicy::Warn => "warn",
            StageCopyPolicy::Error => "error",
            StageCopyPolicy::Forbid => "forbid",
        }
    }
    /// Closed-set repair candidates for
    /// `deploy/stage-copy-policy-unknown`. Spec line 2518-2519 lists
    /// {warn, error, forbid} verbatim — this method is the single
    /// source of truth for the diagnostic's `Fix::ReplaceOneOf`.
    pub const ALL: &'static [&'static str] = &["warn", "error", "forbid"];
}

/// Machine-wide pool-defaults block (SCE Protocol-Synthesis RFC §synth-5-K
/// lines 2350-2369). Today carries only `stage_copy_policy`; further
/// pool-default fields are consumer-gated and land here additively
/// (each gated on its consumer per `[[feedback-silently-broken-hooks]]`).
///
/// `#[serde(deny_unknown_fields)]` parse-rejects unknown nested keys
/// — future fields (`cache_default_policy`, `dma_alignment_floor`,
/// etc.) must land alongside their consumers, not as anticipatory
/// schema entries.
///
/// **Schema type for `stage_copy_policy`**: `String` (not the typed
/// `StageCopyPolicy` enum) — mirrors the `LinkConfig::driver: String`
/// precedent for closed-set fields. Serde would reject unknown enum
/// values via a generic "unknown variant" error before
/// `validate_pool_defaults` could fire the spec-named
/// `deploy/stage-copy-policy-unknown` diagnostic; the String shape
/// lets the post-parse validator surface that typo guard verbatim
/// (RFC §synth-5-K line 2517-2519).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PoolDefaults {
    /// Spec line 2351: warn (default) | error | forbid.
    /// Validated by [`validate_pool_defaults`] which fires
    /// `deploy/stage-copy-policy-unknown` on closed-set miss; typed
    /// access via [`MachineConfig::resolved_stage_copy_policy`] which
    /// runs `StageCopyPolicy::from_wire_str` (infallible after parse-
    /// time validation per the contract).
    #[serde(default = "default_stage_copy_policy_str")]
    pub stage_copy_policy: String,
}

fn default_stage_copy_policy_str() -> String {
    "warn".to_string()
}

impl StageCopyPolicy {
    /// Inverse of [`Self::as_str`] — maps the validated wire string
    /// back to the typed enum. Returns `None` on unknown values;
    /// callers downstream of [`validate_pool_defaults`] should never
    /// hit `None` since that validator parse-rejects unknown values
    /// before this conversion is ever invoked.
    pub fn from_wire_str(s: &str) -> Option<Self> {
        match s {
            "warn" => Some(StageCopyPolicy::Warn),
            "error" => Some(StageCopyPolicy::Error),
            "forbid" => Some(StageCopyPolicy::Forbid),
            _ => None,
        }
    }
}

/// HMAC primitive used by the `stateless_accept` cookie scheme
/// (SCE Protocol-Synthesis RFC §synth-5-K lines 2325-2330). Today's MVP variant is
/// `cookie_hmac_sha256`; alternative primitives (e.g. Blake2s for
/// SoCs without SHA-256 acceleration) land as new enum values when
/// the need is concrete per spec line 2326-2330 — not preemptively.
/// Serde rejects unknown values with the generic "unknown variant"
/// error, which surfaces as `DeployError::Yaml` at parse time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HmacMode {
    /// HMAC-SHA-256 truncated to 32 bytes (spec line 2325).
    CookieHmacSha256,
}

/// Per-peer tracking table parameters (SCE Protocol-Synthesis RFC §synth-5-K line
/// 2460-2462 + §synth-5-M lines 2705-2706). Author-declared capacity of the
/// peer-tracking table the FSM maintains for anti-flood and per-peer
/// quota accounting. C13 deferred-2 carries only `capacity`; future
/// per-peer parameters (e.g. eviction policy) land alongside their
/// consumers per [[feedback-silently-broken-hooks]].
///
/// Spec dot-notation `peer_table.capacity` is mirrored as a nested
/// sub-struct so authors and the spec text share one schema shape.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PeerTable {
    /// Spec line 2460-2462 + §synth-5-M lines 2705-2706 — capacity of the
    /// per-peer tracking table. Build invariant per §synth-5-K line
    /// 2460-2462: `session_arming_quota × max_handshake_time_s ≤
    /// peer_table.capacity` (else
    /// `deploy/session-arming-quota-vs-peer-table-invariant-violated`
    /// fires). Required when the `peer_table` block is declared.
    pub capacity: u32,
}

/// Stateless-accept cookie scheme block (SCE Protocol-Synthesis RFC §synth-5-K
/// lines 2320-2349). REQUIRED when `LinkDomainAttrs.untrusted_source`
/// is `true`; optional but recommended on `trust_class:
/// session_arming` links facing >0 untrusted peers.
///
/// `#[serde(deny_unknown_fields)]` parse-rejects unknown nested keys
/// — future fields land alongside their consumers per
/// `[[feedback-silently-broken-hooks]]`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatelessAccept {
    /// Spec line 2325 — single MVP variant, see [`HmacMode`].
    pub mode: HmacMode,
    /// Spec line 2331-2333 — cookie validity window in milliseconds.
    /// After this, an echoing OpenSyn is silently rejected (runtime
    /// emits `session/cookie-rejected` with `reason=expired`).
    pub cookie_lifetime_ms: u32,
    /// Spec line 2334-2338 — HMAC key rotation interval in seconds.
    /// Previous key honored for one additional `cookie_lifetime_ms`
    /// window after rotation. Build-time invariant per spec line
    /// 2470-2473: `key_rotation_s × 1000 > 2 × cookie_lifetime_ms`
    /// (else `stateless-accept-key-rotation-shorter-than-lifetime`
    /// fires).
    pub key_rotation_s: u32,
    /// Spec line 2339-2344 — symbol name from the §synth-5-I baseline
    /// intrinsics whitelist OR a loaded `target_plugin` entry.
    /// Cross-doc allowlist validator
    /// (`deploy/stateless-accept-extern-not-whitelisted`) fires when
    /// the symbol is in neither set (C13 deferred-2).
    pub hmac_extern: String,
    /// Spec line 2345-2349 — CSPRNG symbol used to seed key
    /// material at FSM-instance startup. Plugin authors are
    /// responsible for the entropy source. Same allowlist check
    /// surface as `hmac_extern`.
    pub rng_extern: String,
    /// Spec §synth-5-K line 2460-2462 — per-peer tracking table parameters
    /// (peer-tracking shape is anti-flood / DoS-hardening state).
    /// Optional at parse time; absence silent-skips the invariant
    /// check (`deploy/session-arming-quota-vs-peer-table-invariant-
    /// violated`) per the absent-input silent-skip discipline.
    #[serde(default)]
    pub peer_table: Option<PeerTable>,
    /// Spec §synth-5-K line 2460-2462 — per-handshake time budget in
    /// seconds. Build invariant `session_arming_quota ×
    /// max_handshake_time_s ≤ peer_table.capacity` (an attacker
    /// churning the quota cannot evict a slow legitimate handshake).
    /// Sibling of `cookie_lifetime_ms` / `key_rotation_s` at the
    /// `stateless_accept` block level — matches the spec's other
    /// time-budget parameters at this nesting depth. Optional at
    /// parse time; absence silent-skips the invariant check.
    #[serde(default)]
    pub max_handshake_time_s: Option<u32>,
}

/// Per-link domain attributes (SCE Protocol-Synthesis RFC §synth-5-K line 2263-2271).
///
/// When declared, `trust_class` is REQUIRED — spec
/// line 2731 makes `established_session` the explicit gating intent
/// for reassembly; defaulting would mask author confusion.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkDomainAttrs {
    /// `untrusted | session_arming | established_session` (required
    /// when `domain_attrs` declared, no default). Spec line
    /// 2265.
    pub trust_class: TrustClass,
    /// `true` when the link is exposed to a network the deployment
    /// does not control (public Internet, untrusted LAN). When
    /// `true`, `stateless_accept` becomes REQUIRED (anti-flood consumer).
    /// Spec line 2266-2271. Defaults to `false`.
    #[serde(default)]
    pub untrusted_source: bool,
}

/// Per-link configuration entry (SCE Protocol-Synthesis RFC §synth-5-K line 2232-2349).
///
/// Only `bind` + `driver` are required at the schema
/// level; every other field is `Option` because spec mandates them
/// conditionally on sibling-field presence (e.g. `burst_pps` only
/// matters when `rx_dispatch: isr_to_pool` is in play). Conditional
/// requirements are enforced by `validate_link_*` consumers (parser-
/// time, run after `parse_str`).
///
/// The struct carries the 7 core link fields plus the anti-flood
/// family (`session_arming_quota`, `accept_rate_*`,
/// `accepting_inactivity_timeout_ms`) and the `stateless_accept`
/// sub-block, each landed together with its semantic enforcement per
/// `[[feedback-silently-broken-hooks]]`.
///
/// **Cross-doc resolution**: the `name` axis (HashMap key) is joined
/// against forge `<scxml sce:kind="link" name="X">` document names via
/// `validate_link_name_cross_doc`. Two diagnostics:
/// `deploy/link-not-declared-in-deploy` (forge name has no deploy
/// counterpart) + `deploy/link-not-declared-in-forge` (deploy name
/// has no forge counterpart).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkConfig {
    /// `bind:` (spec line 2234) — endpoint address. Wire format is
    /// driver-specific (UDP: `host:port`, TCP: `host:port`, multicast:
    /// `224.x.x.x:port`); parser-side is opaque `String`. Parse-time
    /// validators don't normalize or split — driver-level parsing
    /// belongs to the link-driver runtime.
    pub bind: String,
    /// `driver:` (spec line 2235) — link-driver kind name,
    /// kept as `String` (not Rust enum) so forge `<sce:link>` author-
    /// declared drivers extend organically without freezing the set.
    /// Closed-allowlist validator (`deploy/link-driver-unknown`) rejects
    /// values absent from the known-driver baseline (currently
    /// `{lwip_udp, lwip_tcp}`; extended as new forge link-kind docs
    /// ship) AND from any cross-doc forge link-kind registry entry.
    pub driver: String,
    /// `role:`
    /// — explicit cross-document listener-role declaration. Pairs with the
    /// SCXML-side `<sce:session-role kind="..."/>` top-level element
    /// on the machine's source SCXML.
    ///
    /// Consumed by the
    /// `resolve_listener_links` join + the three typed partial-claim
    /// diagnostics (`link/deploy-role-listener-without-scxml-accept-
    /// side-role`, `scxml/accept-side-role-without-listener-link`,
    /// `link/role-listener-with-non-session-arming-trust-class`).
    ///
    /// Default `None` is the explicit "this link does not claim a
    /// session-FSM role" case (legacy fixtures silent-pass per the
    /// staged-migration discipline); the role/trust-class matrix
    /// rejects `role: listener` on any non-`session_arming` link.
    #[serde(default)]
    pub role: Option<LinkRole>,
    /// `mtu_bytes:` (spec line 2236-2242) — link-layer MTU. REQUIRED
    /// for fragmenting links (per the `reassembly/max-fragments-
    /// insufficient-for-mtu` consumer, RFC §synth-5-M line 2947). Optional at
    /// parse time; when missing on a Fragment-FSM-bound link, the
    /// consumer raises `deploy/link-mtu-missing-on-fragmenting-link`.
    #[serde(default)]
    pub mtu_bytes: Option<u32>,
    /// `expected_p99_bytes:` (spec line 2243-2247) — declared application
    /// p99 payload size. Drives the stage-copy rate warning
    /// (`reassembly/expected-fragmentation-rate-high`, RFC §synth-5-M line
    /// 2950) and the stage-copy WCET check (`reassembly/stage-copy-
    /// wcet-exceeds-slot-budget`, RFC §synth-5-M line 2995). Optional; when
    /// absent, the build assumes `p99 = mtu_bytes` (no warning).
    #[serde(default)]
    pub expected_p99_bytes: Option<u32>,
    /// `burst_pps:` (spec line 2248-2253) — declared peak inbound
    /// packets-per-second. Drives the RX pool sizing check
    /// (`deploy/link-burst-absorption-insufficient`, RFC §synth-5-K line
    /// 2489-2495). For multicast: derive from worst peer count × per-
    /// peer rate. REQUIRED when `rx_dispatch: isr_to_pool` per spec
    /// line 2261-2262 (conditional `rx_dispatch` default).
    #[serde(default)]
    pub burst_pps: Option<u32>,
    /// `rx_dispatch:` (spec line 2254-2262) — `isr_to_pool` for IRQ-
    /// driven wire-rate absorption, `worker_tick` for cooperative-tick-
    /// driven RX. Conditional default: `IsrToPool` when `burst_pps`
    /// declared, `WorkerTick` otherwise. Default applied by the
    /// field-resolver layer (post-parse).
    #[serde(default)]
    pub rx_dispatch: Option<RxDispatch>,
    /// `domain_attrs:` (spec line 2263-2271) — trust-class + untrusted-
    /// source flag. Optional at parse time; presence opens reassembly cross-
    /// doc validator paths (`reassembly/{untrusted-link-binding,
    /// trust-class-missing-on-fragmenting-link}`).
    #[serde(default)]
    pub domain_attrs: Option<LinkDomainAttrs>,

    // ── Anti-flood + stateless_accept (RFC §synth-5-K lines
    //    2272-2349 + 2449-2473). All five anti-flood fields plus the
    //    stateless_accept block are conditionally required when
    //    `domain_attrs.trust_class: session_arming` (the listener
    //    half of a handshake-bearing link). When the link is
    //    `untrusted` / `established_session`, declaring any of these
    //    fields fires `deploy/session-arming-fields-on-non-arming-link`
    //    — Accepting.* is never instantiated on those classes so the
    //    fields would be dead config. ──
    /// Spec line 2279-2289 — max concurrent half-open `Accepting.*`
    /// slots per link. MCU default 8, AP default 32 (not auto-applied
    /// — validator fires `session-arming-quota-missing` on absence
    /// when `trust_class: session_arming`). Build invariant per spec
    /// line 2460-2462: `session_arming_quota × max_handshake_time_s
    /// ≤ peer_table.capacity` (validator
    /// `deploy/session-arming-quota-vs-peer-table-invariant-violated`
    /// fires when violated; both sibling fields live on the
    /// `stateless_accept` sub-block, so the check is conditional on
    /// stateless_accept presence — absent-input silent-skip).
    #[serde(default)]
    pub session_arming_quota: Option<u32>,
    /// Spec line 2290-2299 — token-bucket refill rate per (link,
    /// src_addr). MCU default 4, AP default 16. Validator fires
    /// `accept-rate-config-missing` on absence when `trust_class:
    /// session_arming`.
    #[serde(default)]
    pub accept_rate_per_sec: Option<u32>,
    /// Spec line 2300-2302 — token-bucket capacity per (link,
    /// src_addr). Default `2 × accept_rate_per_sec` (not auto-
    /// applied — validator fires `accept-rate-config-missing` on
    /// absence when `trust_class: session_arming`).
    #[serde(default)]
    pub accept_rate_burst: Option<u32>,
    /// Spec line 2303-2311 — capacity of the per-source token-bucket
    /// table. Default `4 × session_arming_quota`. Spike from many
    /// src_addrs falls through to a single shared bucket (degraded
    /// mode) emitting runtime `session/accept-rate-table-saturated`.
    /// Not required at parse-time (downstream consumer fires
    /// runtime informational only).
    #[serde(default)]
    pub accept_rate_table_capacity: Option<u32>,
    /// Spec line 2312-2319 — bound on the worst-case
    /// `Accepting.AwaitingInitSyn` / `Accepting.SentInitAck` hold
    /// time. Optional; downstream FSM consumer enforces.
    #[serde(default)]
    pub accepting_inactivity_timeout_ms: Option<u32>,
    /// Spec line 2320-2349 — HMAC cookie stateless accept block.
    /// REQUIRED when `domain_attrs.untrusted_source: true`
    /// (validator fires `stateless-accept-required-on-untrusted-source`).
    /// Optional otherwise. Per spec line 2466-2469, the
    /// `hmac_extern` + `rng_extern` symbol allowlist check
    /// (`deploy/stateless-accept-extern-not-whitelisted`) consumes
    /// the loaded target_plugin set + the §synth-5-I baseline whitelist;
    /// the validator runs at the orchestrator level where both
    /// inputs converge (C13 deferred-2).
    #[serde(default)]
    pub stateless_accept: Option<StatelessAccept>,
}

impl LinkConfig {
    /// Conditional default resolution: `IsrToPool` when
    /// `burst_pps` is declared, `WorkerTick` otherwise. Spec line 2261
    /// verbatim. Returns the explicit author value when present.
    pub fn resolved_rx_dispatch(&self) -> RxDispatch {
        match (self.rx_dispatch, self.burst_pps.is_some()) {
            (Some(rxd), _) => rxd,
            (None, true) => RxDispatch::IsrToPool,
            (None, false) => RxDispatch::WorkerTick,
        }
    }
}

/// Per-machine scheduler descriptor (SCE Mesh §mesh-14, SCE Protocol-Synthesis RFC
/// §synth-5-K lines 2209-2222).
///
/// Three knobs are REQUIRED when `kind: cooperative`:
/// - `worker_stack_budget` ([`validate_scheduler_cooperative_stack_budget`])
///   — TLV-decode recursion bound (spec line 2426 `deploy/worker-stack-budget-missing`).
/// - `worker_slot_budget_us` ([`validate_worker_slot_budget_required_when_cooperative`])
///   — per-slot WCET ceiling, microseconds (spec line 2428-2429
///   `deploy/worker-slot-budget-missing`).
/// - `keepalive_jitter_budget_us` ([`validate_keepalive_jitter_required_when_cooperative`])
///   — keepalive emission jitter ceiling (spec line 2430-2431
///   `deploy/keepalive-jitter-budget-missing`).
///
/// `tick_period_us` is optional at parse time; when present alongside
/// `worker_slot_budget_us`, derives the cooperative scheduler's
/// per-tick slot capacity (`floor(tick_period_us / worker_slot_budget_us)`).
/// The derived count is the ceiling enforced by
/// [`validate_machine_scheduler_worker_capacity`]
/// (spec line 2423 `deploy/scheduler-incompatible-with-worker-count`).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MachineSchedulerConfig {
    /// Scheduler kind. Required when the section is present.
    pub kind: SchedulerKind,
    /// Worker-stack budget in bytes. REQUIRED when `kind: cooperative`;
    /// optional otherwise (other kinds use thread-stack defaults from
    /// the host runtime, not a build-time bound).
    #[serde(default)]
    pub worker_stack_budget: Option<u32>,
    /// Cooperative scheduler tick period in microseconds (spec line 2211).
    /// Together with `worker_slot_budget_us` derives the per-tick slot
    /// capacity used by [`validate_machine_scheduler_worker_capacity`].
    /// Optional at parse time; required for the worker-count vs slot-count
    /// check to fire (when absent, the check silent-skips per the
    /// absent-input precedent).
    #[serde(default)]
    pub tick_period_us: Option<u32>,
    /// Per-slot WCET ceiling in microseconds (spec line 2213). REQUIRED
    /// when `kind: cooperative` per spec line 2428-2429
    /// `deploy/worker-slot-budget-missing`. Used by the codec / algorithm
    /// aggregate-WCET check (§synth-5-B + §synth-5-A) and by the cooperative slot-count
    /// derivation in [`validate_machine_scheduler_worker_capacity`].
    #[serde(default)]
    pub worker_slot_budget_us: Option<u32>,
    /// Keepalive emission jitter ceiling in microseconds (spec line 2218).
    /// REQUIRED when `kind: cooperative` per spec line 2430-2431
    /// `deploy/keepalive-jitter-budget-missing`. Sum of worst-case slot
    /// budgets in one tick window MUST fit inside this bound; the
    /// downstream check lands with the §synth-5-B aggregate WCET consumer.
    #[serde(default)]
    pub keepalive_jitter_budget_us: Option<u32>,
    /// Static timer wheel depth — number of timer slots available
    /// (SCE Protocol-Synthesis RFC §synth-5-D line 904 "compile-time slot in a
    /// static timer wheel" + line 910 `timer/slot-overflow`).
    /// Optional at parse time; when present alongside
    /// `machines.<m>.timers`, the slot-overflow validator
    /// ([`validate_machine_timer_wheel_capacity`]) fires when
    /// `timers.len() > timer_wheel_depth`. Absent ⇒ silent-skip
    /// (absent-input silent-skip precedent — deploy-unaware paths
    /// don't have the wheel sizing information).
    #[serde(default)]
    pub timer_wheel_depth: Option<u32>,
    /// SCE Protocol-Synthesis RFC §synth-5-J-2 + §synth-5-L (item C3):
    /// fallback event-queue capacity for machines
    /// whose SCXML document omits the per-instance
    /// `<scxml sce:capacity="N">` attribute. Unit: events.
    ///
    /// Resolution rule: per-instance `SCXMLModel.event_queue_capacity`
    /// wins; this field supplies the deploy-default for the
    /// remainder. Both absent is permitted on std builds (they do
    /// not consume the capacity); the heapless no_std path makes
    /// the value load-bearing for `no_std` builds and adds a
    /// `default_event_queue_capacity-missing` diagnostic when the
    /// no_std codegen path has nothing to source the literal from.
    #[serde(default)]
    pub default_event_queue_capacity: Option<u32>,
    /// SCE Protocol-Synthesis RFC §synth-5-N line 3056-3057 — per-link
    /// work cap inside one cooperative scheduler tick. Unit:
    /// microseconds. Optional at parse time; required for both
    /// §synth-5-N codes that consume it
    /// (`link/concurrent-count-exceeds-scheduler-slots` derives the
    /// MCU slot ceiling via
    /// `floor(tick_period_us / per_link_budget_us)`;
    /// `link/per-link-budget-exceeds-tick-period` fires when
    /// `per_link_budget_us > tick_period_us`).
    /// Silent-skip when absent — single-doc
    /// compile paths + AP machines that use `tokio::spawn` per link
    /// don't consume the slot accounting.
    #[serde(default)]
    pub per_link_budget_us: Option<u32>,
}

/// Per-machine scheduler kind axis (SCE Mesh §mesh-14, SCE Protocol-Synthesis RFC
/// §synth-5-K). `tokio` and `rt` host the scheduler in async / RTOS-task
/// contexts; `cooperative` is the SCE-built single-thread tick loop
/// used on bare-metal MCUs (the §synth-7 foundation target).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SchedulerKind {
    Tokio,
    Cooperative,
    Rt,
}

impl SchedulerKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SchedulerKind::Tokio => "tokio",
            SchedulerKind::Cooperative => "cooperative",
            SchedulerKind::Rt => "rt",
        }
    }
}

/// Per-machine worker placement entry (SCE Protocol-Synthesis RFC §synth-5-D + §synth-5-I).
/// Declares which core hosts each worker doc's inbox producer
/// (link-rx-driven path) and consumer (SCXML processing thread).
///
/// Threaded into [`crate::ForgeCompileOptions::worker_placement`] by
/// [`crate::compile_forge_with_deploy`] for the codegen-invariant
/// validator [`crate::validate_worker_inbox_ordering_placement`]
/// (`e2980d83`) to detect cross-core relaxed-ordering violations
/// (`worker/inbox-ordering-relaxed-across-cores`).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerPlacementConfig {
    /// Core index hosting the inbox producer.
    pub producer_core: u32,
    /// Core index hosting the inbox consumer.
    pub consumer_core: u32,
}

/// Per-machine worker descriptor (SCE Protocol-Synthesis RFC §synth-5-D + §synth-5-K).
/// Authors list every worker doc bound to the machine and declare its
/// runtime placement when cross-core ordering matters. Absent
/// `placement:` ⇒ codegen-invariant validator silent-skips for that
/// worker (single-core mode); the cooperative slot-count check
/// ([`validate_machine_scheduler_worker_capacity`]) still counts the
/// entry toward the machine's worker budget.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerDeployConfig {
    /// Optional cross-core placement (see [`WorkerPlacementConfig`]).
    /// Required when the worker doc declares `<sce:inbox ordering="relaxed"/>`
    /// AND `core_count > 1`; the codegen-invariant validator catches the
    /// cross-core violation. Single-core machines and `ordering="acq_rel"`
    /// workers omit the block.
    #[serde(default)]
    pub placement: Option<WorkerPlacementConfig>,
}

/// Per-machine timer doc descriptor (SCE Protocol-Synthesis RFC §synth-5-D + §synth-5-K,
/// C1). Authors list every `sce:kind="timer"` doc bound to the
/// machine; the map's length feeds the static timer wheel slot count
/// check ([`validate_machine_timer_wheel_capacity`]). The schema
/// admits an empty struct today so the slot-overflow validator has
/// a count to compare against without forcing every author to declare
/// per-timer metadata yet.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct TimerDeployConfig {}

/// SRAM region descriptor (SCE Mesh §mesh-14, SCE Protocol-Synthesis RFC §synth-5-K).
/// Region attributes ride as raw strings at parse time so the schema
/// admits forward-extension ("dma_coherent", "non_cacheable", "fast",
/// "nocache") without a closed enum here; the §synth-5-E placement validator
/// is the consumer that interprets them.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SramRegionConfig {
    /// Base address. Accepts integer or `0x`-prefixed hex via YAML
    /// native parsing.
    pub base: u64,
    /// Region size in bytes. The YAML scalar form (`64K`, `512K`) is
    /// not interpreted here — authors write decimal or hex; size-suffix
    /// parsing lives in the §synth-5-E consumer when introduced.
    pub size: u64,
    /// Region attributes (e.g. `["dma_coherent", "cacheable"]`).
    #[serde(default)]
    pub attr: Vec<String>,
}

/// Per-machine memory layout (SCE Mesh §mesh-14, SCE Protocol-Synthesis RFC §synth-5-K).
/// SRAM region map and DMA-channel inventory feed the §synth-5-E placement /
/// cache-policy validators (the buffer-pool placement checks in
/// `lib.rs` consume `sram_regions` and the pool `cache_policy`).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryConfig {
    /// SRAM region map keyed by region name.
    #[serde(default)]
    pub sram_regions: HashMap<String, SramRegionConfig>,
    /// DMA channel inventory (target-specific identifiers).
    #[serde(default)]
    pub dma_channels: Vec<String>,
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
/// session) and reference external OEM config files (SCE_MESH.md §mesh-13).
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
    /// custom_tcp device-shared listen endpoint (SCE_MESH.md §mesh-16.8.3).
    /// One TCP server per device on `127.0.0.1:<port>`; each binding's
    /// per-target `connect:` reaches another device's server. Omit for
    /// devices that only initiate connections.
    pub custom_tcp: Option<CustomTcpTransportConfig>,
    /// DDS device-shared participant config. One DomainParticipant per
    /// device joins the declared domain; every dds binding on the device
    /// publishes and subscribes through it. Omit to join domain 0.
    pub dds: Option<DdsTransportConfig>,
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
    /// this file at runtime. SCE_MESH.md §mesh-13 / §mesh-14.
    pub config: Option<PathBuf>,
}

/// custom_tcp device-shared listen endpoint (SCE_MESH.md §mesh-16.8.3).
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

/// DDS device-shared participant configuration.
///
/// A DDS domain is the isolation unit: participants in different domains
/// never discover each other, and there is no cross-domain bridging. That
/// makes `domain_id` the one field a deployment genuinely has to control —
/// it is what keeps two deployments (or two concurrent test fixtures) on
/// the same host from joining each other's discovery. Everything else
/// Cyclone DDS exposes is tuning that belongs in its own XML config,
/// reached through `CYCLONEDDS_URI`, not in the mesh schema.
///
/// `deny_unknown_fields` rejects typos like `domian_id:` at parse time
/// rather than letting them fall through to the default domain, where the
/// symptom would be two deployments silently discovering each other.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DdsTransportConfig {
    /// DDS domain id. Omitted means domain 0, the DDS default. The DDS
    /// specification caps the usable range well below `u32::MAX`, but the
    /// exact bound is a function of the participant's port-mapping
    /// parameters, so the schema accepts any `u32` and lets the DDS
    /// implementation reject an unusable value at participant creation —
    /// a build-time bound here would encode one vendor's mapping.
    pub domain_id: Option<u32>,
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

/// SOME/IP device-shared configuration (SCE_MESH.md §mesh-13).
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

/// Ordering guarantee declared on a per-binding basis (SCE_MESH.md §mesh-10.6).
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
/// section (SCE_MESH.md §mesh-10.6.1). 100 ms covers the Zenoh session-refresh
/// window and the SOME/IP retransmit envelope at 1 kHz sender rates. This
/// constant is the single source of truth — the C++ runtime no longer
/// hard-codes a fallback; every emitted router carries an explicit value.
pub const DEFAULT_GAP_TIMEOUT_MS: u64 = 100;

/// Default `tick_period_ms` applied when a machine omits the `ordering:`
/// section (SCE_MESH.md §mesh-10.6.1). One half of [`DEFAULT_GAP_TIMEOUT_MS`]
/// (Nyquist) so worst-case gap recovery latency is bounded by
/// `gap_timeout + tick_period`.
pub const DEFAULT_TICK_PERIOD_MS: u64 = 50;

/// Per-machine ordering buffer timings (SCE_MESH.md §mesh-10.6.1).
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
    /// number. SCE_MESH.md §mesh-10.6.4 — the receiver buffer drains contiguous
    /// envelopes and emits `ORDERING_GAP` on timeout.
    pub gap_timeout_ms: u64,
    /// Cadence at which the generated router drives `OrderingBuffer::tick`
    /// (SCE_MESH.md §mesh-10.6.4). Must be strictly less than
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
/// SCE Mesh §mesh-16.7 row 8 (`PEER_PARTITIONED`) couples peer-failure
/// detection latency to Zenoh's own keepalive cadence. Values below
/// this floor race the router's own internal heartbeat and generate
/// spurious DELETE/PUT churn, so parse-time rejection is preferred
/// over runtime misbehaviour. Matches the Nyquist-style floor the
/// plan memo locked.
pub const MIN_LIVELINESS_LEASE_MS: u64 = 100;

/// Minimum `query_timeout_ms` accepted in a `server:` section.
///
/// SCE Mesh §mesh-9.5 Zenoh server queryable timeout (gap Z2): values
/// below this floor are almost certainly typos — even a trivial
/// engine macrostep usually takes longer than 10 ms, so a sub-floor
/// value would cause every inbound query to time out before the
/// engine can respond. Parse-time rejection surfaces the mistake
/// at the offending deploy.yaml line rather than a silent runtime
/// cleanup cascade.
pub const MIN_SERVER_QUERY_TIMEOUT_MS: u64 = 10;

/// Per-machine Zenoh liveliness configuration (SCE Mesh §mesh-16.7 row 8).
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
/// per-partition liveness is tracked as a separate §mesh-16.9 E2/F
/// sub-landing — see SCE_MESH.md §mesh-16.4 for the deferral rationale.
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
/// SCE Mesh §mesh-10.10 (`OutboundBuffer`): a buffer with capacity zero is
/// semantically equivalent to the pre-§mesh-10.10 "silently drop if not
/// ready" behaviour — it cannot hold anything. Rejecting zero at parse
/// time surfaces the mistake at the offending deploy.yaml line rather
/// than generating a router that compiles but cannot honour the §mesh-10.7
/// contract. Values of one or above are accepted regardless of
/// perceived "too small" judgement: a single-slot buffer is a
/// legitimate test-harness shape (one in-flight envelope during
/// readiness gating).
pub const MIN_OUTBOUND_BUFFER_MAX_PENDING: u32 = 1;

/// Per-machine outbound readiness-gated buffer (SCE Mesh §mesh-10.10).
///
/// Opt-in: absent section ⇒ no buffer emitted; every outbound send
/// goes straight to the transport and any pre-readiness send is
/// silently lost per the pre-§mesh-10.10 behaviour. Section present ⇒ the
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
    /// `BACKPRESSURE_DROP` (§mesh-16.7 row 9) and drops the newest. Must
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

/// Minimum `max_retries` accepted in a per-binding `retry:` section.
///
/// SCE Mesh §mesh-16.7 row 3 retry layer: `max_retries: 0` is semantically
/// equivalent to omitting the section — the retry wrapper would fast-fail
/// every dispatcher failure and SEND_FAILED would fire per Stage 1/2
/// behaviour. Rejecting zero at parse time surfaces the mistake at the
/// offending deploy.yaml line rather than emitting a no-op retry wrapper
/// that adds cost without benefit. Values of one or above are accepted
/// regardless of perceived "too small" judgement: `max_retries: 1`
/// (single retry) is a legitimate "give it exactly one more chance"
/// shape that some authors prefer over either extreme.
pub const MIN_RETRY_MAX_RETRIES: u32 = 1;

/// Per-binding retry policy (SCE Mesh §mesh-16.7 row 3 DELIVERY_EXHAUSTED).
///
/// Opt-in: absent section ⇒ no retry layer emitted; the OutboundBuffer's
/// dispatcher closure goes straight to the transport and any
/// `SendResult::failure()` raises SEND_FAILED per Stage 1/2 (terminal).
/// Section present ⇒ the generated router wraps the dispatcher in a
/// `RetryingDispatcher` configured with these values; transient
/// (`retryable=true`) failures are retried with exponential backoff up
/// to `max_retries` additional attempts, then DELIVERY_EXHAUSTED fires
/// with `attempts = max_retries + 1`. Terminal (`retryable=false`)
/// failures fast-fail with `attempts = 1`.
///
/// All timing fields are in milliseconds against
/// `std::chrono::steady_clock` (monotonic, immune to wall-clock jumps).
/// The runtime never re-reads the values after init() — they are
/// codegen-baked into the RetryingDispatcher ctor at deploy.yaml parse
/// time.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RetryPolicyConfig {
    /// Maximum number of ADDITIONAL attempts after the first failure.
    /// Total dispatch attempts before exhaustion = `max_retries + 1`.
    /// Must be `>= MIN_RETRY_MAX_RETRIES` (zero is rejected — omit the
    /// section to disable retries instead).
    pub max_retries: u32,

    /// Initial backoff before the first retry, in milliseconds.
    /// Subsequent backoffs apply `backoff_multiplier` (capped at
    /// `max_backoff_ms`). Default: 100ms.
    #[serde(default = "default_initial_backoff_ms")]
    pub initial_backoff_ms: u64,

    /// Multiplier applied to the previous backoff to compute the next
    /// (exponential growth). `1.0` ⇒ fixed-delay retries; `2.0` ⇒
    /// classic exponential backoff (each retry waits twice as long as
    /// the previous one). Must be `>= 1.0` — values below 1.0 would
    /// shrink backoff toward zero, defeating the purpose of retry
    /// pacing. Default: 2.0.
    #[serde(default = "default_backoff_multiplier")]
    pub backoff_multiplier: f64,

    /// Upper bound on a single backoff interval, in milliseconds.
    /// Caps the exponential growth so a long-running retry train
    /// doesn't escalate to minute-scale delays. Must be
    /// `>= initial_backoff_ms`. Default: 5000ms.
    #[serde(default = "default_max_backoff_ms")]
    pub max_backoff_ms: u64,

    /// ±N% randomized jitter applied to each computed backoff interval
    /// (thundering-herd mitigation). `0` ⇒ deterministic backoff;
    /// `100` ⇒ each interval randomly chosen in `[0, 2*computed]`.
    /// Must be `<= 100`. Default: 10.
    #[serde(default = "default_backoff_jitter_pct")]
    pub backoff_jitter_pct: u32,
}

/// Per-binding authorization policy (SCE Mesh §mesh-16.7 row 10 UNAUTHORIZED).
///
/// Opt-in: absent section ⇒ no auth layer emitted; transport-level
/// rejection (Zenoh `ZException` on `Session::open`, SOME/IP
/// `register_availability_handler(false)`) routes through the existing
/// row 1 TRANSPORT_UNAVAILABLE / row 8 PEER_PARTITIONED classifications.
/// Section present ⇒ the generated router observes the transport's
/// rejection signal and classifies a known-auth-fail subset (Zenoh
/// `ZException::what()` containing certificate/tls/auth tokens; SOME/IP
/// SD denial code) as UNAUTHORIZED row 10 instead of the generic
/// row 1 / row 8.
///
/// Per-target granularity (Q1 = (b)): mirrors `RetryPolicyConfig`'s
/// per-binding shape so each outbound target can pin a distinct peer
/// fingerprint without dragging the whole binary into one trust
/// boundary.
///
/// One-shot semantics (Q5 = one-shot per (binary-startup, target)):
/// after UNAUTHORIZED fires for a target, the OutboundBuffer's
/// `ready_` stays false; outbound envelopes accumulate up to
/// `max_pending_per_target` and drop with BACKPRESSURE_DROP (row 9).
/// The author's `error.communication` transition is the cleanup
/// boundary; operator must restart the binary to re-trust.
///
/// Per-transport applicability (auth = mTLS-style peer pinning, wired
/// per transport binding):
/// * `zenoh` — `peer_fingerprint` pins the peer cert; failed handshake
///   classifies on `ZException::what()` text.
/// * `someip` — `sd_denied_classifies_as_unauthorized: true` opts in
///   to classifying SOMEIP SD denial as UNAUTHORIZED instead of
///   PEER_PARTITIONED.
/// * `custom_tcp` / `shm` — rejected at parse time: no auth wiring is
///   defined for these transports. The validator surfaces a clear error message
///   pointing authors at zenoh / someip.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AuthPolicyConfig {
    /// Opt-in flag. Absent / `false` ⇒ no auth layer emitted (defaults
    /// apply). `true` ⇒ transport-specific auth wiring kicks in; the
    /// other fields below become honoured per-transport.
    #[serde(default)]
    pub required: bool,

    /// SHA-256 peer-certificate fingerprint (`"sha256:<hex>"`). For
    /// `zenoh` bindings: pinned against the peer's TLS handshake cert
    /// chain. Required when `required: true` AND `transport == "zenoh"`.
    /// Format validation at parse time: literal `"sha256:"` prefix
    /// followed by 64 lowercase hex characters (the SHA-256 digest
    /// length). Non-zenoh transports must omit this field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub peer_fingerprint: Option<String>,

    /// For `someip` bindings only: when `register_availability_handler`
    /// reports `is_available=false`, classify as row 10 UNAUTHORIZED
    /// (vs the row 8 PEER_PARTITIONED default). The SOMEIP wire
    /// protocol does not surface a distinct auth-vs-network failure
    /// signal — this flag is the author's contract with the SD
    /// responder ("an availability=false event after I requested
    /// service IS an auth denial in my deployment"). Required when
    /// `required: true` AND `transport == "someip"`; ignored for other
    /// transports.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sd_denied_classifies_as_unauthorized: Option<bool>,
}

impl AuthPolicyConfig {
    /// Validate the constraints (independent of transport — the caller
    /// supplies the transport-specific applicability check separately).
    /// Returns the rejection reason without the machine / target name;
    /// the caller wraps this into [`DeployError::InvalidAuthPolicy`].
    fn validation_error(&self, transport: &str) -> Option<String> {
        if !self.required {
            // `required: false` (or omitted) ⇒ all other fields are
            // ignored; presence of `peer_fingerprint` /
            // `sd_denied_classifies_as_unauthorized` alongside
            // `required: false` is a likely authoring mistake (the
            // values would be silently ignored at codegen time).
            if self.peer_fingerprint.is_some()
                || self.sd_denied_classifies_as_unauthorized.is_some()
            {
                return Some(
                    "`auth: { required: false }` cannot carry `peer_fingerprint` or \
                     `sd_denied_classifies_as_unauthorized` — those fields are only \
                     honoured when `required: true`. Either set `required: true` or \
                     drop the ignored fields"
                        .to_string(),
                );
            }
            return None;
        }
        match transport {
            "zenoh" => {
                let fp = match &self.peer_fingerprint {
                    Some(s) => s,
                    None => {
                        return Some(
                            "zenoh `auth: { required: true }` requires `peer_fingerprint: \"sha256:<64-hex>\"` — \
                             without a pinned cert digest there is nothing to authorize against. \
                             Either pin the peer fingerprint or set `required: false`"
                                .to_string(),
                        );
                    }
                };
                if let Some(reason) = validate_sha256_fingerprint(fp) {
                    return Some(reason);
                }
                if self.sd_denied_classifies_as_unauthorized.is_some() {
                    return Some(
                        "zenoh `auth:` block must not declare \
                         `sd_denied_classifies_as_unauthorized` — that field is \
                         SOMEIP-specific"
                            .to_string(),
                    );
                }
                None
            }
            "someip" => {
                let opt_in = self.sd_denied_classifies_as_unauthorized.unwrap_or(false);
                if !opt_in {
                    return Some(
                        "someip `auth: { required: true }` requires \
                         `sd_denied_classifies_as_unauthorized: true` — without \
                         opting in to SD-denial classification, the SOMEIP availability \
                         handler cannot distinguish row 8 PEER_PARTITIONED from row 10 \
                         UNAUTHORIZED. Either set the flag or set `required: false`"
                            .to_string(),
                    );
                }
                if self.peer_fingerprint.is_some() {
                    return Some(
                        "someip `auth:` block must not declare `peer_fingerprint` — \
                         that field is zenoh-specific (TLS handshake cert pinning). \
                         SOMEIP authorization is delegated to the SD responder; this \
                         binding observes the SD denial code, not a cert chain"
                            .to_string(),
                    );
                }
                None
            }
            other => Some(format!(
                "transport '{}' does not support §16.7 row 10 UNAUTHORIZED in this release — \
                 only `zenoh` (mTLS cert pinning) and `someip` (SD denial classification) \
                 are wired. Either move the binding to a supported transport or set \
                 `required: false`",
                other,
            )),
        }
    }
}

/// Validate that a string matches the `sha256:<64-hex>` literal shape.
/// Returns the rejection reason on failure, `None` on success.
fn validate_sha256_fingerprint(fp: &str) -> Option<String> {
    const PREFIX: &str = "sha256:";
    let hex_part = match fp.strip_prefix(PREFIX) {
        Some(s) => s,
        None => {
            return Some(format!(
                "peer_fingerprint ({:?}) must start with the literal prefix `sha256:` — \
                 SCE Mesh §16.7 row 10 pins SHA-256 digests only in this release",
                fp,
            ));
        }
    };
    if hex_part.len() != 64 {
        return Some(format!(
            "peer_fingerprint hex portion ({:?}) must be exactly 64 lowercase hex characters \
             (SHA-256 digest length); observed length {}",
            hex_part,
            hex_part.len(),
        ));
    }
    if !hex_part
        .bytes()
        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Some(format!(
            "peer_fingerprint hex portion ({:?}) must contain only lowercase hex characters \
             (`[0-9a-f]`) — uppercase hex and non-hex bytes are rejected for byte-for-byte \
             canonical-form pinning",
            hex_part,
        ));
    }
    None
}

fn default_initial_backoff_ms() -> u64 {
    100
}

fn default_backoff_multiplier() -> f64 {
    2.0
}

fn default_max_backoff_ms() -> u64 {
    5000
}

fn default_backoff_jitter_pct() -> u32 {
    10
}

impl RetryPolicyConfig {
    /// Validate the constraints. Returns the rejection reason without
    /// the machine / target name — the caller wraps this into
    /// [`DeployError::InvalidRetryPolicy`].
    fn validation_error(&self) -> Option<String> {
        if self.max_retries < MIN_RETRY_MAX_RETRIES {
            return Some(format!(
                "max_retries ({}) must be >= {} — a zero-retry policy is \
                 semantically equivalent to omitting the section (the \
                 dispatcher would fast-fail every failure and SEND_FAILED \
                 would fire per Stage 1/2 behaviour); omit the section \
                 entirely to opt out of retries instead",
                self.max_retries, MIN_RETRY_MAX_RETRIES,
            ));
        }
        if self.initial_backoff_ms == 0 {
            return Some(
                "initial_backoff_ms must be > 0 — a zero-delay retry would \
                 hammer the failing transport at the engine-tick cadence, \
                 defeating the purpose of pacing"
                    .to_string(),
            );
        }
        // NaN-safe: partial_cmp returns None on NaN; only
        // Greater/Equal pass the >= 1.0 check. The explicit Ordering
        // match makes the NaN handling visible at the call site
        // instead of hiding it behind `!(x >= 1.0)` inverted-compare.
        use std::cmp::Ordering;
        let backoff_ok = matches!(
            self.backoff_multiplier.partial_cmp(&1.0),
            Some(Ordering::Greater | Ordering::Equal)
        );
        if !backoff_ok {
            return Some(format!(
                "backoff_multiplier ({}) must be >= 1.0 — sub-unit \
                 multipliers shrink backoff toward zero across retries, \
                 defeating exponential pacing",
                self.backoff_multiplier,
            ));
        }
        if self.max_backoff_ms < self.initial_backoff_ms {
            return Some(format!(
                "max_backoff_ms ({}) must be >= initial_backoff_ms ({}) — \
                 the cap cannot be smaller than the first interval",
                self.max_backoff_ms, self.initial_backoff_ms,
            ));
        }
        if self.backoff_jitter_pct > 100 {
            return Some(format!(
                "backoff_jitter_pct ({}) must be <= 100 — values above \
                 100 would invert the jitter range (negative backoffs)",
                self.backoff_jitter_pct,
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
    // §mesh-15.4: the deployment topology is exactly these three roles, so the
    // set is closed here rather than passed through as a string — a vehicle
    // network mixes peers with a router+client gateway, and a typo that
    // silently degraded a gateway to a peer would partition the mesh.
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
    /// Machine-lifetime subscriptions (SCE_MESH.md §mesh-13). Subscribe on
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
    /// Server-side transport registration (SCE_MESH.md §mesh-13 Session E).
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
    /// Per-machine ordering buffer timings (SCE_MESH.md §mesh-10.6.1). Absent
    /// section ⇒ [`OrderingTimings::default_const`] (100 ms /
    /// 50 ms). Section present ⇒ both fields are required and validated
    /// (positive, Nyquist) at parse time. The values are emitted directly
    /// into the generated router; no fallback exists below the deploy
    /// layer.
    #[serde(default)]
    pub ordering: Option<OrderingTimings>,
    /// Per-machine Zenoh liveliness configuration (SCE Mesh §mesh-16.7 row 8).
    /// Absent section ⇒ no liveliness token declared and no subscriber
    /// installed; the generated router emits zero liveliness code.
    /// Section present ⇒ `lease_ms` is required and validated at parse
    /// time. Opt-in by design — see [`LivelinessConfig`].
    #[serde(default)]
    pub liveliness: Option<LivelinessConfig>,
    /// Per-machine outbound buffer for readiness-gated send paths
    /// (SCE Mesh §mesh-10.10). Absent section ⇒ no buffer emitted; outbound
    /// sends go straight to the transport and any pre-readiness send
    /// is silently lost (SOME/IP before `offer_service`, Zenoh PUT
    /// before any subscriber declares). Section present ⇒ opt-in
    /// targets route through `OutboundBuffer::admit`, the transport's
    /// native readiness primitive feeds `markReady` / `markNotReady`,
    /// and overflow raises `error.communication` with reason
    /// `BACKPRESSURE_DROP` (§mesh-16.7 row 9). See [`OutboundBufferConfig`].
    #[serde(default)]
    pub outbound_buffer: Option<OutboundBufferConfig>,
    /// Author-pinned §mesh-9.6 SOMEIP scxml-invoke service ID (RFC F.X-1
    /// hybrid allocator). Optional — when present, the hybrid allocator
    /// reserves this ID for the machine and skips it during counter
    /// auto-assignment. Absent ⇒ counter auto-assigns from the lowest
    /// unreserved slot in lex order.
    ///
    /// **Range constraint**: must lie inside the §mesh-9.6 invoke sub-range
    /// `[0x8100, 0x817F]` — the upper half of the SCE-reserved 256-slot
    /// space is reserved for §mesh-16.4 region-liveness (RFC F.X-3).
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

    /// Optional author-pin for the §mesh-16.7 row 8 SOME/IP machine-level
    /// liveness service ID, validated against the F.X-4 sub-range
    /// `[0x8280, 0x82FF]`. Symmetric with [`MachineConfig::someip_service_id`]
    /// at the per-machine block (the participant key for machine-liveness
    /// is `<machine>`, not `<machine>__P__<partition>`).
    ///
    /// `None` → the F.X-4 hybrid allocator
    /// ([`crate::mesh::transport::someip::assign_machine_liveness_service_ids`])
    /// auto-assigns the lowest unreserved slot in lex order.
    /// `Some(id)` → the allocator validates the pin is inside the sub-range
    /// and disjoint from any other machine's pin; out-of-range / collision
    /// raise the matching `DeployError::SomeipMachineLivenessServiceIdPin*`
    /// at parse time.
    ///
    /// **YAML grammar**: same as `someip_service_id` — integer literal or
    /// quoted hex string (preferred). Per RFC F.X-4 D3.
    #[serde(
        default,
        deserialize_with = "deserialize_someip_machine_liveness_service_id"
    )]
    pub someip_machine_liveness_service_id: Option<u16>,

    /// Per-machine platform descriptor (SCE Mesh §mesh-14, SCE Protocol-Synthesis RFC
    /// §synth-5-K). Absent ⇒ no platform classification declared on this
    /// machine; downstream codegen-matrix consumers fall back to their
    /// own defaults. Present ⇒ class/os pair is admissible per
    /// [`PlatformClass::admits_os`], enforced at parse time by
    /// [`validate_platform_class_os_consistency`].
    #[serde(default)]
    pub platform: Option<PlatformConfig>,

    /// Per-machine scheduler descriptor (SCE Mesh §mesh-14, SCE Protocol-Synthesis RFC
    /// §synth-5-K). Absent ⇒ machine inherits the partition / device runtime
    /// defaults. Present ⇒ `kind` is required, and `kind: cooperative`
    /// requires `worker_stack_budget` ([`validate_scheduler_cooperative_stack_budget`]).
    #[serde(default)]
    pub scheduler: Option<MachineSchedulerConfig>,

    /// Per-machine memory layout (SCE Mesh §mesh-14, SCE Protocol-Synthesis RFC §synth-5-K).
    /// Absent ⇒ no SRAM/DMA layout declared; the §synth-5-E placement
    /// validator skips this machine. Present ⇒ region attributes ride as
    /// raw strings; structural interpretation lives in the §synth-5-E
    /// placement / cache-policy validators.
    #[serde(default)]
    pub memory: Option<MemoryConfig>,

    /// Per-machine worker doc registry (SCE Protocol-Synthesis RFC §synth-5-D + §synth-5-K).
    /// Keyed by worker name (matches `<scxml sce:kind="worker"
    /// name="...">`). The map's length feeds the cooperative slot-count
    /// check ([`validate_machine_scheduler_worker_capacity`]); each entry
    /// can carry an optional cross-core `placement:` block consumed by
    /// the inbox-ordering codegen-invariant validator.
    ///
    /// Absent ⇒ machine declares no workers; the slot-count check
    /// silent-skips. Present ⇒ slot-count check fires when
    /// `workers.len() > derived_slot_count`
    /// (spec line 2423 `deploy/scheduler-incompatible-with-worker-count`).
    #[serde(default)]
    pub workers: HashMap<String, WorkerDeployConfig>,

    /// Per-machine Timer doc registry (SCE Protocol-Synthesis RFC §synth-5-D, C1).
    /// Keyed by timer name (matches `<scxml sce:kind="timer"
    /// name="...">`). The map's length feeds the static timer wheel
    /// slot-overflow check
    /// ([`validate_machine_timer_wheel_capacity`]).
    ///
    /// Absent ⇒ machine declares no timers; the slot-overflow
    /// validator silent-skips. Present ⇒ slot-overflow fires when
    /// `timers.len() > scheduler.timer_wheel_depth` (spec line 910
    /// `timer/slot-overflow`).
    #[serde(default)]
    pub timers: HashMap<String, TimerDeployConfig>,

    /// Per-machine dynamic-state capacity ceilings (SCE Protocol-Synthesis RFC
    /// §synth-5-L lines 2570-2585 + 2649). Keyed by limit name —
    /// the dotted suffix of a `<sce:capacity source="deploy"
    /// key="machines.<machine>.limits.<limit>">` reference on a
    /// bounded-collection doc. Value is the compile-time slot count
    /// the codegen lowers into a per-language constant (Rust
    /// `heapless::Vec<T, N>` / Cpp `std::array<T, N>` / etc per spec
    /// §synth-5-J-5).
    ///
    /// Absent ⇒ machine declares no limits;
    /// [`validate_bounded_collection_capacity_resolution`] silent-
    /// skips for any BC doc whose `<sce:capacity>` keys this machine.
    /// Present ⇒ each BC doc with a `deploy` capacity source resolves
    /// its limit name against this map; missing entries fire
    /// `collection/capacity-unresolved` with `Fix::ReplaceOneOf`
    /// carrying the sorted list of declared limit names so authors
    /// see legal alternatives.
    ///
    /// ```yaml
    /// machines:
    ///   mcu_node:
    ///     source: mcu_node.scxml
    ///     limits:
    ///       local_subscriptions: 32
    ///       in_flight_reassembly: 8
    /// ```
    #[serde(default)]
    pub limits: HashMap<String, u32>,

    /// Per-machine link configuration registry (SCE Protocol-Synthesis RFC §synth-5-K
    /// line 2232-2349). Keyed by link name (joined against forge
    /// `<scxml sce:kind="link" name="X">` document names via the
    /// cross-doc validator pair `deploy/{link-not-declared-in-deploy,
    /// link-not-declared-in-forge}`).
    ///
    /// Absent ⇒ machine declares no link instances; cross-doc validator
    /// silent-skips (no forge-side link references to resolve). Present
    /// ⇒ each entry's `bind` + `driver` + optional fields are validated
    /// at parse time + cross-doc resolved at orchestrator pass-2.
    ///
    /// [`LinkConfig`] carries the core link fields (`bind`,
    /// `driver`, `mtu_bytes`, `expected_p99_bytes`, `burst_pps`,
    /// `rx_dispatch`, `domain_attrs`) plus the anti-flood fields
    /// (`session_arming_quota`, `accept_rate_*`,
    /// `accepting_inactivity_timeout_ms`, `stateless_accept` block);
    /// any other key parse-rejects via
    /// `#[serde(deny_unknown_fields)]` on [`LinkConfig`].
    #[serde(default)]
    pub links: HashMap<String, LinkConfig>,

    /// Machine-wide pool-defaults block (SCE Protocol-Synthesis RFC §synth-5-K
    /// lines 2350-2369). Today carries only
    /// `stage_copy_policy`; further consumer-gated fields land additively per
    /// `[[feedback-silently-broken-hooks]]`. Absent ⇒
    /// `stage_copy_policy = Warn` (existing
    /// `reassembly/expected-fragmentation-rate-high` warning
    /// semantic preserved).
    #[serde(default)]
    pub pool_defaults: Option<PoolDefaults>,
}

impl MachineConfig {
    /// Resolved stage-copy policy. Absence of `pool_defaults` entirely
    /// keeps the default behavior (Warn). When `pool_defaults` is
    /// declared, its `stage_copy_policy` String field maps to the
    /// typed enum via [`StageCopyPolicy::from_wire_str`]; unknown values
    /// are unreachable here because [`validate_pool_defaults`]
    /// parse-rejects them. The `unwrap_or(Warn)` on the from_str
    /// result is defense-in-depth — a missed validator wiring would
    /// fall back to the spec-default behavior rather than panic.
    pub fn resolved_stage_copy_policy(&self) -> StageCopyPolicy {
        self.pool_defaults
            .as_ref()
            .and_then(|pd| StageCopyPolicy::from_wire_str(&pd.stage_copy_policy))
            .unwrap_or(StageCopyPolicy::Warn)
    }
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

/// Custom deserializer for [`MachineConfig::someip_machine_liveness_service_id`].
/// Parallel to [`deserialize_someip_service_id`] for §mesh-16.7 row 8 SOME/IP
/// machine-level liveness pins (RFC F.X-4 D3).
fn deserialize_someip_machine_liveness_service_id<'de, D>(
    deserializer: D,
) -> Result<Option<u16>, D::Error>
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
            let trimmed = if let Some(rest) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X"))
            {
                rest
            } else {
                return Err(serde::de::Error::custom(format!(
                    "someip_machine_liveness_service_id: hex string '{s}' must start with `0x` \
                     (e.g. `\"0x8280\"`); raw decimal integers are also accepted but bare hex \
                     strings without the prefix are rejected to avoid confusion"
                )));
            };
            u16::from_str_radix(trimmed, 16).map(Some).map_err(|e| {
                serde::de::Error::custom(format!(
                    "someip_machine_liveness_service_id: cannot parse hex literal '{s}' as u16: \
                     {e} (expected `0x8280`-style hex inside [0x0000, 0xFFFF])"
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

/// Server-side transport binding (SCE_MESH.md §mesh-13 Session E).
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
    /// DDS request-leg topic this server reads. The reply and
    /// notification topics are derived from it, so a server answers on
    /// the leg paired with the topic it serves and there is no field in
    /// which to express an unpaired one.
    #[serde(default)]
    pub topic: Option<String>,
    /// SCE Mesh §mesh-14.4 — server-side multi-instance pool.
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

    /// Per-server Zenoh queryable response deadline (SCE Mesh §mesh-9.5, gap
    /// Z2).
    ///
    /// **Zenoh-only scope**: SOME/IP (and other non-zenoh) server-side
    /// response lifecycles use distinct transport-native state
    /// (`pending_server_requests_` for vsomeip), not the
    /// `pending_server_queries_` map that this knob targets. Parse-time
    /// validation rejects the knob on non-zenoh servers so a SOME/IP
    /// author cannot inadvertently ship a silent no-op. A SOME/IP
    /// equivalent is consumer-gated and would land under its own knob.
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
        let ms = self.query_timeout_ms?;
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

/// A single machine-lifetime subscription declaration (SCE_MESH.md §mesh-13).
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

/// Per-event SOME/IP binding (SCE_MESH.md §mesh-14).
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
/// time (SCE_MESH.md §mesh-13, §mesh-14).
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
    /// deadline (SCE_MESH.md §mesh-9.5 precedence). Applied when a per-invoke
    /// `<param name="_mesh_deadline_ms">` is absent. A per-invoke value
    /// always overrides; if both are present with different values
    /// `sce-build` emits an informational notice (per-invoke override is
    /// expected usage). Absent here AND on the `<param>` ⇒ no deadline
    /// (the request can wait indefinitely for a reply).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline_ms: Option<u64>,

    /// Per-binding ordering declaration (SCE_MESH.md §mesh-10.6). Default
    /// `None` keeps the legacy "engine sees arrival order" behavior;
    /// `Required` activates the runtime `OrderingBuffer` for transports
    /// whose `supplies_ordering` is `false`, and is a topology error for
    /// transports whose `ordering_representable` is `false` (CAN).
    #[serde(default)]
    pub ordering: OrderingRequirement,

    /// SCE Mesh §mesh-14.4 — bounded instance pool for a SOME/IP binding that
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

    /// SCE Mesh §mesh-14.4 — names the `<param>` whose runtime value feeds
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

    /// SCE Mesh §mesh-14.6 — the set of targets whose RpcReply may be
    /// correlated against a request sent to THIS binding's target.
    ///
    /// Absent ⇒ the responder set is the binding's own target: a reply
    /// arriving from anywhere else does not retire the correlation
    /// entry. That default is what §mesh-14.6 calls the same-target
    /// constraint, and it is the safe shape — a correlation entry is a
    /// one-shot resource, so any peer permitted to spend it can retire
    /// another peer's pending request.
    ///
    /// Present ⇒ a broker / proxy / fan-in topology: the request goes to
    /// `#alpha`, but `#broker` is also allowed to answer it. Each member
    /// must name a machine that exists in the topology, the list may not
    /// be empty, and the binding's own target is always implicitly a
    /// member (declaring it explicitly is allowed and idempotent).
    ///
    /// Rejected at parse time on transports whose
    /// [`TransportDescriptor::supports_cross_target_reply`] is `false`
    /// — on Zenoh the reply closure is bound to one target's KeyExpr at
    /// the protocol layer, so a wider set could never be realised.
    ///
    /// [`TransportDescriptor::supports_cross_target_reply`]: crate::mesh::transport::TransportDescriptor::supports_cross_target_reply
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_from: Option<Vec<String>>,

    /// SCE Protocol-Synthesis RFC §synth-5-E — name reference into the forge
    /// pool registry naming the buffer-pool kind artifact whose slots
    /// the link's RX-side `Sample::take()` copies into. Resolved
    /// against [`forge::pool_registry::ForgePoolRegistry`] (built by
    /// walking every parsed `.forge` file in the build) and validated
    /// at parse time:
    ///
    /// * absent ⇒ `LinkConfig::stage_copy_hook = PanicOnTakeHook`
    ///   (sce-link-runtime default; subscriber callbacks that never
    ///   call `take()` are unaffected, callbacks that do call `take()`
    ///   panic with a clear message);
    /// * present + name unknown ⇒
    ///   `mesh/deploy-stage-pool-not-declared`;
    /// * present + name resolves to a non-buffer-pool kind ⇒
    ///   `mesh/deploy-stage-pool-wrong-kind`;
    /// * present on a transport that has no buffer-pool RX staging
    ///   surface ⇒ `mesh/deploy-stage-pool-transport-mismatch`.
    ///
    /// Single source of truth: the pool *template* (slot count, slot
    /// size, section, alignment, DMA channel, cache policy) lives in
    /// the `.forge` file. deploy.yaml only adds the
    /// per-binding name reference; duplicating the template fields
    /// here would split authoring across two files.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage_pool: Option<String>,

    /// SCE Mesh §mesh-16.7 row 3 — per-binding retry policy. Opt-in: absent
    /// ⇒ no retry layer, SEND_FAILED fires per Stage 1/2 on the first
    /// dispatcher decline. Present ⇒ the generated router wraps the
    /// OutboundBuffer's dispatcher in a `RetryingDispatcher` configured
    /// with the parsed values; transient dispatcher failures are
    /// retried with exponential backoff up to `max_retries`, then
    /// DELIVERY_EXHAUSTED fires at exhaustion.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry: Option<RetryPolicyConfig>,

    /// SCE Mesh §mesh-16.7 row 10 — per-binding authorization policy.
    /// Opt-in: absent / `required: false` ⇒ transport-level rejection
    /// signals stay classified as row 1 TRANSPORT_UNAVAILABLE or
    /// row 8 PEER_PARTITIONED. Present + `required: true` ⇒ the
    /// generated router observes the transport's reject signal and
    /// classifies known-auth-fail patterns (Zenoh ZException::what()
    /// containing certificate/tls/auth tokens; SOMEIP SD denial code)
    /// as row 10 UNAUTHORIZED. Per-transport feature gates apply —
    /// custom_tcp / shm bindings cannot opt in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<AuthPolicyConfig>,

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
    validate_reply_from(&cfg)?;
    validate_stage_pool_transport(&cfg)?;
    validate_outbound_buffer(&cfg)?;
    validate_retry_policy(&cfg)?;
    validate_auth_policy(&cfg)?;
    validate_discovery_not_supported(&cfg)?;
    validate_synth_invoke_infix(&cfg)?;
    validate_partitions_schema(&cfg)?;
    validate_someip_scxml_invoke_service_ids(&cfg)?;
    validate_someip_liveness_service_ids(&cfg)?;
    validate_someip_machine_liveness_service_ids(&cfg)?;
    validate_platform_class_os_consistency(&cfg)?;
    validate_scheduler_cooperative_stack_budget(&cfg)?;
    validate_worker_slot_budget_required_when_cooperative(&cfg)?;
    validate_keepalive_jitter_required_when_cooperative(&cfg)?;
    validate_machine_scheduler_worker_capacity(&cfg)?;
    validate_machine_scheduler_link_concurrency(&cfg)?;
    validate_machine_timer_wheel_capacity(&cfg)?;
    validate_links(&cfg)?;
    validate_pool_defaults(&cfg)?;

    Ok(cfg)
}

/// SCE Protocol-Synthesis RFC §synth-5-K line 2517-2519 parse-time typo guard
/// (`deploy/stage-copy-policy-unknown`). Walks every machine's
/// `pool_defaults.stage_copy_policy` String field and rejects values
/// outside the closed set [`StageCopyPolicy::ALL`] = {warn, error,
/// forbid}. Mirrors [`validate_links`]'s `LinkDriverUnknown` pattern
/// — String at schema time, closed-allowlist validator post-parse.
fn validate_pool_defaults(cfg: &DeployConfig) -> Result<(), DeployError> {
    for device in cfg.topology.values() {
        for (machine_name, machine) in device.machines.iter() {
            let Some(pool_defaults) = machine.pool_defaults.as_ref() else {
                continue;
            };
            if StageCopyPolicy::from_wire_str(&pool_defaults.stage_copy_policy).is_none() {
                let candidates: Vec<String> = StageCopyPolicy::ALL
                    .iter()
                    .map(|s| (*s).to_string())
                    .collect();
                let candidates_list = candidates.join(", ");
                return Err(DeployError::StageCopyPolicyUnknown {
                    machine: machine_name.clone(),
                    value: pool_defaults.stage_copy_policy.clone(),
                    candidates,
                    candidates_list,
                });
            }
        }
    }
    Ok(())
}

/// `machines.<n>.links.<name>` parse-time validators (§synth-5-K).
///
/// Five intra-link checks (RFC §synth-5-K lines 2421-2503):
///   1. `deploy/link-driver-unknown` — driver in known baseline or
///      forge cross-doc registry. Cross-doc lookup runs in the
///      orchestrator pass; this parse-time pass only checks the
///      built-in `{lwip_udp, lwip_tcp}` baseline + emits a candidate
///      list pre-populated from the baseline (orchestrator pass
///      extends it).
///   2. `deploy/link-mtu-below-driver-floor` — if `mtu_bytes` declared
///      AND driver is in known baseline, check against floor.
///   3. `deploy/link-expected-p99-exceeds-mtu` — if both fields
///      declared, check `expected_p99_bytes <= mtu_bytes`.
///   4. `deploy/link-burst-pps-missing-on-isr-dispatch` — if the
///      resolved `rx_dispatch == IsrToPool` AND `burst_pps.is_none()`,
///      fire.
///   5. `deploy/link-mtu-missing-on-fragmenting-link` — if
///      `domain_attrs.trust_class == EstablishedSession` (the only
///      class permitted to carry Fragment traffic per RFC §synth-5-M line
///      2731) AND `mtu_bytes.is_none()`, fire.
///
/// The cross-doc validators `deploy/link-not-declared-in-deploy` +
/// `deploy/link-not-declared-in-forge` need the forge
/// cross-doc registry; they live in [`validate_links_cross_doc`] and
/// run from the orchestrator pass, not from `parse_str`.
///
/// `deploy/link-burst-absorption-insufficient` + `deploy/link-rx-
/// dispatch-worker-tick-on-high-burst` live in
/// [`validate_links_burst_invariants`] — both
/// require RX pool slot_count cross-doc resolution, so they run from
/// the orchestrator pass as well.
/// Known-driver baseline carrying protocol class + min-MTU floor.
///
/// Single source of truth for the driver allowlist. Each core driver
/// implements exactly one protocol class (per RFC §synth-5-C lines 765-771 +
/// §synth-8 Q8 line 3747); co-locating the class with the driver name
/// keeps `KNOWN_DRIVERS` the authoritative source — no parallel map
/// to drift against.
///
/// IP-stack drivers carry an IP-encapsulation floor:
///   - `lwip_udp = 28` (IPv4 minimum header)
///   - `lwip_tcp = 40` (IPv4 + TCP minimum)
///   - `websocket_tcp = 40` (runs over IPv4 + TCP — same
///     encapsulation floor as `lwip_tcp`; the per-frame
///     WebSocket header is application-protocol framing
///     carried by the §synth-5-B framer codec, not by the driver
///     MTU floor. Spec §synth-8 Q8 line 3747 names the driver;
///     spec §synth-5-C row 4 (line 770) names the class).
///
/// Non-IP drivers carry floor `0` to mark "skip floor check"
/// explicitly — the §synth-5-B framer codec carries the frame-size
/// invariant at the protocol-decoder layer instead:
///   - `serial_uart = 0` (UART has no IP-stack overhead;
///     SCE Protocol-Synthesis RFC §synth-5-C line 729 + spec C11 atomic)
///
/// Unknown drivers fall through to forge cross-doc registry
/// lookup in the orchestrator pass; the parse-time validator
/// silent-skips the floor check for them.
const KNOWN_DRIVERS: &[(&str, LinkClass, u32)] = &[
    ("lwip_tcp", LinkClass::Tcp, 40),
    ("lwip_udp", LinkClass::Udp, 28),
    ("serial_uart", LinkClass::Serial, 0),
    ("websocket_tcp", LinkClass::Websocket, 40),
];

fn known_driver_floor(driver: &str) -> Option<u32> {
    KNOWN_DRIVERS
        .iter()
        .find_map(|(name, _class, floor)| if *name == driver { Some(*floor) } else { None })
}

/// Returns the protocol class implemented by `driver`, or `None` if
/// the driver is not in the SCE-side allowlist. Target-plugin
/// drivers (declared via `extern_symbols.target_plugin`) are
/// silent-skipped — their class-check rides §synth-5-I plugin contract.
fn known_driver_class(driver: &str) -> Option<LinkClass> {
    KNOWN_DRIVERS
        .iter()
        .find_map(|(name, class, _floor)| if *name == driver { Some(*class) } else { None })
}

fn validate_links(cfg: &DeployConfig) -> Result<(), DeployError> {
    for device in cfg.topology.values() {
        for (machine_name, machine) in device.machines.iter() {
            for (link_name, link) in machine.links.iter() {
                // 1. driver-unknown — parse-time fires only when the
                //    driver name is absent from the baseline. The
                //    cross-doc orchestrator pass extends the candidate
                //    set with forge `<sce:link>` doc names + may emit
                //    the same code at that layer; here we only catch
                //    the no-forge-link case.
                if known_driver_floor(&link.driver).is_none() {
                    // Stable candidate baseline (sorted). Orchestrator
                    // pass adds forge link-doc names if available.
                    let mut candidates: Vec<String> = KNOWN_DRIVERS
                        .iter()
                        .map(|(n, _class, _floor)| (*n).to_string())
                        .collect();
                    candidates.sort();
                    let candidates_list = candidates.join(", ");
                    return Err(DeployError::LinkDriverUnknown {
                        machine: machine_name.clone(),
                        link_name: link_name.clone(),
                        driver: link.driver.clone(),
                        candidates,
                        candidates_list,
                    });
                }

                // 2. mtu-below-driver-floor — only fires when driver IS
                //    in baseline AND mtu_bytes is declared (Optional).
                if let Some(mtu) = link.mtu_bytes {
                    if let Some(floor) = known_driver_floor(&link.driver) {
                        if mtu < floor {
                            return Err(DeployError::LinkMtuBelowDriverFloor {
                                machine: machine_name.clone(),
                                link_name: link_name.clone(),
                                driver: link.driver.clone(),
                                declared_mtu: mtu,
                                driver_floor: floor,
                            });
                        }
                    }
                }

                // 3. expected-p99-exceeds-mtu — both fields declared.
                if let (Some(p99), Some(mtu)) = (link.expected_p99_bytes, link.mtu_bytes) {
                    if p99 > mtu {
                        return Err(DeployError::LinkExpectedP99ExceedsMtu {
                            machine: machine_name.clone(),
                            link_name: link_name.clone(),
                            expected_p99_bytes: p99,
                            mtu_bytes: mtu,
                        });
                    }
                }

                // 4. burst-pps-missing-on-isr-dispatch — resolved
                //    rx_dispatch via [`LinkConfig::resolved_rx_dispatch`].
                //    The conditional default makes
                //    `burst_pps` declared → `IsrToPool` already; the
                //    failure mode is `rx_dispatch: isr_to_pool` set
                //    explicitly without `burst_pps`, OR future user-
                //    error of declaring isr without rate. With the
                //    conditional default this fires only when the
                //    author explicitly sets `rx_dispatch: isr_to_pool`.
                if matches!(link.resolved_rx_dispatch(), RxDispatch::IsrToPool)
                    && link.burst_pps.is_none()
                {
                    return Err(DeployError::LinkBurstPpsMissingOnIsrDispatch {
                        machine: machine_name.clone(),
                        link_name: link_name.clone(),
                    });
                }

                // 5. mtu-missing-on-fragmenting-link —
                //    under-approximation per
                //    [`DiagnosticCode::MeshDeployLinkMtuMissingOnFragmentingLink`]
                //    doc comment.
                if let Some(domain) = link.domain_attrs.as_ref() {
                    if matches!(domain.trust_class, TrustClass::EstablishedSession)
                        && link.mtu_bytes.is_none()
                    {
                        return Err(DeployError::LinkMtuMissingOnFragmentingLink {
                            machine: machine_name.clone(),
                            link_name: link_name.clone(),
                        });
                    }
                }

                // ── Anti-flood + stateless_accept ──
                //
                // The five checks below mirror the spec-section walk
                // order of `RFC §synth-5-K lines 2449-2473`:
                //  - 6. Dead-config rejection (line 2454-2459) —
                //    anti-flood / stateless_accept on a non-arming
                //    link. Surfaced FIRST in deterministic walk order
                //    because the diagnostic names a class of error
                //    that supersedes the missing-field codes: if the
                //    trust_class is wrong, "*-missing" advice would
                //    mislead the author.
                //  - 7. session_arming_quota-missing (line 2449-2451).
                //  - 8. accept_rate config missing (line 2452-2453).
                //  - 9. stateless_accept required on untrusted_source
                //    (line 2463-2465).
                //  - 10. stateless_accept key-rotation vs lifetime
                //    invariant (line 2470-2473).
                let trust_class = link.domain_attrs.as_ref().map(|d| d.trust_class);
                let untrusted_source = link
                    .domain_attrs
                    .as_ref()
                    .is_some_and(|d| d.untrusted_source);

                // 6. session-arming-fields-on-non-arming-link.
                // The fields are "dead config" when trust_class is NOT
                // session_arming, OR when domain_attrs is absent
                // entirely (no Accepting.* can ever instantiate
                // without a trust class declaration).
                let is_session_arming = matches!(trust_class, Some(TrustClass::SessionArming));
                if !is_session_arming {
                    let mut offending: Vec<&str> = Vec::new();
                    if link.session_arming_quota.is_some() {
                        offending.push("session_arming_quota");
                    }
                    if link.accept_rate_per_sec.is_some() {
                        offending.push("accept_rate_per_sec");
                    }
                    if link.accept_rate_burst.is_some() {
                        offending.push("accept_rate_burst");
                    }
                    if link.accept_rate_table_capacity.is_some() {
                        offending.push("accept_rate_table_capacity");
                    }
                    if link.accepting_inactivity_timeout_ms.is_some() {
                        offending.push("accepting_inactivity_timeout_ms");
                    }
                    if link.stateless_accept.is_some() {
                        offending.push("stateless_accept");
                    }
                    if !offending.is_empty() {
                        let offending_fields = offending.join(", ");
                        // `actual` carries the offending trust_class
                        // value (or "<absent>" when domain_attrs is
                        // entirely missing) so the wire payload names
                        // the axis the author got wrong.
                        let trust_class_str = trust_class
                            .map_or_else(|| "<absent>".to_string(), |tc| tc.as_str().to_string());
                        return Err(DeployError::SessionArmingFieldsOnNonArmingLink {
                            machine: machine_name.clone(),
                            link_name: link_name.clone(),
                            trust_class: trust_class_str,
                            offending_fields,
                        });
                    }
                }

                // 7. session-arming-quota-missing — fires when
                // `trust_class: session_arming` AND no quota.
                if is_session_arming && link.session_arming_quota.is_none() {
                    return Err(DeployError::SessionArmingQuotaMissing {
                        machine: machine_name.clone(),
                        link_name: link_name.clone(),
                    });
                }

                // 8. accept-rate-config-missing — fires when
                // `trust_class: session_arming` AND any of the two
                // load-bearing fields are missing (spec line 2453
                // names `accept_rate_per_sec` OR `accept_rate_burst`).
                if is_session_arming {
                    let mut missing: Vec<&str> = Vec::new();
                    if link.accept_rate_per_sec.is_none() {
                        missing.push("accept_rate_per_sec");
                    }
                    if link.accept_rate_burst.is_none() {
                        missing.push("accept_rate_burst");
                    }
                    if !missing.is_empty() {
                        return Err(DeployError::AcceptRateConfigMissing {
                            machine: machine_name.clone(),
                            link_name: link_name.clone(),
                            missing_fields: missing.join(", "),
                        });
                    }
                }

                // 9. stateless-accept-required-on-untrusted-source.
                if untrusted_source && link.stateless_accept.is_none() {
                    return Err(DeployError::StatelessAcceptRequiredOnUntrustedSource {
                        machine: machine_name.clone(),
                        link_name: link_name.clone(),
                    });
                }

                // 10. stateless-accept-key-rotation-shorter-than-lifetime.
                // Spec line 2470-2473 invariant verbatim:
                //   `key_rotation_s × 1000 > 2 × cookie_lifetime_ms`
                if let Some(sa) = link.stateless_accept.as_ref() {
                    let rotation_ms = sa.key_rotation_s as u64 * 1000;
                    let lifetime_doubled = sa.cookie_lifetime_ms as u64 * 2;
                    if rotation_ms <= lifetime_doubled {
                        return Err(DeployError::StatelessAcceptKeyRotationShorterThanLifetime {
                            machine: machine_name.clone(),
                            link_name: link_name.clone(),
                            key_rotation_s: sa.key_rotation_s,
                            cookie_lifetime_ms: sa.cookie_lifetime_ms,
                            rotation_ms,
                            lifetime_doubled,
                        });
                    }
                }

                // 11. session-arming-quota-vs-peer-table-invariant-violated.
                // Spec line 2460-2462 invariant verbatim:
                //   `session_arming_quota × max_handshake_time_s ≤
                //    peer_table.capacity`
                // (else a slow legitimate handshake can be evicted under
                // attack — the attacker churns the quota faster than the
                // per-peer table absorbs).
                //
                // Silent-skip when any of the three inputs is absent
                // (stateless_accept block omitted, peer_table sub-block
                // omitted, max_handshake_time_s sibling omitted, or
                // session_arming_quota omitted at link level) — per
                // the absent-input silent-skip discipline. session_arming_quota
                // missing on a session_arming link is already caught by
                // check #7 above, so the silent-skip here doesn't mask
                // that case.
                if let Some(sa) = link.stateless_accept.as_ref() {
                    if let (
                        Some(peer_table),
                        Some(max_handshake_time_s),
                        Some(session_arming_quota),
                    ) = (
                        sa.peer_table.as_ref(),
                        sa.max_handshake_time_s,
                        link.session_arming_quota,
                    ) {
                        let product = session_arming_quota as u64 * max_handshake_time_s as u64;
                        if product > peer_table.capacity as u64 {
                            return Err(
                                DeployError::SessionArmingQuotaVsPeerTableInvariantViolated {
                                    machine: machine_name.clone(),
                                    link_name: link_name.clone(),
                                    session_arming_quota,
                                    max_handshake_time_s,
                                    peer_table_capacity: peer_table.capacity,
                                    product,
                                },
                            );
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

/// Cross-doc link-name resolution (§synth-5-K).
///
/// Two validators run after the forge cross-doc registry is populated:
///   - `deploy/link-not-declared-in-deploy` — every forge
///     `<scxml sce:kind="link" name="X">` document declared in the
///     registry must have at least one `machines.<n>.links.X` entry
///     in the deploy.
///   - `deploy/link-not-declared-in-forge` — every
///     `machines.<n>.links.X` entry must have a matching forge
///     `<sce:link>` document name in the registry.
///
/// Returns the first failure encountered (deterministic order: iterate
/// devices → machines → links in declaration order).
///
/// `forge_link_names` is the sorted set of names returned by
/// `SceCrossDocRegistry::names_of_kind(ScxmlDocKind::Link)`. Callers
/// supply it explicitly to keep this module free of a `forge::*`
/// dependency.
pub fn validate_links_cross_doc(
    cfg: &DeployConfig,
    forge_link_names: &[String],
) -> Result<(), DeployError> {
    use std::collections::BTreeSet;

    // Build the sorted deploy-side link-name union across all machines.
    let mut deploy_link_names: BTreeSet<&str> = BTreeSet::new();
    for device in cfg.topology.values() {
        for machine in device.machines.values() {
            for name in machine.links.keys() {
                deploy_link_names.insert(name.as_str());
            }
        }
    }
    let forge_set: BTreeSet<&str> = forge_link_names.iter().map(|s| s.as_str()).collect();

    // Pass A: forge → deploy. Every forge link doc must have at least
    // one deploy entry across the build.
    for forge_name in &forge_set {
        if !deploy_link_names.contains(forge_name) {
            let candidates: Vec<String> = deploy_link_names.iter().map(|s| s.to_string()).collect();
            let candidates_list = candidates.join(", ");
            return Err(DeployError::LinkNotDeclaredInDeploy {
                link_name: (*forge_name).to_string(),
                candidates,
                candidates_list,
            });
        }
    }

    // Pass B: deploy → forge. Every deploy entry must have a matching
    // forge link doc name. Iterate per-machine so the error carries
    // the host machine name.
    for device in cfg.topology.values() {
        for (machine_name, machine) in device.machines.iter() {
            for link_name in machine.links.keys() {
                if !forge_set.contains(link_name.as_str()) {
                    let candidates: Vec<String> = forge_set.iter().map(|s| s.to_string()).collect();
                    let candidates_list = candidates.join(", ");
                    return Err(DeployError::LinkNotDeclaredInForge {
                        machine: machine_name.clone(),
                        link_name: link_name.clone(),
                        candidates,
                        candidates_list,
                    });
                }
            }
        }
    }

    Ok(())
}

/// SCE Protocol-Synthesis RFC §synth-5-C lines 765-771 + §synth-8 Q8 line 3747 cross-
/// doc consistency check between forge `<sce:link-class>` and the
/// deploy.yaml `driver:` allowlist entry.
///
/// Each entry in [`KNOWN_DRIVERS`] declares its implemented class;
/// this validator joins the deploy-side driver string against the
/// forge-side `LinkModel.class` and fires
/// `deploy/link-driver-class-mismatch` when they disagree.
///
/// Silent-skip cases (per `[[feedback-silently-broken-hooks]]` —
/// "data unavailable" must not synthesize false errors):
///   - Driver not in `KNOWN_DRIVERS` — falls through to target-
///     plugin path; class-check rides §synth-5-I plugin contract there.
///   - No matching forge `LinkModel` for the deploy link name —
///     `validate_links_cross_doc` already gates this case as
///     `deploy/link-not-declared-in-forge`; reaching this point
///     implies the join succeeds.
///
/// Iteration order: devices → machines → links in declaration
/// order. First failure short-circuits.
pub fn validate_link_driver_class_consistency(
    cfg: &DeployConfig,
    forge_link_models: &std::collections::HashMap<String, &crate::forge::model::LinkModel>,
) -> Result<(), DeployError> {
    for device in cfg.topology.values() {
        for (machine_name, machine) in device.machines.iter() {
            for (link_name, link) in machine.links.iter() {
                let Some(expected_class) = known_driver_class(&link.driver) else {
                    continue;
                };
                let Some(forge_link) = forge_link_models.get(link_name) else {
                    continue;
                };
                let declared_class = forge_link.class;
                if declared_class == expected_class {
                    continue;
                }
                let driver_candidates: Vec<String> = KNOWN_DRIVERS
                    .iter()
                    .filter(|(_, class, _)| *class == declared_class)
                    .map(|(name, _, _)| (*name).to_string())
                    .collect();
                let driver_candidates_list = driver_candidates.join(", ");
                return Err(DeployError::LinkDriverClassMismatch(Box::new(
                    crate::mesh::error::LinkDriverClassMismatchPayload {
                        machine: machine_name.clone(),
                        link_name: link_name.clone(),
                        driver: link.driver.clone(),
                        declared_class: declared_class.to_string(),
                        expected_class: expected_class.to_string(),
                        driver_candidates,
                        driver_candidates_list,
                    },
                )));
            }
        }
    }
    Ok(())
}

/// Cross-document join for §synth-5-K + §synth-5-M validators that need the RX
/// pool slot count of a deploy-declared link.
///
/// Three steps, each silent-skipping on absence per the absent-input precedent
/// (the [`MachineSchedulerConfig::tick_period_us`] populator): forge
/// `<sce:link name=link_name>` must exist, its `<sce:rx-pool ref=Y>`
/// must be declared, and that pool name must resolve to a
/// [`crate::forge::model::BufferPoolModel`] entry in the supplied
/// registry map. When all three resolve, returns the pool name + its
/// declared `slot_count` + the [`BufferPoolVariant`] discriminant so
/// consumers can distinguish reassembly bindings (§synth-5-M) from regular
/// RX (§synth-5-K burst-rate) cases without re-joining.
///
/// Single source of truth for the 3-way join.
/// Callers are validators in this module (`validate_links_burst_invariants`)
/// and in [`crate::forge::validate`] / cross-doc orchestration paths
/// where the same join is required. The function avoids `forge::*`
/// imports beyond the explicit `BufferPoolVariant` re-export to keep
/// the mesh module's dependency surface minimal — the
/// `BufferPoolModel` is consumed by reference through the registry map.
pub fn resolve_link_rx_pool_slot_count<'a>(
    link_name: &str,
    forge_link_models: &'a std::collections::HashMap<String, &'a crate::forge::model::LinkModel>,
    pool_registry_full: &'a std::collections::HashMap<
        String,
        &'a crate::forge::model::BufferPoolModel,
    >,
) -> Option<(&'a str, u32, &'a crate::forge::model::BufferPoolVariant)> {
    let forge_link = forge_link_models.get(link_name)?;
    let rx_pool_ref = forge_link.rx_pool.as_deref()?;
    let pool = pool_registry_full.get(rx_pool_ref)?;
    Some((rx_pool_ref, pool.slot_count, &pool.variant))
}

/// SCE Protocol-Synthesis RFC §synth-5-K lines 2489-2500 (`deploy/link-burst-
/// absorption-insufficient` + `deploy/link-rx-dispatch-worker-tick-
/// on-high-burst`) — cross-doc validators that consume
/// [`resolve_link_rx_pool_slot_count`] to check the cooperative-tick
/// drain capacity against the declared inbound burst rate.
///
/// Silent-skips when any of the following are absent (per the
/// absent-input precedent — `[[feedback-silently-broken-hooks]]` discipline,
/// "data unavailable" must not synthesize false errors):
///   - `link.burst_pps` (no declared burst rate to test against)
///   - `scheduler.tick_period_us` (no cooperative tick to bound the
///     drain capacity by; the failure mode is only meaningful for
///     `kind: cooperative` per spec line 2489)
///   - cross-doc join (forge link / rx_pool / BufferPoolModel) per
///     [`resolve_link_rx_pool_slot_count`]
///
/// Returns the first failure (deterministic order: devices → machines
/// → links in declaration order, then the spec's burst-absorption
/// invariant before the worker_tick-overrun invariant).
pub fn validate_links_burst_invariants(
    cfg: &DeployConfig,
    forge_link_models: &std::collections::HashMap<String, &crate::forge::model::LinkModel>,
    pool_registry_full: &std::collections::HashMap<String, &crate::forge::model::BufferPoolModel>,
) -> Result<(), DeployError> {
    for device in cfg.topology.values() {
        for (machine_name, machine) in device.machines.iter() {
            let Some(tick_period_us) = machine.scheduler.as_ref().and_then(|s| s.tick_period_us)
            else {
                continue;
            };
            for (link_name, link) in machine.links.iter() {
                let Some(burst_pps) = link.burst_pps else {
                    continue;
                };
                let Some((pool_name, slot_count, _variant)) = resolve_link_rx_pool_slot_count(
                    link_name,
                    forge_link_models,
                    pool_registry_full,
                ) else {
                    continue;
                };

                // ── `deploy/link-burst-absorption-insufficient` ──
                // Spec verbatim (line 2489-2495):
                //   `slot_count × ticks_per_second / burst_pps < 1.0`
                //   with safety factor 2.0
                // ticks_per_second = 1_000_000 / tick_period_us.
                // Re-arranged to integer math (no float):
                //   slot_count × 1_000_000 < burst_pps × tick_period_us × 2
                // The safety factor lives on the RHS to keep both
                // operands as u64; overflow-safe because all four inputs
                // are u32.
                let drain_capacity_u64 = slot_count as u64 * 1_000_000;
                let burst_load_u64 = burst_pps as u64 * tick_period_us as u64 * 2;
                if drain_capacity_u64 < burst_load_u64 {
                    // drain_per_second = slot_count × ticks_per_second
                    //                  = slot_count × 1_000_000 /
                    //                    tick_period_us
                    // For the message text we expose the effective
                    // drain-per-second WITHOUT the safety factor so the
                    // numbers reflect raw pool capacity; the burst
                    // comparison itself uses the safety factor per
                    // spec.
                    let drain_per_second = (slot_count as u64 * 1_000_000 / tick_period_us as u64)
                        .min(u32::MAX as u64) as u32;
                    return Err(DeployError::LinkBurstAbsorptionInsufficient {
                        machine: machine_name.clone(),
                        link_name: link_name.clone(),
                        pool_name: pool_name.to_string(),
                        slot_count,
                        burst_pps,
                        tick_period_us,
                        drain_per_second,
                    });
                }

                // ── `deploy/link-rx-dispatch-worker-tick-on-high-burst` ──
                // Spec verbatim (line 2496-2500):
                //   `rx_dispatch: worker_tick` declared AND
                //   `burst_pps × tick_period_us > slot_count` (one tick
                //   of arrivals overruns the pool).
                // Note the spec compares against tick_period_us in
                // microseconds; one tick = tick_period_us / 1_000_000
                // seconds, so arrivals per tick = burst_pps ×
                // tick_period_us / 1_000_000.
                if matches!(link.resolved_rx_dispatch(), RxDispatch::WorkerTick) {
                    let arrivals_per_tick_u64 =
                        (burst_pps as u64 * tick_period_us as u64) / 1_000_000;
                    if arrivals_per_tick_u64 > slot_count as u64 {
                        let arrivals_per_tick = arrivals_per_tick_u64.min(u32::MAX as u64) as u32;
                        return Err(DeployError::LinkRxDispatchWorkerTickOnHighBurst {
                            machine: machine_name.clone(),
                            link_name: link_name.clone(),
                            pool_name: pool_name.to_string(),
                            slot_count,
                            burst_pps,
                            tick_period_us,
                            arrivals_per_tick,
                        });
                    }
                }
            }
        }
    }
    Ok(())
}

/// SCE Protocol-Synthesis RFC §synth-5-M lines 2946-2999 cross-doc validators for
/// reassembly-variant buffer pools bound to deploy-declared links.
///
/// Six codes ride through this one entry point;
/// each fires from a join of `deploy.links.<X>` → forge `<sce:link
/// name=X>` → its `<sce:rx-pool ref=Y>` → `BufferPoolModel` for Y
/// (resolved via [`resolve_link_rx_pool_slot_count`]). The validators
/// emit [`crate::forge::error::ValidationError`] rather than
/// [`DeployError`] because the spec anchor (§synth-5-M) lives in the forge
/// kinds catalog, not the deploy schema chapter — and the
/// `mem/*` + `reassembly/*` slash-paths align with the validation
/// stage in `[[SCE_ERROR_CONTRACT]]`.
///
/// Each validator silent-skips on absent inputs. The
/// six are ordered by spec line in the same way the diagnostic catalog
/// presents them, so a deterministic first-failure return reproduces
/// the catalog reading order.
///
/// Reassembly-specific axes use the bound pool's
/// [`crate::forge::model::BufferPoolVariant::Reassembly`] config; the
/// "regular RX pool" axis (codes #3 expected-fragmentation-rate-high
/// uses `(expected_p99 - regular_pool.slot_size) / expected_p99 >
/// 0.25` per spec line 2902) walks every `BufferPoolVariant::Default`
/// pool bound to the link by scanning both rx_pool and tx_pool refs
/// for completeness, but silent-skips when no Default
/// pool is bound to the link — the formula references "the regular
/// RX pool's slot_size" which doesn't exist for the link in that
/// scenario.
pub fn validate_reassembly_cross_doc(
    cfg: &DeployConfig,
    forge_link_models: &std::collections::HashMap<String, &crate::forge::model::LinkModel>,
    pool_registry_full: &std::collections::HashMap<String, &crate::forge::model::BufferPoolModel>,
    // Orchestrator-resolved listener-link set
    // (`<link_name>` for every `session_arming` link whose machine's
    // source SCXML carries any `Accepting.*` substate). Drives the
    // session_arming branch of the #4 check below: when the bound
    // link is a listener, the binding silently rebinds to the
    // synthesized `established_session` sibling (RFC §synth-5-C lines
    // 802-803 + 821-825 + 2782-2783); when it is not, the binding
    // has no valid landing site and the validator fires
    // `reassembly/binding-on-unpaired-listener` in place of the
    // historic `reassembly/untrusted-link-binding` for the
    // session-arming subcase.
    //
    // Pass `&BTreeSet::new()` from deploy-unaware test paths and
    // single-doc compile paths (no SCXML axis available); the
    // session_arming branch then ALWAYS falls through to the
    // `binding-on-unpaired-listener` code, surfacing the binding
    // mistake at the only check that can reach it. Untrusted +
    // EstablishedSession branches are unaffected by this parameter.
    listener_links: &std::collections::BTreeSet<String>,
) -> Result<(), Box<crate::forge::error::ValidationError>> {
    use crate::forge::error::ValidationError;
    use crate::forge::model::BufferPoolVariant;

    for device in cfg.topology.values() {
        for (machine_name, machine) in device.machines.iter() {
            // Scheduler-derived inputs for #6 stage-copy-wcet (spec
            // line 2995-2999). Silent-skip when any input absent.
            let worker_slot_budget_us = machine
                .scheduler
                .as_ref()
                .and_then(|s| s.worker_slot_budget_us);
            let (clock_freq_mhz, memcpy_cycles_per_byte) =
                machine.platform.as_ref().map_or((None, None), |p| {
                    (p.clock_freq_mhz, p.memcpy_cycles_per_byte)
                });

            for (link_name, link) in machine.links.iter() {
                let Some((pool_name, _slot_count, variant)) = resolve_link_rx_pool_slot_count(
                    link_name,
                    forge_link_models,
                    pool_registry_full,
                ) else {
                    continue;
                };
                // accept_stage_copy_rate lookup — same join the
                // resolver did, surfacing the forge LinkModel reference
                // for the opt-out semantics. The lookup cannot fail
                // when the resolver succeeded (3-way join shares the
                // first step).
                let forge_link = forge_link_models.get(link_name).copied();
                // Re-fetch the full BufferPoolModel for slot_size and
                // reassembly-specific config; the resolver returns only
                // slot_count + variant ref to keep its signature lean.
                // Both halves come from the same registry entry, so the
                // second lookup is O(1) and cannot fail when the
                // resolver succeeded.
                let pool = pool_registry_full.get(pool_name).expect(
                    "resolver succeeded ⇒ pool present in registry; \
                     consistent map view assumed",
                );
                let slot_size = pool.slot_size;

                // ── #1 mem/reassembly-slot-size-below-declared-mtu ──
                // Spec line 2946. Applies to ANY RX-bound pool (the
                // happy-path datagram must fit in a single slot,
                // regardless of variant). Silent-skip when mtu_bytes
                // is absent — the under-approximation already lives
                // on `MeshDeployLinkMtuMissingOnFragmentingLink` at
                // parse time.
                if let Some(mtu_bytes) = link.mtu_bytes {
                    if slot_size < mtu_bytes {
                        return Err(Box::new(
                            ValidationError::MemReassemblySlotSizeBelowDeclaredMtu {
                                pool_name: pool_name.to_string(),
                                slot_size,
                                mtu_bytes,
                                machine: machine_name.clone(),
                                link_name: link_name.clone(),
                            },
                        ));
                    }
                }

                // ── Reassembly-variant-only checks (#2/#4/#5) ──
                // Spec lines 2947-2949 + 2964-2969 + 2970-2975 apply
                // only when the bound pool carries the
                // `BufferPoolVariant::Reassembly` discriminator. The
                // `Default` arm silent-skips them entirely; this
                // matches the spec's "reassembly pool bound to a
                // link" framing — non-reassembly bindings are
                // governed by the spec's §synth-5-K / §synth-5-E plain RX
                // pathways.
                if let BufferPoolVariant::Reassembly(reassembly_cfg) = variant {
                    // ── #2 reassembly/max-fragments-insufficient-for-mtu ──
                    // Spec line 2947-2949 verbatim. Silent-skip when
                    // link.mtu_bytes absent (same reasoning as #1).
                    if let Some(mtu_bytes) = link.mtu_bytes {
                        let required = (reassembly_cfg.max_fragments_per_message as u64
                            * mtu_bytes as u64)
                            .min(u32::MAX as u64) as u32;
                        if slot_size < required {
                            return Err(Box::new(
                                ValidationError::ReassemblyMaxFragmentsInsufficientForMtu {
                                    pool_name: pool_name.to_string(),
                                    slot_size,
                                    max_fragments_per_message: reassembly_cfg
                                        .max_fragments_per_message,
                                    mtu_bytes,
                                    required,
                                    machine: machine_name.clone(),
                                    link_name: link_name.clone(),
                                },
                            ));
                        }
                    }

                    // ── #4 reassembly/untrusted-link-binding +
                    //    reassembly/binding-on-unpaired-listener ──
                    //
                    // Spec line 2964-2969 frames `untrusted-link-
                    // binding` as the rejection for non-
                    // `established_session` bindings; spec lines
                    // 2982-2994 (`binding-on-unpaired-listener`)
                    // narrows the `session_arming` subcase to the
                    // listener-pair-aware path. Routing:
                    //
                    //   trust_class = EstablishedSession → pass
                    //   trust_class = SessionArming + listener →
                    //     silent-pass (binding rebinds to the
                    //     synthesized Sibling EstablishedSession
                    //     instance, RFC §synth-5-C lines 821-825)
                    //   trust_class = SessionArming + non-listener →
                    //     fire `reassembly/binding-on-unpaired-listener`
                    //   trust_class = Untrusted → fire
                    //     `reassembly/untrusted-link-binding`
                    //     (Untrusted-only since the listener-pair split)
                    //
                    // Silent-skip when domain_attrs entirely absent
                    // — that scenario is named by #5 below.
                    if let Some(domain) = link.domain_attrs.as_ref() {
                        match domain.trust_class {
                            TrustClass::EstablishedSession => {
                                // Happy path — reassembly binding is
                                // valid on post-handshake trust class.
                            }
                            TrustClass::SessionArming => {
                                if !listener_links.contains(link_name) {
                                    // No `Accepting.*` substate on
                                    // the machine's source SCXML;
                                    // the listener-pair walker did
                                    // not synthesize the Sibling
                                    // EstablishedSession instance,
                                    // so the binding has no valid
                                    // landing site. RFC §synth-5-M lines
                                    // 2982-2994.
                                    return Err(Box::new(
                                        ValidationError::ReassemblyBindingOnUnpairedListener {
                                            pool_name: pool_name.to_string(),
                                            machine: machine_name.clone(),
                                            link_name: link_name.clone(),
                                        },
                                    ));
                                }
                                // Listener — binding auto-rebinds to
                                // the synthesized Sibling
                                // EstablishedSession instance. Falls
                                // through to subsequent checks
                                // (#3 stage-copy rate, #6 stage-copy
                                // WCET) which evaluate against the
                                // same field set.
                            }
                            TrustClass::Untrusted => {
                                return Err(Box::new(
                                    ValidationError::ReassemblyUntrustedLinkBinding {
                                        pool_name: pool_name.to_string(),
                                        trust_class: domain.trust_class.as_str().to_string(),
                                        machine: machine_name.clone(),
                                        link_name: link_name.clone(),
                                    },
                                ));
                            }
                        }
                    } else {
                        // ── #5 reassembly/trust-class-missing-on-fragmenting-link ──
                        // Spec line 2970-2975:
                        // domain_attrs absent on a reassembly-bound
                        // link triggers the diagnostic; the
                        // "declared without trust_class" case is
                        // already parse-rejected by
                        // `LinkDomainAttrs.trust_class` required-when-
                        // block-declared shape (parse-time).
                        return Err(Box::new(
                            ValidationError::ReassemblyTrustClassMissingOnFragmentingLink {
                                pool_name: pool_name.to_string(),
                                machine: machine_name.clone(),
                                link_name: link_name.clone(),
                            },
                        ));
                    }

                    // ── Declared-consumption — reassembly/
                    //    per-peer-quota-build-invariant-violated ──
                    //
                    // Spec line 2841-2861 verbatim:
                    //   `peer_table.capacity × per-peer-quota ≥ slot_count`
                    //
                    // This check lives here (rather than a
                    // reassembly-side consumer) because the
                    // `peer_table.capacity` source is available at
                    // this join.
                    //
                    // Silent-skip when:
                    //   - link.stateless_accept absent (no session_arming
                    //     hardening block on the link — the peer_table
                    //     source is unreachable)
                    //   - stateless_accept.peer_table absent (block
                    //     declared but capacity not enumerated)
                    //   - pool.variant.per_peer_quota == 0 (parse-time
                    //     rejection already covers; defense-in-depth)
                    //
                    // The check fires only when peer_table IS declared
                    // AND the multiplication shortfall is real. Mirrors
                    // the existing `SessionArmingQuotaVsPeerTableInvariantViolated`
                    // discipline at lines 3084-3124 above (same
                    // multiplication structure, different multiplicands).
                    if reassembly_cfg.per_peer_quota > 0 {
                        if let Some(peer_table) = link
                            .stateless_accept
                            .as_ref()
                            .and_then(|sa| sa.peer_table.as_ref())
                        {
                            let pool_slot_count = pool.slot_count;
                            let product =
                                peer_table.capacity as u64 * reassembly_cfg.per_peer_quota as u64;
                            if product < pool_slot_count as u64 {
                                return Err(Box::new(
                                    ValidationError::ReassemblyPerPeerQuotaBuildInvariantViolated {
                                        pool_name: pool_name.to_string(),
                                        slot_count: pool_slot_count,
                                        machine: machine_name.clone(),
                                        link_name: link_name.clone(),
                                        peer_table_capacity: peer_table.capacity,
                                        per_peer_quota: reassembly_cfg.per_peer_quota,
                                        product,
                                    },
                                ));
                            }
                        }
                    }
                }

                // ── pool/stage-copy-accept-rejected-under-forbid ──
                // Spec line 2512-2516 — under `forbid` policy, the
                // mere presence of `<sce:accept-stage-copy-rate>` on
                // a link source is a hard error regardless of whether
                // the rate gate would fire. Checked before #3 + #6
                // because spec contract is "the opt-out itself is
                // rejected outright"; deterministic-first-failure
                // walk order surfaces this code before the rate-gate
                // codes when both would apply.
                let policy = machine.resolved_stage_copy_policy();
                if matches!(policy, StageCopyPolicy::Forbid)
                    && forge_link.is_some_and(|l| l.accept_stage_copy_rate)
                {
                    return Err(Box::new(
                        ValidationError::PoolStageCopyAcceptRejectedUnderForbid {
                            machine: machine_name.clone(),
                            link_name: link_name.clone(),
                        },
                    ));
                }

                // ── #3 reassembly/expected-fragmentation-rate-high ──
                //     (or its policy promotion `pool/stage-copy-policy-error`)
                //
                // Spec line 2950-2952 verbatim — the formula references
                // "the regular RX pool's slot_size", which is the
                // `BufferPoolVariant::Default` pool bound to the link.
                // Silent-skip when no Default pool
                // is bound — that means the link has no "regular RX"
                // path the formula can reference. When the resolved
                // pool IS the Default variant, use its slot_size.
                //
                // Policy promotion semantics (RFC §synth-5-K lines 2358-2367):
                //   - `Warn` (default): #3 fires unless the link
                //     declares `<sce:accept-stage-copy-rate>` (opt-out
                //     suppresses the warning per spec line 2356-2357).
                //   - `Error`: warning promoted to
                //     `pool/stage-copy-policy-error`; opt-out still
                //     suppresses (spec line 2358-2361).
                //   - `Forbid`: same promotion as `Error`; opt-out
                //     itself was already rejected above, so reaching
                //     this point with `Forbid` implies no opt-out
                //     and the promoted hard error fires.
                if let Some(expected_p99_bytes) = link.expected_p99_bytes {
                    if let BufferPoolVariant::Default = variant {
                        if expected_p99_bytes > slot_size {
                            // rate_percent = (p99 - slot_size) / p99 × 100
                            // Integer math: (p99 - slot_size) × 100 / p99
                            let excess = expected_p99_bytes - slot_size;
                            let rate_percent =
                                (excess as u64 * 100 / expected_p99_bytes as u64) as u32;
                            if rate_percent > 25 {
                                let opt_out = forge_link.is_some_and(|l| l.accept_stage_copy_rate);
                                // Opt-out suppresses the diagnostic
                                // under `warn` and `error` per spec
                                // line 2356-2361. Under `forbid` the
                                // opt-out was rejected above so this
                                // branch is unreachable with opt-out
                                // = true on `forbid`.
                                if !opt_out {
                                    return Err(Box::new(match policy {
                                        StageCopyPolicy::Warn => {
                                            ValidationError::ReassemblyExpectedFragmentationRateHigh {
                                                pool_name: pool_name.to_string(),
                                                slot_size,
                                                expected_p99_bytes,
                                                rate_percent,
                                                machine: machine_name.clone(),
                                                link_name: link_name.clone(),
                                            }
                                        }
                                        StageCopyPolicy::Error
                                        | StageCopyPolicy::Forbid => {
                                            ValidationError::PoolStageCopyPolicyError {
                                                pool_name: pool_name.to_string(),
                                                slot_size,
                                                expected_p99_bytes,
                                                rate_percent,
                                                machine: machine_name.clone(),
                                                link_name: link_name.clone(),
                                                policy: policy.as_str().to_string(),
                                            }
                                        }
                                    }));
                                }
                            }
                        }
                    }
                }

                // ── #6 reassembly/stage-copy-wcet-exceeds-slot-budget ──
                // Spec line 2995-2999 verbatim. Four inputs required:
                // expected_p99_bytes (link), memcpy_cycles_per_byte +
                // clock_freq_mhz (platform), worker_slot_budget_us
                // (scheduler). Silent-skip on any absence — the
                // formula has no meaningful interpretation otherwise.
                if let (
                    Some(expected_p99_bytes),
                    Some(memcpy_cycles_per_byte),
                    Some(clock_freq_mhz),
                    Some(worker_slot_budget_us),
                ) = (
                    link.expected_p99_bytes,
                    memcpy_cycles_per_byte,
                    clock_freq_mhz,
                    worker_slot_budget_us,
                ) {
                    // stage_copy_wcet_us = bytes × cycles_per_byte /
                    //                      clock_freq_mhz
                    // Cycles ÷ MHz = microseconds (1 MHz = 1 cycle/us).
                    // Use f64 for the intermediate product to avoid
                    // overflow on large p99; round half-up to u32.
                    let stage_copy_wcet_us = ((expected_p99_bytes as f64
                        * memcpy_cycles_per_byte as f64)
                        / clock_freq_mhz as f64)
                        .ceil()
                        .min(u32::MAX as f64) as u32;
                    if stage_copy_wcet_us > worker_slot_budget_us {
                        return Err(Box::new(
                            ValidationError::ReassemblyStageCopyWcetExceedsSlotBudget {
                                machine: machine_name.clone(),
                                link_name: link_name.clone(),
                                expected_p99_bytes,
                                memcpy_cycles_per_byte,
                                clock_freq_mhz,
                                worker_slot_budget_us,
                                stage_copy_wcet_us,
                            },
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

/// SCE Protocol-Synthesis RFC §synth-5-K line 2466-2469 — stateless_accept extern
/// allowlist (C13 deferred-2). For each link with a `stateless_accept`
/// block, the `hmac_extern` + `rng_extern` symbol names must be
/// present in the §synth-5-I baseline intrinsics whitelist
/// ([`crate::forge::intrinsic_registry::BASELINE_SYMBOLS`]) OR in the
/// passed `plugin_symbols` slice (target_plugin-loaded entries per the
/// §synth-5-I plugin loader). When the symbol is in neither, the validator returns
/// [`DeployError::StatelessAcceptExternNotWhitelisted`] carrying the
/// sorted union of baseline + plugin names as `Fix::ReplaceOneOf`
/// candidates.
///
/// Lives at the orchestrator level because target-plugin loading is
/// deploy-driven, mirroring the target-plugin loader precedent — the baseline
/// whitelist is a compile-time const, but the plugin set varies per
/// deploy.
pub fn validate_stateless_accept_externs(
    cfg: &DeployConfig,
    plugin_symbols: &[crate::forge::target_plugin::PluginSymbol],
) -> Result<(), DeployError> {
    use crate::forge::intrinsic_registry::{lookup_symbol, BASELINE_SYMBOLS};

    let plugin_has = |name: &str| plugin_symbols.iter().any(|s| s.name == name);
    let resolved = |name: &str| lookup_symbol(name).is_some() || plugin_has(name);

    // Closed-set candidates = sorted union of baseline + plugin names.
    // Built lazily on first miss so the happy path stays allocation-
    // free. The union is sorted because the wire payload's
    // `Fix::ReplaceOneOf` ride determinism matters for byte-stable
    // golden tests (FixCarriesCandidates non_overlap_class).
    let build_candidates = || -> Vec<String> {
        let mut out: Vec<String> = BASELINE_SYMBOLS
            .iter()
            .map(|s| s.name.to_string())
            .collect();
        for ps in plugin_symbols {
            out.push(ps.name.clone());
        }
        out.sort();
        out.dedup();
        out
    };

    for device in cfg.topology.values() {
        for (machine_name, machine) in device.machines.iter() {
            for (link_name, link) in machine.links.iter() {
                let Some(sa) = link.stateless_accept.as_ref() else {
                    continue;
                };
                for (role, extern_name) in [("hmac", &sa.hmac_extern), ("rng", &sa.rng_extern)] {
                    if !resolved(extern_name) {
                        return Err(DeployError::StatelessAcceptExternNotWhitelisted {
                            machine: machine_name.clone(),
                            link_name: link_name.clone(),
                            extern_name: extern_name.clone(),
                            role: role.to_string(),
                            candidates: build_candidates(),
                        });
                    }
                }
            }
        }
    }
    Ok(())
}

/// SCE Mesh §mesh-14 (SCE Protocol-Synthesis RFC §synth-5-K) — when a machine declares a
/// `platform:` block, the `class` axis (`mcu` / `ap`) and the `os`
/// axis must be mutually admissible per [`PlatformClass::admits_os`].
/// `class: mcu` admits only `bare_metal` / `rtos`; `class: ap` admits
/// only the general-purpose OS values (`linux`, `qnx`, `macos`,
/// `freebsd`, `windows`).
///
/// Enforced at parse time so a contradictory pairing (e.g. `class: mcu` +
/// `os: linux`) cannot reach the codegen-matrix walker (RFC §synth-5-J-4 /
/// §synth-5-J-5) that consumes `class` to gate MCU-only kinds.
fn validate_platform_class_os_consistency(cfg: &DeployConfig) -> Result<(), DeployError> {
    for device in cfg.topology.values() {
        for (machine_name, machine) in device.machines.iter() {
            let Some(platform) = machine.platform.as_ref() else {
                continue;
            };
            if !platform.class.admits_os(platform.os) {
                return Err(DeployError::PlatformClassOsMismatch {
                    machine: machine_name.clone(),
                    class: platform.class.as_str(),
                    os: platform.os.as_str(),
                });
            }
        }
    }
    Ok(())
}

/// SCE Mesh §mesh-14 (SCE Protocol-Synthesis RFC §synth-5-K, line 2160-2164) — when a
/// machine's scheduler runs in cooperative mode, `worker_stack_budget`
/// is REQUIRED. The cooperative worker drives `<send>` queue draining
/// inside a fixed stack frame; without an authored bound the codegen
/// has no static budget to check TLV-decode recursion against, and a
/// malformed TLV-chain could silently overflow at runtime.
///
/// Rejected at parse time so a `kind: cooperative` block cannot reach
/// the §synth-5-J-1 cooperative tick template emitter without a budget.
fn validate_scheduler_cooperative_stack_budget(cfg: &DeployConfig) -> Result<(), DeployError> {
    for device in cfg.topology.values() {
        for (machine_name, machine) in device.machines.iter() {
            let Some(sched) = machine.scheduler.as_ref() else {
                continue;
            };
            if matches!(sched.kind, SchedulerKind::Cooperative)
                && sched.worker_stack_budget.is_none()
            {
                return Err(DeployError::SchedulerCooperativeMissingStackBudget {
                    machine: machine_name.clone(),
                });
            }
        }
    }
    Ok(())
}

/// SCE Protocol-Synthesis RFC §synth-5-K line 2428-2429 (`deploy/worker-slot-budget-missing`)
/// — when a machine's scheduler runs in cooperative mode,
/// `worker_slot_budget_us` is REQUIRED. The per-slot WCET ceiling feeds
/// the §synth-5-B aggregate WCET check and the cooperative slot-count
/// derivation; without it the build cannot bound TLV/algorithm worst-
/// case execution time per tick, and the slot-count vs worker-count
/// invariant cannot be enforced.
fn validate_worker_slot_budget_required_when_cooperative(
    cfg: &DeployConfig,
) -> Result<(), DeployError> {
    for device in cfg.topology.values() {
        for (machine_name, machine) in device.machines.iter() {
            let Some(sched) = machine.scheduler.as_ref() else {
                continue;
            };
            if matches!(sched.kind, SchedulerKind::Cooperative)
                && sched.worker_slot_budget_us.is_none()
            {
                return Err(DeployError::SchedulerCooperativeMissingSlotBudget {
                    machine: machine_name.clone(),
                });
            }
        }
    }
    Ok(())
}

/// SCE Protocol-Synthesis RFC §synth-5-K line 2430-2431
/// (`deploy/keepalive-jitter-budget-missing`) — when a machine's
/// scheduler runs in cooperative mode, `keepalive_jitter_budget_us` is
/// REQUIRED. The sum of worst-case slot budgets in one tick window must
/// fit inside this bound; without an authored ceiling, the §synth-5-B
/// aggregate WCET consumer cannot enforce keepalive emission jitter
/// limits and zenoh peers may drop liveliness tokens under scheduler
/// stress.
fn validate_keepalive_jitter_required_when_cooperative(
    cfg: &DeployConfig,
) -> Result<(), DeployError> {
    for device in cfg.topology.values() {
        for (machine_name, machine) in device.machines.iter() {
            let Some(sched) = machine.scheduler.as_ref() else {
                continue;
            };
            if matches!(sched.kind, SchedulerKind::Cooperative)
                && sched.keepalive_jitter_budget_us.is_none()
            {
                return Err(
                    DeployError::SchedulerCooperativeMissingKeepaliveJitterBudget {
                        machine: machine_name.clone(),
                    },
                );
            }
        }
    }
    Ok(())
}

/// SCE Protocol-Synthesis RFC §synth-5-K line 2423
/// (`deploy/scheduler-incompatible-with-worker-count`) — when a machine
/// declares more workers than the cooperative scheduler can host in one
/// tick window, raise the deploy-side anchor for the over-subscription.
///
/// The derived slot count is `floor(tick_period_us / worker_slot_budget_us)`.
/// Validator silent-skips when:
/// - `scheduler.kind` is not `cooperative` (tokio/rt use preemption, no
///   slot accounting),
/// - `tick_period_us` is absent (no derivation possible —
///   absent-input silent-skip on missing deploy info),
/// - `worker_slot_budget_us` is absent (already caught by
///   [`validate_worker_slot_budget_required_when_cooperative`]).
///
/// The forge-side anchor for the same axis is `worker/scheduler-unsupported`
/// (spec §synth-5-D line 912), raised during [`crate::compile_forge_with_deploy`]
/// when a Worker doc compiles against a machine without an entry for
/// itself in `machines.<m>.workers` (signals: undeclared worker, scheduler
/// cannot account for it).
/// SCE Protocol-Synthesis RFC §synth-5-N lines 3060-3061 — paired
/// validators for the multi-link concurrency contract on the
/// cooperative-scheduler path.
///
/// **#1 `link/concurrent-count-exceeds-scheduler-slots`** (MCU-only
/// per spec line 3060 prose, gated on `platform.class: mcu`). The
/// per-tick slot ceiling is
/// `floor(tick_period_us / per_link_budget_us)`,
/// mirroring [`validate_machine_scheduler_worker_capacity`]. Fires
/// when `links.len() > slot_count`.
///
/// **#2 `link/per-link-budget-exceeds-tick-period`** (all classes —
/// the sanity check is "a single link's budget can't exceed one
/// tick" regardless of platform). Fires when
/// `per_link_budget_us > tick_period_us`.
///
/// Both silent-skip when any of the following are absent:
///   - `scheduler.kind` != `cooperative` (tokio/rt use preemption;
///     spec §synth-5-N AP path uses `tokio::spawn` per link — no slot
///     accounting),
///   - `tick_period_us` absent,
///   - `per_link_budget_us` absent (single-doc
///     compile paths + machines that opt out of the budget cap
///     silent-skip).
///
/// Returns the first failure (deterministic order: devices →
/// machines → per-link budget check first → slot count second).
fn validate_machine_scheduler_link_concurrency(cfg: &DeployConfig) -> Result<(), DeployError> {
    for device in cfg.topology.values() {
        for (machine_name, machine) in device.machines.iter() {
            let Some(sched) = machine.scheduler.as_ref() else {
                continue;
            };
            if !matches!(sched.kind, SchedulerKind::Cooperative) {
                continue;
            }
            let (Some(tick_period_us), Some(per_link_budget_us)) =
                (sched.tick_period_us, sched.per_link_budget_us)
            else {
                continue;
            };

            // ── #2 `link/per-link-budget-exceeds-tick-period` ──
            // Spec line 3061 verbatim. All cooperative-scheduled
            // platforms, regardless of class. Single-link sanity
            // check (literal code-name reading).
            if per_link_budget_us > tick_period_us {
                return Err(DeployError::LinkPerLinkBudgetExceedsTickPeriod {
                    machine: machine_name.clone(),
                    per_link_budget_us,
                    tick_period_us,
                });
            }

            // ── #1 `link/concurrent-count-exceeds-scheduler-slots` ──
            // Spec line 3060 verbatim, MCU-only. AP cooperative
            // schedulers (rare) use `tokio::spawn` per link per spec
            // line 3046-3049 — no slot accounting. Silent-skip when
            // platform.class != mcu OR platform entirely absent.
            let is_mcu = machine
                .platform
                .as_ref()
                .is_some_and(|p| matches!(p.class, PlatformClass::Mcu));
            if !is_mcu {
                continue;
            }
            if per_link_budget_us == 0 {
                continue;
            }
            let slot_count = tick_period_us / per_link_budget_us;
            let link_count = machine.links.len() as u32;
            if link_count > slot_count {
                return Err(DeployError::LinkConcurrentCountExceedsSchedulerSlots {
                    machine: machine_name.clone(),
                    link_count,
                    slot_count,
                    tick_period_us,
                    per_link_budget_us,
                });
            }
        }
    }
    Ok(())
}

fn validate_machine_scheduler_worker_capacity(cfg: &DeployConfig) -> Result<(), DeployError> {
    for device in cfg.topology.values() {
        for (machine_name, machine) in device.machines.iter() {
            let Some(sched) = machine.scheduler.as_ref() else {
                continue;
            };
            if !matches!(sched.kind, SchedulerKind::Cooperative) {
                continue;
            }
            let (Some(tick_period_us), Some(worker_slot_budget_us)) =
                (sched.tick_period_us, sched.worker_slot_budget_us)
            else {
                continue;
            };
            if worker_slot_budget_us == 0 {
                continue;
            }
            let slot_count = tick_period_us / worker_slot_budget_us;
            let worker_count = machine.workers.len() as u32;
            if worker_count > slot_count {
                return Err(DeployError::SchedulerIncompatibleWithWorkerCount {
                    machine: machine_name.clone(),
                    worker_count,
                    slot_count,
                    tick_period_us,
                    worker_slot_budget_us,
                });
            }
        }
    }
    Ok(())
}

/// SCE Protocol-Synthesis RFC §synth-5-D line 910 (`timer/slot-overflow`) — when a
/// machine declares more `Timer` docs under `machines.<m>.timers` than
/// `scheduler.timer_wheel_depth` static wheel slots can accommodate,
/// the build cannot fit the timer set into the wheel at compile time.
///
/// Validator silent-skips when:
/// - `machine.scheduler` is absent (no scheduler declared → no wheel),
/// - `scheduler.timer_wheel_depth` is absent (absent-input
///   silent-skip — the deploy doesn't carry wheel sizing),
/// - `machine.timers` is empty (no timers to overflow).
fn validate_machine_timer_wheel_capacity(cfg: &DeployConfig) -> Result<(), DeployError> {
    for device in cfg.topology.values() {
        for (machine_name, machine) in device.machines.iter() {
            let Some(sched) = machine.scheduler.as_ref() else {
                continue;
            };
            let Some(wheel_depth) = sched.timer_wheel_depth else {
                continue;
            };
            let timer_count = machine.timers.len() as u32;
            if timer_count > wheel_depth {
                return Err(DeployError::TimerSlotOverflow {
                    machine: machine_name.clone(),
                    timer_count,
                    wheel_depth,
                });
            }
        }
    }
    Ok(())
}

/// SCE Mesh §mesh-14 rule 5 — author-declared machine ids must not use the
/// reserved `__sce_synth_invoke__` infix. Synthesized children from
/// `<invoke type="scxml">` inline `<content>` (§mesh-9.6.6) are named
/// `<parent>__sce_synth_invoke__<id>`; a collision would silently
/// shadow or be shadowed by the synthesized peer at runtime, and the
/// partition coverage rules could not tell the two apart.
///
/// **Explicit override carve-out** (§mesh-9.6.6 rule 3): when the author
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
                continue; // no infix, no collision concern
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
/// inline `<content>` (SCE_MESH.md §mesh-14 rule 5 + §mesh-9.6.6). Single source
/// of truth so the parser, the validator, and any future synthesizer
/// agree on the reserved string.
pub const SYNTH_INVOKE_INFIX: &str = "__sce_synth_invoke__";

/// SCE_MESH.md §mesh-14 rules 7-10 — structural checks on `partitions:`
/// that do not require SCXML cross-reference. Rule 6 (duplicate
/// partition names) is enforced at deserialization time via the
/// custom [`PartitionMap`] visitor; rules 1, 2, 5, 11 (coverage,
/// default-partition discipline, synthesized-invoke infix collision,
/// nested-parallel partitioning) require SCXML inspection and land
/// in a later phase. This validator is a no-op when `partitions:` is
/// absent.
/// SCE_MESH §mesh-14 arch-debt #4 closure helper — partition names are
/// emitted as a C++ namespace segment (`P_<partition>`) by codegen, so
/// they must be valid C++ identifiers. Mirrors the C/C++ identifier
/// rule: first character is a letter (ASCII) or underscore, subsequent
/// characters are letters, digits, or underscores. Unicode identifiers
/// (`u8R"(..."` syntax) are intentionally not accepted — the underlying
/// codegen also emits machine names verbatim, so partition naming
/// should follow the same conservative subset.
fn is_cpp_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

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

    // SCE_MESH.md §mesh-14.4 × §mesh-14 — SOME/IP server pool machines cannot be
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
        // SCE_MESH §mesh-14 arch-debt #4 closure — codegen bakes the
        // partition name into `SCE::Generated::<machine>::P_<partition>`.
        // A name that is not a legal C++ identifier (starts with a
        // digit, contains hyphens / dots / spaces / etc.) would emit
        // non-compiling generated code. Detected at deploy parse so
        // the failure surfaces before codegen rather than at gcc.
        if !is_cpp_identifier(partition_name) {
            return Err(DeployError::PartitionNameNotIdentifier {
                partition: partition_name.clone(),
            });
        }

        // Rule 10 — empty partition. Checked first so it pre-empts the
        // rule-9 check (which would read no entries and pass vacuously).
        if decl.contains.parallel_regions.is_empty() && decl.contains.invokes.is_empty() {
            return Err(DeployError::PartitionEmpty {
                partition: partition_name.clone(),
            });
        }

        // §mesh-14 L2729-2730 — `transport_binding:` must name a transport
        // whose primary purpose is same-machine IPC. Unknown names and
        // known-but-incapable transports both fall here, with `reason`
        // telling the two shapes apart so the diagnostic is self-
        // explaining without the reader needing to cross-reference the
        // registry. Absent ⇒ skip (§mesh-14 L2730 defaults apply at codegen
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

        // §mesh-14 L2731-2732 — `barrier_timeout_ms:` is Option<u32>; `None`
        // / absent selects the W3C normative default of infinity. A
        // finite value of `0` would fire the §mesh-16.5 barrier before any
        // region can report `ParallelRegionDone`, unconditionally
        // raising `error.communication / PARALLEL_BARRIER_TIMEOUT` on
        // every `<parallel>` activation — the knob exists to bound
        // authentic hangs, not to convert barriers into errors. Authors
        // wanting "do not wait" must omit the key and rely on standard
        // SCXML transitions. Root-hosting-only semantics (spec
        // L2733-2735 "applies only to partitions hosting the root of a
        // `<parallel>`") is SCXML cross-reference scope (§mesh-16.5 runtime)
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

        // §mesh-14.4 × §mesh-14 pool-in-partition guard — reject before rule 7
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

/// Reject any `discovery:` top-level block (SCE Mesh §mesh-3.3 invariant,
/// §mesh-13 rejected list). Parsed as opaque [`serde_yaml_ng::Value`] so an authored
/// `discovery:` key lands here rather than triggering the generic
/// `deny_unknown_fields` message; the validator produces a spec-linked
/// diagnostic that names the replacement mechanisms (§mesh-14.4 binding
/// value-field placeholders for per-binding runtime target selection,
/// external OEM config for transport-level peer discovery). `null` /
/// absent discovery values deserialise as `None` and pass through.
fn validate_discovery_not_supported(cfg: &DeployConfig) -> Result<(), DeployError> {
    // §mesh-3.3: transport-native routing is the source of truth for peer
    // availability, so there is no SCE-side discovery config to interpret.
    // Absence is the only supported state; anything authored is an error.
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
/// machines whose transport-set + partition shape would compile a
/// handler that never fires. Runs at parse time so the diagnostic
/// surfaces the offending deploy.yaml line rather than a deferred
/// runtime misbehaviour.
///
/// The transport-compat check closes the silent-broken window where a
/// `liveliness:`-declared machine carried the `error.communication`
/// handler required by
/// [`sce-build/src/generator.rs::reject_liveliness_without_handler`]
/// but the codegen template emitted zero observer code, leaving the
/// handler unreachable (`feedback_silently_broken_hooks`).
///
/// **Acceptance shapes (post-RFC F.X-4):**
/// - **Zenoh transport (any partition shape).** `sce/live/<machine>` and
///   per-partition `sce/live/<machine>/<partition>` tokens are emitted
///   on every Zenoh binding; row 8 + row 13 always reachable.
/// - **SomeIP transport (any partition shape).** Under F.X-4 D4-shape-1
///   every SOMEIP `liveliness:` opt-in implicitly enables row 8
///   machine-level emission via the F.X-4 sub-range `[0x8280, 0x82FF]`
///   (vsomeip `register_availability_handler` on the machine-level
///   service). Row 13 region-liveness additionally fires when the
///   machine appears across ≥2 sibling partitions (RFC F.X-3); single-
///   partition machines are valid because row 8 alone keeps the silent-
///   broken window closed.
///
/// **Rejection shapes:**
/// - Lease value invalid (any transport).
/// - Neither Zenoh nor SomeIP transport — no observer code emitted at
///   all.
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
        let has_zenoh = machine_uses_zenoh_transport(machine);
        let has_someip = machine_uses_someip_transport(machine);
        if has_zenoh {
            // Zenoh path covers row 8 + row 13 unconditionally.
            continue;
        }
        if has_someip {
            // RFC F.X-4 D4-shape-1: every SOMEIP `liveliness:` opt-in
            // is implicitly accepted because machine-level emission
            // (row 8) is automatic. Partition count only affects whether
            // row 13 (region-liveness) ALSO fires; a single-partition
            // machine still has row 8 coverage and the
            // reject_liveliness_without_handler gate ensures the SCXML
            // carries an `error.communication` transition. The
            // silent-broken window is closed.
            continue;
        }
        return Err(DeployError::InvalidLiveliness {
            machine: machine_name.to_string(),
            reason: "machine has neither Zenoh nor SomeIP transport; `liveliness:` \
                     requires at least one `transport: zenoh` (any partition shape) \
                     or `transport: someip` (any partition shape, RFC F.X-4 covers \
                     row 8 unconditionally) — add a compatible binding/server or \
                     drop `liveliness:`"
                .to_string(),
        });
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

/// Returns true when at least one of the machine's bindings or its
/// server declaration selects the SomeIP transport. Sibling to
/// [`machine_uses_zenoh_transport`]; used by [`validate_liveliness`]
/// for the §mesh-16.4 SomeIP region-liveness acceptance branch (RFC F.X-3).
fn machine_uses_someip_transport(machine: &MachineConfig) -> bool {
    machine.bindings.values().any(|b| b.transport == "someip")
        || machine
            .server
            .as_ref()
            .is_some_and(|s| s.transport == "someip")
}

// ── SCE Mesh §mesh-14.4 binding pool support ─────────────────────
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
            if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
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
fn binding_placeholder_names(binding: &BindingConfig) -> Result<Vec<String>, String> {
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

/// SCE_MESH.md §mesh-14.4 — server-side multi-instance pool gating.
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
            .is_some_and(|d| d.supports_multi_instance_server);
        if !supported {
            return Err(DeployError::ServerPoolNotSupported {
                machine: machine_name.to_string(),
                transport: server.transport.clone(),
            });
        }
    }
    Ok(())
}

/// SCE_MESH.md §mesh-14.4 — a binding requesting a runtime pool may only
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
    let total_declarations: usize = cfg.topology.values().map(|d| d.machines.len()).sum();
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
                    reason: "SOME/IP bindings express runtime instance selection via \
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

/// SCE_MESH.md §mesh-14.6 — validate every binding's `reply_from:`
/// responder set.
///
/// A correlation entry is a one-shot resource: whoever is allowed to
/// match it retires it, and the request can then never be answered by
/// anyone else. The responder set is therefore the security boundary of
/// the RPC reply path, and it is validated here rather than at codegen
/// so an unresolvable or unrealisable set surfaces against the
/// deploy.yaml line that declared it.
///
/// Three rejections, all at parse time:
///
/// * empty list — an empty responder set would make every reply
///   uncorrelatable, which is never the author's intent; omitting the
///   field is how you ask for the same-target default.
/// * a member that is not a `#<machine>` target, or names a machine the
///   topology does not declare — the gate would silently never match.
/// * a set wider than the binding's own target on a transport whose
///   [`crate::mesh::transport::TransportDescriptor::supports_cross_target_reply`]
///   is `false` — the declared set could never be exercised.
///
/// Declaring only the binding's own target is always legal on every
/// transport: that is the default set written out explicitly.
fn validate_reply_from(cfg: &DeployConfig) -> Result<(), DeployError> {
    // §mesh-14.6: the responder set is the security boundary of the RPC
    // reply path, so its three rejections are enforced here at parse
    // time rather than at codegen.
    use std::collections::{BTreeMap, BTreeSet};

    // Every machine name across all devices — a responder may live on a
    // different ECU than the requester.
    let mut known_machines: BTreeSet<&str> = BTreeSet::new();
    for device in cfg.topology.values() {
        for name in device.machines.keys() {
            known_machines.insert(name.as_str());
        }
    }

    // Deterministic sorted scan so the first reported violation is
    // stable across runs even though `topology` is a HashMap.
    let mut by_binding: BTreeMap<(&str, &TargetId), &BindingConfig> = BTreeMap::new();
    for device in cfg.topology.values() {
        for (machine_name, machine) in &device.machines {
            for (target, binding) in &machine.bindings {
                by_binding.insert((machine_name.as_str(), target), binding);
            }
        }
    }

    for ((machine_name, target), binding) in by_binding {
        let Some(responders) = &binding.reply_from else {
            continue;
        };
        let invalid = |reason: String| DeployError::InvalidReplyFrom {
            machine: machine_name.to_string(),
            binding: target.as_str().to_string(),
            reason,
        };

        if responders.is_empty() {
            return Err(invalid(
                "the list is empty; omit `reply_from:` to keep the same-target default".to_string(),
            ));
        }

        let mut widens = false;
        for raw in responders {
            if !raw.starts_with('#') {
                return Err(invalid(format!(
                    "member '{raw}' is not a target identifier — write '#{raw}' to name a machine"
                )));
            }
            let Some(member) = TargetId::new(raw.clone()) else {
                return Err(invalid("a member is empty".to_string()));
            };
            if !known_machines.contains(member.name()) {
                return Err(invalid(format!(
                    "names machine '{}', which the topology does not declare",
                    member.name()
                )));
            }
            if member.as_str() != target.as_str() {
                widens = true;
            }
        }

        if widens {
            let supported = crate::mesh::transport::lookup(&binding.transport)
                .is_some_and(|d| d.supports_cross_target_reply);
            if !supported {
                return Err(DeployError::CrossTargetReplyNotSupported {
                    machine: machine_name.to_string(),
                    binding: target.as_str().to_string(),
                    transport: binding.transport.clone(),
                });
            }
        }
    }
    Ok(())
}

/// Transports whose RX path supports buffer-pool kind staging
/// (SCE Protocol-Synthesis RFC §synth-5-E). A binding may
/// declare `stage_pool: <name>` only on a transport in this list; any
/// other transport raises `mesh/deploy-stage-pool-transport-mismatch`.
///
/// Currently empty. The mesh RPC transports SCE knows today (zenoh,
/// someip, dds, shm, local, custom_tcp) route logical events rather
/// than allocating from a slot table — none of them actually surface
/// `Sample::take()` semantics. Entries are consumer-gated on
/// concrete transport-side wiring landing.
/// The empty list keeps the diagnostic strict — every
/// `stage_pool` declaration today fails loud, matching the
/// `feedback_silently_broken_hooks.md` invariant. See SCE Protocol-Synthesis
/// RFC §synth-5-E.
const TRANSPORTS_SUPPORTING_STAGE_POOL: &[&str] = &[];

/// Validate that every `binding.stage_pool` declaration sits on a
/// transport whose RX path supports buffer-pool kind staging
/// (SCE Protocol-Synthesis RFC §synth-5-E). Runs as part of
/// [`parse_deploy_str`] so the diagnostic fires at parse time —
/// independent of forge-side cross-reference resolution, which lives
/// in [`validate_stage_pool_references`] (a separate post-parse pass
/// gated on the build's [`ForgePoolRegistry`] being available).
fn validate_stage_pool_transport(cfg: &DeployConfig) -> Result<(), DeployError> {
    use std::collections::BTreeMap;
    let mut by_machine: BTreeMap<&str, &MachineConfig> = BTreeMap::new();
    for device in cfg.topology.values() {
        for (machine_name, machine) in &device.machines {
            by_machine.insert(machine_name.as_str(), machine);
        }
    }
    for (machine_name, machine) in by_machine {
        let mut sorted_bindings: Vec<(&TargetId, &BindingConfig)> =
            machine.bindings.iter().collect();
        sorted_bindings.sort_by_key(|(k, _)| k.as_str());
        for (binding_key, binding) in sorted_bindings {
            let Some(stage_pool) = binding.stage_pool.as_deref() else {
                continue;
            };
            if !TRANSPORTS_SUPPORTING_STAGE_POOL.contains(&binding.transport.as_str()) {
                return Err(DeployError::StagePoolTransportMismatch {
                    machine: machine_name.to_string(),
                    binding: binding_key.as_str().to_string(),
                    stage_pool: stage_pool.to_string(),
                    transport: binding.transport.clone(),
                });
            }
        }
    }
    Ok(())
}

/// Validate that every `binding.stage_pool` reference resolves to a
/// declared forge buffer-pool kind name (SCE Protocol-Synthesis RFC §synth-5-E
/// cross-schema reference resolution). Runs
/// as a post-parse pass — `parse_deploy_str` produces the
/// `DeployConfig` first, the build pipeline assembles the
/// [`crate::forge::pool_registry::ForgePoolRegistry`] from every
/// parsed `.forge` file, then this validator checks each declared
/// reference against the registry.
///
/// Two diagnostics emerge from this pass:
///
/// * `mesh/deploy-stage-pool-not-declared` — name not present in any
///   `.forge` file in the build. The candidate list rides
///   `Fix::ReplaceOneOf` over the registry's declared buffer-pool
///   names (sorted) so authors can pick a legal pool or extend the
///   build with the missing forge file.
/// * `mesh/deploy-stage-pool-wrong-kind` — name resolves to a forge
///   artifact whose kind is not `buffer-pool` (today the only kind
///   that backs the `Sample::take()` slot contract).
///
/// Transport-mismatch is **not** checked here — `parse_deploy_str`
/// already rejected those bindings via [`validate_stage_pool_transport`];
/// any binding that reaches this validator has already cleared the
/// transport gate.
pub fn validate_stage_pool_references(
    cfg: &DeployConfig,
    pool_registry: &crate::forge::pool_registry::ForgePoolRegistry,
) -> Result<(), DeployError> {
    validate_stage_pool_references_with(cfg, pool_registry, TRANSPORTS_SUPPORTING_STAGE_POOL)
}

/// Inner form of [`validate_stage_pool_references`] that takes the
/// supported-transports list as a parameter. The public API plumbs
/// the production `TRANSPORTS_SUPPORTING_STAGE_POOL` const; tests
/// pass a synthetic list to exercise the cross-ref arms while the
/// production list is still empty (every entry is forward-looking
/// infrastructure consumed by future atomics, see
/// `TRANSPORTS_SUPPORTING_STAGE_POOL` doc-comment).
fn validate_stage_pool_references_with(
    cfg: &DeployConfig,
    pool_registry: &crate::forge::pool_registry::ForgePoolRegistry,
    supported_transports: &[&str],
) -> Result<(), DeployError> {
    use crate::forge::pool_registry::ForgePoolKind;
    use std::collections::BTreeMap;
    let mut by_machine: BTreeMap<&str, &MachineConfig> = BTreeMap::new();
    for device in cfg.topology.values() {
        for (machine_name, machine) in &device.machines {
            by_machine.insert(machine_name.as_str(), machine);
        }
    }
    for (machine_name, machine) in by_machine {
        let mut sorted_bindings: Vec<(&TargetId, &BindingConfig)> =
            machine.bindings.iter().collect();
        sorted_bindings.sort_by_key(|(k, _)| k.as_str());
        for (binding_key, binding) in sorted_bindings {
            let Some(stage_pool) = binding.stage_pool.as_deref() else {
                continue;
            };
            // Skip bindings whose transport doesn't support stage
            // pool — `validate_stage_pool_transport` is the canonical
            // gate; reaching this validator implies that gate already
            // passed. Defensively skip rather than re-raise so the
            // two passes can run in either order without producing a
            // duplicate diagnostic.
            if !supported_transports.contains(&binding.transport.as_str()) {
                continue;
            }
            match pool_registry.lookup(stage_pool) {
                Some(ForgePoolKind::BufferPool) => {} // canonical case
                None => {
                    let candidates = pool_registry.names_of_kind(ForgePoolKind::BufferPool);
                    return Err(DeployError::StagePoolNotDeclared {
                        machine: machine_name.to_string(),
                        binding: binding_key.as_str().to_string(),
                        stage_pool: stage_pool.to_string(),
                        candidates,
                    });
                }
            }
            // future: when the registry classifies more pool kinds,
            // a `Some(other_kind)` arm raises `StagePoolWrongKind`.
            // The diagnostic is already wired through DeployError +
            // DiagnosticPayload + golden — this match is the only
            // place to grow when that day comes.
        }
    }
    Ok(())
}

/// Walk every binding that declared an explicit `retry:` section and
/// reject malformed values (SCE Mesh §mesh-16.7 row 3). Runs at parse time
/// so the diagnostic surfaces the offending deploy.yaml line rather
/// than generating a router whose retry layer behaves identically to
/// the opt-out path or whose timing values are arithmetically degenerate
/// (zero backoff, sub-unit multiplier, jitter >100%, etc.).
///
/// Mirrors the [`validate_outbound_buffer`] pattern below for symmetry:
/// determinism over machine name → target id order keeps diagnostics
/// stable across runs even when the underlying HashMap iteration is not.
fn validate_retry_policy(cfg: &DeployConfig) -> Result<(), DeployError> {
    use std::collections::BTreeMap;
    let mut by_path: BTreeMap<(String, String), &RetryPolicyConfig> = BTreeMap::new();
    for device in cfg.topology.values() {
        for (machine_name, machine) in &device.machines {
            for (target_id, binding) in &machine.bindings {
                if let Some(r) = &binding.retry {
                    by_path.insert((machine_name.to_string(), target_id.to_string()), r);
                }
            }
        }
    }
    for ((machine, target), policy) in by_path {
        if let Some(reason) = policy.validation_error() {
            return Err(DeployError::InvalidRetryPolicy {
                machine,
                target,
                reason,
            });
        }
    }
    Ok(())
}

/// Walk every binding that declared an explicit `auth:` section and
/// reject malformed values + per-transport unsupported placements
/// (SCE Mesh §mesh-16.7 row 10). Runs at parse time so the diagnostic
/// surfaces the offending deploy.yaml line rather than generating a
/// router whose row-10 wiring would silently no-op (custom_tcp / shm
/// have no observable auth signal) or whose pinned fingerprint cannot
/// be byte-for-byte canonicalized at runtime.
///
/// Mirrors [`validate_retry_policy`] for stable diagnostic ordering
/// across runs: BTreeMap keyed on (machine, target) drains in
/// canonical order regardless of HashMap iteration noise.
fn validate_auth_policy(cfg: &DeployConfig) -> Result<(), DeployError> {
    use std::collections::BTreeMap;
    let mut by_path: BTreeMap<(String, String), (&str, &AuthPolicyConfig)> = BTreeMap::new();
    for device in cfg.topology.values() {
        for (machine_name, machine) in &device.machines {
            for (target_id, binding) in &machine.bindings {
                if let Some(a) = &binding.auth {
                    by_path.insert(
                        (machine_name.to_string(), target_id.to_string()),
                        (binding.transport.as_str(), a),
                    );
                }
            }
        }
    }
    for ((machine, target), (transport, policy)) in by_path {
        if let Some(reason) = policy.validation_error(transport) {
            return Err(DeployError::InvalidAuthPolicy {
                machine,
                target,
                reason,
            });
        }
    }
    Ok(())
}

/// Walk every machine that declared an explicit `outbound_buffer:`
/// section and reject capacity-zero values (SCE Mesh §mesh-10.10). Runs at
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

/// SCE Mesh RFC F.X-1 — hybrid (counter + optional author-pin) §mesh-9.6 SOMEIP
/// scxml-invoke service ID assignment. Returns the assignment map for use
/// by codegen; errors are converted to [`DeployError`] variants on the way
/// out so the validator and the codegen call site share one source of
/// truth.
///
/// **Participant projection.** A machine is a participant iff it (a)
/// declares `bindings["#X"].transport: someip` for a declared peer `X`
/// (excluding internal targets and dangling references) or (b) is named as
/// the peer `X` in such a binding from another machine. Mirrors the legacy
/// [`validate_someip_scxml_invoke_service_id_collisions`] projection so the
/// two validators see the same set during the F.X-1 → F.X-3 coexistence
/// window. Same conservative single-domain assumption — multi-OEM
/// `vsomeip.json` `network:` boundaries are a separate landing.
///
/// **Rejection shapes** (each maps to a typed [`DeployError`] variant):
/// 1. **Overflow** — participant count > 128 (the invoke sub-range ceiling
///    under subsystem range partitioning).
/// 2. **Pin out-of-range** — author-pinned `someip_service_id:` falls
///    outside the §mesh-9.6 invoke sub-range `[0x8100, 0x817F]` (the upper half
///    of the SCE-reserved range is reserved for §mesh-16.4 region-liveness).
/// 3. **Pin-vs-pin collision** — two or more machines pin the same value.
///
/// Pin-vs-auto collision is impossible by construction:
/// [`crate::mesh::transport::someip::assign_invoke_service_ids`]'s counter
/// skips slots already claimed by pins.
pub(crate) fn assign_someip_invoke_service_ids(
    cfg: &DeployConfig,
) -> Result<std::collections::BTreeMap<String, u16>, DeployError> {
    use crate::mesh::transport::someip::{assign_invoke_service_ids, AssignInvokeServiceIdError};

    let declared_machines: std::collections::HashSet<&str> = cfg
        .topology
        .values()
        .flat_map(|d| d.machines.keys().map(|k| k.as_str()))
        .collect();

    let mut participants: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
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
    // §mesh-9.6 service. Surfacing a "pin on non-participant" rejection would
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

    match assign_invoke_service_ids(&participants_with_pins) {
        Ok(map) => Ok(map),
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

/// Parse-time validator wrapper around [`assign_someip_invoke_service_ids`].
/// Discards the assignment map; codegen calls the assigner directly to
/// obtain the same map without re-running the participant projection.
fn validate_someip_scxml_invoke_service_ids(cfg: &DeployConfig) -> Result<(), DeployError> {
    assign_someip_invoke_service_ids(cfg).map(|_| ())
}

/// Walk the deploy participant set for §mesh-16.4 SomeIP region-partition
/// liveness, project each (machine, partition) pair into the canonical
/// participant key `<machine>__P__<partition>`, gather per-partition
/// pins from `someip_liveness_service_id:`, and run the F.X-3 hybrid
/// allocator
/// ([`crate::mesh::transport::someip::assign_liveness_service_ids`]).
/// Returns the deterministic
/// `BTreeMap<participant_key, vsomeip::service_t>` codegen consumes for
/// the §mesh-16.4 SomeIP path.
///
/// **Participant projection.** Every partition that hosts a machine
/// which (a) declares `liveliness:` AND (b) uses SomeIP transport AND
/// (c) appears across ≥2 sibling partitions is a liveness participant.
/// The `validate_liveliness` gate already enforces (b) AND (c) when (a)
/// holds, so this function is a participant collector — the
/// well-formedness of the input is upstream's concern.
///
/// **Rejection shapes** (each maps to a typed [`DeployError`] variant):
/// 1. **Overflow** — participant count > 128 (the liveness sub-range
///    ceiling under subsystem range partitioning, RFC F.X-3 D1).
/// 2. **Pin out-of-range** — author-pinned `someip_liveness_service_id:`
///    falls outside the §mesh-16.4 liveness sub-range `[0x8180, 0x81FF]`.
/// 3. **Pin-vs-pin collision** — two or more partitions pin the same
///    value.
///
/// Pin-vs-auto collision is impossible by construction (assigner counter
/// skips reserved slots).
pub(crate) fn assign_someip_liveness_service_ids(
    cfg: &DeployConfig,
) -> Result<std::collections::BTreeMap<String, u16>, DeployError> {
    use crate::mesh::transport::someip::{
        assign_liveness_service_ids, AssignLivenessServiceIdError,
    };

    // 1. Identify SomeIP-transport machines that opt into `liveliness:`
    //    AND appear across ≥2 sibling partitions. Region-liveness (row 13)
    //    is partition-axis-keyed; a machine in a single partition has no
    //    sibling for `register_availability_handler` to observe. This
    //    self-contained projection check no longer relies on
    //    `validate_liveliness` to gate single-partition out — RFC F.X-4
    //    D4 loosened that gate to accept single-partition SOMEIP
    //    (row 8 covers it), so the F.X-3 assigner explicitly narrows to
    //    its own row-13 scope here.
    let mut someip_liveness_machines: std::collections::BTreeSet<&str> =
        std::collections::BTreeSet::new();
    for device in cfg.topology.values() {
        for (machine_name, machine_cfg) in &device.machines {
            if machine_cfg.liveliness.is_none() {
                continue;
            }
            if !machine_uses_someip_transport(machine_cfg) {
                continue;
            }
            // A machine using both Zenoh and SomeIP routes liveness over
            // Zenoh; the F.X-3 SomeIP path activates only when SomeIP
            // is the sole transport carrying the liveness signal.
            if machine_uses_zenoh_transport(machine_cfg) {
                continue;
            }
            // ≥2 partition gate (row-13-specific; row 8 has no such
            // requirement under RFC F.X-4).
            let sibling_partition_count = cfg.partitions.as_ref().map_or(0, |m| {
                m.iter()
                    .filter(|(_, p)| p.machines.iter().any(|n| n == machine_name))
                    .count()
            });
            if sibling_partition_count < 2 {
                continue;
            }
            someip_liveness_machines.insert(machine_name.as_str());
        }
    }

    // 2. For each (machine, partition) pair, build the participant key
    //    and collect the optional pin. BTreeMap insertion order = lex
    //    order so the assigner's deterministic output is preserved.
    let mut participants_with_pins: std::collections::BTreeMap<String, Option<u16>> =
        std::collections::BTreeMap::new();
    if let Some(partitions) = cfg.partitions.as_ref() {
        for (partition_name, partition_decl) in partitions.iter() {
            for machine_name in &partition_decl.machines {
                if !someip_liveness_machines.contains(machine_name.as_str()) {
                    continue;
                }
                let key = format!("{machine_name}__P__{partition_name}");
                participants_with_pins.insert(key, partition_decl.someip_liveness_service_id);
            }
        }
    }

    match assign_liveness_service_ids(&participants_with_pins) {
        Ok(map) => Ok(map),
        Err(AssignLivenessServiceIdError::Overflow {
            participant_count,
            ceiling,
        }) => Err(DeployError::SomeipLivenessServiceIdOverflow {
            participant_count,
            ceiling,
        }),
        Err(AssignLivenessServiceIdError::PinOutOfRange {
            partition_key,
            pinned_id,
            range_lo,
            range_hi,
        }) => Err(DeployError::SomeipLivenessServiceIdPinOutOfRange {
            partition_key,
            pinned_id,
            range_lo,
            range_hi,
        }),
        Err(AssignLivenessServiceIdError::PinCollision {
            partition_keys,
            pinned_id,
        }) => Err(DeployError::SomeipLivenessServiceIdPinCollision {
            partition_keys,
            pinned_id,
        }),
    }
}

/// Parse-time validator wrapper around [`assign_someip_liveness_service_ids`].
/// Discards the assignment map; codegen calls the assigner directly to
/// obtain the same map without re-running the participant projection.
fn validate_someip_liveness_service_ids(cfg: &DeployConfig) -> Result<(), DeployError> {
    assign_someip_liveness_service_ids(cfg).map(|_| ())
}

/// Walk the deploy participant set for §mesh-16.7 row 8 SOME/IP machine-level
/// liveness, collect each SOME/IP `liveliness:`-opt-in machine as a
/// participant keyed on `<machine>`, gather per-machine pins from
/// `someip_machine_liveness_service_id:`, and run the F.X-4 hybrid
/// allocator
/// ([`crate::mesh::transport::someip::assign_machine_liveness_service_ids`]).
/// Returns the deterministic
/// `BTreeMap<machine_name, vsomeip::service_t>` codegen consumes for the
/// §mesh-16.7 SOMEIP machine-level liveness path.
///
/// **Participant projection (RFC F.X-4 D4-shape-1).** Every machine that
/// (a) declares `liveliness:` AND (b) uses SOME/IP transport AND (c) does
/// NOT use Zenoh transport (Zenoh covers row 8 unconditionally via the
/// 2-segment `sce/live/<machine>` token) is a participant. Critically,
/// **partition count is irrelevant for machine-level liveness** —
/// single-partition SOME/IP machines that opt into `liveliness:` are
/// valid F.X-4 participants and must reach codegen. This is the explicit
/// difference from F.X-3's region-liveness participant projection (which
/// requires ≥2 sibling partitions).
///
/// **Rejection shapes** (each maps to a typed [`DeployError`] variant):
/// 1. **Overflow** — participant count > 128 (the F.X-4 sub-range
///    ceiling, RFC F.X-4 D1).
/// 2. **Pin out-of-range** — author-pinned
///    `someip_machine_liveness_service_id:` falls outside the §mesh-16.7
///    SOME/IP machine-liveness sub-range `[0x8280, 0x82FF]`.
/// 3. **Pin-vs-pin collision** — two or more machines pin the same value.
///
/// Pin-vs-auto collision is impossible by construction (assigner counter
/// skips reserved slots).
pub(crate) fn assign_someip_machine_liveness_service_ids(
    cfg: &DeployConfig,
) -> Result<std::collections::BTreeMap<String, u16>, DeployError> {
    use crate::mesh::transport::someip::{
        assign_machine_liveness_service_ids, AssignMachineLivenessServiceIdError,
    };

    // BTreeMap insertion preserves lex order so the assigner's
    // deterministic output is stable across deploy.yaml edits that
    // don't change the participant set.
    let mut participants_with_pins: std::collections::BTreeMap<String, Option<u16>> =
        std::collections::BTreeMap::new();
    for device in cfg.topology.values() {
        for (machine_name, machine_cfg) in &device.machines {
            if machine_cfg.liveliness.is_none() {
                continue;
            }
            if !machine_uses_someip_transport(machine_cfg) {
                continue;
            }
            // A machine using both Zenoh and SomeIP routes liveness over
            // Zenoh; the F.X-4 SomeIP path activates only when SomeIP
            // is the sole transport carrying the liveness signal.
            if machine_uses_zenoh_transport(machine_cfg) {
                continue;
            }
            participants_with_pins.insert(
                machine_name.clone(),
                machine_cfg.someip_machine_liveness_service_id,
            );
        }
    }

    match assign_machine_liveness_service_ids(&participants_with_pins) {
        Ok(map) => Ok(map),
        Err(AssignMachineLivenessServiceIdError::Overflow {
            participant_count,
            ceiling,
        }) => Err(DeployError::SomeipMachineLivenessServiceIdOverflow {
            participant_count,
            ceiling,
        }),
        Err(AssignMachineLivenessServiceIdError::PinOutOfRange {
            machine,
            pinned_id,
            range_lo,
            range_hi,
        }) => Err(DeployError::SomeipMachineLivenessServiceIdPinOutOfRange {
            machine,
            pinned_id,
            range_lo,
            range_hi,
        }),
        Err(AssignMachineLivenessServiceIdError::PinCollision {
            machines,
            pinned_id,
        }) => Err(DeployError::SomeipMachineLivenessServiceIdPinCollision {
            machines,
            pinned_id,
        }),
    }
}

/// Parse-time validator wrapper around
/// [`assign_someip_machine_liveness_service_ids`]. Discards the assignment
/// map; codegen calls the assigner directly to obtain the same map without
/// re-running the participant projection.
fn validate_someip_machine_liveness_service_ids(cfg: &DeployConfig) -> Result<(), DeployError> {
    assign_someip_machine_liveness_service_ids(cfg).map(|_| ())
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
                if cfg.source == source_suffix || cfg.source.ends_with(&format!("/{source_suffix}"))
                {
                    return Some(name.clone());
                }
            }
        }
        None
    }
}

// ── Cross-machine EventSchema validation ───────────────────────
//
// For every `<send target="#X">` whose sender and receiver are
// distinct machines on the mesh topology, the validator compares
// the EventSchema each side imports for the send's `event` name and
// rejects three divergence shapes (per
// [`crate::mesh::error::EventSchemaMismatchReason`]):
//
//   * StructuralHashMismatch — both sides declare a schema but the
//     canonical field shapes differ.
//   * SenderOnly — sender declares a schema, receiver does not.
//   * ReceiverOnly — receiver declares a schema, sender does not.
//
// Both-schemaless events fall through silently per the schemaless
// fallback (the same contract the receive-side and send-side
// validators honour). W3C-reserved targets (`#_parent` / `#_child`
// / `#_internal` / `#_scxml_`) and intra-machine sends skip the
// cross-machine check by construction. Dynamic targets
// (`<send targetexpr="…">`) and dynamic event names
// (`<send eventexpr="…">`) are not statically resolvable and skip
// silently — the existing typed-expression pipeline catches deeper
// misuse at codegen time.

/// Per-statechart `<send>` / `<raise>` action gathered for the
/// cross-machine schema validator. Carries only the fields the
/// validator reads to keep the borrow surface small and the walker
/// independent of the full executable-content shape.
struct CrossMachineSendSite<'a> {
    /// SCXML event name (the `event="..."` attribute).
    event_name: &'a str,
    /// Raw `target` attribute value, including the leading `#`.
    target: &'a str,
}

/// Recursive walk over an action sequence (transition body,
/// onentry/onexit block, initial-transition / history-default
/// sequence, `<if>` / `<foreach>` body) collecting every
/// `<send target="#X">` site whose event + target are statically
/// resolvable. Dynamic shapes silently skip by contract.
fn collect_cross_machine_send_sites<'a>(
    actions: &'a [crate::model::Action],
    out: &mut Vec<CrossMachineSendSite<'a>>,
) {
    for action in actions {
        if action.action_type == "send" || action.action_type == "raise" {
            // Dynamic event name (`eventexpr`) or dynamic target
            // (`targetexpr`) sites are not resolvable here; the
            // typed-expression pipeline catches misuse at codegen.
            if !action.event.is_empty() && !action.target.is_empty() {
                out.push(CrossMachineSendSite {
                    event_name: &action.event,
                    target: &action.target,
                });
            }
        }
        // Composite-action bodies. The shapes are mutually exclusive
        // by `action_type` but the parallel `Vec<Action>` fields are
        // safe to walk unconditionally (empty vecs cost nothing).
        collect_cross_machine_send_sites(&action.then_actions, out);
        for branch in &action.elseif_branches {
            collect_cross_machine_send_sites(&branch.actions, out);
        }
        collect_cross_machine_send_sites(&action.else_actions, out);
        collect_cross_machine_send_sites(&action.actions, out);
    }
}

/// Canonical structural hash of an [`EventSchemaModel`]. Used by the
/// cross-machine validator to compare two schemas without imposing a
/// `PartialEq` derive cascade on `ForgeField` / `SceType` /
/// `Quantity` / `EnumRef` etc. (every transitive struct would need
/// `Eq` even though the comparison runs once per cross-machine send
/// during validation).
///
/// Canonicalisation:
///
///   1. Fields sorted by `id` so declaration-order shuffles do not
///      mask structural equivalence.
///   2. Source-location metadata stripped (a schema's wire shape is
///      the typed contract, not the file position the author wrote
///      it at).
///   3. serde_json's default ordering on the resulting fields is
///      stable for the typed payload (struct fields are emitted in
///      declaration order; serde does not reshuffle).
///
/// The 64-bit FNV-1a digest gives a short hex string suitable for
/// human-readable diagnostic messages without dragging in a
/// cryptographic hash dependency.
///
/// Changed from the prior `LookupRef` shape: an `Enum(EnumRef)`
/// field's hash incorporates the alias string verbatim (the same
/// text the author wrote in `sce:type="enum:<alias>"`), so the
/// structural hash is invariant under the Lookup→Enum reshape of
/// the underlying ref type. Two machines pointing at the same enum
/// document via the same alias hash identically; the hash is
/// deterministic on the alias text + variant surface, not on the
/// Rust struct variant carrying the alias.
fn canonical_event_schema_hash(schema: &crate::forge::model::EventSchemaModel) -> String {
    let mut sorted_fields = schema.fields.clone();
    sorted_fields.sort_by(|a, b| a.id.cmp(&b.id));
    // Stripped projection: event_name + sorted fields. `name` and
    // `source_location` are author/file metadata that vary
    // legitimately across machines for the same wire contract.
    let canonical = serde_json::json!({
        "event_name": &schema.event_name,
        "fields": &sorted_fields,
    });
    let canonical_str = serde_json::to_string(&canonical).unwrap_or_default();
    // FNV-1a 64-bit. Constants per
    // http://www.isthe.com/chongo/tech/comp/fnv/index.html#FNV-param.
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in canonical_str.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", hash)
}

/// Cross-machine
/// EventSchema consistency check. See module-level commentary above
/// for the full rejection-shape contract.
///
/// `scxml_models` is the orchestrator's pass-2 capture (each entry
/// is `(path, SCXMLModel)`). The deploy topology resolves each
/// machine's `source:` against the SCXML model by file basename so
/// the validator can build per-machine schema visibility from the
/// statechart's `<sce:import>` declarations.
///
/// `event_schemas_by_doc_name` is the build-wide EventSchema
/// registry keyed by file stem; the per-statechart resolver
/// (`crate::forge::event_schema_check::resolve_imported_event_schemas`)
/// projects each machine's in-scope schemas out of this registry.
///
/// Returns the first failing diagnostic. Today's traversal order is
/// machine-name lexicographic (BTreeMap iteration) and within a
/// machine, document-order over the SCXML state graph — stable
/// across builds so fixture asserts can pin the offending send
/// without flakiness.
pub fn validate_event_schemas_cross_machine(
    cfg: &DeployConfig,
    scxml_models: &[(std::path::PathBuf, crate::model::SCXMLModel)],
) -> Result<(), DeployError> {
    use std::collections::BTreeMap;
    use std::path::Path;

    // Step 1: machine name → &SCXMLModel. Match by file basename
    // (each machine's `source:` resolves to exactly one SCXMLModel
    // in pass-2's vector; collisions are unreachable because the
    // orchestrator already de-dupes scxml paths upstream).
    let mut machine_to_scxml: BTreeMap<&str, &crate::model::SCXMLModel> = BTreeMap::new();
    for device in cfg.topology.values() {
        for (machine_name, machine_cfg) in &device.machines {
            let source_basename = Path::new(&machine_cfg.source)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            if source_basename.is_empty() {
                continue;
            }
            if let Some((_, model)) = scxml_models.iter().find(|(path, _)| {
                path.file_name().and_then(|s| s.to_str()) == Some(source_basename)
            }) {
                machine_to_scxml.insert(machine_name.as_str(), model);
            }
        }
    }

    // Step 2: machine name → its resolved schema visibility, read from
    // the model rather than re-resolved here.
    //
    // `imported_event_schemas` is the map the parse seam resolved by
    // following each `<sce:import src="…">` to the document it names, and
    // it is what every backend lowers against. Re-deriving per-machine
    // visibility from a build-wide index keyed by file stem would answer
    // the same question a second, weaker way — two documents with the same
    // stem index identically — so a cross-machine mismatch could be
    // declared (or missed) against a schema neither machine actually
    // generates. Comparing what the machines really carry is the only
    // comparison worth making.
    let mut machine_to_schemas: BTreeMap<
        &str,
        &BTreeMap<String, crate::forge::model::EventSchemaModel>,
    > = BTreeMap::new();
    for (machine_name, scxml) in &machine_to_scxml {
        machine_to_schemas.insert(*machine_name, &scxml.imported_event_schemas);
    }

    // Step 3: walk each machine's SCXML for cross-machine
    // `<send target="#X">` sites and compare schemas.
    for (sender_machine, scxml) in &machine_to_scxml {
        let mut sites: Vec<CrossMachineSendSite> = Vec::new();
        for state in scxml.states.values() {
            for transition in &state.transitions {
                collect_cross_machine_send_sites(&transition.actions, &mut sites);
            }
            for block in &state.on_entry_blocks {
                collect_cross_machine_send_sites(block, &mut sites);
            }
            for block in &state.on_exit_blocks {
                collect_cross_machine_send_sites(block, &mut sites);
            }
            collect_cross_machine_send_sites(&state.initial_transition_actions, &mut sites);
            collect_cross_machine_send_sites(&state.initial_history_default_actions, &mut sites);
        }

        let sender_schemas = machine_to_schemas
            .get(sender_machine)
            .expect("inserted above");

        for site in &sites {
            // Strip the leading `#` per
            // [`crate::mesh::target::TargetId::name`]. Reject
            // W3C-reserved targets here without a TargetId roundtrip
            // — the literal-set match is short and the
            // failure-mode-of-interest here is cross-machine
            // routing, not the W3C internals.
            if !site.target.starts_with('#') {
                continue;
            }
            let target_name = &site.target[1..];
            if matches!(target_name, "_parent" | "_child" | "_internal" | "_scxml_") {
                continue;
            }
            if target_name.is_empty() {
                continue;
            }
            if target_name == *sender_machine {
                // Intra-machine self-send — no cross-machine
                // consistency question.
                continue;
            }
            let Some(receiver_schemas) = machine_to_schemas.get(target_name) else {
                // Target machine is unknown to this build's deploy
                // topology. Either the author named a non-mesh
                // target (e.g. `#http_gateway` resolved through a
                // binding to an external transport) or the deploy
                // declaration is incomplete. Either way, schema
                // validation has no peer to compare against; the
                // existing topology/transport validators are the
                // load-bearing rejection signal for unrouted
                // targets.
                continue;
            };
            let sender_schema = sender_schemas.get(site.event_name);
            let receiver_schema = receiver_schemas.get(site.event_name);
            let mismatch = match (sender_schema, receiver_schema) {
                (None, None) => None,
                (Some(_), None) => Some(crate::mesh::error::EventSchemaMismatchReason::SenderOnly),
                (None, Some(_)) => {
                    Some(crate::mesh::error::EventSchemaMismatchReason::ReceiverOnly)
                }
                (Some(s), Some(r)) => {
                    let s_hash = canonical_event_schema_hash(s);
                    let r_hash = canonical_event_schema_hash(r);
                    if s_hash == r_hash {
                        None
                    } else {
                        Some(
                            crate::mesh::error::EventSchemaMismatchReason::StructuralHashMismatch {
                                sender_hash: s_hash,
                                receiver_hash: r_hash,
                            },
                        )
                    }
                }
            };
            if let Some(reason) = mismatch {
                return Err(DeployError::EventSchemaMismatch {
                    event_name: site.event_name.to_string(),
                    sender_machine: sender_machine.to_string(),
                    receiver_machine: target_name.to_string(),
                    reason,
                });
            }
        }
    }
    Ok(())
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
        assert_eq!(
            zenoh.connect.as_deref(),
            Some(&["tcp/192.168.1.1:7447".to_string()][..])
        );
        assert_eq!(
            zenoh.listen.as_deref(),
            Some(&["tcp/0.0.0.0:7447".to_string()][..])
        );
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
        assert_eq!(
            brake.subscriptions[0].event,
            "event.notification.vehicle_speed"
        );
        assert_eq!(brake.subscriptions[0].source, "#chassis");
        assert_eq!(
            brake.subscriptions[1].event,
            "event.notification.brake_pressure"
        );
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

    // ── ordering (SCE_MESH.md §mesh-10.6) ──────────────────────────

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

    // ── ordering timings (SCE_MESH.md §mesh-10.6.1) ────────────────

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
    fn liveliness_someip_only_machine_accepted_under_fx4() {
        // RFC F.X-4 D4-shape-1: SomeIP-only machine + `liveliness:` is
        // accepted because every SOMEIP `liveliness:` opt-in implicitly
        // enables row 8 machine-level emission (vsomeip
        // `register_availability_handler` on the F.X-4 machine-level
        // service ID in [0x8280, 0x82FF]). The silent-broken window from
        // F.X-3's deferral state is closed; partition count is irrelevant
        // for row-8 coverage. The pre-F.X-4 rejection branch ("SomeIP +
        // liveliness needs Zenoh") is gone.
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
        let cfg = parse_deploy_str(yaml).expect("F.X-4 D4-shape-1 accepts SomeIP + liveliness");
        // Machine-level liveness allocator must include `brake` in the
        // F.X-4 sub-range [0x8280, 0x82FF] — the load-bearing positive
        // assertion that distinguishes "accepted" from "silently no-op".
        let machine_ids =
            assign_someip_machine_liveness_service_ids(&cfg).expect("assigner must succeed");
        let brake_id = *machine_ids
            .get("brake")
            .expect("brake must be a machine-liveness participant under F.X-4 D4-shape-1");
        assert!(
            (0x8280..=0x82FF).contains(&brake_id),
            "brake machine-liveness ID must land in F.X-4 sub-range: got {brake_id:#06x}"
        );
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
        let cfg =
            parse_deploy_str(yaml).expect("zenoh server must satisfy liveliness transport-compat");
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
                assert!(
                    reason.contains("100"),
                    "reason must cite the floor: {reason}"
                );
            }
            other => panic!("expected InvalidLiveliness, got {other:?}"),
        }
    }

    // ── §mesh-16.4 SomeIP region-liveness validator (RFC F.X-3) ─────────────

    #[test]
    fn liveliness_someip_with_two_partitions_accepted() {
        // RFC F.X-3 D6 acceptance shape: SomeIP transport + machine
        // appears across ≥2 sibling partitions + `liveliness:` opt-in.
        // The validator must accept; the F.X-3 codegen branch will
        // emit `register_availability_handler` for the sibling.
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
partitions:
  brake_left:
    machines: [brake]
    contains:
      parallel_regions:
        - { machine: brake, region: r_left }
  brake_right:
    machines: [brake]
    contains:
      parallel_regions:
        - { machine: brake, region: r_right }
"##;
        parse_deploy_str(yaml)
            .expect("SomeIP + ≥2 sibling partitions of same machine must satisfy F.X-3 D6");
    }

    #[test]
    fn liveliness_someip_with_single_partition_accepted_under_fx4() {
        // RFC F.X-4 D4-shape-1: SomeIP + single partition + liveliness
        // is now valid because row 8 machine-level emission covers the
        // single-process case. F.X-3's "≥2 sibling partitions required"
        // branch only applied to row 13 (region-liveness); row 8 has no
        // such requirement under F.X-4.
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
partitions:
  brake_only:
    machines: [brake]
    contains:
      parallel_regions:
        - { machine: brake, region: r_left }
"##;
        let cfg = parse_deploy_str(yaml).expect("F.X-4 D4-shape-1 accepts single-partition SOMEIP");
        // The F.X-4 machine-level allocator includes `brake` regardless
        // of partition count.
        let machine_ids =
            assign_someip_machine_liveness_service_ids(&cfg).expect("assigner must succeed");
        assert!(
            machine_ids.contains_key("brake"),
            "single-partition SOMEIP machine must still be a row-8 participant"
        );
        // Region-liveness allocator filters by partition count under
        // F.X-3's existing semantics: with only 1 partition, there is
        // no row-13 participant.
        let region_ids = assign_someip_liveness_service_ids(&cfg).expect("assigner must succeed");
        assert!(
            region_ids.is_empty(),
            "single-partition machine must not generate row-13 region-liveness participants: {region_ids:?}"
        );
    }

    #[test]
    fn liveliness_someip_partitions_assign_partition_keyed_service_ids() {
        // RFC F.X-3 D2: participant key shape `<machine>__P__<partition>`.
        // The assigner output must use that exact key form so codegen
        // and downstream consumers see consistent identifiers.
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
partitions:
  brake_left:
    machines: [brake]
    contains:
      parallel_regions:
        - { machine: brake, region: r_left }
  brake_right:
    machines: [brake]
    contains:
      parallel_regions:
        - { machine: brake, region: r_right }
"##;
        let cfg = parse_deploy_str(yaml).expect("setup must parse");
        let map = assign_someip_liveness_service_ids(&cfg)
            .expect("assigner must succeed for accepted shape");
        assert_eq!(map.len(), 2, "two partitions = two participants");
        assert!(map.contains_key("brake__P__brake_left"));
        assert!(map.contains_key("brake__P__brake_right"));
        // Auto-assigned IDs land at the liveness sub-range base in lex
        // order (brake_left < brake_right).
        assert_eq!(map["brake__P__brake_left"], 0x8180);
        assert_eq!(map["brake__P__brake_right"], 0x8181);
    }

    #[test]
    fn liveliness_someip_partition_pin_collision_surfaces_deploy_error() {
        // RFC F.X-3 D6: pin-vs-pin collision across partitions must
        // surface as a typed DeployError variant (not generic
        // InvalidLiveliness), so operators can distinguish allocator
        // failures from acceptance-shape rejections.
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
partitions:
  brake_left:
    machines: [brake]
    contains:
      parallel_regions:
        - { machine: brake, region: r_left }
    someip_liveness_service_id: "0x8185"
  brake_right:
    machines: [brake]
    contains:
      parallel_regions:
        - { machine: brake, region: r_right }
    someip_liveness_service_id: "0x8185"
"##;
        match parse_deploy_str(yaml) {
            Err(DeployError::SomeipLivenessServiceIdPinCollision {
                partition_keys,
                pinned_id,
            }) => {
                assert_eq!(pinned_id, 0x8185);
                assert!(partition_keys.contains(&"brake__P__brake_left".to_string()));
                assert!(partition_keys.contains(&"brake__P__brake_right".to_string()));
            }
            other => panic!("expected SomeipLivenessServiceIdPinCollision, got {other:?}"),
        }
    }

    #[test]
    fn liveliness_someip_partition_pin_out_of_range_surfaces_deploy_error() {
        // RFC F.X-3 D6: pin below F.X-3 sub-range (0x817F = highest
        // F.X-1 invoke slot) must surface a typed PinOutOfRange.
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
partitions:
  brake_left:
    machines: [brake]
    contains:
      parallel_regions:
        - { machine: brake, region: r_left }
    someip_liveness_service_id: "0x817F"
  brake_right:
    machines: [brake]
    contains:
      parallel_regions:
        - { machine: brake, region: r_right }
"##;
        match parse_deploy_str(yaml) {
            Err(DeployError::SomeipLivenessServiceIdPinOutOfRange {
                partition_key,
                pinned_id,
                range_lo,
                range_hi,
            }) => {
                assert_eq!(partition_key, "brake__P__brake_left");
                assert_eq!(pinned_id, 0x817F);
                assert_eq!(range_lo, 0x8180);
                assert_eq!(range_hi, 0x81FF);
            }
            other => panic!("expected SomeipLivenessServiceIdPinOutOfRange, got {other:?}"),
        }
    }

    #[test]
    fn liveliness_someip_no_partitions_accepted_under_fx4() {
        // RFC F.X-4 D4-shape-1: SomeIP + no `partitions:` block at all
        // is accepted because row 8 machine-level emission covers
        // non-partitioned machines. The F.X-3 "no partitions" rejection
        // branch is gone — the silent-broken window stays closed
        // because handler-gate (`reject_liveliness_without_handler`)
        // requires `error.communication`, and machine-level emission
        // ensures it fires.
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
        let cfg = parse_deploy_str(yaml).expect("F.X-4 D4-shape-1 accepts no-partitions SOMEIP");
        let machine_ids =
            assign_someip_machine_liveness_service_ids(&cfg).expect("assigner must succeed");
        assert!(
            machine_ids.contains_key("brake"),
            "no-partitions SOMEIP machine must be a row-8 participant"
        );
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
    fn retry_section_absent_is_none() {
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
"##;
        let cfg = parse_deploy_str(yaml).expect("parse");
        let machine = &cfg.topology["ecu1"].machines["brake"];
        let binding = machine
            .bindings
            .get(&TargetId::new("#motor").unwrap())
            .unwrap();
        assert!(
            binding.retry.is_none(),
            "absent retry section must deserialize as None (opt-in gate — §16.7 row 3)"
        );
    }

    #[test]
    fn retry_section_present_parses_with_defaults() {
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
            retry:
              max_retries: 3
"##;
        let cfg = parse_deploy_str(yaml).expect("parse");
        let machine = &cfg.topology["ecu1"].machines["brake"];
        let binding = machine
            .bindings
            .get(&TargetId::new("#motor").unwrap())
            .unwrap();
        let r = binding.retry.expect("retry must be Some");
        assert_eq!(r.max_retries, 3);
        // Defaults must populate from the per-field serde defaults.
        assert_eq!(r.initial_backoff_ms, 100);
        assert!((r.backoff_multiplier - 2.0).abs() < 1e-9);
        assert_eq!(r.max_backoff_ms, 5000);
        assert_eq!(r.backoff_jitter_pct, 10);
    }

    #[test]
    fn retry_section_full_override_parses() {
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        bindings:
          "#motor":
            transport: someip
            retry:
              max_retries: 5
              initial_backoff_ms: 50
              backoff_multiplier: 1.5
              max_backoff_ms: 2000
              backoff_jitter_pct: 25
"##;
        let cfg = parse_deploy_str(yaml).expect("parse");
        let machine = &cfg.topology["ecu1"].machines["brake"];
        let binding = machine
            .bindings
            .get(&TargetId::new("#motor").unwrap())
            .unwrap();
        let r = binding.retry.expect("retry must be Some");
        assert_eq!(r.max_retries, 5);
        assert_eq!(r.initial_backoff_ms, 50);
        assert!((r.backoff_multiplier - 1.5).abs() < 1e-9);
        assert_eq!(r.max_backoff_ms, 2000);
        assert_eq!(r.backoff_jitter_pct, 25);
    }

    #[test]
    fn retry_zero_max_retries_rejected() {
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
            retry:
              max_retries: 0
"##;
        match parse_deploy_str(yaml) {
            Err(DeployError::InvalidRetryPolicy {
                machine,
                target,
                reason,
            }) => {
                assert_eq!(machine, "brake");
                assert_eq!(target, "#motor");
                assert!(
                    reason.contains("max_retries"),
                    "reason must cite the rejected knob: {reason}",
                );
            }
            other => panic!("expected InvalidRetryPolicy, got {other:?}"),
        }
    }

    #[test]
    fn retry_sub_unit_multiplier_rejected() {
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
            retry:
              max_retries: 2
              backoff_multiplier: 0.5
"##;
        match parse_deploy_str(yaml) {
            Err(DeployError::InvalidRetryPolicy { reason, .. }) => {
                assert!(
                    reason.contains("backoff_multiplier"),
                    "reason must cite the rejected knob: {reason}",
                );
            }
            other => panic!("expected InvalidRetryPolicy, got {other:?}"),
        }
    }

    #[test]
    fn retry_jitter_over_100_rejected() {
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
            retry:
              max_retries: 2
              backoff_jitter_pct: 150
"##;
        match parse_deploy_str(yaml) {
            Err(DeployError::InvalidRetryPolicy { reason, .. }) => {
                assert!(
                    reason.contains("backoff_jitter_pct"),
                    "reason must cite the rejected knob: {reason}",
                );
            }
            other => panic!("expected InvalidRetryPolicy, got {other:?}"),
        }
    }

    #[test]
    fn retry_max_below_initial_rejected() {
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
            retry:
              max_retries: 2
              initial_backoff_ms: 500
              max_backoff_ms: 100
"##;
        match parse_deploy_str(yaml) {
            Err(DeployError::InvalidRetryPolicy { reason, .. }) => {
                assert!(
                    reason.contains("max_backoff_ms"),
                    "reason must cite the rejected knob: {reason}",
                );
            }
            other => panic!("expected InvalidRetryPolicy, got {other:?}"),
        }
    }

    // ── §mesh-16.7 row 10 auth-policy parser tests ─────────────────────

    #[test]
    fn auth_section_absent_is_none() {
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
"##;
        let cfg = parse_deploy_str(yaml).expect("parse");
        let machine = &cfg.topology["ecu1"].machines["brake"];
        let binding = machine
            .bindings
            .get(&TargetId::new("#motor").unwrap())
            .unwrap();
        assert!(
            binding.auth.is_none(),
            "absent auth section must deserialize as None (opt-in gate — §16.7 row 10)"
        );
    }

    #[test]
    fn auth_zenoh_with_valid_fingerprint_parses() {
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
            auth:
              required: true
              peer_fingerprint: "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
"##;
        let cfg = parse_deploy_str(yaml).expect("parse");
        let machine = &cfg.topology["ecu1"].machines["brake"];
        let binding = machine
            .bindings
            .get(&TargetId::new("#motor").unwrap())
            .unwrap();
        let a = binding.auth.as_ref().expect("auth must be Some");
        assert!(a.required);
        assert_eq!(
            a.peer_fingerprint.as_deref(),
            Some("sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"),
        );
        assert!(a.sd_denied_classifies_as_unauthorized.is_none());
    }

    #[test]
    fn auth_someip_with_sd_denied_flag_parses() {
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        bindings:
          "#motor":
            transport: someip
            auth:
              required: true
              sd_denied_classifies_as_unauthorized: true
"##;
        let cfg = parse_deploy_str(yaml).expect("parse");
        let machine = &cfg.topology["ecu1"].machines["brake"];
        let binding = machine
            .bindings
            .get(&TargetId::new("#motor").unwrap())
            .unwrap();
        let a = binding.auth.as_ref().expect("auth must be Some");
        assert!(a.required);
        assert_eq!(a.sd_denied_classifies_as_unauthorized, Some(true));
        assert!(a.peer_fingerprint.is_none());
    }

    #[test]
    fn auth_zenoh_required_without_fingerprint_rejected() {
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
            auth:
              required: true
"##;
        match parse_deploy_str(yaml) {
            Err(DeployError::InvalidAuthPolicy { reason, .. }) => {
                assert!(
                    reason.contains("peer_fingerprint"),
                    "reason must cite the missing knob: {reason}",
                );
            }
            other => panic!("expected InvalidAuthPolicy, got {other:?}"),
        }
    }

    #[test]
    fn auth_zenoh_malformed_fingerprint_rejected() {
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
            auth:
              required: true
              peer_fingerprint: "sha256:NOTHEX"
"##;
        match parse_deploy_str(yaml) {
            Err(DeployError::InvalidAuthPolicy { reason, .. }) => {
                assert!(
                    reason.contains("hex"),
                    "reason must cite the malformed hex: {reason}",
                );
            }
            other => panic!("expected InvalidAuthPolicy, got {other:?}"),
        }
    }

    #[test]
    fn auth_someip_required_without_flag_rejected() {
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        bindings:
          "#motor":
            transport: someip
            auth:
              required: true
"##;
        match parse_deploy_str(yaml) {
            Err(DeployError::InvalidAuthPolicy { reason, .. }) => {
                assert!(
                    reason.contains("sd_denied_classifies_as_unauthorized"),
                    "reason must cite the missing flag: {reason}",
                );
            }
            other => panic!("expected InvalidAuthPolicy, got {other:?}"),
        }
    }

    #[test]
    fn auth_custom_tcp_rejected() {
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
            auth:
              required: true
              peer_fingerprint: "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
"##;
        match parse_deploy_str(yaml) {
            Err(DeployError::InvalidAuthPolicy { reason, .. }) => {
                assert!(
                    reason.contains("custom_tcp") && reason.contains("row 10"),
                    "reason must cite the transport rejection: {reason}",
                );
            }
            other => panic!("expected InvalidAuthPolicy, got {other:?}"),
        }
    }

    #[test]
    fn auth_zenoh_fingerprint_with_required_false_rejected() {
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
            auth:
              required: false
              peer_fingerprint: "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
"##;
        match parse_deploy_str(yaml) {
            Err(DeployError::InvalidAuthPolicy { reason, .. }) => {
                assert!(
                    reason.contains("required: false"),
                    "reason must cite the ignored field placement: {reason}",
                );
            }
            other => panic!("expected InvalidAuthPolicy, got {other:?}"),
        }
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

    // ── SCE_MESH.md §mesh-14.6 responder set ────────────────────────
    //
    // Every arm below builds the same two-machine topology and varies
    // only the `reply_from:` value, so a failure names the rule that
    // broke rather than a fixture difference.
    fn reply_from_yaml(transport: &str, reply_from_clause: &str) -> String {
        format!(
            r##"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        bindings:
          "#alpha":
            transport: {transport}
            service: alpha_svc
{reply_from_clause}
      alpha:
        source: alpha.scxml
      broker:
        source: broker.scxml
"##
        )
    }

    #[test]
    fn reply_from_absent_keeps_the_same_target_default() {
        // The safe default must not require an opt-out.
        parse_deploy_str(&reply_from_yaml("someip", "")).expect("absent reply_from must parse");
    }

    #[test]
    fn reply_from_naming_only_its_own_target_is_accepted_everywhere() {
        // Writing the default out explicitly does not widen the set, so
        // it stays legal even on a transport that cannot cross targets.
        parse_deploy_str(&reply_from_yaml(
            "zenoh",
            "            reply_from: [\"#alpha\"]",
        ))
        .expect("self-only reply_from must parse on any transport");
    }

    #[test]
    fn reply_from_empty_list_rejected() {
        match parse_deploy_str(&reply_from_yaml("someip", "            reply_from: []")) {
            Err(DeployError::InvalidReplyFrom {
                machine,
                binding,
                reason,
            }) => {
                assert_eq!(machine, "brake");
                assert_eq!(binding, "#alpha");
                assert!(reason.contains("empty"), "reason: {reason}");
            }
            other => panic!("expected InvalidReplyFrom, got {other:?}"),
        }
    }

    #[test]
    fn reply_from_unknown_machine_rejected() {
        match parse_deploy_str(&reply_from_yaml(
            "someip",
            "            reply_from: [\"#alpha\", \"#ghost\"]",
        )) {
            Err(DeployError::InvalidReplyFrom { reason, .. }) => {
                assert!(
                    reason.contains("ghost"),
                    "reason must name the miss: {reason}"
                );
            }
            other => panic!("expected InvalidReplyFrom, got {other:?}"),
        }
    }

    #[test]
    fn reply_from_member_without_hash_rejected() {
        // `alpha` and `#alpha` must not both be accepted: the gate
        // compares against the binding key, which always carries `#`.
        match parse_deploy_str(&reply_from_yaml(
            "someip",
            "            reply_from: [\"alpha\"]",
        )) {
            Err(DeployError::InvalidReplyFrom { reason, .. }) => {
                assert!(
                    reason.contains("#alpha"),
                    "reason must show the fix: {reason}"
                );
            }
            other => panic!("expected InvalidReplyFrom, got {other:?}"),
        }
    }

    #[test]
    fn reply_from_wider_set_accepted_on_someip() {
        parse_deploy_str(&reply_from_yaml(
            "someip",
            "            reply_from: [\"#alpha\", \"#broker\"]",
        ))
        .expect("someip correlates through pending_rpcs_, so a wider set is realisable");
    }

    #[test]
    fn reply_from_wider_set_rejected_on_zenoh() {
        // `session.get` binds the reply closure to one target's KeyExpr,
        // so a wider set could never be exercised — reject at parse time
        // rather than generating a router that silently ignores it.
        match parse_deploy_str(&reply_from_yaml(
            "zenoh",
            "            reply_from: [\"#alpha\", \"#broker\"]",
        )) {
            Err(DeployError::CrossTargetReplyNotSupported {
                machine,
                binding,
                transport,
            }) => {
                assert_eq!(machine, "brake");
                assert_eq!(binding, "#alpha");
                assert_eq!(transport, "zenoh");
            }
            other => panic!("expected CrossTargetReplyNotSupported, got {other:?}"),
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
        // Authored §mesh-4.3 example-shaped block — rejected per §mesh-3.3 and
        // §mesh-13 rejection of `discovery.mode: static | dynamic`.
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
        // An empty `discovery: {}` map is still Some(Value::Mapping(_)); §mesh-3.3
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

    // ── SCE Mesh §mesh-14.4 binding pool ──────────────────────────

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
            Err(DeployError::PoolNotSupportedByTransport {
                machine,
                binding,
                transport,
            }) => {
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
            Err(DeployError::PoolInvalidPlaceholder {
                machine,
                binding,
                reason,
            }) => {
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
            Err(DeployError::PoolNotSupportedByTransport {
                machine,
                binding,
                transport,
            }) => {
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
        // SCE_MESH.md §mesh-14.4 (Gap 7): SOME/IP is the
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

    // SCE_MESH.md §mesh-14 rules 6-10 — partitions schema coverage. Each
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
            Err(DeployError::PartitionMachineNotListed { partition, machine }) => {
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
        // SCE_MESH.md §mesh-14.4 × §mesh-14 — a SOME/IP pool machine declares
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
        // partition must parse (§mesh-14.4 pool + absent partitioning is
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
            motor
                .server
                .as_ref()
                .and_then(|s| s.instances.as_ref())
                .is_some(),
            "motor must retain its pool declaration",
        );
    }

    #[test]
    fn non_pool_machine_in_partition_passes() {
        // Regression guard — a non-pool machine listed in a partition
        // must parse under the partition baseline rules alone; the Gap I
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
        // §mesh-14 L2729-2730 — `shm` is the canonical same-machine IPC
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
            part.get("brake_main").unwrap().transport_binding.as_deref(),
            Some("shm")
        );
    }

    #[test]
    fn accept_partition_transport_binding_custom_tcp() {
        // §mesh-14 L2730 "kind tcp/shm" — the `tcp` half is `custom_tcp`
        // (§mesh-16.8.3 reference transport).
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
            part.get("brake_main").unwrap().transport_binding.as_deref(),
            Some("custom_tcp")
        );
    }

    #[test]
    fn reject_partition_transport_binding_unknown() {
        // Unknown transport name — `iceoryx2` is not in the registry.
        // §mesh-14 L2729-2730 accepts only registry-known transports whose
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
        // the OS process boundary `partitions:` defines. §mesh-14 L2729
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
        // §mesh-14 L2729 intends.
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
        // same-machine IPC channel §mesh-14 L2729 intends.
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
        // §mesh-14 L2731-2732 — finite positive values are accepted; the
        // runtime consumer (§mesh-16.5) interprets them against the
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
        // per §mesh-14 L2732. Regression guard: the knob must stay
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
        // §mesh-16.5 barrier timeout: zero would fire before the first
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

    // ── §mesh-9.6.6 rule 3 explicit override carve-out ─────────────

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

    // ── §mesh-9.6 Session 4c legacy validator tests removed under RFC F.X-1.
    //    The counter scheme is collision-free by construction; the
    //    `someip_service_id_*` tests below cover the F.X-1 rejection
    //    shapes (overflow / pin-out-of-range / pin-vs-pin collision).

    /// The 4-machine fixture set used across `tests/mesh` (parent /
    /// worker / motor / brake) must accept under the F.X-1 hybrid
    /// allocator: lex-sorted counter assigns brake=0x8100, motor=0x8101,
    /// parent=0x8102, worker=0x8103 (all distinct, well under the
    /// 128-slot ceiling). Accept regardless of which machine declares
    /// the outbound binding to whom — the §mesh-9.6 participant union is
    /// `{parent, worker, motor, brake}` and the counter assignment is
    /// collision-free by construction.
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

    /// Single §mesh-9.6 someip participant (one machine references one peer)
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
        // a §mesh-9.6 invoke participant. The pin carries no meaning and the
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

    // ── §mesh-14 / RFC §synth-5-K item A2 — per-machine platform/scheduler/memory ──

    #[test]
    fn platform_mcu_with_baremetal_parses() {
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      mcu_node:
        source: mcu.scxml
        platform:
          class: mcu
          os: bare_metal
          has_dcache: true
          dcache_line_size: 32
          has_speculative_prefetch: false
          core_count: 1
"##;
        let cfg = parse_deploy_str(yaml).expect("mcu+bare_metal must parse");
        let machine = cfg
            .topology
            .get("ecu1")
            .and_then(|d| d.machines.get("mcu_node"))
            .expect("machine present");
        let platform = machine.platform.as_ref().expect("platform parsed");
        assert_eq!(platform.class, PlatformClass::Mcu);
        assert_eq!(platform.os, OsKind::BareMetal);
        assert_eq!(platform.has_dcache, Some(true));
        assert_eq!(platform.dcache_line_size, Some(32));
        assert_eq!(platform.has_speculative_prefetch, Some(false));
        assert_eq!(platform.core_count, Some(1));
    }

    #[test]
    fn platform_ap_with_linux_parses() {
        let yaml = r##"
version: "1.0"
topology:
  host:
    machines:
      ap_node:
        source: ap.scxml
        platform:
          class: ap
          os: linux
"##;
        let cfg = parse_deploy_str(yaml).expect("ap+linux must parse");
        let platform = cfg
            .topology
            .get("host")
            .and_then(|d| d.machines.get("ap_node"))
            .and_then(|m| m.platform.as_ref())
            .expect("platform parsed");
        assert_eq!(platform.class, PlatformClass::Ap);
        assert_eq!(platform.os, OsKind::Linux);
    }

    #[test]
    fn platform_class_os_mismatch_mcu_linux_rejected() {
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      bad:
        source: bad.scxml
        platform:
          class: mcu
          os: linux
"##;
        match parse_deploy_str(yaml) {
            Err(DeployError::PlatformClassOsMismatch { machine, class, os }) => {
                assert_eq!(machine, "bad");
                assert_eq!(class, "mcu");
                assert_eq!(os, "linux");
            }
            other => panic!("expected PlatformClassOsMismatch, got {other:?}"),
        }
    }

    #[test]
    fn platform_class_os_mismatch_ap_baremetal_rejected() {
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      bad:
        source: bad.scxml
        platform:
          class: ap
          os: bare_metal
"##;
        match parse_deploy_str(yaml) {
            Err(DeployError::PlatformClassOsMismatch { class, os, .. }) => {
                assert_eq!(class, "ap");
                assert_eq!(os, "bare_metal");
            }
            other => panic!("expected PlatformClassOsMismatch, got {other:?}"),
        }
    }

    #[test]
    fn scheduler_cooperative_with_budget_parses() {
        // The required-when-cooperative set: stack budget,
        // slot budget (line 2428-9), keepalive jitter budget
        // (line 2430-1). All three must be present for the cooperative
        // arm to parse without error.
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      worker:
        source: worker.scxml
        scheduler:
          kind: cooperative
          worker_stack_budget: 4096
          worker_slot_budget_us: 200
          keepalive_jitter_budget_us: 5000
"##;
        let cfg = parse_deploy_str(yaml).expect("cooperative+budget must parse");
        let sched = cfg
            .topology
            .get("ecu1")
            .and_then(|d| d.machines.get("worker"))
            .and_then(|m| m.scheduler.as_ref())
            .expect("scheduler parsed");
        assert_eq!(sched.kind, SchedulerKind::Cooperative);
        assert_eq!(sched.worker_stack_budget, Some(4096));
        assert_eq!(sched.worker_slot_budget_us, Some(200));
        assert_eq!(sched.keepalive_jitter_budget_us, Some(5000));
    }

    #[test]
    fn scheduler_cooperative_without_budget_rejected() {
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      worker:
        source: worker.scxml
        scheduler:
          kind: cooperative
"##;
        match parse_deploy_str(yaml) {
            Err(DeployError::SchedulerCooperativeMissingStackBudget { machine }) => {
                assert_eq!(machine, "worker");
            }
            other => panic!("expected SchedulerCooperativeMissingStackBudget, got {other:?}"),
        }
    }

    #[test]
    fn scheduler_tokio_without_budget_parses() {
        // Only `kind: cooperative` requires worker_stack_budget; tokio / rt
        // inherit host runtime stack defaults.
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      worker:
        source: worker.scxml
        scheduler:
          kind: tokio
"##;
        let cfg = parse_deploy_str(yaml).expect("tokio without budget must parse");
        let sched = cfg
            .topology
            .get("ecu1")
            .and_then(|d| d.machines.get("worker"))
            .and_then(|m| m.scheduler.as_ref())
            .expect("scheduler parsed");
        assert_eq!(sched.kind, SchedulerKind::Tokio);
        assert_eq!(sched.worker_stack_budget, None);
    }

    #[test]
    fn memory_sram_and_dma_parses() {
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      mcu_node:
        source: mcu.scxml
        memory:
          sram_regions:
            dtcm:
              base: 0x20000000
              size: 65536
              attr: [fast, nocache]
            sram1:
              base: 0x08000000
              size: 524288
              attr: [dma_coherent, cacheable]
          dma_channels: [DW0_CH0, DW0_CH1]
"##;
        let cfg = parse_deploy_str(yaml).expect("memory section must parse");
        let mem = cfg
            .topology
            .get("ecu1")
            .and_then(|d| d.machines.get("mcu_node"))
            .and_then(|m| m.memory.as_ref())
            .expect("memory parsed");
        assert_eq!(mem.sram_regions.len(), 2);
        let dtcm = mem.sram_regions.get("dtcm").expect("dtcm region");
        assert_eq!(dtcm.base, 0x20000000);
        assert_eq!(dtcm.size, 65536);
        assert_eq!(dtcm.attr, vec!["fast".to_string(), "nocache".to_string()]);
        assert_eq!(mem.dma_channels.len(), 2);
        assert_eq!(mem.dma_channels[0], "DW0_CH0");
    }

    #[test]
    fn machine_without_platform_section_parses_unclassified() {
        // The §mesh-14 sections are all optional; absence ⇒ no classification.
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      bare:
        source: bare.scxml
"##;
        let cfg = parse_deploy_str(yaml).expect("absent platform/scheduler/memory must parse");
        let machine = cfg
            .topology
            .get("ecu1")
            .and_then(|d| d.machines.get("bare"))
            .expect("machine present");
        assert!(machine.platform.is_none());
        assert!(machine.scheduler.is_none());
        assert!(machine.memory.is_none());
    }

    // ── SCE Protocol-Synthesis RFC §synth-5-E stage_pool field ────────────────────
    //
    // These cover the deploy.yaml side of the cross-schema reference
    // surface. The transport-mismatch path is exercised at parse time
    // through `parse_deploy_str`; the cross-ref paths
    // (`StagePoolNotDeclared`, `StagePoolWrongKind`) live in
    // [`validate_stage_pool_references`] which is invoked separately
    // by callers that have already assembled the
    // `ForgePoolRegistry`. Both surfaces are unit-tested here so the
    // diagnostic plumbing stays exercised even before the η' codegen
    // atomic begins consuming the field.

    #[test]
    fn stage_pool_field_default_absent() {
        // An ordinary mesh RPC binding without `stage_pool:` parses
        // unchanged; the field defaults to `None` and the diagnostic
        // plumbing stays inert.
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
        let brake = &cfg.topology["ecu1"].machines["brake"];
        let binding = &brake.bindings[&TargetId::new("#motor").unwrap()];
        assert!(binding.stage_pool.is_none());
    }

    #[test]
    fn stage_pool_on_unsupported_transport_rejected() {
        // Today no mesh RPC transport is in
        // `TRANSPORTS_SUPPORTING_STAGE_POOL`, so any present
        // `stage_pool:` raises `StagePoolTransportMismatch` — this is
        // the canonical fail-loud rejection.
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      mcu_node:
        source: mcu.scxml
        bindings:
          "#sub":
            transport: zenoh
            stage_pool: rx_pool_sram1
"##;
        match parse_deploy_str(yaml) {
            Err(DeployError::StagePoolTransportMismatch {
                machine,
                binding,
                stage_pool,
                transport,
            }) => {
                assert_eq!(machine, "mcu_node");
                assert_eq!(binding, "#sub");
                assert_eq!(stage_pool, "rx_pool_sram1");
                assert_eq!(transport, "zenoh");
            }
            other => panic!("expected StagePoolTransportMismatch, got {other:?}"),
        }
    }

    #[test]
    fn stage_pool_field_parses_when_unused() {
        // The `stage_pool:` rejection happens in
        // `validate_stage_pool_transport`; if no binding declares it,
        // the validator is a no-op even with the empty supported list.
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
"##;
        parse_deploy_str(yaml).expect("absent stage_pool ⇒ pass");
    }

    #[test]
    fn stage_pool_cross_ref_not_declared_emits_candidates() {
        // Cross-ref happy/sad: with a synthetic supported-transport
        // list (production const is currently empty until η' codegen
        // wires real transports) and an empty registry, the
        // unresolved name surfaces as
        // `StagePoolNotDeclared`. The candidate list rides
        // `Fix::ReplaceOneOf` and is sorted (registry contract).
        use crate::forge::pool_registry::{ForgePoolKind, ForgePoolRegistry};
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      mcu_node:
        source: mcu.scxml
        bindings:
          "#sub":
            transport: zenoh
            stage_pool: rx_pool_sram1
"##;
        let cfg: DeployConfig = serde_yaml_ng::from_str(yaml).expect("YAML parses");
        // Registry seeded with one alternate name so `Fix::ReplaceOneOf`
        // candidate list is non-trivial.
        let mut registry = ForgePoolRegistry::new();
        registry
            .record("scout_rx_pool", ForgePoolKind::BufferPool)
            .unwrap();
        match validate_stage_pool_references_with(&cfg, &registry, &["zenoh"]) {
            Err(DeployError::StagePoolNotDeclared {
                machine,
                binding,
                stage_pool,
                candidates,
            }) => {
                assert_eq!(machine, "mcu_node");
                assert_eq!(binding, "#sub");
                assert_eq!(stage_pool, "rx_pool_sram1");
                assert_eq!(candidates, vec!["scout_rx_pool".to_string()]);
            }
            other => panic!("expected StagePoolNotDeclared, got {other:?}"),
        }
    }

    #[test]
    fn stage_pool_cross_ref_resolves_when_registered() {
        // Happy path: registry has the referenced buffer-pool name,
        // validator returns Ok(()).
        use crate::forge::pool_registry::{ForgePoolKind, ForgePoolRegistry};
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      mcu_node:
        source: mcu.scxml
        bindings:
          "#sub":
            transport: zenoh
            stage_pool: rx_pool_sram1
"##;
        let cfg: DeployConfig = serde_yaml_ng::from_str(yaml).expect("YAML parses");
        let mut registry = ForgePoolRegistry::new();
        registry
            .record("rx_pool_sram1", ForgePoolKind::BufferPool)
            .unwrap();
        validate_stage_pool_references_with(&cfg, &registry, &["zenoh"])
            .expect("registered name resolves cleanly");
    }

    #[test]
    fn stage_pool_references_validator_skips_unsupported_transport() {
        // The cross-ref validator defensively skips bindings whose
        // transport is not in the supported list — those are already
        // rejected by `validate_stage_pool_transport`, and re-raising
        // here would produce a duplicate diagnostic. The skip is the
        // co-pass invariant: the two validators run in either order
        // without colliding.
        //
        // Construction path: deserialize the deploy YAML directly via
        // serde_yaml_ng (bypasses `parse_deploy_str` and its
        // transport-gate validator), then call the cross-ref
        // validator in isolation. This is the only way to reach the
        // skip arm today — every supported transport entry is
        // forward-looking until the η' codegen atomic populates the
        // list.
        use crate::forge::pool_registry::ForgePoolRegistry;
        let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      mcu_node:
        source: mcu.scxml
        bindings:
          "#sub":
            transport: zenoh
            stage_pool: rx_pool_sram1
"##;
        let cfg: DeployConfig =
            serde_yaml_ng::from_str(yaml).expect("YAML parses without invoking validators");
        let registry = ForgePoolRegistry::new();
        // Empty registry + unsupported transport ⇒ defensive skip,
        // not a duplicate diagnostic.
        validate_stage_pool_references(&cfg, &registry)
            .expect("co-pass invariant: cross-ref skips when transport gate already rejected");
    }
}
