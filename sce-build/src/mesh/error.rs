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

    /// All targets for a machine must use the same transport type.
    #[error("machine '{machine}' has mixed transport types: {transports}. \
             All <send> targets must use the same transport in deploy.yaml",
             transports = .transports.join(", "))]
    MixedTransports {
        machine: String,
        transports: Vec<String>,
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
