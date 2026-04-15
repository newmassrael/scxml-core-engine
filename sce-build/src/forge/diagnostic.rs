// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Machine-readable diagnostic schema for SCE Forge errors.
//
// Upstream agents (LangGraph-style triage, IDEs, CI) consume this
// format. The design leans on four invariants:
//
//   1. `code` is a closed enum (`DiagnosticCode`), not a free string —
//      agents dispatch by variant, not by parsing text.
//   2. The `id` is a content hash of semantic fields only (never the
//      Display message) — rewording an error message does not shift
//      its identity.
//   3. Every populated `Fix` variant carries a real payload. If no
//      deterministic repair is available, `fix` is `None` (honest).
//   4. `location` rides on any error via the `Located<E>` wrapper
//      struct. Leaf errors stay location-free; call-sites that know
//      *where* the failure originated construct `Located::new(err,
//      file, line, col)` and propagate that instead.
//
// Human rendering lives in `Display for ForgeError`. Machine rendering
// goes through `to_diagnostics()` → `serde_json`.

use crate::forge::error::{
    ExprError, ForgeError, GenerateError, ImportError, Located, ManifestError, SourceLocation,
    ValidationError, XmlError,
};
use serde::Serialize;

/// Common interface for any error type that can be rendered to a
/// machine-readable [`Diagnostic`] and terminate the process with a
/// stage-specific exit code.
///
/// Callers (CLI entrypoints, build.rs helpers) depend only on this
/// trait. Each error family (`ForgeError`, `MeshError`, CLI-level
/// errors) provides its own mapping without coupling to the others.
pub trait ToDiagnostics {
    /// Expand this error into one or more diagnostic records.
    ///
    /// Returns a `Vec` because a single error may represent multiple
    /// independent violations — XSD validation is the canonical case:
    /// one invocation can surface three enum-violations on three
    /// different lines, and merging them into a single record would
    /// hide the per-violation line data that upstream agents need.
    ///
    /// Call-sites that know the error is single-valued (everything
    /// except XSD schema validation today) simply return `vec![one]`.
    fn to_diagnostics(&self) -> Vec<Diagnostic>;
    fn exit_code(&self) -> i32;
}

// ── Top-level diagnostic record ────────────────────────────────

/// Current wire-format version. Bumped on *breaking* changes to the
/// diagnostic shape (renamed field, dropped field, changed semantics).
/// Purely additive changes (new optional field) do **not** bump it —
/// consumers must ignore unknown fields, per NDJSON contract.
pub const SCHEMA_VERSION: u32 = 1;

/// A single machine-readable diagnostic, one record per NDJSON line.
///
/// Serialized field order is fixed: `v` first so any consumer can
/// version-gate before reading anything else, `id` second so streams
/// can dedup without a full parse.
#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    /// Wire-format version. Always present, always first. Agents that
    /// see a higher value than they were built against should fall
    /// back to a best-effort parse rather than crash.
    #[serde(rename = "v")]
    pub schema_version: u32,

    /// Content-hash id. Prefix names the algorithm so future migration
    /// (e.g. to blake3) can be rolled out without breaking consumers
    /// that pattern-match the format.
    pub id: String,

    /// Closed enum, serialized as a slash-path string. Agents dispatch
    /// on this field and may safely enumerate all variants at build time.
    pub code: DiagnosticCode,

    /// Pipeline stage. Routes the failure to the correct repair loop
    /// (parser vs generator vs I/O).
    pub stage: Stage,

    /// Specification reference that justifies the rule, when applicable.
    /// Enables LLM grounding against known documents.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec: Option<&'static str>,

    /// English, one-line. Not machine-parsed — kept for UI rendering
    /// and as a tiebreaker in ambiguous triage. Do **not** derive
    /// identity from this field; `id` is the stable identifier.
    pub message: String,

    /// Source location, present when the error was raised through a
    /// call-site that wrapped it in `Located<ForgeError>`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,

    /// Non-repair expectation metadata — parser-level expectations
    /// (e.g. "identifier") or cardinality constraints (e.g. "exactly
    /// one match"). Never holds a candidate list for substitution;
    /// that role belongs exclusively to `fix`. The two fields are
    /// disjoint by contract: a consumer that needs a repair signal
    /// reads `fix`, a consumer that needs to know what the producer
    /// was grammatically expecting reads `expected`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<Vec<String>>,

    /// Observed value that triggered the rejection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual: Option<String>,

    /// Structured repair proposal. The sole channel for repair
    /// signals — when populated, agents apply (or choose) based on
    /// the variant; when absent, no structured repair exists and
    /// there is no fallback to `expected`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<Fix>,
}

/// Pipeline stage taxonomy. Variant names mirror the stage comments
/// in `forge::error` and `mesh::error` so those modules stay in
/// lockstep with the wire format. `Cli` covers errors that originate
/// in the command-line driver itself (argument parsing, workspace
/// layout) rather than in a compiler pipeline.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Stage {
    Xml,
    Validation,
    Expression,
    Import,
    Manifest,
    Generate,
    Io,
    Cli,
    MeshDeploy,
    MeshExternal,
    MeshTopology,
    MeshCodegen,
}

impl Stage {
    /// Stable short name used in the content hash. Kept in sync with
    /// the serde `rename_all = "kebab-case"` output so JSON and hash
    /// agree on the canonical form.
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Stage::Xml => "xml",
            Stage::Validation => "validation",
            Stage::Expression => "expression",
            Stage::Import => "import",
            Stage::Manifest => "manifest",
            Stage::Generate => "generate",
            Stage::Io => "io",
            Stage::Cli => "cli",
            Stage::MeshDeploy => "mesh-deploy",
            Stage::MeshExternal => "mesh-external",
            Stage::MeshTopology => "mesh-topology",
            Stage::MeshCodegen => "mesh-codegen",
        }
    }
}

/// Closed set of diagnostic codes. Every `ForgeError` variant maps to
/// exactly one code; adding a new error variant forces adding a new
/// `DiagnosticCode` (compile-time exhaustiveness on the `to_fields`
/// match). Agents depending on SCE can enumerate this type to build
/// a complete dispatch table.
#[derive(Debug, Clone, Copy, Serialize)]
pub enum DiagnosticCode {
    #[serde(rename = "xml/parse")]
    XmlParse,
    #[serde(rename = "xml/schema-validation")]
    XmlSchemaValidation,

    #[serde(rename = "validation/missing-element")]
    ValidationMissingElement,
    #[serde(rename = "validation/missing-attribute")]
    ValidationMissingAttribute,
    #[serde(rename = "validation/invalid-attribute")]
    ValidationInvalidAttribute,
    #[serde(rename = "validation/unsupported-kind")]
    ValidationUnsupportedKind,
    #[serde(rename = "validation/duplicate-id")]
    ValidationDuplicateId,
    #[serde(rename = "validation/empty-collection")]
    ValidationEmptyCollection,
    #[serde(rename = "validation/count-mismatch")]
    ValidationCountMismatch,
    #[serde(rename = "validation/incompatible-attributes")]
    ValidationIncompatibleAttributes,
    #[serde(rename = "validation/invalid-reference")]
    ValidationInvalidReference,
    #[serde(rename = "validation/invalid-direction")]
    ValidationInvalidDirection,
    #[serde(rename = "validation/numeric-parse")]
    ValidationNumericParse,
    #[serde(rename = "validation/empty-value")]
    ValidationEmptyValue,
    #[serde(rename = "validation/singleton-violation")]
    ValidationSingletonViolation,
    #[serde(rename = "validation/require-either")]
    ValidationRequireEither,
    #[serde(rename = "validation/wrong-pipeline")]
    ValidationWrongPipeline,

    #[serde(rename = "expression/empty")]
    ExpressionEmpty,
    #[serde(rename = "expression/lex")]
    ExpressionLex,
    #[serde(rename = "expression/unsupported-construct")]
    ExpressionUnsupportedConstruct,
    #[serde(rename = "expression/strict-equality")]
    ExpressionStrictEquality,
    #[serde(rename = "expression/parse-mismatch")]
    ExpressionParseMismatch,
    #[serde(rename = "expression/unexpected-token")]
    ExpressionUnexpectedToken,
    #[serde(rename = "expression/invalid-lvalue")]
    ExpressionInvalidLvalue,
    #[serde(rename = "expression/type-coercion")]
    ExpressionTypeCoercion,
    #[serde(rename = "expression/go-ternary-unsupported")]
    ExpressionGoTernaryUnsupported,

    #[serde(rename = "import/file-not-found")]
    ImportFileNotFound,
    #[serde(rename = "import/kind-mismatch")]
    ImportKindMismatch,
    #[serde(rename = "import/not-forge")]
    ImportNotForge,
    #[serde(rename = "import/read-error")]
    ImportReadError,

    #[serde(rename = "manifest/circular-dependency")]
    ManifestCircularDependency,
    #[serde(rename = "manifest/io")]
    ManifestIo,

    #[serde(rename = "generate/invalid-config")]
    GenerateInvalidConfig,
    #[serde(rename = "generate/template-load")]
    GenerateTemplateLoad,
    #[serde(rename = "generate/template-render")]
    GenerateTemplateRender,

    #[serde(rename = "io/filesystem")]
    IoFilesystem,

    // ── CLI-level errors ─────────────────────────────────────
    #[serde(rename = "cli/unknown-language")]
    CliUnknownLanguage,
    #[serde(rename = "cli/unsupported-language")]
    CliUnsupportedLanguage,
    #[serde(rename = "cli/read-input")]
    CliReadInput,
    #[serde(rename = "cli/write-output")]
    CliWriteOutput,
    #[serde(rename = "cli/create-output-dir")]
    CliCreateOutputDir,
    #[serde(rename = "cli/scxml-parse")]
    CliScxmlParse,
    #[serde(rename = "cli/scxml-generate")]
    CliScxmlGenerate,
    #[serde(rename = "cli/dynamic-features")]
    CliDynamicFeatures,
    #[serde(rename = "cli/missing-metadata-field")]
    CliMissingMetadataField,
    #[serde(rename = "cli/not-a-directory")]
    CliNotADirectory,
    #[serde(rename = "cli/invalid-format-option")]
    CliInvalidFormatOption,
    #[serde(rename = "cli/json-serialization")]
    CliJsonSerialization,
    #[serde(rename = "cli/project-root-not-found")]
    CliProjectRootNotFound,
    #[serde(rename = "cli/format-style-not-found")]
    CliFormatStyleNotFound,
    #[serde(rename = "cli/no-scxml-tag")]
    CliNoScxmlTag,

    // ── Mesh pipeline ────────────────────────────────────────
    // Deploy stage
    #[serde(rename = "mesh/deploy-read")]
    MeshDeployRead,
    #[serde(rename = "mesh/deploy-parse")]
    MeshDeployParse,
    #[serde(rename = "mesh/deploy-unsupported-version")]
    MeshDeployUnsupportedVersion,
    #[serde(rename = "mesh/deploy-duplicate-machine")]
    MeshDeployDuplicateMachine,
    // External config stage
    #[serde(rename = "mesh/external-read")]
    MeshExternalRead,
    #[serde(rename = "mesh/external-parse")]
    MeshExternalParse,
    #[serde(rename = "mesh/external-unresolved-names")]
    MeshExternalUnresolvedNames,
    #[serde(rename = "mesh/external-ambiguous-event-group")]
    MeshExternalAmbiguousEventGroup,
    #[serde(rename = "mesh/external-empty-event-group")]
    MeshExternalEmptyEventGroup,
    #[serde(rename = "mesh/external-named-reference-without-config")]
    MeshExternalNamedReferenceWithoutConfig,
    #[serde(rename = "mesh/external-reserved-someip-id-keys")]
    MeshExternalReservedSomeipIdKeys,
    #[serde(rename = "mesh/external-someip-field-on-non-someip-transport")]
    MeshExternalSomeipFieldOnNonSomeipTransport,
    #[serde(rename = "mesh/external-conflicting-event-schema")]
    MeshExternalConflictingEventSchema,
    #[serde(rename = "mesh/external-conflicting-event-field-kinds")]
    MeshExternalConflictingEventFieldKinds,
    #[serde(rename = "mesh/external-empty-event-entry")]
    MeshExternalEmptyEventEntry,
    // Topology stage
    #[serde(rename = "mesh/topology-unresolved-targets")]
    MeshTopologyUnresolvedTargets,
    #[serde(rename = "mesh/topology-machine-not-found")]
    MeshTopologyMachineNotFound,
    #[serde(rename = "mesh/topology-receiver-not-declared")]
    MeshTopologyReceiverNotDeclared,
    #[serde(rename = "mesh/topology-absolute-source-path")]
    MeshTopologyAbsoluteSourcePath,
    #[serde(rename = "mesh/topology-receiver-source-read")]
    MeshTopologyReceiverSourceRead,
    #[serde(rename = "mesh/topology-receiver-source-parse")]
    MeshTopologyReceiverSourceParse,
    #[serde(rename = "mesh/topology-uncovered-events")]
    MeshTopologyUncoveredEvents,
    #[serde(rename = "mesh/topology-pattern-capability-violation")]
    MeshTopologyPatternCapabilityViolation,
    #[serde(rename = "mesh/topology-missing-binding-field")]
    MeshTopologyMissingBindingField,
    #[serde(rename = "mesh/topology-invalid-binding-field")]
    MeshTopologyInvalidBindingField,
    #[serde(rename = "mesh/topology-event-binding-unused")]
    MeshTopologyEventBindingUnused,
    // Codegen stage
    #[serde(rename = "mesh/codegen-unsupported-language")]
    MeshCodegenUnsupportedLanguage,
    #[serde(rename = "mesh/codegen-unsupported-transport")]
    MeshCodegenUnsupportedTransport,
    #[serde(rename = "mesh/codegen-template-read")]
    MeshCodegenTemplateRead,
    #[serde(rename = "mesh/codegen-template-render")]
    MeshCodegenTemplateRender,
    #[serde(rename = "mesh/codegen-event-name-collision")]
    MeshCodegenEventNameCollision,
    // Mesh I/O
    #[serde(rename = "mesh/io")]
    MeshIo,
}

impl Diagnostic {
    /// Construct a last-resort diagnostic for failures in the
    /// diagnostic pipeline itself — e.g. serde serialization error,
    /// OOM during `to_diagnostic`. Flowing even this path through
    /// the struct (instead of hand-building a JSON string) keeps the
    /// wire contract a single source of truth: schema bumps touch
    /// exactly one place.
    pub fn meta_failure(message: impl Into<String>) -> Self {
        let message = message.into();
        let id = compute_id(
            DiagnosticCode::IoFilesystem.as_str(),
            Stage::Io.as_str(),
            None,
            std::slice::from_ref(&message),
        );
        Diagnostic {
            schema_version: SCHEMA_VERSION,
            id,
            code: DiagnosticCode::IoFilesystem,
            stage: Stage::Io,
            spec: None,
            message,
            location: None,
            expected: None,
            actual: None,
            fix: None,
        }
    }
}

impl DiagnosticCode {
    /// Slash-path string form used in the content hash. Must match the
    /// serde `rename` on each variant exactly.
    pub(crate) fn as_str(&self) -> &'static str {
        use DiagnosticCode::*;
        match self {
            XmlParse => "xml/parse",
            XmlSchemaValidation => "xml/schema-validation",
            ValidationMissingElement => "validation/missing-element",
            ValidationMissingAttribute => "validation/missing-attribute",
            ValidationInvalidAttribute => "validation/invalid-attribute",
            ValidationUnsupportedKind => "validation/unsupported-kind",
            ValidationDuplicateId => "validation/duplicate-id",
            ValidationEmptyCollection => "validation/empty-collection",
            ValidationCountMismatch => "validation/count-mismatch",
            ValidationIncompatibleAttributes => "validation/incompatible-attributes",
            ValidationInvalidReference => "validation/invalid-reference",
            ValidationInvalidDirection => "validation/invalid-direction",
            ValidationNumericParse => "validation/numeric-parse",
            ValidationEmptyValue => "validation/empty-value",
            ValidationSingletonViolation => "validation/singleton-violation",
            ValidationRequireEither => "validation/require-either",
            ValidationWrongPipeline => "validation/wrong-pipeline",
            ExpressionEmpty => "expression/empty",
            ExpressionLex => "expression/lex",
            ExpressionUnsupportedConstruct => "expression/unsupported-construct",
            ExpressionStrictEquality => "expression/strict-equality",
            ExpressionParseMismatch => "expression/parse-mismatch",
            ExpressionUnexpectedToken => "expression/unexpected-token",
            ExpressionInvalidLvalue => "expression/invalid-lvalue",
            ExpressionTypeCoercion => "expression/type-coercion",
            ExpressionGoTernaryUnsupported => "expression/go-ternary-unsupported",
            ImportFileNotFound => "import/file-not-found",
            ImportKindMismatch => "import/kind-mismatch",
            ImportNotForge => "import/not-forge",
            ImportReadError => "import/read-error",
            ManifestCircularDependency => "manifest/circular-dependency",
            ManifestIo => "manifest/io",
            GenerateInvalidConfig => "generate/invalid-config",
            GenerateTemplateLoad => "generate/template-load",
            GenerateTemplateRender => "generate/template-render",
            IoFilesystem => "io/filesystem",
            CliUnknownLanguage => "cli/unknown-language",
            CliUnsupportedLanguage => "cli/unsupported-language",
            CliReadInput => "cli/read-input",
            CliWriteOutput => "cli/write-output",
            CliCreateOutputDir => "cli/create-output-dir",
            CliScxmlParse => "cli/scxml-parse",
            CliScxmlGenerate => "cli/scxml-generate",
            CliDynamicFeatures => "cli/dynamic-features",
            CliMissingMetadataField => "cli/missing-metadata-field",
            CliNotADirectory => "cli/not-a-directory",
            CliInvalidFormatOption => "cli/invalid-format-option",
            CliJsonSerialization => "cli/json-serialization",
            CliProjectRootNotFound => "cli/project-root-not-found",
            CliFormatStyleNotFound => "cli/format-style-not-found",
            CliNoScxmlTag => "cli/no-scxml-tag",
            MeshDeployRead => "mesh/deploy-read",
            MeshDeployParse => "mesh/deploy-parse",
            MeshDeployUnsupportedVersion => "mesh/deploy-unsupported-version",
            MeshDeployDuplicateMachine => "mesh/deploy-duplicate-machine",
            MeshExternalRead => "mesh/external-read",
            MeshExternalParse => "mesh/external-parse",
            MeshExternalUnresolvedNames => "mesh/external-unresolved-names",
            MeshExternalAmbiguousEventGroup => "mesh/external-ambiguous-event-group",
            MeshExternalEmptyEventGroup => "mesh/external-empty-event-group",
            MeshExternalNamedReferenceWithoutConfig => "mesh/external-named-reference-without-config",
            MeshExternalReservedSomeipIdKeys => "mesh/external-reserved-someip-id-keys",
            MeshExternalSomeipFieldOnNonSomeipTransport => "mesh/external-someip-field-on-non-someip-transport",
            MeshExternalConflictingEventSchema => "mesh/external-conflicting-event-schema",
            MeshExternalConflictingEventFieldKinds => "mesh/external-conflicting-event-field-kinds",
            MeshExternalEmptyEventEntry => "mesh/external-empty-event-entry",
            MeshTopologyUnresolvedTargets => "mesh/topology-unresolved-targets",
            MeshTopologyMachineNotFound => "mesh/topology-machine-not-found",
            MeshTopologyReceiverNotDeclared => "mesh/topology-receiver-not-declared",
            MeshTopologyAbsoluteSourcePath => "mesh/topology-absolute-source-path",
            MeshTopologyReceiverSourceRead => "mesh/topology-receiver-source-read",
            MeshTopologyReceiverSourceParse => "mesh/topology-receiver-source-parse",
            MeshTopologyUncoveredEvents => "mesh/topology-uncovered-events",
            MeshTopologyPatternCapabilityViolation => "mesh/topology-pattern-capability-violation",
            MeshTopologyMissingBindingField => "mesh/topology-missing-binding-field",
            MeshTopologyInvalidBindingField => "mesh/topology-invalid-binding-field",
            MeshTopologyEventBindingUnused => "mesh/topology-event-binding-unused",
            MeshCodegenUnsupportedLanguage => "mesh/codegen-unsupported-language",
            MeshCodegenUnsupportedTransport => "mesh/codegen-unsupported-transport",
            MeshCodegenTemplateRead => "mesh/codegen-template-read",
            MeshCodegenTemplateRender => "mesh/codegen-template-render",
            MeshCodegenEventNameCollision => "mesh/codegen-event-name-collision",
            MeshIo => "mesh/io",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Location {
    pub file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub col: Option<u32>,
}

/// Structured repair proposal.
///
/// `Fix` is the sole channel for repair signals: whenever the producer
/// knows how the document could be changed to satisfy the rejected
/// constraint, the payload is attached here. Agents therefore inspect
/// `fix` and `fix` only to drive repair — `expected` carries a
/// different kind of information (see `Diagnostic::expected`) and the
/// two fields never overlap.
///
/// The variant encodes the *shape* of the repair:
///
/// * Deterministic: `AddAttribute`, `RenameDuplicate`, `RemoveFields`,
///   `ReplaceWith` — applicable without further judgment.
/// * Choice-based: `ReplaceOneOf`, `AddOneOf` — the producer lists the
///   closed candidate set and the agent (or the human) picks.
///
/// `fix` is absent when no structured repair can be named — e.g. an
/// `Io` failure, a `generate/template-render` crash, or cardinality
/// violations that demand a redesign rather than a local edit. In
/// those cases `message` is the only remaining signal; there is no
/// fallback to `expected`.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Fix {
    /// Add a required attribute to the named element. Both `element`
    /// and `attr` are always known at construction time.
    AddAttribute { element: String, attr: String },

    /// Rename a duplicated identifier to something unique. `what`
    /// disambiguates the namespace (state id, event name, …).
    RenameDuplicate { what: String, id: String },

    /// Delete one or more fields at the given config location. Used
    /// where the producer can name *exactly* what is wrong (reserved
    /// keys, unused entries) and the repair is a single-direction
    /// removal. `location` is a dotted path (e.g.
    /// `"machines.x.bindings.y"`) and `fields` are the keys under it
    /// that must be removed.
    RemoveFields { location: String, fields: Vec<String> },

    /// Replace the offending value — carried in `actual` on the parent
    /// record — with `to`. Emitted only for errors whose repair is
    /// deterministic and single-valued: e.g. `==` → `===` under strict
    /// equality, or a mismatched `<sce:import kind=…>` where the
    /// imported file's real kind is the one correct answer.
    ReplaceWith { to: String },

    /// Replace the offending value — carried in `actual` — with one of
    /// the listed `candidates`. Emitted when the producer knows the
    /// closed enumeration of legal values (attribute value constraints,
    /// cross-reference resolution) but cannot deterministically pick a
    /// single answer. The agent must choose which candidate fits the
    /// surrounding context.
    ReplaceOneOf { candidates: Vec<String> },

    /// Add one of several legal attributes to the named element. Used
    /// for "require either X or Y" constraints (e.g. `<send>` needs
    /// `event` or `eventexpr`). The agent must choose which of `attrs`
    /// to emit based on surrounding context.
    AddOneOf { element: String, attrs: Vec<String> },
}

// ── Per-error-variant field extraction ─────────────────────────

/// Structured fields extracted from a single `ForgeError`. Collected
/// into a struct (instead of a big tuple) so the variant-mapping code
/// below reads as data rather than positional arguments.
struct DiagnosticFields {
    code: DiagnosticCode,
    stage: Stage,
    spec: Option<&'static str>,
    expected: Option<Vec<String>>,
    actual: Option<String>,
    fix: Option<Fix>,
    /// Canonical identifying payload for hashing. Distinct from
    /// `actual` — e.g. `MissingAttribute` identity is (element, attr),
    /// not any single value. Order matters: it is the canonical key.
    key_fragments: Vec<String>,
}

impl ToDiagnostics for ForgeError {
    fn exit_code(&self) -> i32 {
        ForgeError::exit_code(self)
    }

    /// Render this error into one or more machine-readable
    /// [`Diagnostic`]s with no source location. Used for error paths
    /// that bubble up before any frame with file/line context (e.g.
    /// pure I/O failures). XSD validation errors expand to one record
    /// per violation; every other variant returns a single record.
    fn to_diagnostics(&self) -> Vec<Diagnostic> {
        build_forge_diagnostics(self, None)
    }
}

impl ToDiagnostics for Located<ForgeError> {
    fn exit_code(&self) -> i32 {
        self.error.exit_code()
    }

    /// Render this located error into one or more machine-readable
    /// [`Diagnostic`]s carrying source location data. Preferred
    /// emission path — the location field is the single largest
    /// signal upstream agents use for repair routing, so reaching
    /// this impl (instead of the bare-`ForgeError` one) means a leaf
    /// call-site did its job. XSD validation errors ignore the outer
    /// `location` here: each inner `XsdDiag` already carries its own
    /// line from libxml2, which is strictly more precise.
    fn to_diagnostics(&self) -> Vec<Diagnostic> {
        build_forge_diagnostics(&self.error, Some(&self.location))
    }
}

/// Shared emission routine used by both `ForgeError` and
/// `Located<ForgeError>`. Returns a `Vec` because a single error can
/// represent many violations — XSD validation surfaces one record per
/// libxml2 diagnostic. All other variants return exactly one record.
///
/// Multi-record expansion for XSD is delegated to `XsdErrors`' own
/// `ToDiagnostics` impl, keeping the per-violation emission logic
/// next to the data that carries the line numbers.
fn build_forge_diagnostics(
    err: &ForgeError,
    location_ctx: Option<&SourceLocation>,
) -> Vec<Diagnostic> {
    if let ForgeError::Xml(XmlError::SchemaValidation(xsd_errors)) = err {
        return xsd_errors.to_diagnostics();
    }
    vec![build_single_forge_diagnostic(err, location_ctx)]
}

/// Render a non-XSD error as its single diagnostic record.
///
/// XSD validation errors must go through [`expand_xsd_diagnostics`]
/// instead — they carry per-violation line data that would be hidden
/// by a single-record emission.
fn build_single_forge_diagnostic(
    err: &ForgeError,
    location_ctx: Option<&SourceLocation>,
) -> Diagnostic {
    let fields = forge_error_fields(err);
    let location = location_ctx.map(|loc| Location {
        file: loc.file.clone(),
        line: loc.line,
        col: loc.col,
    });

    let id = compute_id(
        fields.code.as_str(),
        fields.stage.as_str(),
        location.as_ref().map(|l| l.file.as_str()),
        &fields.key_fragments,
    );

    Diagnostic {
        schema_version: SCHEMA_VERSION,
        id,
        code: fields.code,
        stage: fields.stage,
        spec: fields.spec,
        message: err.to_string(),
        location,
        expected: fields.expected,
        actual: fields.actual,
        fix: fields.fix,
    }
}


fn forge_error_fields(err: &ForgeError) -> DiagnosticFields {
    match err {
        ForgeError::Xml(e) => xml_fields(e),
        ForgeError::Validation(e) => validation_fields(e),
        ForgeError::Expression(e) => expression_fields(e),
        ForgeError::Import(e) => import_fields(e),
        ForgeError::Manifest(e) => manifest_fields(e),
        ForgeError::Generate(e) => generate_fields(e),
        ForgeError::Io { path, .. } => DiagnosticFields {
            code: DiagnosticCode::IoFilesystem,
            stage: Stage::Io,
            spec: None,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![path.display().to_string()],
        },
    }
}

fn xml_fields(e: &XmlError) -> DiagnosticFields {
    match e {
        XmlError::Parse(detail) => DiagnosticFields {
            code: DiagnosticCode::XmlParse,
            stage: Stage::Xml,
            spec: None,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![detail.clone()],
        },
        XmlError::SchemaValidation(_) => DiagnosticFields {
            code: DiagnosticCode::XmlSchemaValidation,
            stage: Stage::Xml,
            spec: Some("SCE Forge XSD"),
            expected: None,
            actual: None,
            fix: None,
            key_fragments: Vec::new(),
        },
    }
}

fn validation_fields(e: &ValidationError) -> DiagnosticFields {
    match e {
        ValidationError::MissingElement { kind, element } => DiagnosticFields {
            code: DiagnosticCode::ValidationMissingElement,
            stage: Stage::Validation,
            spec: None,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![kind.to_string(), element.clone()],
        },
        ValidationError::MissingAttribute { element, attr } => DiagnosticFields {
            code: DiagnosticCode::ValidationMissingAttribute,
            stage: Stage::Validation,
            spec: None,
            expected: None,
            actual: None,
            fix: Some(Fix::AddAttribute {
                element: element.clone(),
                attr: attr.clone(),
            }),
            key_fragments: vec![element.clone(), attr.clone()],
        },
        ValidationError::InvalidAttribute {
            element,
            attr,
            value,
            expected,
        } => DiagnosticFields {
            code: DiagnosticCode::ValidationInvalidAttribute,
            stage: Stage::Validation,
            spec: None,
            // Candidate list rides `fix`; `expected` stays None because
            // duplicating the list in both fields would violate the
            // fix/expected non-overlap rule in the contract.
            expected: None,
            actual: Some(value.clone()),
            fix: Some(Fix::ReplaceOneOf {
                candidates: split_expected(expected),
            }),
            key_fragments: vec![element.clone(), attr.clone(), value.clone()],
        },
        ValidationError::UnsupportedKind(value) => DiagnosticFields {
            code: DiagnosticCode::ValidationUnsupportedKind,
            stage: Stage::Validation,
            spec: None,
            expected: None,
            actual: Some(value.clone()),
            // `sce:kind` has a closed enumeration (`ForgeKind::from_attr`).
            // The list is authoritative, so agents get a structured
            // replace-one-of instead of message-prose parsing.
            fix: Some(Fix::ReplaceOneOf {
                candidates: crate::forge::model::ForgeKind::ALL_ATTR_NAMES
                    .iter()
                    .map(|s| (*s).to_string())
                    .collect(),
            }),
            key_fragments: vec![value.clone()],
        },
        ValidationError::DuplicateId { kind, what, id } => DiagnosticFields {
            code: DiagnosticCode::ValidationDuplicateId,
            stage: Stage::Validation,
            spec: None,
            expected: None,
            actual: Some(id.clone()),
            fix: Some(Fix::RenameDuplicate {
                what: what.clone(),
                id: id.clone(),
            }),
            key_fragments: vec![kind.to_string(), what.clone(), id.clone()],
        },
        ValidationError::EmptyCollection { kind, what } => DiagnosticFields {
            code: DiagnosticCode::ValidationEmptyCollection,
            stage: Stage::Validation,
            spec: None,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![kind.to_string(), what.clone()],
        },
        ValidationError::CountMismatch { kind, detail } => DiagnosticFields {
            code: DiagnosticCode::ValidationCountMismatch,
            stage: Stage::Validation,
            spec: None,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![kind.to_string(), detail.clone()],
        },
        ValidationError::IncompatibleAttributes { element, detail } => DiagnosticFields {
            code: DiagnosticCode::ValidationIncompatibleAttributes,
            stage: Stage::Validation,
            spec: None,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![element.clone(), detail.clone()],
        },
        ValidationError::InvalidReference {
            kind,
            name,
            what,
            available,
        } => DiagnosticFields {
            code: DiagnosticCode::ValidationInvalidReference,
            stage: Stage::Validation,
            spec: None,
            expected: None,
            actual: Some(name.clone()),
            fix: Some(Fix::ReplaceOneOf {
                candidates: split_expected(available),
            }),
            key_fragments: vec![kind.to_string(), what.clone(), name.clone()],
        },
        ValidationError::InvalidDirection {
            kind,
            direction,
            field,
        } => DiagnosticFields {
            code: DiagnosticCode::ValidationInvalidDirection,
            stage: Stage::Validation,
            spec: None,
            expected: None,
            actual: Some(direction.clone()),
            // Every forge kind with directional `<data>` fields accepts
            // exactly `input` and `output`; `internal` and everything
            // else are rejected. The candidate list is closed, so the
            // repair is expressible as a `ReplaceOneOf`.
            fix: Some(Fix::ReplaceOneOf {
                candidates: vec!["input".to_string(), "output".to_string()],
            }),
            key_fragments: vec![kind.to_string(), field.clone(), direction.clone()],
        },
        ValidationError::NumericParse {
            element,
            attr,
            value,
            ..
        } => DiagnosticFields {
            code: DiagnosticCode::ValidationNumericParse,
            stage: Stage::Validation,
            spec: None,
            expected: None,
            actual: Some(value.clone()),
            fix: None,
            key_fragments: vec![element.clone(), attr.clone(), value.clone()],
        },
        ValidationError::EmptyValue { element, attr } => DiagnosticFields {
            code: DiagnosticCode::ValidationEmptyValue,
            stage: Stage::Validation,
            spec: None,
            expected: None,
            actual: None,
            fix: Some(Fix::AddAttribute {
                element: element.clone(),
                attr: attr.clone(),
            }),
            key_fragments: vec![element.clone(), attr.clone()],
        },
        ValidationError::SingletonViolation { kind, attr } => DiagnosticFields {
            code: DiagnosticCode::ValidationSingletonViolation,
            stage: Stage::Validation,
            spec: None,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![kind.to_string(), attr.clone()],
        },
        ValidationError::RequireEither {
            element,
            alternatives,
        } => DiagnosticFields {
            code: DiagnosticCode::ValidationRequireEither,
            stage: Stage::Validation,
            spec: None,
            expected: None,
            actual: None,
            fix: Some(Fix::AddOneOf {
                element: element.clone(),
                attrs: alternatives.clone(),
            }),
            key_fragments: {
                let mut k = vec![element.clone()];
                k.extend(alternatives.iter().cloned());
                k
            },
        },
        ValidationError::WrongPipeline { kind } => DiagnosticFields {
            code: DiagnosticCode::ValidationWrongPipeline,
            stage: Stage::Validation,
            spec: None,
            expected: None,
            actual: Some(kind.to_string()),
            fix: None,
            key_fragments: vec![kind.to_string()],
        },
    }
}

fn expression_fields(e: &ExprError) -> DiagnosticFields {
    let subset_spec = Some("Extended SCXML stateless subset");
    match e {
        ExprError::Empty { what } => DiagnosticFields {
            code: DiagnosticCode::ExpressionEmpty,
            stage: Stage::Expression,
            spec: None,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![(*what).to_string()],
        },
        ExprError::Lex { position, detail } => DiagnosticFields {
            code: DiagnosticCode::ExpressionLex,
            stage: Stage::Expression,
            spec: None,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![position.to_string(), detail.clone()],
        },
        ExprError::UnsupportedConstruct { construct } => DiagnosticFields {
            code: DiagnosticCode::ExpressionUnsupportedConstruct,
            stage: Stage::Expression,
            spec: subset_spec,
            expected: None,
            actual: Some(construct.clone()),
            fix: None,
            key_fragments: vec![construct.clone()],
        },
        ExprError::StrictEquality { operator, strict } => DiagnosticFields {
            code: DiagnosticCode::ExpressionStrictEquality,
            stage: Stage::Expression,
            spec: subset_spec,
            // Single legal replacement (`==` → `===`, `!=` → `!==`).
            // It rides `fix` as a deterministic `ReplaceWith`;
            // duplicating it in `expected` would violate non-overlap.
            expected: None,
            actual: Some((*operator).to_string()),
            fix: Some(Fix::ReplaceWith { to: (*strict).to_string() }),
            key_fragments: vec![(*operator).to_string()],
        },
        ExprError::ParseMismatch { expected, got } => DiagnosticFields {
            code: DiagnosticCode::ExpressionParseMismatch,
            stage: Stage::Expression,
            spec: None,
            expected: Some(vec![expected.clone()]),
            actual: Some(got.clone()),
            fix: None,
            key_fragments: vec![expected.clone(), got.clone()],
        },
        ExprError::UnexpectedToken { token } => DiagnosticFields {
            code: DiagnosticCode::ExpressionUnexpectedToken,
            stage: Stage::Expression,
            spec: None,
            expected: None,
            actual: Some(token.clone()),
            fix: None,
            key_fragments: vec![token.clone()],
        },
        ExprError::InvalidLvalue { location, detail } => DiagnosticFields {
            code: DiagnosticCode::ExpressionInvalidLvalue,
            stage: Stage::Expression,
            spec: None,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![location.clone(), detail.clone()],
        },
        ExprError::TypeCoercion { lang, detail } => DiagnosticFields {
            code: DiagnosticCode::ExpressionTypeCoercion,
            stage: Stage::Expression,
            spec: None,
            expected: None,
            actual: Some((*lang).to_string()),
            fix: None,
            key_fragments: vec![(*lang).to_string(), detail.clone()],
        },
        ExprError::GoTernary => DiagnosticFields {
            code: DiagnosticCode::ExpressionGoTernaryUnsupported,
            stage: Stage::Expression,
            spec: None,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: Vec::new(),
        },
    }
}

fn import_fields(e: &ImportError) -> DiagnosticFields {
    match e {
        ImportError::FileNotFound { src, .. } => DiagnosticFields {
            code: DiagnosticCode::ImportFileNotFound,
            stage: Stage::Import,
            spec: None,
            expected: None,
            actual: Some(src.clone()),
            fix: None,
            key_fragments: vec![src.clone()],
        },
        ImportError::KindMismatch {
            src,
            declared,
            actual,
        } => DiagnosticFields {
            code: DiagnosticCode::ImportKindMismatch,
            stage: Stage::Import,
            spec: None,
            // Single deterministic replacement: rewrite
            // `<sce:import kind="…">` to the imported file's real
            // kind. `fix.to` is authoritative; `expected` stays None.
            expected: None,
            actual: Some(declared.clone()),
            fix: Some(Fix::ReplaceWith { to: actual.clone() }),
            key_fragments: vec![src.clone(), declared.clone(), actual.clone()],
        },
        ImportError::NotForge { src } => DiagnosticFields {
            code: DiagnosticCode::ImportNotForge,
            stage: Stage::Import,
            spec: None,
            expected: None,
            actual: Some(src.clone()),
            fix: None,
            key_fragments: vec![src.clone()],
        },
        ImportError::ReadError { src, .. } => DiagnosticFields {
            code: DiagnosticCode::ImportReadError,
            stage: Stage::Import,
            spec: None,
            expected: None,
            actual: Some(src.clone()),
            fix: None,
            key_fragments: vec![src.clone()],
        },
    }
}

fn manifest_fields(e: &ManifestError) -> DiagnosticFields {
    match e {
        ManifestError::CircularDependency(cycle) => DiagnosticFields {
            code: DiagnosticCode::ManifestCircularDependency,
            stage: Stage::Manifest,
            spec: None,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: cycle.clone(),
        },
        ManifestError::Io { context, .. } => DiagnosticFields {
            code: DiagnosticCode::ManifestIo,
            stage: Stage::Manifest,
            spec: None,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![context.clone()],
        },
    }
}

fn generate_fields(e: &GenerateError) -> DiagnosticFields {
    match e {
        GenerateError::InvalidConfig(detail) => DiagnosticFields {
            code: DiagnosticCode::GenerateInvalidConfig,
            stage: Stage::Generate,
            spec: None,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![detail.clone()],
        },
        GenerateError::TemplateLoad(detail) => DiagnosticFields {
            code: DiagnosticCode::GenerateTemplateLoad,
            stage: Stage::Generate,
            spec: None,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![detail.clone()],
        },
        GenerateError::TemplateRender(detail) => DiagnosticFields {
            code: DiagnosticCode::GenerateTemplateRender,
            stage: Stage::Generate,
            spec: None,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![detail.clone()],
        },
    }
}

// ── Helpers ────────────────────────────────────────────────────

/// Split a human-readable "expected" list ("foo, bar | baz") into a
/// vector of individual tokens. Several validation errors carry this
/// as free text for the Display impl; agents need it structured.
fn split_expected(s: &str) -> Vec<String> {
    s.split(|c: char| c == ',' || c == '|')
        .map(|part| part.trim().to_string())
        .filter(|part| !part.is_empty())
        .collect()
}

/// Content-addressed id over *semantic* fields only.
///
/// The canonical key is:
///
/// ```text
/// code-str | stage-str | file-or-empty | key-fragments-with-unit-sep
/// ```
///
/// Notably excludes the Display message — rewording a thiserror
/// `#[error(...)]` template must not change the id of the underlying
/// semantic error. Includes the source file when `Located` is present
/// so the same error reported on two files is distinguishable.
pub(crate) fn compute_id(
    code: &str,
    stage: &str,
    file: Option<&str>,
    key_fragments: &[String],
) -> String {
    let mut hasher = Fnv1a64::new();
    hasher.write(code.as_bytes());
    hasher.write(b"|");
    hasher.write(stage.as_bytes());
    hasher.write(b"|");
    hasher.write(file.unwrap_or("").as_bytes());
    for frag in key_fragments {
        // ASCII unit separator disambiguates fragment boundaries so
        // e.g. ["ab", "c"] and ["a", "bc"] hash differently.
        hasher.write(&[0x1f]);
        hasher.write(frag.as_bytes());
    }
    format!("fnv1a:{:016x}", hasher.finish())
}

/// Minimal FNV-1a 64-bit. Stable across Rust versions (unlike
/// `DefaultHasher`), so ids are reproducible indefinitely.
struct Fnv1a64(u64);
impl Fnv1a64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    fn new() -> Self {
        Self(Self::OFFSET)
    }
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 ^= b as u64;
            self.0 = self.0.wrapping_mul(Self::PRIME);
        }
    }
    fn finish(&self) -> u64 {
        self.0
    }
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::model::ForgeKind;

    fn missing_attr(attr: &str) -> ForgeError {
        ValidationError::MissingAttribute {
            element: "sce:field".into(),
            attr: attr.into(),
        }
        .into()
    }

    /// Extract the single diagnostic from any `ToDiagnostics`. Panics
    /// if the error produced zero or multiple records — tests that
    /// target multi-record paths (XSD) call `.to_diagnostics()`
    /// directly instead.
    fn single(err: &impl ToDiagnostics) -> Diagnostic {
        let mut v = err.to_diagnostics();
        assert_eq!(v.len(), 1, "expected single diagnostic, got {}", v.len());
        v.pop().unwrap()
    }

    #[test]
    fn id_is_stable_across_calls() {
        let err = missing_attr("id");
        assert_eq!(single(&err).id, single(&err).id);
        assert!(single(&err).id.starts_with("fnv1a:"));
    }

    #[test]
    fn id_distinguishes_semantic_payload() {
        // Same variant, different attr → different id.
        assert_ne!(
            single(&missing_attr("id")).id,
            single(&missing_attr("type")).id
        );
    }

    #[test]
    fn id_is_independent_of_prose_message() {
        // `compute_id` never sees the Display string. Proxy check:
        // the hash must be reproducible purely from (code, stage,
        // file, key_fragments), so invoking `compute_id` directly
        // with those fields yields the same id as `to_diagnostic`.
        let err = missing_attr("id");
        let d = single(&err);
        let expected = compute_id(
            DiagnosticCode::ValidationMissingAttribute.as_str(),
            Stage::Validation.as_str(),
            None,
            &["sce:field".to_string(), "id".to_string()],
        );
        assert_eq!(d.id, expected);
    }

    #[test]
    fn located_fills_location_and_shifts_id() {
        let bare = missing_attr("id");
        let located = Located::new(missing_attr("id"), "checkout.scxml", Some(42), None);

        let d_bare = single(&bare);
        let d_loc = single(&located);

        assert!(d_bare.location.is_none());
        let loc = d_loc.location.as_ref().expect("location populated");
        assert_eq!(loc.file, "checkout.scxml");
        assert_eq!(loc.line, Some(42));

        // Same semantic error, different location → different id.
        assert_ne!(d_bare.id, d_loc.id);
    }

    #[test]
    fn located_preserves_stage_and_exit_code() {
        let err = Located::new(missing_attr("id"), "x.scxml", None, None);
        assert_eq!(err.exit_code(), 3); // Validation
        let d = single(&err);
        assert!(matches!(d.stage, Stage::Validation));
        assert!(matches!(d.code, DiagnosticCode::ValidationMissingAttribute));
    }

    #[test]
    fn code_serializes_as_slash_path() {
        let err = missing_attr("id");
        let json = serde_json::to_string(&single(&err)).unwrap();
        assert!(json.contains("\"code\":\"validation/missing-attribute\""));
    }

    #[test]
    fn invalid_attribute_emits_replace_one_of_fix() {
        let err: ForgeError = ValidationError::InvalidAttribute {
            element: "sce:field".into(),
            attr: "sce:type".into(),
            value: "blob".into(),
            expected: "u8, u16, u32".into(),
        }
        .into();
        let d = single(&err);
        assert_eq!(d.actual.as_deref(), Some("blob"));
        // Non-overlap rule: the candidate list rides only `fix`.
        // `expected` must stay None so consumers do not see the same
        // data in two different shapes.
        assert!(d.expected.is_none(), "expected must not duplicate fix.candidates");
        let candidates = ["u8".to_string(), "u16".to_string(), "u32".to_string()];
        match d.fix {
            Some(Fix::ReplaceOneOf { candidates: got }) => assert_eq!(got, candidates),
            other => panic!("expected ReplaceOneOf, got {other:?}"),
        }
    }

    #[test]
    fn invalid_reference_emits_replace_one_of_fix() {
        let err: ForgeError = ValidationError::InvalidReference {
            kind: ForgeKind::Statechart,
            what: "transition target".into(),
            name: "missing".into(),
            available: "armed, disarmed".into(),
        }
        .into();
        let d = single(&err);
        assert_eq!(d.actual.as_deref(), Some("missing"));
        let candidates = ["armed".to_string(), "disarmed".to_string()];
        match d.fix {
            Some(Fix::ReplaceOneOf { candidates: got }) => assert_eq!(got, candidates),
            other => panic!("expected ReplaceOneOf, got {other:?}"),
        }
    }

    #[test]
    fn require_either_emits_add_one_of_fix() {
        let err: ForgeError = ValidationError::RequireEither {
            element: "send".into(),
            alternatives: vec!["event".into(), "eventexpr".into()],
        }
        .into();
        let d = single(&err);
        assert!(d.actual.is_none(), "RequireEither has no observed value");
        match d.fix {
            Some(Fix::AddOneOf { element, attrs }) => {
                assert_eq!(element, "send");
                assert_eq!(attrs, vec!["event".to_string(), "eventexpr".to_string()]);
            }
            other => panic!("expected AddOneOf, got {other:?}"),
        }
    }

    #[test]
    fn missing_attribute_emits_add_attribute_fix() {
        let err: ForgeError = ValidationError::MissingAttribute {
            element: "sce:field".into(),
            attr: "id".into(),
        }
        .into();
        let d = single(&err);
        match d.fix {
            Some(Fix::AddAttribute { element, attr }) => {
                assert_eq!(element, "sce:field");
                assert_eq!(attr, "id");
            }
            other => panic!("expected AddAttribute, got {other:?}"),
        }
    }

    #[test]
    fn serialization_is_single_line_ndjson_shape() {
        let err: ForgeError = ValidationError::DuplicateId {
            kind: ForgeKind::Statechart,
            what: "state id".into(),
            id: "armed".into(),
        }
        .into();
        let json = serde_json::to_string(&single(&err)).unwrap();
        assert!(!json.contains('\n'), "NDJSON records must be single-line");
        assert!(json.starts_with('{'));
        assert!(json.ends_with('}'));
    }

    #[test]
    fn skip_serializing_if_keeps_wire_format_tight() {
        // A bare Io error has no expected/actual/location/fix/spec.
        // None of those keys should appear on the wire.
        let err = ForgeError::Io {
            path: std::path::PathBuf::from("/tmp/x"),
            source: std::io::Error::from(std::io::ErrorKind::NotFound),
        };
        let json = serde_json::to_string(&single(&err)).unwrap();
        for absent in [
            "\"expected\"",
            "\"actual\"",
            "\"location\"",
            "\"fix\"",
            "\"spec\"",
        ] {
            assert!(!json.contains(absent), "unexpected key in: {json}");
        }
    }

    /// Byte-stable goldens: each error variant listed here produces
    /// the exact JSON string pinned in the table. A byte mismatch
    /// means a consumer that dedup'd on `id` yesterday now sees a
    /// different record for the same semantic error — a wire-format
    /// regression. Update this table deliberately (alongside a
    /// schema-version bump, when appropriate), never silently.
    #[test]
    fn diagnostic_goldens_are_byte_stable() {
        use crate::mesh::error::{MeshError, TopologyError};
        use crate::mesh::target::TargetId;

        // Each entry: (label, actual_json, expected_golden). Entries
        // are evaluated eagerly so all mismatches surface in one
        // failure report — no drip-feed debugging.
        let actual_forge = |e: ForgeError| serde_json::to_string(&single(&e)).unwrap();
        let actual_mesh = |e: MeshError| serde_json::to_string(&single(&e)).unwrap();

        let cases: Vec<(&str, String, &str)> = vec![
            (
                "forge/missing-attribute",
                actual_forge(
                    ValidationError::MissingAttribute {
                        element: "sce:field".into(),
                        attr: "id".into(),
                    }
                    .into(),
                ),
                r#"{"v":1,"id":"fnv1a:1c56b923b2b2b87f","code":"validation/missing-attribute","stage":"validation","message":"sce:field must have an 'id' attribute","fix":{"kind":"add_attribute","element":"sce:field","attr":"id"}}"#,
            ),
            (
                "forge/invalid-attribute",
                actual_forge(
                    ValidationError::InvalidAttribute {
                        element: "sce:field".into(),
                        attr: "sce:type".into(),
                        value: "blob".into(),
                        expected: "u8, u16, u32".into(),
                    }
                    .into(),
                ),
                r#"{"v":1,"id":"fnv1a:dd04a37de468ffb4","code":"validation/invalid-attribute","stage":"validation","message":"sce:field: unknown sce:type value 'blob' (expected: u8, u16, u32)","actual":"blob","fix":{"kind":"replace_one_of","candidates":["u8","u16","u32"]}}"#,
            ),
            (
                "forge/invalid-reference",
                actual_forge(
                    ValidationError::InvalidReference {
                        kind: ForgeKind::Statechart,
                        what: "transition target".into(),
                        name: "missing".into(),
                        available: "armed, disarmed".into(),
                    }
                    .into(),
                ),
                r#"{"v":1,"id":"fnv1a:2e4c02e2b0e7e383","code":"validation/invalid-reference","stage":"validation","message":"statechart: missing does not match any transition target (available: armed, disarmed)","actual":"missing","fix":{"kind":"replace_one_of","candidates":["armed","disarmed"]}}"#,
            ),
            (
                "forge/require-either",
                actual_forge(
                    ValidationError::RequireEither {
                        element: "send".into(),
                        alternatives: vec!["event".into(), "eventexpr".into()],
                    }
                    .into(),
                ),
                r#"{"v":1,"id":"fnv1a:e10a747be1752ef3","code":"validation/require-either","stage":"validation","message":"send must have at least one of: event, eventexpr","fix":{"kind":"add_one_of","element":"send","attrs":["event","eventexpr"]}}"#,
            ),
            (
                "forge/go-ternary",
                actual_forge(ExprError::GoTernary.into()),
                r#"{"v":1,"id":"fnv1a:ef5b56dbf74b8718","code":"expression/go-ternary-unsupported","stage":"expression","message":"cannot transpile ternary expression to Go: Go has no conditional expression"}"#,
            ),
            (
                "forge/not-forge",
                actual_forge(ImportError::NotForge { src: "neighbour.scxml".into() }.into()),
                r#"{"v":1,"id":"fnv1a:7cec8a8357830a5a","code":"import/not-forge","stage":"import","message":"<sce:import src=\"neighbour.scxml\">: not a forge document (no sce:kind)","actual":"neighbour.scxml"}"#,
            ),
            (
                "forge/strict-equality",
                actual_forge(ExprError::StrictEquality { operator: "==", strict: "===" }.into()),
                r#"{"v":1,"id":"fnv1a:056d2165b00f16bd","code":"expression/strict-equality","stage":"expression","spec":"Extended SCXML stateless subset","message":"loose == is not permitted in Extended SCXML. Use === instead.","actual":"==","fix":{"kind":"replace_with","to":"==="}}"#,
            ),
            (
                "forge/import-kind-mismatch",
                actual_forge(
                    ImportError::KindMismatch {
                        src: "peer.scxml".into(),
                        declared: "validator".into(),
                        actual: "codec".into(),
                    }
                    .into(),
                ),
                r#"{"v":1,"id":"fnv1a:5f500ed01d12c1bb","code":"import/kind-mismatch","stage":"import","message":"<sce:import src=\"peer.scxml\" kind=\"validator\">: actual kind is 'codec' (mismatch)","actual":"validator","fix":{"kind":"replace_with","to":"codec"}}"#,
            ),
            (
                "mesh/missing-binding-field",
                actual_mesh(
                    TopologyError::MissingBindingField {
                        machine: "ecu_a".into(),
                        target: TargetId::new("#motor").unwrap(),
                        transport: "someip".into(),
                        field: "service".into(),
                    }
                    .into(),
                ),
                r#"{"v":1,"id":"fnv1a:d7ba280d1556705e","code":"mesh/topology-missing-binding-field","stage":"mesh-topology","message":"machine 'ecu_a': binding for '#motor' (transport: someip) is missing required field 'service'. Add 'service:' to the binding in deploy.yaml.","fix":{"kind":"add_attribute","element":"machines.ecu_a.bindings.#motor","attr":"service"}}"#,
            ),
            (
                "mesh/event-binding-unused",
                actual_mesh(
                    TopologyError::EventBindingUnused {
                        machine: "ecu_a".into(),
                        target: TargetId::new("#motor").unwrap(),
                        event: "legacy.ping".into(),
                    }
                    .into(),
                ),
                r#"{"v":1,"id":"fnv1a:4fdd02e5a9781de8","code":"mesh/topology-event-binding-unused","stage":"mesh-topology","message":"machine 'ecu_a': binding '#motor' declares events.legacy.ping in deploy.yaml, but the SCXML model never sends 'legacy.ping' to this target. Remove the unused entry, or correct the event name.","actual":"legacy.ping","fix":{"kind":"remove_fields","location":"machines.ecu_a.bindings.#motor.events","fields":["legacy.ping"]}}"#,
            ),
        ];

        let mismatches: Vec<String> = cases
            .iter()
            .filter(|(_, actual, golden)| actual != golden)
            .map(|(label, actual, golden)| {
                format!("\n[{label}]\nexpected: {golden}\n  actual: {actual}")
            })
            .collect();
        assert!(
            mismatches.is_empty(),
            "byte-stable goldens drifted:\n{}\n\nIf this change is intentional, update the table AND bump SCHEMA_VERSION if the shape changed.",
            mismatches.join("\n")
        );
    }

    // ── Non-overlap invariant: exhaustive contract table ──────────
    //
    // `fix` and `expected` are disjoint by contract (SCE_ERROR_CONTRACT.md
    // §3.2). The enforcement is a two-layer check:
    //
    //   Layer 1 (compile-time): `non_overlap_class()` exhaustively
    //     classifies every `DiagnosticCode` into one of three buckets.
    //     Adding a new code without placing it into the match fails
    //     the build — contributors cannot sidestep the invariant.
    //
    //   Layer 2 (runtime): `fix_carries_candidates_emitters_obey_non_overlap`
    //     and `expected_is_metadata_emitters_obey_non_overlap` construct
    //     one sample per code in the non-trivial buckets and verify
    //     that actual emission agrees with classification.
    //
    // Together these guarantee: every diagnostic ever emitted satisfies
    // the invariant, and any new variant forces an explicit decision.

    /// Bucket for the non-overlap rule. Distinct from the `Fix?` column
    /// in the contract's code catalog — the classes track invariant
    /// shape, not which fix variant is used.
    #[derive(Debug, PartialEq)]
    enum NonOverlapClass {
        /// Emits `fix` as `ReplaceOneOf` / `AddOneOf`. `expected` must
        /// be absent; the candidate list rides `fix` alone.
        FixCarriesCandidates,
        /// Emits `expected` as non-repair metadata (parser expectation,
        /// cardinality). `fix` must be absent; the producer cannot name
        /// a structured repair.
        ExpectedIsMetadata,
        /// Either emits a deterministic fix (`AddAttribute`, `ReplaceWith`,
        /// …) or no fix at all. Either way, `expected` must be absent —
        /// non-overlap still applies, but neither field carries
        /// candidate lists.
        NeutralOrDeterministic,
    }

    fn non_overlap_class(code: DiagnosticCode) -> NonOverlapClass {
        use DiagnosticCode::*;
        use NonOverlapClass::*;
        // Exhaustive match — adding a new DiagnosticCode fails the
        // build until it is placed into one of the three buckets.
        match code {
            // ── Fix carries a closed candidate / attr list ─────
            ValidationInvalidAttribute
            | ValidationInvalidReference
            | ValidationRequireEither
            | ValidationUnsupportedKind
            | ValidationInvalidDirection
            | MeshDeployUnsupportedVersion
            | MeshTopologyMachineNotFound
            | MeshCodegenUnsupportedLanguage
            | MeshCodegenUnsupportedTransport
            | CliUnknownLanguage
            | CliInvalidFormatOption => FixCarriesCandidates,

            // ── `expected` carries non-repair metadata ────────
            ExpressionParseMismatch | MeshExternalAmbiguousEventGroup => ExpectedIsMetadata,

            // ── Deterministic fix or no fix; expected=None ────
            XmlParse
            | XmlSchemaValidation
            | ValidationMissingElement
            | ValidationMissingAttribute
            | ValidationDuplicateId
            | ValidationEmptyCollection
            | ValidationCountMismatch
            | ValidationIncompatibleAttributes
            | ValidationNumericParse
            | ValidationEmptyValue
            | ValidationSingletonViolation
            | ValidationWrongPipeline
            | ExpressionEmpty
            | ExpressionLex
            | ExpressionUnsupportedConstruct
            | ExpressionStrictEquality
            | ExpressionUnexpectedToken
            | ExpressionInvalidLvalue
            | ExpressionTypeCoercion
            | ExpressionGoTernaryUnsupported
            | ImportFileNotFound
            | ImportKindMismatch
            | ImportNotForge
            | ImportReadError
            | ManifestCircularDependency
            | ManifestIo
            | GenerateInvalidConfig
            | GenerateTemplateLoad
            | GenerateTemplateRender
            | IoFilesystem
            | CliUnsupportedLanguage
            | CliReadInput
            | CliWriteOutput
            | CliCreateOutputDir
            | CliScxmlParse
            | CliScxmlGenerate
            | CliDynamicFeatures
            | CliMissingMetadataField
            | CliNotADirectory
            | CliJsonSerialization
            | CliProjectRootNotFound
            | CliFormatStyleNotFound
            | CliNoScxmlTag
            | MeshDeployRead
            | MeshDeployParse
            | MeshDeployDuplicateMachine
            | MeshExternalRead
            | MeshExternalParse
            | MeshExternalUnresolvedNames
            | MeshExternalEmptyEventGroup
            | MeshExternalNamedReferenceWithoutConfig
            | MeshExternalReservedSomeipIdKeys
            | MeshExternalSomeipFieldOnNonSomeipTransport
            | MeshExternalConflictingEventSchema
            | MeshExternalConflictingEventFieldKinds
            | MeshExternalEmptyEventEntry
            | MeshTopologyUnresolvedTargets
            | MeshTopologyReceiverNotDeclared
            | MeshTopologyAbsoluteSourcePath
            | MeshTopologyReceiverSourceRead
            | MeshTopologyReceiverSourceParse
            | MeshTopologyUncoveredEvents
            | MeshTopologyPatternCapabilityViolation
            | MeshTopologyMissingBindingField
            | MeshTopologyInvalidBindingField
            | MeshTopologyEventBindingUnused
            | MeshCodegenTemplateRead
            | MeshCodegenTemplateRender
            | MeshCodegenEventNameCollision
            | MeshIo => NeutralOrDeterministic,
        }
    }

    /// Every code classified as `FixCarriesCandidates` must emit a
    /// `ReplaceOneOf` or `AddOneOf` fix with no `expected`. CLI codes
    /// live in the binary so they are covered structurally by the
    /// `Diagnostic { expected: None, ... }` literal in
    /// `bin/sce_codegen.rs` — runtime spawning happens in
    /// `tests/error_format_json.rs`.
    #[test]
    fn fix_carries_candidates_emitters_obey_non_overlap() {
        use crate::mesh::error::{
            CodegenError as MeshCodegen, DeployError, MeshError, TopologyError,
        };

        let forge_samples: Vec<ForgeError> = vec![
            ValidationError::InvalidAttribute {
                element: "sce:field".into(),
                attr: "sce:type".into(),
                value: "blob".into(),
                expected: "u8, u16, u32".into(),
            }
            .into(),
            ValidationError::InvalidReference {
                kind: ForgeKind::Statechart,
                what: "transition target".into(),
                name: "missing".into(),
                available: "armed, disarmed".into(),
            }
            .into(),
            ValidationError::RequireEither {
                element: "send".into(),
                alternatives: vec!["event".into(), "eventexpr".into()],
            }
            .into(),
            ValidationError::UnsupportedKind("bogus".into()).into(),
            ValidationError::InvalidDirection {
                kind: ForgeKind::Transform,
                direction: "internal".into(),
                field: "input".into(),
            }
            .into(),
        ];

        let mesh_samples: Vec<MeshError> = vec![
            DeployError::UnsupportedVersion {
                found: "99".into(),
                supported: vec!["1"],
            }
            .into(),
            TopologyError::MachineNotFound {
                machine: "ecu_z".into(),
                available: vec!["ecu_a".into(), "ecu_b".into()],
            }
            .into(),
            MeshCodegen::UnsupportedLanguage("ruby".into()).into(),
            MeshCodegen::UnsupportedTransport {
                transport: "carrier_pigeon".into(),
                target: crate::mesh::target::TargetId::new("#motor").unwrap(),
            }
            .into(),
        ];

        let all: Vec<Diagnostic> = forge_samples
            .into_iter()
            .map(|e| single(&e))
            .chain(mesh_samples.into_iter().map(|e| single(&e)))
            .collect();

        for d in &all {
            assert_eq!(
                non_overlap_class(d.code),
                NonOverlapClass::FixCarriesCandidates,
                "test sample for {:?} is not in the FixCarriesCandidates bucket",
                d.code
            );
            assert!(
                matches!(
                    &d.fix,
                    Some(Fix::ReplaceOneOf { .. }) | Some(Fix::AddOneOf { .. })
                ),
                "{:?}: fix must be ReplaceOneOf or AddOneOf, got {:?}",
                d.code,
                d.fix
            );
            assert!(
                d.expected.is_none(),
                "{:?}: non-overlap violated — expected must be absent when fix carries candidates",
                d.code
            );
        }
    }

    /// Every code classified as `ExpectedIsMetadata` must emit
    /// `expected` with no `fix`. The producer has no structured repair
    /// to propose; the field documents what was grammatically expected
    /// or what cardinality rule was broken.
    #[test]
    fn expected_is_metadata_emitters_obey_non_overlap() {
        use crate::mesh::error::{ExternalConfigError, MeshError};

        let forge_samples: Vec<ForgeError> = vec![
            ExprError::ParseMismatch {
                expected: "identifier".into(),
                got: ";".into(),
            }
            .into(),
        ];

        let mesh_samples: Vec<MeshError> = vec![
            ExternalConfigError::AmbiguousEventGroup {
                machine: "ecu_a".into(),
                target: "#motor".into(),
                event_group: "overspeed".into(),
                count: 3,
                config_path: "vsomeip.json".into(),
            }
            .into(),
        ];

        let all: Vec<Diagnostic> = forge_samples
            .into_iter()
            .map(|e| single(&e))
            .chain(mesh_samples.into_iter().map(|e| single(&e)))
            .collect();

        for d in &all {
            assert_eq!(
                non_overlap_class(d.code),
                NonOverlapClass::ExpectedIsMetadata,
                "test sample for {:?} is not in the ExpectedIsMetadata bucket",
                d.code
            );
            assert!(
                d.expected.is_some(),
                "{:?}: expected must be populated with metadata",
                d.code
            );
            assert!(
                d.fix.is_none(),
                "{:?}: non-overlap violated — fix must be absent when expected carries metadata",
                d.code
            );
        }
    }

    #[test]
    fn stage_str_matches_serde_output() {
        // The hash uses `Stage::as_str()`; the wire uses serde's
        // `rename_all = "lowercase"`. They must agree.
        for stage in [
            Stage::Xml,
            Stage::Validation,
            Stage::Expression,
            Stage::Import,
            Stage::Manifest,
            Stage::Generate,
            Stage::Io,
        ] {
            let via_serde = serde_json::to_string(&stage).unwrap();
            let expected = format!("\"{}\"", stage.as_str());
            assert_eq!(via_serde, expected, "stage {stage:?} str vs serde disagree");
        }
    }
}
