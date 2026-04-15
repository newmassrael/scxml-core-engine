// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
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
    compute_id, Diagnostic, DiagnosticCode, Fix, Location, Stage, ToDiagnostics, SCHEMA_VERSION,
};

struct MeshDiagFields {
    code: DiagnosticCode,
    stage: Stage,
    actual: Option<String>,
    expected: Option<Vec<String>>,
    fix: Option<Fix>,
    key: Vec<String>,
}

fn deploy_fields(e: &DeployError) -> MeshDiagFields {
    match e {
        DeployError::ReadFile { path, .. } => MeshDiagFields {
            code: DiagnosticCode::MeshDeployRead,
            stage: Stage::MeshDeploy,
            actual: Some(path.clone()),
            expected: None,
            fix: None,
            key: vec![path.clone()],
        },
        DeployError::Yaml(reason) => MeshDiagFields {
            code: DiagnosticCode::MeshDeployParse,
            stage: Stage::MeshDeploy,
            actual: None,
            expected: None,
            fix: None,
            key: vec![reason.clone()],
        },
        DeployError::UnsupportedVersion { found, supported } => MeshDiagFields {
            code: DiagnosticCode::MeshDeployUnsupportedVersion,
            stage: Stage::MeshDeploy,
            actual: Some(found.clone()),
            // Candidate list rides `fix`; `expected` stays None so the
            // two fields never duplicate each other (contract §3.2).
            expected: None,
            fix: Some(Fix::ReplaceOneOf {
                candidates: supported.iter().map(|s| (*s).to_string()).collect(),
            }),
            key: vec![found.clone()],
        },
        DeployError::DuplicateMachine { machine, devices } => MeshDiagFields {
            code: DiagnosticCode::MeshDeployDuplicateMachine,
            stage: Stage::MeshDeploy,
            actual: Some(machine.clone()),
            expected: None,
            fix: None,
            key: {
                let mut k = vec![machine.clone()];
                k.extend(devices.iter().cloned());
                k
            },
        },
    }
}

fn external_fields(e: &ExternalConfigError) -> MeshDiagFields {
    match e {
        ExternalConfigError::Read { path, .. } => MeshDiagFields {
            code: DiagnosticCode::MeshExternalRead,
            stage: Stage::MeshExternal,
            actual: Some(path.clone()),
            expected: None,
            fix: None,
            key: vec![path.clone()],
        },
        ExternalConfigError::Parse { path, reason } => MeshDiagFields {
            code: DiagnosticCode::MeshExternalParse,
            stage: Stage::MeshExternal,
            actual: None,
            expected: None,
            fix: None,
            key: vec![path.clone(), reason.clone()],
        },
        ExternalConfigError::UnresolvedNames { machine, config_path, missing } => MeshDiagFields {
            code: DiagnosticCode::MeshExternalUnresolvedNames,
            stage: Stage::MeshExternal,
            actual: None,
            expected: None,
            fix: None,
            key: {
                let mut k = vec![machine.clone(), config_path.clone()];
                k.extend(missing.iter().map(|m| format!("{}:{}", m.kind, m.name)));
                k
            },
        },
        ExternalConfigError::AmbiguousEventGroup { machine, target, event_group, count, .. } => MeshDiagFields {
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
            key: vec![machine.clone(), target.clone(), event_group.clone()],
        },
        ExternalConfigError::EmptyEventGroup { machine, target, event_group, .. } => MeshDiagFields {
            code: DiagnosticCode::MeshExternalEmptyEventGroup,
            stage: Stage::MeshExternal,
            actual: None,
            expected: None,
            fix: None,
            key: vec![machine.clone(), target.clone(), event_group.clone()],
        },
        ExternalConfigError::NamedReferenceWithoutConfig { machine, device, target } => MeshDiagFields {
            code: DiagnosticCode::MeshExternalNamedReferenceWithoutConfig,
            stage: Stage::MeshExternal,
            actual: None,
            expected: None,
            fix: None,
            key: vec![machine.clone(), device.clone(), target.clone()],
        },
        ExternalConfigError::ReservedSomeipIdKeys { machine, target, transport, fields } => MeshDiagFields {
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
            key: {
                let mut k = vec![machine.clone(), target.clone(), transport.clone()];
                k.extend(fields.iter().map(|f| (*f).to_string()));
                k
            },
        },
        ExternalConfigError::SomeipFieldOnNonSomeipTransport { machine, target, transport, fields } => MeshDiagFields {
            code: DiagnosticCode::MeshExternalSomeipFieldOnNonSomeipTransport,
            stage: Stage::MeshExternal,
            actual: Some(transport.clone()),
            // Single deterministic answer: the offending fields are
            // SOME/IP-specific, so the only repair that preserves them
            // is to switch the binding's transport to `someip`.
            expected: None,
            fix: Some(Fix::ReplaceWith { to: "someip".to_string() }),
            key: {
                let mut k = vec![machine.clone(), target.clone(), transport.clone()];
                k.extend(fields.iter().map(|f| (*f).to_string()));
                k
            },
        },
        ExternalConfigError::ConflictingEventSchema { machine, target, flat_fields } => MeshDiagFields {
            code: DiagnosticCode::MeshExternalConflictingEventSchema,
            stage: Stage::MeshExternal,
            actual: None,
            expected: None,
            fix: None,
            key: {
                let mut k = vec![machine.clone(), target.clone()];
                k.extend(flat_fields.iter().map(|f| (*f).to_string()));
                k
            },
        },
        ExternalConfigError::ConflictingEventFieldKinds { machine, target, event, fields } => MeshDiagFields {
            code: DiagnosticCode::MeshExternalConflictingEventFieldKinds,
            stage: Stage::MeshExternal,
            actual: None,
            expected: None,
            fix: None,
            key: {
                let mut k = vec![machine.clone(), target.clone(), event.clone()];
                k.extend(fields.iter().cloned());
                k
            },
        },
        ExternalConfigError::EmptyEventEntry { machine, target, event } => MeshDiagFields {
            code: DiagnosticCode::MeshExternalEmptyEventEntry,
            stage: Stage::MeshExternal,
            actual: None,
            expected: None,
            fix: None,
            key: vec![machine.clone(), target.clone(), event.clone()],
        },
    }
}

fn topology_fields(e: &TopologyError) -> MeshDiagFields {
    match e {
        TopologyError::UnresolvedTargets { machine, targets } => MeshDiagFields {
            code: DiagnosticCode::MeshTopologyUnresolvedTargets,
            stage: Stage::MeshTopology,
            actual: None,
            expected: None,
            fix: None,
            key: {
                let mut k = vec![machine.clone()];
                k.extend(targets.iter().map(|t| t.as_str().to_string()));
                k
            },
        },
        TopologyError::MachineNotFound { machine, available } => MeshDiagFields {
            code: DiagnosticCode::MeshTopologyMachineNotFound,
            stage: Stage::MeshTopology,
            actual: Some(machine.clone()),
            expected: None,
            fix: Some(Fix::ReplaceOneOf {
                candidates: available.clone(),
            }),
            key: vec![machine.clone()],
        },
        TopologyError::ReceiverNotDeclared { sender, target, receiver } => MeshDiagFields {
            code: DiagnosticCode::MeshTopologyReceiverNotDeclared,
            stage: Stage::MeshTopology,
            actual: Some(receiver.clone()),
            expected: None,
            fix: None,
            key: vec![sender.clone(), target.as_str().to_string(), receiver.clone()],
        },
        TopologyError::AbsoluteSourcePath { machine, path } => MeshDiagFields {
            code: DiagnosticCode::MeshTopologyAbsoluteSourcePath,
            stage: Stage::MeshTopology,
            actual: Some(path.clone()),
            expected: None,
            fix: None,
            key: vec![machine.clone(), path.clone()],
        },
        TopologyError::ReceiverSourceRead { machine, path, .. } => MeshDiagFields {
            code: DiagnosticCode::MeshTopologyReceiverSourceRead,
            stage: Stage::MeshTopology,
            actual: Some(path.clone()),
            expected: None,
            fix: None,
            key: vec![machine.clone(), path.clone()],
        },
        TopologyError::ReceiverSourceParse { machine, path, reason } => MeshDiagFields {
            code: DiagnosticCode::MeshTopologyReceiverSourceParse,
            stage: Stage::MeshTopology,
            actual: None,
            expected: None,
            fix: None,
            key: vec![machine.clone(), path.clone(), reason.clone()],
        },
        TopologyError::UncoveredEvents { sender, findings } => MeshDiagFields {
            code: DiagnosticCode::MeshTopologyUncoveredEvents,
            stage: Stage::MeshTopology,
            actual: None,
            expected: None,
            fix: None,
            key: {
                let mut k = vec![sender.clone()];
                for f in findings {
                    k.push(format!("{}:{}", f.target.as_str(), f.event));
                }
                k
            },
        },
        TopologyError::PatternCapabilityViolation { sender, violations } => MeshDiagFields {
            code: DiagnosticCode::MeshTopologyPatternCapabilityViolation,
            stage: Stage::MeshTopology,
            actual: None,
            expected: None,
            fix: None,
            key: {
                let mut k = vec![sender.clone()];
                k.extend(violations.iter().map(|v| v.to_string()));
                k
            },
        },
        TopologyError::MissingBindingField { machine, target, transport, field } => MeshDiagFields {
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
            key: vec![
                machine.clone(),
                target.as_str().to_string(),
                transport.clone(),
                field.clone(),
            ],
        },
        TopologyError::InvalidBindingField { machine, target, transport, field, reason } => MeshDiagFields {
            code: DiagnosticCode::MeshTopologyInvalidBindingField,
            stage: Stage::MeshTopology,
            actual: None,
            expected: None,
            fix: None,
            key: vec![
                machine.clone(),
                target.as_str().to_string(),
                transport.clone(),
                field.clone(),
                reason.clone(),
            ],
        },
        TopologyError::EventBindingUnused { machine, target, event } => MeshDiagFields {
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
            key: vec![machine.clone(), target.as_str().to_string(), event.clone()],
        },
    }
}

fn codegen_fields(e: &CodegenError) -> MeshDiagFields {
    match e {
        CodegenError::UnsupportedLanguage(lang) => MeshDiagFields {
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
            key: vec![lang.clone()],
        },
        CodegenError::UnsupportedTransport { transport, target } => MeshDiagFields {
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
            key: vec![transport.clone(), target.as_str().to_string()],
        },
        CodegenError::TemplateRead { path, .. } => MeshDiagFields {
            code: DiagnosticCode::MeshCodegenTemplateRead,
            stage: Stage::MeshCodegen,
            actual: Some(path.clone()),
            expected: None,
            fix: None,
            key: vec![path.clone()],
        },
        CodegenError::TemplateRender(detail) => MeshDiagFields {
            code: DiagnosticCode::MeshCodegenTemplateRender,
            stage: Stage::MeshCodegen,
            actual: None,
            expected: None,
            fix: None,
            key: vec![detail.clone()],
        },
        CodegenError::EventNameCollision { target, suffix, events } => MeshDiagFields {
            code: DiagnosticCode::MeshCodegenEventNameCollision,
            stage: Stage::MeshCodegen,
            actual: Some(suffix.clone()),
            expected: None,
            fix: None,
            key: {
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
        let fields = match self {
            MeshError::Deploy(e) => deploy_fields(e),
            MeshError::External(e) => external_fields(e),
            MeshError::Topology(e) => topology_fields(e),
            MeshError::Codegen(e) => codegen_fields(e),
            MeshError::Io { path, .. } => MeshDiagFields {
                code: DiagnosticCode::MeshIo,
                stage: Stage::Io,
                actual: None,
                expected: None,
                fix: None,
                key: vec![path.display().to_string()],
            },
        };

        let id = compute_id(fields.code, fields.stage, None, &fields.key);
        vec![Diagnostic {
            schema_version: SCHEMA_VERSION,
            id,
            code: fields.code,
            stage: fields.stage,
            spec: fields.code.spec_anchor(),
            message: self.to_string(),
            location: None::<Location>,
            expected: fields.expected,
            actual: fields.actual,
            fix: fields.fix,
        }]
    }
}
