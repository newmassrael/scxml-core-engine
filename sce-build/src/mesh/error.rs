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
