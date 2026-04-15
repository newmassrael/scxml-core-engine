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
    #[serde(rename = "validation/duplicate-context-object")]
    ValidationDuplicateContextObject,
    #[serde(rename = "validation/empty-collection")]
    ValidationEmptyCollection,
    #[serde(rename = "validation/count-mismatch")]
    ValidationCountMismatch,
    #[serde(rename = "validation/incompatible-attributes")]
    ValidationIncompatibleAttributes,
    #[serde(rename = "validation/missing-context")]
    ValidationMissingContext,
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
    #[serde(rename = "validation/dynamic-features")]
    ValidationDynamicFeatures,

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

/// Canonical enumeration of every `DiagnosticCode` variant.
///
/// The enum is the declarative source of truth; this slice is the
/// runtime counterpart needed by tests that iterate all variants
/// (serde/as_str parity, JSON-Schema drift guard). Without a derive
/// macro (strum) Rust cannot enumerate enum variants, so the slice
/// is hand-maintained.
///
/// Drift protection is layered:
///
///   1. Adding a new variant forces compile errors in three
///      exhaustive matches — [`DiagnosticCode::as_str`],
///      [`DiagnosticCode::spec_anchor`], and (in the test module)
///      `non_overlap_class`. Contributors must touch each one.
///   2. The JSON-Schema enum in
///      `schemas/sce-diagnostic.v1.schema.json` duplicates this
///      list verbatim, with byte-for-byte drift caught by
///      `json_schema_enums_match_rust_source_of_truth`. Any slice
///      shrinkage or omission surfaces as a schema/slice mismatch,
///      not a silent test pass.
///
/// Entries are ordered by pipeline stage, matching the enum's source
/// order so a `diff` against the enum definition reveals any gap.
#[cfg(test)]
pub(crate) const ALL_DIAGNOSTIC_CODES: &[DiagnosticCode] = {
    use DiagnosticCode::*;
    &[
        // Xml
        XmlParse,
        XmlSchemaValidation,
        // Validation
        ValidationMissingElement,
        ValidationMissingAttribute,
        ValidationInvalidAttribute,
        ValidationUnsupportedKind,
        ValidationDuplicateId,
        ValidationDuplicateContextObject,
        ValidationEmptyCollection,
        ValidationCountMismatch,
        ValidationIncompatibleAttributes,
        ValidationMissingContext,
        ValidationInvalidReference,
        ValidationInvalidDirection,
        ValidationNumericParse,
        ValidationEmptyValue,
        ValidationSingletonViolation,
        ValidationRequireEither,
        ValidationWrongPipeline,
        ValidationDynamicFeatures,
        // Expression
        ExpressionEmpty,
        ExpressionLex,
        ExpressionUnsupportedConstruct,
        ExpressionStrictEquality,
        ExpressionParseMismatch,
        ExpressionUnexpectedToken,
        ExpressionInvalidLvalue,
        ExpressionTypeCoercion,
        ExpressionGoTernaryUnsupported,
        // Import
        ImportFileNotFound,
        ImportKindMismatch,
        ImportNotForge,
        ImportReadError,
        // Manifest
        ManifestCircularDependency,
        ManifestIo,
        // Generate
        GenerateInvalidConfig,
        GenerateTemplateLoad,
        GenerateTemplateRender,
        // Io
        IoFilesystem,
        // Cli
        CliUnknownLanguage,
        CliUnsupportedLanguage,
        CliReadInput,
        CliWriteOutput,
        CliCreateOutputDir,
        CliScxmlParse,
        CliScxmlGenerate,
        CliDynamicFeatures,
        CliMissingMetadataField,
        CliNotADirectory,
        CliInvalidFormatOption,
        CliJsonSerialization,
        CliProjectRootNotFound,
        CliFormatStyleNotFound,
        CliNoScxmlTag,
        // Mesh Deploy
        MeshDeployRead,
        MeshDeployParse,
        MeshDeployUnsupportedVersion,
        MeshDeployDuplicateMachine,
        // Mesh External config
        MeshExternalRead,
        MeshExternalParse,
        MeshExternalUnresolvedNames,
        MeshExternalAmbiguousEventGroup,
        MeshExternalEmptyEventGroup,
        MeshExternalNamedReferenceWithoutConfig,
        MeshExternalReservedSomeipIdKeys,
        MeshExternalSomeipFieldOnNonSomeipTransport,
        MeshExternalConflictingEventSchema,
        MeshExternalConflictingEventFieldKinds,
        MeshExternalEmptyEventEntry,
        // Mesh Topology
        MeshTopologyUnresolvedTargets,
        MeshTopologyMachineNotFound,
        MeshTopologyReceiverNotDeclared,
        MeshTopologyAbsoluteSourcePath,
        MeshTopologyReceiverSourceRead,
        MeshTopologyReceiverSourceParse,
        MeshTopologyUncoveredEvents,
        MeshTopologyPatternCapabilityViolation,
        MeshTopologyMissingBindingField,
        MeshTopologyInvalidBindingField,
        MeshTopologyEventBindingUnused,
        // Mesh Codegen
        MeshCodegenUnsupportedLanguage,
        MeshCodegenUnsupportedTransport,
        MeshCodegenTemplateRead,
        MeshCodegenTemplateRender,
        MeshCodegenEventNameCollision,
        // Mesh Io
        MeshIo,
    ]
};

impl Diagnostic {
    /// Construct a last-resort diagnostic for failures in the
    /// diagnostic pipeline itself — e.g. serde serialization error,
    /// OOM during `to_diagnostic`. Flowing even this path through
    /// the struct (instead of hand-building a JSON string) keeps the
    /// wire contract a single source of truth: schema bumps touch
    /// exactly one place.
    pub fn meta_failure(message: impl Into<String>) -> Self {
        let message = message.into();
        let code = DiagnosticCode::IoFilesystem;
        let stage = Stage::Io;
        let id = compute_id(code, stage, None, std::slice::from_ref(&message));
        Diagnostic {
            schema_version: SCHEMA_VERSION,
            id,
            code,
            stage,
            spec: code.spec_anchor(),
            message,
            location: None,
            expected: None,
            actual: None,
            fix: None,
        }
    }
}

impl DiagnosticCode {
    /// Specification anchor for this code, when the rule it enforces
    /// has a well-defined section in an authoritative document.
    ///
    /// Single source of truth for the `spec` wire field. Emission
    /// sites no longer carry a per-variant `spec: Some(...)` /
    /// `spec: None` literal — they all route through this method so
    /// that a spec reference bumps cost once per code, not once per
    /// call-site. Returning `None` is explicit: a code with no
    /// anchor means "no verifiable citation exists". Inventing a
    /// plausible-looking section label here is strictly worse than
    /// leaving it empty, because agents would ground hallucinated
    /// references against a real document and silently drift.
    ///
    /// Anchors currently use the following convention:
    ///   - `"SCE Forge §N.M"` → section N.M of SCE_FORGE.md
    ///   - `"SCE Mesh §N"` → section N of SCE_MESH.md
    ///   - `"SCE Forge XSD"` → the XSD schema (schemas/sce-forge*.xsd)
    pub fn spec_anchor(&self) -> Option<&'static str> {
        use DiagnosticCode::*;
        match self {
            // ── Forge XSD ────────────────────────────────────────
            XmlSchemaValidation => Some("SCE Forge XSD"),

            // ── Forge kind system (SCE_FORGE.md) ─────────────────
            ValidationUnsupportedKind => Some("SCE Forge §3.2"),
            ValidationInvalidDirection => Some("SCE Forge §3.3"),
            ValidationWrongPipeline => Some("SCE Forge §4"),

            // ── Forge expression language (SCE_FORGE.md §3.4) ────
            ExpressionEmpty
            | ExpressionLex
            | ExpressionUnsupportedConstruct
            | ExpressionStrictEquality
            | ExpressionParseMismatch
            | ExpressionUnexpectedToken
            | ExpressionInvalidLvalue
            | ExpressionTypeCoercion
            | ExpressionGoTernaryUnsupported => Some("SCE Forge §3.4"),

            // ── Mesh deploy.yaml schema (SCE_MESH.md §14) ────────
            MeshDeployParse
            | MeshDeployUnsupportedVersion
            | MeshDeployDuplicateMachine
            | MeshTopologyMachineNotFound
            | MeshTopologyMissingBindingField
            | MeshTopologyInvalidBindingField
            | MeshTopologyEventBindingUnused => Some("SCE Mesh §14"),

            // ── Mesh remote invoke / topology (SCE_MESH.md §9) ───
            MeshTopologyUnresolvedTargets
            | MeshTopologyReceiverNotDeclared
            | MeshTopologyUncoveredEvents
            | MeshTopologyPatternCapabilityViolation => Some("SCE Mesh §9"),

            // ── Mesh build pipeline (SCE_MESH.md §7) ─────────────
            MeshCodegenUnsupportedLanguage => Some("SCE Mesh §7"),

            // ── Mesh protocol mapping (SCE_MESH.md §8) ───────────
            MeshCodegenUnsupportedTransport => Some("SCE Mesh §8"),

            // ── No authoritative citation ────────────────────────
            //
            // Anchors are deliberately narrow: a code lands here when
            // the rule is operational (I/O failures, template render
            // crashes, CLI argument parsing) or tied to policy that
            // does not have a pinned section yet. Leaving `None` keeps
            // the wire format honest; the message still carries the
            // repair guidance.
            XmlParse
            | ValidationMissingElement
            | ValidationMissingAttribute
            | ValidationInvalidAttribute
            | ValidationDuplicateId
            | ValidationDuplicateContextObject
            | ValidationEmptyCollection
            | ValidationCountMismatch
            | ValidationIncompatibleAttributes
            | ValidationMissingContext
            | ValidationInvalidReference
            | ValidationNumericParse
            | ValidationEmptyValue
            | ValidationSingletonViolation
            | ValidationRequireEither
            | ValidationDynamicFeatures
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
            | CliUnknownLanguage
            | CliUnsupportedLanguage
            | CliReadInput
            | CliWriteOutput
            | CliCreateOutputDir
            | CliScxmlParse
            | CliScxmlGenerate
            | CliDynamicFeatures
            | CliMissingMetadataField
            | CliNotADirectory
            | CliInvalidFormatOption
            | CliJsonSerialization
            | CliProjectRootNotFound
            | CliFormatStyleNotFound
            | CliNoScxmlTag
            | MeshDeployRead
            | MeshExternalRead
            | MeshExternalParse
            | MeshExternalUnresolvedNames
            | MeshExternalAmbiguousEventGroup
            | MeshExternalEmptyEventGroup
            | MeshExternalNamedReferenceWithoutConfig
            | MeshExternalReservedSomeipIdKeys
            | MeshExternalSomeipFieldOnNonSomeipTransport
            | MeshExternalConflictingEventSchema
            | MeshExternalConflictingEventFieldKinds
            | MeshExternalEmptyEventEntry
            | MeshTopologyAbsoluteSourcePath
            | MeshTopologyReceiverSourceRead
            | MeshTopologyReceiverSourceParse
            | MeshCodegenTemplateRead
            | MeshCodegenTemplateRender
            | MeshCodegenEventNameCollision
            | MeshIo => None,
        }
    }

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
            ValidationDuplicateContextObject => "validation/duplicate-context-object",
            ValidationEmptyCollection => "validation/empty-collection",
            ValidationCountMismatch => "validation/count-mismatch",
            ValidationIncompatibleAttributes => "validation/incompatible-attributes",
            ValidationMissingContext => "validation/missing-context",
            ValidationInvalidReference => "validation/invalid-reference",
            ValidationInvalidDirection => "validation/invalid-direction",
            ValidationNumericParse => "validation/numeric-parse",
            ValidationEmptyValue => "validation/empty-value",
            ValidationSingletonViolation => "validation/singleton-violation",
            ValidationRequireEither => "validation/require-either",
            ValidationWrongPipeline => "validation/wrong-pipeline",
            ValidationDynamicFeatures => "validation/dynamic-features",
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
///
/// The `spec` wire field is intentionally absent here: it is a
/// property of the `DiagnosticCode`, resolved at emission time via
/// [`DiagnosticCode::spec_anchor`]. Keeping the anchor on the code
/// rather than on the per-variant payload means contributors update
/// one table when they add a new code, not two.
struct DiagnosticFields {
    code: DiagnosticCode,
    stage: Stage,
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
        fields.code,
        fields.stage,
        location.as_ref().map(|l| l.file.as_str()),
        &fields.key_fragments,
    );

    Diagnostic {
        schema_version: SCHEMA_VERSION,
        id,
        code: fields.code,
        stage: fields.stage,
        spec: fields.code.spec_anchor(),
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
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![detail.clone()],
        },
        XmlError::SchemaValidation(_) => DiagnosticFields {
            code: DiagnosticCode::XmlSchemaValidation,
            stage: Stage::Xml,
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
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![kind.to_string(), element.clone()],
        },
        ValidationError::MissingAttribute { element, attr } => DiagnosticFields {
            code: DiagnosticCode::ValidationMissingAttribute,
            stage: Stage::Validation,
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
            expected: None,
            actual: Some(id.clone()),
            fix: Some(Fix::RenameDuplicate {
                what: what.clone(),
                id: id.clone(),
            }),
            key_fragments: vec![kind.to_string(), what.clone(), id.clone()],
        },
        ValidationError::DuplicateContextObject { id } => DiagnosticFields {
            code: DiagnosticCode::ValidationDuplicateContextObject,
            stage: Stage::Validation,
            expected: None,
            actual: Some(id.clone()),
            // `<sce:context>` is a document-wide scope, so the repair
            // surface is identical to any other duplicate id — rename
            // one of the declarations. `what` names the namespace so
            // agents can disambiguate from state/field/event id reuse.
            fix: Some(Fix::RenameDuplicate {
                what: "sce:context id".to_string(),
                id: id.clone(),
            }),
            key_fragments: vec![id.clone()],
        },
        ValidationError::EmptyCollection { kind, what } => DiagnosticFields {
            code: DiagnosticCode::ValidationEmptyCollection,
            stage: Stage::Validation,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![kind.to_string(), what.clone()],
        },
        ValidationError::CountMismatch { kind, detail } => DiagnosticFields {
            code: DiagnosticCode::ValidationCountMismatch,
            stage: Stage::Validation,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![kind.to_string(), detail.clone()],
        },
        ValidationError::IncompatibleAttributes { element, detail } => DiagnosticFields {
            code: DiagnosticCode::ValidationIncompatibleAttributes,
            stage: Stage::Validation,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![element.clone(), detail.clone()],
        },
        ValidationError::MissingContext { site, detail } => DiagnosticFields {
            code: DiagnosticCode::ValidationMissingContext,
            stage: Stage::Validation,
            // `actual` carries the offending expression so agents can
            // locate the reference in the source document; the repair
            // shape (add a sibling `<sce:context>` element) does not
            // match any existing `Fix` variant, so `fix` stays `None`
            // rather than fabricating one that misleads the repair
            // loop. The message is precise enough for human drivers.
            expected: None,
            actual: Some(detail.clone()),
            fix: None,
            key_fragments: vec![site.clone(), detail.clone()],
        },
        ValidationError::InvalidReference {
            kind,
            name,
            what,
            available,
        } => DiagnosticFields {
            code: DiagnosticCode::ValidationInvalidReference,
            stage: Stage::Validation,
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
            expected: None,
            actual: Some(value.clone()),
            fix: None,
            key_fragments: vec![element.clone(), attr.clone(), value.clone()],
        },
        ValidationError::EmptyValue { element, attr } => DiagnosticFields {
            code: DiagnosticCode::ValidationEmptyValue,
            stage: Stage::Validation,
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
            expected: None,
            actual: Some(kind.to_string()),
            fix: None,
            key_fragments: vec![kind.to_string()],
        },
        ValidationError::DynamicFeatures { name, reason } => DiagnosticFields {
            code: DiagnosticCode::ValidationDynamicFeatures,
            stage: Stage::Validation,
            // `actual` carries the specific blocker so agents route
            // between Interpreter fallback (dynamic invoke) and
            // document rewrite (missing initial); no closed candidate
            // set exists for the repair, so `fix` stays `None`.
            expected: None,
            actual: Some(reason.clone()),
            fix: None,
            key_fragments: vec![name.clone(), reason.clone()],
        },
    }
}

fn expression_fields(e: &ExprError) -> DiagnosticFields {
    match e {
        ExprError::Empty { what } => DiagnosticFields {
            code: DiagnosticCode::ExpressionEmpty,
            stage: Stage::Expression,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![(*what).to_string()],
        },
        ExprError::Lex { position, detail } => DiagnosticFields {
            code: DiagnosticCode::ExpressionLex,
            stage: Stage::Expression,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![position.to_string(), detail.clone()],
        },
        ExprError::UnsupportedConstruct { construct } => DiagnosticFields {
            code: DiagnosticCode::ExpressionUnsupportedConstruct,
            stage: Stage::Expression,
            expected: None,
            actual: Some(construct.clone()),
            fix: None,
            key_fragments: vec![construct.clone()],
        },
        ExprError::StrictEquality { operator, strict } => DiagnosticFields {
            code: DiagnosticCode::ExpressionStrictEquality,
            stage: Stage::Expression,
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
            expected: Some(vec![expected.clone()]),
            actual: Some(got.clone()),
            fix: None,
            key_fragments: vec![expected.clone(), got.clone()],
        },
        ExprError::UnexpectedToken { token } => DiagnosticFields {
            code: DiagnosticCode::ExpressionUnexpectedToken,
            stage: Stage::Expression,
            expected: None,
            actual: Some(token.clone()),
            fix: None,
            key_fragments: vec![token.clone()],
        },
        ExprError::InvalidLvalue { location, detail } => DiagnosticFields {
            code: DiagnosticCode::ExpressionInvalidLvalue,
            stage: Stage::Expression,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![location.clone(), detail.clone()],
        },
        ExprError::TypeCoercion { lang, detail } => DiagnosticFields {
            code: DiagnosticCode::ExpressionTypeCoercion,
            stage: Stage::Expression,
            expected: None,
            actual: Some((*lang).to_string()),
            fix: None,
            key_fragments: vec![(*lang).to_string(), detail.clone()],
        },
        ExprError::GoTernary => DiagnosticFields {
            code: DiagnosticCode::ExpressionGoTernaryUnsupported,
            stage: Stage::Expression,
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
            expected: None,
            actual: Some(src.clone()),
            fix: None,
            key_fragments: vec![src.clone()],
        },
        ImportError::ReadError { src, .. } => DiagnosticFields {
            code: DiagnosticCode::ImportReadError,
            stage: Stage::Import,
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
            expected: None,
            actual: None,
            fix: None,
            key_fragments: cycle.clone(),
        },
        ManifestError::Io { context, .. } => DiagnosticFields {
            code: DiagnosticCode::ManifestIo,
            stage: Stage::Manifest,
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
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![detail.clone()],
        },
        GenerateError::TemplateLoad(detail) => DiagnosticFields {
            code: DiagnosticCode::GenerateTemplateLoad,
            stage: Stage::Generate,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![detail.clone()],
        },
        GenerateError::TemplateRender(detail) => DiagnosticFields {
            code: DiagnosticCode::GenerateTemplateRender,
            stage: Stage::Generate,
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
/// code-str | stage-str | file-or-empty | <US>frag<US>frag...
/// ```
///
/// - `code` / `stage` are typed so the canonical string form (and thus
///   the id bytes) is controlled in exactly one place: their `as_str`
///   impls. Callers cannot pass arbitrary strings.
/// - `file` is `None` for errors with no source file (mesh deploy.yaml,
///   CLI-boundary failures); the slot is serialised as an empty string
///   so `Some("")` and `None` hash identically.
/// - Fragments are separated by ASCII unit separator (0x1f) so that
///   e.g. `["ab", "c"]` and `["a", "bc"]` hash differently.
///
/// Excludes the Display message on purpose: rewording a thiserror
/// `#[error(...)]` template must not change the id of the underlying
/// semantic error. Public so the CLI binary (separate crate) can
/// compute ids for its own error family using the same canonical
/// shape; the canonical key format is part of the error contract
/// (see SCE_ERROR_CONTRACT.md §3).
pub fn compute_id(
    code: DiagnosticCode,
    stage: Stage,
    file: Option<&str>,
    key_fragments: &[String],
) -> String {
    let mut hasher = Fnv1a64::new();
    hasher.write(code.as_str().as_bytes());
    hasher.write(b"|");
    hasher.write(stage.as_str().as_bytes());
    hasher.write(b"|");
    hasher.write(file.unwrap_or("").as_bytes());
    for frag in key_fragments {
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
            DiagnosticCode::ValidationMissingAttribute,
            Stage::Validation,
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

    /// Shared golden table for first-party `ForgeError` cases.
    ///
    /// Single source of truth consumed by both
    /// [`diagnostic_goldens_are_byte_stable`] (JSON wire shape) and
    /// [`human_mode_matches_json_message`] (Display↔`message` invariant).
    /// A contributor adding a new variant updates both surfaces
    /// atomically — the tests cannot drift apart because neither owns
    /// its own cases list.
    ///
    /// Each entry: `(label, error_instance, expected_json_golden)`.
    /// Update the JSON string deliberately alongside a `SCHEMA_VERSION`
    /// bump when the wire shape changes.
    fn forge_golden_entries() -> Vec<(&'static str, ForgeError, &'static str)> {
        vec![
            (
                "forge/missing-attribute",
                ValidationError::MissingAttribute {
                    element: "sce:field".into(),
                    attr: "id".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:1c56b923b2b2b87f","code":"validation/missing-attribute","stage":"validation","message":"sce:field must have an 'id' attribute","fix":{"kind":"add_attribute","element":"sce:field","attr":"id"}}"#,
            ),
            (
                "forge/invalid-attribute",
                ValidationError::InvalidAttribute {
                    element: "sce:field".into(),
                    attr: "sce:type".into(),
                    value: "blob".into(),
                    expected: "u8, u16, u32".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:dd04a37de468ffb4","code":"validation/invalid-attribute","stage":"validation","message":"sce:field: unknown sce:type value 'blob' (expected: u8, u16, u32)","actual":"blob","fix":{"kind":"replace_one_of","candidates":["u8","u16","u32"]}}"#,
            ),
            (
                "forge/invalid-reference",
                ValidationError::InvalidReference {
                    kind: ForgeKind::Statechart,
                    what: "transition target".into(),
                    name: "missing".into(),
                    available: "armed, disarmed".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:2e4c02e2b0e7e383","code":"validation/invalid-reference","stage":"validation","message":"statechart: missing does not match any transition target (available: armed, disarmed)","actual":"missing","fix":{"kind":"replace_one_of","candidates":["armed","disarmed"]}}"#,
            ),
            (
                "forge/require-either",
                ValidationError::RequireEither {
                    element: "send".into(),
                    alternatives: vec!["event".into(), "eventexpr".into()],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:e10a747be1752ef3","code":"validation/require-either","stage":"validation","message":"send must have at least one of: event, eventexpr","fix":{"kind":"add_one_of","element":"send","attrs":["event","eventexpr"]}}"#,
            ),
            (
                "forge/go-ternary",
                ExprError::GoTernary.into(),
                r#"{"v":1,"id":"fnv1a:ef5b56dbf74b8718","code":"expression/go-ternary-unsupported","stage":"expression","spec":"SCE Forge §3.4","message":"cannot transpile ternary expression to Go: Go has no conditional expression"}"#,
            ),
            (
                "forge/not-forge",
                ImportError::NotForge { src: "neighbour.scxml".into() }.into(),
                r#"{"v":1,"id":"fnv1a:7cec8a8357830a5a","code":"import/not-forge","stage":"import","message":"<sce:import src=\"neighbour.scxml\">: not a forge document (no sce:kind)","actual":"neighbour.scxml"}"#,
            ),
            (
                "forge/strict-equality",
                ExprError::StrictEquality { operator: "==", strict: "===" }.into(),
                r#"{"v":1,"id":"fnv1a:056d2165b00f16bd","code":"expression/strict-equality","stage":"expression","spec":"SCE Forge §3.4","message":"loose == is not permitted in Extended SCXML. Use === instead.","actual":"==","fix":{"kind":"replace_with","to":"==="}}"#,
            ),
            (
                "forge/import-kind-mismatch",
                ImportError::KindMismatch {
                    src: "peer.scxml".into(),
                    declared: "validator".into(),
                    actual: "codec".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:5f500ed01d12c1bb","code":"import/kind-mismatch","stage":"import","message":"<sce:import src=\"peer.scxml\" kind=\"validator\">: actual kind is 'codec' (mismatch)","actual":"validator","fix":{"kind":"replace_with","to":"codec"}}"#,
            ),
        ]
    }

    /// Shared golden table for first-party `MeshError` cases. See
    /// [`forge_golden_entries`] for the rationale; same shape.
    fn mesh_golden_entries() -> Vec<(&'static str, crate::mesh::error::MeshError, &'static str)> {
        use crate::mesh::error::TopologyError;
        use crate::mesh::target::TargetId;
        vec![
            (
                "mesh/missing-binding-field",
                TopologyError::MissingBindingField {
                    machine: "ecu_a".into(),
                    target: TargetId::new("#motor").unwrap(),
                    transport: "someip".into(),
                    field: "service".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:d7ba280d1556705e","code":"mesh/topology-missing-binding-field","stage":"mesh-topology","spec":"SCE Mesh §14","message":"machine 'ecu_a': binding for '#motor' (transport: someip) is missing required field 'service'. Add 'service:' to the binding in deploy.yaml.","fix":{"kind":"add_attribute","element":"machines.ecu_a.bindings.#motor","attr":"service"}}"#,
            ),
            (
                "mesh/event-binding-unused",
                TopologyError::EventBindingUnused {
                    machine: "ecu_a".into(),
                    target: TargetId::new("#motor").unwrap(),
                    event: "legacy.ping".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:4fdd02e5a9781de8","code":"mesh/topology-event-binding-unused","stage":"mesh-topology","spec":"SCE Mesh §14","message":"machine 'ecu_a': binding '#motor' declares events.legacy.ping in deploy.yaml, but the SCXML model never sends 'legacy.ping' to this target. Remove the unused entry, or correct the event name.","actual":"legacy.ping","fix":{"kind":"remove_fields","location":"machines.ecu_a.bindings.#motor.events","fields":["legacy.ping"]}}"#,
            ),
        ]
    }

    /// Byte-stable goldens: each error variant in
    /// [`forge_golden_entries`] / [`mesh_golden_entries`] produces the
    /// exact JSON string pinned in the table. A byte mismatch means a
    /// consumer that dedup'd on `id` yesterday now sees a different
    /// record for the same semantic error — a wire-format regression.
    /// Update the goldens deliberately (alongside a schema-version
    /// bump, when appropriate), never silently.
    #[test]
    fn diagnostic_goldens_are_byte_stable() {
        let mut mismatches: Vec<String> = Vec::new();
        for (label, err, golden) in forge_golden_entries() {
            let actual = serde_json::to_string(&single(&err)).unwrap();
            if actual != golden {
                mismatches.push(format!(
                    "\n[{label}]\nexpected: {golden}\n  actual: {actual}"
                ));
            }
        }
        for (label, err, golden) in mesh_golden_entries() {
            let actual = serde_json::to_string(&single(&err)).unwrap();
            if actual != golden {
                mismatches.push(format!(
                    "\n[{label}]\nexpected: {golden}\n  actual: {actual}"
                ));
            }
        }
        assert!(
            mismatches.is_empty(),
            "byte-stable goldens drifted:\n{}\n\nIf this change is intentional, update the table AND bump SCHEMA_VERSION if the shape changed.",
            mismatches.join("\n")
        );
    }

    /// Structural invariant: for every first-party error
    /// (`ForgeError`, `MeshError`), the human-mode `Display` output
    /// equals the JSON `message` field byte-for-byte.
    ///
    /// Held by construction today — `ToDiagnostics` impls at
    /// `diagnostic.rs:920` and `mesh/error.rs:876` set
    /// `message: self.to_string()`, so the two surfaces cannot drift
    /// independently. The test locks that derivation in: anyone
    /// refactoring a `ToDiagnostics::to_diagnostics` impl to compute a
    /// bespoke `message` string (instead of delegating to `Display`)
    /// must either keep this test green or delete it with a documented
    /// rationale in `SCE_ERROR_CONTRACT.md`.
    ///
    /// Why this matters: operators read `format!("{}", err)` on stderr
    /// via `ErrorFormat::Human`; upstream agents consume
    /// `Diagnostic.message` via `--error-format=json`. If the two ever
    /// diverge, the same error gets described two different ways —
    /// operator pages the agent, agent's memory references a wording
    /// the operator has never seen. The JSON byte-goldens cover the
    /// JSON surface; this test covers the human surface via the
    /// invariant that derives one from the other.
    ///
    /// Cases are shared with [`diagnostic_goldens_are_byte_stable`]
    /// via [`forge_golden_entries`] / [`mesh_golden_entries`] — the
    /// two tests cannot drift out of coverage parity by construction.
    #[test]
    fn human_mode_matches_json_message() {
        let mut mismatches: Vec<String> = Vec::new();
        for (label, err, _golden) in forge_golden_entries() {
            let human = format!("{err}");
            let json_message = single(&err).message;
            if human != json_message {
                mismatches.push(format!(
                    "\n[{label}]\n  human: {human}\n   json: {json_message}"
                ));
            }
        }
        for (label, err, _golden) in mesh_golden_entries() {
            let human = format!("{err}");
            let json_message = single(&err).message;
            if human != json_message {
                mismatches.push(format!(
                    "\n[{label}]\n  human: {human}\n   json: {json_message}"
                ));
            }
        }
        assert!(
            mismatches.is_empty(),
            "human-mode Display diverged from JSON message field:\n{}\n\n\
             First-party ToDiagnostics impls must derive `message` from \
             `self.to_string()` (see diagnostic.rs:920, mesh/error.rs:876). \
             If a bespoke message is intentional, remove the offending case \
             and document the exception in SCE_ERROR_CONTRACT.md §3.",
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
            | ValidationDuplicateContextObject
            | ValidationEmptyCollection
            | ValidationCountMismatch
            | ValidationIncompatibleAttributes
            | ValidationMissingContext
            | ValidationNumericParse
            | ValidationEmptyValue
            | ValidationSingletonViolation
            | ValidationWrongPipeline
            | ValidationDynamicFeatures
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

    /// No two entries in [`ALL_DIAGNOSTIC_CODES`] resolve to the
    /// same slash-path. Duplicates would silently skew the serde and
    /// schema-drift tests (both iterate the slice), so catching them
    /// as their own assertion keeps failure messages sharp. Accidental
    /// slice shrinkage is caught by
    /// `json_schema_enums_match_rust_source_of_truth`, which compares
    /// the slice to the schema's 83-entry code enum.
    #[test]
    fn all_diagnostic_codes_are_distinct() {
        let mut slash_paths: Vec<&'static str> =
            ALL_DIAGNOSTIC_CODES.iter().map(|c| c.as_str()).collect();
        slash_paths.sort_unstable();
        let total = slash_paths.len();
        slash_paths.dedup();
        assert_eq!(
            slash_paths.len(),
            total,
            "ALL_DIAGNOSTIC_CODES contains duplicate slash-path entries",
        );
    }

    /// Each variant's serde-rendered slash-path must match its
    /// `as_str()` form. The id hash feeds on `as_str`, the wire
    /// contract feeds on serde; a silent rename on one side (or a
    /// missing `#[serde(rename = ...)]` on a newly added variant)
    /// would split the two consumers without otherwise failing any
    /// existing test.
    #[test]
    fn code_serde_rename_matches_as_str() {
        for code in ALL_DIAGNOSTIC_CODES {
            let via_serde = serde_json::to_string(code).unwrap();
            let expected = format!("\"{}\"", code.as_str());
            assert_eq!(
                via_serde, expected,
                "diagnostic code {code:?}: serde rename vs as_str disagree",
            );
        }
    }

    /// External JSON Schema at `schemas/sce-diagnostic.v1.schema.json`
    /// duplicates the `code` enumeration and the `stage` enumeration
    /// for consumers that validate records without linking the
    /// sce-build library. Those enums must agree with the Rust source
    /// of truth or agents will reject (or accept) the wrong records.
    ///
    /// Loaded via `include_str!` so the test fires at compile time —
    /// no external filesystem assumptions — and parsed with `serde_json`
    /// rather than a schema validator because the guard's concern is
    /// source-of-truth parity, not self-consistency of the schema.
    #[test]
    fn json_schema_enums_match_rust_source_of_truth() {
        const SCHEMA_BYTES: &str =
            include_str!("../../../schemas/sce-diagnostic.v1.schema.json");
        let schema: serde_json::Value = serde_json::from_str(SCHEMA_BYTES)
            .expect("diagnostic schema is valid JSON");

        let code_enum: Vec<String> = schema["properties"]["code"]["enum"]
            .as_array()
            .expect("code.enum is an array")
            .iter()
            .map(|v| v.as_str().expect("code enum member is a string").to_string())
            .collect();
        let rust_codes: Vec<String> = ALL_DIAGNOSTIC_CODES
            .iter()
            .map(|c| c.as_str().to_string())
            .collect();
        assert_eq!(
            code_enum, rust_codes,
            "schemas/sce-diagnostic.v1.schema.json code.enum drifted from \
             DiagnosticCode::as_str. Regenerate the schema's code enum in \
             the source order of ALL_DIAGNOSTIC_CODES.",
        );

        let stage_enum: Vec<String> = schema["properties"]["stage"]["enum"]
            .as_array()
            .expect("stage.enum is an array")
            .iter()
            .map(|v| v.as_str().expect("stage enum member is a string").to_string())
            .collect();
        let rust_stages: Vec<String> = [
            Stage::Xml,
            Stage::Validation,
            Stage::Expression,
            Stage::Import,
            Stage::Manifest,
            Stage::Generate,
            Stage::Io,
            Stage::Cli,
            Stage::MeshDeploy,
            Stage::MeshExternal,
            Stage::MeshTopology,
            Stage::MeshCodegen,
        ]
        .iter()
        .map(|s| s.as_str().to_string())
        .collect();
        assert_eq!(
            stage_enum, rust_stages,
            "schemas/sce-diagnostic.v1.schema.json stage.enum drifted from \
             Stage::as_str.",
        );

        let schema_version = schema["properties"]["v"]["const"]
            .as_u64()
            .expect("v.const is an integer");
        assert_eq!(
            schema_version as u32, SCHEMA_VERSION,
            "schemas/sce-diagnostic.v1.schema.json v.const disagrees with SCHEMA_VERSION",
        );
    }
}
