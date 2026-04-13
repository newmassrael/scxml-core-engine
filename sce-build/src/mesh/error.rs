// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Structured error hierarchy for the SCE Mesh pipeline.
//
// Each variant maps to a pipeline stage:
//   Deploy   → stage 1 (deploy.yaml parsing)
//   Topology → stage 2 (target resolution + validation)
//   Codegen  → stage 3 (template rendering)
//   Io       → cross-cutting filesystem errors

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
}

// ── Stage 2: Topology resolution ─────────────────────────────

/// Errors from <send> target collection and deploy.yaml binding matching.
#[derive(Debug, thiserror::Error)]
pub enum TopologyError {
    /// SCXML <send> targets that have no matching deploy.yaml binding.
    #[error("unresolved send targets for machine '{machine}': {targets}. \
             Each <send target=\"...\"> in SCXML must have a corresponding \
             binding in deploy.yaml", targets = .targets.join(", "))]
    UnresolvedTargets {
        machine: String,
        targets: Vec<String>,
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
        target: String,
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
                 f.target, f.event, f.target.trim_start_matches('#')))
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

    /// Unrecognized `sce:pattern` attribute value on a `<send>` action.
    /// Likely a typo — fails the build to prevent silent validation bypass.
    #[error("unrecognized sce:pattern=\"{value}\" on <send target=\"{target}\" event=\"{event}\"/> \
             in state '{state}' of machine '{sender}'. \
             Valid values: request, response, fire_forget, subscribe, notification, field_get, field_set, none")]
    UnrecognizedPattern {
        sender: String,
        state: String,
        target: String,
        event: String,
        value: String,
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
        target: String,
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
            MeshError::Topology(_) => 11,
            MeshError::Codegen(_) => 12,
            MeshError::Io { .. } => 13,
        }
    }
}
