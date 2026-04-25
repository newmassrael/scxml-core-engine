// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Structured error hierarchy for the SCE Mesh pipeline.
//
// Each variant maps to a pipeline stage:
//   Deploy         → stage 1 (deploy.yaml parsing)
//   External       → stage 1b (vsomeip.json / zenoh.json5 parsing + 3-way check)
//   Topology       → stage 2 (target resolution + validation)
//   Codegen        → stage 3 (template rendering)
//   Io             → cross-cutting filesystem errors

use std::path::PathBuf;

/// Top-level error for the mesh code-generation pipeline.
///
/// Variants correspond to pipeline stages so callers can react
/// programmatically (distinct CLI exit codes, IDE diagnostics, etc.)
/// without parsing error message strings.
#[derive(Debug, thiserror::Error)]
pub enum MeshError {
    #[error(transparent)]
    Deploy(#[from] DeployError),

    #[error(transparent)]
    External(#[from] ExternalConfigError),

    #[error(transparent)]
    Topology(#[from] TopologyError),

    #[error(transparent)]
    Codegen(#[from] CodegenError),

    #[error("I/O error on {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
}

// ── Stage 1: deploy.yaml parsing ─────────────────────────────

/// Why a `transport_binding:` value was rejected by
/// [`DeployError::PartitionTransportBindingUnsupported`]. Two shapes
/// share one diagnostic code (§14 L2729-2730); this enum keeps them
/// structurally distinguishable without forcing consumers to
/// substring-grep the prose.
///
/// The `Display` impl emits the exact phrases the §14 diagnostic
/// template splices via `{failure}`, so the generated JSON payload is
/// byte-identical to the pre-typed `reason: String` shape — the goal
/// of this refactor is drift-trap closure, not message change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartitionTransportBindingFailure {
    /// The transport name is not in the implemented registry. The
    /// known set is carried so the diagnostic can list valid names
    /// without the enum needing a back-reference to
    /// [`crate::mesh::transport`]; callers pass
    /// `implemented_names()` at construction time.
    Unknown { known_names: Vec<String> },
    /// The transport is registered but its
    /// [`crate::mesh::transport::TransportDescriptor::supports_inter_partition_ipc`]
    /// flag is `false`. Carries the transport name for the message
    /// template, quoted identically to the previous `format!(...)` output.
    Incapable { transport: String },
}

impl std::fmt::Display for PartitionTransportBindingFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown { known_names } => write!(
                f,
                "unknown transport name (known implemented transports: {})",
                known_names.join(", ")
            ),
            Self::Incapable { transport } => write!(
                f,
                "transport '{transport}' does not carry inter-partition IPC \
                 (supports_inter_partition_ipc = false)"
            ),
        }
    }
}

/// Why a cross-device `<invoke type="scxml" src="#<peer>">` deploy
/// declaration was rejected by
/// [`DeployError::ScxmlInvokeCrossDeviceTransport`] (SCE_MESH.md §9.6
/// L1393). The shapes are structurally distinct so tests + IDE
/// diagnostics can match without prose-parsing:
///
/// - `MissingBinding` — parent declares no `bindings["#<peer>"]` entry
///   at all; the cross-device invoke has no transport declaration.
/// - `TransportIncapable` — binding present but names `shm` or `local`,
///   transports that cannot cross a device boundary (shm segments are
///   pid-namespaced; local is in-process).
/// - `TransportUnwired` — binding names a structurally capable
///   transport (someip / zenoh / dds) but the C++ wire-14/20 dispatch
///   has not yet landed for it. Same precedent as §16.5's
///   `partition-wire21-custom-tcp-unimplemented`: reject at build time
///   rather than silent runtime fallback.
/// - `TransportListenMissing` — binding selects a wired transport
///   whose device-shared server model requires a `listen:` endpoint
///   on the named device (today: custom_tcp on parent or peer), but
///   the device's `transports.<transport>` config omits it. Without
///   `listen:` the generated `TransportRouter` skips the Server
///   emission, so wire-15/16/18/20 replies (parent-side) or
///   wire-14/17/19 requests (worker-side) have no inbound channel —
///   silent send-only failure. The device field names which side is
///   missing the config so the fix is unambiguous.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScxmlInvokeCrossDeviceFailure {
    MissingBinding,
    TransportIncapable { transport: String },
    TransportUnwired { transport: String },
    TransportListenMissing { transport: String, device: String },
}

impl std::fmt::Display for ScxmlInvokeCrossDeviceFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingBinding => write!(
                f,
                "parent declares no `bindings[\"#<peer>\"]` entry for the cross-device peer"
            ),
            Self::TransportIncapable { transport } => write!(
                f,
                "transport '{transport}' cannot cross a device boundary \
                 (shm segments are pid-namespaced; local is in-process)"
            ),
            Self::TransportUnwired { transport } => write!(
                f,
                "transport '{transport}' is structurally capable but \
                 cross-device wire-14/20 dispatch has not landed for it \
                 yet (shm same-device and custom_tcp are the wired paths)"
            ),
            Self::TransportListenMissing { transport, device } => write!(
                f,
                "transport '{transport}' requires `transports.{transport}.listen:` \
                 on device '{device}' so the device-shared server can receive \
                 the reply stream (parent) or request stream (worker); without it \
                 the generated TransportRouter skips the Server emission and the \
                 inbound direction is silently dropped"
            ),
        }
    }
}

/// Errors from deploy.yaml deserialization.
#[derive(Debug, thiserror::Error)]
pub enum DeployError {
    /// Cannot read the deploy.yaml file from disk.
    #[error("cannot read deploy config '{path}': {source}")]
    ReadFile {
        path: String,
        source: std::io::Error,
    },

    /// YAML syntax or schema error in deploy.yaml.
    #[error("deploy.yaml parse error: {0}")]
    Yaml(String),

    /// `version:` field specifies a schema version this compiler does not know.
    /// Prevents silent misinterpretation of fields whose semantics change
    /// between schema revisions.
    #[error("deploy.yaml version '{found}' is not supported. Supported: {supported}. \
             Update sce-codegen or change the `version:` field.",
             supported = .supported.join(", "))]
    UnsupportedVersion {
        found: String,
        supported: Vec<&'static str>,
    },

    /// A machine name appears under more than one device in the topology.
    /// Since machine names are used globally (receiver lookup, `<send
    /// target="#X"/>` resolution, generated namespace), they must be unique
    /// across the entire deployment.
    ///
    /// Fix: rename one of the machines, or remove the duplicate declaration.
    #[error("machine '{machine}' is declared on multiple devices: {}. \
             Machine names must be globally unique across the deployment.",
             .devices.join(", "))]
    DuplicateMachine {
        machine: String,
        devices: Vec<String>,
    },

    /// A machine declared an `ordering:` section with a value that
    /// violates the per-machine ordering timing constraints
    /// (SCE_MESH.md §10.6.1): both fields must be positive AND
    /// `tick_period_ms` must be strictly less than `gap_timeout_ms`
    /// (Nyquist). Rejected at parse time so a bad value cannot reach
    /// the generated router.
    #[error("machine '{machine}': invalid `ordering:` section in deploy.yaml — {reason}. \
             Either fix the values or omit the section entirely to accept the defaults.")]
    InvalidOrderingTimings {
        machine: String,
        reason: String,
    },

    /// A machine declared a `liveliness:` section whose `lease_ms`
    /// violates the minimum-floor constraint (SCE Mesh §16.7 row 8;
    /// see `MIN_LIVELINESS_LEASE_MS` in deploy.rs). Rejected at parse
    /// time so a bad value cannot reach the generated router.
    #[error("machine '{machine}': invalid `liveliness:` section in deploy.yaml — {reason}. \
             Either fix the value or omit the section entirely to disable liveliness.")]
    InvalidLiveliness {
        machine: String,
        reason: String,
    },

    /// A machine declared a `server.query_timeout_ms` whose value
    /// violates the minimum-floor constraint (SCE Mesh §9.5 gap Z2;
    /// see `MIN_SERVER_QUERY_TIMEOUT_MS` in deploy.rs). Rejected at
    /// parse time so a bad value cannot reach the generated router.
    #[error("machine '{machine}': invalid `server.query_timeout_ms` in deploy.yaml — {reason}. \
             Either fix the value or omit the knob entirely to disable the server deadline.")]
    InvalidServerQueryTimeout {
        machine: String,
        reason: String,
    },

    /// A machine declared an `outbound_buffer:` section whose
    /// `max_pending_per_target` violates the minimum constraint
    /// (SCE Mesh §10.10; see `MIN_OUTBOUND_BUFFER_MAX_PENDING` in
    /// deploy.rs). Rejected at parse time so a zero-capacity buffer —
    /// semantically equivalent to no buffer — cannot reach the
    /// generated router under the guise of opting into §10.10
    /// readiness gating.
    #[error("machine '{machine}': invalid `outbound_buffer:` section in deploy.yaml — {reason}. \
             Either fix the value or omit the section entirely to opt out of §10.10 buffering.")]
    InvalidOutboundBuffer {
        machine: String,
        reason: String,
    },

    /// A `discovery:` top-level block appeared in deploy.yaml. SCE Mesh
    /// §3.3 invariant: transport-native routing is the source of truth
    /// for peer availability, and SCE does not maintain a peer table
    /// (§2572 rejected list, §2574 rejection of `discovery.mode:
    /// static | dynamic`). Authors wanting per-binding runtime target
    /// selection use value-field placeholders (§14.4); authors wanting
    /// transport-level peer discovery configure the external OEM config
    /// (zenoh.json5 scouting, vsomeip.json service-discovery). Rejected
    /// at parse time so an authored block cannot silently round-trip
    /// through codegen producing no runtime behaviour.
    #[error("deploy.yaml 'discovery:' top-level block is not supported ({content_kind}). \
             SCE Mesh §3.3 invariant: transport-native routing is the source of truth for \
             peer availability; SCE does not maintain a peer table (§2572 rejected list, \
             §2574 rejection of `discovery.mode: static | dynamic`). For per-binding runtime \
             target selection use value-field placeholders (§14.4). For transport-level peer \
             discovery configure the external OEM config (zenoh.json5 scouting, \
             vsomeip.json service-discovery).")]
    DiscoveryNotSupported { content_kind: String },

    /// A binding carries a `{name}` placeholder value but the binding's
    /// transport declares `supports_pool: false` in the registry
    /// (SCE Mesh §14.4). Transports without a native routing layer
    /// (local, shm, custom_tcp, can) cannot substitute runtime values
    /// without SCE reimplementing transport discovery, which the §3.3
    /// design invariant explicitly rejects.
    #[error("machine '{machine}': binding '{binding}' on transport '{transport}' carries a \
             '{{name}}' placeholder, but this transport does not support pool bindings \
             (supports_pool = false). Use a routing-capable transport (zenoh, someip) or \
             drop the placeholder.")]
    PoolNotSupportedByTransport {
        machine: String,
        binding: String,
        transport: String,
    },

    /// A SOME/IP binding uses a placeholder but does not declare the
    /// required `instances:` list (SCE Mesh §14.4). vsomeip's
    /// `request_service(SERVICE, ANY_INSTANCE)` is interpreted as
    /// specific-instance-0xFFFF, not a wildcard, so codegen must know
    /// the finite set of instance IDs to pre-request at init().
    #[error("machine '{machine}': SOME/IP binding '{binding}' uses a '{{name}}' placeholder \
             but is missing the required `instances:` list. vsomeip does not support \
             open-ended instance subscription; declare the expected instance IDs explicitly.")]
    PoolMissingInstanceList {
        machine: String,
        binding: String,
    },

    /// A binding declared an empty `instances: []` list (SCE Mesh §14.4).
    /// An empty pool would generate zero `request_service` calls and the
    /// runtime would refuse every placeholder value, so the
    /// configuration is silently broken.
    #[error("machine '{machine}': binding '{binding}' has an empty `instances: []` list. \
             Declare at least one instance ID or remove the list entirely.")]
    PoolEmptyInstanceList {
        machine: String,
        binding: String,
    },

    /// A binding value field contains a malformed placeholder (unbalanced
    /// braces, empty name, invalid characters). Rejected at parse time
    /// so a malformed placeholder cannot be confused with a literal
    /// brace in the value string.
    #[error("machine '{machine}': binding '{binding}' has an invalid placeholder — {reason}. \
             Fix the placeholder syntax or escape intended literal braces.")]
    PoolInvalidPlaceholder {
        machine: String,
        binding: String,
        reason: String,
    },

    /// A machine's `server:` section declared `instances: [...]` — a
    /// server-side pool on a transport whose native routing has no
    /// peer-identifying inbound distinguisher. Today only SOME/IP
    /// exposes `msg->get_instance()` at dispatch time, so it is the
    /// sole transport whose registry entry sets
    /// `supports_multi_instance_server: true`. Other transports reject
    /// `server.instances:` at parse time; the `transport` field lets
    /// the author see which transport the deploy.yaml declared so the
    /// diagnostic distinguishes the per-transport policy from the
    /// machine-level one. See SCE_MESH.md §14.4.
    #[error("machine '{machine}': `server.instances:` is not supported on transport '{transport}' \
             — only transports with a peer-identifying inbound distinguisher (SOME/IP today) can \
             host a multi-instance server pool. Drop `instances:` from the server section, switch \
             the server transport to one that supports pools, or run N processes each hosting a \
             single-instance server. See SCE_MESH.md §14.4.")]
    ServerPoolNotSupported {
        machine: String,
        transport: String,
    },

    /// A single machine is used as both a remote `<invoke type="scxml"
    /// src="#<M>">` target (mesh peer) and a local `<invoke src="<M's
    /// source path>">` target (direct file reference) within the same
    /// deployment (SCE_MESH.md §9.6). The two shapes require different
    /// code-generation decisions on `<M>`: a mesh peer must emit a
    /// non-templated, default-constructible engine for
    /// `ChildSessionAdapter<Engine>` (§9.6 child session lifecycle),
    /// while a local invoke target must emit the `ParentStateMachine`-
    /// templated shape so the parent's `Event` enum is reachable at
    /// `<send target="#_parent">` emission time. Supporting both
    /// simultaneously would silently break one caller.
    ///
    /// Fix: either drop the local-path `<invoke src="...">` on the
    /// named invoker and call the mesh peer through `#<M>`, or remove
    /// `<M>` from the deploy.yaml topology so it ceases to be a mesh
    /// peer.
    #[error("machine '{machine}' is both a remote `<invoke type=\"scxml\" src=\"#{machine}\">` \
             target (mesh peer, inbound from: {inbound_peers}) and a local-path invoke target of \
             machine '{local_invoker}' (src=\"{local_src}\"). These two shapes cannot coexist: \
             the mesh peer shape is default-constructible for SCE_MESH.md §9.6 \
             `ChildSessionAdapter<Engine>`, while the local shape carries a `ParentStateMachine` \
             template parameter. Fix: drop one — either change '{local_invoker}' to invoke \
             '#{machine}' through mesh, or remove '{machine}' from deploy.yaml topology.",
             inbound_peers = .inbound_peers.join(", "))]
    ScxmlInvokeTargetConflict {
        machine: String,
        inbound_peers: Vec<String>,
        local_invoker: String,
        local_src: String,
    },

    /// Two entries under `partitions:` declare the same partition name
    /// (SCE_MESH.md §14 rule 6). Partition names are process identities
    /// at runtime and double as log-correlation tags; aliased entries
    /// would silently mask the earlier one under standard YAML map
    /// semantics, so the parser detects the collision via a custom
    /// [`crate::mesh::deploy::PartitionMap`] deserializer and surfaces
    /// it here before the downstream validators run on a truncated map.
    #[error("partition name '{name}' is declared more than once under `partitions:`. \
             Partition names are globally unique process identities (SCE_MESH.md §14 rule 6). \
             Rename one of the entries or delete the duplicate.")]
    PartitionDuplicateName { name: String },

    /// A partition's `contains:` references machines hosted on more
    /// than one device (SCE_MESH.md §14 rule 7). A partition occupies
    /// exactly one process on one device; spanning multiple devices
    /// would require cross-device transport for the partition's
    /// internal membership, contradicting the single-process
    /// abstraction. Split the partition into one-per-device entries.
    #[error("partition '{partition}': its `machines:` list spans more than one device ({devices}). \
             A partition is one process on one device (SCE_MESH.md §14 rule 7). Split the \
             partition into one entry per device, or narrow `machines:` to a single-device set.",
             devices = .devices.join(", "))]
    PartitionMultiDevice {
        partition: String,
        devices: Vec<String>,
    },

    /// A single orthogonal unit (parallel region or invoke) appears in
    /// more than one partition's `contains:` block (SCE_MESH.md §14
    /// rule 8). The unit would have no well-defined host process; the
    /// analyzer never silently picks one. Remove the unit from every
    /// partition except the intended one.
    #[error("unit '{unit}' appears in more than one partition ({partitions}). Each \
             orthogonal unit belongs to exactly one partition (SCE_MESH.md §14 rule 8). \
             Remove the entry from every partition except the intended one.",
             partitions = .partitions.join(", "))]
    PartitionUnitDuplicate {
        unit: String,
        partitions: Vec<String>,
    },

    /// A partition's `contains:` entry references a machine that is
    /// not listed in the same partition's `machines:` field
    /// (SCE_MESH.md §14 rule 9). A partition cannot reach into
    /// another partition's address space; the membership declaration
    /// and the `contains:` entries must agree.
    #[error("partition '{partition}': `contains:` entry references machine '{machine}', \
             but '{machine}' is not listed under the partition's `machines:` field. \
             Add '{machine}' to `machines:` or remove the stray entry (SCE_MESH.md §14 rule 9).")]
    PartitionMachineNotListed {
        partition: String,
        machine: String,
    },

    /// A partition has no `contains:` entries at all — no parallel
    /// regions and no invokes (SCE_MESH.md §14 rule 10). An empty
    /// partition has no runtime purpose and usually indicates a
    /// copy-paste error. Authors who want a reserved entry must
    /// declare the units they plan to host.
    #[error("partition '{partition}' is empty (no `contains.parallel_regions:` and no \
             `contains.invokes:`). Empty partitions have no runtime purpose (SCE_MESH.md \
             §14 rule 10); either add the units this partition hosts or delete the entry.")]
    PartitionEmpty { partition: String },

    /// An author-declared machine id contains the reserved
    /// `__sce_synth_invoke__` infix (SCE_MESH.md §14 rule 5 + §9.6.6).
    /// The infix is used to name machines synthesised from
    /// `<invoke type="scxml">` inline `<content>`: a colliding author
    /// id would shadow or be shadowed by a synthesised peer at runtime,
    /// and partition rules 1-2 could not disambiguate the two. Detected
    /// unconditionally across every `topology.*.machines.*` key — the
    /// reservation stands even when `partitions:` is absent, so that
    /// opting into partitions later never turns previously-valid ids
    /// into silent collisions.
    #[error("machine '{machine}' uses the reserved `__sce_synth_invoke__` infix in its \
             name. SCE Mesh §14 rule 5 reserves this substring for machines synthesised \
             from `<invoke type=\"scxml\">` inline `<content>` (§9.6.6); an author id \
             collision would silently shadow the synthesised peer at runtime. Rename the \
             machine to drop the substring.")]
    PartitionSynthInfixCollision { machine: String },

    /// A machine listed under some partition's `machines:` leaves one or
    /// more orthogonal units (parallel regions or invokes) uncovered,
    /// *and* a `<machine>_default:` partition already exists
    /// (SCE_MESH.md §14 rule 1). The default is therefore incomplete —
    /// the cheapest repair is to extend its `contains:` with the
    /// missing units, which is why this diagnostic is distinct from
    /// [`Self::PartitionPartialCoverageRequiresDefault`] (where the
    /// default partition has not been declared at all and the author
    /// may prefer to assign units into other existing partitions).
    #[error("machine '{machine}' has partitions declared but the following orthogonal \
             units are not covered by any partition's `contains:`: {}. The \
             '{machine}_default' partition exists, so the direct repair is to extend its \
             `contains:` with the missing entries (SCE_MESH.md §14 rule 1).",
             .units.iter().map(|u| format!("\n  - {u}")).collect::<String>())]
    PartitionUncoveredUnit {
        machine: String,
        units: Vec<String>,
    },

    /// A machine is mentioned in some partition's `machines:` list but
    /// one or more of its orthogonal units is not covered by any
    /// partition's `contains:`, and **no** `<machine>_default:`
    /// partition has been declared (SCE_MESH.md §14 rule 2). The error
    /// message reproduces the spec L2793-2800 wording verbatim so
    /// authors can follow the prescribed repair (extend an existing
    /// partition or add a dedicated `<machine>_default:` with the
    /// missing entries). Distinct from [`Self::PartitionUncoveredUnit`]
    /// because the `_default` suggestion is only honest when that
    /// partition does not already exist.
    #[error("machine '{machine}' has partitions declared, but the following orthogonal \
             units are unassigned:{}\n            Either add them to an existing \
             partition under `machines: [{machine}]`, or declare a '{machine}_default' \
             partition with `contains:` entries for each (SCE_MESH.md §14 rule 2).",
             .missing.iter().map(|u| format!("\n              - {u}")).collect::<String>())]
    PartitionPartialCoverageRequiresDefault {
        machine: String,
        missing: Vec<String>,
    },

    /// A machine that declares a SOME/IP server pool (`server.instances:`,
    /// SCE_MESH.md §14.4) is listed under some partition's `machines:`
    /// block. The two grammars describe orthogonal axes: a pool is one
    /// router offering N SOME/IP sessions on a single process, while a
    /// partition splits a machine across M OS processes (SCE_MESH.md
    /// §14). deploy.yaml does not define a combined meaning today, so
    /// the combination is rejected at parse time instead of silently
    /// accepted. Authors drop the pool to partition the machine, or
    /// drop the partition listing to keep the pool.
    #[error("machine '{machine}' declares `server.instances:` (SCE Mesh §14.4 SOME/IP \
             server pool) but partition '{partition}' lists it under `machines:`. A pool \
             is one router hosting N SOME/IP sessions on a single process; a partition \
             splits a machine across M OS processes (SCE_MESH.md §14). deploy.yaml does \
             not define the combined meaning — either remove '{machine}' from \
             partition '{partition}' `machines:` (keep the pool as one monolithic \
             process), or drop `server.instances:` from the machine and run N processes \
             each hosting a single-instance server.")]
    PartitionPoolMachine {
        machine: String,
        partition: String,
    },

    /// A partition declared `transport_binding:` naming a transport
    /// that does not carry inter-partition IPC within a single
    /// machine (SCE_MESH.md §14 L2729-2730). The spec default is
    /// "kind tcp/shm"; today `shm` and `custom_tcp` qualify. A
    /// transport the registry does not recognise, or a recognised
    /// transport whose `supports_inter_partition_ipc` is `false`,
    /// both fall here — [`PartitionTransportBindingFailure`]
    /// discriminates the two shapes so the author sees why the value
    /// was rejected and downstream consumers can match structurally
    /// without parsing the prose. Partition IPC carried over `local`
    /// cannot cross process boundaries; carrying it over `someip` /
    /// `zenoh` / `dds` / `can` routes through a middleware daemon or
    /// broadcast fabric instead of the direct channel §14 intends.
    #[error("partition '{partition}': `transport_binding: {transport}` is not a valid \
             inter-partition IPC transport — {failure}. SCE Mesh §14 requires a transport \
             whose primary purpose is same-machine IPC (today: shm, custom_tcp). Switch to \
             one of those or omit `transport_binding:` to accept the default (§14 L2730 \
             \"kind tcp/shm\").")]
    PartitionTransportBindingUnsupported {
        partition: String,
        transport: String,
        failure: PartitionTransportBindingFailure,
    },

    /// SCE_MESH.md §9.6 L1393 — `<invoke type="scxml" src="#<peer>">`
    /// classified as cross-device (parent's partition's `device:` differs
    /// from peer's partition's `device:`) but the parent's
    /// `bindings["#<peer>"]` declaration is absent, names an incapable
    /// transport, or names a transport whose Session F wire-14/20
    /// dispatch has not yet landed in C++ codegen. [`ScxmlInvokeCrossDeviceFailure`]
    /// discriminates the three shapes so downstream consumers (tests,
    /// IDE diagnostics, CI error mapping) can match structurally.
    ///
    /// Same-device cross-partition invokes are accepted without any
    /// `bindings` declaration — they take the implicit shm channel
    /// which is today's only wired path (§9.6.2 wire-14/20 over shm).
    #[error("machine '{parent}' (device '{parent_device}') → \
             `<invoke type=\"scxml\" src=\"#{peer}\">` on device '{peer_device}': {failure}. \
             SCE Mesh §9.6 L1393 requires each cross-device scxml-remote peer to declare \
             its transport on `machines.{parent}.bindings[\"#{peer}\"].transport`, and that \
             transport must be both capable of crossing devices AND wired by the Session F \
             C++ dispatch.")]
    ScxmlInvokeCrossDeviceTransport {
        parent: String,
        peer: String,
        parent_device: String,
        peer_device: String,
        failure: ScxmlInvokeCrossDeviceFailure,
    },

    /// §9.6 SOMEIP scxml-invoke participant count exceeds the
    /// hybrid-allocator sub-range ceiling (RFC F.X-1). Subsystem range
    /// partitioning gives invoke 128 slots in `[0x8100, 0x817F]`; the upper
    /// half of the SCE-reserved range is reserved for §16.4 region-liveness
    /// (F.X-3). Beyond 128 §9.6 SOMEIP participants, the hybrid counter +
    /// pin allocator cannot fit them inside the sub-range — operator must
    /// either reduce the participant count or wait on the multi-domain
    /// landing (today's single-domain assumption is the conservative
    /// trade).
    #[error("§9.6 SOME/IP scxml-invoke service-ID overflow: \
             {participant_count} participants exceed the {ceiling}-slot \
             sub-range ceiling [0x8100, 0x817F] (RFC F.X-1 subsystem range \
             partitioning reserves [0x8180, 0x81FF] for §16.4 region-liveness). \
             Reduce the §9.6 SOMEIP participant count or split deploy.yaml \
             across multi-OEM domains (multi-domain support is a separate \
             landing).")]
    SomeipScxmlInvokeServiceIdOverflow {
        participant_count: usize,
        ceiling: usize,
    },

    /// A machine pinned `someip_service_id:` outside the §9.6 invoke
    /// sub-range `[0x8100, 0x817F]`. Pins outside the sub-range either
    /// collide with the F.X-3 region-liveness reservation (`[0x8180,
    /// 0x81FF]`) or escape the SCE-reserved range entirely
    /// (`[0x0000, 0x80FF]` / `[0x8200, 0xFFFF]` are OEM-owned). Operator
    /// fix: choose a pin inside the sub-range, or remove the pin to let
    /// the counter auto-assign.
    #[error("machine '{machine}': pinned `someip_service_id: {pinned_id:#06x}` \
             is outside the §9.6 SOMEIP scxml-invoke sub-range \
             [{range_lo:#06x}, {range_hi:#06x}] (RFC F.X-1). The upper half of \
             the SCE-reserved range is reserved for §16.4 region-liveness; pins \
             outside the SCE-reserved range collide with OEM-owned service \
             space. Pick a value inside [{range_lo:#06x}, {range_hi:#06x}] \
             or drop the pin to use the auto-assigner.")]
    SomeipScxmlInvokeServiceIdPinOutOfRange {
        machine: String,
        pinned_id: u16,
        range_lo: u16,
        range_hi: u16,
    },

    /// Two or more machines pinned the same `someip_service_id:` — author
    /// error, deterministic conflict at parse time. Operator fix: choose a
    /// distinct pin for one of the listed machines, or drop one pin to let
    /// the counter auto-assign.
    #[error("§9.6 SOME/IP scxml-invoke service-ID pin collision at \
             {pinned_id:#06x}: machines [{}] all pin the same value via \
             deploy.yaml `someip_service_id:`. Each pin must be unique \
             inside the [0x8100, 0x817F] sub-range. Repick the pin on one \
             of the listed machines or drop a pin to fall back to the \
             counter auto-assigner.",
             .machines.iter().map(|m| format!("'{m}'"))
                 .collect::<Vec<_>>().join(", "))]
    SomeipScxmlInvokeServiceIdPinCollision {
        machines: Vec<String>,
        pinned_id: u16,
    },

    /// Total §16.4 SOMEIP region-liveness participant count exceeds the
    /// 128-slot ceiling of the liveness sub-range `[0x8180, 0x81FF]`
    /// (RFC F.X-3). Operator fix: reduce the partition count, or split
    /// the deploy across multi-OEM domains (separate landing).
    #[error("§16.4 SOME/IP region-liveness service-ID overflow: \
             {participant_count} partitions exceed the {ceiling}-slot \
             sub-range ceiling [0x8180, 0x81FF] (RFC F.X-3 subsystem range \
             partitioning reserves the upper half of the SCE-reserved \
             space for region-liveness, disjoint from §9.6 invoke's \
             [0x8100, 0x817F]). Reduce the §16.4 SOMEIP partition count \
             or split deploy.yaml across multi-OEM domains.")]
    SomeipLivenessServiceIdOverflow {
        participant_count: usize,
        ceiling: usize,
    },

    /// A partition pinned `someip_liveness_service_id:` outside the §16.4
    /// liveness sub-range `[0x8180, 0x81FF]`. Pins below the sub-range
    /// collide with the F.X-1 invoke reservation; pins above escape the
    /// SCE-reserved range entirely. Operator fix: choose a pin inside
    /// the sub-range, or remove the pin to let the counter auto-assign.
    #[error("partition '{partition_key}': pinned \
             `someip_liveness_service_id: {pinned_id:#06x}` is outside \
             the §16.4 SOMEIP region-liveness sub-range \
             [{range_lo:#06x}, {range_hi:#06x}] (RFC F.X-3). The lower \
             half of the SCE-reserved range is reserved for §9.6 \
             scxml-invoke; pins outside the SCE-reserved range collide \
             with OEM-owned service space. Pick a value inside \
             [{range_lo:#06x}, {range_hi:#06x}] or drop the pin to use \
             the auto-assigner.")]
    SomeipLivenessServiceIdPinOutOfRange {
        partition_key: String,
        pinned_id: u16,
        range_lo: u16,
        range_hi: u16,
    },

    /// Two or more partitions pinned the same `someip_liveness_service_id:`
    /// — author error, deterministic conflict at parse time. Operator
    /// fix: choose a distinct pin for one of the listed partitions, or
    /// drop one pin to let the counter auto-assign.
    #[error("§16.4 SOME/IP region-liveness service-ID pin collision at \
             {pinned_id:#06x}: partitions [{}] all pin the same value \
             via deploy.yaml `someip_liveness_service_id:`. Each pin must \
             be unique inside the [0x8180, 0x81FF] sub-range. Repick the \
             pin on one of the listed partitions or drop a pin to fall \
             back to the counter auto-assigner.",
             .partition_keys.iter().map(|k| format!("'{k}'"))
                 .collect::<Vec<_>>().join(", "))]
    SomeipLivenessServiceIdPinCollision {
        partition_keys: Vec<String>,
        pinned_id: u16,
    },

    /// A partition declared `barrier_timeout_ms:` with a value that
    /// makes the distributed parallel-final barrier (SCE_MESH.md
    /// §16.5) meaningless. Today the sole rejected value is `0`: a
    /// zero-millisecond timer would fire before the first region can
    /// report `ParallelRegionDone`, raising
    /// `error.communication / PARALLEL_BARRIER_TIMEOUT` on every
    /// `<parallel>` activation irrespective of region progress — the
    /// knob exists to bound authentic hangs, not to unconditionally
    /// convert barriers into errors. Authors who genuinely want "do
    /// not wait" should omit the knob and let the W3C normative
    /// default (infinity, per spec L2732) apply, then handle
    /// non-completion through standard SCXML transitions. Range-only
    /// at parse time; the deeper rule that `barrier_timeout_ms:`
    /// applies only on partitions hosting a `<parallel>` root is a
    /// SCXML cross-reference rule (SCE_MESH.md §16.5) — its runtime
    /// consumer is the §16.5 scope, not this validator.
    #[error("partition '{partition}': `barrier_timeout_ms: {value}` is invalid — {reason}. \
             SCE Mesh §14 L2731-2732 pins the W3C normative default as infinity (null / \
             field omitted); finite values must be >= 1 ms. Either fix the value or drop \
             the key to accept the default.")]
    PartitionBarrierTimeoutInvalid {
        partition: String,
        value: u32,
        reason: String,
    },

    /// A distributed `<parallel>` (regions span two or more partitions)
    /// has no partition claiming its root via `hosts_parallel_roots:`
    /// (SCE_MESH.md §14 rule 12, L2838). Without a claimant, the
    /// §16.5 `ParallelCompletionTracker` has no unique owner and
    /// `done.state.<parallel_id>` cannot be raised. The author repairs
    /// by naming exactly one partition (of those hosting at least one
    /// region of the parallel) as the root.
    #[error("machine '{machine}': distributed `<parallel id=\"{parallel}\">` (regions span \
             partitions {}) has no root claimant. SCE Mesh §14 rule 12 requires exactly \
             one partition to declare `hosts_parallel_roots: [{{ machine: {machine}, \
             parallel: {parallel} }}]`. Add the entry to one of the listed partitions — \
             the root must co-host at least one region of the parallel.",
             .hosting_partitions.iter().map(|p| format!("'{p}'"))
                 .collect::<Vec<_>>().join(", "))]
    PartitionParallelRootUndesignated {
        machine: String,
        parallel: String,
        hosting_partitions: Vec<String>,
    },

    /// Two or more partitions claim the same `(machine, parallel)` pair
    /// as their root (SCE_MESH.md §14 rule 12, L2839). Tracker
    /// ownership is per-`<parallel>`-unique; ambiguous claims would
    /// produce two `done.state.<parallel_id>` raises (one per root).
    /// The author repairs by removing all but one claim.
    #[error("machine '{machine}': `<parallel id=\"{parallel}\">` is claimed as root by \
             multiple partitions: {}. SCE Mesh §14 rule 12 requires exactly one claimant \
             per distributed parallel. Remove the entry from all but one partition's \
             `hosts_parallel_roots:`.",
             .claiming_partitions.iter().map(|p| format!("'{p}'"))
                 .collect::<Vec<_>>().join(", "))]
    PartitionParallelRootAmbiguous {
        machine: String,
        parallel: String,
        claiming_partitions: Vec<String>,
    },

    /// A `hosts_parallel_roots[*].machine:` entry names a machine that
    /// is not in the partition's `machines:` list (SCE_MESH.md §14 rule
    /// 12, L2840 — rule 9 shape applied to root entries). One partition
    /// cannot claim root for a parallel in another partition's
    /// machine's document.
    #[error("partition '{partition}': `hosts_parallel_roots:` entry claims machine \
             '{claimed_machine}' but the partition's `machines:` list is [{}]. SCE Mesh \
             §14 rule 12 applies rule 9 shape to root entries — the claimed machine must \
             be one the partition already lists. Either add '{claimed_machine}' to \
             `machines:` or move the `hosts_parallel_roots:` entry to a partition that \
             already lists it.",
             .partition_machines.iter().map(|m| format!("'{m}'"))
                 .collect::<Vec<_>>().join(", "))]
    PartitionParallelRootNotInMachines {
        partition: String,
        claimed_machine: String,
        partition_machines: Vec<String>,
    },

    /// A partition claims `(machine, parallel)` as its root but hosts
    /// no region of that `<parallel>` in its `contains.parallel_regions:`
    /// (SCE_MESH.md §14 rule 12, L2841). A root that co-hosts no region
    /// would force every region update to cross process boundaries as
    /// inter-partition traffic — the spec rejects the shape to keep the
    /// §16.5 tracker's aggregation path coherent.
    #[error("partition '{partition}': claims root for machine '{machine}' \
             `<parallel id=\"{parallel}\">` but hosts no region of that parallel in \
             `contains.parallel_regions:`. SCE Mesh §14 rule 12 requires a root claimant \
             to co-host at least one region — otherwise every region update crosses \
             partitions as inter-partition traffic. Either add a region of the parallel \
             to this partition's `contains:`, or move the `hosts_parallel_roots:` entry \
             to a partition that already hosts one.")]
    PartitionParallelRootNonHost {
        partition: String,
        machine: String,
        parallel: String,
    },

    /// A partition declared `barrier_timeout_ms:` but did not claim
    /// any `<parallel>` root via `hosts_parallel_roots:` (SCE_MESH.md
    /// §14 rule 12, L2842). The timeout has no §16.5 tracker to gate
    /// — it would silently do nothing. Distinct from
    /// [`Self::PartitionBarrierTimeoutInvalid`] which is a value-range
    /// check; this diagnostic catches the orthogonal configuration
    /// error of setting a timeout on a partition that is not a root.
    #[error("partition '{partition}': `barrier_timeout_ms: {value}` is set but the \
             partition has no `hosts_parallel_roots:` entries. SCE Mesh §14 rule 12 \
             (L2842) requires the timeout to gate a §16.5 `ParallelCompletionTracker`, \
             and trackers only exist on root-hosting partitions. Either add a \
             `hosts_parallel_roots:` entry (making this partition a root) or drop \
             `barrier_timeout_ms:` (which has no consumer here).")]
    PartitionBarrierTimeoutWithoutRoot {
        partition: String,
        value: u32,
    },

    /// A partition declared `transport_binding: custom_tcp` and
    /// participates in a distributed `<parallel>` (§16.5) wire-21
    /// route — but the wire-21 channel emitter currently materializes
    /// `PartitionWire21Channel = SCE::Mesh::ShmChannel<>` unconditionally.
    /// Compiling such a partition would produce a shm channel the
    /// runtime never opens, which surfaces at SM step time as a
    /// missing-callback throw inside `sendParallelRegionDone`.
    /// Reject at deploy-validation time so the configuration gap
    /// surfaces here instead of as a delayed runtime fault. Spec §14
    /// rule 4 accepts custom_tcp; the gap is in the codegen surface
    /// (SCE_MESH.md §16.5 banner + matrix carve-out).
    #[error("partition '{partition}': `transport_binding: custom_tcp` is set, but the \
             partition participates in distributed `<parallel id=\"{parallel}\">` \
             (machine '{machine}') wire-21 routing. The §16.5 wire-21 channel emitter \
             currently supports `transport_binding: shm` only — a `custom_tcp` channel \
             is not yet generated for ParallelRegionDone forwarding. Either change this \
             partition's `transport_binding:` to `shm` (same-device deployments), or \
             remove the partition from any distributed `<parallel>` route until the \
             custom_tcp wire-21 emitter lands.")]
    PartitionWire21CustomTcpUnimplemented {
        partition: String,
        machine: String,
        parallel: String,
    },

    /// SCE_MESH.md §16.3 R1 — two or more child regions of a
    /// `<parallel>` write the same ancestor-scope data location.
    /// Under `distributability: strict` this is a build failure;
    /// `permissive` (the default) auto-merges the offending regions
    /// per §16.4 and records a
    /// [`crate::mesh::distributability::MergeNotice`] instead.
    #[error("machine '{machine}', `<parallel id=\"{parallel}\">`: R1 shared-write — \
             regions {} all assign to ancestor data '{location}'. \
             SCE_MESH.md §16.3 R1 forbids this because distribution cannot preserve \
             W3C sequential consistency on shared writable state without cross-process \
             locks. Either place these regions in the same partition or move the \
             shared variable into per-region datamodels. (Set `distributability: \
             permissive` in deploy.yaml to auto-merge instead of failing the build.)",
             .regions.iter().map(|r| format!("'{r}'"))
                 .collect::<Vec<_>>().join(", "))]
    DistributabilityR1SharedWrite {
        machine: String,
        parallel: String,
        location: String,
        regions: Vec<String>,
    },

    /// SCE_MESH.md §16.3 R2 — a `<transition target>` resolves to a
    /// state inside a sibling region of the same `<parallel>`.
    /// Cross-region transitions require macrostep atomicity across
    /// the W3C exit-set/enter-set computation, which distribution
    /// cannot supply. `strict` fails; `permissive` auto-merges via
    /// §16.4.
    #[error("machine '{machine}', `<parallel id=\"{parallel}\">`: R2 cross-region \
             transition — regions {} are connected by a transition that crosses \
             the region boundary. SCE_MESH.md §16.3 R2 forbids this because \
             distribution cannot preserve the W3C exit-set/enter-set computation \
             atomically across partitions. Either merge the regions into one \
             partition, or refactor the transition target to an ancestor of the \
             `<parallel>` (which exits it wholesale and is distribution-safe). \
             (Set `distributability: permissive` in deploy.yaml to auto-merge \
             instead of failing the build.)",
             .regions.iter().map(|r| format!("'{r}'"))
                 .collect::<Vec<_>>().join(", "))]
    DistributabilityR2CrossRegionTransition {
        machine: String,
        parallel: String,
        regions: Vec<String>,
    },
}

// ── Stage 1b: External infrastructure config ─────────────────

/// Errors from vsomeip.json / zenoh.json5 parsing and 3-way name resolution.
///
/// External config files are owned by the platform team (OEM tooling,
/// ARXML/Franca pipelines). sce-build reads them to resolve deploy.yaml
/// name references into numeric transport IDs; diagnostics therefore carry
/// the file path so operators can correlate with their OEM source.
#[derive(Debug, thiserror::Error)]
pub enum ExternalConfigError {
    /// Cannot read the external config file from disk.
    #[error("cannot read external config '{path}': {source}")]
    Read {
        path: String,
        source: std::io::Error,
    },

    /// External config file is malformed (JSON / JSON5 syntax or schema).
    #[error("external config '{path}' parse error: {reason}")]
    Parse { path: String, reason: String },

    /// deploy.yaml references one or more entities that do not exist in
    /// the referenced external config. All unresolved references for a
    /// single (machine, config) pair are batched into one error so operators
    /// see the full picture instead of fixing them one at a time.
    ///
    /// Format mirrors SCE_MESH.md §13 example — one line per missing entity
    /// with the deploy.yaml kind (`service`/`method`/`event_group`) and the
    /// unmet assertion.
    #[error("deploy.yaml for machine '{machine}' references SOME/IP entities that do not exist in\n\
             {config_path}:\n{}",
             .missing.iter().map(|m| format!("  - {m}"))
                 .collect::<Vec<_>>().join("\n"))]
    UnresolvedNames {
        machine: String,
        config_path: String,
        missing: Vec<UnresolvedName>,
    },

    /// A binding uses the `event_group:` sugar but the referenced event group
    /// in vsomeip.json contains more than one event. The current template
    /// models one event per binding; resolving a multi-event group would
    /// silently pick a single id. Rejected at the Rust level.
    #[error("machine '{machine}': binding '{target}' references event_group '{event_group}' \
             in '{config_path}', which contains {count} events. \
             Per-event fanout is not yet supported; declare a single-event group \
             or add a per-event binding.")]
    AmbiguousEventGroup {
        machine: String,
        target: String,
        config_path: String,
        event_group: String,
        count: usize,
    },

    /// A binding uses the `event_group:` sugar but the referenced event group
    /// in vsomeip.json contains no events. Building on that would emit an
    /// event_id of 0 and route nothing.
    #[error("machine '{machine}': binding '{target}' references event_group '{event_group}' \
             in '{config_path}', which has no events declared. Add the event id in vsomeip.json.")]
    EmptyEventGroup {
        machine: String,
        target: String,
        config_path: String,
        event_group: String,
    },

    /// A SOME/IP binding declared a name-based reference (e.g. `service: motor`)
    /// but the owning device did not declare `transports.someip.config:`.
    /// Without the config file path there is no way to resolve the name.
    #[error("machine '{machine}': binding '{target}' uses name-based SOME/IP references \
             but device '{device}' does not declare 'transports.someip.config:'. \
             Add the vsomeip.json path to the device's transports block.")]
    NamedReferenceWithoutConfig {
        machine: String,
        device: String,
        target: String,
    },

    /// A binding declares a reserved SOME/IP numeric-ID key (`service_id`,
    /// `method_id`, `event_group_id`, `event_id`, `getter_id`, `setter_id`,
    /// `instance_id`). These key names are reserved: SOME/IP numeric IDs
    /// come from the external vsomeip.json, not from deploy.yaml. The keys
    /// are rejected on every transport — they never had a meaning on
    /// non-SOME/IP transports, and on SOME/IP they are replaced by
    /// name-based references (`service:`, `events.*.method:`, …) that
    /// resolve against `transports.someip.config:`. See SCE_MESH.md §14.
    #[error("machine '{machine}': binding '{target}' (transport: {transport}) uses \
             reserved SOME/IP numeric-ID key(s) {fields:?}. deploy.yaml does not declare \
             numeric IDs directly — for SOME/IP bindings reference names against \
             `transports.someip.config:` (vsomeip.json); on other transports remove these keys.")]
    ReservedSomeipIdKeys {
        machine: String,
        target: String,
        transport: String,
        fields: Vec<&'static str>,
    },

    /// A binding whose `transport:` is not `someip` carries SOME/IP-only
    /// name-based fields (`service`, `method`, `event_group`, `getter`,
    /// `setter`). Catches misconfigured deploy.yaml at build time instead
    /// of silently ignoring the fields.
    #[error("machine '{machine}': binding '{target}' uses transport '{transport}' but \
             declares SOME/IP-only fields {fields:?}. Either change the transport to 'someip' \
             or remove the SOME/IP-specific fields.")]
    SomeipFieldOnNonSomeipTransport {
        machine: String,
        target: String,
        transport: String,
        fields: Vec<&'static str>,
    },

    /// A binding declared both flat per-binding fields (`method:`,
    /// `event_group:`, `getter:`, `setter:`) AND a per-event `events:` block.
    /// Rejected because the two are mutually exclusive and a reader would
    /// have to know a precedence rule that the spec does not define.
    #[error("machine '{machine}': binding '{target}' declares both flat fields ({flat_fields:?}) \
             and an 'events:' block. These are mutually exclusive — use 'events:' for per-event \
             mappings, or the flat fields for a single mapping shared by every event on this target.")]
    ConflictingEventSchema {
        machine: String,
        target: String,
        flat_fields: Vec<&'static str>,
    },

    /// A single per-event entry sets more than one field family (e.g. both
    /// `method:` and `event_group:`). Each SCXML event addresses exactly
    /// one SOME/IP resource kind — mixing families within one entry would
    /// silently pick a variant at codegen time. Rejected at resolution.
    #[error("machine '{machine}': binding '{target}' event '{event}' sets multiple \
             field kinds ({fields:?}). Each per-event entry must declare exactly one \
             of method / event_group / getter / setter.")]
    ConflictingEventFieldKinds {
        machine: String,
        target: String,
        event: String,
        fields: Vec<String>,
    },

    /// A per-event entry declares no field at all (`events.foo: {}`).
    /// Every entry must set exactly one of `method`/`event_group`/
    /// `getter`/`setter` — an empty entry contributes no mapping and
    /// would silently drop the event at codegen time. Rejected at
    /// resolution so the diagnostic points at the SCXML event.
    #[error("machine '{machine}': binding '{target}' event '{event}' declares no field. \
             Each per-event entry must set exactly one of method / event_group / getter / setter.")]
    EmptyEventEntry {
        machine: String,
        target: String,
        event: String,
    },

    // EventBindingUnused lives on TopologyError because detection requires
    // the SCXML send summary (an SCXML-stage input).
}

/// One unresolved name entry inside `ExternalConfigError::UnresolvedNames`.
#[derive(Debug, Clone)]
pub struct UnresolvedName {
    /// The deploy.yaml key kind: "service", "method", "event_group",
    /// "getter", "setter", or "application_name".
    pub kind: &'static str,
    /// The unresolved name as declared in deploy.yaml.
    pub name: String,
    /// Extra context for multi-level lookups (e.g. "in service \"motor\"").
    pub context: Option<String>,
}

impl std::fmt::Display for UnresolvedName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.context {
            Some(ctx) => write!(
                f,
                "{kind} \"{name}\" → no match {ctx}",
                kind = self.kind,
                name = self.name,
                ctx = ctx
            ),
            None => write!(
                f,
                "{kind} \"{name}\" → no match",
                kind = self.kind,
                name = self.name
            ),
        }
    }
}

// ── Stage 2: Topology resolution ─────────────────────────────

/// Errors from <send> target collection and deploy.yaml binding matching.
#[derive(Debug, thiserror::Error)]
pub enum TopologyError {
    /// SCXML <send> targets that have no matching deploy.yaml binding.
    #[error("unresolved send targets for machine '{machine}': {targets}. \
             Each <send target=\"...\"> in SCXML must have a corresponding \
             binding in deploy.yaml",
             targets = .targets.iter().map(|t| t.as_str()).collect::<Vec<_>>().join(", "))]
    UnresolvedTargets {
        machine: String,
        targets: Vec<super::target::TargetId>,
    },

    /// Machine name (from SCXML <scxml name="...">) not found in deploy.yaml.
    #[error("machine '{machine}' not found in deploy.yaml topology. \
             Available: {available}", available = if .available.is_empty() { "(none)".to_string() } else { .available.join(", ") })]
    MachineNotFound {
        machine: String,
        available: Vec<String>,
    },

    /// A <send target="#X"/> references a receiver machine not declared in
    /// deploy.yaml. Required so the event coverage analyzer can locate the
    /// receiver's SCXML source and read its <transition event="..."> set.
    #[error("machine '{sender}' sends to '{target}' but no machine '{receiver}' \
             is declared in deploy.yaml. Add the receiver under topology.*.machines \
             with its `source:` path.")]
    ReceiverNotDeclared {
        sender: String,
        target: super::target::TargetId,
        receiver: String,
    },

    /// The `source:` field uses an absolute path. Deploy descriptors must be
    /// portable across checkouts and build roots — absolute paths defeat that.
    #[error("machine '{machine}' has absolute source path '{path}'. Use a path \
             relative to the deploy.yaml file instead.")]
    AbsoluteSourcePath {
        machine: String,
        path: String,
    },

    /// Cannot read a receiver's SCXML source file during event coverage validation.
    #[error("cannot read receiver SCXML '{path}' (for machine '{machine}'): {source}. \
             Check the `source:` field in deploy.yaml for this machine.")]
    ReceiverSourceRead {
        machine: String,
        path: String,
        source: std::io::Error,
    },

    /// Cannot parse a receiver's SCXML source file.
    #[error("cannot parse receiver SCXML '{path}' (for machine '{machine}'): {reason}")]
    ReceiverSourceParse {
        machine: String,
        path: String,
        reason: String,
    },

    /// Send events that have no matching <transition event="..."> in the
    /// receiver machine. Detected at build time; otherwise these events
    /// would be silently dropped at runtime by the transport layer.
    #[error("event coverage violations in machine '{sender}':\n{}\nEach <send event=\"X\"> must have a matching <transition event=\"X\"> in the receiver. \
             Fix: add the missing transition in the receiver, or correct the event name in the sender.",
             .findings.iter().map(|f| format!("  - send target=\"{}\" event=\"{}\" has no matching transition in '{}'",
                 f.target, f.event, f.target.name()))
                 .collect::<Vec<_>>().join("\n"))]
    UncoveredEvents {
        sender: String,
        findings: Vec<super::topology::EventCoverageWarning>,
    },

    /// SCXML `<send>` uses a communication pattern that the bound transport
    /// does not support (SCE_MESH.md Section 8.2). Build error, not warning:
    /// the generated code would fail at runtime.
    #[error("pattern capability violations in machine '{sender}':\n{}\nEach communication pattern must be supported by the bound transport. \
             Fix: change the transport in deploy.yaml, or use a different event pattern.",
             .violations.iter().map(|v| format!("  - {v}"))
                 .collect::<Vec<_>>().join("\n"))]
    PatternCapabilityViolation {
        sender: String,
        violations: Vec<super::pattern::PatternViolation>,
    },

    /// A deploy.yaml binding is missing a field required by its transport.
    /// Detected at the Rust level (topology stage) so users get a clear
    /// deploy.yaml diagnostic instead of a deferred C++ `#error`.
    #[error("machine '{machine}': binding for '{target}' (transport: {transport}) \
             is missing required field '{field}'. \
             Add '{field}:' to the binding in deploy.yaml.")]
    MissingBindingField {
        machine: String,
        target: super::target::TargetId,
        transport: String,
        field: String,
    },

    /// A binding field has an invalid value (wrong type, out of range,
    /// violates a constraint like power-of-two). Reported from the Rust
    /// validation stage so the diagnostic points at deploy.yaml, not at
    /// a deferred C++ static_assert.
    #[error("machine '{machine}': binding '{target}' (transport: {transport}) \
             has invalid '{field}': {reason}")]
    InvalidBindingField {
        machine: String,
        target: super::target::TargetId,
        transport: String,
        field: String,
        reason: String,
    },

    /// A binding's `events:` table declares an entry for an SCXML event
    /// that the sender never `<send>`s to this target. Detecting this
    /// here (instead of at runtime where the event would silently route
    /// to no method_id) catches deploy.yaml typos at build time.
    #[error("machine '{machine}': binding '{target}' declares events.{event} in deploy.yaml, \
             but the SCXML model never sends '{event}' to this target. Remove the unused \
             entry, or correct the event name.")]
    EventBindingUnused {
        machine: String,
        target: super::target::TargetId,
        event: String,
    },

    /// A binding declares `ordering: required` on a transport whose
    /// broadcast semantics leave no per-(sender, receiver) sequence
    /// domain — the runtime `OrderingBuffer` cannot operate because a
    /// sender-stamped `sequence_no` is indistinguishable at each
    /// receiver on the bus (SCE_MESH.md §10.6.2). CAN is the only
    /// in-tree transport in this category today.
    #[error("machine '{machine}': binding for '{target}' (transport: {transport}) declares \
             `ordering: required`, but '{transport}' is a broadcast bus whose semantics do not \
             support per-(sender, receiver) sequence reconstruction (SCE Mesh §10.6.2). \
             Either change the transport to a point-to-point one (e.g. local, shm, custom_tcp, \
             someip, zenoh) or remove the `ordering: required` declaration from this binding.")]
    OrderingCannotBeGuaranteed {
        machine: String,
        target: super::target::TargetId,
        transport: String,
    },

    /// A binding declares a runtime pool (Zenoh `{name}` placeholders or
    /// SOME/IP `instance_from:`) but at least one `<invoke
    /// type="sce:mesh-rpc">` site targeting this binding does not
    /// supply the corresponding `<param>` name. Without a value at the
    /// invoke site there is nothing to substitute into the transport
    /// address, and the runtime would raise `error.invoke.<id>` with
    /// `RpcStatus::Unavailable` on every dispatch — a silently-broken
    /// deployment. Detecting at build time pinpoints the offending
    /// invoke site (state + invoke id) in a single diagnostic instead
    /// of dozens of runtime misfires.
    #[error("machine '{machine}': binding '{target}' declares a runtime pool that needs \
             <param> values {missing:?} at every using <invoke>, but invoke '{invoke_id}' \
             in state '{state}' does not supply {missing:?}. Add the missing <param>(s) \
             to that invoke, or drop the placeholder / `instance_from:` from the binding.")]
    PoolParamNameMissing {
        machine: String,
        target: super::target::TargetId,
        state: String,
        invoke_id: String,
        missing: Vec<String>,
    },

    /// A deploy.yaml `machines.<name>.subscriptions:` entry names a
    /// `source:` that has no matching binding in the machine's
    /// `bindings:` map. Without the binding, transport synthesis has
    /// no key expression / service identity to subscribe on, so the
    /// machine-lifetime subscribe would silently never reach the wire.
    /// Detected at topology time so the diagnostic points at the
    /// deploy.yaml entry rather than surfacing as a runtime no-op.
    ///
    /// SCE_MESH.md §13 machine-lifetime path: the `source:` attribute
    /// is the same target identifier the machine's `bindings:` map is
    /// keyed on — the two must agree for the subscribe to resolve.
    #[error("machine '{machine}': subscription source '{source_target}' has no matching binding. \
             Available: {available}. Add the source to machines.{machine}.bindings:, \
             or drop the subscription from machines.{machine}.subscriptions:.",
             available = if .available.is_empty() {
                 "(none)".to_string()
             } else {
                 .available.iter().map(|t| t.as_str()).collect::<Vec<_>>().join(", ")
             })]
    SubscriptionSourceUnbound {
        machine: String,
        source_target: String,
        available: Vec<super::target::TargetId>,
    },

    /// A deploy.yaml `machines.<name>.subscriptions:` entry names a
    /// `source:` whose binding transport does not support the
    /// machine-lifetime synthesis path. The init-time subscribe
    /// envelope synthesised from deploy.yaml alone would drop at the
    /// transport's send path — not because pub/sub is missing, but
    /// because the send path needs per-event external metadata
    /// (e.g. SOME/IP event_group_id) that synthesis cannot supply.
    /// Detected at topology time so the diagnostic fires at the
    /// build rather than surfacing as a never-delivered subscription
    /// at runtime.
    ///
    /// SSoT: `super::transport::TransportDescriptor::
    /// supports_machine_lifetime_subscribe`. Currently `true` only
    /// for `zenoh`; SOME/IP support is tracked under
    /// `mesh_someip_sd_gaps_roadmap.md`.
    #[error("machine '{machine}': subscription on source '{source_target}' for event \
             '{event}' uses transport '{transport}', which does not support the \
             machine-lifetime subscription path in this build. Move the binding to a \
             transport that supports it (e.g. 'zenoh') or drop the subscription from \
             machines.{machine}.subscriptions:.")]
    MachineLifetimeSubscriptionUnsupported {
        machine: String,
        source_target: super::target::TargetId,
        event: String,
        transport: String,
    },
}

// ── Stage 3: Template rendering ──────────────────────────────

/// Errors from transport template selection and rendering.
#[derive(Debug, thiserror::Error)]
pub enum CodegenError {
    /// Mesh codegen is not yet implemented for this language.
    #[error("mesh codegen not yet supported for language '{0}'")]
    UnsupportedLanguage(String),

    /// Transport type is not yet implemented.
    #[error("transport '{transport}' not yet supported (target '{target}')")]
    UnsupportedTransport {
        transport: String,
        target: super::target::TargetId,
    },

    /// Cannot read the mesh Jinja2 template file.
    #[error("cannot read mesh template '{path}': {source}")]
    TemplateRead {
        path: String,
        source: std::io::Error,
    },

    /// Jinja2 template rendering failure.
    #[error("mesh template render error: {0}")]
    TemplateRender(String),

    /// Two distinct SCXML event names on the same target map to the same
    /// C++ identifier suffix (e.g. `service.request.x` and
    /// `service-request-x` both collapse to `SERVICE_REQUEST_X`). Without
    /// this check the collision would only surface as a C++ redefinition
    /// error on the downstream compiler — an unhelpful diagnostic location.
    #[error("target '{target}': SCXML events {events:?} both map to the same \
             C++ constant suffix '{suffix}'. Rename one of the events (or use \
             a per-event explicit mapping) so generated constants are unique.")]
    EventNameCollision {
        target: super::target::TargetId,
        suffix: String,
        events: Vec<String>,
    },

    /// A machine combines a multi-instance SOME/IP server pool
    /// (`server.instances: [N > 1]`) with an outbound RPC client
    /// path whose correlation state lives in a router-scoped table
    /// (SCE Mesh §9.5 + §10.9 + §14.4). Two kinds of RPC client are
    /// covered:
    ///
    /// * [`RpcClientKind::MeshRpc`] — any `<invoke type="sce:mesh-rpc">`
    ///   site on the machine. `invoke_correlation_` and
    ///   `active_invokes_` are router-scoped; hosting multiple
    ///   SCXML sessions would alias their invoke_id tables.
    /// * [`RpcClientKind::SomeipRpcRequest`] — any SOME/IP target
    ///   with an outbound `<send>` RpcRequest pattern. `pending_rpcs_`
    ///   is the router-scoped correlation table that maps
    ///   `correlation_id → reply-event-name`, and the generated
    ///   client-side receive handler dispatches replies to
    ///   `sessions_[0]` because there is no per-session identity
    ///   threaded through the correlation key.
    ///
    /// Two equally-valid repairs (drop the RPC client site or
    /// reduce `server.instances:` to a single entry) — the
    /// diagnostic keeps both arms so the author picks. Split
    /// across deployments is fine: the rejection is per-router,
    /// not per-deployment.
    #[error("machine '{machine}': SOME/IP server pool (`server.instances: [...]` with more than \
             one entry) cannot be combined with {kind} in the same router. Router-scoped \
             correlation tables (`invoke_correlation_` / `active_invokes_` / `pending_rpcs_`) \
             cannot safely alias across hosted sessions. Either remove the RPC client site(s) \
             from this machine or reduce `server.instances:` to a single instance. See \
             SCE_MESH.md §14.4.")]
    PoolWithRpcClientUnsupported {
        machine: String,
        kind: RpcClientKind,
    },
}

/// Which router-scoped correlation surface drove a
/// [`CodegenError::PoolWithRpcClientUnsupported`] rejection. The
/// `Display` rendering is consumed verbatim by the `#[error(...)]`
/// format string so the diagnostic message names the exact shape
/// the deployment picked up (authors fix one or the other, not a
/// generic category).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RpcClientKind {
    /// `<invoke type="sce:mesh-rpc">` outbound site — consumes
    /// `invoke_correlation_` + `active_invokes_`.
    MeshRpc,
    /// SOME/IP `<send>` RpcRequest outbound event — consumes
    /// `pending_rpcs_` with a `sessions_[0]`-hard-coded reply
    /// dispatch path.
    SomeipRpcRequest,
}

impl std::fmt::Display for RpcClientKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MeshRpc => write!(f, "`<invoke type=\"sce:mesh-rpc\">`"),
            Self::SomeipRpcRequest => write!(f, "SOME/IP `<send>` RpcRequest"),
        }
    }
}

/// CLI exit code by error category.
impl MeshError {
    pub fn exit_code(&self) -> i32 {
        match self {
            MeshError::Deploy(_) => 10,
            MeshError::External(_) => 14,
            MeshError::Topology(_) => 11,
            MeshError::Codegen(_) => 12,
            MeshError::Io { .. } => 13,
        }
    }
}

// ── Machine-readable diagnostic mapping ──────────────────────────
//
// Kept in this module (not diagnostic.rs) so variant additions to
// MeshError and its mapping stay next to each other. The trait impl
// lives in `crate::forge::diagnostic`; we only supply the per-variant
// `(code, stage, key_fragments)` triple.

use crate::forge::diagnostic::{
    Diagnostic, DiagnosticCode, DiagnosticPayload, Fix, SingleDiagnostic, Stage, ToDiagnostics,
};

fn deploy_fields(e: &DeployError) -> DiagnosticPayload {
    match e {
        DeployError::ReadFile { path, .. } => DiagnosticPayload {
            code: DiagnosticCode::MeshDeployRead,
            stage: Stage::MeshDeploy,
            actual: Some(path.clone()),
            expected: None,
            fix: None,
            key_fragments: vec![path.clone()],
        },
        DeployError::Yaml(reason) => DiagnosticPayload {
            code: DiagnosticCode::MeshDeployParse,
            stage: Stage::MeshDeploy,
            actual: None,
            expected: None,
            fix: None,
            key_fragments: vec![reason.clone()],
        },
        DeployError::UnsupportedVersion { found, supported } => DiagnosticPayload {
            code: DiagnosticCode::MeshDeployUnsupportedVersion,
            stage: Stage::MeshDeploy,
            actual: Some(found.clone()),
            // Candidate list rides `fix`; `expected` stays None so the
            // two fields never duplicate each other (contract §3.2).
            expected: None,
            fix: Some(Fix::ReplaceOneOf {
                candidates: supported.iter().map(|s| (*s).to_string()).collect(),
            }),
            key_fragments: vec![found.clone()],
        },
        DeployError::DuplicateMachine { machine, devices } => DiagnosticPayload {
            code: DiagnosticCode::MeshDeployDuplicateMachine,
            stage: Stage::MeshDeploy,
            actual: Some(machine.clone()),
            expected: None,
            fix: None,
            key_fragments: {
                let mut k = vec![machine.clone()];
                k.extend(devices.iter().cloned());
                k
            },
        },
        DeployError::InvalidOrderingTimings { machine, reason } => DiagnosticPayload {
            code: DiagnosticCode::MeshDeployInvalidOrderingTimings,
            stage: Stage::MeshDeploy,
            actual: Some(machine.clone()),
            // No mechanical repair: the author chooses between fixing
            // the explicit values and dropping back to the defaults
            // (omit the section). Same shape as
            // `MeshTopologyOrderingCannotBeGuaranteed` — author intent
            // decides which side gives way.
            expected: None,
            fix: None,
            key_fragments: vec![machine.clone(), reason.clone()],
        },
        DeployError::InvalidLiveliness { machine, reason } => DiagnosticPayload {
            code: DiagnosticCode::MeshDeployInvalidLiveliness,
            stage: Stage::MeshDeploy,
            actual: Some(machine.clone()),
            // Same author-intent shape as InvalidOrderingTimings:
            // either fix the explicit value or omit the section.
            expected: None,
            fix: None,
            key_fragments: vec![machine.clone(), reason.clone()],
        },
        DeployError::InvalidServerQueryTimeout { machine, reason } => DiagnosticPayload {
            code: DiagnosticCode::MeshDeployInvalidServerQueryTimeout,
            stage: Stage::MeshDeploy,
            actual: Some(machine.clone()),
            // Same author-intent shape as InvalidLiveliness: either fix
            // the explicit value or omit the knob.
            expected: None,
            fix: None,
            key_fragments: vec![machine.clone(), reason.clone()],
        },
        DeployError::InvalidOutboundBuffer { machine, reason } => DiagnosticPayload {
            code: DiagnosticCode::MeshDeployInvalidOutboundBuffer,
            stage: Stage::MeshDeploy,
            actual: Some(machine.clone()),
            // Same author-intent shape as InvalidServerQueryTimeout:
            // either fix the explicit value or omit the section.
            expected: None,
            fix: None,
            key_fragments: vec![machine.clone(), reason.clone()],
        },
        DeployError::DiscoveryNotSupported { content_kind } => DiagnosticPayload {
            code: DiagnosticCode::MeshDeployDiscoveryNotSupported,
            stage: Stage::MeshDeploy,
            // No per-machine target: `discovery:` is a top-level deploy
            // key. `actual` surfaces the rejected content summary so
            // the CLI wire format points the author at what was seen.
            actual: Some(content_kind.clone()),
            expected: None,
            fix: None,
            // fnv1a keying on the content summary so two differently
            // shaped rejections hash distinct.
            key_fragments: vec![content_kind.clone()],
        },
        DeployError::PoolNotSupportedByTransport {
            machine,
            binding,
            transport,
        } => DiagnosticPayload {
            code: DiagnosticCode::MeshDeployPoolNotSupportedByTransport,
            stage: Stage::MeshDeploy,
            actual: Some(machine.clone()),
            expected: None,
            fix: None,
            key_fragments: vec![machine.clone(), binding.clone(), transport.clone()],
        },
        DeployError::PoolMissingInstanceList { machine, binding } => DiagnosticPayload {
            code: DiagnosticCode::MeshDeployPoolMissingInstanceList,
            stage: Stage::MeshDeploy,
            actual: Some(machine.clone()),
            expected: None,
            fix: None,
            key_fragments: vec![machine.clone(), binding.clone()],
        },
        DeployError::PoolEmptyInstanceList { machine, binding } => DiagnosticPayload {
            code: DiagnosticCode::MeshDeployPoolEmptyInstanceList,
            stage: Stage::MeshDeploy,
            actual: Some(machine.clone()),
            expected: None,
            fix: None,
            key_fragments: vec![machine.clone(), binding.clone()],
        },
        DeployError::PoolInvalidPlaceholder {
            machine,
            binding,
            reason,
        } => DiagnosticPayload {
            code: DiagnosticCode::MeshDeployPoolInvalidPlaceholder,
            stage: Stage::MeshDeploy,
            actual: Some(machine.clone()),
            expected: None,
            fix: None,
            key_fragments: vec![machine.clone(), binding.clone(), reason.clone()],
        },
        DeployError::ServerPoolNotSupported { machine, transport } => DiagnosticPayload {
            code: DiagnosticCode::MeshDeployServerPoolNotSupported,
            stage: Stage::MeshDeploy,
            actual: Some(machine.clone()),
            // The deterministic repair remains "remove `instances:`";
            // the alternative ("switch transport" / "run N processes")
            // is deployment-topology advice, not a field-level edit,
            // so it stays in the error prose. `key_fragments` carries
            // the transport so duplicate-deploy aggregation groups
            // rejections by (machine, transport).
            expected: None,
            fix: Some(Fix::RemoveFields {
                location: format!("topology.*.machines.{machine}.server"),
                fields: vec!["instances".to_string()],
            }),
            key_fragments: vec![machine.clone(), transport.clone()],
        },
        DeployError::ScxmlInvokeTargetConflict {
            machine,
            inbound_peers,
            local_invoker,
            local_src,
        } => DiagnosticPayload {
            code: DiagnosticCode::MeshDeployScxmlInvokeTargetConflict,
            stage: Stage::MeshDeploy,
            actual: Some(machine.clone()),
            // Two equally-valid repairs — flip '{local_invoker}' to the
            // `#<machine>` shape, OR remove '{machine}' from deploy.yaml
            // topology. Author intent decides; no mechanical single-edit
            // Fix applies. Same shape as PartitionMultiDevice above.
            expected: None,
            fix: None,
            key_fragments: {
                let mut k = vec![machine.clone(), local_invoker.clone(), local_src.clone()];
                k.extend(inbound_peers.iter().cloned());
                k
            },
        },
        DeployError::PartitionDuplicateName { name } => DiagnosticPayload {
            code: DiagnosticCode::MeshDeployPartitionDuplicateName,
            stage: Stage::MeshDeploy,
            actual: Some(name.clone()),
            expected: None,
            // The author decides which of the two aliased entries to
            // rename; no mechanical repair exists.
            fix: None,
            key_fragments: vec![name.clone()],
        },
        DeployError::PartitionMultiDevice { partition, devices } => DiagnosticPayload {
            code: DiagnosticCode::MeshDeployPartitionMultiDevice,
            stage: Stage::MeshDeploy,
            actual: Some(partition.clone()),
            expected: None,
            // Two equally-valid repairs (split the partition per device
            // OR narrow `machines:`); author intent decides.
            fix: None,
            key_fragments: {
                let mut k = vec![partition.clone()];
                k.extend(devices.iter().cloned());
                k
            },
        },
        DeployError::PartitionUnitDuplicate { unit, partitions } => DiagnosticPayload {
            code: DiagnosticCode::MeshDeployPartitionUnitDuplicate,
            stage: Stage::MeshDeploy,
            actual: Some(unit.clone()),
            expected: None,
            // Author picks which partition keeps the unit; the other
            // entry must be removed by hand.
            fix: None,
            key_fragments: {
                let mut k = vec![unit.clone()];
                k.extend(partitions.iter().cloned());
                k
            },
        },
        DeployError::PartitionMachineNotListed { partition, machine } => DiagnosticPayload {
            code: DiagnosticCode::MeshDeployPartitionMachineNotListed,
            stage: Stage::MeshDeploy,
            actual: Some(machine.clone()),
            expected: None,
            // Two equally-valid repairs (add the machine to the
            // partition's `machines:` list OR remove the stray
            // `contains:` entry); author intent decides.
            fix: None,
            key_fragments: vec![partition.clone(), machine.clone()],
        },
        DeployError::PartitionEmpty { partition } => DiagnosticPayload {
            code: DiagnosticCode::MeshDeployPartitionEmpty,
            stage: Stage::MeshDeploy,
            actual: Some(partition.clone()),
            expected: None,
            fix: None,
            key_fragments: vec![partition.clone()],
        },
        DeployError::PartitionSynthInfixCollision { machine } => DiagnosticPayload {
            code: DiagnosticCode::MeshDeployPartitionSynthInfixCollision,
            stage: Stage::MeshDeploy,
            actual: Some(machine.clone()),
            expected: None,
            // The only mechanical repair is "rename the machine", which
            // the author must do with full knowledge of downstream
            // name references — not a field-level edit sce-build can
            // pre-compute.
            fix: None,
            key_fragments: vec![machine.clone()],
        },
        DeployError::PartitionUncoveredUnit { machine, units } => DiagnosticPayload {
            code: DiagnosticCode::MeshDeployPartitionUncoveredUnit,
            stage: Stage::MeshDeploy,
            actual: Some(machine.clone()),
            expected: None,
            // Repair is semantic: the author picks which partition
            // (existing or <machine>_default) each unit belongs in.
            fix: None,
            key_fragments: {
                let mut k = vec![machine.clone()];
                k.extend(units.iter().cloned());
                k
            },
        },
        DeployError::PartitionPartialCoverageRequiresDefault { machine, missing } => {
            DiagnosticPayload {
                code: DiagnosticCode::MeshDeployPartitionPartialCoverageRequiresDefault,
                stage: Stage::MeshDeploy,
                actual: Some(machine.clone()),
                expected: None,
                // Two equally-valid repairs (extend an existing
                // partition or add <machine>_default); the spec
                // prose carries both options, so no mechanical fix.
                fix: None,
                key_fragments: {
                    let mut k = vec![machine.clone()];
                    k.extend(missing.iter().cloned());
                    k
                },
            }
        }
        DeployError::PartitionPoolMachine { machine, partition } => DiagnosticPayload {
            code: DiagnosticCode::MeshDeployPartitionPoolMachine,
            stage: Stage::MeshDeploy,
            actual: Some(machine.clone()),
            expected: None,
            // Two equally-valid repairs (remove the machine from the
            // partition's `machines:` list OR drop `server.instances:`);
            // the spec prose names both and author intent decides.
            fix: None,
            key_fragments: vec![machine.clone(), partition.clone()],
        },
        DeployError::PartitionTransportBindingUnsupported {
            partition,
            transport,
            failure,
        } => DiagnosticPayload {
            code: DiagnosticCode::MeshDeployPartitionTransportBindingUnsupported,
            stage: Stage::MeshDeploy,
            actual: Some(partition.clone()),
            expected: None,
            // Two equally-valid repairs: switch to a supported transport
            // OR drop the key entirely to accept §14 L2730 defaults.
            // Author intent decides — no mechanical one.
            fix: None,
            // key_fragments feed the fnv1a id hash; formatting `failure`
            // via Display yields the same bytes the pre-typed
            // `reason: String` produced, preserving the diagnostic id
            // across this refactor (see
            // `forge/diagnostic.rs::mesh_golden_entries`).
            key_fragments: vec![partition.clone(), transport.clone(), failure.to_string()],
        },
        DeployError::ScxmlInvokeCrossDeviceTransport {
            parent,
            peer,
            parent_device,
            peer_device,
            failure,
        } => DiagnosticPayload {
            code: DiagnosticCode::MeshDeployScxmlInvokeCrossDeviceTransport,
            stage: Stage::MeshDeploy,
            // `{parent}/{peer}` names the per-invoke pair that triggered
            // the rejection — matches the shape `ScxmlInvokeTargetConflict`
            // uses so downstream UIs can render both §9.6 diagnostics the
            // same way.
            actual: Some(format!("{parent}/{peer}")),
            expected: None,
            // Three equally-valid repairs depending on `failure`: add the
            // binding, pick a different transport, or wait for the
            // Session 2 C++ wire-14/20 dispatch to land. Author intent
            // decides — no mechanical fix.
            fix: None,
            // All discriminating data flows through `key_fragments` so
            // the fnv1a id is stable across the three failure shapes.
            key_fragments: vec![
                parent.clone(),
                peer.clone(),
                parent_device.clone(),
                peer_device.clone(),
                failure.to_string(),
            ],
        },
        DeployError::SomeipScxmlInvokeServiceIdOverflow {
            participant_count,
            ceiling,
        } => DiagnosticPayload {
            code: DiagnosticCode::MeshDeploySomeipScxmlInvokeServiceIdOverflow,
            stage: Stage::MeshDeploy,
            // Participant count is the violation site. Hex would be
            // misleading (it's not an ID); raw decimal pinpoints the
            // operator's "how many did I declare" question.
            actual: Some(participant_count.to_string()),
            expected: None,
            // No mechanical fix: operator either reduces the count or
            // waits on multi-domain landing. Same shape as
            // MeshDeploySomeipScxmlInvokeServiceIdCollision.
            fix: None,
            key_fragments: vec![participant_count.to_string(), ceiling.to_string()],
        },
        DeployError::SomeipScxmlInvokeServiceIdPinOutOfRange {
            machine,
            pinned_id,
            range_lo,
            range_hi,
        } => DiagnosticPayload {
            code: DiagnosticCode::MeshDeploySomeipScxmlInvokeServiceIdPinOutOfRange,
            stage: Stage::MeshDeploy,
            // Pinned ID names the violation site at the routing-layer
            // key. Hex format mirrors SCXML_INVOKE_SERVICE_BASE.
            actual: Some(format!("{pinned_id:#06x}")),
            expected: None,
            // No mechanical fix variant fits — operator picks any value
            // inside the sub-range, no closed candidate set.
            fix: None,
            key_fragments: vec![
                machine.clone(),
                format!("{pinned_id:#06x}"),
                format!("{range_lo:#06x}"),
                format!("{range_hi:#06x}"),
            ],
        },
        DeployError::SomeipScxmlInvokeServiceIdPinCollision {
            machines,
            pinned_id,
        } => DiagnosticPayload {
            code: DiagnosticCode::MeshDeploySomeipScxmlInvokeServiceIdPinCollision,
            stage: Stage::MeshDeploy,
            actual: Some(format!("{pinned_id:#06x}")),
            expected: None,
            // Same "rename, no closed candidates" shape as the legacy
            // MeshDeploySomeipScxmlInvokeServiceIdCollision diagnostic
            // — operator picks any non-colliding value.
            fix: None,
            key_fragments: {
                let mut k = vec![format!("{pinned_id:#06x}")];
                k.extend(machines.iter().cloned());
                k
            },
        },
        DeployError::SomeipLivenessServiceIdOverflow {
            participant_count,
            ceiling,
        } => DiagnosticPayload {
            code: DiagnosticCode::MeshDeploySomeipLivenessServiceIdOverflow,
            stage: Stage::MeshDeploy,
            actual: Some(participant_count.to_string()),
            expected: None,
            fix: None,
            key_fragments: vec![participant_count.to_string(), ceiling.to_string()],
        },
        DeployError::SomeipLivenessServiceIdPinOutOfRange {
            partition_key,
            pinned_id,
            range_lo,
            range_hi,
        } => DiagnosticPayload {
            code: DiagnosticCode::MeshDeploySomeipLivenessServiceIdPinOutOfRange,
            stage: Stage::MeshDeploy,
            actual: Some(format!("{pinned_id:#06x}")),
            expected: None,
            fix: None,
            key_fragments: vec![
                partition_key.clone(),
                format!("{pinned_id:#06x}"),
                format!("{range_lo:#06x}"),
                format!("{range_hi:#06x}"),
            ],
        },
        DeployError::SomeipLivenessServiceIdPinCollision {
            partition_keys,
            pinned_id,
        } => DiagnosticPayload {
            code: DiagnosticCode::MeshDeploySomeipLivenessServiceIdPinCollision,
            stage: Stage::MeshDeploy,
            actual: Some(format!("{pinned_id:#06x}")),
            expected: None,
            fix: None,
            key_fragments: {
                let mut k = vec![format!("{pinned_id:#06x}")];
                k.extend(partition_keys.iter().cloned());
                k
            },
        },
        DeployError::PartitionBarrierTimeoutInvalid {
            partition,
            value,
            reason,
        } => DiagnosticPayload {
            code: DiagnosticCode::MeshDeployPartitionBarrierTimeoutInvalid,
            stage: Stage::MeshDeploy,
            actual: Some(partition.clone()),
            expected: None,
            // Same author-intent shape as the `Invalid*Timings` family:
            // either fix the value or omit the knob to accept the
            // W3C normative default (infinity).
            fix: None,
            key_fragments: vec![partition.clone(), value.to_string(), reason.clone()],
        },
        DeployError::PartitionParallelRootUndesignated {
            machine,
            parallel,
            hosting_partitions,
        } => DiagnosticPayload {
            code: DiagnosticCode::MeshPartitionParallelRootUndesignated,
            stage: Stage::MeshDeploy,
            actual: Some(format!("{machine}/{parallel}")),
            expected: None,
            fix: None,
            key_fragments: {
                let mut frags = vec![machine.clone(), parallel.clone()];
                frags.extend(hosting_partitions.iter().cloned());
                frags
            },
        },
        DeployError::PartitionParallelRootAmbiguous {
            machine,
            parallel,
            claiming_partitions,
        } => DiagnosticPayload {
            code: DiagnosticCode::MeshPartitionParallelRootAmbiguous,
            stage: Stage::MeshDeploy,
            actual: Some(format!("{machine}/{parallel}")),
            expected: None,
            fix: None,
            key_fragments: {
                let mut frags = vec![machine.clone(), parallel.clone()];
                frags.extend(claiming_partitions.iter().cloned());
                frags
            },
        },
        DeployError::PartitionParallelRootNotInMachines {
            partition,
            claimed_machine,
            partition_machines,
        } => DiagnosticPayload {
            code: DiagnosticCode::MeshPartitionParallelRootNotInMachines,
            stage: Stage::MeshDeploy,
            actual: Some(partition.clone()),
            expected: None,
            fix: None,
            key_fragments: {
                let mut frags = vec![partition.clone(), claimed_machine.clone()];
                frags.extend(partition_machines.iter().cloned());
                frags
            },
        },
        DeployError::PartitionParallelRootNonHost {
            partition,
            machine,
            parallel,
        } => DiagnosticPayload {
            code: DiagnosticCode::MeshPartitionParallelRootNonHost,
            stage: Stage::MeshDeploy,
            actual: Some(partition.clone()),
            expected: None,
            fix: None,
            key_fragments: vec![partition.clone(), machine.clone(), parallel.clone()],
        },
        DeployError::PartitionBarrierTimeoutWithoutRoot { partition, value } => {
            DiagnosticPayload {
                code: DiagnosticCode::MeshPartitionBarrierTimeoutWithoutRoot,
                stage: Stage::MeshDeploy,
                actual: Some(partition.clone()),
                expected: None,
                fix: None,
                key_fragments: vec![partition.clone(), value.to_string()],
            }
        }
        DeployError::PartitionWire21CustomTcpUnimplemented {
            partition,
            machine,
            parallel,
        } => DiagnosticPayload {
            code: DiagnosticCode::MeshPartitionWire21CustomTcpUnimplemented,
            stage: Stage::MeshDeploy,
            actual: Some("custom_tcp".to_string()),
            // `expected` carries the supported binding so authors can
            // copy-paste the repair without re-reading the message;
            // mirrors the `PartitionTransportBindingUnsupported` shape.
            expected: Some(vec!["shm".to_string()]),
            fix: None,
            key_fragments: vec![partition.clone(), machine.clone(), parallel.clone()],
        },
        DeployError::DistributabilityR1SharedWrite {
            machine,
            parallel,
            location,
            regions,
        } => DiagnosticPayload {
            code: DiagnosticCode::MeshDistributabilityR1SharedWrite,
            stage: Stage::MeshDeploy,
            actual: Some(location.clone()),
            expected: None,
            fix: None,
            key_fragments: {
                let mut k = vec![machine.clone(), parallel.clone(), location.clone()];
                k.extend(regions.iter().cloned());
                k
            },
        },
        DeployError::DistributabilityR2CrossRegionTransition {
            machine,
            parallel,
            regions,
        } => DiagnosticPayload {
            code: DiagnosticCode::MeshDistributabilityR2CrossRegionTransition,
            stage: Stage::MeshDeploy,
            actual: None,
            expected: None,
            fix: None,
            key_fragments: {
                let mut k = vec![machine.clone(), parallel.clone()];
                k.extend(regions.iter().cloned());
                k
            },
        },
    }
}

fn external_fields(e: &ExternalConfigError) -> DiagnosticPayload {
    match e {
        ExternalConfigError::Read { path, .. } => DiagnosticPayload {
            code: DiagnosticCode::MeshExternalRead,
            stage: Stage::MeshExternal,
            actual: Some(path.clone()),
            expected: None,
            fix: None,
            key_fragments: vec![path.clone()],
        },
        ExternalConfigError::Parse { path, reason } => DiagnosticPayload {
            code: DiagnosticCode::MeshExternalParse,
            stage: Stage::MeshExternal,
            actual: None,
            expected: None,
            fix: None,
            key_fragments: vec![path.clone(), reason.clone()],
        },
        ExternalConfigError::UnresolvedNames { machine, config_path, missing } => DiagnosticPayload {
            code: DiagnosticCode::MeshExternalUnresolvedNames,
            stage: Stage::MeshExternal,
            actual: None,
            expected: None,
            fix: None,
            key_fragments: {
                let mut k = vec![machine.clone(), config_path.clone()];
                k.extend(missing.iter().map(|m| format!("{}:{}", m.kind, m.name)));
                k
            },
        },
        ExternalConfigError::AmbiguousEventGroup { machine, target, event_group, count, .. } => DiagnosticPayload {
            code: DiagnosticCode::MeshExternalAmbiguousEventGroup,
            stage: Stage::MeshExternal,
            actual: Some(count.to_string()),
            // Cardinality metadata: "exactly 1 match expected, got N".
            // The `1` is not a substitution candidate for `count` —
            // it describes the rule. No deterministic repair exists
            // (the author must re-author the event_group mapping), so
            // `fix` stays None and `expected` carries pure metadata,
            // which is exactly what the non-overlap contract allows.
            expected: Some(vec!["1".to_string()]),
            fix: None,
            key_fragments: vec![machine.clone(), target.clone(), event_group.clone()],
        },
        ExternalConfigError::EmptyEventGroup { machine, target, event_group, .. } => DiagnosticPayload {
            code: DiagnosticCode::MeshExternalEmptyEventGroup,
            stage: Stage::MeshExternal,
            actual: None,
            expected: None,
            fix: None,
            key_fragments: vec![machine.clone(), target.clone(), event_group.clone()],
        },
        ExternalConfigError::NamedReferenceWithoutConfig { machine, device, target } => DiagnosticPayload {
            code: DiagnosticCode::MeshExternalNamedReferenceWithoutConfig,
            stage: Stage::MeshExternal,
            actual: None,
            expected: None,
            fix: None,
            key_fragments: vec![machine.clone(), device.clone(), target.clone()],
        },
        ExternalConfigError::ReservedSomeipIdKeys { machine, target, transport, fields } => DiagnosticPayload {
            code: DiagnosticCode::MeshExternalReservedSomeipIdKeys,
            stage: Stage::MeshExternal,
            actual: None,
            expected: None,
            // Fully deterministic repair: the reserved keys were
            // listed by the producer, and the fix is always "remove
            // them" — never "rename" or "replace". The dotted path
            // names the binding precisely so agents apply without
            // re-parsing the error message.
            fix: Some(Fix::RemoveFields {
                location: format!("machines.{machine}.bindings.{target}"),
                fields: fields.iter().map(|f| (*f).to_string()).collect(),
            }),
            key_fragments: {
                let mut k = vec![machine.clone(), target.clone(), transport.clone()];
                k.extend(fields.iter().map(|f| (*f).to_string()));
                k
            },
        },
        ExternalConfigError::SomeipFieldOnNonSomeipTransport { machine, target, transport, fields } => DiagnosticPayload {
            code: DiagnosticCode::MeshExternalSomeipFieldOnNonSomeipTransport,
            stage: Stage::MeshExternal,
            actual: Some(transport.clone()),
            // Single deterministic answer: the offending fields are
            // SOME/IP-specific, so the only repair that preserves them
            // is to switch the binding's transport to `someip`.
            expected: None,
            fix: Some(Fix::ReplaceWith { to: "someip".to_string() }),
            key_fragments: {
                let mut k = vec![machine.clone(), target.clone(), transport.clone()];
                k.extend(fields.iter().map(|f| (*f).to_string()));
                k
            },
        },
        ExternalConfigError::ConflictingEventSchema { machine, target, flat_fields } => DiagnosticPayload {
            code: DiagnosticCode::MeshExternalConflictingEventSchema,
            stage: Stage::MeshExternal,
            actual: None,
            expected: None,
            fix: None,
            key_fragments: {
                let mut k = vec![machine.clone(), target.clone()];
                k.extend(flat_fields.iter().map(|f| (*f).to_string()));
                k
            },
        },
        ExternalConfigError::ConflictingEventFieldKinds { machine, target, event, fields } => DiagnosticPayload {
            code: DiagnosticCode::MeshExternalConflictingEventFieldKinds,
            stage: Stage::MeshExternal,
            actual: None,
            expected: None,
            fix: None,
            key_fragments: {
                let mut k = vec![machine.clone(), target.clone(), event.clone()];
                k.extend(fields.iter().cloned());
                k
            },
        },
        ExternalConfigError::EmptyEventEntry { machine, target, event } => DiagnosticPayload {
            code: DiagnosticCode::MeshExternalEmptyEventEntry,
            stage: Stage::MeshExternal,
            actual: None,
            expected: None,
            fix: None,
            key_fragments: vec![machine.clone(), target.clone(), event.clone()],
        },
    }
}

fn topology_fields(e: &TopologyError) -> DiagnosticPayload {
    match e {
        TopologyError::UnresolvedTargets { machine, targets } => DiagnosticPayload {
            code: DiagnosticCode::MeshTopologyUnresolvedTargets,
            stage: Stage::MeshTopology,
            actual: None,
            expected: None,
            fix: None,
            key_fragments: {
                let mut k = vec![machine.clone()];
                k.extend(targets.iter().map(|t| t.as_str().to_string()));
                k
            },
        },
        TopologyError::MachineNotFound { machine, available } => DiagnosticPayload {
            code: DiagnosticCode::MeshTopologyMachineNotFound,
            stage: Stage::MeshTopology,
            actual: Some(machine.clone()),
            expected: None,
            fix: Some(Fix::ReplaceOneOf {
                candidates: available.clone(),
            }),
            key_fragments: vec![machine.clone()],
        },
        TopologyError::ReceiverNotDeclared { sender, target, receiver } => DiagnosticPayload {
            code: DiagnosticCode::MeshTopologyReceiverNotDeclared,
            stage: Stage::MeshTopology,
            actual: Some(receiver.clone()),
            expected: None,
            fix: None,
            key_fragments: vec![sender.clone(), target.as_str().to_string(), receiver.clone()],
        },
        TopologyError::AbsoluteSourcePath { machine, path } => DiagnosticPayload {
            code: DiagnosticCode::MeshTopologyAbsoluteSourcePath,
            stage: Stage::MeshTopology,
            actual: Some(path.clone()),
            expected: None,
            fix: None,
            key_fragments: vec![machine.clone(), path.clone()],
        },
        TopologyError::ReceiverSourceRead { machine, path, .. } => DiagnosticPayload {
            code: DiagnosticCode::MeshTopologyReceiverSourceRead,
            stage: Stage::MeshTopology,
            actual: Some(path.clone()),
            expected: None,
            fix: None,
            key_fragments: vec![machine.clone(), path.clone()],
        },
        TopologyError::ReceiverSourceParse { machine, path, reason } => DiagnosticPayload {
            code: DiagnosticCode::MeshTopologyReceiverSourceParse,
            stage: Stage::MeshTopology,
            actual: None,
            expected: None,
            fix: None,
            key_fragments: vec![machine.clone(), path.clone(), reason.clone()],
        },
        TopologyError::UncoveredEvents { sender, findings } => DiagnosticPayload {
            code: DiagnosticCode::MeshTopologyUncoveredEvents,
            stage: Stage::MeshTopology,
            actual: None,
            expected: None,
            fix: None,
            key_fragments: {
                let mut k = vec![sender.clone()];
                for f in findings {
                    k.push(format!("{}:{}", f.target.as_str(), f.event));
                }
                k
            },
        },
        TopologyError::PatternCapabilityViolation { sender, violations } => DiagnosticPayload {
            code: DiagnosticCode::MeshTopologyPatternCapabilityViolation,
            stage: Stage::MeshTopology,
            actual: None,
            expected: None,
            fix: None,
            key_fragments: {
                let mut k = vec![sender.clone()];
                k.extend(violations.iter().map(|v| v.to_string()));
                k
            },
        },
        TopologyError::MissingBindingField { machine, target, transport, field } => DiagnosticPayload {
            code: DiagnosticCode::MeshTopologyMissingBindingField,
            stage: Stage::MeshTopology,
            actual: None,
            expected: None,
            // The binding path and the missing field are both known;
            // the fix is to add one attribute. Reuses the same Fix
            // variant as forge ValidationError::MissingAttribute so
            // agents share one dispatch arm.
            fix: Some(Fix::AddAttribute {
                element: format!("machines.{machine}.bindings.{}", target.as_str()),
                attr: field.clone(),
            }),
            key_fragments: vec![
                machine.clone(),
                target.as_str().to_string(),
                transport.clone(),
                field.clone(),
            ],
        },
        TopologyError::InvalidBindingField { machine, target, transport, field, reason } => DiagnosticPayload {
            code: DiagnosticCode::MeshTopologyInvalidBindingField,
            stage: Stage::MeshTopology,
            actual: None,
            expected: None,
            fix: None,
            key_fragments: vec![
                machine.clone(),
                target.as_str().to_string(),
                transport.clone(),
                field.clone(),
                reason.clone(),
            ],
        },
        TopologyError::EventBindingUnused { machine, target, event } => DiagnosticPayload {
            code: DiagnosticCode::MeshTopologyEventBindingUnused,
            stage: Stage::MeshTopology,
            actual: Some(event.clone()),
            expected: None,
            // The unused entry is a single known key under the
            // binding's `events:` map; removing it is the only
            // well-defined repair. (The alternative — "rename the
            // sender's <send event=...>" — lives elsewhere and is
            // not local to this binding.)
            fix: Some(Fix::RemoveFields {
                location: format!(
                    "machines.{machine}.bindings.{}.events",
                    target.as_str()
                ),
                fields: vec![event.clone()],
            }),
            key_fragments: vec![machine.clone(), target.as_str().to_string(), event.clone()],
        },
        TopologyError::OrderingCannotBeGuaranteed { machine, target, transport } => DiagnosticPayload {
            code: DiagnosticCode::MeshTopologyOrderingCannotBeGuaranteed,
            stage: Stage::MeshTopology,
            actual: Some(transport.clone()),
            expected: None,
            // Two equally-valid repairs (switch transport OR drop the
            // ordering requirement). Neither is mechanically derivable
            // from this diagnostic alone — the author's intent decides.
            // Leaving `fix: None` keeps agents from prescribing a
            // transport switch when the author may have meant to accept
            // arrival order.
            fix: None,
            key_fragments: vec![
                machine.clone(),
                target.as_str().to_string(),
                transport.clone(),
            ],
        },
        TopologyError::PoolParamNameMissing { machine, target, state, invoke_id, missing } => DiagnosticPayload {
            code: DiagnosticCode::MeshTopologyPoolParamNameMissing,
            stage: Stage::MeshTopology,
            actual: Some(invoke_id.clone()),
            // Two equally-valid repairs (add the missing <param>(s) OR
            // drop the binding-level pool). Author intent decides;
            // leaving fix unstructured keeps agents from prescribing
            // either automatically.
            expected: None,
            fix: None,
            key_fragments: {
                let mut k = vec![
                    machine.clone(),
                    target.as_str().to_string(),
                    state.clone(),
                    invoke_id.clone(),
                ];
                k.extend(missing.iter().cloned());
                k
            },
        },
        TopologyError::SubscriptionSourceUnbound { machine, source_target, available } => DiagnosticPayload {
            code: DiagnosticCode::MeshTopologySubscriptionSourceUnbound,
            stage: Stage::MeshTopology,
            actual: Some(source_target.clone()),
            // Candidate list rides `fix` alone; `expected` stays None
            // to preserve non-overlap. Same shape as MachineNotFound.
            expected: None,
            fix: Some(Fix::ReplaceOneOf {
                candidates: available.iter().map(|t| t.as_str().to_string()).collect(),
            }),
            key_fragments: vec![machine.clone(), source_target.clone()],
        },
        TopologyError::MachineLifetimeSubscriptionUnsupported {
            machine, source_target, event, transport,
        } => DiagnosticPayload {
            code: DiagnosticCode::MeshTopologyMachineLifetimeSubscriptionUnsupported,
            stage: Stage::MeshTopology,
            actual: Some(transport.clone()),
            // Two equally-valid repairs (change transport OR drop the
            // subscription). Neither is mechanically derivable from
            // this diagnostic alone — author intent decides, same
            // shape as OrderingCannotBeGuaranteed.
            expected: None,
            fix: None,
            key_fragments: vec![
                machine.clone(),
                source_target.as_str().to_string(),
                event.clone(),
                transport.clone(),
            ],
        },
    }
}

fn codegen_fields(e: &CodegenError) -> DiagnosticPayload {
    match e {
        CodegenError::UnsupportedLanguage(lang) => DiagnosticPayload {
            code: DiagnosticCode::MeshCodegenUnsupportedLanguage,
            stage: Stage::MeshCodegen,
            actual: Some(lang.clone()),
            expected: None,
            // Closed set of currently-implemented mesh backends.
            // More languages will join over time; the structured list
            // lets agents decide without regexing the message.
            fix: Some(Fix::ReplaceOneOf {
                candidates: vec!["cpp".to_string()],
            }),
            key_fragments: vec![lang.clone()],
        },
        CodegenError::UnsupportedTransport { transport, target } => DiagnosticPayload {
            code: DiagnosticCode::MeshCodegenUnsupportedTransport,
            stage: Stage::MeshCodegen,
            actual: Some(transport.clone()),
            expected: None,
            // Implemented transports live in a single registry
            // (`mesh::transport::implemented_names`). The repair path
            // is authoritative; agents don't parse error prose.
            fix: Some(Fix::ReplaceOneOf {
                candidates: super::transport::implemented_names()
                    .iter()
                    .map(|s| (*s).to_string())
                    .collect(),
            }),
            key_fragments: vec![transport.clone(), target.as_str().to_string()],
        },
        CodegenError::TemplateRead { path, .. } => DiagnosticPayload {
            code: DiagnosticCode::MeshCodegenTemplateRead,
            stage: Stage::MeshCodegen,
            actual: Some(path.clone()),
            expected: None,
            fix: None,
            key_fragments: vec![path.clone()],
        },
        CodegenError::TemplateRender(detail) => DiagnosticPayload {
            code: DiagnosticCode::MeshCodegenTemplateRender,
            stage: Stage::MeshCodegen,
            actual: None,
            expected: None,
            fix: None,
            key_fragments: vec![detail.clone()],
        },
        CodegenError::EventNameCollision { target, suffix, events } => DiagnosticPayload {
            code: DiagnosticCode::MeshCodegenEventNameCollision,
            stage: Stage::MeshCodegen,
            actual: Some(suffix.clone()),
            expected: None,
            fix: None,
            key_fragments: {
                let mut k = vec![target.as_str().to_string(), suffix.clone()];
                k.extend(events.iter().cloned());
                k
            },
        },
        CodegenError::PoolWithRpcClientUnsupported { machine, kind } => DiagnosticPayload {
            code: DiagnosticCode::MeshCodegenPoolWithRpcClientUnsupported,
            stage: Stage::MeshCodegen,
            actual: Some(machine.clone()),
            // Two equally-valid repairs (drop the RPC client site(s)
            // OR reduce `server.instances:` to a single entry).
            // Neither is mechanically derivable from this diagnostic
            // alone — the author's intent decides. Same shape as
            // `MeshTopologyOrderingCannotBeGuaranteed`. The kind
            // discriminator is keyed so diagnostics from the two
            // correlation surfaces stay distinguishable in golden
            // snapshots + downstream tooling.
            expected: None,
            fix: None,
            key_fragments: vec![machine.clone(), rpc_client_kind_tag(kind).to_string()],
        },
    }
}

/// Stable short tag for the [`RpcClientKind`] arm, fed into the
/// diagnostic's `key_fragments` so `fnv1a:...` identity differs
/// between the two rejection shapes. Tags stay ASCII-only and are
/// never rendered to users — the human message reads `kind` via
/// its [`Display`] impl.
fn rpc_client_kind_tag(kind: &RpcClientKind) -> &'static str {
    match kind {
        RpcClientKind::MeshRpc => "mesh_rpc",
        RpcClientKind::SomeipRpcRequest => "someip_rpc_request",
    }
}

impl ToDiagnostics for MeshError {
    fn exit_code(&self) -> i32 {
        MeshError::exit_code(self)
    }

    fn to_diagnostics(&self) -> Vec<Diagnostic> {
        vec![self.to_single_diagnostic()]
    }
}

impl SingleDiagnostic for MeshError {
    fn diagnostic_payload(&self) -> DiagnosticPayload {
        match self {
            MeshError::Deploy(e) => deploy_fields(e),
            MeshError::External(e) => external_fields(e),
            MeshError::Topology(e) => topology_fields(e),
            MeshError::Codegen(e) => codegen_fields(e),
            MeshError::Io { path, .. } => DiagnosticPayload {
                code: DiagnosticCode::MeshIo,
                stage: Stage::Io,
                actual: None,
                expected: None,
                fix: None,
                key_fragments: vec![path.display().to_string()],
            },
        }
    }
}
