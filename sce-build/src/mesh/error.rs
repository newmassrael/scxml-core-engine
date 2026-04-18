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
    /// violates the minimum-floor constraint (SCE Mesh §16.7 row 9;
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
    /// server-side pool that SCE currently does not support. A single
    /// SCXML session cannot semantically back N independent service
    /// instances (the W3C SCXML execution model ties one document to
    /// one state machine to one identity). Multi-instance server
    /// support requires per-instance SCXML sessions, which is a
    /// separate spec track; until it lands, deploy "N independent
    /// instances of the same service" as N processes each hosting a
    /// single instance. See SCE_MESH.md §14.4.
    #[error("machine '{machine}': `server.instances:` is not supported — a single SCXML \
             session cannot host N independent SOME/IP instances (multi-session \
             territory). Drop `instances:` from the server section (exposing exactly one \
             instance), or run N processes each hosting a single-instance server. \
             See SCE_MESH.md §14.4.")]
    ServerPoolNotSupported {
        machine: String,
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
        DeployError::ServerPoolNotSupported { machine } => DiagnosticPayload {
            code: DiagnosticCode::MeshDeployServerPoolNotSupported,
            stage: Stage::MeshDeploy,
            actual: Some(machine.clone()),
            // Single deterministic repair: remove `instances:` from the
            // server section. The alternative shape ("run N processes")
            // is deployment-topology advice, not a field-level edit, so
            // it stays in the prose rather than `fix`.
            expected: None,
            fix: Some(Fix::RemoveFields {
                location: format!("topology.*.machines.{machine}.server"),
                fields: vec!["instances".to_string()],
            }),
            key_fragments: vec![machine.clone()],
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
