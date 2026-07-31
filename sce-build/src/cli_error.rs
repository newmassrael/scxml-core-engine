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
    ReadInput {
        path: String,
        source: std::io::Error,
    },

    #[error("Cannot write {path}: {source}")]
    WriteOutput {
        path: String,
        source: std::io::Error,
    },

    #[error("Cannot create output directory {path}: {source}")]
    CreateOutputDir {
        path: String,
        source: std::io::Error,
    },

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

    /// Spec §synth-6.2.6 generated-source drift: the embedded `source-hash`
    /// or `template-hash` in a generated file no longer matches the
    /// recomputed value over the current source + template state.
    /// `axis` carries `"source"` or `"template"` to disambiguate which
    /// half drifted; `actual_hex` is the embedded (out-of-date) value,
    /// `expected_hex` is the freshly-computed one. Repair is
    /// deterministic: rerun `sce-codegen` with the same inputs.
    #[error(
        "{path}: §6.2.6 {axis}-hash mismatch (embedded={actual_hex}, recomputed={expected_hex}) — regenerate via sce-codegen"
    )]
    VerifySourceHashMismatch {
        path: String,
        axis: &'static str,
        expected_hex: String,
        actual_hex: String,
    },

    /// Spec §synth-6.2.6 source-set coverage: the `source-hash` about to be
    /// embedded in generated output does not describe the input that
    /// produced it.
    ///
    /// The set is folded from every `**/*.scxml` under `root`; when that
    /// walk resolves to nothing the fold still produces a well-formed
    /// sha256 (the empty-input digest), which no downstream drift check
    /// can tell apart from a real one. Refusing to emit is the only
    /// signal that survives — a wrong-but-plausible hash does not.
    ///
    /// Two ways to get here, distinguishable from `hashed`:
    /// - `hashed == 0` — the root resolved to nothing, so the header would
    ///   carry the empty-input digest. Always an error.
    /// - `hashed > 0` — the root was inferred from the input's own
    ///   location, yet the input is absent from the set the walk built.
    ///   A caller that named `--input-root` explicitly is not held to this:
    ///   it may be generating from a staged derivative of a tracked source
    ///   (the fixture regen scripts do exactly that), and the root it named
    ///   is an assertion rather than an inference to second-guess.
    #[error(
        "{input}: §6.2.6 source-hash would not describe it — {hashed} file(s) \
         collected from {root}; pass --input-root <DIR> containing the input"
    )]
    SourceHashInputUncovered {
        input: String,
        root: String,
        hashed: usize,
    },
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
            CliError::VerifySourceHashMismatch {
                path,
                axis,
                expected_hex,
                actual_hex,
            } => (
                DiagnosticCode::ForgeSourceHashMismatch,
                vec![
                    path.clone(),
                    (*axis).to_string(),
                    expected_hex.clone(),
                    actual_hex.clone(),
                ],
                // `actual` field carries the axis label + embedded value
                // so consumers parsing the wire can identify which half
                // drifted without re-reading the file.
                Some(format!("{axis}-hash={actual_hex}")),
                None,
            ),
            CliError::SourceHashInputUncovered {
                input,
                root,
                hashed,
            } => (
                DiagnosticCode::ForgeSourceHashInputUncovered,
                vec![input.clone(), root.clone()],
                // `actual` carries the root that came up short plus the
                // collected count, which is what an agent needs to decide
                // between "root resolved to nothing" and "input lives
                // elsewhere" without re-walking the tree itself.
                Some(format!("root={root} hashed={hashed}")),
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
