// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
    ExprError, ForgeError, GenerateError, ImportError, Located, ManifestError, ValidationError,
    XmlError,
};
use serde::Serialize;

/// Common interface for any error type that can be rendered to a
/// machine-readable [`Diagnostic`] stream and terminate the process
/// with a stage-specific exit code.
///
/// Callers (CLI entrypoints, build.rs helpers) depend only on this
/// trait. Each error family (`ForgeError`, `MeshError`, CLI-level
/// errors) provides its own mapping without coupling to the others.
///
/// # Single- vs multi-record emission
///
/// Most error types are *single-record*: one error produces one
/// `Diagnostic`. They implement [`SingleDiagnostic`], which provides
/// the defaulted [`SingleDiagnostic::to_single_diagnostic`] assembly
/// method that derives `message` from `Display` at the one call site
/// in the codebase — the structural guarantee that human-mode text
/// and JSON `message` cannot diverge.
///
/// A few container types are *multi-record*: one error fans out to
/// several `Diagnostic`s, each with its own `message`. Today that
/// is only [`XsdErrors`], which emits one record per libxml2
/// violation. These types implement `ToDiagnostics` directly and do
/// **not** implement `SingleDiagnostic` — there is nothing the type
/// system lets them pretend about in terms of a single payload.
pub trait ToDiagnostics: std::fmt::Display {
    fn exit_code(&self) -> i32;

    /// Expand this error into one or more diagnostic records.
    fn to_diagnostics(&self) -> Vec<Diagnostic>;
}

/// Implemented by single-record error types.
///
/// Required: [`diagnostic_payload`](Self::diagnostic_payload) —
/// classifies the variant. Defaulted:
/// [`diagnostic_location`](Self::diagnostic_location) (no location)
/// and [`to_single_diagnostic`](Self::to_single_diagnostic), which is
/// the **single source of truth for `message: self.to_string()`** in
/// the entire crate. Overriding `to_single_diagnostic` is not needed
/// by any current type and is strongly discouraged — if an emitter
/// wants a bespoke `message`, it should emit records directly via
/// `ToDiagnostics` (the multi-record path) so the intent is explicit.
///
/// Multi-record containers like [`XsdErrors`] do **not** implement
/// this trait. They implement only `ToDiagnostics`, which is why the
/// trait split exists: there is no `unreachable!` payload method on
/// any type, because multi-record containers are not obliged by the
/// type system to have a single payload in the first place.
pub trait SingleDiagnostic: ToDiagnostics {
    /// Per-variant structured payload: code, stage, key fragments, and
    /// optional `expected` / `actual` / `fix` fields.
    fn diagnostic_payload(&self) -> DiagnosticPayload;

    /// Optional source location. Default `None` — override in wrapper
    /// types that carry file/line/col context (`Located<E>`).
    fn diagnostic_location(&self) -> Option<Location> {
        None
    }

    /// Build the single-record diagnostic. The default body is the
    /// canonical assembly site: it reads the payload, projects the
    /// location, computes the id, and — crucially — sets
    /// `message: self.to_string()`, the one place in the crate where
    /// a `Diagnostic`'s `message` is derived from `Display`.
    fn to_single_diagnostic(&self) -> Diagnostic {
        let payload = self.diagnostic_payload();
        let location = self.diagnostic_location();
        let id = compute_id(
            payload.code,
            payload.stage,
            location.as_ref().map(|l| l.file.as_str()),
            &payload.key_fragments,
        );
        Diagnostic {
            schema_version: SCHEMA_VERSION,
            id,
            code: payload.code,
            stage: payload.stage,
            spec: payload.code.spec_anchor(),
            message: self.to_string(),
            location,
            expected: payload.expected,
            actual: payload.actual,
            fix: payload.fix,
        }
    }
}

/// Shared `ToDiagnostics::to_diagnostics` body for any error whose
/// inner classification is a `ForgeError`. Handles the XSD multi-record
/// fan-out in one place so `ForgeError` and `Located<ForgeError>`
/// cannot drift.
///
/// `outer` provides the `SingleDiagnostic` behaviour (payload and
/// location) for the non-XSD path; `inner` is the underlying
/// `ForgeError` that may carry `Xml(SchemaValidation)`.
pub(crate) fn forge_to_diagnostics<T: SingleDiagnostic>(
    outer: &T,
    inner: &ForgeError,
) -> Vec<Diagnostic> {
    if let ForgeError::Xml(XmlError::SchemaValidation(xsd_errors)) = inner {
        return xsd_errors.to_diagnostics();
    }
    vec![outer.to_single_diagnostic()]
}

// ── Top-level diagnostic record ────────────────────────────────

/// Current wire-format version. Bumped on *breaking* changes to the
/// diagnostic shape (renamed field, dropped field, changed semantics).
/// Purely additive changes (new optional field) do **not** bump it —
/// consumers must ignore unknown fields, per NDJSON contract.
pub const SCHEMA_VERSION: u32 = 1;

/// Stability of wire schema `v1`. Transitions from `"pre-release"` to
/// `"stable"` when the criterion in `SCE_ERROR_CONTRACT.md` §8.1 is
/// met (first external consumer dependency, or 30 consecutive days at
/// HEAD with no non-additive change). Emitted as `x-sce-schema-status`
/// at the top of `schemas/sce-diagnostic.v1.schema.json` so downstream
/// consumers can read the signal without linking this crate. The
/// `schema_file_declares_status` test guards the two declarations
/// against drift.
pub const SCHEMA_STATUS: &str = "pre-release";

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
    // ── XInclude preprocessing (runs before roxmltree; C++ runtime
    //    parity with PugiXMLDocument::processXInclude). Split by
    //    repair shape so agents can dispatch: missing-href gets a
    //    deterministic add_attribute fix, cycle/too-deep indicate
    //    structural restructuring, malformed points at the included
    //    file, unsupported lists the feature to remove. ─────────
    #[serde(rename = "xml/xinclude-missing-href")]
    XmlXIncludeMissingHref,
    #[serde(rename = "xml/xinclude-not-found")]
    XmlXIncludeNotFound,
    #[serde(rename = "xml/xinclude-read-error")]
    XmlXIncludeReadError,
    #[serde(rename = "xml/xinclude-cycle")]
    XmlXIncludeCycle,
    #[serde(rename = "xml/xinclude-too-deep")]
    XmlXIncludeTooDeep,
    #[serde(rename = "xml/xinclude-malformed")]
    XmlXIncludeMalformed,
    #[serde(rename = "xml/xinclude-unsupported")]
    XmlXIncludeUnsupported,
    // ── sce:template preprocessing (AOT-only, RFC §6.5 Phase A).
    //    Split by repair shape so agents can dispatch without
    //    parsing message text: missing-attribute / missing-param
    //    carry deterministic add_attribute fixes, unknown-param
    //    lists declared names so agents correct typos, cycle /
    //    too-deep indicate structural chain problems, malformed
    //    points at the template file (call-site attribute errors
    //    ride missing-attribute), not-found / read-error are
    //    filesystem-level. ──
    #[serde(rename = "xml/template-not-found")]
    XmlTemplateNotFound,
    #[serde(rename = "xml/template-read-error")]
    XmlTemplateReadError,
    #[serde(rename = "xml/template-malformed")]
    XmlTemplateMalformed,
    #[serde(rename = "xml/template-missing-attribute")]
    XmlTemplateMissingAttribute,
    #[serde(rename = "xml/template-missing-param")]
    XmlTemplateMissingParam,
    #[serde(rename = "xml/template-unknown-param")]
    XmlTemplateUnknownParam,
    #[serde(rename = "xml/template-cycle")]
    XmlTemplateCycle,
    #[serde(rename = "xml/template-too-deep")]
    XmlTemplateTooDeep,

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
    #[serde(rename = "validation/reserved-context-id")]
    ValidationReservedContextId,
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
    #[serde(rename = "validation/mesh-rpc-reserved-param")]
    ValidationMeshRpcReservedParam,
    #[serde(rename = "validation/mesh-rpc-missing-target")]
    ValidationMeshRpcMissingTarget,
    #[serde(rename = "validation/mesh-rpc-duplicate-target")]
    ValidationMeshRpcDuplicateTarget,
    #[serde(rename = "validation/removed-attribute")]
    ValidationRemovedAttribute,

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
    #[serde(rename = "generate/unsupported-feature")]
    GenerateUnsupportedFeature,

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
    #[serde(rename = "cli/scxml-generate")]
    CliScxmlGenerate,
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
    #[serde(rename = "mesh/deploy-invalid-ordering-timings")]
    MeshDeployInvalidOrderingTimings,
    #[serde(rename = "mesh/deploy-invalid-liveliness")]
    MeshDeployInvalidLiveliness,
    #[serde(rename = "mesh/deploy-invalid-server-query-timeout")]
    MeshDeployInvalidServerQueryTimeout,
    #[serde(rename = "mesh/deploy-invalid-outbound-buffer")]
    MeshDeployInvalidOutboundBuffer,
    #[serde(rename = "mesh/deploy-discovery-not-supported")]
    MeshDeployDiscoveryNotSupported,
    #[serde(rename = "mesh/deploy-pool-not-supported-by-transport")]
    MeshDeployPoolNotSupportedByTransport,
    #[serde(rename = "mesh/deploy-pool-missing-instance-list")]
    MeshDeployPoolMissingInstanceList,
    #[serde(rename = "mesh/deploy-pool-empty-instance-list")]
    MeshDeployPoolEmptyInstanceList,
    #[serde(rename = "mesh/deploy-pool-invalid-placeholder")]
    MeshDeployPoolInvalidPlaceholder,
    #[serde(rename = "mesh/deploy-server-pool-not-supported")]
    MeshDeployServerPoolNotSupported,
    #[serde(rename = "mesh/deploy-scxml-invoke-target-conflict")]
    MeshDeployScxmlInvokeTargetConflict,
    #[serde(rename = "mesh/deploy-partition-duplicate-name")]
    MeshDeployPartitionDuplicateName,
    #[serde(rename = "mesh/deploy-partition-multi-device")]
    MeshDeployPartitionMultiDevice,
    #[serde(rename = "mesh/deploy-partition-unit-duplicate")]
    MeshDeployPartitionUnitDuplicate,
    #[serde(rename = "mesh/deploy-partition-machine-not-listed")]
    MeshDeployPartitionMachineNotListed,
    #[serde(rename = "mesh/deploy-partition-empty")]
    MeshDeployPartitionEmpty,
    #[serde(rename = "mesh/deploy-partition-synth-infix-collision")]
    MeshDeployPartitionSynthInfixCollision,
    #[serde(rename = "mesh/deploy-partition-uncovered-unit")]
    MeshDeployPartitionUncoveredUnit,
    #[serde(rename = "mesh/deploy-partition-partial-coverage-requires-default")]
    MeshDeployPartitionPartialCoverageRequiresDefault,
    #[serde(rename = "mesh/deploy-partition-pool-machine")]
    MeshDeployPartitionPoolMachine,
    #[serde(rename = "mesh/deploy-partition-transport-binding-unsupported")]
    MeshDeployPartitionTransportBindingUnsupported,
    #[serde(rename = "mesh/deploy-scxml-invoke-cross-device-transport")]
    MeshDeployScxmlInvokeCrossDeviceTransport,
    #[serde(rename = "mesh/deploy-someip-scxml-invoke-service-id-overflow")]
    MeshDeploySomeipScxmlInvokeServiceIdOverflow,
    #[serde(rename = "mesh/deploy-someip-scxml-invoke-service-id-pin-out-of-range")]
    MeshDeploySomeipScxmlInvokeServiceIdPinOutOfRange,
    #[serde(rename = "mesh/deploy-someip-scxml-invoke-service-id-pin-collision")]
    MeshDeploySomeipScxmlInvokeServiceIdPinCollision,
    #[serde(rename = "mesh/deploy-partition-barrier-timeout-invalid")]
    MeshDeployPartitionBarrierTimeoutInvalid,
    #[serde(rename = "mesh/partition-parallel-root-undesignated")]
    MeshPartitionParallelRootUndesignated,
    #[serde(rename = "mesh/partition-parallel-root-ambiguous")]
    MeshPartitionParallelRootAmbiguous,
    #[serde(rename = "mesh/partition-parallel-root-not-in-machines")]
    MeshPartitionParallelRootNotInMachines,
    #[serde(rename = "mesh/partition-parallel-root-non-host")]
    MeshPartitionParallelRootNonHost,
    #[serde(rename = "mesh/partition-barrier-timeout-without-root")]
    MeshPartitionBarrierTimeoutWithoutRoot,
    #[serde(rename = "mesh/partition-wire21-custom-tcp-unimplemented")]
    MeshPartitionWire21CustomTcpUnimplemented,
    // §16.3 / §16.4 distributability analyzer
    #[serde(rename = "mesh/distributability-r1-shared-write")]
    MeshDistributabilityR1SharedWrite,
    #[serde(rename = "mesh/distributability-r2-cross-region-transition")]
    MeshDistributabilityR2CrossRegionTransition,
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
    #[serde(rename = "mesh/topology-ordering-cannot-be-guaranteed")]
    MeshTopologyOrderingCannotBeGuaranteed,
    #[serde(rename = "mesh/topology-pool-param-name-missing")]
    MeshTopologyPoolParamNameMissing,
    #[serde(rename = "mesh/topology-subscription-source-unbound")]
    MeshTopologySubscriptionSourceUnbound,
    #[serde(rename = "mesh/topology-machine-lifetime-subscription-unsupported")]
    MeshTopologyMachineLifetimeSubscriptionUnsupported,
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
    #[serde(rename = "mesh/codegen-pool-with-rpc-client-unsupported")]
    MeshCodegenPoolWithRpcClientUnsupported,
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
        XmlXIncludeMissingHref,
        XmlXIncludeNotFound,
        XmlXIncludeReadError,
        XmlXIncludeCycle,
        XmlXIncludeTooDeep,
        XmlXIncludeMalformed,
        XmlXIncludeUnsupported,
        XmlTemplateNotFound,
        XmlTemplateReadError,
        XmlTemplateMalformed,
        XmlTemplateMissingAttribute,
        XmlTemplateMissingParam,
        XmlTemplateUnknownParam,
        XmlTemplateCycle,
        XmlTemplateTooDeep,
        // Validation
        ValidationMissingElement,
        ValidationMissingAttribute,
        ValidationInvalidAttribute,
        ValidationUnsupportedKind,
        ValidationDuplicateId,
        ValidationDuplicateContextObject,
        ValidationReservedContextId,
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
        ValidationMeshRpcReservedParam,
        ValidationMeshRpcMissingTarget,
        ValidationMeshRpcDuplicateTarget,
        ValidationRemovedAttribute,
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
        GenerateUnsupportedFeature,
        // Io
        IoFilesystem,
        // Cli
        CliUnknownLanguage,
        CliUnsupportedLanguage,
        CliReadInput,
        CliWriteOutput,
        CliCreateOutputDir,
        CliScxmlGenerate,
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
        MeshDeployInvalidOrderingTimings,
        MeshDeployInvalidLiveliness,
        MeshDeployInvalidServerQueryTimeout,
        MeshDeployInvalidOutboundBuffer,
        MeshDeployDiscoveryNotSupported,
        MeshDeployPoolNotSupportedByTransport,
        MeshDeployPoolMissingInstanceList,
        MeshDeployPoolEmptyInstanceList,
        MeshDeployPoolInvalidPlaceholder,
        MeshDeployServerPoolNotSupported,
        MeshDeployScxmlInvokeTargetConflict,
        MeshDeployPartitionDuplicateName,
        MeshDeployPartitionMultiDevice,
        MeshDeployPartitionUnitDuplicate,
        MeshDeployPartitionMachineNotListed,
        MeshDeployPartitionEmpty,
        MeshDeployPartitionSynthInfixCollision,
        MeshDeployPartitionUncoveredUnit,
        MeshDeployPartitionPartialCoverageRequiresDefault,
        MeshDeployPartitionPoolMachine,
        MeshDeployPartitionTransportBindingUnsupported,
        MeshDeployScxmlInvokeCrossDeviceTransport,
        MeshDeploySomeipScxmlInvokeServiceIdOverflow,
        MeshDeploySomeipScxmlInvokeServiceIdPinOutOfRange,
        MeshDeploySomeipScxmlInvokeServiceIdPinCollision,
        MeshDeployPartitionBarrierTimeoutInvalid,
        MeshPartitionParallelRootUndesignated,
        MeshPartitionParallelRootAmbiguous,
        MeshPartitionParallelRootNotInMachines,
        MeshPartitionParallelRootNonHost,
        MeshPartitionBarrierTimeoutWithoutRoot,
        MeshPartitionWire21CustomTcpUnimplemented,
        MeshDistributabilityR1SharedWrite,
        MeshDistributabilityR2CrossRegionTransition,
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
        MeshTopologyOrderingCannotBeGuaranteed,
        MeshTopologyPoolParamNameMissing,
        MeshTopologySubscriptionSourceUnbound,
        MeshTopologyMachineLifetimeSubscriptionUnsupported,
        // Mesh Codegen
        MeshCodegenUnsupportedLanguage,
        MeshCodegenUnsupportedTransport,
        MeshCodegenTemplateRead,
        MeshCodegenTemplateRender,
        MeshCodegenEventNameCollision,
        MeshCodegenPoolWithRpcClientUnsupported,
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

            // ── Mesh-RPC invoke reserved params (SCE_MESH.md §9.5) ──
            ValidationMeshRpcReservedParam
            | ValidationMeshRpcMissingTarget
            | ValidationMeshRpcDuplicateTarget => Some("SCE Mesh §9.5"),

            // ── Session C/D attribute deprecation (SCE_MESH.md §13) ──
            ValidationRemovedAttribute => Some("SCE Mesh §13"),

            // ── Machine-lifetime subscription wiring (SCE_MESH.md §13) ──
            MeshTopologySubscriptionSourceUnbound
            | MeshTopologyMachineLifetimeSubscriptionUnsupported => Some("SCE Mesh §13"),

            // ── Mesh build pipeline (SCE_MESH.md §7) ─────────────
            MeshCodegenUnsupportedLanguage => Some("SCE Mesh §7"),

            // ── Mesh protocol mapping (SCE_MESH.md §8) ───────────
            MeshCodegenUnsupportedTransport => Some("SCE Mesh §8"),

            // ── Mesh sequence ordering (SCE_MESH.md §10.6) ──────
            MeshTopologyOrderingCannotBeGuaranteed => Some("SCE Mesh §10.6"),
            MeshDeployInvalidOrderingTimings => Some("SCE Mesh §10.6"),

            // ── Mesh communication errors (SCE_MESH.md §16.7) ────
            MeshDeployInvalidLiveliness => Some("SCE Mesh §16.7"),

            // ── Mesh server-side lifecycle (SCE_MESH.md §9.5) ────
            MeshDeployInvalidServerQueryTimeout => Some("SCE Mesh §9.5"),
            MeshDeployInvalidOutboundBuffer => Some("SCE Mesh §10.10"),

            // ── Discovery invariant (SCE_MESH.md §3.3) ──────────
            MeshDeployDiscoveryNotSupported => Some("SCE Mesh §3.3"),

            // ── Mesh remote invoke codegen-shape exclusivity (SCE_MESH.md §9.6) ──
            MeshDeployScxmlInvokeTargetConflict => Some("SCE Mesh §9.6"),

            // ── Mesh cross-device scxml-remote transport (SCE_MESH.md §9.6 L1393) ──
            MeshDeployScxmlInvokeCrossDeviceTransport => Some("SCE Mesh §9.6 L1393"),

            // ── Mesh §9.6 SOME/IP service-ID hybrid allocator (RFC F.X-1) ──
            MeshDeploySomeipScxmlInvokeServiceIdOverflow
            | MeshDeploySomeipScxmlInvokeServiceIdPinOutOfRange
            | MeshDeploySomeipScxmlInvokeServiceIdPinCollision => Some("SCE Mesh §9.6"),

            // ── Mesh binding placeholder + server pool (SCE_MESH.md §14.4) ──
            MeshDeployPoolNotSupportedByTransport
            | MeshDeployPoolMissingInstanceList
            | MeshDeployPoolEmptyInstanceList
            | MeshDeployPoolInvalidPlaceholder
            | MeshDeployServerPoolNotSupported
            | MeshTopologyPoolParamNameMissing => Some("SCE Mesh §14.4"),

            // ── Mesh partitions schema (SCE_MESH.md §14 rules 6-10) ──
            MeshDeployPartitionDuplicateName
            | MeshDeployPartitionMultiDevice
            | MeshDeployPartitionUnitDuplicate
            | MeshDeployPartitionMachineNotListed
            | MeshDeployPartitionEmpty
            | MeshDeployPartitionSynthInfixCollision
            | MeshDeployPartitionUncoveredUnit
            | MeshDeployPartitionPartialCoverageRequiresDefault
            | MeshDeployPartitionPoolMachine
            | MeshDeployPartitionTransportBindingUnsupported
            | MeshDeployPartitionBarrierTimeoutInvalid => Some("SCE Mesh §14"),

            // ── Mesh rule 12 — parallel root designation (SCE_MESH.md §14 rule 12) ──
            MeshPartitionParallelRootUndesignated
            | MeshPartitionParallelRootAmbiguous
            | MeshPartitionParallelRootNotInMachines
            | MeshPartitionParallelRootNonHost
            | MeshPartitionBarrierTimeoutWithoutRoot => Some("SCE Mesh §14 rule 12"),

            // ── Mesh §16.5 wire-21 transport implementation gap ──
            // The wire-21 channel emitter materializes shm only; an
            // explicit `transport_binding: custom_tcp` on a partition
            // that participates in a distributed `<parallel>` route
            // would compile to a shm channel the runtime never opens.
            // Spec accepts custom_tcp (§14 rule 4); the gap is in the
            // codegen surface (§16.5).
            MeshPartitionWire21CustomTcpUnimplemented => Some("SCE Mesh §16.5"),

            // ── Mesh §16.3 distributability analyzer ─────────────
            MeshDistributabilityR1SharedWrite
            | MeshDistributabilityR2CrossRegionTransition => Some("SCE Mesh §16.3"),

            // ── No authoritative citation ────────────────────────
            //
            // Anchors are deliberately narrow: a code lands here when
            // the rule is operational (I/O failures, template render
            // crashes, CLI argument parsing) or tied to policy that
            // does not have a pinned section yet. Leaving `None` keeps
            // the wire format honest; the message still carries the
            // repair guidance.
            XmlParse
            | XmlXIncludeMissingHref
            | XmlXIncludeNotFound
            | XmlXIncludeReadError
            | XmlXIncludeCycle
            | XmlXIncludeTooDeep
            | XmlXIncludeMalformed
            | XmlXIncludeUnsupported
            | XmlTemplateNotFound
            | XmlTemplateReadError
            | XmlTemplateMalformed
            | XmlTemplateMissingAttribute
            | XmlTemplateMissingParam
            | XmlTemplateUnknownParam
            | XmlTemplateCycle
            | XmlTemplateTooDeep
            | ValidationMissingElement
            | ValidationMissingAttribute
            | ValidationInvalidAttribute
            | ValidationDuplicateId
            | ValidationDuplicateContextObject
            | ValidationReservedContextId
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
            | GenerateUnsupportedFeature
            | IoFilesystem
            | CliUnknownLanguage
            | CliUnsupportedLanguage
            | CliReadInput
            | CliWriteOutput
            | CliCreateOutputDir
            | CliScxmlGenerate
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
            | MeshCodegenPoolWithRpcClientUnsupported
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
            XmlXIncludeMissingHref => "xml/xinclude-missing-href",
            XmlXIncludeNotFound => "xml/xinclude-not-found",
            XmlXIncludeReadError => "xml/xinclude-read-error",
            XmlXIncludeCycle => "xml/xinclude-cycle",
            XmlXIncludeTooDeep => "xml/xinclude-too-deep",
            XmlXIncludeMalformed => "xml/xinclude-malformed",
            XmlXIncludeUnsupported => "xml/xinclude-unsupported",
            XmlTemplateNotFound => "xml/template-not-found",
            XmlTemplateReadError => "xml/template-read-error",
            XmlTemplateMalformed => "xml/template-malformed",
            XmlTemplateMissingAttribute => "xml/template-missing-attribute",
            XmlTemplateMissingParam => "xml/template-missing-param",
            XmlTemplateUnknownParam => "xml/template-unknown-param",
            XmlTemplateCycle => "xml/template-cycle",
            XmlTemplateTooDeep => "xml/template-too-deep",
            ValidationMissingElement => "validation/missing-element",
            ValidationMissingAttribute => "validation/missing-attribute",
            ValidationInvalidAttribute => "validation/invalid-attribute",
            ValidationUnsupportedKind => "validation/unsupported-kind",
            ValidationDuplicateId => "validation/duplicate-id",
            ValidationDuplicateContextObject => "validation/duplicate-context-object",
            ValidationReservedContextId => "validation/reserved-context-id",
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
            ValidationMeshRpcReservedParam => "validation/mesh-rpc-reserved-param",
            ValidationMeshRpcMissingTarget => "validation/mesh-rpc-missing-target",
            ValidationMeshRpcDuplicateTarget => "validation/mesh-rpc-duplicate-target",
            ValidationRemovedAttribute => "validation/removed-attribute",
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
            GenerateUnsupportedFeature => "generate/unsupported-feature",
            IoFilesystem => "io/filesystem",
            CliUnknownLanguage => "cli/unknown-language",
            CliUnsupportedLanguage => "cli/unsupported-language",
            CliReadInput => "cli/read-input",
            CliWriteOutput => "cli/write-output",
            CliCreateOutputDir => "cli/create-output-dir",
            CliScxmlGenerate => "cli/scxml-generate",
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
            MeshDeployInvalidOrderingTimings => "mesh/deploy-invalid-ordering-timings",
            MeshDeployInvalidLiveliness => "mesh/deploy-invalid-liveliness",
            MeshDeployInvalidServerQueryTimeout => "mesh/deploy-invalid-server-query-timeout",
            MeshDeployInvalidOutboundBuffer => "mesh/deploy-invalid-outbound-buffer",
            MeshDeployDiscoveryNotSupported => "mesh/deploy-discovery-not-supported",
            MeshDeployPoolNotSupportedByTransport => "mesh/deploy-pool-not-supported-by-transport",
            MeshDeployPoolMissingInstanceList => "mesh/deploy-pool-missing-instance-list",
            MeshDeployPoolEmptyInstanceList => "mesh/deploy-pool-empty-instance-list",
            MeshDeployPoolInvalidPlaceholder => "mesh/deploy-pool-invalid-placeholder",
            MeshDeployServerPoolNotSupported => "mesh/deploy-server-pool-not-supported",
            MeshDeployScxmlInvokeTargetConflict => "mesh/deploy-scxml-invoke-target-conflict",
            MeshDeployPartitionDuplicateName => "mesh/deploy-partition-duplicate-name",
            MeshDeployPartitionMultiDevice => "mesh/deploy-partition-multi-device",
            MeshDeployPartitionUnitDuplicate => "mesh/deploy-partition-unit-duplicate",
            MeshDeployPartitionMachineNotListed => "mesh/deploy-partition-machine-not-listed",
            MeshDeployPartitionEmpty => "mesh/deploy-partition-empty",
            MeshDeployPartitionSynthInfixCollision => "mesh/deploy-partition-synth-infix-collision",
            MeshDeployPartitionUncoveredUnit => "mesh/deploy-partition-uncovered-unit",
            MeshDeployPartitionPartialCoverageRequiresDefault => "mesh/deploy-partition-partial-coverage-requires-default",
            MeshDeployPartitionPoolMachine => "mesh/deploy-partition-pool-machine",
            MeshDeployPartitionTransportBindingUnsupported => "mesh/deploy-partition-transport-binding-unsupported",
            MeshDeployScxmlInvokeCrossDeviceTransport => "mesh/deploy-scxml-invoke-cross-device-transport",
            MeshDeploySomeipScxmlInvokeServiceIdOverflow => "mesh/deploy-someip-scxml-invoke-service-id-overflow",
            MeshDeploySomeipScxmlInvokeServiceIdPinOutOfRange => "mesh/deploy-someip-scxml-invoke-service-id-pin-out-of-range",
            MeshDeploySomeipScxmlInvokeServiceIdPinCollision => "mesh/deploy-someip-scxml-invoke-service-id-pin-collision",
            MeshDeployPartitionBarrierTimeoutInvalid => "mesh/deploy-partition-barrier-timeout-invalid",
            MeshPartitionParallelRootUndesignated => "mesh/partition-parallel-root-undesignated",
            MeshPartitionParallelRootAmbiguous => "mesh/partition-parallel-root-ambiguous",
            MeshPartitionParallelRootNotInMachines => "mesh/partition-parallel-root-not-in-machines",
            MeshPartitionParallelRootNonHost => "mesh/partition-parallel-root-non-host",
            MeshPartitionBarrierTimeoutWithoutRoot => "mesh/partition-barrier-timeout-without-root",
            MeshPartitionWire21CustomTcpUnimplemented => "mesh/partition-wire21-custom-tcp-unimplemented",
            MeshDistributabilityR1SharedWrite => "mesh/distributability-r1-shared-write",
            MeshDistributabilityR2CrossRegionTransition => "mesh/distributability-r2-cross-region-transition",
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
            MeshTopologyOrderingCannotBeGuaranteed => "mesh/topology-ordering-cannot-be-guaranteed",
            MeshTopologyPoolParamNameMissing => "mesh/topology-pool-param-name-missing",
            MeshTopologySubscriptionSourceUnbound => "mesh/topology-subscription-source-unbound",
            MeshTopologyMachineLifetimeSubscriptionUnsupported => "mesh/topology-machine-lifetime-subscription-unsupported",
            MeshCodegenUnsupportedLanguage => "mesh/codegen-unsupported-language",
            MeshCodegenUnsupportedTransport => "mesh/codegen-unsupported-transport",
            MeshCodegenTemplateRead => "mesh/codegen-template-read",
            MeshCodegenTemplateRender => "mesh/codegen-template-render",
            MeshCodegenEventNameCollision => "mesh/codegen-event-name-collision",
            MeshCodegenPoolWithRpcClientUnsupported => "mesh/codegen-pool-with-rpc-client-unsupported",
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
pub struct DiagnosticPayload {
    pub code: DiagnosticCode,
    pub stage: Stage,
    pub expected: Option<Vec<String>>,
    pub actual: Option<String>,
    pub fix: Option<Fix>,
    /// Canonical identifying payload for hashing. Distinct from
    /// `actual` — e.g. `MissingAttribute` identity is (element, attr),
    /// not any single value. Order matters: it is the canonical key.
    pub key_fragments: Vec<String>,
}

impl ToDiagnostics for ForgeError {
    fn exit_code(&self) -> i32 {
        ForgeError::exit_code(self)
    }

    fn to_diagnostics(&self) -> Vec<Diagnostic> {
        forge_to_diagnostics(self, self)
    }
}

impl SingleDiagnostic for ForgeError {
    fn diagnostic_payload(&self) -> DiagnosticPayload {
        forge_error_fields(self)
    }
}

impl ToDiagnostics for Located<ForgeError> {
    fn exit_code(&self) -> i32 {
        self.error.exit_code()
    }

    /// XSD validation errors ignore the outer `Located` location:
    /// each inner `XsdDiag` already carries its own libxml2 line,
    /// which is strictly more precise. The XSD dispatch itself lives
    /// in `forge_to_diagnostics` so both `ForgeError` and
    /// `Located<ForgeError>` share the same fan-out logic.
    fn to_diagnostics(&self) -> Vec<Diagnostic> {
        forge_to_diagnostics(self, &self.error)
    }
}

impl SingleDiagnostic for Located<ForgeError> {
    fn diagnostic_payload(&self) -> DiagnosticPayload {
        self.error.diagnostic_payload()
    }

    fn diagnostic_location(&self) -> Option<Location> {
        Some(Location {
            file: self.location.file.clone(),
            line: self.location.line,
            col: self.location.col,
        })
    }
}

fn forge_error_fields(err: &ForgeError) -> DiagnosticPayload {
    match err {
        ForgeError::Xml(e) => xml_fields(e),
        ForgeError::Validation(e) => validation_fields(e),
        ForgeError::Expression(e) => expression_fields(e),
        ForgeError::Import(e) => import_fields(e),
        ForgeError::Manifest(e) => manifest_fields(e),
        ForgeError::Generate(e) => generate_fields(e),
        ForgeError::Io { path, .. } => DiagnosticPayload {
            code: DiagnosticCode::IoFilesystem,
            stage: Stage::Io,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![path.display().to_string()],
        },
    }
}

fn xml_fields(e: &XmlError) -> DiagnosticPayload {
    use crate::template::TemplateError;
    use crate::xinclude::XIncludeError;
    match e {
        XmlError::Parse(detail) => DiagnosticPayload {
            code: DiagnosticCode::XmlParse,
            stage: Stage::Xml,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![detail.clone()],
        },
        XmlError::SchemaValidation(_) => DiagnosticPayload {
            code: DiagnosticCode::XmlSchemaValidation,
            stage: Stage::Xml,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: Vec::new(),
        },
        // XInclude failure modes. The leaf variant drives the code —
        // agents can dispatch without text parsing. `actual` carries
        // the offending href when known so repair bots can substitute
        // without re-parsing the message; `key_fragments` tie into
        // the content-hash `id` so two runs against the same broken
        // document yield the same identifier.
        XmlError::XInclude(XIncludeError::MissingHref) => DiagnosticPayload {
            code: DiagnosticCode::XmlXIncludeMissingHref,
            stage: Stage::Xml,
            expected: None,
            actual: None,
            fix: Some(Fix::AddAttribute {
                element: "xi:include".to_string(),
                attr: "href".to_string(),
            }),
            key_fragments: Vec::new(),
        },
        XmlError::XInclude(XIncludeError::NotFound { href, searched }) => DiagnosticPayload {
            code: DiagnosticCode::XmlXIncludeNotFound,
            stage: Stage::Xml,
            expected: None,
            actual: Some(href.clone()),
            fix: None,
            key_fragments: vec![href.clone(), searched.clone()],
        },
        XmlError::XInclude(XIncludeError::ReadError { href, .. }) => DiagnosticPayload {
            code: DiagnosticCode::XmlXIncludeReadError,
            stage: Stage::Xml,
            expected: None,
            actual: Some(href.clone()),
            fix: None,
            key_fragments: vec![href.clone()],
        },
        XmlError::XInclude(XIncludeError::Cycle { href, chain }) => DiagnosticPayload {
            code: DiagnosticCode::XmlXIncludeCycle,
            stage: Stage::Xml,
            expected: None,
            actual: Some(href.clone()),
            fix: None,
            key_fragments: vec![href.clone(), chain.clone()],
        },
        XmlError::XInclude(XIncludeError::TooDeep { limit }) => DiagnosticPayload {
            code: DiagnosticCode::XmlXIncludeTooDeep,
            stage: Stage::Xml,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![limit.to_string()],
        },
        XmlError::XInclude(XIncludeError::Malformed { href, detail }) => DiagnosticPayload {
            code: DiagnosticCode::XmlXIncludeMalformed,
            stage: Stage::Xml,
            expected: None,
            actual: Some(href.clone()),
            fix: None,
            key_fragments: vec![href.clone(), detail.clone()],
        },
        XmlError::XInclude(XIncludeError::Unsupported { href, feature }) => DiagnosticPayload {
            code: DiagnosticCode::XmlXIncludeUnsupported,
            stage: Stage::Xml,
            expected: None,
            actual: Some((*feature).to_string()),
            fix: None,
            key_fragments: vec![href.clone(), feature.to_string()],
        },
        // `sce:template` failure modes. Parallel to XInclude: the
        // leaf variant drives the code so agents can dispatch
        // without parsing text; `actual` carries the offending
        // template path (or parameter name, for the param-shaped
        // variants) so repair bots can act without re-parsing the
        // message; `key_fragments` tie into the `id` hash.
        XmlError::Template(TemplateError::NotFound {
            template,
            searched,
        }) => DiagnosticPayload {
            code: DiagnosticCode::XmlTemplateNotFound,
            stage: Stage::Xml,
            expected: None,
            actual: Some(template.clone()),
            fix: None,
            key_fragments: vec![template.clone(), searched.clone()],
        },
        XmlError::Template(TemplateError::ReadError { template, .. }) => DiagnosticPayload {
            code: DiagnosticCode::XmlTemplateReadError,
            stage: Stage::Xml,
            expected: None,
            actual: Some(template.clone()),
            fix: None,
            key_fragments: vec![template.clone()],
        },
        XmlError::Template(TemplateError::Malformed { template, detail }) => DiagnosticPayload {
            code: DiagnosticCode::XmlTemplateMalformed,
            stage: Stage::Xml,
            expected: None,
            actual: Some(template.clone()),
            fix: None,
            key_fragments: vec![template.clone(), detail.clone()],
        },
        XmlError::Template(TemplateError::MissingTemplateAttribute) => DiagnosticPayload {
            code: DiagnosticCode::XmlTemplateMissingAttribute,
            stage: Stage::Xml,
            expected: None,
            actual: None,
            fix: Some(Fix::AddAttribute {
                element: "sce:use".to_string(),
                attr: "template".to_string(),
            }),
            key_fragments: Vec::new(),
        },
        XmlError::Template(TemplateError::MissingParam { template, param }) => DiagnosticPayload {
            code: DiagnosticCode::XmlTemplateMissingParam,
            stage: Stage::Xml,
            expected: None,
            actual: Some(param.clone()),
            fix: Some(Fix::AddAttribute {
                element: "sce:use".to_string(),
                attr: param.clone(),
            }),
            key_fragments: vec![template.clone(), param.clone()],
        },
        XmlError::Template(TemplateError::UnknownParam {
            template,
            param,
            declared,
        }) => DiagnosticPayload {
            code: DiagnosticCode::XmlTemplateUnknownParam,
            stage: Stage::Xml,
            expected: None,
            actual: Some(param.clone()),
            fix: None,
            key_fragments: vec![template.clone(), param.clone(), declared.clone()],
        },
        XmlError::Template(TemplateError::Cycle { template, chain }) => DiagnosticPayload {
            code: DiagnosticCode::XmlTemplateCycle,
            stage: Stage::Xml,
            expected: None,
            actual: Some(template.clone()),
            fix: None,
            key_fragments: vec![template.clone(), chain.clone()],
        },
        XmlError::Template(TemplateError::TooDeep { limit }) => DiagnosticPayload {
            code: DiagnosticCode::XmlTemplateTooDeep,
            stage: Stage::Xml,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![limit.to_string()],
        },
    }
}

fn validation_fields(e: &ValidationError) -> DiagnosticPayload {
    match e {
        ValidationError::MissingElement { kind, element } => DiagnosticPayload {
            code: DiagnosticCode::ValidationMissingElement,
            stage: Stage::Validation,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![kind.to_string(), element.clone()],
        },
        ValidationError::MissingAttribute { element, attr } => DiagnosticPayload {
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
        } => DiagnosticPayload {
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
        ValidationError::UnsupportedKind(value) => DiagnosticPayload {
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
        ValidationError::DuplicateId { kind, what, id } => DiagnosticPayload {
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
        ValidationError::DuplicateContextObject { id } => DiagnosticPayload {
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
        ValidationError::ReservedContextId { id, reserved: _ } => DiagnosticPayload {
            code: DiagnosticCode::ValidationReservedContextId,
            stage: Stage::Validation,
            expected: None,
            actual: Some(id.clone()),
            // The reserved list is closed, but the valid set is
            // infinite (any identifier not in the list). No existing
            // `Fix` variant fits "pick anything except this closed
            // list" — the message carries the list, and the author
            // picks a concrete replacement themselves. Consistent
            // with `ValidationMissingContext`, which also leaves
            // `fix: None` for an open-ended repair.
            fix: None,
            key_fragments: vec![id.clone()],
        },
        ValidationError::EmptyCollection { kind, what } => DiagnosticPayload {
            code: DiagnosticCode::ValidationEmptyCollection,
            stage: Stage::Validation,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![kind.to_string(), what.clone()],
        },
        ValidationError::CountMismatch { kind, detail } => DiagnosticPayload {
            code: DiagnosticCode::ValidationCountMismatch,
            stage: Stage::Validation,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![kind.to_string(), detail.clone()],
        },
        ValidationError::IncompatibleAttributes { element, detail } => DiagnosticPayload {
            code: DiagnosticCode::ValidationIncompatibleAttributes,
            stage: Stage::Validation,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![element.clone(), detail.clone()],
        },
        ValidationError::MissingContext { site, detail } => DiagnosticPayload {
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
        } => DiagnosticPayload {
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
        } => DiagnosticPayload {
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
        } => DiagnosticPayload {
            code: DiagnosticCode::ValidationNumericParse,
            stage: Stage::Validation,
            expected: None,
            actual: Some(value.clone()),
            fix: None,
            key_fragments: vec![element.clone(), attr.clone(), value.clone()],
        },
        ValidationError::EmptyValue { element, attr } => DiagnosticPayload {
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
        ValidationError::SingletonViolation { kind, attr } => DiagnosticPayload {
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
        } => DiagnosticPayload {
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
        ValidationError::WrongPipeline { kind } => DiagnosticPayload {
            code: DiagnosticCode::ValidationWrongPipeline,
            stage: Stage::Validation,
            expected: None,
            actual: Some(kind.to_string()),
            fix: None,
            key_fragments: vec![kind.to_string()],
        },
        ValidationError::DynamicFeatures { name, reason } => DiagnosticPayload {
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
        ValidationError::MeshRpcReservedParam { param, detail } => DiagnosticPayload {
            code: DiagnosticCode::ValidationMeshRpcReservedParam,
            stage: Stage::Validation,
            // `actual` carries the offending reserved param name so
            // agents can locate the element to repair; `detail` flows
            // out in `message` via Display. No `Fix` — the repair
            // ("rename or retype your `<param>`") is author-specific,
            // not a closed candidate list, so fabricating a
            // `ReplaceOneOf` would mislead the repair loop.
            expected: None,
            actual: Some(param.clone()),
            fix: None,
            key_fragments: vec![param.clone(), detail.clone()],
        },
        ValidationError::RemovedAttribute { attribute, event } => DiagnosticPayload {
            code: DiagnosticCode::ValidationRemovedAttribute,
            stage: Stage::Validation,
            // `actual` carries the removed attribute name (including
            // its `sce:` prefix) so agents can target the exact
            // syntax node. The repair is "remove the attribute" —
            // deterministic enough to emit as a `RemoveFields`
            // structured fix. `location` uses `<send>` with the
            // event hint when available so consumers can disambiguate
            // which `<send>` to repair.
            expected: None,
            actual: Some(attribute.clone()),
            fix: Some(Fix::RemoveFields {
                location: match event {
                    Some(ev) => format!("<send event=\"{ev}\">"),
                    None => "<send>".to_string(),
                },
                fields: vec![attribute.clone()],
            }),
            key_fragments: {
                let mut k = vec![attribute.clone()];
                if let Some(ev) = event {
                    k.push(ev.clone());
                }
                k
            },
        },
        ValidationError::MeshRpcMissingTarget => DiagnosticPayload {
            code: DiagnosticCode::ValidationMeshRpcMissingTarget,
            stage: Stage::Validation,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: Vec::new(),
        },
        ValidationError::MeshRpcDuplicateTarget => DiagnosticPayload {
            code: DiagnosticCode::ValidationMeshRpcDuplicateTarget,
            stage: Stage::Validation,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: Vec::new(),
        },
    }
}

fn expression_fields(e: &ExprError) -> DiagnosticPayload {
    match e {
        ExprError::Empty { what } => DiagnosticPayload {
            code: DiagnosticCode::ExpressionEmpty,
            stage: Stage::Expression,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![(*what).to_string()],
        },
        ExprError::Lex { position, detail } => DiagnosticPayload {
            code: DiagnosticCode::ExpressionLex,
            stage: Stage::Expression,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![position.to_string(), detail.clone()],
        },
        ExprError::UnsupportedConstruct { construct } => DiagnosticPayload {
            code: DiagnosticCode::ExpressionUnsupportedConstruct,
            stage: Stage::Expression,
            expected: None,
            actual: Some(construct.clone()),
            fix: None,
            key_fragments: vec![construct.clone()],
        },
        ExprError::StrictEquality { operator, strict } => DiagnosticPayload {
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
        ExprError::ParseMismatch { expected, got } => DiagnosticPayload {
            code: DiagnosticCode::ExpressionParseMismatch,
            stage: Stage::Expression,
            expected: Some(vec![expected.clone()]),
            actual: Some(got.clone()),
            fix: None,
            key_fragments: vec![expected.clone(), got.clone()],
        },
        ExprError::UnexpectedToken { token } => DiagnosticPayload {
            code: DiagnosticCode::ExpressionUnexpectedToken,
            stage: Stage::Expression,
            expected: None,
            actual: Some(token.clone()),
            fix: None,
            key_fragments: vec![token.clone()],
        },
        ExprError::InvalidLvalue { location, detail } => DiagnosticPayload {
            code: DiagnosticCode::ExpressionInvalidLvalue,
            stage: Stage::Expression,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![location.clone(), detail.clone()],
        },
        ExprError::TypeCoercion { lang, detail } => DiagnosticPayload {
            code: DiagnosticCode::ExpressionTypeCoercion,
            stage: Stage::Expression,
            expected: None,
            actual: Some((*lang).to_string()),
            fix: None,
            key_fragments: vec![(*lang).to_string(), detail.clone()],
        },
        ExprError::GoTernary => DiagnosticPayload {
            code: DiagnosticCode::ExpressionGoTernaryUnsupported,
            stage: Stage::Expression,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: Vec::new(),
        },
    }
}

fn import_fields(e: &ImportError) -> DiagnosticPayload {
    match e {
        ImportError::FileNotFound { src, .. } => DiagnosticPayload {
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
        } => DiagnosticPayload {
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
        ImportError::NotForge { src } => DiagnosticPayload {
            code: DiagnosticCode::ImportNotForge,
            stage: Stage::Import,
            expected: None,
            actual: Some(src.clone()),
            fix: None,
            key_fragments: vec![src.clone()],
        },
        ImportError::ReadError { src, .. } => DiagnosticPayload {
            code: DiagnosticCode::ImportReadError,
            stage: Stage::Import,
            expected: None,
            actual: Some(src.clone()),
            fix: None,
            key_fragments: vec![src.clone()],
        },
    }
}

fn manifest_fields(e: &ManifestError) -> DiagnosticPayload {
    match e {
        ManifestError::CircularDependency(cycle) => DiagnosticPayload {
            code: DiagnosticCode::ManifestCircularDependency,
            stage: Stage::Manifest,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: cycle.clone(),
        },
        ManifestError::Io { context, .. } => DiagnosticPayload {
            code: DiagnosticCode::ManifestIo,
            stage: Stage::Manifest,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![context.clone()],
        },
    }
}

fn generate_fields(e: &GenerateError) -> DiagnosticPayload {
    match e {
        GenerateError::InvalidConfig(detail) => DiagnosticPayload {
            code: DiagnosticCode::GenerateInvalidConfig,
            stage: Stage::Generate,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![detail.clone()],
        },
        GenerateError::TemplateLoad(detail) => DiagnosticPayload {
            code: DiagnosticCode::GenerateTemplateLoad,
            stage: Stage::Generate,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![detail.clone()],
        },
        GenerateError::TemplateRender(detail) => DiagnosticPayload {
            code: DiagnosticCode::GenerateTemplateRender,
            stage: Stage::Generate,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![detail.clone()],
        },
        GenerateError::UnsupportedFeature(detail) => DiagnosticPayload {
            code: DiagnosticCode::GenerateUnsupportedFeature,
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
        use crate::forge::error::{ExprError, GenerateError, ImportError, ManifestError, XmlError};
        vec![
            (
                "forge/xml-parse",
                ForgeError::Xml(XmlError::Parse("unexpected end tag </scxml>".into())),
                r#"{"v":1,"id":"fnv1a:16e2e2901e2b9b96","code":"xml/parse","stage":"xml","message":"XML parse error: unexpected end tag </scxml>"}"#,
            ),
            (
                "forge/missing-element",
                ValidationError::MissingElement {
                    kind: ForgeKind::Transform,
                    element: "datamodel".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:f647afe305a652e4","code":"validation/missing-element","stage":"validation","message":"transform kind requires a <datamodel> element"}"#,
            ),
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
            (
                "forge/unsupported-kind",
                ValidationError::UnsupportedKind("bogus".into()).into(),
                r#"{"v":1,"id":"fnv1a:812898e1a23fda4d","code":"validation/unsupported-kind","stage":"validation","spec":"SCE Forge §3.2","message":"unsupported sce:kind value: 'bogus'","actual":"bogus","fix":{"kind":"replace_one_of","candidates":["statechart","transform","lookup","condition","codec","procedure","validator","filter","interpolation","timer","observer"]}}"#,
            ),
            (
                "forge/duplicate-id",
                ValidationError::DuplicateId {
                    kind: ForgeKind::Statechart,
                    what: "state id".into(),
                    id: "armed".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:b7a473dbee90ecba","code":"validation/duplicate-id","stage":"validation","message":"statechart: duplicate state id: 'armed'","actual":"armed","fix":{"kind":"rename_duplicate","what":"state id","id":"armed"}}"#,
            ),
            (
                "forge/duplicate-context-object",
                ValidationError::DuplicateContextObject { id: "ctx1".into() }.into(),
                r#"{"v":1,"id":"fnv1a:5915eba3f66f34b0","code":"validation/duplicate-context-object","stage":"validation","message":"duplicate <sce:context id=\"ctx1\"> declaration","actual":"ctx1","fix":{"kind":"rename_duplicate","what":"sce:context id","id":"ctx1"}}"#,
            ),
            (
                "forge/reserved-context-id",
                ValidationError::ReservedContextId {
                    id: "policy".into(),
                    reserved: &["policy"],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:55056635542ce833","code":"validation/reserved-context-id","stage":"validation","message":"<sce:context id=\"policy\"> uses reserved name; rename to any identifier not in: policy","actual":"policy"}"#,
            ),
            (
                "forge/empty-collection",
                ValidationError::EmptyCollection {
                    kind: ForgeKind::Codec,
                    what: "field with byte layout".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:85bab620d8b00304","code":"validation/empty-collection","stage":"validation","message":"codec kind requires at least one field with byte layout"}"#,
            ),
            (
                "forge/count-mismatch",
                ValidationError::CountMismatch {
                    kind: ForgeKind::Lookup,
                    detail: "value count (5) must match axis breakpoints (4)".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:b084f9ec0d995866","code":"validation/count-mismatch","stage":"validation","message":"lookup: value count (5) must match axis breakpoints (4)"}"#,
            ),
            (
                "forge/incompatible-attributes",
                ValidationError::IncompatibleAttributes {
                    element: "sce:field".into(),
                    detail: "sce:on-miss='error' is incompatible with sce:default".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:916c56cb5f75059f","code":"validation/incompatible-attributes","stage":"validation","message":"sce:field: sce:on-miss='error' is incompatible with sce:default"}"#,
            ),
            (
                "forge/missing-context",
                ValidationError::MissingContext {
                    site: "cpp: condition".into(),
                    detail: "ctx.nonexistent".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:b34a86196217f3b4","code":"validation/missing-context","stage":"validation","message":"cpp: condition: ctx.nonexistent","actual":"ctx.nonexistent"}"#,
            ),
            (
                "forge/invalid-direction",
                ValidationError::InvalidDirection {
                    kind: ForgeKind::Transform,
                    direction: "internal".into(),
                    field: "input".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:0975034c3bd4f30d","code":"validation/invalid-direction","stage":"validation","spec":"SCE Forge §3.3","message":"transform kind does not support 'internal' direction (field 'input')","actual":"internal","fix":{"kind":"replace_one_of","candidates":["input","output"]}}"#,
            ),
            (
                "forge/numeric-parse",
                ValidationError::NumericParse {
                    element: "sce:field".into(),
                    attr: "sce:byte".into(),
                    value: "0xZZ".into(),
                    detail: "invalid hex digit".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:4bbe0c088c2db6ae","code":"validation/numeric-parse","stage":"validation","message":"invalid sce:byte value '0xZZ' on sce:field: invalid hex digit","actual":"0xZZ"}"#,
            ),
            (
                "forge/empty-value",
                ValidationError::EmptyValue {
                    element: "sce:helper".into(),
                    attr: "name".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:d944a323b238f66f","code":"validation/empty-value","stage":"validation","message":"sce:helper 'name' attribute must not be empty","fix":{"kind":"add_attribute","element":"sce:helper","attr":"name"}}"#,
            ),
            (
                "forge/singleton-violation",
                ValidationError::SingletonViolation {
                    kind: ForgeKind::Lookup,
                    attr: "sce:plausibility".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:dd928fd5d29f9ebd","code":"validation/singleton-violation","stage":"validation","message":"only one sce:plausibility attribute allowed per lookup kind"}"#,
            ),
            (
                "forge/wrong-pipeline",
                ValidationError::WrongPipeline {
                    kind: ForgeKind::Statechart,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:71f73dde2407223e","code":"validation/wrong-pipeline","stage":"validation","spec":"SCE Forge §4","message":"statechart kind cannot be processed by the forge pipeline","actual":"statechart"}"#,
            ),
            (
                "forge/dynamic-features",
                ValidationError::DynamicFeatures {
                    name: "chart".into(),
                    reason: "initial state attribute names a state that is not declared".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:aaaac1f1c1e4cf6e","code":"validation/dynamic-features","stage":"validation","message":"cannot generate static code for 'chart': initial state attribute names a state that is not declared","actual":"initial state attribute names a state that is not declared"}"#,
            ),
            (
                "forge/mesh-rpc-reserved-param",
                ValidationError::MeshRpcReservedParam {
                    param: "_mesh_event".into(),
                    detail: "required <param name=\"_mesh_event\"> is missing".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:ea4f2baf300b7c54","code":"validation/mesh-rpc-reserved-param","stage":"validation","spec":"SCE Mesh §9.5","message":"<invoke type=\"sce:mesh-rpc\">: required <param name=\"_mesh_event\"> is missing (param '_mesh_event')","actual":"_mesh_event"}"#,
            ),
            (
                "forge/mesh-rpc-missing-target",
                ValidationError::MeshRpcMissingTarget.into(),
                r##"{"v":1,"id":"fnv1a:690659a64033caa8","code":"validation/mesh-rpc-missing-target","stage":"validation","spec":"SCE Mesh §9.5","message":"<invoke type=\"sce:mesh-rpc\"> must declare exactly one of `src` or `srcexpr` — both are missing. Add `src=\"#<machine>\"` for a build-time target, or `srcexpr=\"...\"` to pick among declared bindings at runtime."}"##,
            ),
            (
                "forge/mesh-rpc-duplicate-target",
                ValidationError::MeshRpcDuplicateTarget.into(),
                r##"{"v":1,"id":"fnv1a:03a5004c0b9aa29b","code":"validation/mesh-rpc-duplicate-target","stage":"validation","spec":"SCE Mesh §9.5","message":"<invoke type=\"sce:mesh-rpc\"> declares both `src` and `srcexpr` — they are mutually exclusive. Keep only the one matching how the target is chosen (static vs runtime)."}"##,
            ),
            (
                "forge/removed-attribute",
                ValidationError::RemovedAttribute {
                    attribute: "sce:qos".into(),
                    event: Some("brake.activate".into()),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:79f17de8d89256c7","code":"validation/removed-attribute","stage":"validation","spec":"SCE Mesh §13","message":"deprecated attribute sce:qos on <send event=\"brake.activate\"> was removed in SCE Mesh §13 path B; pattern is now inferred from event-name conventions and RPC reply pairing from topology structure. Remove the attribute.","actual":"sce:qos","fix":{"kind":"remove_fields","location":"<send event=\"brake.activate\">","fields":["sce:qos"]}}"#,
            ),
            (
                "forge/expression-empty",
                ExprError::Empty { what: "condition" }.into(),
                r#"{"v":1,"id":"fnv1a:87a50d789871d4b9","code":"expression/empty","stage":"expression","spec":"SCE Forge §3.4","message":"empty condition"}"#,
            ),
            (
                "forge/expression-lex",
                ExprError::Lex {
                    position: 7,
                    detail: "unterminated string literal".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:317e5d0fc5f07ffe","code":"expression/lex","stage":"expression","spec":"SCE Forge §3.4","message":"at position 7: unterminated string literal"}"#,
            ),
            (
                "forge/expression-unsupported-construct",
                ExprError::UnsupportedConstruct {
                    construct: "arrow function".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:d953d35f95a3575d","code":"expression/unsupported-construct","stage":"expression","spec":"SCE Forge §3.4","message":"unsupported ECMAScript construct: arrow function. Extended SCXML expressions must use the stateless subset.","actual":"arrow function"}"#,
            ),
            (
                "forge/expression-parse-mismatch",
                ExprError::ParseMismatch {
                    expected: "identifier".into(),
                    got: ";".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:68b81585decce3e8","code":"expression/parse-mismatch","stage":"expression","spec":"SCE Forge §3.4","message":"expected identifier, got ';'","expected":["identifier"],"actual":";"}"#,
            ),
            (
                "forge/expression-unexpected-token",
                ExprError::UnexpectedToken { token: "else".into() }.into(),
                r#"{"v":1,"id":"fnv1a:2f9a3376c0ed3f13","code":"expression/unexpected-token","stage":"expression","spec":"SCE Forge §3.4","message":"unexpected token: 'else'","actual":"else"}"#,
            ),
            (
                "forge/expression-invalid-lvalue",
                ExprError::InvalidLvalue {
                    location: "call expression".into(),
                    detail: "cannot assign to a function call".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:0b54880b78b1e343","code":"expression/invalid-lvalue","stage":"expression","spec":"SCE Forge §3.4","message":"assign location \"call expression\" is not an lvalue: cannot assign to a function call"}"#,
            ),
            (
                "forge/expression-type-coercion",
                ExprError::TypeCoercion {
                    lang: "Rust",
                    detail: "mixing i32 and String".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:af0be9fbfaff2085","code":"expression/type-coercion","stage":"expression","spec":"SCE Forge §3.4","message":"cannot coerce Rust expression: mixing i32 and String","actual":"Rust"}"#,
            ),
            (
                "forge/import-file-not-found",
                ImportError::FileNotFound {
                    src: "peer.scxml".into(),
                    searched: "./scxml/peer.scxml, ./peer.scxml".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:c42ab6780d3198c3","code":"import/file-not-found","stage":"import","message":"<sce:import src=\"peer.scxml\">: file not found (searched: ./scxml/peer.scxml, ./peer.scxml)","actual":"peer.scxml"}"#,
            ),
            (
                "forge/import-read-error",
                ImportError::ReadError {
                    src: "peer.scxml".into(),
                    source: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:a446640c76415d17","code":"import/read-error","stage":"import","message":"<sce:import src=\"peer.scxml\">: cannot read: permission denied","actual":"peer.scxml"}"#,
            ),
            (
                "forge/manifest-circular-dependency",
                ManifestError::CircularDependency(vec![
                    "a.scxml".into(),
                    "b.scxml".into(),
                    "a.scxml".into(),
                ])
                .into(),
                r#"{"v":1,"id":"fnv1a:f8d8d18f937c5411","code":"manifest/circular-dependency","stage":"manifest","message":"circular dependency detected among: a.scxml, b.scxml, a.scxml"}"#,
            ),
            (
                "forge/manifest-io",
                ManifestError::Io {
                    context: "scanning ./forge".into(),
                    source: std::io::Error::from(std::io::ErrorKind::NotFound),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:dc01746049886012","code":"manifest/io","stage":"manifest","message":"scanning ./forge: entity not found"}"#,
            ),
            (
                "forge/generate-invalid-config",
                GenerateError::InvalidConfig("missing `targets` section".into()).into(),
                r#"{"v":1,"id":"fnv1a:326c68123b3f4738","code":"generate/invalid-config","stage":"generate","message":"missing `targets` section"}"#,
            ),
            (
                "forge/generate-template-load",
                GenerateError::TemplateLoad("codec.cpp.jinja2 not found in template dir".into())
                    .into(),
                r#"{"v":1,"id":"fnv1a:71b634149cbd2384","code":"generate/template-load","stage":"generate","message":"template load error: codec.cpp.jinja2 not found in template dir"}"#,
            ),
            (
                "forge/generate-template-render",
                GenerateError::TemplateRender("undefined variable `fields` at line 12".into())
                    .into(),
                r#"{"v":1,"id":"fnv1a:98d75683f2764cff","code":"generate/template-render","stage":"generate","message":"template render error: undefined variable `fields` at line 12"}"#,
            ),
            (
                "forge/generate-unsupported-feature",
                GenerateError::UnsupportedFeature(
                    "<invoke type=\"sce:mesh-rpc\"> in 'brake' has no Rust codegen path".into(),
                )
                .into(),
                r#"{"v":1,"id":"fnv1a:6d877591cf4360d3","code":"generate/unsupported-feature","stage":"generate","message":"feature unsupported in this language: <invoke type=\"sce:mesh-rpc\"> in 'brake' has no Rust codegen path"}"#,
            ),
            (
                "forge/io-filesystem",
                ForgeError::Io {
                    path: std::path::PathBuf::from("/tmp/build/out.rs"),
                    source: std::io::Error::from(std::io::ErrorKind::NotFound),
                },
                r#"{"v":1,"id":"fnv1a:dac850da181d48e9","code":"io/filesystem","stage":"io","message":"I/O error on /tmp/build/out.rs: entity not found"}"#,
            ),
            // XInclude preprocessing variants. Each distinct repair
            // shape (missing-href → add_attribute, cycle / too-deep →
            // structural, not-found / read-error → filesystem,
            // malformed / unsupported → content) rides its own golden
            // so byte-stability drift reveals the specific variant.
            (
                "forge/xinclude-missing-href",
                XmlError::XInclude(crate::xinclude::XIncludeError::MissingHref).into(),
                r#"{"v":1,"id":"fnv1a:797b1211b61e7cb7","code":"xml/xinclude-missing-href","stage":"xml","message":"<xi:include> missing or empty `href` attribute","fix":{"kind":"add_attribute","element":"xi:include","attr":"href"}}"#,
            ),
            (
                "forge/xinclude-not-found",
                XmlError::XInclude(crate::xinclude::XIncludeError::NotFound {
                    href: "guards.xml".into(),
                    searched: "/project/src/guards.xml".into(),
                })
                .into(),
                r#"{"v":1,"id":"fnv1a:36347afad13073c5","code":"xml/xinclude-not-found","stage":"xml","message":"<xi:include href=\"guards.xml\">: file not found (searched: /project/src/guards.xml)","actual":"guards.xml"}"#,
            ),
            (
                "forge/xinclude-read-error",
                XmlError::XInclude(crate::xinclude::XIncludeError::ReadError {
                    href: "guards.xml".into(),
                    source: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
                })
                .into(),
                r#"{"v":1,"id":"fnv1a:c3d60d1cfc810e30","code":"xml/xinclude-read-error","stage":"xml","message":"<xi:include href=\"guards.xml\">: cannot read: permission denied","actual":"guards.xml"}"#,
            ),
            (
                "forge/xinclude-cycle",
                XmlError::XInclude(crate::xinclude::XIncludeError::Cycle {
                    href: "a.xml".into(),
                    chain: "/a.xml → /b.xml → /a.xml".into(),
                })
                .into(),
                r#"{"v":1,"id":"fnv1a:bf44e30be028ada7","code":"xml/xinclude-cycle","stage":"xml","message":"<xi:include href=\"a.xml\">: cycle detected (/a.xml → /b.xml → /a.xml)","actual":"a.xml"}"#,
            ),
            (
                "forge/xinclude-too-deep",
                XmlError::XInclude(crate::xinclude::XIncludeError::TooDeep {
                    limit: crate::xinclude::MAX_XINCLUDE_DEPTH,
                })
                .into(),
                r#"{"v":1,"id":"fnv1a:2574d9d5d0fadfc4","code":"xml/xinclude-too-deep","stage":"xml","message":"<xi:include> nesting exceeds depth limit of 10"}"#,
            ),
            (
                "forge/xinclude-malformed",
                XmlError::XInclude(crate::xinclude::XIncludeError::Malformed {
                    href: "frag.xml".into(),
                    detail: "unexpected end tag".into(),
                })
                .into(),
                r#"{"v":1,"id":"fnv1a:2ed3aed0e1159c97","code":"xml/xinclude-malformed","stage":"xml","message":"<xi:include href=\"frag.xml\">: included file is malformed: unexpected end tag","actual":"frag.xml"}"#,
            ),
            (
                "forge/xinclude-unsupported",
                XmlError::XInclude(crate::xinclude::XIncludeError::Unsupported {
                    href: "frag.xml".into(),
                    feature: "parse=\"text\" (only parse=\"xml\" is supported)",
                })
                .into(),
                r#"{"v":1,"id":"fnv1a:76b67406e28c984e","code":"xml/xinclude-unsupported","stage":"xml","message":"<xi:include href=\"frag.xml\">: unsupported feature: parse=\"text\" (only parse=\"xml\" is supported)","actual":"parse=\"text\" (only parse=\"xml\" is supported)"}"#,
            ),
            // sce:template preprocessing variants. Each distinct
            // repair shape (missing-param → add_attribute, cycle /
            // too-deep → structural, not-found / read-error →
            // filesystem, malformed / unknown-param → content)
            // rides its own golden so byte-stability drift reveals
            // the specific variant.
            (
                "forge/template-not-found",
                XmlError::Template(crate::template::TemplateError::NotFound {
                    template: "guard.sce-template.xml".into(),
                    searched: "/project/src/guard.sce-template.xml".into(),
                })
                .into(),
                r#"{"v":1,"id":"fnv1a:8908a8dea70619f7","code":"xml/template-not-found","stage":"xml","message":"<sce:use template=\"guard.sce-template.xml\">: file not found (searched: /project/src/guard.sce-template.xml)","actual":"guard.sce-template.xml"}"#,
            ),
            (
                "forge/template-read-error",
                XmlError::Template(crate::template::TemplateError::ReadError {
                    template: "guard.sce-template.xml".into(),
                    source: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
                })
                .into(),
                r#"{"v":1,"id":"fnv1a:51f1b394ed4bd273","code":"xml/template-read-error","stage":"xml","message":"<sce:use template=\"guard.sce-template.xml\">: cannot read: permission denied","actual":"guard.sce-template.xml"}"#,
            ),
            (
                "forge/template-malformed",
                XmlError::Template(crate::template::TemplateError::Malformed {
                    template: "bad.sce-template.xml".into(),
                    detail: "root element must be <sce:template>, got <not-a-template>".into(),
                })
                .into(),
                r#"{"v":1,"id":"fnv1a:94db895eb4789a8d","code":"xml/template-malformed","stage":"xml","message":"<sce:use template=\"bad.sce-template.xml\">: template is malformed: root element must be <sce:template>, got <not-a-template>","actual":"bad.sce-template.xml"}"#,
            ),
            (
                "forge/template-missing-attribute",
                XmlError::Template(crate::template::TemplateError::MissingTemplateAttribute)
                    .into(),
                r#"{"v":1,"id":"fnv1a:0cace1208b4c7470","code":"xml/template-missing-attribute","stage":"xml","message":"<sce:use> missing required `template` attribute","fix":{"kind":"add_attribute","element":"sce:use","attr":"template"}}"#,
            ),
            (
                "forge/template-missing-param",
                XmlError::Template(crate::template::TemplateError::MissingParam {
                    template: "guard.sce-template.xml".into(),
                    param: "port".into(),
                })
                .into(),
                r#"{"v":1,"id":"fnv1a:58b212f0a042eeea","code":"xml/template-missing-param","stage":"xml","message":"<sce:use template=\"guard.sce-template.xml\">: missing required parameter 'port'","actual":"port","fix":{"kind":"add_attribute","element":"sce:use","attr":"port"}}"#,
            ),
            (
                "forge/template-unknown-param",
                XmlError::Template(crate::template::TemplateError::UnknownParam {
                    template: "guard.sce-template.xml".into(),
                    param: "typo".into(),
                    declared: "port, proto".into(),
                })
                .into(),
                r#"{"v":1,"id":"fnv1a:6f7f0531e1ab2dad","code":"xml/template-unknown-param","stage":"xml","message":"<sce:use template=\"guard.sce-template.xml\">: unknown parameter 'typo' (declared: port, proto)","actual":"typo"}"#,
            ),
            (
                "forge/template-cycle",
                XmlError::Template(crate::template::TemplateError::Cycle {
                    template: "a.sce-template.xml".into(),
                    chain: "/a.sce-template.xml → /b.sce-template.xml → /a.sce-template.xml".into(),
                })
                .into(),
                r#"{"v":1,"id":"fnv1a:9cbb70932a16f657","code":"xml/template-cycle","stage":"xml","message":"<sce:use template=\"a.sce-template.xml\">: cycle detected (/a.sce-template.xml → /b.sce-template.xml → /a.sce-template.xml)","actual":"a.sce-template.xml"}"#,
            ),
            (
                "forge/template-too-deep",
                XmlError::Template(crate::template::TemplateError::TooDeep {
                    limit: crate::template::MAX_TEMPLATE_DEPTH,
                })
                .into(),
                r#"{"v":1,"id":"fnv1a:b3b0c2c9723a4ca2","code":"xml/template-too-deep","stage":"xml","message":"<sce:use> template nesting exceeds depth limit of 10"}"#,
            ),
        ]
    }

    /// Multi-record emitter goldens. `XsdErrors` is the one production
    /// type where Display (a per-violation `file:line: message` line)
    /// deliberately diverges from JSON `message` (bare libxml2 text).
    /// Documented in `xsd_validator.rs`: each violation rides its own
    /// record with `location.line` set — the inline file:line form in
    /// Display is the editor-friendly counterpart.
    ///
    /// Participates in [`diagnostic_goldens_are_byte_stable`] and
    /// [`every_code_has_a_golden`], but **not** in
    /// [`human_mode_matches_json_message`] — that invariant does not
    /// hold by construction here.
    fn xsd_golden_entries() -> Vec<(&'static str, ForgeError, &'static str)> {
        use crate::forge::error::XmlError;
        use crate::forge::xsd_validator::{XsdDiag, XsdErrors};
        vec![
            (
                "forge/xml-schema-validation",
                ForgeError::Xml(XmlError::SchemaValidation(XsdErrors {
                    source_label: "chart.scxml".into(),
                    diagnostics: vec![XsdDiag {
                        line: Some(7),
                        col: None,
                        message: "Element 'sce:field': missing required attribute 'id'.".into(),
                    }],
                })),
                r#"{"v":1,"id":"fnv1a:cd97e1d8cb41cb8c","code":"xml/schema-validation","stage":"xml","spec":"SCE Forge XSD","message":"Element 'sce:field': missing required attribute 'id'.","location":{"file":"chart.scxml","line":7}}"#,
            ),
        ]
    }

    /// Shared golden table for first-party `MeshError` cases. See
    /// [`forge_golden_entries`] for the rationale; same shape.
    fn mesh_golden_entries() -> Vec<(&'static str, crate::mesh::error::MeshError, &'static str)> {
        use crate::mesh::error::{
            CodegenError, DeployError, ExternalConfigError, MeshError, RpcClientKind,
            TopologyError, UnresolvedName,
        };
        use crate::mesh::pattern::{CommunicationPattern, PatternViolation};
        use crate::mesh::target::TargetId;
        use crate::mesh::topology::EventCoverageWarning;
        use crate::mesh::transport::TransportCapability;
        vec![
            (
                "mesh/deploy-read",
                DeployError::ReadFile {
                    path: "deploy.yaml".into(),
                    source: std::io::Error::from(std::io::ErrorKind::NotFound),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:bb3b4a7b769b5da5","code":"mesh/deploy-read","stage":"mesh-deploy","message":"cannot read deploy config 'deploy.yaml': entity not found","actual":"deploy.yaml"}"#,
            ),
            (
                "mesh/deploy-parse",
                DeployError::Yaml("line 3: unexpected token".into()).into(),
                r#"{"v":1,"id":"fnv1a:e4569cf099b99bad","code":"mesh/deploy-parse","stage":"mesh-deploy","spec":"SCE Mesh §14","message":"deploy.yaml parse error: line 3: unexpected token"}"#,
            ),
            (
                "mesh/deploy-unsupported-version",
                DeployError::UnsupportedVersion {
                    found: "99".into(),
                    supported: vec!["1"],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:1ca45c4f97a6bbad","code":"mesh/deploy-unsupported-version","stage":"mesh-deploy","spec":"SCE Mesh §14","message":"deploy.yaml version '99' is not supported. Supported: 1. Update sce-codegen or change the `version:` field.","actual":"99","fix":{"kind":"replace_one_of","candidates":["1"]}}"#,
            ),
            (
                "mesh/deploy-duplicate-machine",
                DeployError::DuplicateMachine {
                    machine: "motor".into(),
                    devices: vec!["ecu_a".into(), "ecu_b".into()],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:2696f6f9c7da22b8","code":"mesh/deploy-duplicate-machine","stage":"mesh-deploy","spec":"SCE Mesh §14","message":"machine 'motor' is declared on multiple devices: ecu_a, ecu_b. Machine names must be globally unique across the deployment.","actual":"motor"}"#,
            ),
            (
                "mesh/deploy-invalid-ordering-timings",
                DeployError::InvalidOrderingTimings {
                    machine: "brake".into(),
                    reason: "tick_period_ms (100) must be strictly less than gap_timeout_ms (100) so a missed sequence is detected within `gap_timeout + tick_period`".into(),
                }
                .into(),
                // Hash placeholder — will be replaced with the runtime-
                // observed value on first failure of the byte-stability
                // assertion. The shape is what's pinned here.
                r#"{"v":1,"id":"fnv1a:8dedacb2b46600f4","code":"mesh/deploy-invalid-ordering-timings","stage":"mesh-deploy","spec":"SCE Mesh §10.6","message":"machine 'brake': invalid `ordering:` section in deploy.yaml — tick_period_ms (100) must be strictly less than gap_timeout_ms (100) so a missed sequence is detected within `gap_timeout + tick_period`. Either fix the values or omit the section entirely to accept the defaults.","actual":"brake"}"#,
            ),
            (
                "mesh/deploy-invalid-liveliness",
                DeployError::InvalidLiveliness {
                    machine: "brake".into(),
                    reason: "lease_ms (50) must be >= 100 ms — values below this floor race Zenoh's own keepalive and generate spurious DELETE/PUT churn".into(),
                }
                .into(),
                // Hash placeholder — will be replaced with the runtime-
                // observed value on first failure of the byte-stability
                // assertion. The shape is what's pinned here.
                r#"{"v":1,"id":"fnv1a:45153e0eac48ec1b","code":"mesh/deploy-invalid-liveliness","stage":"mesh-deploy","spec":"SCE Mesh §16.7","message":"machine 'brake': invalid `liveliness:` section in deploy.yaml — lease_ms (50) must be >= 100 ms — values below this floor race Zenoh's own keepalive and generate spurious DELETE/PUT churn. Either fix the value or omit the section entirely to disable liveliness.","actual":"brake"}"#,
            ),
            (
                "mesh/deploy-invalid-server-query-timeout",
                DeployError::InvalidServerQueryTimeout {
                    machine: "motor".into(),
                    reason: "query_timeout_ms (5) must be >= 10 ms — values below this floor race typical engine macrostep latency and would cause every inbound query to time out before the engine can respond".into(),
                }
                .into(),
                // Hash placeholder — the byte-stability assertion patches
                // it on first run. Shape + message are the contract.
                r#"{"v":1,"id":"fnv1a:f3bf5c36574e7396","code":"mesh/deploy-invalid-server-query-timeout","stage":"mesh-deploy","spec":"SCE Mesh §9.5","message":"machine 'motor': invalid `server.query_timeout_ms` in deploy.yaml — query_timeout_ms (5) must be >= 10 ms — values below this floor race typical engine macrostep latency and would cause every inbound query to time out before the engine can respond. Either fix the value or omit the knob entirely to disable the server deadline.","actual":"motor"}"#,
            ),
            (
                "mesh/deploy-invalid-outbound-buffer",
                DeployError::InvalidOutboundBuffer {
                    machine: "brake".into(),
                    reason: "max_pending_per_target (0) must be >= 1 — a zero-capacity buffer cannot hold any envelope, which is indistinguishable from the pre-§10.10 silent-drop behaviour; omit the section entirely to opt out of buffering instead".into(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:a62483a5bfc65457","code":"mesh/deploy-invalid-outbound-buffer","stage":"mesh-deploy","spec":"SCE Mesh §10.10","message":"machine 'brake': invalid `outbound_buffer:` section in deploy.yaml — max_pending_per_target (0) must be >= 1 — a zero-capacity buffer cannot hold any envelope, which is indistinguishable from the pre-§10.10 silent-drop behaviour; omit the section entirely to opt out of buffering instead. Either fix the value or omit the section entirely to opt out of §10.10 buffering.","actual":"brake"}"#,
            ),
            (
                "mesh/deploy-discovery-not-supported",
                DeployError::DiscoveryNotSupported {
                    content_kind: "object with keys [mode, resolution]".into(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:b515214da13d33fa","code":"mesh/deploy-discovery-not-supported","stage":"mesh-deploy","spec":"SCE Mesh §3.3","message":"deploy.yaml 'discovery:' top-level block is not supported (object with keys [mode, resolution]). SCE Mesh §3.3 invariant: transport-native routing is the source of truth for peer availability; SCE does not maintain a peer table (§2572 rejected list, §2574 rejection of `discovery.mode: static | dynamic`). For per-binding runtime target selection use value-field placeholders (§14.4). For transport-level peer discovery configure the external OEM config (zenoh.json5 scouting, vsomeip.json service-discovery).","actual":"object with keys [mode, resolution]"}"#,
            ),
            (
                "mesh/deploy-pool-not-supported-by-transport",
                DeployError::PoolNotSupportedByTransport {
                    machine: "brake".into(),
                    binding: "#logger".into(),
                    transport: "shm".into(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:d6c4a65cf22dfccc","code":"mesh/deploy-pool-not-supported-by-transport","stage":"mesh-deploy","spec":"SCE Mesh §14.4","message":"machine 'brake': binding '#logger' on transport 'shm' carries a '{name}' placeholder, but this transport does not support pool bindings (supports_pool = false). Use a routing-capable transport (zenoh, someip) or drop the placeholder.","actual":"brake"}"#,
            ),
            (
                "mesh/deploy-pool-missing-instance-list",
                DeployError::PoolMissingInstanceList {
                    machine: "brake".into(),
                    binding: "#player".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:0bcba658bc670781","code":"mesh/deploy-pool-missing-instance-list","stage":"mesh-deploy","spec":"SCE Mesh §14.4","message":"machine 'brake': SOME/IP binding '#player' uses a '{name}' placeholder but is missing the required `instances:` list. vsomeip does not support open-ended instance subscription; declare the expected instance IDs explicitly.","actual":"brake"}"#,
            ),
            (
                "mesh/deploy-pool-empty-instance-list",
                DeployError::PoolEmptyInstanceList {
                    machine: "brake".into(),
                    binding: "#player".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:1bacaac533ee1cea","code":"mesh/deploy-pool-empty-instance-list","stage":"mesh-deploy","spec":"SCE Mesh §14.4","message":"machine 'brake': binding '#player' has an empty `instances: []` list. Declare at least one instance ID or remove the list entirely.","actual":"brake"}"#,
            ),
            (
                "mesh/deploy-pool-invalid-placeholder",
                DeployError::PoolInvalidPlaceholder {
                    machine: "brake".into(),
                    binding: "#player".into(),
                    reason: "unbalanced '{' at byte 10 — every '{' must have a matching '}' within the same value".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:d4345eb44590146d","code":"mesh/deploy-pool-invalid-placeholder","stage":"mesh-deploy","spec":"SCE Mesh §14.4","message":"machine 'brake': binding '#player' has an invalid placeholder — unbalanced '{' at byte 10 — every '{' must have a matching '}' within the same value. Fix the placeholder syntax or escape intended literal braces.","actual":"brake"}"#,
            ),
            (
                "mesh/deploy-server-pool-not-supported",
                DeployError::ServerPoolNotSupported {
                    machine: "motor".into(),
                    transport: "zenoh".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:5f473bdda8049652","code":"mesh/deploy-server-pool-not-supported","stage":"mesh-deploy","spec":"SCE Mesh §14.4","message":"machine 'motor': `server.instances:` is not supported on transport 'zenoh' — only transports with a peer-identifying inbound distinguisher (SOME/IP today) can host a multi-instance server pool. Drop `instances:` from the server section, switch the server transport to one that supports pools, or run N processes each hosting a single-instance server. See SCE_MESH.md §14.4.","actual":"motor","fix":{"kind":"remove_fields","location":"topology.*.machines.motor.server","fields":["instances"]}}"#,
            ),
            (
                "mesh/deploy-scxml-invoke-target-conflict",
                DeployError::ScxmlInvokeTargetConflict {
                    machine: "worker".into(),
                    inbound_peers: vec!["parent_mesh".into()],
                    local_invoker: "parent_local".into(),
                    local_src: "worker.scxml".into(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion on first run.
                r##"{"v":1,"id":"fnv1a:8891aa8035b7c7da","code":"mesh/deploy-scxml-invoke-target-conflict","stage":"mesh-deploy","spec":"SCE Mesh §9.6","message":"machine 'worker' is both a remote `<invoke type=\"scxml\" src=\"#worker\">` target (mesh peer, inbound from: parent_mesh) and a local-path invoke target of machine 'parent_local' (src=\"worker.scxml\"). These two shapes cannot coexist: the mesh peer shape is default-constructible for SCE_MESH.md §9.6 `ChildSessionAdapter<Engine>`, while the local shape carries a `ParentStateMachine` template parameter. Fix: drop one — either change 'parent_local' to invoke '#worker' through mesh, or remove 'worker' from deploy.yaml topology.","actual":"worker"}"##,
            ),
            (
                "mesh/deploy-partition-duplicate-name",
                DeployError::PartitionDuplicateName {
                    name: "brake_main".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:b397cf843f9bb794","code":"mesh/deploy-partition-duplicate-name","stage":"mesh-deploy","spec":"SCE Mesh §14","message":"partition name 'brake_main' is declared more than once under `partitions:`. Partition names are globally unique process identities (SCE_MESH.md §14 rule 6). Rename one of the entries or delete the duplicate.","actual":"brake_main"}"#,
            ),
            (
                "mesh/deploy-partition-multi-device",
                DeployError::PartitionMultiDevice {
                    partition: "cross_part".into(),
                    devices: vec!["ecu_a".into(), "ecu_b".into()],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:aade5f5b0da2ca31","code":"mesh/deploy-partition-multi-device","stage":"mesh-deploy","spec":"SCE Mesh §14","message":"partition 'cross_part': its `machines:` list spans more than one device (ecu_a, ecu_b). A partition is one process on one device (SCE_MESH.md §14 rule 7). Split the partition into one entry per device, or narrow `machines:` to a single-device set.","actual":"cross_part"}"#,
            ),
            (
                "mesh/deploy-partition-unit-duplicate",
                DeployError::PartitionUnitDuplicate {
                    unit: "parallel_region:brake/shared".into(),
                    partitions: vec!["part_a".into(), "part_b".into()],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:6bb09cb8cb802106","code":"mesh/deploy-partition-unit-duplicate","stage":"mesh-deploy","spec":"SCE Mesh §14","message":"unit 'parallel_region:brake/shared' appears in more than one partition (part_a, part_b). Each orthogonal unit belongs to exactly one partition (SCE_MESH.md §14 rule 8). Remove the entry from every partition except the intended one.","actual":"parallel_region:brake/shared"}"#,
            ),
            (
                "mesh/deploy-partition-machine-not-listed",
                DeployError::PartitionMachineNotListed {
                    partition: "brake_only".into(),
                    machine: "motor".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:71de62a977d18b9f","code":"mesh/deploy-partition-machine-not-listed","stage":"mesh-deploy","spec":"SCE Mesh §14","message":"partition 'brake_only': `contains:` entry references machine 'motor', but 'motor' is not listed under the partition's `machines:` field. Add 'motor' to `machines:` or remove the stray entry (SCE_MESH.md §14 rule 9).","actual":"motor"}"#,
            ),
            (
                "mesh/deploy-partition-empty",
                DeployError::PartitionEmpty {
                    partition: "empty_part".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:5c3f22193c998160","code":"mesh/deploy-partition-empty","stage":"mesh-deploy","spec":"SCE Mesh §14","message":"partition 'empty_part' is empty (no `contains.parallel_regions:` and no `contains.invokes:`). Empty partitions have no runtime purpose (SCE_MESH.md §14 rule 10); either add the units this partition hosts or delete the entry.","actual":"empty_part"}"#,
            ),
            (
                "mesh/deploy-partition-synth-infix-collision",
                DeployError::PartitionSynthInfixCollision {
                    machine: "parent__sce_synth_invoke__child".into(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion
                // on first run; shape + message are the contract.
                r#"{"v":1,"id":"fnv1a:6c17e971ff381f93","code":"mesh/deploy-partition-synth-infix-collision","stage":"mesh-deploy","spec":"SCE Mesh §14","message":"machine 'parent__sce_synth_invoke__child' uses the reserved `__sce_synth_invoke__` infix in its name. SCE Mesh §14 rule 5 reserves this substring for machines synthesised from `<invoke type=\"scxml\">` inline `<content>` (§9.6.6); an author id collision would silently shadow the synthesised peer at runtime. Rename the machine to drop the substring.","actual":"parent__sce_synth_invoke__child"}"#,
            ),
            (
                "mesh/deploy-partition-uncovered-unit",
                DeployError::PartitionUncoveredUnit {
                    machine: "brake".into(),
                    units: vec![
                        "parallel_region:brake/monitoring".into(),
                        "invoke:brake/compute_force_inv".into(),
                    ],
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:0ccf7ab1ec88806c","code":"mesh/deploy-partition-uncovered-unit","stage":"mesh-deploy","spec":"SCE Mesh §14","message":"machine 'brake' has partitions declared but the following orthogonal units are not covered by any partition's `contains:`: \n  - parallel_region:brake/monitoring\n  - invoke:brake/compute_force_inv. The 'brake_default' partition exists, so the direct repair is to extend its `contains:` with the missing entries (SCE_MESH.md §14 rule 1).","actual":"brake"}"#,
            ),
            (
                "mesh/deploy-partition-partial-coverage-requires-default",
                DeployError::PartitionPartialCoverageRequiresDefault {
                    machine: "brake".into(),
                    missing: vec![
                        "parallel_region:brake/monitoring".into(),
                        "invoke:brake/compute_force_inv".into(),
                    ],
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:f4c724f239f9f76f","code":"mesh/deploy-partition-partial-coverage-requires-default","stage":"mesh-deploy","spec":"SCE Mesh §14","message":"machine 'brake' has partitions declared, but the following orthogonal units are unassigned:\n              - parallel_region:brake/monitoring\n              - invoke:brake/compute_force_inv\n            Either add them to an existing partition under `machines: [brake]`, or declare a 'brake_default' partition with `contains:` entries for each (SCE_MESH.md §14 rule 2).","actual":"brake"}"#,
            ),
            (
                "mesh/deploy-partition-pool-machine",
                DeployError::PartitionPoolMachine {
                    machine: "motor".into(),
                    partition: "motor_region_a".into(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion
                // on first run; shape + message are the contract.
                r#"{"v":1,"id":"fnv1a:02f99e5ae78bf9e8","code":"mesh/deploy-partition-pool-machine","stage":"mesh-deploy","spec":"SCE Mesh §14","message":"machine 'motor' declares `server.instances:` (SCE Mesh §14.4 SOME/IP server pool) but partition 'motor_region_a' lists it under `machines:`. A pool is one router hosting N SOME/IP sessions on a single process; a partition splits a machine across M OS processes (SCE_MESH.md §14). deploy.yaml does not define the combined meaning — either remove 'motor' from partition 'motor_region_a' `machines:` (keep the pool as one monolithic process), or drop `server.instances:` from the machine and run N processes each hosting a single-instance server.","actual":"motor"}"#,
            ),
            (
                "mesh/deploy-partition-transport-binding-unsupported",
                DeployError::PartitionTransportBindingUnsupported {
                    partition: "motor_left".into(),
                    transport: "zenoh".into(),
                    failure: crate::mesh::error::PartitionTransportBindingFailure::Incapable {
                        transport: "zenoh".into(),
                    },
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion
                // on first run; shape + message are the contract.
                r#"{"v":1,"id":"fnv1a:1e038670d04a2bdb","code":"mesh/deploy-partition-transport-binding-unsupported","stage":"mesh-deploy","spec":"SCE Mesh §14","message":"partition 'motor_left': `transport_binding: zenoh` is not a valid inter-partition IPC transport — transport 'zenoh' does not carry inter-partition IPC (supports_inter_partition_ipc = false). SCE Mesh §14 requires a transport whose primary purpose is same-machine IPC (today: shm, custom_tcp). Switch to one of those or omit `transport_binding:` to accept the default (§14 L2730 \"kind tcp/shm\").","actual":"motor_left"}"#,
            ),
            (
                "mesh/deploy-scxml-invoke-cross-device-transport",
                DeployError::ScxmlInvokeCrossDeviceTransport {
                    parent: "parent_x".into(),
                    peer: "worker_y".into(),
                    parent_device: "ecu_a".into(),
                    peer_device: "ecu_b".into(),
                    failure: crate::mesh::error::ScxmlInvokeCrossDeviceFailure::MissingBinding,
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion
                // on first run; shape + message are the contract.
                // r##"..."## because the `<invoke src=\"#worker_y\">`
                // payload contains `"#` which would close r#"..."# early.
                r##"{"v":1,"id":"fnv1a:612c7ce1ce735e8d","code":"mesh/deploy-scxml-invoke-cross-device-transport","stage":"mesh-deploy","spec":"SCE Mesh §9.6 L1393","message":"machine 'parent_x' (device 'ecu_a') → `<invoke type=\"scxml\" src=\"#worker_y\">` on device 'ecu_b': parent declares no `bindings[\"#<peer>\"]` entry for the cross-device peer. SCE Mesh §9.6 L1393 requires each cross-device scxml-remote peer to declare its transport on `machines.parent_x.bindings[\"#worker_y\"].transport`, and that transport must be both capable of crossing devices AND wired by the Session F C++ dispatch.","actual":"parent_x/worker_y"}"##,
            ),
            (
                "mesh/deploy-someip-scxml-invoke-service-id-overflow",
                DeployError::SomeipScxmlInvokeServiceIdOverflow {
                    participant_count: 129,
                    ceiling: 128,
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion
                // on first run; shape + message are the contract.
                r#"{"v":1,"id":"fnv1a:e5d3768d0e062066","code":"mesh/deploy-someip-scxml-invoke-service-id-overflow","stage":"mesh-deploy","spec":"SCE Mesh §9.6","message":"§9.6 SOME/IP scxml-invoke service-ID overflow: 129 participants exceed the 128-slot sub-range ceiling [0x8100, 0x817F] (RFC F.X-1 subsystem range partitioning reserves [0x8180, 0x81FF] for §16.4 region-liveness). Reduce the §9.6 SOMEIP participant count or split deploy.yaml across multi-OEM domains (multi-domain support is a separate landing).","actual":"129"}"#,
            ),
            (
                "mesh/deploy-someip-scxml-invoke-service-id-pin-out-of-range",
                DeployError::SomeipScxmlInvokeServiceIdPinOutOfRange {
                    machine: "brake".into(),
                    pinned_id: 0x8180,
                    range_lo: 0x8100,
                    range_hi: 0x817F,
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion
                // on first run; shape + message are the contract.
                r#"{"v":1,"id":"fnv1a:31a71109b709256a","code":"mesh/deploy-someip-scxml-invoke-service-id-pin-out-of-range","stage":"mesh-deploy","spec":"SCE Mesh §9.6","message":"machine 'brake': pinned `someip_service_id: 0x8180` is outside the §9.6 SOMEIP scxml-invoke sub-range [0x8100, 0x817f] (RFC F.X-1). The upper half of the SCE-reserved range is reserved for §16.4 region-liveness; pins outside the SCE-reserved range collide with OEM-owned service space. Pick a value inside [0x8100, 0x817f] or drop the pin to use the auto-assigner.","actual":"0x8180"}"#,
            ),
            (
                "mesh/deploy-someip-scxml-invoke-service-id-pin-collision",
                DeployError::SomeipScxmlInvokeServiceIdPinCollision {
                    machines: vec!["alpha".into(), "beta".into()],
                    pinned_id: 0x8105,
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion
                // on first run; shape + message are the contract.
                r#"{"v":1,"id":"fnv1a:aad388fcf85f5fd0","code":"mesh/deploy-someip-scxml-invoke-service-id-pin-collision","stage":"mesh-deploy","spec":"SCE Mesh §9.6","message":"§9.6 SOME/IP scxml-invoke service-ID pin collision at 0x8105: machines ['alpha', 'beta'] all pin the same value via deploy.yaml `someip_service_id:`. Each pin must be unique inside the [0x8100, 0x817F] sub-range. Repick the pin on one of the listed machines or drop a pin to fall back to the counter auto-assigner.","actual":"0x8105"}"#,
            ),
            (
                "mesh/deploy-partition-barrier-timeout-invalid",
                DeployError::PartitionBarrierTimeoutInvalid {
                    partition: "motor_left".into(),
                    value: 0,
                    reason: "barrier_timeout_ms (0) would fire the §16.5 parallel-final barrier before any region can report ParallelRegionDone".into(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion
                // on first run; shape + message are the contract.
                r#"{"v":1,"id":"fnv1a:2b289d036acef025","code":"mesh/deploy-partition-barrier-timeout-invalid","stage":"mesh-deploy","spec":"SCE Mesh §14","message":"partition 'motor_left': `barrier_timeout_ms: 0` is invalid — barrier_timeout_ms (0) would fire the §16.5 parallel-final barrier before any region can report ParallelRegionDone. SCE Mesh §14 L2731-2732 pins the W3C normative default as infinity (null / field omitted); finite values must be >= 1 ms. Either fix the value or drop the key to accept the default.","actual":"motor_left"}"#,
            ),
            (
                "mesh/partition-parallel-root-undesignated",
                DeployError::PartitionParallelRootUndesignated {
                    machine: "motor".into(),
                    parallel: "root".into(),
                    hosting_partitions: vec!["motor_left".into(), "motor_right".into()],
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion
                // on first run; shape + message are the contract.
                r#"{"v":1,"id":"fnv1a:3e54ff7bc3314b81","code":"mesh/partition-parallel-root-undesignated","stage":"mesh-deploy","spec":"SCE Mesh §14 rule 12","message":"machine 'motor': distributed `<parallel id=\"root\">` (regions span partitions 'motor_left', 'motor_right') has no root claimant. SCE Mesh §14 rule 12 requires exactly one partition to declare `hosts_parallel_roots: [{ machine: motor, parallel: root }]`. Add the entry to one of the listed partitions — the root must co-host at least one region of the parallel.","actual":"motor/root"}"#,
            ),
            (
                "mesh/partition-parallel-root-ambiguous",
                DeployError::PartitionParallelRootAmbiguous {
                    machine: "motor".into(),
                    parallel: "root".into(),
                    claiming_partitions: vec!["motor_left".into(), "motor_right".into()],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:edd08e9e1ce92654","code":"mesh/partition-parallel-root-ambiguous","stage":"mesh-deploy","spec":"SCE Mesh §14 rule 12","message":"machine 'motor': `<parallel id=\"root\">` is claimed as root by multiple partitions: 'motor_left', 'motor_right'. SCE Mesh §14 rule 12 requires exactly one claimant per distributed parallel. Remove the entry from all but one partition's `hosts_parallel_roots:`.","actual":"motor/root"}"#,
            ),
            (
                "mesh/partition-parallel-root-not-in-machines",
                DeployError::PartitionParallelRootNotInMachines {
                    partition: "motor_left".into(),
                    claimed_machine: "brake".into(),
                    partition_machines: vec!["motor".into()],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:2c9a433f272223bc","code":"mesh/partition-parallel-root-not-in-machines","stage":"mesh-deploy","spec":"SCE Mesh §14 rule 12","message":"partition 'motor_left': `hosts_parallel_roots:` entry claims machine 'brake' but the partition's `machines:` list is ['motor']. SCE Mesh §14 rule 12 applies rule 9 shape to root entries — the claimed machine must be one the partition already lists. Either add 'brake' to `machines:` or move the `hosts_parallel_roots:` entry to a partition that already lists it.","actual":"motor_left"}"#,
            ),
            (
                "mesh/partition-parallel-root-non-host",
                DeployError::PartitionParallelRootNonHost {
                    partition: "motor_default".into(),
                    machine: "motor".into(),
                    parallel: "root".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:62c032a8a89709d9","code":"mesh/partition-parallel-root-non-host","stage":"mesh-deploy","spec":"SCE Mesh §14 rule 12","message":"partition 'motor_default': claims root for machine 'motor' `<parallel id=\"root\">` but hosts no region of that parallel in `contains.parallel_regions:`. SCE Mesh §14 rule 12 requires a root claimant to co-host at least one region — otherwise every region update crosses partitions as inter-partition traffic. Either add a region of the parallel to this partition's `contains:`, or move the `hosts_parallel_roots:` entry to a partition that already hosts one.","actual":"motor_default"}"#,
            ),
            (
                "mesh/partition-barrier-timeout-without-root",
                DeployError::PartitionBarrierTimeoutWithoutRoot {
                    partition: "motor_right".into(),
                    value: 5000,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:2c7305b9bb84a6e1","code":"mesh/partition-barrier-timeout-without-root","stage":"mesh-deploy","spec":"SCE Mesh §14 rule 12","message":"partition 'motor_right': `barrier_timeout_ms: 5000` is set but the partition has no `hosts_parallel_roots:` entries. SCE Mesh §14 rule 12 (L2842) requires the timeout to gate a §16.5 `ParallelCompletionTracker`, and trackers only exist on root-hosting partitions. Either add a `hosts_parallel_roots:` entry (making this partition a root) or drop `barrier_timeout_ms:` (which has no consumer here).","actual":"motor_right"}"#,
            ),
            (
                "mesh/partition-wire21-custom-tcp-unimplemented",
                DeployError::PartitionWire21CustomTcpUnimplemented {
                    partition: "motor_left".into(),
                    machine: "motor".into(),
                    parallel: "root".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:dae8841d93dfb4c6","code":"mesh/partition-wire21-custom-tcp-unimplemented","stage":"mesh-deploy","spec":"SCE Mesh §16.5","message":"partition 'motor_left': `transport_binding: custom_tcp` is set, but the partition participates in distributed `<parallel id=\"root\">` (machine 'motor') wire-21 routing. The §16.5 wire-21 channel emitter currently supports `transport_binding: shm` only — a `custom_tcp` channel is not yet generated for ParallelRegionDone forwarding. Either change this partition's `transport_binding:` to `shm` (same-device deployments), or remove the partition from any distributed `<parallel>` route until the custom_tcp wire-21 emitter lands.","expected":["shm"],"actual":"custom_tcp"}"#,
            ),
            (
                "mesh/distributability-r1-shared-write",
                DeployError::DistributabilityR1SharedWrite {
                    machine: "motor".into(),
                    parallel: "root".into(),
                    location: "shared".into(),
                    regions: vec!["left".into(), "right".into()],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:2aeb9f32f5c45f20","code":"mesh/distributability-r1-shared-write","stage":"mesh-deploy","spec":"SCE Mesh §16.3","message":"machine 'motor', `<parallel id=\"root\">`: R1 shared-write — regions 'left', 'right' all assign to ancestor data 'shared'. SCE_MESH.md §16.3 R1 forbids this because distribution cannot preserve W3C sequential consistency on shared writable state without cross-process locks. Either place these regions in the same partition or move the shared variable into per-region datamodels. (Set `distributability: permissive` in deploy.yaml to auto-merge instead of failing the build.)","actual":"shared"}"#,
            ),
            (
                "mesh/distributability-r2-cross-region-transition",
                DeployError::DistributabilityR2CrossRegionTransition {
                    machine: "motor".into(),
                    parallel: "root".into(),
                    regions: vec!["left".into(), "right".into()],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:2dcde5ac51ed902f","code":"mesh/distributability-r2-cross-region-transition","stage":"mesh-deploy","spec":"SCE Mesh §16.3","message":"machine 'motor', `<parallel id=\"root\">`: R2 cross-region transition — regions 'left', 'right' are connected by a transition that crosses the region boundary. SCE_MESH.md §16.3 R2 forbids this because distribution cannot preserve the W3C exit-set/enter-set computation atomically across partitions. Either merge the regions into one partition, or refactor the transition target to an ancestor of the `<parallel>` (which exits it wholesale and is distribution-safe). (Set `distributability: permissive` in deploy.yaml to auto-merge instead of failing the build.)"}"#,
            ),
            (
                "mesh/external-read",
                ExternalConfigError::Read {
                    path: "vsomeip.json".into(),
                    source: std::io::Error::from(std::io::ErrorKind::NotFound),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:aa558451e9e2d96a","code":"mesh/external-read","stage":"mesh-external","message":"cannot read external config 'vsomeip.json': entity not found","actual":"vsomeip.json"}"#,
            ),
            (
                "mesh/external-parse",
                ExternalConfigError::Parse {
                    path: "vsomeip.json".into(),
                    reason: "expected `}` at line 5".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:bb3ddd8b1609d557","code":"mesh/external-parse","stage":"mesh-external","message":"external config 'vsomeip.json' parse error: expected `}` at line 5"}"#,
            ),
            (
                "mesh/external-unresolved-names",
                ExternalConfigError::UnresolvedNames {
                    machine: "ecu_a".into(),
                    config_path: "vsomeip.json".into(),
                    missing: vec![UnresolvedName {
                        kind: "service",
                        name: "motor".into(),
                        context: None,
                    }],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:087408c4f27fcc9f","code":"mesh/external-unresolved-names","stage":"mesh-external","message":"deploy.yaml for machine 'ecu_a' references SOME/IP entities that do not exist in\nvsomeip.json:\n  - service \"motor\" → no match"}"#,
            ),
            (
                "mesh/external-ambiguous-event-group",
                ExternalConfigError::AmbiguousEventGroup {
                    machine: "ecu_a".into(),
                    target: "#motor".into(),
                    config_path: "vsomeip.json".into(),
                    event_group: "overspeed".into(),
                    count: 3,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:9bca8939ab487cd2","code":"mesh/external-ambiguous-event-group","stage":"mesh-external","message":"machine 'ecu_a': binding '#motor' references event_group 'overspeed' in 'vsomeip.json', which contains 3 events. Per-event fanout is not yet supported; declare a single-event group or add a per-event binding.","expected":["1"],"actual":"3"}"#,
            ),
            (
                "mesh/external-empty-event-group",
                ExternalConfigError::EmptyEventGroup {
                    machine: "ecu_a".into(),
                    target: "#motor".into(),
                    config_path: "vsomeip.json".into(),
                    event_group: "overspeed".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:80fb287b140e259b","code":"mesh/external-empty-event-group","stage":"mesh-external","message":"machine 'ecu_a': binding '#motor' references event_group 'overspeed' in 'vsomeip.json', which has no events declared. Add the event id in vsomeip.json."}"#,
            ),
            (
                "mesh/external-named-reference-without-config",
                ExternalConfigError::NamedReferenceWithoutConfig {
                    machine: "ecu_a".into(),
                    device: "dev0".into(),
                    target: "#motor".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:66693396e8f7200e","code":"mesh/external-named-reference-without-config","stage":"mesh-external","message":"machine 'ecu_a': binding '#motor' uses name-based SOME/IP references but device 'dev0' does not declare 'transports.someip.config:'. Add the vsomeip.json path to the device's transports block."}"#,
            ),
            (
                "mesh/external-reserved-someip-id-keys",
                ExternalConfigError::ReservedSomeipIdKeys {
                    machine: "ecu_a".into(),
                    target: "#motor".into(),
                    transport: "someip".into(),
                    fields: vec!["service_id", "method_id"],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:7fad79aaf5b8d7e8","code":"mesh/external-reserved-someip-id-keys","stage":"mesh-external","message":"machine 'ecu_a': binding '#motor' (transport: someip) uses reserved SOME/IP numeric-ID key(s) [\"service_id\", \"method_id\"]. deploy.yaml does not declare numeric IDs directly — for SOME/IP bindings reference names against `transports.someip.config:` (vsomeip.json); on other transports remove these keys.","fix":{"kind":"remove_fields","location":"machines.ecu_a.bindings.#motor","fields":["service_id","method_id"]}}"#,
            ),
            (
                "mesh/external-someip-field-on-non-someip-transport",
                ExternalConfigError::SomeipFieldOnNonSomeipTransport {
                    machine: "ecu_a".into(),
                    target: "#motor".into(),
                    transport: "zenoh".into(),
                    fields: vec!["service", "method"],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:a86f56b42fa5fd7e","code":"mesh/external-someip-field-on-non-someip-transport","stage":"mesh-external","message":"machine 'ecu_a': binding '#motor' uses transport 'zenoh' but declares SOME/IP-only fields [\"service\", \"method\"]. Either change the transport to 'someip' or remove the SOME/IP-specific fields.","actual":"zenoh","fix":{"kind":"replace_with","to":"someip"}}"#,
            ),
            (
                "mesh/external-conflicting-event-schema",
                ExternalConfigError::ConflictingEventSchema {
                    machine: "ecu_a".into(),
                    target: "#motor".into(),
                    flat_fields: vec!["method", "event_group"],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:85d4a28ba6402f73","code":"mesh/external-conflicting-event-schema","stage":"mesh-external","message":"machine 'ecu_a': binding '#motor' declares both flat fields ([\"method\", \"event_group\"]) and an 'events:' block. These are mutually exclusive — use 'events:' for per-event mappings, or the flat fields for a single mapping shared by every event on this target."}"#,
            ),
            (
                "mesh/external-conflicting-event-field-kinds",
                ExternalConfigError::ConflictingEventFieldKinds {
                    machine: "ecu_a".into(),
                    target: "#motor".into(),
                    event: "brake.activate".into(),
                    fields: vec!["method".into(), "getter".into()],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:b020ea734bf7eacc","code":"mesh/external-conflicting-event-field-kinds","stage":"mesh-external","message":"machine 'ecu_a': binding '#motor' event 'brake.activate' sets multiple field kinds ([\"method\", \"getter\"]). Each per-event entry must declare exactly one of method / event_group / getter / setter."}"#,
            ),
            (
                "mesh/external-empty-event-entry",
                ExternalConfigError::EmptyEventEntry {
                    machine: "ecu_a".into(),
                    target: "#motor".into(),
                    event: "brake.activate".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:1c24ba9d4506b785","code":"mesh/external-empty-event-entry","stage":"mesh-external","message":"machine 'ecu_a': binding '#motor' event 'brake.activate' declares no field. Each per-event entry must set exactly one of method / event_group / getter / setter."}"#,
            ),
            (
                "mesh/topology-unresolved-targets",
                TopologyError::UnresolvedTargets {
                    machine: "ecu_a".into(),
                    targets: vec![TargetId::new("#motor").unwrap()],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:7cf5ee3bd9f38fc1","code":"mesh/topology-unresolved-targets","stage":"mesh-topology","spec":"SCE Mesh §9","message":"unresolved send targets for machine 'ecu_a': #motor. Each <send target=\"...\"> in SCXML must have a corresponding binding in deploy.yaml"}"#,
            ),
            (
                "mesh/topology-machine-not-found",
                TopologyError::MachineNotFound {
                    machine: "ecu_z".into(),
                    available: vec!["ecu_a".into(), "ecu_b".into()],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:24627485b3d1728d","code":"mesh/topology-machine-not-found","stage":"mesh-topology","spec":"SCE Mesh §14","message":"machine 'ecu_z' not found in deploy.yaml topology. Available: ecu_a, ecu_b","actual":"ecu_z","fix":{"kind":"replace_one_of","candidates":["ecu_a","ecu_b"]}}"#,
            ),
            (
                "mesh/topology-receiver-not-declared",
                TopologyError::ReceiverNotDeclared {
                    sender: "ecu_a".into(),
                    target: TargetId::new("#motor").unwrap(),
                    receiver: "motor".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:0df0ff337a608f8b","code":"mesh/topology-receiver-not-declared","stage":"mesh-topology","spec":"SCE Mesh §9","message":"machine 'ecu_a' sends to '#motor' but no machine 'motor' is declared in deploy.yaml. Add the receiver under topology.*.machines with its `source:` path.","actual":"motor"}"#,
            ),
            (
                "mesh/topology-absolute-source-path",
                TopologyError::AbsoluteSourcePath {
                    machine: "motor".into(),
                    path: "/absolute/path.scxml".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:d48777da2ca3f441","code":"mesh/topology-absolute-source-path","stage":"mesh-topology","message":"machine 'motor' has absolute source path '/absolute/path.scxml'. Use a path relative to the deploy.yaml file instead.","actual":"/absolute/path.scxml"}"#,
            ),
            (
                "mesh/topology-receiver-source-read",
                TopologyError::ReceiverSourceRead {
                    machine: "motor".into(),
                    path: "motor.scxml".into(),
                    source: std::io::Error::from(std::io::ErrorKind::NotFound),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:b607ba8f6eede073","code":"mesh/topology-receiver-source-read","stage":"mesh-topology","message":"cannot read receiver SCXML 'motor.scxml' (for machine 'motor'): entity not found. Check the `source:` field in deploy.yaml for this machine.","actual":"motor.scxml"}"#,
            ),
            (
                "mesh/topology-receiver-source-parse",
                TopologyError::ReceiverSourceParse {
                    machine: "motor".into(),
                    path: "motor.scxml".into(),
                    reason: "unexpected end tag".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:665c9e5b41ae8ce7","code":"mesh/topology-receiver-source-parse","stage":"mesh-topology","message":"cannot parse receiver SCXML 'motor.scxml' (for machine 'motor'): unexpected end tag"}"#,
            ),
            (
                "mesh/topology-uncovered-events",
                TopologyError::UncoveredEvents {
                    sender: "ecu_a".into(),
                    findings: vec![EventCoverageWarning {
                        sender: "ecu_a".into(),
                        target: TargetId::new("#motor").unwrap(),
                        event: "brake.activate".into(),
                    }],
                }
                .into(),
                r##"{"v":1,"id":"fnv1a:40828afc2df752c2","code":"mesh/topology-uncovered-events","stage":"mesh-topology","spec":"SCE Mesh §9","message":"event coverage violations in machine 'ecu_a':\n  - send target=\"#motor\" event=\"brake.activate\" has no matching transition in 'motor'\nEach <send event=\"X\"> must have a matching <transition event=\"X\"> in the receiver. Fix: add the missing transition in the receiver, or correct the event name in the sender."}"##,
            ),
            (
                "mesh/topology-pattern-capability-violation",
                TopologyError::PatternCapabilityViolation {
                    sender: "ecu_a".into(),
                    violations: vec![PatternViolation {
                        state: "on".into(),
                        target: TargetId::new("#motor").unwrap(),
                        event: "service.request.stop".into(),
                        pattern: CommunicationPattern::ServiceRequest,
                        required: TransportCapability::RequestReply,
                        transport: "local_fire_forget".into(),
                    }],
                }
                .into(),
                r##"{"v":1,"id":"fnv1a:1facc4cb6fb444ba","code":"mesh/topology-pattern-capability-violation","stage":"mesh-topology","spec":"SCE Mesh §9","message":"pattern capability violations in machine 'ecu_a':\n  - send target=\"#motor\" event=\"service.request.stop\" uses pattern 'service.request' (requires request/reply capability), but transport 'local_fire_forget' does not support it\nEach communication pattern must be supported by the bound transport. Fix: change the transport in deploy.yaml, or use a different event pattern."}"##,
            ),
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
                "mesh/topology-invalid-binding-field",
                TopologyError::InvalidBindingField {
                    machine: "ecu_a".into(),
                    target: TargetId::new("#motor").unwrap(),
                    transport: "shm".into(),
                    field: "size".into(),
                    reason: "must be a power of two".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:17263f5574103176","code":"mesh/topology-invalid-binding-field","stage":"mesh-topology","spec":"SCE Mesh §14","message":"machine 'ecu_a': binding '#motor' (transport: shm) has invalid 'size': must be a power of two"}"#,
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
            (
                "mesh/topology-ordering-cannot-be-guaranteed",
                TopologyError::OrderingCannotBeGuaranteed {
                    machine: "ecu_a".into(),
                    target: TargetId::new("#motor").unwrap(),
                    transport: "can".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:ad421a502df8f70d","code":"mesh/topology-ordering-cannot-be-guaranteed","stage":"mesh-topology","spec":"SCE Mesh §10.6","message":"machine 'ecu_a': binding for '#motor' (transport: can) declares `ordering: required`, but 'can' is a broadcast bus whose semantics do not support per-(sender, receiver) sequence reconstruction (SCE Mesh §10.6.2). Either change the transport to a point-to-point one (e.g. local, shm, custom_tcp, someip, zenoh) or remove the `ordering: required` declaration from this binding.","actual":"can"}"#,
            ),
            (
                "mesh/topology-pool-param-name-missing",
                TopologyError::PoolParamNameMissing {
                    machine: "brake".into(),
                    target: TargetId::new("#player").unwrap(),
                    state: "s".into(),
                    invoke_id: "_invoke_0".into(),
                    missing: vec!["id".into()],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:a5b1eb818d07e3a7","code":"mesh/topology-pool-param-name-missing","stage":"mesh-topology","spec":"SCE Mesh §14.4","message":"machine 'brake': binding '#player' declares a runtime pool that needs <param> values [\"id\"] at every using <invoke>, but invoke '_invoke_0' in state 's' does not supply [\"id\"]. Add the missing <param>(s) to that invoke, or drop the placeholder / `instance_from:` from the binding.","actual":"_invoke_0"}"#,
            ),
            (
                "mesh/topology-subscription-source-unbound",
                TopologyError::SubscriptionSourceUnbound {
                    machine: "brake".into(),
                    source_target: "#ghost".into(),
                    available: vec![
                        TargetId::new("#motor").unwrap(),
                        TargetId::new("#chassis").unwrap(),
                    ],
                }
                .into(),
                r##"{"v":1,"id":"fnv1a:899faa900fbcb018","code":"mesh/topology-subscription-source-unbound","stage":"mesh-topology","spec":"SCE Mesh §13","message":"machine 'brake': subscription source '#ghost' has no matching binding. Available: #motor, #chassis. Add the source to machines.brake.bindings:, or drop the subscription from machines.brake.subscriptions:.","actual":"#ghost","fix":{"kind":"replace_one_of","candidates":["#motor","#chassis"]}}"##,
            ),
            (
                "mesh/topology-machine-lifetime-subscription-unsupported",
                TopologyError::MachineLifetimeSubscriptionUnsupported {
                    machine: "brake".into(),
                    source_target: TargetId::new("#motor").unwrap(),
                    event: "event.notification.status".into(),
                    transport: "someip".into(),
                }
                .into(),
                r##"{"v":1,"id":"fnv1a:a66a86a130ed11be","code":"mesh/topology-machine-lifetime-subscription-unsupported","stage":"mesh-topology","spec":"SCE Mesh §13","message":"machine 'brake': subscription on source '#motor' for event 'event.notification.status' uses transport 'someip', which does not support the machine-lifetime subscription path in this build. Move the binding to a transport that supports it (e.g. 'zenoh') or drop the subscription from machines.brake.subscriptions:.","actual":"someip"}"##,
            ),
            (
                "mesh/codegen-unsupported-language",
                CodegenError::UnsupportedLanguage("ruby".into()).into(),
                r#"{"v":1,"id":"fnv1a:7d6e0a4752f9975b","code":"mesh/codegen-unsupported-language","stage":"mesh-codegen","spec":"SCE Mesh §7","message":"mesh codegen not yet supported for language 'ruby'","actual":"ruby","fix":{"kind":"replace_one_of","candidates":["cpp"]}}"#,
            ),
            (
                "mesh/codegen-unsupported-transport",
                CodegenError::UnsupportedTransport {
                    transport: "carrier_pigeon".into(),
                    target: TargetId::new("#motor").unwrap(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:aa145685cde035e6","code":"mesh/codegen-unsupported-transport","stage":"mesh-codegen","spec":"SCE Mesh §8","message":"transport 'carrier_pigeon' not yet supported (target '#motor')","actual":"carrier_pigeon","fix":{"kind":"replace_one_of","candidates":["local","shm","someip","zenoh","custom_tcp"]}}"#,
            ),
            (
                "mesh/codegen-template-read",
                CodegenError::TemplateRead {
                    path: "mesh/someip.cpp.jinja2".into(),
                    source: std::io::Error::from(std::io::ErrorKind::NotFound),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:5a1fa3fbe4418aba","code":"mesh/codegen-template-read","stage":"mesh-codegen","message":"cannot read mesh template 'mesh/someip.cpp.jinja2': entity not found","actual":"mesh/someip.cpp.jinja2"}"#,
            ),
            (
                "mesh/codegen-template-render",
                CodegenError::TemplateRender("unknown filter `upper_camel`".into()).into(),
                r#"{"v":1,"id":"fnv1a:17464f8403ad70d9","code":"mesh/codegen-template-render","stage":"mesh-codegen","message":"mesh template render error: unknown filter `upper_camel`"}"#,
            ),
            (
                "mesh/codegen-event-name-collision",
                CodegenError::EventNameCollision {
                    target: TargetId::new("#motor").unwrap(),
                    suffix: "SERVICE_REQUEST_X".into(),
                    events: vec!["service.request.x".into(), "service-request-x".into()],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:97c61b17b678ca97","code":"mesh/codegen-event-name-collision","stage":"mesh-codegen","message":"target '#motor': SCXML events [\"service.request.x\", \"service-request-x\"] both map to the same C++ constant suffix 'SERVICE_REQUEST_X'. Rename one of the events (or use a per-event explicit mapping) so generated constants are unique.","actual":"SERVICE_REQUEST_X"}"#,
            ),
            (
                "mesh/codegen-pool-with-rpc-client-unsupported (mesh-rpc)",
                CodegenError::PoolWithRpcClientUnsupported {
                    machine: "motor_pool".into(),
                    kind: RpcClientKind::MeshRpc,
                }
                .into(),
                // Hash placeholder — the byte-stability assertion replaces
                // it with the runtime-observed value on first failure.
                // Shape + message are the contract.
                r#"{"v":1,"id":"fnv1a:3386d55e1a264fda","code":"mesh/codegen-pool-with-rpc-client-unsupported","stage":"mesh-codegen","message":"machine 'motor_pool': SOME/IP server pool (`server.instances: [...]` with more than one entry) cannot be combined with `<invoke type=\"sce:mesh-rpc\">` in the same router. Router-scoped correlation tables (`invoke_correlation_` / `active_invokes_` / `pending_rpcs_`) cannot safely alias across hosted sessions. Either remove the RPC client site(s) from this machine or reduce `server.instances:` to a single instance. See SCE_MESH.md §14.4.","actual":"motor_pool"}"#,
            ),
            (
                "mesh/codegen-pool-with-rpc-client-unsupported (someip-rpc-request)",
                CodegenError::PoolWithRpcClientUnsupported {
                    machine: "motor_pool".into(),
                    kind: RpcClientKind::SomeipRpcRequest,
                }
                .into(),
                // Distinct `id` from the MeshRpc arm above because the
                // rejection kind is part of `key_fragments`. Keeps the
                // two arms independently traceable downstream.
                r#"{"v":1,"id":"fnv1a:2b42825f2462bed8","code":"mesh/codegen-pool-with-rpc-client-unsupported","stage":"mesh-codegen","message":"machine 'motor_pool': SOME/IP server pool (`server.instances: [...]` with more than one entry) cannot be combined with SOME/IP `<send>` RpcRequest in the same router. Router-scoped correlation tables (`invoke_correlation_` / `active_invokes_` / `pending_rpcs_`) cannot safely alias across hosted sessions. Either remove the RPC client site(s) from this machine or reduce `server.instances:` to a single instance. See SCE_MESH.md §14.4.","actual":"motor_pool"}"#,
            ),
            (
                "mesh/io",
                MeshError::Io {
                    path: std::path::PathBuf::from("/tmp/mesh.log"),
                    source: std::io::Error::from(std::io::ErrorKind::NotFound),
                },
                r#"{"v":1,"id":"fnv1a:8d27e49c9b609f34","code":"mesh/io","stage":"io","message":"I/O error on /tmp/mesh.log: entity not found"}"#,
            ),
        ]
    }

    /// Production-routed goldens for the CLI-boundary error family.
    ///
    /// These exercise the same `ToDiagnostics` path the `sce-codegen`
    /// binary uses — each entry constructs a real [`CliError`] and
    /// renders it via `to_single_diagnostic`, so Display, payload, and
    /// wire shape are pinned from the library-owned type.
    /// Participates in all three invariant tests
    /// ([`diagnostic_goldens_are_byte_stable`],
    /// [`human_mode_matches_json_message`],
    /// [`every_code_has_a_golden`]).
    fn cli_golden_entries() -> Vec<(&'static str, crate::cli_error::CliError, &'static str)> {
        use crate::cli_error::CliError;
        vec![
            (
                "cli/unknown-language",
                CliError::UnknownLanguage { lang: "ruby".into() },
                r#"{"v":1,"id":"fnv1a:0b7cd966f5ada566","code":"cli/unknown-language","stage":"cli","message":"Unknown language: ruby. Use rust, cpp, kotlin, or go.","actual":"ruby","fix":{"kind":"replace_one_of","candidates":["rust","cpp","kotlin","go"]}}"#,
            ),
            (
                "cli/unsupported-language",
                CliError::UnsupportedLanguage {
                    lang: "Python statechart".into(),
                },
                r#"{"v":1,"id":"fnv1a:d4b360b4aec82aca","code":"cli/unsupported-language","stage":"cli","message":"Python statechart codegen is not yet supported","actual":"Python statechart"}"#,
            ),
            (
                "cli/read-input",
                CliError::ReadInput {
                    path: "chart.scxml".into(),
                    source: std::io::Error::from(std::io::ErrorKind::NotFound),
                },
                r#"{"v":1,"id":"fnv1a:5f6b3cd5af06c049","code":"cli/read-input","stage":"cli","message":"Cannot read chart.scxml: entity not found","actual":"chart.scxml"}"#,
            ),
            (
                "cli/write-output",
                CliError::WriteOutput {
                    path: "out/chart.rs".into(),
                    source: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
                },
                r#"{"v":1,"id":"fnv1a:b097092587b48b16","code":"cli/write-output","stage":"cli","message":"Cannot write out/chart.rs: permission denied","actual":"out/chart.rs"}"#,
            ),
            (
                "cli/create-output-dir",
                CliError::CreateOutputDir {
                    path: "out/".into(),
                    source: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
                },
                r#"{"v":1,"id":"fnv1a:d5090869606d1afc","code":"cli/create-output-dir","stage":"cli","message":"Cannot create output directory out/: permission denied","actual":"out/"}"#,
            ),
            (
                "cli/scxml-generate",
                CliError::ScxmlGenerate {
                    stage: "validation",
                    detail: "state id 'armed' duplicated".into(),
                },
                r#"{"v":1,"id":"fnv1a:add14c90ed7da6d2","code":"cli/scxml-generate","stage":"cli","message":"validation: state id 'armed' duplicated"}"#,
            ),
            (
                "cli/missing-metadata-field",
                CliError::MissingMetadataField {
                    path: "metadata.txt".into(),
                },
                r#"{"v":1,"id":"fnv1a:0492d0b2283e41f1","code":"cli/missing-metadata-field","stage":"cli","message":"No description field found in metadata.txt","actual":"metadata.txt"}"#,
            ),
            (
                "cli/not-a-directory",
                CliError::NotADirectory {
                    path: "build/out.rs".into(),
                },
                r#"{"v":1,"id":"fnv1a:11b8224ce9934d18","code":"cli/not-a-directory","stage":"cli","message":"Not a directory: build/out.rs","actual":"build/out.rs"}"#,
            ),
            (
                "cli/invalid-format-option",
                CliError::InvalidFormatOption {
                    value: "yaml".into(),
                    expected: "human|json".into(),
                },
                r#"{"v":1,"id":"fnv1a:0cdfa05d11fc5b3d","code":"cli/invalid-format-option","stage":"cli","message":"unknown --format yaml; expected human|json","actual":"yaml","fix":{"kind":"replace_one_of","candidates":["human","json"]}}"#,
            ),
            (
                "cli/json-serialization",
                CliError::JsonSerialization {
                    detail: "control character in string".into(),
                },
                r#"{"v":1,"id":"fnv1a:60be469b9af31308","code":"cli/json-serialization","stage":"cli","message":"JSON serialization failed: control character in string"}"#,
            ),
            (
                "cli/project-root-not-found",
                CliError::ProjectRootNotFound,
                r#"{"v":1,"id":"fnv1a:8ef6a814f15343f7","code":"cli/project-root-not-found","stage":"cli","message":"Cannot find project root. Run from project directory or set --registry/--resources."}"#,
            ),
            (
                "cli/format-style-not-found",
                CliError::FormatStyleNotFound {
                    path: ".rustfmt.toml".into(),
                },
                r#"{"v":1,"id":"fnv1a:84e445e58bbb1bde","code":"cli/format-style-not-found","stage":"cli","message":"--format-style file not found: .rustfmt.toml","actual":".rustfmt.toml"}"#,
            ),
            (
                "cli/no-scxml-tag",
                CliError::NoScxmlTag {
                    path: "notes.txt".into(),
                },
                r#"{"v":1,"id":"fnv1a:94f3f36322d6b380","code":"cli/no-scxml-tag","stage":"cli","message":"No <scxml> tag found in notes.txt","actual":"notes.txt"}"#,
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
        for (label, err, golden) in xsd_golden_entries() {
            // XsdErrors is multi-record by design; sample a single-
            // diagnostic instance so `single()` applies. Structure is
            // validated the same way as forge/mesh: serialize and
            // compare to the pinned JSON.
            let actual = serde_json::to_string(&single(&err)).unwrap();
            if actual != golden {
                mismatches.push(format!(
                    "\n[{label}]\nexpected: {golden}\n  actual: {actual}"
                ));
            }
        }
        for (label, err, golden) in cli_golden_entries() {
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

    /// Defense-in-depth guardrail: for every first-party error
    /// (`ForgeError`, `MeshError`), the human-mode `Display` output
    /// equals the JSON `message` field byte-for-byte.
    ///
    /// **Structurally enforced by the `ToDiagnostics` trait.** The
    /// default `to_single_diagnostic` body sets `message:
    /// self.to_string()` at the single call site in the codebase, so a
    /// conforming implementer cannot diverge the two surfaces without
    /// overriding `to_diagnostics` or `to_single_diagnostic` entirely.
    /// This test therefore exists as a guardrail against two futures:
    ///   1. a new override that silently diverges (e.g. a second
    ///      multi-record emitter that inlines its own `message`);
    ///   2. manual `Diagnostic` construction outside the trait (e.g.
    ///      `Diagnostic::meta_failure`, should its contract change).
    ///
    /// Why this matters: operators read `format!("{}", err)` on stderr
    /// via `ErrorFormat::Human`; upstream agents consume
    /// `Diagnostic.message` via `--error-format=json`. If the two ever
    /// diverge, the same error gets described two different ways —
    /// operator pages the agent, agent's memory references a wording
    /// the operator has never seen. The JSON byte-goldens cover the
    /// JSON surface; this test covers the human surface.
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
        for (label, err, _golden) in cli_golden_entries() {
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
             The `ToDiagnostics` default `to_single_diagnostic` body sets \
             `message: self.to_string()`. A failure here means either a \
             `to_diagnostics`/`to_single_diagnostic` override now builds \
             `message` from something other than `Display`, or a manual \
             `Diagnostic` is being constructed outside the trait. If the \
             divergence is intentional, remove the offending case and \
             document the exception in SCE_ERROR_CONTRACT.md §3.",
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
            | MeshTopologySubscriptionSourceUnbound
            | MeshCodegenUnsupportedLanguage
            | MeshCodegenUnsupportedTransport
            | CliUnknownLanguage
            | CliInvalidFormatOption => FixCarriesCandidates,

            // ── `expected` carries non-repair metadata ────────
            ExpressionParseMismatch | MeshExternalAmbiguousEventGroup => ExpectedIsMetadata,

            // ── Deterministic fix or no fix; expected=None ────
            XmlParse
            | XmlSchemaValidation
            | XmlXIncludeMissingHref
            | XmlXIncludeNotFound
            | XmlXIncludeReadError
            | XmlXIncludeCycle
            | XmlXIncludeTooDeep
            | XmlXIncludeMalformed
            | XmlXIncludeUnsupported
            | XmlTemplateNotFound
            | XmlTemplateReadError
            | XmlTemplateMalformed
            | XmlTemplateMissingAttribute
            | XmlTemplateMissingParam
            | XmlTemplateUnknownParam
            | XmlTemplateCycle
            | XmlTemplateTooDeep
            | ValidationMissingElement
            | ValidationMissingAttribute
            | ValidationDuplicateId
            | ValidationDuplicateContextObject
            | ValidationReservedContextId
            | ValidationEmptyCollection
            | ValidationCountMismatch
            | ValidationIncompatibleAttributes
            | ValidationMissingContext
            | ValidationNumericParse
            | ValidationEmptyValue
            | ValidationSingletonViolation
            | ValidationWrongPipeline
            | ValidationDynamicFeatures
            | ValidationMeshRpcReservedParam
            | ValidationMeshRpcMissingTarget
            | ValidationMeshRpcDuplicateTarget
            | ValidationRemovedAttribute
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
            | GenerateUnsupportedFeature
            | IoFilesystem
            | CliUnsupportedLanguage
            | CliReadInput
            | CliWriteOutput
            | CliCreateOutputDir
            | CliScxmlGenerate
            | CliMissingMetadataField
            | CliNotADirectory
            | CliJsonSerialization
            | CliProjectRootNotFound
            | CliFormatStyleNotFound
            | CliNoScxmlTag
            | MeshDeployRead
            | MeshDeployParse
            | MeshDeployDuplicateMachine
            | MeshDeployInvalidOrderingTimings
            | MeshDeployInvalidLiveliness
            | MeshDeployInvalidServerQueryTimeout
            | MeshDeployInvalidOutboundBuffer
            | MeshDeployDiscoveryNotSupported
            | MeshDeployPoolNotSupportedByTransport
            | MeshDeployPoolMissingInstanceList
            | MeshDeployPoolEmptyInstanceList
            | MeshDeployPoolInvalidPlaceholder
            | MeshDeployServerPoolNotSupported
            | MeshDeployScxmlInvokeTargetConflict
            | MeshDeployPartitionDuplicateName
            | MeshDeployPartitionMultiDevice
            | MeshDeployPartitionUnitDuplicate
            | MeshDeployPartitionMachineNotListed
            | MeshDeployPartitionEmpty
            | MeshDeployPartitionSynthInfixCollision
            | MeshDeployPartitionUncoveredUnit
            | MeshDeployPartitionPartialCoverageRequiresDefault
            | MeshDeployPartitionPoolMachine
            | MeshDeployPartitionTransportBindingUnsupported
            | MeshDeployScxmlInvokeCrossDeviceTransport
            | MeshDeploySomeipScxmlInvokeServiceIdOverflow
            | MeshDeploySomeipScxmlInvokeServiceIdPinOutOfRange
            | MeshDeploySomeipScxmlInvokeServiceIdPinCollision
            | MeshDeployPartitionBarrierTimeoutInvalid
            | MeshPartitionParallelRootUndesignated
            | MeshPartitionParallelRootAmbiguous
            | MeshPartitionParallelRootNotInMachines
            | MeshPartitionParallelRootNonHost
            | MeshPartitionBarrierTimeoutWithoutRoot
            | MeshPartitionWire21CustomTcpUnimplemented
            | MeshDistributabilityR1SharedWrite
            | MeshDistributabilityR2CrossRegionTransition
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
            | MeshTopologyOrderingCannotBeGuaranteed
            | MeshTopologyPoolParamNameMissing
            | MeshTopologyMachineLifetimeSubscriptionUnsupported
            | MeshCodegenTemplateRead
            | MeshCodegenTemplateRender
            | MeshCodegenEventNameCollision
            | MeshCodegenPoolWithRpcClientUnsupported
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
            TopologyError::SubscriptionSourceUnbound {
                machine: "brake".into(),
                source_target: "#ghost".into(),
                available: vec![
                    crate::mesh::target::TargetId::new("#motor").unwrap(),
                    crate::mesh::target::TargetId::new("#chassis").unwrap(),
                ],
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
    /// the slice to the schema's code enum.
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

    /// Compile-time drift guard that binds [`ALL_DIAGNOSTIC_CODES`] to
    /// the `DiagnosticCode` enum. Adding a new variant fails to compile
    /// here — contributors must update both the enum and the slice in
    /// the same change. Do **not** add a `_ => true` wildcard arm: it
    /// defeats the guard.
    ///
    /// The `len()` assertion complements the exhaustive match: a
    /// variant named in the match but omitted from the slice survives
    /// the build but fires this assertion. Keep the count in sync with
    /// the enum definition when intentionally adding or removing codes.
    #[test]
    fn all_diagnostic_codes_is_exhaustive() {
        fn must_be_listed(code: DiagnosticCode) -> bool {
            use DiagnosticCode::*;
            match code {
                XmlParse | XmlSchemaValidation
                | XmlXIncludeMissingHref | XmlXIncludeNotFound | XmlXIncludeReadError
                | XmlXIncludeCycle | XmlXIncludeTooDeep | XmlXIncludeMalformed
                | XmlXIncludeUnsupported
                | XmlTemplateNotFound | XmlTemplateReadError | XmlTemplateMalformed
                | XmlTemplateMissingAttribute
                | XmlTemplateMissingParam | XmlTemplateUnknownParam
                | XmlTemplateCycle | XmlTemplateTooDeep
                | ValidationMissingElement
                | ValidationMissingAttribute | ValidationInvalidAttribute
                | ValidationUnsupportedKind | ValidationDuplicateId
                | ValidationDuplicateContextObject | ValidationReservedContextId
                | ValidationEmptyCollection
                | ValidationCountMismatch | ValidationIncompatibleAttributes
                | ValidationMissingContext | ValidationInvalidReference
                | ValidationInvalidDirection | ValidationNumericParse
                | ValidationEmptyValue | ValidationSingletonViolation
                | ValidationRequireEither | ValidationWrongPipeline
                | ValidationDynamicFeatures | ValidationMeshRpcReservedParam
                | ValidationMeshRpcMissingTarget
                | ValidationMeshRpcDuplicateTarget
                | ValidationRemovedAttribute
                | ExpressionEmpty | ExpressionLex
                | ExpressionUnsupportedConstruct | ExpressionStrictEquality
                | ExpressionParseMismatch | ExpressionUnexpectedToken
                | ExpressionInvalidLvalue | ExpressionTypeCoercion
                | ExpressionGoTernaryUnsupported | ImportFileNotFound
                | ImportKindMismatch | ImportNotForge | ImportReadError
                | ManifestCircularDependency | ManifestIo | GenerateInvalidConfig
                | GenerateTemplateLoad | GenerateTemplateRender
                | GenerateUnsupportedFeature | IoFilesystem
                | CliUnknownLanguage | CliUnsupportedLanguage | CliReadInput
                | CliWriteOutput | CliCreateOutputDir | CliScxmlGenerate
                | CliMissingMetadataField | CliNotADirectory
                | CliInvalidFormatOption | CliJsonSerialization
                | CliProjectRootNotFound | CliFormatStyleNotFound | CliNoScxmlTag
                | MeshDeployRead | MeshDeployParse | MeshDeployUnsupportedVersion
                | MeshDeployDuplicateMachine | MeshDeployInvalidOrderingTimings
                | MeshDeployInvalidLiveliness
                | MeshDeployInvalidServerQueryTimeout
                | MeshDeployInvalidOutboundBuffer
                | MeshDeployDiscoveryNotSupported
                | MeshDeployPoolNotSupportedByTransport
                | MeshDeployPoolMissingInstanceList
                | MeshDeployPoolEmptyInstanceList
                | MeshDeployPoolInvalidPlaceholder
                | MeshDeployServerPoolNotSupported
                | MeshDeployScxmlInvokeTargetConflict
                | MeshDeployPartitionDuplicateName
                | MeshDeployPartitionMultiDevice
                | MeshDeployPartitionUnitDuplicate
                | MeshDeployPartitionMachineNotListed
                | MeshDeployPartitionEmpty
                | MeshDeployPartitionSynthInfixCollision
                | MeshDeployPartitionUncoveredUnit
                | MeshDeployPartitionPartialCoverageRequiresDefault
                | MeshDeployPartitionPoolMachine
                | MeshDeployPartitionTransportBindingUnsupported
                | MeshDeployScxmlInvokeCrossDeviceTransport
                | MeshDeploySomeipScxmlInvokeServiceIdOverflow
                | MeshDeploySomeipScxmlInvokeServiceIdPinOutOfRange
                | MeshDeploySomeipScxmlInvokeServiceIdPinCollision
                | MeshDeployPartitionBarrierTimeoutInvalid
                | MeshPartitionParallelRootUndesignated
                | MeshPartitionParallelRootAmbiguous
                | MeshPartitionParallelRootNotInMachines
                | MeshPartitionParallelRootNonHost
                | MeshPartitionBarrierTimeoutWithoutRoot
                | MeshPartitionWire21CustomTcpUnimplemented
                | MeshDistributabilityR1SharedWrite
                | MeshDistributabilityR2CrossRegionTransition
                | MeshExternalRead | MeshExternalParse
                | MeshExternalUnresolvedNames | MeshExternalAmbiguousEventGroup
                | MeshExternalEmptyEventGroup
                | MeshExternalNamedReferenceWithoutConfig
                | MeshExternalReservedSomeipIdKeys
                | MeshExternalSomeipFieldOnNonSomeipTransport
                | MeshExternalConflictingEventSchema
                | MeshExternalConflictingEventFieldKinds
                | MeshExternalEmptyEventEntry | MeshTopologyUnresolvedTargets
                | MeshTopologyMachineNotFound | MeshTopologyReceiverNotDeclared
                | MeshTopologyAbsoluteSourcePath | MeshTopologyReceiverSourceRead
                | MeshTopologyReceiverSourceParse | MeshTopologyUncoveredEvents
                | MeshTopologyPatternCapabilityViolation
                | MeshTopologyMissingBindingField | MeshTopologyInvalidBindingField
                | MeshTopologyEventBindingUnused | MeshTopologyOrderingCannotBeGuaranteed
                | MeshTopologyPoolParamNameMissing
                | MeshTopologySubscriptionSourceUnbound
                | MeshTopologyMachineLifetimeSubscriptionUnsupported
                | MeshCodegenUnsupportedLanguage
                | MeshCodegenUnsupportedTransport | MeshCodegenTemplateRead
                | MeshCodegenTemplateRender | MeshCodegenEventNameCollision
                | MeshCodegenPoolWithRpcClientUnsupported
                | MeshIo => true,
            }
        }
        for &code in ALL_DIAGNOSTIC_CODES {
            assert!(
                must_be_listed(code),
                "{code:?} is in ALL_DIAGNOSTIC_CODES but missing from \
                 the exhaustive match — paste-typo?",
            );
        }
        assert_eq!(
            ALL_DIAGNOSTIC_CODES.len(),
            144,
            "ALL_DIAGNOSTIC_CODES has duplicates or missing entries — \
             expected 144 distinct variants to match the DiagnosticCode \
             enum.",
        );
    }

    /// Drift guard between [`ALL_DIAGNOSTIC_CODES`] and the
    /// positive-form acceptance spec at `docs/SCE_ACCEPTED_SUBSET.md`.
    ///
    /// The acceptance doc is the build-time counterpart to
    /// `SCE_ERROR_CONTRACT.md`: the contract enumerates rejection
    /// signals, the acceptance doc enumerates the accepted subset.
    /// The doc's appendix partitions every `DiagnosticCode` into
    /// "Acceptance boundary" (author-preventable) vs "Diagnostic-only"
    /// (I/O / infrastructure).
    ///
    /// The check matches each slash-path against the table-row form
    /// `| `code` |`, not a loose substring. This catches three drift
    /// modes the loose match would miss:
    ///
    /// 1. Code mentioned only in prose, never in an appendix table.
    /// 2. Code placed in both tables (duplicate partition membership).
    /// 3. Appendix row deleted while the code is still referenced in
    ///    prose.
    ///
    /// The `include_str!` binds the check to the file at compile time
    /// so a stale checkout cannot pass CI with a missing file. When
    /// this fires, place the code in exactly one of the two appendix
    /// tables — "Acceptance boundary" if the author can prevent it by
    /// writing better SCXML, "Diagnostic-only" if it is an I/O or
    /// infrastructure failure.
    #[test]
    fn acceptance_doc_covers_every_code() {
        let doc = include_str!("../../../docs/SCE_ACCEPTED_SUBSET.md");
        let mut not_in_appendix: Vec<&'static str> = Vec::new();
        let mut duplicated: Vec<(&'static str, usize)> = Vec::new();
        for &code in ALL_DIAGNOSTIC_CODES {
            let row_anchor = format!("| `{}` |", code.as_str());
            let hits = doc.matches(&row_anchor).count();
            match hits {
                0 => not_in_appendix.push(code.as_str()),
                1 => {}
                n => duplicated.push((code.as_str(), n)),
            }
        }
        assert!(
            not_in_appendix.is_empty(),
            "DiagnosticCode entries not present as an appendix row in \
             docs/SCE_ACCEPTED_SUBSET.md (expected a line beginning \
             `| `<code>` |`):\n{not_in_appendix:#?}\n\n\
             Place each missing code in exactly one appendix table — \
             'Acceptance boundary' if the author can prevent it by \
             writing better SCXML, 'Diagnostic-only' if it is an I/O \
             or infrastructure failure.",
        );
        assert!(
            duplicated.is_empty(),
            "DiagnosticCode entries appear in more than one appendix \
             row of docs/SCE_ACCEPTED_SUBSET.md (code, occurrences):\n\
             {duplicated:#?}\n\n\
             Each code must sit in exactly one partition.",
        );
    }

    /// Drift guard between [`SCHEMA_STATUS`] (the Rust source of
    /// truth) and the `x-sce-schema-status` field in
    /// `schemas/sce-diagnostic.v1.schema.json` (the downstream-visible
    /// declaration). Both must agree; otherwise external consumers
    /// reading the schema file would see a stability claim that
    /// diverges from the crate.
    ///
    /// The check also enforces the closed value set (`pre-release`
    /// or `stable`) so typos do not slip past review. See
    /// `SCE_ERROR_CONTRACT.md` §8.1 for the transition criterion.
    #[test]
    fn schema_file_declares_status() {
        let schema_bytes =
            include_str!("../../../schemas/sce-diagnostic.v1.schema.json");
        let parsed: serde_json::Value = serde_json::from_str(schema_bytes)
            .expect("schema file must be valid JSON");
        let declared = parsed
            .get("x-sce-schema-status")
            .and_then(|v| v.as_str())
            .expect(
                "schema must declare x-sce-schema-status at top level — \
                 see SCE_ERROR_CONTRACT.md §8.1",
            );
        assert!(
            matches!(declared, "pre-release" | "stable"),
            "x-sce-schema-status must be 'pre-release' or 'stable'; \
             got {declared:?}",
        );
        assert_eq!(
            declared, SCHEMA_STATUS,
            "schema file's x-sce-schema-status drifted from \
             SCHEMA_STATUS const — update one to match the other and \
             verify SCE_ERROR_CONTRACT.md §8.1",
        );
    }

    /// Coverage drift guard: every `DiagnosticCode` variant must have
    /// at least one entry in [`forge_golden_entries`] or
    /// [`mesh_golden_entries`]. Without this, the byte-stability and
    /// human↔JSON parity tests would sample an arbitrary subset of
    /// variants and miss drift on the uncovered ones.
    ///
    /// When this test fails, read the `missing` list and add one
    /// representative error instance per variant to the appropriate
    /// golden table. Capture the JSON output with `serde_json::to_string`
    /// rather than hand-typing the hash — the `fnv1a:` prefix plus the
    /// content hash is mechanical.
    #[test]
    fn every_code_has_a_golden() {
        use std::collections::HashSet;
        let mut covered: HashSet<&'static str> = HashSet::new();
        for (_label, err, _golden) in forge_golden_entries() {
            covered.insert(single(&err).code.as_str());
        }
        for (_label, err, _golden) in mesh_golden_entries() {
            covered.insert(single(&err).code.as_str());
        }
        for (_label, err, _golden) in xsd_golden_entries() {
            covered.insert(single(&err).code.as_str());
        }
        for (_label, err, _golden) in cli_golden_entries() {
            covered.insert(single(&err).code.as_str());
        }
        let missing: Vec<&'static str> = ALL_DIAGNOSTIC_CODES
            .iter()
            .map(|c| c.as_str())
            .filter(|s| !covered.contains(s))
            .collect();
        assert!(
            missing.is_empty(),
            "DiagnosticCode variants without goldens ({}):\n{}\n\n\
             Add representative cases to forge_golden_entries, \
             mesh_golden_entries, xsd_golden_entries, or \
             cli_golden_entries in sce-build/src/forge/diagnostic.rs.",
            missing.len(),
            missing.join("\n"),
        );
        assert_eq!(
            covered.len(),
            ALL_DIAGNOSTIC_CODES.len(),
            "golden coverage out of sync with enum: covered={}, enum={}",
            covered.len(),
            ALL_DIAGNOSTIC_CODES.len(),
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
