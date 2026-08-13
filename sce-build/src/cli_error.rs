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
/// Almost every variant exits 20 so build systems that branch on
/// 0 / non-zero keep working while consumers use the structured
/// `code` field for finer routing. Dedicated codes would be
/// over-fitting — the user-visible distinctions (unknown language
/// vs missing file) aren't pipeline stages in the forge/mesh sense.
///
/// [`CliError::QueryNoMatch`] is the one exception, and it is an
/// exception about what happened rather than about how loudly to
/// complain: nothing failed there, so collapsing it into the
/// driver-failure status would erase the distinction a shell consumer
/// branches on. `SCE_ERROR_CONTRACT.md` §6 registers both statuses.
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    /// A `--language` value no backend answers to.
    ///
    /// The menu comes from the route rather than from a sentence written
    /// here: this variant is raised by seven subcommands that do not all
    /// serve the same backends, and the one sentence that used to end it
    /// ("Use rust, cpp, kotlin, or go.") named a set that was stale for
    /// two of them and wrong for all seven.
    #[error("Unknown language: {lang}. `{}` takes {}.", .route.subcommand(), .route.menu())]
    UnknownLanguage {
        lang: String,
        route: crate::cli_language::LanguageRoute,
    },

    /// A backend that exists but that this route does not serve.
    ///
    /// Distinct from [`CliError::UnknownLanguage`] because the repair is
    /// different — the caller spelled a real backend and needs to know
    /// which route reaches it, not which names parse. `lang` holds the
    /// backend, so `actual` on the wire means one thing across both
    /// variants; the route and its reason ride the message.
    #[error("`{}` does not target {lang}. It takes {}{}",
        .route.subcommand(),
        .route.menu(),
        .route.exclusion_reason().map(|r| format!(" — {r}")).unwrap_or_default())]
    UnsupportedLanguage {
        lang: String,
        route: crate::cli_language::LanguageRoute,
    },

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

    /// `--suite-package` names what an emitted W3C conformance suite
    /// calls itself, and there are three ways to name it wrongly: the
    /// backend emits nothing that spells a suite, the name cannot be
    /// spelled in the target language, or the run is writing into this
    /// repository — where the committed build files already fix the
    /// name, so a rename would leave the emitted sources naming a
    /// package that does not exist.
    ///
    /// One variant rather than three because the repair is the same
    /// shape in all three — change or drop the flag — and `detail`
    /// carries which of the three it was. Splitting would put a
    /// pipeline-stage distinction on what is one option's validation.
    #[error("--suite-package: {detail}")]
    InvalidSuitePackage { detail: String },

    /// The binary was not built from the sources it is standing in.
    ///
    /// The message carries the rebuild command rather than a description
    /// of the mismatch, because the mismatch is never the interesting
    /// half: whoever sees this needs the one command that makes it go
    /// away, and every caller — CMake, gradle, a developer — repairs it
    /// the same way. See [`crate::generator_witness`] for why the check
    /// is a digest over bytes and not a comparison of timestamps.
    #[error(
        "this sce-codegen was built from different sources than {root} holds \
         (binary={embedded_hex}, tree={recomputed_hex}) — rebuild it with: \
         cargo build --bin sce-codegen --features cli -p sce-build"
    )]
    GeneratorSourceDrift {
        root: String,
        embedded_hex: String,
        recomputed_hex: String,
    },

    /// Freshness could not be established, on either side of the
    /// comparison. Deliberately not spelled as drift: nothing was
    /// measured, so there is no disagreement to report, and saying there
    /// is would send the reader to rebuild against a tree that was never
    /// the problem.
    ///
    /// `reason` carries which half was missing — the binary was built
    /// where the witness set was unreadable, or the tree in front of it
    /// does not hold one. One variant rather than two because the reader's
    /// next move is the same in both cases (get a full checkout on the
    /// side that lacks one) and `reason` already names which side.
    #[error(
        "cannot establish whether this sce-codegen matches {root}: {reason}. \
         Both halves need a full checkout — rebuild the binary with \
         `cargo build --bin sce-codegen --features cli -p sce-build`, or point \
         --root at the workspace it was built from"
    )]
    GeneratorSourceUnverifiable { root: String, reason: String },

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

    /// Spec §synth-6.2.6 source-set enumeration did not terminate within its
    /// descent ceiling.
    ///
    /// A directory symlink naming a sibling contributes under every name
    /// that reaches it, so nested levels of such links name a number of
    /// root-relative paths exponential in the depth. Those paths are all
    /// genuine inputs under the documented rule, which is why the walk
    /// refuses rather than hashing whichever prefix it reached: a digest
    /// over a subset is the same unauditable header the empty-set refusal
    /// above exists to prevent.
    ///
    /// Distinct from `ReadInput` on purpose. Both used to surface as
    /// `cli/read-input`, which routes a repair consumer at file permissions;
    /// the repair here is the input layout — re-point `--input-root` below
    /// the link farm, or remove the aliasing.
    ///
    /// `limit` is the ceiling, never how far the traversal got. The latter
    /// varies with directory iteration order, and a diagnostic that shifts
    /// between machines for one tree is not a diagnostic.
    #[error(
        "{root}: §6.2.6 source set exceeds {limit} directories — a directory \
         symlink reaching a sibling multiplies the paths under it; re-point \
         --input-root at a tree without the aliasing, or remove it"
    )]
    SourceHashWalkUnbounded { root: String, limit: usize },

    /// The command line did not parse.
    ///
    /// `detail` is the argument parser's own rendering, kept verbatim
    /// because it already names the offending token and prints the
    /// usage line — restating it here would be a second sentence about
    /// one fact, which is how the two halves drift apart.
    ///
    /// This variant is why the binary parses with `try_parse` rather
    /// than `parse`: the derive helper's failure path prints prose and
    /// exits 2 on its own, and 2 is the status
    /// `SCE_ERROR_CONTRACT.md` §6 assigns to `xml/*`. A machine caller
    /// that mistypes a flag was being told its *document* was
    /// malformed, with no record to read.
    #[error("{detail}")]
    Usage { detail: String },

    /// A query ran against a well-formed artifact and matched nothing.
    ///
    /// `searched` names what was looked in so the caller can tell an
    /// empty artifact from a wrong query; it stays out of
    /// `key_fragments` because it carries a caller-supplied path and
    /// the `id` must not move with it.
    #[error("{tool}: {query} matched nothing in {searched}")]
    QueryNoMatch {
        tool: &'static str,
        query: String,
        searched: String,
    },
}

impl CliError {
    /// Canonical CLI-boundary exit code. Shared across every variant
    /// except [`CliError::QueryNoMatch`] — see the type-level comment.
    pub const EXIT_CODE: i32 = 20;

    /// Exit status for a query that ran and found nothing.
    ///
    /// Separate from [`CliError::EXIT_CODE`] because the two answer
    /// different questions: 20 means the driver could not do the work,
    /// 1 means it did the work and the answer was "none". The query
    /// subcommands document this so a build gate can assert symbol
    /// presence without parsing JSON.
    pub const EXIT_QUERY_NO_MATCH: i32 = 1;
}

impl ToDiagnostics for CliError {
    fn exit_code(&self) -> i32 {
        match self {
            CliError::QueryNoMatch { .. } => Self::EXIT_QUERY_NO_MATCH,
            _ => Self::EXIT_CODE,
        }
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
            // The route joins the hash key because the same rejected
            // name means a different diagnostic on a different route:
            // `python` is unknown to nobody, but `c11` is a dead end on
            // `generate-w3c` and a working target on `generate`, and the
            // candidate list a consumer repairs from differs with it.
            CliError::UnknownLanguage { lang, route } => (
                DiagnosticCode::CliUnknownLanguage,
                vec![lang.clone(), route.subcommand().to_string()],
                Some(lang.clone()),
                Some(Fix::ReplaceOneOf {
                    candidates: route.candidates(),
                }),
            ),
            // Carries the same repair as the unknown case: the caller
            // named a real backend, so the actionable answer is which
            // backends this route does reach. Leaving it absent made a
            // machine consumer's only recourse the prose message.
            CliError::UnsupportedLanguage { lang, route } => (
                DiagnosticCode::CliUnsupportedLanguage,
                vec![lang.clone(), route.subcommand().to_string()],
                Some(lang.clone()),
                Some(Fix::ReplaceOneOf {
                    candidates: route.candidates(),
                }),
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
            CliError::InvalidSuitePackage { detail } => (
                DiagnosticCode::CliInvalidSuitePackage,
                vec![detail.clone()],
                None,
                None,
            ),
            CliError::GeneratorSourceDrift {
                root,
                embedded_hex,
                recomputed_hex,
            } => (
                DiagnosticCode::CliGeneratorSourceDrift,
                // The tree's digest is the identity of the answer: the
                // same stale binary in front of two different trees is
                // two different facts, while the same tree reported twice
                // is one. Keying on the embedded digest instead would
                // collapse the first pair and split the second.
                vec![root.clone(), recomputed_hex.clone()],
                // `actual` is what the binary claims to be — the value a
                // consumer needs in order to say *which* generator it
                // was holding.
                Some(embedded_hex.clone()),
                None,
            ),
            CliError::GeneratorSourceUnverifiable { root, reason } => (
                DiagnosticCode::CliGeneratorSourceUnverifiable,
                vec![root.clone(), reason.clone()],
                None,
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
                // collected count, which is what a consumer needs to decide
                // between "root resolved to nothing" and "input lives
                // elsewhere" without re-walking the tree itself.
                Some(format!("root={root} hashed={hashed}")),
                None,
            ),
            CliError::SourceHashWalkUnbounded { root, limit } => (
                DiagnosticCode::ForgeSourceHashWalkUnbounded,
                vec![root.clone()],
                // `actual` states the ceiling rather than a traversal count
                // so the record is identical for one tree on any machine.
                Some(format!("root={root} descent-limit={limit}")),
                None,
            ),
            // The parser's text is the whole diagnostic, so it is also
            // the whole hash key: two different malformed command lines
            // are two different diagnostics.
            CliError::Usage { detail } => {
                (DiagnosticCode::CliUsage, vec![detail.clone()], None, None)
            }
            CliError::QueryNoMatch { tool, query, .. } => (
                DiagnosticCode::CliQueryNoMatch,
                vec![(*tool).to_string(), query.clone()],
                Some(query.clone()),
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
