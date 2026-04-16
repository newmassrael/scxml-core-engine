// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// CLI-boundary error family.
//
// This error set sits alongside [`forge::error::ForgeError`] and
// [`mesh::error::MeshError`] as the third first-party diagnostic
// family: its variants map one-to-one to the `cli/*` codes on the
// wire vocabulary. Variants cover argument parsing, workspace
// layout discovery, and I/O at the CLI boundary — failure modes
// that the original implementation handled with ad-hoc
// `eprintln!` + `exit(1)` calls.
//
// Housing it in the library (rather than the `sce_codegen` binary)
// is what lets every downstream embedder — library tests, future
// IDE plugins, alternate binaries — emit these codes uniformly
// through the same `ToDiagnostics` pipeline. The binary continues
// to be the only process that routes these errors to stderr.

use crate::forge::diagnostic::{
    Diagnostic, DiagnosticCode, DiagnosticPayload, Fix, SingleDiagnostic, Stage, ToDiagnostics,
};

/// CLI-driver errors that do not originate in a compiler pipeline.
///
/// Every subcommand dispatches through [`ToDiagnostics`] so the
/// `--error-format=json` flag is honoured uniformly: the flag is not
/// a contract if half the failure modes still emit prose.
///
/// Exit codes share a single value (20) so build systems that branch
/// on 0 / non-zero keep working while agents use the structured
/// `code` field for finer routing. Dedicated codes would be
/// over-fitting — the user-visible distinctions (unknown language
/// vs missing file) aren't pipeline stages in the forge/mesh sense.
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("Unknown language: {lang}. Use rust, cpp, kotlin, or go.")]
    UnknownLanguage { lang: String },

    #[error("{lang} codegen is not yet supported")]
    UnsupportedLanguage { lang: String },

    #[error("Cannot read {path}: {source}")]
    ReadInput { path: String, source: std::io::Error },

    #[error("Cannot write {path}: {source}")]
    WriteOutput { path: String, source: std::io::Error },

    #[error("Cannot create output directory {path}: {source}")]
    CreateOutputDir { path: String, source: std::io::Error },

    #[error("{stage}: {detail}")]
    ScxmlGenerate { stage: &'static str, detail: String },

    #[error("No description field found in {path}")]
    MissingMetadataField { path: String },

    #[error("Not a directory: {path}")]
    NotADirectory { path: String },

    #[error("unknown --format {value}; expected {expected}")]
    InvalidFormatOption { value: String, expected: String },

    #[error("JSON serialization failed: {detail}")]
    JsonSerialization { detail: String },

    #[error("Cannot find project root. Run from project directory or set --registry/--resources.")]
    ProjectRootNotFound,

    #[error("--format-style file not found: {path}")]
    FormatStyleNotFound { path: String },

    #[error("No <scxml> tag found in {path}")]
    NoScxmlTag { path: String },
}

impl CliError {
    /// Canonical CLI-boundary exit code. Shared across every variant —
    /// see the type-level comment for the rationale.
    pub const EXIT_CODE: i32 = 20;
}

impl ToDiagnostics for CliError {
    fn exit_code(&self) -> i32 {
        Self::EXIT_CODE
    }

    fn to_diagnostics(&self) -> Vec<Diagnostic> {
        vec![self.to_single_diagnostic()]
    }
}

impl SingleDiagnostic for CliError {
    /// Per-variant extraction: `key_fragments` drives the content-hash
    /// id; `actual` / `fix` populate the corresponding diagnostic
    /// fields. `expected` is never populated from CLI errors — every
    /// case with a candidate list routes it through `fix` instead,
    /// keeping the two fields disjoint as required by the diagnostic
    /// contract.
    fn diagnostic_payload(&self) -> DiagnosticPayload {
        let (code, key_fragments, actual, fix) = match self {
            CliError::UnknownLanguage { lang } => (
                DiagnosticCode::CliUnknownLanguage,
                vec![lang.clone()],
                Some(lang.clone()),
                Some(Fix::ReplaceOneOf {
                    candidates: vec!["rust".into(), "cpp".into(), "kotlin".into(), "go".into()],
                }),
            ),
            CliError::UnsupportedLanguage { lang } => (
                DiagnosticCode::CliUnsupportedLanguage,
                vec![lang.clone()],
                Some(lang.clone()),
                None,
            ),
            CliError::ReadInput { path, .. } => (
                DiagnosticCode::CliReadInput,
                vec![path.clone()],
                Some(path.clone()),
                None,
            ),
            CliError::WriteOutput { path, .. } => (
                DiagnosticCode::CliWriteOutput,
                vec![path.clone()],
                Some(path.clone()),
                None,
            ),
            CliError::CreateOutputDir { path, .. } => (
                DiagnosticCode::CliCreateOutputDir,
                vec![path.clone()],
                Some(path.clone()),
                None,
            ),
            CliError::ScxmlGenerate { stage, detail } => (
                DiagnosticCode::CliScxmlGenerate,
                vec![(*stage).to_string(), detail.clone()],
                None,
                None,
            ),
            CliError::MissingMetadataField { path } => (
                DiagnosticCode::CliMissingMetadataField,
                vec![path.clone()],
                Some(path.clone()),
                None,
            ),
            CliError::NotADirectory { path } => (
                DiagnosticCode::CliNotADirectory,
                vec![path.clone()],
                Some(path.clone()),
                None,
            ),
            CliError::InvalidFormatOption { value, expected } => (
                DiagnosticCode::CliInvalidFormatOption,
                vec![value.clone(), expected.clone()],
                Some(value.clone()),
                Some(Fix::ReplaceOneOf {
                    candidates: expected.split('|').map(str::to_string).collect(),
                }),
            ),
            CliError::JsonSerialization { detail } => (
                DiagnosticCode::CliJsonSerialization,
                vec![detail.clone()],
                None,
                None,
            ),
            CliError::ProjectRootNotFound => (
                DiagnosticCode::CliProjectRootNotFound,
                Vec::new(),
                None,
                None,
            ),
            CliError::FormatStyleNotFound { path } => (
                DiagnosticCode::CliFormatStyleNotFound,
                vec![path.clone()],
                Some(path.clone()),
                None,
            ),
            CliError::NoScxmlTag { path } => (
                DiagnosticCode::CliNoScxmlTag,
                vec![path.clone()],
                Some(path.clone()),
                None,
            ),
        };
        DiagnosticPayload {
            code,
            stage: Stage::Cli,
            expected: None,
            actual,
            fix,
            key_fragments,
        }
    }
}
