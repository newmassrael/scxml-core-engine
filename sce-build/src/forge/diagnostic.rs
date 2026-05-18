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
    // ── Top-level parser-entry errors (RFC §W4 α-strict). The two
    //    codes here have full Rust producers in
    //    `crate::parser::SCXMLParser` (parse_file ErrorKind::NotFound
    //    branch + parse_impl root-tag check). Mirrored in C++ by
    //    `SCE::parsing::ParseFileNotFound` and `ParseWrongRootElement`
    //    (`sce/include/parsing/ParseError.h`). The other 3 C++
    //    parser-entry leaves (ParseXmlFailed, ParseException,
    //    ParseNoRootElement) reuse `xml/parse` because the Rust error
    //    model has no distinct producer (Result-based, no exceptions,
    //    roxmltree always-has-root). ──────────────────────────────
    #[serde(rename = "xml/file-not-found")]
    XmlFileNotFound,
    #[serde(rename = "xml/wrong-root-element")]
    XmlWrongRootElement,
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
    // ── Forge bytes-typed slot capacity contract (RFC
    //    `claudedocs/rfc-forge-bytes-bounded.md` §3 B1+B4). The
    //    inconsistency is between two SCXML-declared caps (e.g.
    //    `sce:response-max-size` on a `<send>` exceeds
    //    `sce:max-size` on the destination `<data>` slot), caught
    //    at parse time before any backend codegen. ────────────
    #[serde(rename = "validation/bytes-max-size-violation")]
    ValidationBytesMaxSizeViolation,

    // ── Algorithm kind (watching-zenoh RFC §5.A, Phase A3).
    //    Parser-stage sema for the new pure-function kind. Three
    //    of the six RFC §5.A diagnostics land in A3-δ; the rest
    //    (`return-type-mismatch`, `while-unbounded`,
    //    `call-cycle`) need typed expression flow / deploy-yaml
    //    MCU detection / cross-file import resolution and are
    //    deferred to A4+. ────────────────────────────────────
    #[serde(rename = "algorithm/local-shadows-param")]
    AlgorithmLocalShadowsParam,
    #[serde(rename = "algorithm/lvalue-unsupported")]
    AlgorithmLvalueUnsupported,
    #[serde(rename = "algorithm/return-missing")]
    AlgorithmReturnMissing,

    // ── Algorithm-over-BC dispatch (RFC §5.A line 311 + §5.L lines
    //    2611-2618 + 2642-2647, C7-lowering 2026-05-13). Six codes wire
    //    the `<sce:foreach in="<bc>">` + `<sce:call
    //    target="alias.method">` lowering surface that lets an
    //    algorithm body iterate a bounded-collection import and
    //    dispatch into its read-only method set. Two ride
    //    `FixCarriesCandidates` (alias / method enumeration); four
    //    ride `NeutralOrDeterministic`. ───────────────────────────
    #[serde(rename = "algorithm/foreach-source-not-iterable")]
    AlgorithmForeachSourceNotIterable,
    #[serde(rename = "algorithm/call-target-unknown")]
    AlgorithmCallTargetUnknown,
    #[serde(rename = "algorithm/call-target-method-unknown")]
    AlgorithmCallTargetMethodUnknown,
    #[serde(rename = "algorithm/bc-mutation-forbidden")]
    AlgorithmBcMutationForbidden,
    #[serde(rename = "algorithm/foreach-source-bc-with-bytes-item-type")]
    AlgorithmForeachSourceBcWithBytesItemType,
    #[serde(rename = "algorithm/call-arg-count-mismatch")]
    AlgorithmCallArgCountMismatch,

    // ── SCXML semantic-validation (RFC §W5). Three of the four
    //    SCXML semantic failures fold into existing `validation/*`
    //    codes per the W4 D4 fold precedent — concept identity:
    //    "name does not resolve to declared symbol" is the same
    //    failure shape regardless of which document type produces
    //    it. Only this code is W3C-SCXML-specific (top-level
    //    `<script>` rejection per §5.8 has no forge analog). ──
    #[serde(rename = "scxml/top-level-script-unloaded")]
    ScxmlTopLevelScriptUnloaded,
    // ── watching-zenoh RFC §5.E B7-η' SCXML on-sample family ──
    // Author-facing rules for `<sce:on-sample>` SCE extension. Atomic A
    // ships the structural diagnostics (placement, uniqueness,
    // event-name-conflict); cross-ref diagnostics (link-not-declared,
    // link-wrong-kind) are gated on Atomic B's SceCrossDocRegistry.
    #[serde(rename = "scxml/on-sample-invalid-parent")]
    ScxmlOnSampleInvalidParent,
    #[serde(rename = "scxml/on-sample-link-duplicate-in-state")]
    ScxmlOnSampleLinkDuplicateInState,
    #[serde(rename = "scxml/on-sample-event-name-conflict")]
    ScxmlOnSampleEventNameConflict,
    // ── watching-zenoh RFC §5.E B7-η' Atomic B cross-ref family ──
    // Cross-reference resolution surface for `<sce:on-sample link="X">`
    // against the build's `SceCrossDocRegistry`. Atomic A landed the
    // structural codes above; Atomic B opens the cross-ref pair.
    // `link-wrong-kind` is wired forward-compat — today
    // `ScxmlDocKind` has the `Link` variant only, so the validator's
    // match never reaches the `Some(non-Link)` arm in production.
    #[serde(rename = "scxml/on-sample-link-not-declared")]
    ScxmlOnSampleLinkNotDeclared,
    #[serde(rename = "scxml/on-sample-link-wrong-kind")]
    ScxmlOnSampleLinkWrongKind,

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

    // ── Codegen matrix invariants (watching-zenoh RFC §5.J.4 / §5.J.5).
    //    Shell-only at PR-0 (Phase A1): variants present so downstream
    //    consumers can pin the wire IDs; producer + matrix walker land
    //    with the algorithm kind in Phase A3. Stage = Generate
    //    (codegen-time errors share the existing repair-routing key).
    //    See `docs/rfc-sce-protocol-synthesis.md` §5.J.4 commitment. ──
    #[serde(rename = "codegen/mcu-class-kind-on-non-mcu-language")]
    CodegenMcuClassKindOnNonMcuLanguage,
    #[serde(rename = "codegen/generic-kind-backend-emit-missing")]
    CodegenGenericKindBackendEmitMissing,

    // ── §5.J.2 Rust no_std variant rejection (C3 Atomic B-β).
    //    Producer: `cmd_generate` walks the parsed SCXML model when
    //    `--no-std` is passed to `sce-codegen generate -l rust` and
    //    rejects documents that depend on std-coupled runtime
    //    features. Watching-zenoh RFC §5.J.2 line 1989 prescribes
    //    `zero alloc dependency`; the `sce-rust-runtime` feature
    //    matrix encodes the same: `no_std` is incompatible with
    //    `http-send`, `script-engine-lua`, and `script-engine-quickjs`.
    //    Author repair is to drop `--no-std`, or remove the
    //    incompatible SCXML construct. Stage = Generate (codegen-time
    //    rejection shares the existing repair-routing key). ──
    #[serde(rename = "codegen/no-std-script-not-supported")]
    CodegenNoStdScriptNotSupported,
    #[serde(rename = "codegen/no-std-http-not-supported")]
    CodegenNoStdHttpNotSupported,
    // C3 Atomic B-γ2c: <data src="..."> requires PathBuf + std::fs::read_to_string,
    // both alloc/OS-coupled. RFC §5.J.2 lines 1989-1994 forbid alloc paths in
    // generated no_std code.
    #[serde(rename = "codegen/no-std-fs-load-not-supported")]
    CodegenNoStdFsLoadNotSupported,
    // C3 Atomic B-γ2c: <invoke> binds child-session lifecycle via
    // Arc<Mutex<Vec<...>>> + HashMap, all alloc-coupled. Same RFC §5.J.2 rule.
    #[serde(rename = "codegen/no-std-invoke-not-supported")]
    CodegenNoStdInvokeNotSupported,

    // ── §5.F build-time const-fold (watching-zenoh RFC §5.F, Phase A4-γ).
    //    The host interpreter (`forge::const_fold`) emits these
    //    codegen-time errors when a `<sce:fold>` body — or a scalar
    //    `<sce:const init=...>` — fails the foldable substrate, blows
    //    the iteration budget, or produces a value the declared
    //    element / scalar type cannot hold. Stage = Generate (the
    //    interpreter runs inside `lower_algorithm_consts` during
    //    template rendering). β shipped these as
    //    `generate/unsupported-feature` slug payloads; γ promotes
    //    them to first-class wire codes. ──
    #[serde(rename = "algorithm/const-not-foldable")]
    AlgorithmConstNotFoldable,
    #[serde(rename = "algorithm/const-fold-budget-exceeded")]
    AlgorithmConstFoldBudgetExceeded,
    #[serde(rename = "algorithm/const-yield-type-mismatch")]
    AlgorithmConstYieldTypeMismatch,

    // ── §5.B variant primitive (watching-zenoh RFC §5.B, B1-β).
    //    Build-time check on `<sce:variant>` codec suffix: the
    //    enumerated `<sce:arm value=...>` set must cover the tag
    //    field's value domain when no `<sce:default>` is declared,
    //    otherwise some incoming tag value would have no matching
    //    branch at runtime. v1 considers uint8 (256) and uint16
    //    (65536) practically enumerable; uint32 / uint64 always
    //    require a default. Stage = Validation. ──
    #[serde(rename = "codec/variant-arm-unreachable")]
    CodecVariantArmUnreachable,

    // ── RFC variant-default-uniformity Atomic α (claudedocs/
    //    rfc-variant-default-uniformity.md). Parse-time check on
    //    `<sce:variant>` children: at most one `<sce:arm>` may
    //    declare `default="true"`. The marker steers the outer
    //    codec's `Default::default()` to a single deliberately-
    //    chosen arm; two declarations are ambiguous so the parser
    //    refuses to silently pick one. Distinct from the catch-all
    //    `<sce:default>` element (forward-compat for unknown tag
    //    values) — both may coexist on the same variant. Stage =
    //    Validation. ──
    #[serde(rename = "codec/variant-duplicate-default-arm")]
    CodecVariantDuplicateDefaultArm,

    // ── RFC variant-default-uniformity Atomic γ-1: cross-doc check
    //    that the outer `<sce:arm default="true" value="X"/>` and
    //    the inner codec's matching peek-byte `<sce:flag value="Y"/>`
    //    declare the same wire constant. Mismatch lands the wrong
    //    arm at decode time. Stage = Validation. ──
    #[serde(rename = "codec/variant-default-arm-mid-mismatch")]
    CodecVariantDefaultArmMidMismatch,

    // ── RFC variant-default-uniformity Atomic γ-1: cross-doc check
    //    that the inner codec selected by a default-marked outer
    //    arm declares a wire-MID constant via `<sce:flag value="..."/>`.
    //    Absence means the inner's Default zero-fills the dispatch
    //    byte and round-trip breaks. Stage = Validation. ──
    #[serde(rename = "codec/variant-arm-inner-mid-undeclared")]
    CodecVariantArmInnerMidUndeclared,

    // ── §5.B present-if primitive (watching-zenoh RFC §5.B, B1-δ).
    //    Build-time check on `<sce:field sce:present-if="X.Y"/>`: the
    //    referenced field `X` must be declared earlier in the same
    //    codec so the streaming decoder has already consumed it by
    //    the time this field's predicate is evaluated. A forward
    //    reference (predicate target declared after the consumer) is
    //    rejected here; an unknown predicate target reuses the
    //    generic `validation/invalid-attribute` code (typos in the
    //    flag-name half land there too). Stage = Validation. ──
    #[serde(rename = "codec/present-if-refs-later-field")]
    CodecPresentIfRefsLaterField,

    // ── §5.B repeat primitive (watching-zenoh RFC §5.B, B2). Build-
    //    time check on `<sce:repeat sce:count="X"/>`: the referenced
    //    count field `X` must be declared earlier in the same codec
    //    so the streaming decoder has already consumed it by the time
    //    the repeat loop reads N. A forward reference (count field
    //    declared after the repeat) is rejected here; non-integer
    //    count target and shape mismatches reuse the generic
    //    `validation/invalid-attribute` code. Stage = Validation. ──
    #[serde(rename = "codec/repeat-count-refs-later-field")]
    CodecRepeatCountRefsLaterField,

    // ── §5.B test-vector primitive (watching-zenoh RFC §5.B B2) ──
    //    `<sce:test-vector hex value/>` is supported only on
    //    `sce:kind="algorithm"` in v1. Multi-field codec test vectors
    //    defer to B5 alongside the Zenoh msg-set authoring; until
    //    then the existing `numerical_reference.json` oracle harness
    //    is the route for codec round-trips. Stage = Validation. ──
    #[serde(rename = "algorithm/test-vector-unsupported-kind")]
    AlgorithmTestVectorUnsupportedKind,

    // ── §5.B B3 TLV chain primitive (watching-zenoh RFC §5.B) ──
    //    `<sce:tlv-chain>` is MCU-class and the runtime decoder needs a
    //    build-time bound to size its working set; missing `max-depth`
    //    is rejected here. RFC line 488 "max-depth MUST be specified
    //    for MCU targets" + line 533 "Iterative parse only; max-depth
    //    lowers to a max-iter on the chain traversal loop". Repair is
    //    structural — add `max-depth="N"` (N > 0). Stage = Validation. ──
    #[serde(rename = "codec/tlv-chain-depth-unspecified")]
    CodecTlvChainDepthUnspecified,

    // ── §5.B B3 DMA alignment primitive (watching-zenoh RFC §5.B) ──
    //    `sce:dma-burst-align="N"` requires the field's `sce:byte` be
    //    divisible by N AND every preceding field be Fixed bit-size
    //    (RFC line 558-583 "fixed-offset positions only — no VLE-
    //    following alignment"). Variable-length predecessor or
    //    misaligned offset → reject here so the wire-layout invariant
    //    is enforced statically. Repair is structural — reorder
    //    fields, lower the alignment requirement, or change the
    //    variable predecessor. Stage = Validation. ──
    #[serde(rename = "codec/dma-alignment-unsatisfiable")]
    CodecDmaAlignmentUnsatisfiable,

    // ── §5.B B5-γ parent-flags dependency (watching-zenoh RFC §5.B) ──
    //    A body codec with `<sce:requires-parent-flags carrier="X">`
    //    declares its body fields read flags from a parent codec's
    //    flags carrier (Zenoh upstream pattern: `_z_init_decode(..,
    //    uint8_t header)` gates `sn_res + req_id_res + batch_size`
    //    on the parent header's S-flag). The cross-codec validator
    //    (variant arm wire-up) confirms the parent codec has
    //    `<sce:flags id="<carrier>">` of uint8 with each declared
    //    flag name + bit position matching exactly. Mismatch
    //    surfaces here on three axes: (a) parent codec lacks the
    //    named carrier; (b) carrier is not flags-bearing or is not
    //    uint8; (c) declared flag name+bit doesn't match the parent
    //    layout. Repair is structural — fix the body's declared
    //    parent-flag layout to match the parent's carrier shape,
    //    or wire the body codec to a different parent. Stage =
    //    Validation. ──
    #[serde(rename = "codec/parent-flag-mismatch")]
    CodecParentFlagMismatch,

    // ── §5.C link kind (watching-zenoh RFC §5.C, B6-α/γ). MCU-class
    //    byte-stream link endpoint. Three of seven §5.C diagnostic
    //    codes ship at γ landing — the parse-time pair grew the count
    //    from 172 to 174. The remaining codes
    //    (`link/class-unsupported-on-target`,
    //    `link/link-class-incompatible-with-os`,
    //    `link/listener-link-not-paired-with-established-sibling`)
    //    defer: the OS-axis pair waits on the forge × deploy.yaml
    //    integration atomic (η; `platform.os` lives per-machine in
    //    deploy.yaml per RFC §5.C lines 702-704), the listener self-
    //    check waits on B6-δ, and `link/pool-slot-smaller-than-framer-
    //    max` defers to B7 with the buffer-pool kind. Stage = Validation. ──
    /// `<sce:framer ref="..."/>` is required on `sce:kind="link"`;
    /// absence is rejected at parse time so the codegen never
    /// reaches the missing-codec branch. Repair: add the framer ref.
    #[serde(rename = "link/framer-missing")]
    LinkFramerMissing,

    /// `<sce:link-class>` body text is not in the closed enum
    /// (`udp` / `tcp` / `serial` / `websocket` / `raw_eth` per RFC
    /// §5.C lines 765-771). Promotes the generic
    /// `validation/invalid-attribute` to a dedicated link-kind code so
    /// authors and downstream agents key on the link-class violation
    /// directly. Repair: replace the value with one of the listed
    /// candidates (`fix: ReplaceOneOf`).
    #[serde(rename = "link/link-class-unknown")]
    LinkLinkClassUnknown,

    /// `<sce:backpressure>` element is required on `sce:kind="link"`
    /// declarations — the policy is load-bearing for the runtime
    /// crate's RX queue behavior under load (RFC §5.C body). B6-α
    /// shipped a parser-side `default-to-drop` for the missing case;
    /// γ promotes the absence to a hard error so authors must declare
    /// the policy intentionally. Repair: add a `<sce:backpressure>`
    /// child whose body is `drop`, `block`, or `signal-event`.
    #[serde(rename = "link/backpressure-undeclared")]
    LinkBackpressureUndeclared,

    /// Declared `<sce:link-class>` cannot run on the deploy-resolved
    /// `platform.os` per RFC §5.C lines 765-771 / 838. The matrix
    /// (single source of truth: [`forge::model::LinkClass::admits_os`])
    /// admits `udp`/`tcp` on `bare_metal|linux|qnx`; `serial`/
    /// `websocket`/`raw_eth` on `bare_metal` only. Validate-time —
    /// the diagnostic only reaches the consumer when the new
    /// `compile_forge_with_deploy` entry is invoked with a deploy
    /// context AND the target machine declares `platform.os`. Repair:
    /// `Fix::ReplaceOneOf` carries the OS axis (the list of OSes the
    /// class admits) so the author can change either the class or
    /// the deployment target.
    #[serde(rename = "link/class-unsupported-on-target")]
    LinkClassUnsupportedOnTarget,

    /// `<sce:rx-pool>` / `<sce:tx-pool>` reference binds a buffer-pool
    /// whose `<sce:slot-size>` is smaller than the framer codec's
    /// recursive worst-case encoded byte count. Cross-resolution —
    /// fires from `compile_forge_with_imports` after enrichment
    /// populates both `ImportContext::codec_max_bytes` (framer side)
    /// and `ImportContext::buffer_pool_slot_size` (pool side). Skips
    /// silently when either axis fails to enrich (partial topology).
    /// No candidate set — repair is to raise `<sce:slot-size>` on the
    /// bound pool or shrink the codec body, both author choices, so
    /// `Fix::None`. RFC §5.C lines 793-794 (rx-pool / tx-pool inherit
    /// the §5.E pool model on both sides of the byte-stream link).
    #[serde(rename = "link/pool-slot-smaller-than-framer-max")]
    LinkPoolSlotSmallerThanFramerMax,

    /// RFC §5.E B7-α buffer-pool placement validation: declared
    /// `<sce:section>` body is not in deploy.yaml `machines.<m>.memory.
    /// sram_regions`. Validate-time — fires only via
    /// [`compile_forge_with_deploy`] when both `deploy` and
    /// `target_machine` resolve and the machine has a `memory` block;
    /// missing pieces skip silently per Q-η5 (a) precedent. Repair:
    /// `Fix::ReplaceOneOf` carries the section-name axis (the list of
    /// regions the resolved machine declares) so the author can rename
    /// the pool's `<sce:section>` body or extend the deploy.yaml memory
    /// map. RFC §5.E lines 1000-1023 + 1537 spec anchor.
    #[serde(rename = "mem/pool-section-conflict")]
    MemPoolSectionConflict,

    /// RFC §5.E B7-β buffer-pool size validation: storage footprint
    /// (`slot_count × slot_size`) does not fit inside the resolved
    /// region's `size` field. Validate-time — fires only via
    /// [`compile_forge_with_deploy`] after `mem/pool-section-conflict`
    /// passes (the section must resolve before its size matters); same
    /// Q-η5 (a) silent-skip when deploy.yaml is unavailable. No
    /// candidate set — the repair is to raise the region size in
    /// deploy.yaml or shrink `slot_count` / `slot_size`, both of which
    /// are author choices. RFC §5.E lines 1031-1086 spec anchor.
    #[serde(rename = "mem/pool-too-large")]
    MemPoolTooLarge,

    /// RFC §5.E B7-β codegen self-check: the rendered linker fragment
    /// is missing the explicit `. = ALIGN(<n>);` inter-pool sentinel.
    /// Codegen-invariant violation, not an authoring mistake — fires
    /// only when the buffer-pool linker fragment template itself drops
    /// the sentinel. The artifact makes the inter-pool boundary
    /// diff-visible and protects the post-pool boundary from
    /// master-script INCLUDE re-ordering. RFC §5.E lines 1059-1064.
    #[serde(rename = "mem/inter-pool-padding-not-emitted")]
    MemInterPoolPaddingNotEmitted,

    /// RFC §5.E C5 cache-maintenance validation: pool `<sce:alignment>`
    /// is smaller than the resolved target's `platform.dcache_line_size`
    /// while `cache-policy: maintain` is in effect. Validate-time —
    /// fires only via [`compile_forge_with_deploy`] after section
    /// validation passes (Q-η5 (a) silent-skip when deploy.yaml is
    /// unavailable). Partial-line cache_invalidate_by_addr corrupts
    /// adjacent slot data on the start side. RFC §5.E line 1544 +
    /// §5.I lines 1742-1744 spec anchor.
    #[serde(rename = "mem/cache-line-alignment")]
    MemCacheLineAlignment,

    /// RFC §5.E C5 cache-maintenance validation: `<sce:slot-size>` is
    /// not a whole-number multiple of the resolved target's
    /// `platform.dcache_line_size` while `cache-policy: maintain` is
    /// in effect. Validate-time — fires only via
    /// [`compile_forge_with_deploy`]. The boundary cache line is
    /// shared with the adjacent slot; cache_invalidate_by_addr after
    /// RX would corrupt it. RFC §5.E line 1545 + §5.I lines 1742-1744
    /// spec anchor.
    #[serde(rename = "mem/slot-size-not-cache-line-multiple")]
    MemSlotSizeNotCacheLineMultiple,

    /// RFC §5.E C5 cache-maintenance validation: pool declares
    /// `cache-policy: maintain` (or `non-cacheable`) while the
    /// resolved target platform has `has_dcache: false`. Cache
    /// maintenance call sites are meaningless on a core without a
    /// data cache. Validate-time — fires only via
    /// [`compile_forge_with_deploy`]. Repair: `Fix::ReplaceOneOf`
    /// candidates = `["none"]`. RFC §5.E line 1543 spec anchor.
    #[serde(rename = "mem/cache-policy-unsupported-on-no-dcache-core")]
    MemCachePolicyUnsupportedOnNoDcacheCore,

    /// RFC §5.E C5 cache-maintenance + §5.I author-guard: an
    /// `<sce:extern>` declaration tries to author one of the cache-
    /// maintenance trio (`sce_dcache_clean_by_addr`,
    /// `sce_dcache_invalidate_by_addr`,
    /// `sce_dcache_clean_invalidate_by_addr`). Per spec lines
    /// 1222-1227, cache maintenance is FSM-driven; codegen auto-
    /// injects the externs and emits the calls on the buffer-pool
    /// lifecycle edges. Author authoring is forbidden because it
    /// silently invites the class of bugs ("the maintenance call
    /// sits in the wrong place") the FSM-driven design prevents.
    /// Parse-time — fires before the §5.I baseline whitelist
    /// validator. RFC §5.E line 1548 + lines 1222-1227 spec anchor.
    #[serde(rename = "pool/cache-maintenance-misplaced")]
    PoolCacheMaintenanceMisplaced,

    /// RFC §5.E C5 cache-maintenance config-completeness diagnostic:
    /// a target machine declares `platform.has_dcache: true` without
    /// setting `platform.has_speculative_prefetch`. Validate-time —
    /// fires only via [`compile_forge_with_deploy`] when at least
    /// one buffer-pool with `cache-policy: maintain` exists in the
    /// build. Codegen cannot decide whether to emit the
    /// `free → dma-armed-rx` pre-arm cache-invalidate edge. Author
    /// resolution: declare `has_speculative_prefetch` per the SoC
    /// datasheet (M7+/A-class = true, M3/M4 = false). RFC §5.E line
    /// 1553 spec anchor.
    #[serde(rename = "pool/speculative-prefetch-flag-missing")]
    PoolSpeculativePrefetchFlagMissing,

    /// RFC §5.E C5 cache-maintenance codegen self-check:
    /// `cache-policy: maintain` + `platform.has_speculative_prefetch:
    /// true` resolved, but the rendered buffer-pool template did not
    /// emit a `sce_dcache_invalidate_by_addr` call inside the
    /// `link_arm_rx` body. Codegen-invariant violation, not an
    /// authoring mistake — fires only when the
    /// `tools/codegen/templates/forge/{rust,c}/buffer_pool` template
    /// itself drops the pre-arm invalidate edge. The diagnostic
    /// guards against template regression that would silently
    /// corrupt RX data on M7+ cores. RFC §5.E line 1552 spec anchor.
    #[serde(rename = "pool/cache-pre-arm-invalidate-missing-on-speculative-core")]
    PoolCachePreArmInvalidateMissingOnSpeculativeCore,

    // ── §5.E pool kind Layer 1 ownership (watching-zenoh RFC §5.E,
    //    B7-ε). Layer 1 typestate-attribute family is exposed to
    //    consumer builds through `sce-c-runtime/include/sce/sample.h`,
    //    pulled in by the generated pool header. The diagnostic catches
    //    a future template edit that drops the `#include` — Layer 1
    //    coverage would silently disappear without it. Stage =
    //    Validation. The remaining `pool/...` codes
    //    (`pool/sample-take-without-stage-pool`,
    //    `pool/sample-callback-signature-non-borrow`,
    //    `pool/clang-tidy-not-configured`,
    //    `pool/ownership-violation`,
    //    `pool/cache-maintenance-misplaced`,
    //    `pool/slot-leak-on-error-path`) defer to later atomics gated on
    //    deploy.yaml `stage_pool` field, SCXML `<sce:on-sample>`
    //    extension, and §5.I `<sce:call>` intrinsic registry — see
    //    `claudedocs/rfc-b7-eta-prime-sample-runtime.md` /
    //    `rfc-sce-call-intrinsic-registry.md`. ──
    /// `<sce/sample.h>` runtime header pull-through (the producer of
    /// the Layer 1 `SCE_CONSUMABLE` / `SCE_CALLABLE_WHEN` /
    /// `SCE_SET_TYPESTATE` / `SCE_PARAM_TYPESTATE` /
    /// `SCE_WARN_UNUSED` family) is missing from the generated C11
    /// pool header. Codegen-invariant violation — fires only when the
    /// `tools/codegen/templates/forge/c/buffer_pool.h.jinja2` template
    /// itself drops the `#include` directive. The runtime header
    /// further self-checks at consumer compile time via `#warning`
    /// when `__clang__ && !__has_attribute(consumable)`; this SCE-side
    /// diagnostic is the build-time peer that catches the include
    /// itself going missing rather than the attribute family being
    /// unavailable on the operator's compiler.
    #[serde(rename = "pool/sample-typestate-attributes-disabled")]
    PoolSampleTypestateAttributesDisabled,

    /// watching-zenoh RFC §5.E B7-η' Atomic A1 application-layer
    /// ownership diagnostic (spec lines 1513-1515): a state declares
    /// `<sce:on-sample link="X">` and link X is registered, but the
    /// link's forge document does not declare a `<sce:stage-pool>`
    /// element. Without a stage pool, generated `Sample::take()` has
    /// no destination for stage-copy and the link's runtime-side
    /// `LinkConfig::stage_copy_hook` falls back to `PanicOnTakeHook`
    /// (sce-link-runtime default) — silently turning callbacks that
    /// call `take()` into runtime panics. Surfaced at codegen time
    /// so authors decide consciously between adding
    /// `<sce:stage-pool>` to the link or restricting the callback to
    /// borrow-only access.
    ///
    /// Schema-locality choice (Atomic A1): the stage pool is a *link*
    /// property co-located with rx_pool/tx_pool on the link kind
    /// document, not a deploy-yaml binding property. The Q-StagePool
    /// `BindingConfig.stage_pool` field (already landed) becomes a
    /// deploy-time override mechanism — orthogonal to this diagnostic.
    #[serde(rename = "pool/sample-take-without-stage-pool")]
    PoolSampleTakeWithoutStagePool,

    /// watching-zenoh RFC §5.E B7-η' Atomic A2 application-layer
    /// ownership diagnostic (spec lines 1516-1519): an
    /// `<sce:on-sample callback="rust:...">` attribute carries an
    /// authoring path that fails the Q-Callback-3 Rust path subset.
    /// SCE-side reachable arms today are path-syntax (unknown
    /// language prefix, leading/trailing `::`, malformed segment,
    /// empty path); future signature inspection (β-extension on top
    /// of α) extends the same code with shape-mismatch arms.
    /// Diagnostic name preserves spec wording verbatim per
    /// `feedback_spec_mirror_parity.md`; the per-instance message's
    /// reason clause names the specific path-syntax mistake.
    #[serde(rename = "pool/sample-callback-signature-non-borrow")]
    PoolSampleCallbackSignatureNonBorrow,

    // ── §5.D Worker kind (watching-zenoh RFC §5.D, C2-α). The
    //    worker primitive is a concurrent execution context driven
    //    by a `<sce:link-rx>` source; it owns an SPSC inbox and
    //    communicates only through that channel + an optional outbox.
    //    Spec line 911 enforces encapsulation: "any non-inbox access
    //    to another worker's state" is a diagnostic. C2-α implements
    //    static recognition layers (sibling `<sce:import kind="worker">`
    //    + body SCXML data-refs); C4-composition hardening for the
    //    `<sce:extern>` non-inbox-symbol path is a tracked follow-up
    //    atomic. Diagnostic names are spec verbatim per
    //    `feedback_spec_mirror_parity.md`. ──
    /// `<sce:body>` or sibling-document scope reaches another
    /// worker's state through a path other than its own inbox + a
    /// recipient's inbox (via `<sce:outbox ref>`). Per Q-C2-7 (a)
    /// lock 2026-05-10: C2-α covers `<sce:import kind="worker">`
    /// rejection (layer 1) and body SCXML cross-namespace data-ref
    /// rejection (layer 2). Layer 3 (`<sce:extern>` non-inbox
    /// symbol use in body) defers to a tracked follow-up atomic
    /// gated on C4 intrinsic-registry composition surface.
    ///
    /// Diagnostic name preserves spec wording verbatim
    /// (`worker/shared-mutable-state`) per
    /// `feedback_spec_mirror_parity.md`; the per-instance message's
    /// reason clause names the specific path that crossed the
    /// encapsulation boundary so authors can locate the offending
    /// declaration without grepping. RFC §5.D line 911 spec anchor.
    #[serde(rename = "worker/shared-mutable-state")]
    WorkerSharedMutableState,

    // ── §5.D + §5.I C2-β cross-resolution + inbox ordering ──────────
    //    Worker docs reference (a) the driving link kind via
    //    `<sce:link-rx ref>` and (b) the recipient state machine's
    //    inbox via `<sce:outbox ref>`. Both refs cross-resolve against
    //    the worker doc's own `<sce:import>` declarations
    //    (η-precedent: `validate_link_pool_framer_resolution` resolves
    //    framer codec aliases the same way). The 2 ordering codes
    //    cover §5.I lines 1752-1758: every SPSC inbox must declare
    //    acquire/release vs relaxed; relaxed-across-cores is a
    //    codegen-invariant guard against unsafe cross-cache-coherency
    //    pairing. Diagnostic names preserve spec wording verbatim per
    //    `feedback_spec_mirror_parity.md`. ──
    /// `<sce:link-rx ref>` names an alias not imported as kind=link.
    /// Repair surface = `Fix::ReplaceOneOf` over the sorted set of
    /// link-kind import aliases on this worker doc. Non-spec
    /// diagnostic per Q-C2-2 (a) lock 2026-05-10; cross-resolution is
    /// SCE's per-doc strengthening of the spec example's elided import
    /// shape.
    #[serde(rename = "worker/link-rx-ref-unknown")]
    WorkerLinkRxRefUnknown,

    /// `<sce:inbox>` declared without an explicit `ordering` attribute.
    /// Spec §5.I lines 1757-1758 verbatim: "no ordering chosen, codegen
    /// defaults to acquire/release with a warning". SCE's error-only
    /// wire realizes the warning as a required-when-worker-exists
    /// error: the author must explicitly pick `acq_rel` or `relaxed`
    /// (the choice changes the atomic ops emitted on head/tail
    /// indices in both Rust + C11 codegen). Repair surface = no
    /// closed candidate list (author chooses based on placement);
    /// `fix: None` per NeutralOrDeterministic class. Spec-verbatim
    /// name (`worker/inbox-ordering-unspecified`).
    #[serde(rename = "worker/inbox-ordering-unspecified")]
    WorkerInboxOrderingUnspecified,

    /// `<sce:inbox ordering="relaxed">` declared while deploy.placement
    /// pins inbox producer and consumer on different cores. Spec §5.I
    /// lines 1755-1756 verbatim: relaxed on cross-worker shared state
    /// is insufficient. Codegen-invariant guard: silent-skip when
    /// `ForgeCompileOptions.worker_placement` is absent (Q-η5 (a)
    /// precedent), fires when explicit cross-core placement coexists
    /// with `relaxed` ordering. Repair surface = no closed candidate
    /// list (author either changes ordering to `acq_rel` or co-locates
    /// the worker on a single core); `fix: None`. Spec-verbatim name
    /// (`worker/inbox-ordering-relaxed-across-cores`).
    #[serde(rename = "worker/inbox-ordering-relaxed-across-cores")]
    WorkerInboxOrderingRelaxedAcrossCores,

    /// Worker doc compiles against a target machine that did not list it
    /// in `deploy.machines.<m>.workers`. Spec §5.D line 912 verbatim
    /// (`worker/scheduler-unsupported` — "worker count exceeds scheduler
    /// slot count"). The deploy-side anchor for the sum check is
    /// [`MeshDeploySchedulerIncompatibleWithWorkerCount`] (spec §5.K
    /// line 2423); this forge-side code fires when
    /// [`crate::compile_forge_with_deploy`] sees a Worker doc whose
    /// `name` is absent from the resolved machine's `workers` map,
    /// signaling the worker was not budgeted into the cooperative
    /// scheduler's tick window. Repair surface = no closed candidate
    /// list (author either adds the worker to deploy.yaml or removes
    /// the Worker doc); `fix: None` per NeutralOrDeterministic class.
    #[serde(rename = "worker/scheduler-unsupported")]
    WorkerSchedulerUnsupported,

    // ── §5.D C2 follow-up: SCXML-side `<sce:outbox ref>` cross-
    //    resolution against the build-wide
    //    [`crate::forge::cross_doc_registry::SceCrossDocRegistry`].
    //    Atomic A (`3e5e26e9`) landed the orchestrator + registry +
    //    3-kind variant; Atomic B (this batch) adds the validator
    //    consumer. Q-Outbox-6 (a) strict-suffix lock (`.inbox`) +
    //    Q-Outbox-3 (b) recipient kinds (statechart + worker) +
    //    Q-Outbox-8 (c) 3-code split per repair axis (unknown /
    //    wrong-kind / suffix-invalid).
    /// `<sce:outbox ref>`'s owner segment does not match any
    /// statechart or worker doc in the build's cross-doc registry.
    /// Distinct from [`Self::WorkerOutboxTargetWrongKind`] (owner
    /// found but kind not in {statechart, worker}); distinct from
    /// [`Self::WorkerOutboxTargetSuffixInvalid`] (syntactic suffix
    /// failure independent of registry state). Repair surface =
    /// `Fix::ReplaceOneOf` over the sorted union of statechart +
    /// worker doc names (each suffixed with `.inbox` so candidates
    /// are drop-in replacements). Non-spec diagnostic per Q-Outbox-8
    /// (c) lock 2026-05-12.
    #[serde(rename = "worker/outbox-ref-unknown")]
    WorkerOutboxRefUnknown,

    /// `<sce:outbox ref>`'s owner segment resolves in the cross-doc
    /// registry but to a kind not in {statechart, worker}. Today the
    /// only other kind the registry holds is `link` (forge link
    /// imports), so a wrong-kind hit usually means the author confused
    /// a link import alias with a statechart name. Repair surface =
    /// `Fix::ReplaceOneOf` over the same sorted statechart + worker
    /// `.inbox` set as [`Self::WorkerOutboxRefUnknown`]. Non-spec
    /// diagnostic per Q-Outbox-8 (c) lock 2026-05-12.
    #[serde(rename = "worker/outbox-target-wrong-kind")]
    WorkerOutboxTargetWrongKind,

    /// `<sce:outbox ref>` declares a suffix other than `inbox`
    /// (including missing dot entirely). Spec §5.D line 895 example
    /// writes `session_fsm.inbox`; spec line 1998 codegen table fixes
    /// the recipient queue name to `inbox`. Repair is deterministic:
    /// keep the authored owner segment, replace the suffix with
    /// `inbox`. `Fix::ReplaceWith` carries `"{owner}.inbox"`. Single-
    /// value repair → `NeutralOrDeterministic` non-overlap class.
    /// Non-spec diagnostic per Q-Outbox-6 (a) + Q-Outbox-8 (c) lock
    /// 2026-05-12.
    #[serde(rename = "worker/outbox-target-suffix-invalid")]
    WorkerOutboxTargetSuffixInvalid,

    // ── §5.M Fragment-reassembly buffer-pool variant diagnostics
    //    (watching-zenoh RFC §5.M lines 2944-2945). C9-α ships the two
    //    parse-level structure codes here; cross-doc validators that
    //    reference §5.K `links.<name>.{mtu_bytes, expected_p99_bytes,
    //    domain_attrs.trust_class}` (6-8 codes including
    //    `reassembly/max-fragments-insufficient-for-mtu`,
    //    `reassembly/untrusted-link-binding`, etc.) defer to C9-β
    //    co-landing with C13 §5.K `links:` block. The
    //    `reassembly/per-peer-quota-build-invariant-violated` invariant
    //    (peer_table.capacity × per-peer-quota ≥ slot_count) defers to
    //    C9-β where `peer_table.capacity` source becomes available.
    //    Codegen-side per-slot bitmap/deadline/peer-id emission +
    //    `reassembly/peer-id-not-zid-on-established-session` template-
    //    regression guard defer to C9-γ. Listener-link sibling-split
    //    (`link/listener-link-not-paired-with-established-sibling` +
    //    `reassembly/binding-on-unpaired-listener`) is a §5.C codegen
    //    contract (spec line 2820-2824) and lands with C10/C11.
    //    Q-C9-6 (a) lock: backend coverage diagnostic reuses
    //    `codegen/mcu-class-kind-on-non-mcu-language` per spec line 2664
    //    verbatim — no new MCU-class code minted here. ──
    /// `<sce:variant>reassembly</sce:variant>` declared on a buffer-pool
    /// without an accompanying `<sce:max-fragments-per-message>` sibling.
    /// RFC §5.M line 2944 names this code. Spec line 2688 fixes the
    /// fragment-index bitmap width per slot to the
    /// `max-fragments-per-message` value; without it codegen has no
    /// upper bound on the per-slot fragment-ID tracking. The single
    /// recoverable repair is to add the missing element — but the
    /// concrete fragment count is author-domain knowledge (depends on
    /// the wire framer's per-message maximum), so the non-overlap class
    /// is `NeutralOrDeterministic` (no closed candidate set).
    #[serde(rename = "mem/reassembly-pool-variant-missing-max-fragments")]
    MemReassemblyPoolVariantMissingMaxFragments,

    /// `<sce:variant>reassembly</sce:variant>` declared on a buffer-pool
    /// without an accompanying `<sce:reassembly-timeout-ms>` sibling.
    /// RFC §5.M line 2945 names this code. Spec line 2689 + line 2696
    /// fix the per-slot deadline field to this value; without it the
    /// reassembly FSM has no `Receiving → TimedOut` edge timer
    /// (`docs/reassembly-fsm.md` §2.4.5). The single recoverable
    /// repair is to add the missing element with a concrete millisecond
    /// value — author-domain knowledge (depends on link latency budget
    /// and acceptable hold time), so the non-overlap class is
    /// `NeutralOrDeterministic` (no closed candidate set).
    #[serde(rename = "mem/reassembly-pool-variant-missing-timeout")]
    MemReassemblyPoolVariantMissingTimeout,

    // ── §5.M Fragment-reassembly cross-doc validators (C13-α-2 +
    //    C9-β co-land, watching-zenoh RFC §5.M lines 2946-2995).
    //    Each fires from a cross-doc resolver that walks
    //    `deploy.links.<X>` → forge `<sce:link name=X>` → its
    //    `<sce:rx-pool ref=Y>` → `ForgePoolRegistry`'s BufferPoolModel
    //    for Y. Silent-skip on any join-step failure per Q-η5 (a) /
    //    Q-C13-α2-4 (a). All six ride NeutralOrDeterministic — every
    //    code has multi-axis repair paths (raise slot_size, lower
    //    expected_p99, change max-fragments-per-message, lower
    //    mtu_bytes, change trust_class, raise worker_slot_budget_us)
    //    that are author-domain decisions rather than closed
    //    candidate sets. ──

    /// `<sce:rx-pool ref>` bound to a link whose `mtu_bytes` exceeds
    /// the pool's `<sce:slot-size>`. RFC §5.M line 2946 names this
    /// code. The slot cannot hold a single full-MTU datagram; even
    /// the non-fragmented happy path fails to admit one wire frame.
    /// Repair is multi-axis: raise `<sce:slot-size>` on the pool,
    /// lower `mtu_bytes` on the link, or unbind the pool and emit a
    /// reassembly-variant pool sized to fragments instead.
    #[serde(rename = "mem/reassembly-slot-size-below-declared-mtu")]
    MemReassemblySlotSizeBelowDeclaredMtu,

    /// Reassembly-variant pool's `<sce:slot-size>` cannot hold the
    /// worst-case reassembled message implied by
    /// `<sce:max-fragments-per-message>` and the bound link's
    /// `mtu_bytes`. RFC §5.M line 2947-2949 verbatim: `slot_size <
    /// max-fragments-per-message × mtu_bytes`. Hard error — worst-case
    /// message cannot complete reassembly within declared bounds.
    /// Repair: raise `<sce:slot-size>`, lower
    /// `<sce:max-fragments-per-message>`, or lower link `mtu_bytes`.
    #[serde(rename = "reassembly/max-fragments-insufficient-for-mtu")]
    ReassemblyMaxFragmentsInsufficientForMtu,

    /// Build-time stage-copy rate gate. RFC §5.M line 2950-2952
    /// verbatim: `(expected_p99_bytes - rx_pool.slot_size) /
    /// expected_p99_bytes > 0.25`. The 25% threshold is the spec's
    /// default warning point — beyond it, the link runs the
    /// ARCHITECTURE §9.3 stage-copy path on >¼ of inbound traffic.
    /// Warning (not hard error per C13-α-2 scope; suppressible via
    /// `<sce:accept-stage-copy-rate>` on the link source; promotable
    /// to `pool/stage-copy-policy-error` via §5.K
    /// `pool_defaults.stage_copy_policy: error` — both routes defer
    /// to C13-γ). Silent-skip when no regular RX pool is bound (the
    /// "regular RX pool" the formula references doesn't exist), per
    /// Q-C13-α2-4 (a).
    #[serde(rename = "reassembly/expected-fragmentation-rate-high")]
    ReassemblyExpectedFragmentationRateHigh,

    /// Reassembly pool bound to a link whose
    /// `domain_attrs.trust_class` is `untrusted` or `session_arming`.
    /// RFC §5.M line 2964-2969 verbatim: hard error. Fragmentation
    /// on these links is forbidden; only `established_session` links
    /// may carry fragmented traffic. The constraint defends against
    /// UDP source-IP spoofing exhausting per-peer quota space (the
    /// fragment-flood attack vector named at spec line 2962-2963).
    /// Repair: change `trust_class` to `established_session` (only if
    /// the link is in fact post-handshake), or remove the
    /// reassembly-pool binding from this link.
    #[serde(rename = "reassembly/untrusted-link-binding")]
    ReassemblyUntrustedLinkBinding,

    /// Reassembly pool bound to a link, but the link has no
    /// `domain_attrs` block at all (so `trust_class` is implicitly
    /// undeclared). RFC §5.M line 2970-2975 verbatim: hard error;
    /// build cannot decide whether the binding is safe. Per
    /// Q-C13-α2-8 (a) lock: absence of the `domain_attrs` block is
    /// the trigger — when `domain_attrs` is declared without
    /// `trust_class`, C13-α-1's parser rejection
    /// (`LinkDomainAttrs.trust_class` is required-when-block-declared)
    /// catches it earlier. Repair: declare `trust_class:
    /// established_session` for data-plane links, or remove the
    /// reassembly-pool binding for control-plane links.
    #[serde(rename = "reassembly/trust-class-missing-on-fragmenting-link")]
    ReassemblyTrustClassMissingOnFragmentingLink,

    /// Stage-copy WCET vs cooperative slot budget. RFC §5.M line
    /// 2995-2999 verbatim: `expected_p99_bytes ×
    /// memcpy_cycles_per_byte / clock_freq_mhz >
    /// worker_slot_budget_us`. When triggered, the implicit memcpy in
    /// the stage-copy path alone blows the cooperative slot, starving
    /// Keepalive and other parallel-region timers (ARCHITECTURE §9.3
    /// + §3.4). Silent-skip when any of the four platform/scheduler
    /// inputs are absent (per Q-η5 (a) precedent). Repair: raise
    /// `worker_slot_budget_us`, lower `expected_p99_bytes` so stage
    /// copy is never invoked at that size, or raise the bound pool's
    /// `<sce:slot-size>` to absorb p99 without invoking stage copy.
    #[serde(rename = "reassembly/stage-copy-wcet-exceeds-slot-budget")]
    ReassemblyStageCopyWcetExceedsSlotBudget,

    /// Codegen self-check: the emitted reassembly-variant pool's
    /// per-slot peer-id is not the 16-byte ZID signature mandated for
    /// `trust_class: established_session` bindings. RFC §5.M line
    /// 2976-2981 verbatim: "internal codegen invariant: per-peer quota
    /// check on an `established_session` link must use ZID (handshake-
    /// derived) as the peer key, not the wire source address. Codegen
    /// guard against template regression that would silently fall back
    /// to spoofable wire ID."
    ///
    /// Wired into [`super::generator::render_buffer_pool_rust`] and
    /// [`super::generator::render_buffer_pool_c`] as a post-render
    /// substring check when the resolved variant is
    /// [`super::model::BufferPoolVariant::Reassembly`]. In well-formed
    /// templates the diagnostic never fires — the reassembly variant
    /// only resolves on `established_session` links (cross-doc
    /// validator `reassembly/untrusted-link-binding` gates non-
    /// `established_session` bindings upstream) and the template
    /// hardcodes the 16-byte ZID typedef. The diagnostic exists as a
    /// regression guard for future template edits that drop the ZID
    /// shape; mirrors the `mem/inter-pool-padding-not-emitted`
    /// self-check shape (generator.rs:10225).
    #[serde(rename = "reassembly/peer-id-not-zid-on-established-session")]
    ReassemblyPeerIdNotZidOnEstablishedSession,

    /// Codegen self-check: a `<sce:link>` whose orchestrator-resolved
    /// (deploy `domain_attrs.trust_class: session_arming` × machine
    /// source SCXML `Accepting.*` substate-present) pair makes it a
    /// listener emitted the Listener half but did NOT emit the paired
    /// `established_session` Sibling half. watching-zenoh RFC §5.C
    /// lines 849-856 verbatim: "codegen self-check that every
    /// `session_arming` listener instance has emitted its
    /// `established_session` sibling per the 'Listener-link sibling
    /// emission' contract above. Hard error. This is a template
    /// regression guard, unreachable in well-formed codegen; it exists
    /// to ensure the listener emission template cannot silently
    /// regress to single-instance shape (which would re-introduce the
    /// OQ-W22 contradiction)."
    ///
    /// Wired into [`super::generator::render_link_rust`] +
    /// [`super::generator::render_link_c`] as a post-render substring
    /// check when [`crate::ForgeCompileOptions::listener_links`]
    /// contains the rendered link's name: the emitted output must
    /// carry the durable `EstablishedSession` type-name suffix (Rust)
    /// / `_established_session_t` typedef (C11). In well-formed
    /// templates the diagnostic never fires (the per-language link
    /// template emits both halves unconditionally when the listener
    /// flag is set); mirrors the `reassembly/peer-id-not-zid-on-
    /// established-session` self-check shape per generator.rs:10225.
    /// NeutralOrDeterministic (Q-C10-7 a) — pure template-regression
    /// guard with no closed candidate set.
    #[serde(rename = "link/listener-link-not-paired-with-established-sibling")]
    LinkListenerLinkNotPairedWithEstablishedSibling,

    /// Author-facing hard error: a reassembly-pool binding has
    /// resolved to a `session_arming` link instance whose paired
    /// `established_session` sibling does not exist. watching-zenoh
    /// RFC §5.M lines 2982-2994 verbatim: "a reassembly-pool binding
    /// has resolved to a `session_arming` link instance whose paired
    /// `established_session` sibling does not exist. Hard error. In
    /// well-formed codegen this is unreachable (the listener-link
    /// sibling emission contract in §5.C guarantees pairing); the
    /// diagnostic guards SCXML that explicitly targets the
    /// `session_arming` half (bypassing the auto-resolution) and any
    /// future schema evolution that introduces non-listener
    /// `session_arming` instances. Distinct from
    /// `reassembly/untrusted-link-binding` (which rejects bindings to
    /// `untrusted` and to standalone `session_arming` non-listeners)
    /// and from `link/listener-link-not-paired-with-established-
    /// sibling` (which is the §5.C-side codegen self-check)."
    ///
    /// Wired into
    /// [`crate::mesh::deploy::validate_reassembly_cross_doc`] (C13-α-2
    /// + C13-γ landing site): when the bound link's resolved
    /// `trust_class` is `session_arming` AND the orchestrator-resolved
    /// listener-link set does NOT contain the link name (i.e. no
    /// `Accepting.*` substate on the machine's source SCXML), the
    /// validator fires this code in place of the historic
    /// `reassembly/untrusted-link-binding` for the session-arming
    /// subcase. NeutralOrDeterministic (Q-C10-7 a) — two valid repair
    /// paths: add an `Accepting.*` substate to the machine's source
    /// SCXML (making the link a real listener so the sibling
    /// auto-synthesizes), or remove the reassembly-pool binding.
    #[serde(rename = "reassembly/binding-on-unpaired-listener")]
    MeshDeployReassemblyBindingOnUnpairedListener,

    /// Watching-zenoh RFC §5.N line 3060 verbatim
    /// (`link/concurrent-count-exceeds-scheduler-slots`) — MCU-only
    /// cooperative-scheduler accounting: more links than the
    /// scheduler can accommodate within one tick. Hard error.
    ///
    /// Slot ceiling derivation (Q-C10-β-2 a): `floor(tick_period_us
    /// / per_link_budget_us)` mirrors the C2-γ
    /// `validate_machine_scheduler_worker_capacity` precedent at
    /// mesh/deploy.rs `worker_slot_budget_us`. Validator silent-
    /// skips when `platform.class != mcu`, `scheduler.kind !=
    /// cooperative`, `tick_period_us` absent, or `per_link_budget_us`
    /// absent (per Q-η5 (a) precedent). Repair: raise
    /// `per_link_budget_us`, lower `tick_period_us`, or remove a
    /// link declaration from `machines.<m>.links`.
    #[serde(rename = "link/concurrent-count-exceeds-scheduler-slots")]
    LinkConcurrentCountExceedsSchedulerSlots,

    /// Watching-zenoh RFC §5.N line 3061 verbatim
    /// (`link/per-link-budget-exceeds-tick-period`). Per-link budget
    /// must fit inside one cooperative tick:
    /// `per_link_budget_us > tick_period_us` is the single-link
    /// sanity check (Q-C10-β-3 a literal code-name reading). Hard
    /// error. NeutralOrDeterministic — two-axis repair (lower
    /// `per_link_budget_us` or raise `tick_period_us`). Validator
    /// silent-skips when either input absent or scheduler is not
    /// cooperative.
    #[serde(rename = "link/per-link-budget-exceeds-tick-period")]
    LinkPerLinkBudgetExceedsTickPeriod,

    /// Watching-zenoh RFC §5.N line 3062 verbatim
    /// (`link/inbound-event-queue-unsized`). A `<sce:link>` declares
    /// at least one `<sce:inbound>` event but the downstream FSM's
    /// event queue depth is undeclared. Hard error.
    ///
    /// Two acceptable size sources per Q-C10-β-4 a: SCXML
    /// per-instance `<scxml sce:capacity="N">` (preferred — pins
    /// the FSM-side spsc capacity to the machine's actual event
    /// volume) or deploy
    /// `machines.<m>.scheduler.default_event_queue_capacity`
    /// (fallback — single default applied to every undeclared
    /// machine on the deploy). Validator extends
    /// `compile_scxml_with_imports` pass-2 (C13 + C10-α
    /// orchestrator-level precedent). Silent-skip when the link has
    /// no inbound events declared or when no SCXML imports the link
    /// (no FSM downstream to size). NeutralOrDeterministic —
    /// two-axis repair (per-instance vs per-machine source).
    #[serde(rename = "link/inbound-event-queue-unsized")]
    LinkInboundEventQueueUnsized,

    // ── §5.L Bounded-collection kind diagnostics (watching-zenoh RFC
    //    §5.L lines 2540-2655). C6-α ships the two structure-only codes
    //    here; cross-doc (element-type-not-a-kind / index-by-field-
    //    missing / multi-writer-without-atomics) defers to C6-β,
    //    deploy-time (capacity-unresolved) defers to C6-γ. ──
    /// `<sce:ordering>sorted-by(index-by)</sce:ordering>` declared
    /// without an accompanying `<sce:index-by field="..."/>` element.
    /// Spec line 2559 fixes the SortedByIndex iteration order to the
    /// `index-by` field; without that field there is no comparator
    /// the codegen can lower. The single recoverable repair is to add
    /// an `<sce:index-by>` element naming a field of the element-type
    /// struct — but the field name is author-domain knowledge, so the
    /// non-overlap class is `NeutralOrDeterministic` (no closed
    /// candidate set; no expected metadata).
    #[serde(rename = "collection/ordering-sorted-requires-index-by")]
    CollectionOrderingSortedRequiresIndexBy,

    /// `<sce:on-overflow>oldest-wins</sce:on-overflow>` declared together
    /// with `<sce:ordering>sorted-by(index-by)</sce:ordering>`. Spec line
    /// 2655 lists this combination as the explicit anti-pattern: the
    /// `oldest-wins` policy presumes a temporal ordering (insertion
    /// timestamp) that the `sorted-by` mode replaces with the
    /// `index-by` field comparator, so "oldest" has no defined meaning.
    /// Repair is deterministic — keep the `oldest-wins` policy, change
    /// `<sce:ordering>` to `insertion`. `Fix::ReplaceWith` carries
    /// `"insertion"`; single-value repair → `NeutralOrDeterministic`.
    #[serde(rename = "collection/overflow-policy-oldest-wins-requires-ordering-insertion")]
    CollectionOverflowPolicyOldestWinsRequiresOrderingInsertion,

    /// `<sce:element-type>NAME</sce:element-type>` body text does not
    /// resolve to a codec-kind struct or procedure-kind state record
    /// anywhere in the build's forge-doc set. Spec line 2566-2567
    /// restricts element types to these two kinds. Cross-doc validator
    /// in C6-β consumes the orchestrator-assembled element-type
    /// candidate map (`HashMap<String, ForgeDocument>` populated only
    /// for codec + procedure docs). Closed candidate set rides
    /// `Fix::ReplaceOneOf` ⇒ FixCarriesCandidates non_overlap_class.
    #[serde(rename = "collection/element-type-not-a-kind")]
    CollectionElementTypeNotAKind,

    /// `<sce:index-by field="X"/>` names a field that does not exist
    /// on the resolved element-type struct (codec.fields or
    /// procedure.inputs + internals). Spec line 2615 fixes
    /// `find_by_index` to a declared struct field. Closed candidate
    /// set (sorted field names from the resolved element type) rides
    /// `Fix::ReplaceOneOf` ⇒ FixCarriesCandidates non_overlap_class.
    #[serde(rename = "collection/index-by-field-missing")]
    CollectionIndexByFieldMissing,

    /// `<sce:concurrency>multi-writer</sce:concurrency>` declared
    /// without any §5.I atomic intrinsic having been imported via
    /// `<sce:extern>` anywhere in the build. Spec lines 2560-2562 fix
    /// multi-writer codegen to acquire/release atomics on head/tail;
    /// the build's `<sce:extern>` trust-surface must acknowledge
    /// atomic intrinsics. No closed candidate set — the C4 baseline
    /// atomic family is too large for a useful `Fix::ReplaceOneOf`,
    /// so `fix: None` ⇒ NeutralOrDeterministic non_overlap_class.
    #[serde(rename = "collection/multi-writer-without-atomics")]
    CollectionMultiWriterWithoutAtomics,

    /// `<sce:capacity source="deploy" key="machines.<machine>.limits.<limit>"/>`
    /// names a deploy-key whose `<limit>` segment is not declared
    /// under `machines.<machine>.limits:` in deploy.yaml. Spec lines
    /// 2583-2585 fix `<sce:capacity source="deploy">` to a per-machine
    /// limit lookup whose unresolved state blocks the codegen-time
    /// compile-time constant lowering. Fires only on the
    /// `compile_forge_with_deploy` path (deploy + target_machine both
    /// Some); silent-skips when the key's machine segment != target
    /// per Q-η5 (a) precedent. Closed candidate set (sorted declared
    /// limit names under target_machine.limits) rides
    /// `Fix::ReplaceOneOf` ⇒ FixCarriesCandidates non_overlap_class.
    #[serde(rename = "collection/capacity-unresolved")]
    CollectionCapacityUnresolved,

    // ── §5.D Timer kind diagnostics (watching-zenoh RFC §5.D
    //    lines 909-910). Both codes fire on the MCU cooperative
    //    scheduler axis — silent-skip on AP / preemptive targets
    //    where the runtime owns deadline tracking. ──
    /// `<sce:period>` declared shorter than `scheduler.tick_period_us`.
    /// The cooperative scheduler cannot dispatch a timer faster than
    /// its own tick rate, so a period below the tick rate would miss
    /// every other deadline. Fired from
    /// [`crate::compile_forge_with_deploy`] when a Timer doc
    /// resolves against a `scheduler.kind: cooperative` machine
    /// with `tick_period_us` declared.
    #[serde(rename = "timer/period-below-tick-rate")]
    TimerPeriodBelowTickRate,

    /// Total `Timer` doc count for a machine exceeds
    /// `scheduler.timer_wheel_depth`. The MCU static timer wheel is
    /// sized at compile time; declaring more timers than slots
    /// overflows the wheel. Fired from the deploy.yaml validator
    /// after counting `machines.<m>.timers.len()`. Silent-skip when
    /// `timer_wheel_depth` is absent (Q-η5 (a) precedent — deploy-
    /// unaware paths don't have the wheel sizing information).
    #[serde(rename = "timer/slot-overflow")]
    TimerSlotOverflow,

    // ── §5.I `<sce:extern>` whitelisted intrinsic registry
    //    (watching-zenoh RFC §5.I, Atomic A). Four spec-verbatim
    //    codes (lines 1847-1850) that fire at parse-time on
    //    `<sce:extern>` declarations. The 101-symbol baseline lives
    //    in `crate::forge::intrinsic_registry::BASELINE_SYMBOLS`;
    //    closed-set lookup is in `extern_validator::validate_extern`.
    //    Diagnostic names are spec verbatim per
    //    `feedback_spec_mirror_parity.md`. Plugin-extension axes
    //    (`extern/target-plugin-*`, `extern/linker-flavor-*`) defer
    //    to Atomic B/C of `rfc-sce-call-intrinsic-registry.md`. ──
    /// `<sce:extern name>` references a symbol absent from the
    /// §5.I baseline registry. Repair surface = `Fix::ReplaceOneOf`
    /// over closest baseline names (bounded top-8). Q-Call-4 (a)
    /// parse-time lock; mirrors `LinkLinkClassUnknown` (B6-γ)
    /// closed-enum precedent.
    #[serde(rename = "extern/symbol-not-in-whitelist")]
    ExternSymbolNotInWhitelist,

    /// `<sce:extern abi>` does not match the registry entry's
    /// canonical ABI. Repair surface = `Fix::ReplaceOneOf` from
    /// the closed two-element set `["c", "rust"]`.
    #[serde(rename = "extern/abi-mismatch")]
    ExternAbiMismatch,

    /// `<sce:extern sig>` does not byte-match the registry entry's
    /// canonical signature. Repair surface = `Fix::Replace` with the
    /// canonical sig (registry is the source of truth).
    #[serde(rename = "extern/signature-mismatch")]
    ExternSignatureMismatch,

    /// `<sce:extern name>` is an atomic-family base
    /// (`sce_atomic_load`, `sce_atomic_cas_weak`, `sce_atomic_fence`,
    /// …) written without the required `_<ordering>_<width>` (or
    /// `_<ordering>` for fences) suffix. Repair surface =
    /// `Fix::ReplaceOneOf` over the legal completions for that
    /// family. Distinct from `ExternSymbolNotInWhitelist` because the
    /// repair shape is "pick a suffix" rather than "pick a different
    /// symbol entirely".
    #[serde(rename = "extern/ordering-unspecified")]
    ExternOrderingUnspecified,

    /// Target plugin YAML (`extern_symbols.target_plugin: <path>`)
    /// declares a symbol whose `name` already exists in the §5.I
    /// baseline registry. Spec line 1852 verbatim semantic: "target
    /// plugin redefines a core whitelist symbol". Q-Call-6 (a)
    /// additive-composition lock — plugin entries extend, never
    /// override. Repair surface is non-algorithmic (`fix: None`):
    /// the plugin author must rename the conflicting entry to a
    /// non-baseline name; SCE cannot synthesize a candidate name.
    #[serde(rename = "extern/target-plugin-symbol-conflict")]
    ExternTargetPluginSymbolConflict,

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
    // ── watching-zenoh RFC §5.E B7-η' deploy.yaml stage_pool family ──
    // These diagnostics validate the deploy.yaml `binding.stage_pool:`
    // cross-reference into the forge buffer-pool registry. Distinct
    // family from the `mesh/deploy-pool-*` SOME/IP-instance-routing
    // diagnostics above: those concern routing-pool placeholders for
    // RPC bindings, while these concern buffer-pool kind references
    // for `Sample::take()` stage copies (§5.E Sample API contract).
    #[serde(rename = "mesh/deploy-stage-pool-not-declared")]
    MeshDeployStagePoolNotDeclared,
    #[serde(rename = "mesh/deploy-stage-pool-wrong-kind")]
    MeshDeployStagePoolWrongKind,
    #[serde(rename = "mesh/deploy-stage-pool-transport-mismatch")]
    MeshDeployStagePoolTransportMismatch,
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
    #[serde(rename = "mesh/deploy-partition-name-not-identifier")]
    MeshDeployPartitionNameNotIdentifier,
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
    #[serde(rename = "mesh/deploy-someip-liveness-service-id-overflow")]
    MeshDeploySomeipLivenessServiceIdOverflow,
    #[serde(rename = "mesh/deploy-someip-liveness-service-id-pin-out-of-range")]
    MeshDeploySomeipLivenessServiceIdPinOutOfRange,
    #[serde(rename = "mesh/deploy-someip-liveness-service-id-pin-collision")]
    MeshDeploySomeipLivenessServiceIdPinCollision,
    #[serde(rename = "mesh/deploy-someip-machine-liveness-service-id-overflow")]
    MeshDeploySomeipMachineLivenessServiceIdOverflow,
    #[serde(rename = "mesh/deploy-someip-machine-liveness-service-id-pin-out-of-range")]
    MeshDeploySomeipMachineLivenessServiceIdPinOutOfRange,
    #[serde(rename = "mesh/deploy-someip-machine-liveness-service-id-pin-collision")]
    MeshDeploySomeipMachineLivenessServiceIdPinCollision,
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
    // §14 per-machine platform/scheduler schema (RFC §5.K, Phase A2)
    #[serde(rename = "mesh/deploy-platform-class-os-mismatch")]
    MeshDeployPlatformClassOsMismatch,
    /// Watching-zenoh RFC §5.K line 2426 verbatim
    /// (`deploy/worker-stack-budget-missing`). Cooperative scheduler
    /// declared without `worker_stack_budget`. Renamed from the
    /// SCE-Mesh-prefix wire (`mesh/deploy-scheduler-cooperative-missing-stack-budget`)
    /// to the watching-zenoh-prefix wire at C2-γ landing; SCE Mesh §14
    /// continues to anchor to the same variant.
    #[serde(rename = "deploy/worker-stack-budget-missing")]
    MeshDeploySchedulerCooperativeMissingStackBudget,
    /// Watching-zenoh RFC §5.K line 2428-2429 verbatim
    /// (`deploy/worker-slot-budget-missing`). Cooperative scheduler
    /// declared without `worker_slot_budget_us`. Validator
    /// [`crate::mesh::deploy::validate_worker_slot_budget_required_when_cooperative`]
    /// fires at deploy.yaml parse time.
    #[serde(rename = "deploy/worker-slot-budget-missing")]
    MeshDeploySchedulerCooperativeMissingSlotBudget,
    /// Watching-zenoh RFC §5.K line 2430-2431 verbatim
    /// (`deploy/keepalive-jitter-budget-missing`). Cooperative
    /// scheduler declared without `keepalive_jitter_budget_us`.
    /// Validator
    /// [`crate::mesh::deploy::validate_keepalive_jitter_required_when_cooperative`]
    /// fires at deploy.yaml parse time.
    #[serde(rename = "deploy/keepalive-jitter-budget-missing")]
    MeshDeploySchedulerCooperativeMissingKeepaliveJitterBudget,
    /// Watching-zenoh RFC §5.K line 2423 verbatim
    /// (`deploy/scheduler-incompatible-with-worker-count`). Cooperative
    /// scheduler's derived slot count (`floor(tick_period_us /
    /// worker_slot_budget_us)`) is less than the number of workers
    /// declared under `machines.<m>.workers`. Validator
    /// [`crate::mesh::deploy::validate_machine_scheduler_worker_capacity`]
    /// fires at deploy.yaml parse time after the three "*-missing"
    /// validators have confirmed the budget fields are present. The
    /// forge-side anchor for the same axis is [`WorkerSchedulerUnsupported`]
    /// (spec §5.D line 912).
    #[serde(rename = "deploy/scheduler-incompatible-with-worker-count")]
    MeshDeploySchedulerIncompatibleWithWorkerCount,

    // ── §5.K `links:` block parse-time + cross-doc validators
    //    (watching-zenoh RFC §5.K lines 2232-2540, C13-α-1). 9 codes:
    //    7 spec-named (`link-driver-unknown` at line 2421, `link-mtu-*`
    //    at lines 2440-2448, `link-burst-*` at lines 2489-2503) plus 2
    //    cross-doc joins (Q-C13-5 a) that pair forge `<sce:link name>`
    //    documents against deploy.yaml `machines.<n>.links.<name>`
    //    entries. C13-β anti-flood / stateless_accept codes + the 6
    //    C9-β reassembly cross-doc codes (`mem/reassembly-slot-size-
    //    below-declared-mtu`, `reassembly/{max-fragments-insufficient-
    //    for-mtu, expected-fragmentation-rate-high, untrusted-link-
    //    binding, trust-class-missing-on-fragmenting-link, stage-copy-
    //    wcet-exceeds-slot-budget}`) defer to C13-α-2 follow-up atomic
    //    per scope-split decision. ──

    /// Watching-zenoh RFC §5.K line 2421 verbatim
    /// (`deploy/link-driver-unknown`). `machines.<n>.links.<name>.driver`
    /// value is not in the C13-α-known-driver baseline (currently
    /// `{lwip_udp, lwip_tcp}`; extends as new forge link-kind docs
    /// ship) AND not declared as a forge `<sce:link>` document name in
    /// the build's [`crate::forge::cross_doc_registry::SceCrossDocRegistry`].
    /// `Fix::ReplaceOneOf` over the known-driver + forge-link-doc-name
    /// union, sorted. Q-C13-8 (a) lock: enum-shape kept as `String` so
    /// forge-side driver authoring extends organically; closed-allowlist
    /// validator is the gate, not the type system.
    #[serde(rename = "deploy/link-driver-unknown")]
    MeshDeployLinkDriverUnknown,

    /// Watching-zenoh RFC §5.K line 2440-2442 verbatim
    /// (`deploy/link-mtu-missing-on-fragmenting-link`). A
    /// `machines.<n>.links.<name>` entry is bound to a forge
    /// `<sce:link name="...">` whose FSM emits/consumes Fragment codec
    /// events, but `mtu_bytes` is absent. Without it the build cannot
    /// size reassembly pool slots per §5.M C9-β. The C13-α-1 detector
    /// uses a conservative under-approximation per Q-C13-2 / Q-C13-5
    /// shape — it fires for every `domain_attrs.trust_class:
    /// established_session` link missing `mtu_bytes`, since
    /// `established_session` is the only trust class permitted to carry
    /// Fragment traffic per RFC §5.M line 2731. Precise Fragment-FSM
    /// detection awaits the §5.M reassembly-pool-bound-to-link
    /// cross-doc validator (C13-α-2 + C9-β co-land); the C13-α-1
    /// under-approximation surfaces the same author error class earlier.
    #[serde(rename = "deploy/link-mtu-missing-on-fragmenting-link")]
    MeshDeployLinkMtuMissingOnFragmentingLink,

    /// Watching-zenoh RFC §5.K line 2443-2445 verbatim
    /// (`deploy/link-mtu-below-driver-floor`). `mtu_bytes` declared
    /// smaller than the driver's minimum payload (e.g. UDP/IPv6's
    /// 56-byte floor); driver default would override silently. The
    /// known-driver baseline carries each driver's floor (currently
    /// `{lwip_udp: 28, lwip_tcp: 40}` from IPv4 minimum-header
    /// arithmetic); unknown drivers fall back to silent-skip until
    /// their floor is registered.
    #[serde(rename = "deploy/link-mtu-below-driver-floor")]
    MeshDeployLinkMtuBelowDriverFloor,

    /// Watching-zenoh RFC §5.C lines 765-771 + §8 Q8 line 3747
    /// (`deploy/link-driver-class-mismatch`). The forge
    /// `<sce:link-class>` value on the link doc does not match the
    /// protocol class implied by the deploy.yaml `driver:` allowlist
    /// entry. C11-WebSocket follow-up RFC §5.1 + parent C11 RFC §5.2
    /// named this sibling atomic; `60fba30c` (C11-WebSocket landing)
    /// satisfied the 4×4 matrix trigger.
    ///
    /// `Fix::ReplaceOneOf` single-element candidate set = the driver
    /// name whose KNOWN_DRIVERS class matches the declared forge
    /// class. The forge-side class swap is a parallel valid repair
    /// path the prose names but the structured fix carries only the
    /// deploy-side axis (one axis per non-overlap shape; the
    /// `LinkClassUnsupportedOnTarget` precedent at §5.C uses the
    /// same single-axis Fix discipline).
    #[serde(rename = "deploy/link-driver-class-mismatch")]
    MeshDeployLinkDriverClassMismatch,

    /// Watching-zenoh RFC §5.K line 2446-2448 verbatim
    /// (`deploy/link-expected-p99-exceeds-mtu`). `expected_p99_bytes >
    /// mtu_bytes` AND no reassembly pool is bound to the link (precise
    /// "no reassembly pool" check defers to C13-α-2 + C9-β co-land; the
    /// C13-α-1 detector fires the warning whenever
    /// `expected_p99_bytes > mtu_bytes` regardless of pool binding,
    /// matching the spec's "the p99 message would always fragment but
    /// no reassembly path exists" intent — authors with a reassembly
    /// pool already see C9-β's `reassembly/expected-fragmentation-
    /// rate-high` consumer, the two diagnostics complement).
    #[serde(rename = "deploy/link-expected-p99-exceeds-mtu")]
    MeshDeployLinkExpectedP99ExceedsMtu,

    /// Watching-zenoh RFC §5.K line 2489-2495 verbatim
    /// (`deploy/link-burst-absorption-insufficient`). `burst_pps × 1s`
    /// of worst-case inbound exceeds the RX pool's drain rate within
    /// one cooperative tick window: `slot_count × ticks_per_second /
    /// burst_pps < 1.0` with safety factor 2.0 (i.e. the check fires
    /// when `slot_count × 1_000_000 / tick_period_us < burst_pps × 2`).
    /// Pool will deplete during burst and drop packets. C13-α-2 fires
    /// from a cross-doc resolver that joins `deploy.links.<X>` → forge
    /// `<sce:link name=X>` → `<sce:rx-pool ref>` → ForgePoolRegistry's
    /// `BufferPoolModel.slot_count`. Silent-skip on any join failure
    /// or missing scheduler.tick_period_us per Q-η5 (a). Multi-axis
    /// repair: raise `<sce:slot-count>` on the pool, lower
    /// `scheduler.tick_period_us`, or switch `rx_dispatch` to
    /// `isr_to_pool` when currently `worker_tick`.
    #[serde(rename = "deploy/link-burst-absorption-insufficient")]
    MeshDeployLinkBurstAbsorptionInsufficient,

    /// Watching-zenoh RFC §5.K line 2496-2500 verbatim
    /// (`deploy/link-rx-dispatch-worker-tick-on-high-burst`).
    /// `rx_dispatch: worker_tick` declared but `burst_pps ×
    /// tick_period_us / 1_000_000 > slot_count` (one tick window of
    /// arrivals overruns the pool). Hard error unless author justifies
    /// via `<sce:accept-burst-drop-rate>` on the link source (a
    /// forge-side opt-out gated by C13-γ, not yet shipped at C13-α-2;
    /// the C13-α-2 detector treats the opt-out as absent). C13-α-2
    /// silent-skips when join steps or `tick_period_us` are missing.
    /// Multi-axis repair: switch `rx_dispatch` to `isr_to_pool`, raise
    /// `<sce:slot-count>` to absorb the per-tick burst, or lower
    /// `tick_period_us` so each window admits fewer arrivals.
    #[serde(rename = "deploy/link-rx-dispatch-worker-tick-on-high-burst")]
    MeshDeployLinkRxDispatchWorkerTickOnHighBurst,

    // ── C13-γ pool_defaults.stage_copy_policy (RFC §5.K lines
    //    2350-2369 + 2504-2519). Three codes wire the policy enum
    //    plus the per-link `<sce:accept-stage-copy-rate>` opt-out
    //    semantics. ──

    /// Watching-zenoh RFC §5.K line 2504-2511 verbatim
    /// (`pool/stage-copy-policy-error`). `pool_defaults.stage_copy_policy:
    /// error` (or `forbid`) AND the §5.M / ARCHITECTURE §9.3
    /// stage-copy-rate gate fires. The warning that would have
    /// surfaced as `reassembly/expected-fragmentation-rate-high` is
    /// promoted to a hard error. Author resolutions: raise
    /// `<sce:slot-size>`, lower `expected_p99_bytes`, or add
    /// `<sce:accept-stage-copy-rate>` on the affected link source
    /// (last option unavailable under `forbid` per
    /// `pool/stage-copy-accept-rejected-under-forbid`).
    /// NeutralOrDeterministic — multi-axis repair, author chooses.
    #[serde(rename = "pool/stage-copy-policy-error")]
    PoolStageCopyPolicyError,

    /// Watching-zenoh RFC §5.K line 2512-2516 verbatim
    /// (`pool/stage-copy-accept-rejected-under-forbid`).
    /// `pool_defaults.stage_copy_policy: forbid` AND a link source
    /// carries `<sce:accept-stage-copy-rate>`. The opt-out is rejected
    /// outright; only structural fixes (raise `<sce:slot-size>` or
    /// lower `expected_p99_bytes`) are accepted under `forbid`.
    /// NeutralOrDeterministic — two valid repair paths (remove the
    /// opt-out vs change policy to `error`).
    #[serde(rename = "pool/stage-copy-accept-rejected-under-forbid")]
    PoolStageCopyAcceptRejectedUnderForbid,

    /// Watching-zenoh RFC §5.K line 2517-2519 verbatim
    /// (`deploy/stage-copy-policy-unknown`).
    /// `pool_defaults.stage_copy_policy` declared with a value other
    /// than `warn` / `error` / `forbid`. Hard error (typo guard).
    /// FixCarriesCandidates over the closed set
    /// [`crate::mesh::deploy::StageCopyPolicy::ALL`].
    #[serde(rename = "deploy/stage-copy-policy-unknown")]
    MeshDeployStageCopyPolicyUnknown,

    // ── C13-β anti-flood + stateless_accept (RFC §5.K lines
    //    2272-2349 + 2449-2473). Five codes wire the conditional
    //    requirement, dead-config rejection, opt-out requirement,
    //    and key-rotation invariant. C13 deferred-2 closes the two
    //    follow-ups originally deferred:
    //      - `deploy/session-arming-quota-vs-peer-table-invariant-violated`
    //        (line 2460-2462) — landed alongside `peer_table` +
    //        `max_handshake_time_s` schema fields on the
    //        `stateless_accept` block (validator wired into
    //        `validate_links` per per-link invariant scope).
    //      - `deploy/stateless-accept-extern-not-whitelisted`
    //        (line 2466-2469) — landed at the orchestrator level
    //        where the §5.I baseline + loaded `target_plugin`
    //        symbols converge, mirroring C4 Atomic B precedent.

    /// Watching-zenoh RFC §5.K line 2449-2451 verbatim
    /// (`deploy/session-arming-quota-missing`). Link declares
    /// `trust_class: session_arming` but no `session_arming_quota`.
    /// Hard error; without a cap an attacker can fill every
    /// `Accepting.*` slot. Repair: declare a concrete u32 value
    /// (MCU default 8, AP default 32 per spec line 2282).
    /// NeutralOrDeterministic — author-domain value, no closed
    /// candidate set.
    #[serde(rename = "deploy/session-arming-quota-missing")]
    MeshDeploySessionArmingQuotaMissing,

    /// Watching-zenoh RFC §5.K line 2452-2453 verbatim
    /// (`deploy/accept-rate-config-missing`). `trust_class:
    /// session_arming` link missing `accept_rate_per_sec` or
    /// `accept_rate_burst`. Hard error.
    /// NeutralOrDeterministic — author-domain values.
    #[serde(rename = "deploy/accept-rate-config-missing")]
    MeshDeployAcceptRateConfigMissing,

    /// Watching-zenoh RFC §5.K line 2454-2459 verbatim
    /// (`deploy/session-arming-fields-on-non-arming-link`). Anti-
    /// flood / stateless_accept fields declared on a `trust_class:
    /// untrusted` or `established_session` link where `Accepting.*`
    /// is never instantiated. Dead config; suggests author confusion
    /// about which link is the listener. Hard error.
    /// NeutralOrDeterministic — two valid repair paths (change
    /// trust_class to session_arming vs remove the dead fields).
    #[serde(rename = "deploy/session-arming-fields-on-non-arming-link")]
    MeshDeploySessionArmingFieldsOnNonArmingLink,

    /// Watching-zenoh RFC §5.K line 2463-2465 verbatim
    /// (`deploy/stateless-accept-required-on-untrusted-source`).
    /// Link with `domain_attrs.untrusted_source: true` but no
    /// `stateless_accept` block. Hard error.
    /// NeutralOrDeterministic — author must author the full block.
    #[serde(rename = "deploy/stateless-accept-required-on-untrusted-source")]
    MeshDeployStatelessAcceptRequiredOnUntrustedSource,

    /// Watching-zenoh RFC §5.K line 2470-2473 verbatim
    /// (`deploy/stateless-accept-key-rotation-shorter-than-lifetime`).
    /// `key_rotation_s × 1000 ≤ 2 × cookie_lifetime_ms`. The
    /// previous-key honor window cannot bridge a rotation, so
    /// handshakes near rotation boundaries get spurious cookie
    /// rejection. Hard error.
    /// NeutralOrDeterministic — two-axis repair (raise key_rotation_s
    /// vs lower cookie_lifetime_ms).
    #[serde(rename = "deploy/stateless-accept-key-rotation-shorter-than-lifetime")]
    MeshDeployStatelessAcceptKeyRotationShorterThanLifetime,

    /// Watching-zenoh RFC §5.K line 2460-2462 verbatim
    /// (`deploy/session-arming-quota-vs-peer-table-invariant-violated`).
    /// `session_arming_quota × max_handshake_time_s > peer_table.capacity`.
    /// A slow legitimate handshake can be evicted under attack when
    /// an attacker churns the quota faster than the per-peer table
    /// absorbs. Hard error.
    /// NeutralOrDeterministic — three-axis repair (raise
    /// `peer_table.capacity`, lower `session_arming_quota`, or lower
    /// `max_handshake_time_s`); the wire payload carries the
    /// violating product in `actual` and the bound in `expected`.
    /// C13 deferred-2 lands the consuming validator inside
    /// `validate_links` alongside the other anti-flood checks.
    #[serde(rename = "deploy/session-arming-quota-vs-peer-table-invariant-violated")]
    MeshDeploySessionArmingQuotaVsPeerTableInvariantViolated,

    /// Watching-zenoh RFC §5.K line 2466-2469 verbatim
    /// (`deploy/stateless-accept-extern-not-whitelisted`).
    /// `hmac_extern` or `rng_extern` symbol not present in the
    /// `sce_intrinsics_runtime` core whitelist AND not declared in
    /// any loaded `target_plugin`. Hard error.
    /// FixCarriesCandidates — closed-set candidates is the sorted
    /// union of §5.I baseline names + target-plugin-loaded names
    /// (`Fix::ReplaceOneOf`). C13 deferred-2 lands the consuming
    /// validator at the orchestrator level
    /// (`compile_scxml_with_imports` + `compile_forge_with_deploy`)
    /// because target-plugin loading is deploy-driven, mirroring
    /// the C4 Atomic B precedent.
    #[serde(rename = "deploy/stateless-accept-extern-not-whitelisted")]
    MeshDeployStatelessAcceptExternNotWhitelisted,

    /// Watching-zenoh RFC §5.K line 2501-2503 verbatim
    /// (`deploy/link-burst-pps-missing-on-isr-dispatch`). The resolved
    /// `rx_dispatch` (per [`super::super::mesh::deploy::LinkConfig::resolved_rx_dispatch`])
    /// is `IsrToPool` but `burst_pps` is not declared. ISR fast-path
    /// requires the rate to size descriptor ring + validate stack
    /// budget. Repair: declare `burst_pps`, or set `rx_dispatch:
    /// worker_tick` explicitly. C13-α-1 detector fires at parse-time
    /// since the resolution is purely intra-link-config.
    #[serde(rename = "deploy/link-burst-pps-missing-on-isr-dispatch")]
    MeshDeployLinkBurstPpsMissingOnIsrDispatch,

    /// C13-α-1 cross-doc validator pair (Q-C13-5 a lock). A forge
    /// `<scxml sce:kind="link" name="X">` document was imported by
    /// some statechart/worker on this machine, but no
    /// `deploy.yaml::machines.<n>.links.<X>` entry exists. Build
    /// cannot resolve the link's `bind` address, `mtu_bytes`, or
    /// `domain_attrs`. `Fix::ReplaceOneOf` over the deploy-side
    /// link-name set for this machine (sorted). Mirrors B6-η
    /// `validate_stage_pool_references` + C2-outbox
    /// `validate_worker_outbox_references` precedents.
    #[serde(rename = "deploy/link-not-declared-in-deploy")]
    MeshDeployLinkNotDeclaredInDeploy,

    /// C13-α-1 cross-doc validator pair (Q-C13-5 a lock). A
    /// `deploy.yaml::machines.<n>.links.<X>` entry exists, but no
    /// forge `<scxml sce:kind="link" name="X">` document was
    /// declared/imported. The deploy entry has no wire framer / codec
    /// pairing — dead config. `Fix::ReplaceOneOf` over the forge-side
    /// link-name set (sorted), pulled from
    /// [`crate::forge::cross_doc_registry::SceCrossDocRegistry::names_of_kind`].
    #[serde(rename = "deploy/link-not-declared-in-forge")]
    MeshDeployLinkNotDeclaredInForge,

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

    // ── Forge generated-source drift detection (watching-zenoh RFC
    //    §6.2.6, B9 atomic 2026-05-14). Single code covers both axes
    //    of `sce-codegen verify`'s recomputed-vs-embedded comparison:
    //    `source-hash` mismatch (input SCXML or deploy.yaml drifted
    //    since generation) and `template-hash` mismatch (codegen
    //    template tree or Cargo.lock drifted). Per Q-§6.2.6-5 lock —
    //    `actual` field carries the axis label (`source`|`template`)
    //    plus the embedded hex; `expected` carries the recomputed
    //    hex. Repair is `sce-codegen <regen-command>` (deterministic,
    //    no candidate set), hence NeutralOrDeterministic. ─────────
    #[serde(rename = "forge/source-hash-mismatch")]
    ForgeSourceHashMismatch,

    // ── Traceability §5.O Atomic 0 — IR provenance pre-emit guard
    //    (watching-zenoh RFC §5.O lines 3289-3290 verbatim:
    //    "XInclude / sce:template composition MUST track per-element
    //    (file_id, line, column) and attach it to every IR node.
    //    Codegen failure ... surfaced via
    //    `traceability/scxml-line-range-missing` (codegen-internal)").
    //    Fires at the compile-pipeline pre-emit pass when a node
    //    eligible for marker emission carries `source_location: None`
    //    — guarantees the per-backend SCE-MAP marker family (Atomic 0
    //    function-level, Atomic 1 per-symbol attribution) is never
    //    silently dropped on an unpopulated IR record. Author repair
    //    is empty: this is a codegen-internal invariant, so authors
    //    never see it in practice; the diagnostic exists so a future
    //    parser edit that creates an IR node without populating
    //    `source_location` surfaces immediately rather than producing
    //    silently-broken codegen.
    #[serde(rename = "traceability/scxml-line-range-missing")]
    TraceabilityScxmlLineRangeMissing,

    // ── Traceability §5.O Atomic 1 — symbol mangling collision
    //    detector. Spec lines 3055-3057 fix the per-symbol mangling
    //    pattern (`<machine>__<state_path>__<artifact>`); XInclude or
    //    `sce:template` composition can produce two distinct IR nodes
    //    whose triples mangle to the same C identifier. The dual-
    //    location payload pins both sites so authors can rename one
    //    of the two states to break the collision.
    #[serde(rename = "traceability/state-id-collision")]
    TraceabilityStateIdCollision,

    // ── Traceability §5.O Atomic 1 — mangled symbol exceeds the C99
    //    §5.2.4.1 external identifier limit (31 chars). Default
    //    rendering is warn; `platform.strict_c99_identifiers: true`
    //    in deploy.yaml escalates to hard-error. The diagnostic
    //    carries the offending mangled id + excess-char count so
    //    authors see exactly what overflowed.
    #[serde(rename = "traceability/symbol-name-exceeds-c-identifier-limit")]
    TraceabilitySymbolNameExceedsCIdentifierLimit,

    // ── Traceability §5.O Atomic 1 — sourcemap `source_hash` drift
    //    against the §6.2.6 header. Spec lines 3321-3324 require
    //    byte-equality between the sourcemap JSON's `source_hash`
    //    field and the per-file `// source-hash:` header value;
    //    drift indicates the sourcemap was emitted from a stale
    //    snapshot or hand-edited.
    #[serde(rename = "traceability/sourcemap-source-hash-mismatch")]
    TraceabilitySourcemapSourceHashMismatch,

    // ── Traceability §5.O Atomic 1 — Rust SCE-MAP marker preservation
    //    guard (OQ-W16 (b)). Empirical: rustdoc may strip `#[doc]`
    //    attributes under specific profile / no_std combinations.
    //    Fires from `sce-codegen addr2sce` when the rustdoc JSON dump
    //    lacks the expected `SCE-MAP:` `#[doc]` line; the `// SCE-MAP:`
    //    line-comment fallback (dual-emit since Atomic 0c) covers the
    //    miss. Diagnostic signals the fallback was needed.
    #[serde(rename = "traceability/sce-map-attribute-stripped")]
    TraceabilitySceMapAttributeStripped,

    // ── Traceability §5.O Atomic 1 follow-up — codegen-internal
    //    invariant: every SCE-emitted file (one carrying a §6.2.6
    //    drift header) MUST contain at least one `SCE-MAP:` marker.
    //    Walker `forge::sourcemap::validate_emitted_files_have_markers`
    //    fires this from cmd_generate / cmd_generate_w3c success paths.
    //    Files without a drift header (external meta-generator output)
    //    are silently skipped per ARCHITECTURE.md "Traceability
    //    Ownership Boundary". Author repair is empty — the fix lives
    //    in the template that lost its `sce_map_marker` macro call.
    #[serde(rename = "traceability/meta-generated-source-line-marker-missing")]
    TraceabilityMetaGeneratedSourceLineMarkerMissing,

    // ── Round F-α (watching-zenoh RFC §5.2) — MCU driver/class
    //    boundary on the C11 backend. The two codes split along the
    //    Q-Round-F-D2 + D3 locks:
    //    `mcu/driver-header-not-found` fires at compile-model time
    //    when `<sce:driver href="..."/>` cannot be resolved against
    //    `deploy.yaml`'s `platform.driver_root` (or the SCXML file's
    //    parent directory as fallback). Cross-TU signature checking
    //    is delegated to the C compiler (Q-Round-F-D2); SCE only
    //    confirms the file exists.
    //    `mcu/section-attribute-on-non-mcu-target` fires at codegen
    //    entry when `platform.c11_section_attribute` is set but the
    //    target backend is not C11 — mirrors the Q-Call-7 non-MCU
    //    reject pattern (Q-Round-F-D3) so the section directive does
    //    not silently disappear on non-MCU compiles.
    #[serde(rename = "mcu/driver-header-not-found")]
    McuDriverHeaderNotFound,
    #[serde(rename = "mcu/section-attribute-on-non-mcu-target")]
    McuSectionAttributeOnNonMcuTarget,
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
        XmlFileNotFound,
        XmlWrongRootElement,
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
        ValidationBytesMaxSizeViolation,
        // Algorithm (watching-zenoh RFC §5.A, Phase A3)
        AlgorithmLocalShadowsParam,
        AlgorithmLvalueUnsupported,
        AlgorithmReturnMissing,
        // Algorithm-over-BC dispatch (RFC §5.A line 311 + §5.L lines
        // 2611-2618 + 2642-2647, C7-lowering 2026-05-13)
        AlgorithmForeachSourceNotIterable,
        AlgorithmCallTargetUnknown,
        AlgorithmCallTargetMethodUnknown,
        AlgorithmBcMutationForbidden,
        AlgorithmForeachSourceBcWithBytesItemType,
        AlgorithmCallArgCountMismatch,
        // SCXML semantic (RFC §W5)
        ScxmlTopLevelScriptUnloaded,
        ScxmlOnSampleInvalidParent,
        ScxmlOnSampleLinkDuplicateInState,
        ScxmlOnSampleEventNameConflict,
        ScxmlOnSampleLinkNotDeclared,
        ScxmlOnSampleLinkWrongKind,
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
        // Codegen matrix shells (watching-zenoh RFC §5.J.4 / §5.J.5)
        CodegenMcuClassKindOnNonMcuLanguage,
        CodegenGenericKindBackendEmitMissing,
        // Codegen Rust no_std variant rejection (watching-zenoh RFC §5.J.2,
        // C3 Atomic B-β + B-γ2c)
        CodegenNoStdScriptNotSupported,
        CodegenNoStdHttpNotSupported,
        CodegenNoStdFsLoadNotSupported,
        CodegenNoStdInvokeNotSupported,
        // Algorithm §5.F const-fold (watching-zenoh RFC §5.F, Phase A4-γ)
        AlgorithmConstNotFoldable,
        AlgorithmConstFoldBudgetExceeded,
        AlgorithmConstYieldTypeMismatch,
        // Codec §5.B variant primitive (watching-zenoh RFC §5.B, B1-β)
        CodecVariantArmUnreachable,
        // RFC variant-default-uniformity Atomic α — duplicate default-arm marker
        CodecVariantDuplicateDefaultArm,
        // RFC variant-default-uniformity Atomic γ-1 — cross-doc MID mismatch
        CodecVariantDefaultArmMidMismatch,
        // RFC variant-default-uniformity Atomic γ-1 — inner codec missing wire-MID constant
        CodecVariantArmInnerMidUndeclared,
        // Codec §5.B present-if primitive (watching-zenoh RFC §5.B, B1-δ)
        CodecPresentIfRefsLaterField,
        // Codec §5.B repeat primitive (watching-zenoh RFC §5.B, B2)
        CodecRepeatCountRefsLaterField,
        // Algorithm §5.B test-vector primitive (watching-zenoh RFC §5.B, B2-test-vector)
        AlgorithmTestVectorUnsupportedKind,
        // Codec §5.B B3 TLV chain primitive (watching-zenoh RFC §5.B)
        CodecTlvChainDepthUnspecified,
        // Codec §5.B B3 DMA alignment primitive (watching-zenoh RFC §5.B)
        CodecDmaAlignmentUnsatisfiable,
        // Codec §5.B B5-γ parent-flags dependency (watching-zenoh RFC §5.B)
        CodecParentFlagMismatch,
        // Link §5.C byte-stream link endpoint (watching-zenoh RFC §5.C, B6-α/γ/η + B6-α')
        LinkFramerMissing,
        LinkLinkClassUnknown,
        LinkBackpressureUndeclared,
        LinkClassUnsupportedOnTarget,
        LinkPoolSlotSmallerThanFramerMax,
        // BufferPool §5.E DMA-aligned slot table (watching-zenoh RFC §5.E, B7-α/β)
        MemPoolSectionConflict,
        MemPoolTooLarge,
        MemInterPoolPaddingNotEmitted,
        // BufferPool §5.E C5 cache-maintenance validation + codegen self-checks (watching-zenoh RFC §5.E + §5.I)
        MemCacheLineAlignment,
        MemSlotSizeNotCacheLineMultiple,
        MemCachePolicyUnsupportedOnNoDcacheCore,
        PoolCacheMaintenanceMisplaced,
        PoolSpeculativePrefetchFlagMissing,
        PoolCachePreArmInvalidateMissingOnSpeculativeCore,
        // BufferPool §5.E Layer 1 ownership pull-through (watching-zenoh RFC §5.E, B7-ε)
        PoolSampleTypestateAttributesDisabled,
        // Sample API §5.E B7-η' Atomic A1 application-layer ownership (watching-zenoh RFC §5.E)
        PoolSampleTakeWithoutStagePool,
        // Sample API §5.E B7-η' Atomic A2 callback-path syntax (watching-zenoh RFC §5.E)
        PoolSampleCallbackSignatureNonBorrow,
        // Worker kind shared-state encapsulation (watching-zenoh RFC §5.D, C2-α)
        WorkerSharedMutableState,
        // Worker kind cross-resolution + inbox ordering (watching-zenoh RFC §5.D + §5.I, C2-β)
        WorkerLinkRxRefUnknown,
        WorkerInboxOrderingUnspecified,
        WorkerInboxOrderingRelaxedAcrossCores,
        // Worker kind scheduler-capacity forge-side anchor (watching-zenoh RFC §5.D, C2-γ)
        WorkerSchedulerUnsupported,
        // Worker kind SCXML-side outbox cross-resolution (watching-zenoh RFC §5.D, C2 follow-up Atomic B)
        WorkerOutboxRefUnknown,
        WorkerOutboxTargetWrongKind,
        WorkerOutboxTargetSuffixInvalid,
        // Fragment-reassembly buffer-pool variant parse-time structure validators (watching-zenoh RFC §5.M, C9-α)
        MemReassemblyPoolVariantMissingMaxFragments,
        MemReassemblyPoolVariantMissingTimeout,
        // Fragment-reassembly cross-doc validators (watching-zenoh RFC §5.M, C13-α-2 + C9-β co-land)
        MemReassemblySlotSizeBelowDeclaredMtu,
        ReassemblyMaxFragmentsInsufficientForMtu,
        ReassemblyExpectedFragmentationRateHigh,
        ReassemblyUntrustedLinkBinding,
        ReassemblyTrustClassMissingOnFragmentingLink,
        ReassemblyStageCopyWcetExceedsSlotBudget,
        // Fragment-reassembly codegen self-check (watching-zenoh RFC §5.M, C9-γ)
        ReassemblyPeerIdNotZidOnEstablishedSession,
        // Listener-link sibling-pair (watching-zenoh RFC §5.C + §5.M, C10-α)
        LinkListenerLinkNotPairedWithEstablishedSibling,
        MeshDeployReassemblyBindingOnUnpairedListener,
        // Multi-link concurrency contract (watching-zenoh RFC §5.N, C10-β)
        LinkConcurrentCountExceedsSchedulerSlots,
        LinkPerLinkBudgetExceedsTickPeriod,
        LinkInboundEventQueueUnsized,
        // Bounded-collection kind parse-time structure validators (watching-zenoh RFC §5.L, C6-α)
        CollectionOrderingSortedRequiresIndexBy,
        CollectionOverflowPolicyOldestWinsRequiresOrderingInsertion,
        // Bounded-collection kind cross-doc resolution (watching-zenoh RFC §5.L, C6-β)
        CollectionElementTypeNotAKind,
        CollectionIndexByFieldMissing,
        CollectionMultiWriterWithoutAtomics,
        // Bounded-collection kind deploy-time capacity resolution (watching-zenoh RFC §5.L, C6-γ1)
        CollectionCapacityUnresolved,
        // Timer kind diagnostics (watching-zenoh RFC §5.D, C1)
        TimerPeriodBelowTickRate,
        TimerSlotOverflow,
        // `<sce:extern>` whitelisted intrinsic registry (watching-zenoh RFC §5.I, Atomic A)
        ExternSymbolNotInWhitelist,
        ExternAbiMismatch,
        ExternSignatureMismatch,
        ExternOrderingUnspecified,
        // `<sce:extern>` target-plugin extension (watching-zenoh RFC §5.I, Atomic B)
        ExternTargetPluginSymbolConflict,
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
        MeshDeployStagePoolNotDeclared,
        MeshDeployStagePoolWrongKind,
        MeshDeployStagePoolTransportMismatch,
        MeshDeployScxmlInvokeTargetConflict,
        MeshDeployPartitionDuplicateName,
        MeshDeployPartitionMultiDevice,
        MeshDeployPartitionUnitDuplicate,
        MeshDeployPartitionMachineNotListed,
        MeshDeployPartitionEmpty,
        MeshDeployPartitionNameNotIdentifier,
        MeshDeployPartitionSynthInfixCollision,
        MeshDeployPartitionUncoveredUnit,
        MeshDeployPartitionPartialCoverageRequiresDefault,
        MeshDeployPartitionPoolMachine,
        MeshDeployPartitionTransportBindingUnsupported,
        MeshDeployScxmlInvokeCrossDeviceTransport,
        MeshDeploySomeipScxmlInvokeServiceIdOverflow,
        MeshDeploySomeipScxmlInvokeServiceIdPinOutOfRange,
        MeshDeploySomeipScxmlInvokeServiceIdPinCollision,
        MeshDeploySomeipLivenessServiceIdOverflow,
        MeshDeploySomeipLivenessServiceIdPinOutOfRange,
        MeshDeploySomeipLivenessServiceIdPinCollision,
        MeshDeploySomeipMachineLivenessServiceIdOverflow,
        MeshDeploySomeipMachineLivenessServiceIdPinOutOfRange,
        MeshDeploySomeipMachineLivenessServiceIdPinCollision,
        MeshDeployPartitionBarrierTimeoutInvalid,
        MeshPartitionParallelRootUndesignated,
        MeshPartitionParallelRootAmbiguous,
        MeshPartitionParallelRootNotInMachines,
        MeshPartitionParallelRootNonHost,
        MeshPartitionBarrierTimeoutWithoutRoot,
        MeshPartitionWire21CustomTcpUnimplemented,
        MeshDistributabilityR1SharedWrite,
        MeshDistributabilityR2CrossRegionTransition,
        MeshDeployPlatformClassOsMismatch,
        MeshDeploySchedulerCooperativeMissingStackBudget,
        // C2-γ scheduler-capacity axis (watching-zenoh RFC §5.K lines 2423/2428-9/2430-1)
        MeshDeploySchedulerCooperativeMissingSlotBudget,
        MeshDeploySchedulerCooperativeMissingKeepaliveJitterBudget,
        MeshDeploySchedulerIncompatibleWithWorkerCount,
        // C13-α-1 `links:` block schema + parse-time + cross-doc (RFC §5.K lines 2232-2540)
        MeshDeployLinkDriverUnknown,
        MeshDeployLinkMtuMissingOnFragmentingLink,
        MeshDeployLinkMtuBelowDriverFloor,
        MeshDeployLinkDriverClassMismatch,
        MeshDeployLinkExpectedP99ExceedsMtu,
        MeshDeployLinkBurstPpsMissingOnIsrDispatch,
        MeshDeployLinkNotDeclaredInDeploy,
        MeshDeployLinkNotDeclaredInForge,
        // C13-α-2 cross-doc RX pool burst invariants (RFC §5.K lines 2489-2500)
        MeshDeployLinkBurstAbsorptionInsufficient,
        MeshDeployLinkRxDispatchWorkerTickOnHighBurst,
        // C13-γ pool_defaults.stage_copy_policy (RFC §5.K lines 2350-2369 + 2504-2519)
        PoolStageCopyPolicyError,
        PoolStageCopyAcceptRejectedUnderForbid,
        MeshDeployStageCopyPolicyUnknown,
        // C13-β anti-flood + stateless_accept (RFC §5.K lines 2272-2349 + 2449-2473)
        MeshDeploySessionArmingQuotaMissing,
        MeshDeployAcceptRateConfigMissing,
        MeshDeploySessionArmingFieldsOnNonArmingLink,
        MeshDeployStatelessAcceptRequiredOnUntrustedSource,
        MeshDeployStatelessAcceptKeyRotationShorterThanLifetime,
        // C13 deferred-2: peer_table invariant + extern allowlist (RFC §5.K lines 2460-2462 + 2466-2469)
        MeshDeploySessionArmingQuotaVsPeerTableInvariantViolated,
        MeshDeployStatelessAcceptExternNotWhitelisted,
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
        // Forge generated-source drift detection (watching-zenoh RFC §6.2.6)
        ForgeSourceHashMismatch,
        // Traceability §5.O Atomic 0 — IR provenance pre-emit guard
        // (watching-zenoh RFC §5.O lines 3289-3290).
        TraceabilityScxmlLineRangeMissing,
        // Traceability §5.O Atomic 1 — symbol mangling + sourcemap
        // contract (watching-zenoh RFC §5.O lines 3055-3057, 3219-3243,
        // 3321-3324, OQ-W16 a/b locks).
        TraceabilityStateIdCollision,
        TraceabilitySymbolNameExceedsCIdentifierLimit,
        TraceabilitySourcemapSourceHashMismatch,
        TraceabilitySceMapAttributeStripped,
        // Traceability §5.O Atomic 1 follow-up — ownership-boundary
        // walker fires this when a drift-headered file is missing
        // its SCE-MAP marker. ARCHITECTURE.md "Traceability Ownership
        // Boundary" defines the scope.
        TraceabilityMetaGeneratedSourceLineMarkerMissing,
        // Round F-α (watching-zenoh RFC §5.2) — MCU driver/class
        // boundary on the C11 backend. `mcu/driver-header-not-found`
        // covers `<sce:driver href="..."/>` resolution failure;
        // `mcu/section-attribute-on-non-mcu-target` covers Q-Round-F-D3
        // non-MCU backend reject of `platform.c11_section_attribute`.
        McuDriverHeaderNotFound,
        McuSectionAttributeOnNonMcuTarget,
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

            // ── SCXML §5.8 top-level script (RFC §W5) ─────────────
            ScxmlTopLevelScriptUnloaded => Some("W3C SCXML §5.8"),

            // ── Algorithm kind (watching-zenoh RFC §5.A) ──────────
            AlgorithmLocalShadowsParam
            | AlgorithmLvalueUnsupported
            | AlgorithmReturnMissing => Some("watching-zenoh RFC §5.A"),

            // ── Algorithm-over-BC dispatch (C7-lowering: RFC §5.A
            //    line 311 + §5.L lines 2611-2618 + 2642-2647). Six
            //    codes share the cross-section anchor that names both
            //    the algorithm `Call`/`Foreach` IR shape and the BC
            //    public-method roster. ─────────────────────────────
            AlgorithmForeachSourceNotIterable
            | AlgorithmCallTargetUnknown
            | AlgorithmCallTargetMethodUnknown
            | AlgorithmBcMutationForbidden
            | AlgorithmForeachSourceBcWithBytesItemType
            | AlgorithmCallArgCountMismatch => Some("watching-zenoh RFC §5.A + §5.L"),

            // ── Algorithm §5.F build-time const-fold ─────────────
            AlgorithmConstNotFoldable
            | AlgorithmConstFoldBudgetExceeded
            | AlgorithmConstYieldTypeMismatch => Some("watching-zenoh RFC §5.F"),

            // ── Codec §5.B variant + present-if + repeat + tlv-chain + dma-align primitives (B1-β/δ + B2 + B3) ─
            CodecVariantArmUnreachable
            | CodecVariantDuplicateDefaultArm
            | CodecVariantDefaultArmMidMismatch
            | CodecVariantArmInnerMidUndeclared
            | CodecPresentIfRefsLaterField
            | CodecRepeatCountRefsLaterField
            | AlgorithmTestVectorUnsupportedKind
            | CodecTlvChainDepthUnspecified
            | CodecDmaAlignmentUnsatisfiable
            | CodecParentFlagMismatch => Some("watching-zenoh RFC §5.B"),

            // ── Link §5.C byte-stream link endpoint (B6) ─────────
            LinkFramerMissing
            | LinkLinkClassUnknown
            | LinkBackpressureUndeclared
            | LinkClassUnsupportedOnTarget
            | LinkPoolSlotSmallerThanFramerMax => Some("watching-zenoh RFC §5.C"),

            // ── BufferPool §5.E DMA-aligned slot table + Layer 1
            //    ownership pull-through (B7-α/β/ε) + B7-η' Sample API
            //    application-layer ownership ────────────────────────
            MemPoolSectionConflict
            | MemPoolTooLarge
            | MemInterPoolPaddingNotEmitted
            | MemCacheLineAlignment
            | MemSlotSizeNotCacheLineMultiple
            | MemCachePolicyUnsupportedOnNoDcacheCore
            | PoolCacheMaintenanceMisplaced
            | PoolSpeculativePrefetchFlagMissing
            | PoolCachePreArmInvalidateMissingOnSpeculativeCore
            | PoolSampleTypestateAttributesDisabled
            | PoolSampleTakeWithoutStagePool
            | PoolSampleCallbackSignatureNonBorrow
            | MeshDeployStagePoolNotDeclared
            | MeshDeployStagePoolWrongKind
            | MeshDeployStagePoolTransportMismatch
            | ScxmlOnSampleInvalidParent
            | ScxmlOnSampleLinkDuplicateInState
            | ScxmlOnSampleEventNameConflict
            | ScxmlOnSampleLinkNotDeclared
            | ScxmlOnSampleLinkWrongKind => Some("watching-zenoh RFC §5.E"),

            // ── §5.I `<sce:extern>` whitelisted intrinsic registry
            //    (Atomic A baseline + Atomic B target-plugin extension) ──
            ExternSymbolNotInWhitelist
            | ExternAbiMismatch
            | ExternSignatureMismatch
            | ExternOrderingUnspecified
            | ExternTargetPluginSymbolConflict => Some("watching-zenoh RFC §5.I"),

            // ── §5.D Worker kind encapsulation (C2-α) ───────────────
            WorkerSharedMutableState => Some("watching-zenoh RFC §5.D"),

            // ── Worker cross-resolution (RFC §5.D, C2-β) + SPSC inbox
            //    ordering (RFC §5.I lines 1752-1758, C2-β) ──
            //    Cross-ref codes carry §5.D spec anchor (worker schema
            //    is §5.D's domain). Ordering codes carry §5.I anchor
            //    (the SPSC/MPSC ordering contract is §5.I's domain).
            WorkerLinkRxRefUnknown => Some("watching-zenoh RFC §5.D"),
            WorkerInboxOrderingUnspecified
            | WorkerInboxOrderingRelaxedAcrossCores => Some("watching-zenoh RFC §5.I"),

            // ── Worker scheduler-capacity axis (RFC §5.D + §5.K, C2-γ) ──
            //    Forge-side anchor (line 912) lives in §5.D worker
            //    domain; the three deploy-side anchors (line 2423 /
            //    2428-9 / 2430-1) live in §5.K deploy.yaml domain.
            WorkerSchedulerUnsupported
            | TimerPeriodBelowTickRate
            | TimerSlotOverflow => Some("watching-zenoh RFC §5.D"),

            // ── Worker SCXML-side outbox cross-resolution (RFC §5.D,
            //    C2 follow-up Atomic B). All three axes live in §5.D
            //    worker domain (the worker schema's `<sce:outbox>` is
            //    §5.D's; the recipient codegen contract is §5.D's
            //    inbox lowering).
            WorkerOutboxRefUnknown
            | WorkerOutboxTargetWrongKind
            | WorkerOutboxTargetSuffixInvalid => Some("watching-zenoh RFC §5.D"),

            // ── §5.L Bounded-collection kind parse-time structure
            //    validators (watching-zenoh RFC §5.L lines 2540-2655,
            //    C6-α + C6-β). C6-α codes are XML-structure-only — the
            //    sorted ordering requires an explicit `<sce:index-by>`
            //    field (spec line 2559), and `oldest-wins` overflow
            //    requires `insertion` ordering (spec line 2655). C6-β
            //    codes are cross-doc — element-type kind resolution
            //    (lines 2566-2567), index-by field enumeration
            //    (line 2615), multi-writer atomic-import surface
            //    (lines 2560-2562). All five sit on §5.L.
            CollectionOrderingSortedRequiresIndexBy
            | CollectionOverflowPolicyOldestWinsRequiresOrderingInsertion
            | CollectionElementTypeNotAKind
            | CollectionIndexByFieldMissing
            | CollectionMultiWriterWithoutAtomics
            | CollectionCapacityUnresolved => {
                Some("watching-zenoh RFC §5.L")
            }

            // ── §5.M Fragment-reassembly variant parse-time structure
            //    validators (watching-zenoh RFC §5.M lines 2944-2945,
            //    C9-α). Both codes fire when
            //    `<sce:variant>reassembly</sce:variant>` is declared
            //    without one of its required sibling elements
            //    (max-fragments-per-message at spec line 2688,
            //    reassembly-timeout-ms at spec line 2689). Cross-doc /
            //    cross-link / codegen-side reassembly diagnostics defer
            //    to C9-β/γ + C10/C11 per RFC stub §1.7 phase column.
            MemReassemblyPoolVariantMissingMaxFragments
            | MemReassemblyPoolVariantMissingTimeout
            // C13-α-2 + C9-β cross-doc validators (RFC §5.M lines 2946-2995).
            // The 6 cross-doc reassembly codes share §5.M spec anchor.
            | MemReassemblySlotSizeBelowDeclaredMtu
            | ReassemblyMaxFragmentsInsufficientForMtu
            | ReassemblyExpectedFragmentationRateHigh
            | ReassemblyUntrustedLinkBinding
            | ReassemblyTrustClassMissingOnFragmentingLink
            | ReassemblyStageCopyWcetExceedsSlotBudget
            // C9-γ codegen self-check (RFC §5.M lines 2976-2981).
            | ReassemblyPeerIdNotZidOnEstablishedSession
            // C10-α reassembly side of the listener-pair (RFC §5.M
            // lines 2982-2994). Shares §5.M anchor with the C9-γ
            // codegen self-check + the 6 C9-β / C13-α-2 cross-doc
            // reassembly codes.
            | MeshDeployReassemblyBindingOnUnpairedListener => {
                Some("watching-zenoh RFC §5.M")
            }
            // C10-α listener-pair codegen self-check (RFC §5.C lines
            // 849-856). The §5.C anchor matches the existing B6-α/β/γ
            // link family.
            LinkListenerLinkNotPairedWithEstablishedSibling => {
                Some("watching-zenoh RFC §5.C")
            }

            // C10-β multi-link concurrency contract (RFC §5.N lines
            // 3031-3062). All three codes share the §5.N anchor —
            // distinct §5 section from §5.C (B6 link kind) and §5.M
            // (reassembly variant) so the spec table-of-contents stays
            // readable.
            LinkConcurrentCountExceedsSchedulerSlots
            | LinkPerLinkBudgetExceedsTickPeriod
            | LinkInboundEventQueueUnsized => Some("watching-zenoh RFC §5.N"),

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

            // ── Mesh §16.4 SOME/IP region-liveness hybrid allocator (RFC F.X-3) ──
            MeshDeploySomeipLivenessServiceIdOverflow
            | MeshDeploySomeipLivenessServiceIdPinOutOfRange
            | MeshDeploySomeipLivenessServiceIdPinCollision => Some("SCE Mesh §16.4"),
            MeshDeploySomeipMachineLivenessServiceIdOverflow
            | MeshDeploySomeipMachineLivenessServiceIdPinOutOfRange
            | MeshDeploySomeipMachineLivenessServiceIdPinCollision => Some("SCE Mesh §16.7"),

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
            | MeshDeployPartitionNameNotIdentifier
            | MeshDeployPartitionSynthInfixCollision
            | MeshDeployPartitionUncoveredUnit
            | MeshDeployPartitionPartialCoverageRequiresDefault
            | MeshDeployPartitionPoolMachine
            | MeshDeployPartitionTransportBindingUnsupported
            | MeshDeployPartitionBarrierTimeoutInvalid
            | MeshDeployPlatformClassOsMismatch => Some("SCE Mesh §14"),

            // ── C2-γ scheduler-capacity deploy-side anchors
            //    (watching-zenoh RFC §5.K lines 2423 / 2426 / 2428-9 / 2430-1).
            //    The stack-budget variant was renamed at C2-γ landing
            //    from the SCE-Mesh-prefix wire to the watching-zenoh-prefix
            //    wire (`deploy/worker-stack-budget-missing`); the spec
            //    anchor follows the rename. The three sibling variants
            //    follow the same anchor. ──
            MeshDeploySchedulerCooperativeMissingStackBudget
            | MeshDeploySchedulerCooperativeMissingSlotBudget
            | MeshDeploySchedulerCooperativeMissingKeepaliveJitterBudget
            | MeshDeploySchedulerIncompatibleWithWorkerCount
            // C13-α-1 `links:` block (RFC §5.K lines 2232-2540) +
            // C13-α-2 cross-doc RX-pool burst invariants (RFC §5.K
            // lines 2489-2500).
            | MeshDeployLinkDriverUnknown
            | MeshDeployLinkMtuMissingOnFragmentingLink
            | MeshDeployLinkMtuBelowDriverFloor
            | MeshDeployLinkExpectedP99ExceedsMtu
            | MeshDeployLinkBurstPpsMissingOnIsrDispatch
            | MeshDeployLinkNotDeclaredInDeploy
            | MeshDeployLinkNotDeclaredInForge
            | MeshDeployLinkBurstAbsorptionInsufficient
            | MeshDeployLinkRxDispatchWorkerTickOnHighBurst
            | MeshDeployLinkDriverClassMismatch
            // C13-γ pool_defaults.stage_copy_policy (RFC §5.K lines
            // 2350-2369 + 2504-2519). Three codes share §5.K anchor.
            | PoolStageCopyPolicyError
            | PoolStageCopyAcceptRejectedUnderForbid
            | MeshDeployStageCopyPolicyUnknown
            // C13-β anti-flood + stateless_accept (RFC §5.K lines
            // 2272-2349 + 2449-2473). Five codes share §5.K anchor.
            | MeshDeploySessionArmingQuotaMissing
            | MeshDeployAcceptRateConfigMissing
            | MeshDeploySessionArmingFieldsOnNonArmingLink
            | MeshDeployStatelessAcceptRequiredOnUntrustedSource
            | MeshDeployStatelessAcceptKeyRotationShorterThanLifetime
            // C13 deferred-2: peer_table invariant (RFC §5.K line
            // 2460-2462) + extern allowlist (RFC §5.K line 2466-2469).
            | MeshDeploySessionArmingQuotaVsPeerTableInvariantViolated
            | MeshDeployStatelessAcceptExternNotWhitelisted
                => Some("watching-zenoh RFC §5.K"),

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

            // ── §5.J.2 Rust no_std variant (C3 Atomic B-β) ───────
            //    Both rejections anchor on the watching-zenoh RFC
            //    section that defines the no_std variant of the Rust
            //    backend; the script-engine incompatibility is line
            //    1989 ("zero alloc dependency") and the http-send
            //    incompatibility is line 1983 ("`sce-rust-runtime`
            //    grows a `no_std` feature gate").
            CodegenNoStdScriptNotSupported
            | CodegenNoStdHttpNotSupported
            | CodegenNoStdFsLoadNotSupported
            | CodegenNoStdInvokeNotSupported => Some("watching-zenoh RFC §5.J.2"),

            // ── §6.2.6 generated-source drift detection (B9, 2026-05-14) ──
            //    The single mismatch code covers both axes (source-hash
            //    + template-hash); the spec section defines the header
            //    contract + `sce-build verify` recompute pipeline.
            ForgeSourceHashMismatch => Some("watching-zenoh RFC §6.2.6"),

            // ── §5.O Atomic 0 — traceability IR provenance pre-emit
            //    guard. Spec lines 3289-3290 verbatim: "Codegen failure
            //    ... surfaced via `traceability/scxml-line-range-
            //    missing` (codegen-internal)". Anchors the diagnostic
            //    against the per-IR-node `(file_id, line, column)`
            //    invariant the Atomic 0 markers + Atomic 1 sourcemap
            //    both consume.
            TraceabilityScxmlLineRangeMissing => Some("watching-zenoh RFC §5.O"),

            // ── §5.O Atomic 1 — symbol mangling + sourcemap contract.
            //    All 4 anchor at watching-zenoh RFC §5.O (the same
            //    section heading) per Q-§5.O-9: spec lines 3055-3057
            //    (mangling pattern), 3219-3243 (sourcemap JSON shape),
            //    3321-3324 (source_hash byte-equality), OQ-W16 a/b
            //    (escape encoding + `#[doc]` preservation).
            TraceabilityStateIdCollision
            | TraceabilitySymbolNameExceedsCIdentifierLimit
            | TraceabilitySourcemapSourceHashMismatch
            | TraceabilitySceMapAttributeStripped
            | TraceabilityMetaGeneratedSourceLineMarkerMissing => Some("watching-zenoh RFC §5.O"),

            // ── Round F-α (watching-zenoh RFC §5.2) — MCU driver/class
            //    boundary on the C11 backend. Both anchor at §5.2 per
            //    Q-Round-F-D2/D3/D5/D6 locks (driver header reference
            //    + non-MCU section attribute reject).
            McuDriverHeaderNotFound | McuSectionAttributeOnNonMcuTarget => {
                Some("watching-zenoh RFC §5.2")
            }

            // ── No authoritative citation ────────────────────────
            //
            // Anchors are deliberately narrow: a code lands here when
            // the rule is operational (I/O failures, template render
            // crashes, CLI argument parsing) or tied to policy that
            // does not have a pinned section yet. Leaving `None` keeps
            // the wire format honest; the message still carries the
            // repair guidance.
            XmlParse
            | XmlFileNotFound
            | XmlWrongRootElement
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
            | ValidationBytesMaxSizeViolation
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
            | CodegenMcuClassKindOnNonMcuLanguage
            | CodegenGenericKindBackendEmitMissing
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
            XmlFileNotFound => "xml/file-not-found",
            XmlWrongRootElement => "xml/wrong-root-element",
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
            ValidationBytesMaxSizeViolation => "validation/bytes-max-size-violation",
            AlgorithmLocalShadowsParam => "algorithm/local-shadows-param",
            AlgorithmLvalueUnsupported => "algorithm/lvalue-unsupported",
            AlgorithmReturnMissing => "algorithm/return-missing",
            AlgorithmForeachSourceNotIterable => "algorithm/foreach-source-not-iterable",
            AlgorithmCallTargetUnknown => "algorithm/call-target-unknown",
            AlgorithmCallTargetMethodUnknown => "algorithm/call-target-method-unknown",
            AlgorithmBcMutationForbidden => "algorithm/bc-mutation-forbidden",
            AlgorithmForeachSourceBcWithBytesItemType => {
                "algorithm/foreach-source-bc-with-bytes-item-type"
            }
            AlgorithmCallArgCountMismatch => "algorithm/call-arg-count-mismatch",
            ScxmlTopLevelScriptUnloaded => "scxml/top-level-script-unloaded",
            ScxmlOnSampleInvalidParent => "scxml/on-sample-invalid-parent",
            ScxmlOnSampleLinkDuplicateInState => "scxml/on-sample-link-duplicate-in-state",
            ScxmlOnSampleEventNameConflict => "scxml/on-sample-event-name-conflict",
            ScxmlOnSampleLinkNotDeclared => "scxml/on-sample-link-not-declared",
            ScxmlOnSampleLinkWrongKind => "scxml/on-sample-link-wrong-kind",
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
            CodegenMcuClassKindOnNonMcuLanguage => "codegen/mcu-class-kind-on-non-mcu-language",
            CodegenGenericKindBackendEmitMissing => "codegen/generic-kind-backend-emit-missing",
            CodegenNoStdScriptNotSupported => "codegen/no-std-script-not-supported",
            CodegenNoStdHttpNotSupported => "codegen/no-std-http-not-supported",
            CodegenNoStdFsLoadNotSupported => "codegen/no-std-fs-load-not-supported",
            CodegenNoStdInvokeNotSupported => "codegen/no-std-invoke-not-supported",
            AlgorithmConstNotFoldable => "algorithm/const-not-foldable",
            AlgorithmConstFoldBudgetExceeded => "algorithm/const-fold-budget-exceeded",
            AlgorithmConstYieldTypeMismatch => "algorithm/const-yield-type-mismatch",
            CodecVariantArmUnreachable => "codec/variant-arm-unreachable",
            CodecVariantDuplicateDefaultArm => "codec/variant-duplicate-default-arm",
            CodecVariantDefaultArmMidMismatch => "codec/variant-default-arm-mid-mismatch",
            CodecVariantArmInnerMidUndeclared => "codec/variant-arm-inner-mid-undeclared",
            CodecPresentIfRefsLaterField => "codec/present-if-refs-later-field",
            CodecRepeatCountRefsLaterField => "codec/repeat-count-refs-later-field",
            AlgorithmTestVectorUnsupportedKind => "algorithm/test-vector-unsupported-kind",
            CodecTlvChainDepthUnspecified => "codec/tlv-chain-depth-unspecified",
            CodecDmaAlignmentUnsatisfiable => "codec/dma-alignment-unsatisfiable",
            CodecParentFlagMismatch => "codec/parent-flag-mismatch",
            LinkFramerMissing => "link/framer-missing",
            LinkLinkClassUnknown => "link/link-class-unknown",
            LinkBackpressureUndeclared => "link/backpressure-undeclared",
            LinkClassUnsupportedOnTarget => "link/class-unsupported-on-target",
            LinkPoolSlotSmallerThanFramerMax => "link/pool-slot-smaller-than-framer-max",
            MemPoolSectionConflict => "mem/pool-section-conflict",
            MemPoolTooLarge => "mem/pool-too-large",
            MemInterPoolPaddingNotEmitted => "mem/inter-pool-padding-not-emitted",
            MemCacheLineAlignment => "mem/cache-line-alignment",
            MemSlotSizeNotCacheLineMultiple => "mem/slot-size-not-cache-line-multiple",
            MemCachePolicyUnsupportedOnNoDcacheCore => "mem/cache-policy-unsupported-on-no-dcache-core",
            PoolCacheMaintenanceMisplaced => "pool/cache-maintenance-misplaced",
            PoolSpeculativePrefetchFlagMissing => "pool/speculative-prefetch-flag-missing",
            PoolCachePreArmInvalidateMissingOnSpeculativeCore => "pool/cache-pre-arm-invalidate-missing-on-speculative-core",
            PoolSampleTypestateAttributesDisabled => "pool/sample-typestate-attributes-disabled",
            PoolSampleTakeWithoutStagePool => "pool/sample-take-without-stage-pool",
            PoolSampleCallbackSignatureNonBorrow => "pool/sample-callback-signature-non-borrow",
            WorkerSharedMutableState => "worker/shared-mutable-state",
            WorkerLinkRxRefUnknown => "worker/link-rx-ref-unknown",
            WorkerInboxOrderingUnspecified => "worker/inbox-ordering-unspecified",
            WorkerInboxOrderingRelaxedAcrossCores => "worker/inbox-ordering-relaxed-across-cores",
            WorkerSchedulerUnsupported => "worker/scheduler-unsupported",
            WorkerOutboxRefUnknown => "worker/outbox-ref-unknown",
            WorkerOutboxTargetWrongKind => "worker/outbox-target-wrong-kind",
            WorkerOutboxTargetSuffixInvalid => "worker/outbox-target-suffix-invalid",
            MemReassemblyPoolVariantMissingMaxFragments => "mem/reassembly-pool-variant-missing-max-fragments",
            MemReassemblyPoolVariantMissingTimeout => "mem/reassembly-pool-variant-missing-timeout",
            MemReassemblySlotSizeBelowDeclaredMtu => "mem/reassembly-slot-size-below-declared-mtu",
            ReassemblyMaxFragmentsInsufficientForMtu => "reassembly/max-fragments-insufficient-for-mtu",
            ReassemblyExpectedFragmentationRateHigh => "reassembly/expected-fragmentation-rate-high",
            ReassemblyUntrustedLinkBinding => "reassembly/untrusted-link-binding",
            ReassemblyTrustClassMissingOnFragmentingLink => "reassembly/trust-class-missing-on-fragmenting-link",
            ReassemblyStageCopyWcetExceedsSlotBudget => "reassembly/stage-copy-wcet-exceeds-slot-budget",
            ReassemblyPeerIdNotZidOnEstablishedSession => "reassembly/peer-id-not-zid-on-established-session",
            LinkListenerLinkNotPairedWithEstablishedSibling => "link/listener-link-not-paired-with-established-sibling",
            MeshDeployReassemblyBindingOnUnpairedListener => "reassembly/binding-on-unpaired-listener",
            LinkConcurrentCountExceedsSchedulerSlots => "link/concurrent-count-exceeds-scheduler-slots",
            LinkPerLinkBudgetExceedsTickPeriod => "link/per-link-budget-exceeds-tick-period",
            LinkInboundEventQueueUnsized => "link/inbound-event-queue-unsized",
            CollectionOrderingSortedRequiresIndexBy => "collection/ordering-sorted-requires-index-by",
            CollectionOverflowPolicyOldestWinsRequiresOrderingInsertion => "collection/overflow-policy-oldest-wins-requires-ordering-insertion",
            CollectionElementTypeNotAKind => "collection/element-type-not-a-kind",
            CollectionIndexByFieldMissing => "collection/index-by-field-missing",
            CollectionMultiWriterWithoutAtomics => "collection/multi-writer-without-atomics",
            CollectionCapacityUnresolved => "collection/capacity-unresolved",
            TimerPeriodBelowTickRate => "timer/period-below-tick-rate",
            TimerSlotOverflow => "timer/slot-overflow",
            ExternSymbolNotInWhitelist => "extern/symbol-not-in-whitelist",
            ExternAbiMismatch => "extern/abi-mismatch",
            ExternSignatureMismatch => "extern/signature-mismatch",
            ExternOrderingUnspecified => "extern/ordering-unspecified",
            ExternTargetPluginSymbolConflict => "extern/target-plugin-symbol-conflict",
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
            MeshDeployStagePoolNotDeclared => "mesh/deploy-stage-pool-not-declared",
            MeshDeployStagePoolWrongKind => "mesh/deploy-stage-pool-wrong-kind",
            MeshDeployStagePoolTransportMismatch => "mesh/deploy-stage-pool-transport-mismatch",
            MeshDeployScxmlInvokeTargetConflict => "mesh/deploy-scxml-invoke-target-conflict",
            MeshDeployPartitionDuplicateName => "mesh/deploy-partition-duplicate-name",
            MeshDeployPartitionMultiDevice => "mesh/deploy-partition-multi-device",
            MeshDeployPartitionUnitDuplicate => "mesh/deploy-partition-unit-duplicate",
            MeshDeployPartitionMachineNotListed => "mesh/deploy-partition-machine-not-listed",
            MeshDeployPartitionEmpty => "mesh/deploy-partition-empty",
            MeshDeployPartitionNameNotIdentifier => "mesh/deploy-partition-name-not-identifier",
            MeshDeployPartitionSynthInfixCollision => "mesh/deploy-partition-synth-infix-collision",
            MeshDeployPartitionUncoveredUnit => "mesh/deploy-partition-uncovered-unit",
            MeshDeployPartitionPartialCoverageRequiresDefault => "mesh/deploy-partition-partial-coverage-requires-default",
            MeshDeployPartitionPoolMachine => "mesh/deploy-partition-pool-machine",
            MeshDeployPartitionTransportBindingUnsupported => "mesh/deploy-partition-transport-binding-unsupported",
            MeshDeployScxmlInvokeCrossDeviceTransport => "mesh/deploy-scxml-invoke-cross-device-transport",
            MeshDeploySomeipScxmlInvokeServiceIdOverflow => "mesh/deploy-someip-scxml-invoke-service-id-overflow",
            MeshDeploySomeipScxmlInvokeServiceIdPinOutOfRange => "mesh/deploy-someip-scxml-invoke-service-id-pin-out-of-range",
            MeshDeploySomeipScxmlInvokeServiceIdPinCollision => "mesh/deploy-someip-scxml-invoke-service-id-pin-collision",
            MeshDeploySomeipLivenessServiceIdOverflow => "mesh/deploy-someip-liveness-service-id-overflow",
            MeshDeploySomeipLivenessServiceIdPinOutOfRange => "mesh/deploy-someip-liveness-service-id-pin-out-of-range",
            MeshDeploySomeipLivenessServiceIdPinCollision => "mesh/deploy-someip-liveness-service-id-pin-collision",
            MeshDeploySomeipMachineLivenessServiceIdOverflow => "mesh/deploy-someip-machine-liveness-service-id-overflow",
            MeshDeploySomeipMachineLivenessServiceIdPinOutOfRange => "mesh/deploy-someip-machine-liveness-service-id-pin-out-of-range",
            MeshDeploySomeipMachineLivenessServiceIdPinCollision => "mesh/deploy-someip-machine-liveness-service-id-pin-collision",
            MeshDeployPartitionBarrierTimeoutInvalid => "mesh/deploy-partition-barrier-timeout-invalid",
            MeshPartitionParallelRootUndesignated => "mesh/partition-parallel-root-undesignated",
            MeshPartitionParallelRootAmbiguous => "mesh/partition-parallel-root-ambiguous",
            MeshPartitionParallelRootNotInMachines => "mesh/partition-parallel-root-not-in-machines",
            MeshPartitionParallelRootNonHost => "mesh/partition-parallel-root-non-host",
            MeshPartitionBarrierTimeoutWithoutRoot => "mesh/partition-barrier-timeout-without-root",
            MeshPartitionWire21CustomTcpUnimplemented => "mesh/partition-wire21-custom-tcp-unimplemented",
            MeshDistributabilityR1SharedWrite => "mesh/distributability-r1-shared-write",
            MeshDistributabilityR2CrossRegionTransition => "mesh/distributability-r2-cross-region-transition",
            MeshDeployPlatformClassOsMismatch => "mesh/deploy-platform-class-os-mismatch",
            MeshDeploySchedulerCooperativeMissingStackBudget => "deploy/worker-stack-budget-missing",
            MeshDeploySchedulerCooperativeMissingSlotBudget => "deploy/worker-slot-budget-missing",
            MeshDeploySchedulerCooperativeMissingKeepaliveJitterBudget => "deploy/keepalive-jitter-budget-missing",
            MeshDeploySchedulerIncompatibleWithWorkerCount => "deploy/scheduler-incompatible-with-worker-count",
            MeshDeployLinkDriverUnknown => "deploy/link-driver-unknown",
            MeshDeployLinkMtuMissingOnFragmentingLink => "deploy/link-mtu-missing-on-fragmenting-link",
            MeshDeployLinkMtuBelowDriverFloor => "deploy/link-mtu-below-driver-floor",
            MeshDeployLinkDriverClassMismatch => "deploy/link-driver-class-mismatch",
            MeshDeployLinkExpectedP99ExceedsMtu => "deploy/link-expected-p99-exceeds-mtu",
            MeshDeployLinkBurstPpsMissingOnIsrDispatch => "deploy/link-burst-pps-missing-on-isr-dispatch",
            MeshDeployLinkNotDeclaredInDeploy => "deploy/link-not-declared-in-deploy",
            MeshDeployLinkNotDeclaredInForge => "deploy/link-not-declared-in-forge",
            MeshDeployLinkBurstAbsorptionInsufficient => "deploy/link-burst-absorption-insufficient",
            MeshDeployLinkRxDispatchWorkerTickOnHighBurst => "deploy/link-rx-dispatch-worker-tick-on-high-burst",
            PoolStageCopyPolicyError => "pool/stage-copy-policy-error",
            PoolStageCopyAcceptRejectedUnderForbid => "pool/stage-copy-accept-rejected-under-forbid",
            MeshDeployStageCopyPolicyUnknown => "deploy/stage-copy-policy-unknown",
            MeshDeploySessionArmingQuotaMissing => "deploy/session-arming-quota-missing",
            MeshDeployAcceptRateConfigMissing => "deploy/accept-rate-config-missing",
            MeshDeploySessionArmingFieldsOnNonArmingLink => "deploy/session-arming-fields-on-non-arming-link",
            MeshDeployStatelessAcceptRequiredOnUntrustedSource => "deploy/stateless-accept-required-on-untrusted-source",
            MeshDeployStatelessAcceptKeyRotationShorterThanLifetime => "deploy/stateless-accept-key-rotation-shorter-than-lifetime",
            MeshDeploySessionArmingQuotaVsPeerTableInvariantViolated => "deploy/session-arming-quota-vs-peer-table-invariant-violated",
            MeshDeployStatelessAcceptExternNotWhitelisted => "deploy/stateless-accept-extern-not-whitelisted",
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
            ForgeSourceHashMismatch => "forge/source-hash-mismatch",
            TraceabilityScxmlLineRangeMissing => "traceability/scxml-line-range-missing",
            TraceabilityStateIdCollision => "traceability/state-id-collision",
            TraceabilitySymbolNameExceedsCIdentifierLimit => "traceability/symbol-name-exceeds-c-identifier-limit",
            TraceabilitySourcemapSourceHashMismatch => "traceability/sourcemap-source-hash-mismatch",
            TraceabilitySceMapAttributeStripped => "traceability/sce-map-attribute-stripped",
            TraceabilityMetaGeneratedSourceLineMarkerMissing => {
                "traceability/meta-generated-source-line-marker-missing"
            }
            McuDriverHeaderNotFound => "mcu/driver-header-not-found",
            McuSectionAttributeOnNonMcuTarget => "mcu/section-attribute-on-non-mcu-target",
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
        ForgeError::Scxml(e) => scxml_semantic_fields(e),
        // Delegate to `MeshError`'s `SingleDiagnostic` impl
        // (mesh/error.rs:3219) — it already covers every variant
        // (Deploy / External / Topology / Codegen / Io) with the
        // correct DiagnosticCode + stage + payload. Routing through
        // `ForgeError::Mesh` from `compile_scxml_with_imports`
        // preserves the cross-doc validator wire shape.
        ForgeError::Mesh(e) => {
            <crate::mesh::error::MeshError as SingleDiagnostic>::diagnostic_payload(e)
        }
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
        // Top-level parser-entry errors (RFC §W4 α-strict). `actual`
        // carries the offending path / found tag-name so repair tools
        // can act without parsing the message text; `key_fragments`
        // tie the FNV-1a id to the same payload so two runs against
        // the same broken input yield the same identifier.
        XmlError::FileNotFound { path } => DiagnosticPayload {
            code: DiagnosticCode::XmlFileNotFound,
            stage: Stage::Xml,
            expected: None,
            actual: Some(path.clone()),
            fix: None,
            key_fragments: vec![path.clone()],
        },
        XmlError::WrongRootElement { found } => DiagnosticPayload {
            code: DiagnosticCode::XmlWrongRootElement,
            stage: Stage::Xml,
            expected: Some(vec!["scxml".to_string()]),
            actual: Some(found.clone()),
            fix: None,
            key_fragments: vec![found.clone()],
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
        ValidationError::BytesMaxSizeViolation { procedure, detail } => DiagnosticPayload {
            code: DiagnosticCode::ValidationBytesMaxSizeViolation,
            stage: Stage::Validation,
            // The repair is "edit one of the two declared caps so they
            // agree" — open-ended numeric choice rather than a closed
            // candidate list, so `fix` stays `None`. The `detail`
            // string is the load-bearing context (which slot, which
            // upstream source, which numbers) and rides
            // `key_fragments` for content-hash uniqueness.
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![procedure.clone(), detail.clone()],
        },
        ValidationError::AlgorithmLocalShadowsParam { name, what } => DiagnosticPayload {
            code: DiagnosticCode::AlgorithmLocalShadowsParam,
            stage: Stage::Validation,
            expected: None,
            actual: Some(name.clone()),
            fix: None,
            key_fragments: vec![name.clone(), what.clone()],
        },
        ValidationError::AlgorithmLvalueUnsupported {
            target,
            restriction,
        } => DiagnosticPayload {
            code: DiagnosticCode::AlgorithmLvalueUnsupported,
            stage: Stage::Validation,
            expected: None,
            actual: Some(target.clone()),
            fix: None,
            key_fragments: vec![target.clone(), restriction.clone()],
        },
        ValidationError::AlgorithmReturnMissing => DiagnosticPayload {
            code: DiagnosticCode::AlgorithmReturnMissing,
            stage: Stage::Validation,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: Vec::new(),
        },
        // ── C7-lowering: algorithm-over-BC dispatch ──────────────────
        // Two codes ride `FixCarriesCandidates` (alias / method
        // enumeration); the other four are `NeutralOrDeterministic`.
        ValidationError::AlgorithmForeachSourceNotIterable {
            src,
            candidates,
        } => DiagnosticPayload {
            code: DiagnosticCode::AlgorithmForeachSourceNotIterable,
            stage: Stage::Validation,
            // Multi-axis repair (author may add a BC import OR rename
            // the foreach source to an existing bytes param); no
            // single canonical `Fix::Replace` value. The visible-name
            // union rides `key_fragments` for content-hash stability.
            expected: None,
            actual: Some(src.clone()),
            fix: None,
            key_fragments: {
                let mut k = vec![src.clone()];
                k.extend(candidates.iter().cloned());
                k
            },
        },
        ValidationError::AlgorithmCallTargetUnknown {
            target,
            alias,
            candidates,
        } => DiagnosticPayload {
            code: DiagnosticCode::AlgorithmCallTargetUnknown,
            stage: Stage::Validation,
            // Closed candidate set = declared import alias roster.
            // Mirrors `WorkerOutboxRefUnknown` precedent for sorted
            // alias-name candidates.
            expected: None,
            actual: Some(alias.clone()),
            fix: Some(Fix::ReplaceOneOf {
                candidates: candidates.clone(),
            }),
            key_fragments: vec![target.clone(), alias.clone()],
        },
        ValidationError::AlgorithmCallTargetMethodUnknown {
            target,
            alias,
            method,
            kind,
            candidates,
        } => DiagnosticPayload {
            code: DiagnosticCode::AlgorithmCallTargetMethodUnknown,
            stage: Stage::Validation,
            // Closed candidate set = the import kind's public-method
            // roster (BC: `{find_by_index, get, get_by_slot, len,
            // capacity}`; algorithm: the algorithm name itself).
            expected: None,
            actual: Some(method.clone()),
            fix: Some(Fix::ReplaceOneOf {
                candidates: candidates.clone(),
            }),
            key_fragments: vec![target.clone(), alias.clone(), kind.clone()],
        },
        ValidationError::AlgorithmBcMutationForbidden { target, method } => DiagnosticPayload {
            code: DiagnosticCode::AlgorithmBcMutationForbidden,
            stage: Stage::Validation,
            // Author repair is either (a) move the mutation into a
            // statechart-level `<sce:on-sample>`/`<onentry>` block
            // that legally calls BC `.insert`/`.remove`, or (b)
            // delete the call. No closed candidate — both repairs
            // sit outside the algorithm body so neither rides
            // `Fix::Replace`.
            expected: None,
            actual: Some(method.clone()),
            fix: None,
            key_fragments: vec![target.clone(), method.clone()],
        },
        ValidationError::AlgorithmForeachSourceBcWithBytesItemType { src, var_name } => {
            DiagnosticPayload {
                code: DiagnosticCode::AlgorithmForeachSourceBcWithBytesItemType,
                stage: Stage::Validation,
                // Repair is "remove the bytes-pattern `<sce:var
                // type=uint8>` and rely on the foreach item's
                // element-type binding" — a deletion, not a
                // replacement. `Fix::None`.
                expected: None,
                actual: Some(var_name.clone()),
                fix: None,
                key_fragments: vec![src.clone(), var_name.clone()],
            }
        }
        ValidationError::AlgorithmCallArgCountMismatch {
            target,
            actual,
            expected,
        } => DiagnosticPayload {
            code: DiagnosticCode::AlgorithmCallArgCountMismatch,
            stage: Stage::Validation,
            // Numeric mismatch; the imported callable's arity is the
            // single repair axis but arg expressions are author-
            // domain. `expected`/`fix` stay `None` per
            // `NeutralOrDeterministic` non_overlap_class default;
            // both numbers ride `key_fragments` for content-hash
            // discrimination (matches `ValidationCountMismatch`
            // precedent).
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![target.clone(), actual.to_string(), expected.to_string()],
        },
        ValidationError::CodecVariantArmUnreachable {
            codec,
            tag_field,
            tag_type,
            arm_count,
            domain_size,
        } => DiagnosticPayload {
            code: DiagnosticCode::CodecVariantArmUnreachable,
            stage: Stage::Validation,
            // Repair is open-ended ("add <sce:default>" OR "enumerate
            // missing values") rather than a closed candidate list, so
            // `fix` stays `None`. The structural context (codec name,
            // tag field, tag type, arm count, domain size) rides
            // `key_fragments` for content-hash uniqueness across
            // distinct variant declarations on different codecs.
            expected: None,
            actual: Some(tag_field.clone()),
            fix: None,
            key_fragments: {
                let mut k = vec![
                    codec.clone(),
                    tag_field.clone(),
                    tag_type.clone(),
                    arm_count.to_string(),
                ];
                if let Some(n) = domain_size {
                    k.push(n.to_string());
                }
                k
            },
        },
        ValidationError::CodecPresentIfRefsLaterField {
            codec,
            field,
            refers_to,
        } => DiagnosticPayload {
            code: DiagnosticCode::CodecPresentIfRefsLaterField,
            stage: Stage::Validation,
            // Repair is positional ("declare the carrier earlier" or
            // "fix the typo"), not a closed candidate set, so `fix`
            // stays `None`. The triple (codec, field, refers_to)
            // uniquely keys the violation across multiple present-if
            // declarations on the same codec.
            expected: None,
            actual: Some(field.clone()),
            fix: None,
            key_fragments: vec![codec.clone(), field.clone(), refers_to.clone()],
        },
        ValidationError::CodecVariantDuplicateDefaultArm {
            codec,
            first_arm_value,
            second_arm_value,
        } => DiagnosticPayload {
            code: DiagnosticCode::CodecVariantDuplicateDefaultArm,
            stage: Stage::Validation,
            // Repair is "remove default=\"true\" from all but one arm" —
            // author-domain (which arm is the intended default), not a
            // closed candidate set, so `fix` stays `None`. The triple
            // (codec, first_arm_value, second_arm_value) keys the
            // violation across multiple variants that might trip the
            // same diagnostic. Values format as hex to match the
            // user-facing arm-value convention.
            expected: None,
            actual: Some(codec.clone()),
            fix: None,
            key_fragments: vec![
                codec.clone(),
                format!("{first_arm_value:#x}"),
                format!("{second_arm_value:#x}"),
            ],
        },
        ValidationError::CodecVariantDefaultArmMidMismatch {
            codec,
            arm_value,
            inner_codec,
            inner_flag,
            inner_flag_value,
        } => DiagnosticPayload {
            code: DiagnosticCode::CodecVariantDefaultArmMidMismatch,
            stage: Stage::Validation,
            // Repair is "align outer arm value with inner flag value"
            // — author-domain (which side is canonical), not a closed
            // candidate set, so `fix` stays `None`. The 4-tuple
            // (codec, arm_value, inner_codec, inner_flag_value) keys
            // the violation across multiple variants and multiple
            // arms that might trip the same diagnostic.
            expected: Some(vec![format!("{arm_value:#x}")]),
            actual: Some(format!("{inner_flag_value:#x}")),
            fix: None,
            key_fragments: vec![
                codec.clone(),
                format!("{arm_value:#x}"),
                inner_codec.clone(),
                inner_flag.clone(),
                format!("{inner_flag_value:#x}"),
            ],
        },
        ValidationError::CodecVariantArmInnerMidUndeclared {
            codec,
            arm_value,
            inner_codec,
            expected_flag,
        } => DiagnosticPayload {
            code: DiagnosticCode::CodecVariantArmInnerMidUndeclared,
            stage: Stage::Validation,
            // Repair is "add <sce:flag value=> to the inner codec's
            // dispatch field" — concrete location (inner_codec +
            // expected_flag) and concrete value (arm_value), but the
            // author still chooses whether to edit the inner codec
            // or the outer arm reference, so `fix` stays `None`.
            // Key triple (codec, arm_value, inner_codec) is enough
            // to distinguish overlapping violations.
            expected: Some(vec![format!("{arm_value:#x}")]),
            actual: Some(inner_codec.clone()),
            fix: None,
            key_fragments: vec![
                codec.clone(),
                format!("{arm_value:#x}"),
                inner_codec.clone(),
                expected_flag.clone(),
            ],
        },
        ValidationError::CodecRepeatCountRefsLaterField {
            codec,
            field,
            refers_to,
        } => DiagnosticPayload {
            code: DiagnosticCode::CodecRepeatCountRefsLaterField,
            stage: Stage::Validation,
            // Repair is positional (reorder the count field, or fix
            // the attribute typo) — no closed candidate set, so `fix`
            // stays `None`. The (codec, field, refers_to) triple keys
            // the violation across multiple <sce:repeat> elements on
            // the same codec.
            expected: None,
            actual: Some(field.clone()),
            fix: None,
            key_fragments: vec![codec.clone(), field.clone(), refers_to.clone()],
        },
        ValidationError::TestVectorUnsupportedKind { name, kind } => DiagnosticPayload {
            code: DiagnosticCode::AlgorithmTestVectorUnsupportedKind,
            stage: Stage::Validation,
            // Repair is structural (move the test vector to an
            // algorithm file, or use the JSON oracle harness). The
            // (name, kind-as-attr) pair keys the violation; kind is
            // serialised through ForgeKind::Debug to match how the
            // error's Display surfaces it.
            expected: None,
            actual: Some(name.clone()),
            fix: None,
            key_fragments: vec![name.clone(), format!("{kind:?}")],
        },
        ValidationError::CodecTlvChainDepthUnspecified { codec, field } => DiagnosticPayload {
            code: DiagnosticCode::CodecTlvChainDepthUnspecified,
            stage: Stage::Validation,
            // Repair is text-level (add `max-depth="N"`), but the
            // candidate value is author-domain (depends on the protocol
            // shape) so `fix` stays `None`. (codec, field) keys the
            // violation across multiple <sce:tlv-chain> elements on the
            // same codec.
            expected: None,
            actual: Some(field.clone()),
            fix: None,
            key_fragments: vec![codec.clone(), field.clone()],
        },
        ValidationError::CodecDmaAlignmentUnsatisfiable {
            codec,
            field,
            burst_align,
            reason: _,
        } => DiagnosticPayload {
            code: DiagnosticCode::CodecDmaAlignmentUnsatisfiable,
            stage: Stage::Validation,
            // Repair is structural (reorder fields, lower the
            // alignment, or change the variable predecessor) — no
            // closed candidate set. (codec, field, burst_align) keys
            // the violation across multiple aligned fields on the
            // same codec.
            expected: None,
            actual: Some(field.clone()),
            fix: None,
            key_fragments: vec![codec.clone(), field.clone(), burst_align.to_string()],
        },
        ValidationError::CodecParentFlagMismatch {
            body_codec,
            parent_codec,
            reason: _,
        } => DiagnosticPayload {
            code: DiagnosticCode::CodecParentFlagMismatch,
            stage: Stage::Validation,
            // Repair is structural (fix the body's declared parent-
            // flag layout to match the parent's carrier shape, or
            // wire the body codec to a different parent) — no
            // closed candidate set. (body_codec, parent_codec) keys
            // the violation across multiple variant arm wire-ups
            // referencing the same body codec.
            expected: None,
            actual: Some(body_codec.clone()),
            fix: None,
            key_fragments: vec![body_codec.clone(), parent_codec.clone()],
        },
        ValidationError::LinkFramerMissing { name } => DiagnosticPayload {
            code: DiagnosticCode::LinkFramerMissing,
            stage: Stage::Validation,
            // Deterministic structural repair (add the framer child).
            // The candidate codec name is open — every fixture's
            // framer ref is fixture-specific — so the fix surface is
            // an `AddElement` directive carrying the element shape,
            // not a closed candidate list. v1 emits `fix: None` and
            // relies on the message's prose to guide the author.
            // (This matches the pattern for other Validation
            // structural-repair codes whose targets are open.)
            expected: None,
            actual: Some(name.clone()),
            fix: None,
            key_fragments: vec![name.clone()],
        },
        ValidationError::LinkLinkClassUnknown { name, value } => DiagnosticPayload {
            code: DiagnosticCode::LinkLinkClassUnknown,
            stage: Stage::Validation,
            // Closed-enum candidate set (RFC §5.C lines 765-771) —
            // emit `Fix::ReplaceOneOf` so agents can mechanically
            // pick a legal class. Per the non-overlap invariant
            // (`FixCarriesCandidates` bucket) `expected` stays
            // `None`; the candidate list rides `fix`.
            expected: None,
            actual: Some(value.clone()),
            fix: Some(Fix::ReplaceOneOf {
                candidates: crate::forge::model::LinkClass::ALL_NAMES
                    .iter()
                    .map(|s| (*s).to_string())
                    .collect(),
            }),
            key_fragments: vec![name.clone(), value.clone()],
        },
        ValidationError::LinkBackpressureUndeclared { name } => DiagnosticPayload {
            code: DiagnosticCode::LinkBackpressureUndeclared,
            stage: Stage::Validation,
            // Structural element-add repair; like LinkFramerMissing
            // there is no closed candidate set for the element shape
            // (the body text picks one of three policies, but the
            // missing-element repair is "add the element with one of
            // those bodies", not "replace this value"). v1 emits
            // `fix: None` and relies on the message prose enumerating
            // the three legal bodies.
            expected: None,
            actual: Some(name.clone()),
            fix: None,
            key_fragments: vec![name.clone()],
        },
        ValidationError::LinkClassUnsupportedOnTarget {
            name,
            class,
            target_os,
            candidates,
        } => DiagnosticPayload {
            code: DiagnosticCode::LinkClassUnsupportedOnTarget,
            stage: Stage::Validation,
            // OS-axis candidate set (the list of OSes the declared
            // class admits per RFC §5.C lines 765-771) — emit
            // `Fix::ReplaceOneOf` so agents can mechanically pick a
            // legal target deployment OS. The author can also change
            // the class via the prose; both are valid repair surfaces
            // but the structured fix carries only the OS axis (one
            // axis per ReplaceOneOf entry per non-overlap shape).
            expected: None,
            actual: Some(target_os.clone()),
            fix: Some(Fix::ReplaceOneOf {
                candidates: candidates.clone(),
            }),
            key_fragments: vec![name.clone(), class.clone(), target_os.clone()],
        },
        ValidationError::LinkPoolSlotSmallerThanFramerMax {
            link_name,
            pool_side,
            pool_alias,
            pool_slot_size,
            framer_alias,
            framer_max_bytes,
        } => DiagnosticPayload {
            code: DiagnosticCode::LinkPoolSlotSmallerThanFramerMax,
            stage: Stage::Validation,
            // Two-axis repair (raise pool slot-size OR shrink codec
            // worst-case body) — both author choices, neither machine-
            // decidable from a closed candidate list, so `Fix::None`
            // and the message prose names both axes. The `actual`
            // carries the pool's declared slot-size so the wire record
            // makes the violation magnitude inspectable without
            // re-parsing the message; the framer's max-bytes rides
            // on `key_fragments` so the diagnostic id stays stable
            // under message rewording but distinct across different
            // mismatch magnitudes.
            expected: None,
            actual: Some(pool_slot_size.to_string()),
            fix: None,
            key_fragments: vec![
                link_name.clone(),
                (*pool_side).to_string(),
                pool_alias.clone(),
                pool_slot_size.to_string(),
                framer_alias.clone(),
                framer_max_bytes.to_string(),
            ],
        },
        ValidationError::BufferPoolSectionConflict {
            name,
            machine,
            section,
            candidates,
        } => DiagnosticPayload {
            code: DiagnosticCode::MemPoolSectionConflict,
            stage: Stage::Validation,
            // Section-name candidate set (the list of regions the
            // resolved machine declares in deploy.yaml) — `Fix::ReplaceOneOf`
            // carries the region-name axis so agents can mechanically
            // pick a legal section. The author can alternately extend
            // the deploy.yaml memory map; the message prose names
            // both repair surfaces. RFC §5.E lines 1000-1023 + 1537.
            expected: None,
            actual: Some(section.clone()),
            fix: Some(Fix::ReplaceOneOf {
                candidates: candidates.clone(),
            }),
            key_fragments: vec![name.clone(), machine.clone(), section.clone()],
        },
        ValidationError::BufferPoolTooLarge {
            name,
            machine,
            section,
            slot_count,
            slot_size,
            bytes_required,
            region_size,
        } => DiagnosticPayload {
            code: DiagnosticCode::MemPoolTooLarge,
            stage: Stage::Validation,
            // Two-axis repair (raise region size OR shrink slot dims) —
            // both are author choices, neither machine-decidable from
            // a closed candidate list, so the structured fix is
            // `Fix::None` and the message prose names both axes. The
            // `actual` carries the bytes_required value so the wire
            // record makes the violation magnitude inspectable without
            // re-parsing the message.
            expected: None,
            actual: Some(bytes_required.to_string()),
            fix: None,
            key_fragments: vec![
                name.clone(),
                machine.clone(),
                section.clone(),
                slot_count.to_string(),
                slot_size.to_string(),
                region_size.to_string(),
            ],
        },
        ValidationError::BufferPoolInterPoolPaddingNotEmitted { name } => DiagnosticPayload {
            code: DiagnosticCode::MemInterPoolPaddingNotEmitted,
            stage: Stage::Validation,
            // Codegen-invariant violation — no authoring repair surface
            // exists. `Fix::None` because the user cannot fix this
            // from the SCXML side; the prose links to the issue tracker.
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![name.clone()],
        },
        ValidationError::BufferPoolCacheLineAlignment {
            name,
            machine,
            pool_alignment,
            dcache_line_size,
        } => DiagnosticPayload {
            code: DiagnosticCode::MemCacheLineAlignment,
            stage: Stage::Validation,
            // Multi-axis repair (raise alignment OR adjust deploy
            // dcache_line_size) — `MemPoolTooLarge` precedent
            // (NeutralOrDeterministic, `Fix::None`). The `actual`
            // carries the pool's alignment value so the wire record
            // makes the violation magnitude inspectable without
            // re-parsing the message.
            expected: None,
            actual: Some(pool_alignment.to_string()),
            fix: None,
            key_fragments: vec![
                name.clone(),
                machine.clone(),
                pool_alignment.to_string(),
                dcache_line_size.to_string(),
            ],
        },
        ValidationError::BufferPoolSlotSizeNotCacheLineMultiple {
            name,
            machine,
            slot_size,
            dcache_line_size,
            remainder,
            next_multiple,
        } => DiagnosticPayload {
            code: DiagnosticCode::MemSlotSizeNotCacheLineMultiple,
            stage: Stage::Validation,
            // `Fix::None` per `MemPoolTooLarge` precedent — the
            // repair (round slot_size up to next_multiple) is named
            // in message prose; multi-axis interpretation lets the
            // author choose between rounding the slot or shrinking
            // the dcache_line_size assumption in deploy.yaml.
            expected: None,
            actual: Some(slot_size.to_string()),
            fix: None,
            key_fragments: vec![
                name.clone(),
                machine.clone(),
                slot_size.to_string(),
                dcache_line_size.to_string(),
                remainder.to_string(),
                next_multiple.to_string(),
            ],
        },
        ValidationError::BufferPoolCachePolicyUnsupportedOnNoDcacheCore {
            name,
            machine,
            declared_policy,
        } => DiagnosticPayload {
            code: DiagnosticCode::MemCachePolicyUnsupportedOnNoDcacheCore,
            stage: Stage::Validation,
            // Closed candidate set = `["none"]` — the only legal
            // cache-policy on a core without D-cache. `Fix::ReplaceOneOf`
            // mirrors the `MemPoolSectionConflict` repair pattern.
            expected: None,
            actual: Some(declared_policy.clone()),
            fix: Some(Fix::ReplaceOneOf {
                candidates: vec!["none".to_string()],
            }),
            key_fragments: vec![
                name.clone(),
                machine.clone(),
                declared_policy.clone(),
            ],
        },
        ValidationError::PoolCacheMaintenanceMisplaced { attempted_symbol } => DiagnosticPayload {
            code: DiagnosticCode::PoolCacheMaintenanceMisplaced,
            stage: Stage::Validation,
            // Author guard — repair is to remove the offending
            // `<sce:extern>`. `Fix::None` because there is no
            // alternate symbol to suggest; the buffer-pool kind
            // handles cache calls automatically when
            // `cache-policy: maintain`.
            expected: None,
            actual: Some(attempted_symbol.clone()),
            fix: None,
            key_fragments: vec![attempted_symbol.clone()],
        },
        ValidationError::PoolSpeculativePrefetchFlagMissing { machine, pool_name } => {
            DiagnosticPayload {
                code: DiagnosticCode::PoolSpeculativePrefetchFlagMissing,
                stage: Stage::Validation,
                // `Fix::None` — author choice between two opposite
                // boolean values driven by the SoC datasheet (M7+/A-class
                // = true, M3/M4 = false). Message prose names both axes
                // so the author can pick without consulting external
                // documentation.
                expected: None,
                actual: None,
                fix: None,
                key_fragments: vec![machine.clone(), pool_name.clone()],
            }
        }
        ValidationError::PoolCachePreArmInvalidateMissingOnSpeculativeCore { name, backend } => {
            DiagnosticPayload {
                code: DiagnosticCode::PoolCachePreArmInvalidateMissingOnSpeculativeCore,
                stage: Stage::Validation,
                // Codegen-invariant violation — no authoring repair
                // surface exists. β `mem/inter-pool-padding-not-emitted`
                // precedent: `Fix::None`; the prose links to the issue
                // tracker so a regression report finds the right team.
                expected: None,
                actual: None,
                fix: None,
                key_fragments: vec![name.clone(), backend.clone()],
            }
        }
        ValidationError::BufferPoolSampleTypestateAttributesDisabled { name } => {
            DiagnosticPayload {
                code: DiagnosticCode::PoolSampleTypestateAttributesDisabled,
                stage: Stage::Validation,
                // Codegen-invariant violation — no authoring repair
                // surface exists. β `mem/inter-pool-padding-not-emitted`
                // precedent: `Fix::None`; the prose links to the issue
                // tracker so a regression report finds the right team.
                expected: None,
                actual: None,
                fix: None,
                key_fragments: vec![name.clone()],
            }
        }
        ValidationError::OnSampleInvalidParent {
            path,
            actual_parent,
        } => DiagnosticPayload {
            code: DiagnosticCode::ScxmlOnSampleInvalidParent,
            stage: Stage::Validation,
            // The repair is "move the element under a state/parallel"
            // — too contextual to express as a structured fix. Author
            // reads the message and resolves at the source.
            expected: None,
            actual: Some(actual_parent.clone()),
            fix: None,
            key_fragments: vec![path.clone(), actual_parent.clone()],
        },
        ValidationError::OnSampleLinkDuplicateInState { state_id, link } => DiagnosticPayload {
            code: DiagnosticCode::ScxmlOnSampleLinkDuplicateInState,
            stage: Stage::Validation,
            // No structured fix — author chooses which duplicate to
            // keep. `actual` surfaces the offending link name so CLI
            // consumers see it without parsing the message body.
            expected: None,
            actual: Some(link.clone()),
            fix: None,
            key_fragments: vec![state_id.clone(), link.clone()],
        },
        ValidationError::OnSampleEventNameConflict {
            event,
            reserved_prefix,
        } => DiagnosticPayload {
            code: DiagnosticCode::ScxmlOnSampleEventNameConflict,
            stage: Stage::Validation,
            // No structured fix — the suggested replacement
            // `sample.<event>` is one of many valid choices. Prose
            // makes the recommendation; author picks the final name.
            expected: None,
            actual: Some(event.clone()),
            fix: None,
            key_fragments: vec![event.clone(), reserved_prefix.clone()],
        },
        ValidationError::OnSampleLinkNotDeclared {
            state_id,
            link,
            candidates,
        } => DiagnosticPayload {
            code: DiagnosticCode::ScxmlOnSampleLinkNotDeclared,
            stage: Stage::Validation,
            // Candidate list rides `Fix::ReplaceOneOf` per the
            // FixCarriesCandidates non-overlap class — `expected` is
            // None to keep the contract's fix/expected separation.
            expected: None,
            actual: Some(link.clone()),
            fix: Some(Fix::ReplaceOneOf {
                candidates: candidates.clone(),
            }),
            key_fragments: vec![state_id.clone(), link.clone()],
        },
        ValidationError::OnSampleLinkWrongKind {
            state_id,
            link,
            actual_kind,
            candidates,
        } => DiagnosticPayload {
            code: DiagnosticCode::ScxmlOnSampleLinkWrongKind,
            stage: Stage::Validation,
            // `actual` carries the resolved-but-wrong forge kind
            // label so consumers see what was found without parsing
            // the message body; the candidate list rides `fix` for
            // the legal alternatives.
            expected: None,
            actual: Some(actual_kind.clone()),
            fix: Some(Fix::ReplaceOneOf {
                candidates: candidates.clone(),
            }),
            key_fragments: vec![state_id.clone(), link.clone(), actual_kind.clone()],
        },
        ValidationError::PoolSampleTakeWithoutStagePool {
            state_id,
            link,
            candidates,
        } => DiagnosticPayload {
            code: DiagnosticCode::PoolSampleTakeWithoutStagePool,
            stage: Stage::Validation,
            // `actual` carries the link name — same shape as the
            // OnSampleLink* sister diagnostics so consumers see "which
            // link triggered this" without parsing the message. The
            // candidate list rides `Fix::ReplaceOneOf` over the build's
            // declared buffer-pool kind names so authors picking a
            // `<sce:stage-pool ref="...">` value see legal pools at hand.
            expected: None,
            actual: Some(link.clone()),
            fix: Some(Fix::ReplaceOneOf {
                candidates: candidates.clone(),
            }),
            key_fragments: vec![state_id.clone(), link.clone()],
        },
        ValidationError::PoolSampleCallbackSignatureNonBorrow {
            state_id,
            link,
            callback,
            reason,
        } => DiagnosticPayload {
            code: DiagnosticCode::PoolSampleCallbackSignatureNonBorrow,
            stage: Stage::Validation,
            // `actual` carries the offending callback verbatim so
            // consumers can quote it without parsing the message. No
            // closed-set fix surface today — the path is free-form
            // and the author repair depends on which `reason` arm
            // fired (NeutralOrDeterministic non_overlap_class).
            // `key_fragments` includes a stable token of the reason
            // arm so duplicate-detection across multiple bad paths
            // doesn't collapse them into one wire id.
            expected: None,
            actual: Some(callback.clone()),
            fix: None,
            key_fragments: vec![
                state_id.clone(),
                link.clone(),
                callback.clone(),
                callback_reason_tag(reason).to_string(),
            ],
        },
        ValidationError::WorkerSharedMutableState {
            worker_name,
            reason,
        } => DiagnosticPayload {
            code: DiagnosticCode::WorkerSharedMutableState,
            stage: Stage::Validation,
            // `actual` carries a stable wire token derived from the
            // layer that fired, plus the offending fragment for that
            // layer (alias name + src for layer 1; element + attr +
            // foreign prefix for layer 2). Closed-set repair surface
            // is empty — the offending path may be removed, refactored
            // through the inbox, or replaced with a `<sce:outbox>` ref;
            // SCE cannot synthesize the choice. `fix: None` per the
            // NeutralOrDeterministic non_overlap_class.
            // `key_fragments` includes the layer tag so duplicate
            // detection across multiple layer-1 + layer-2 violations
            // in the same worker doesn't collapse them.
            expected: None,
            actual: Some(worker_shared_state_actual(reason)),
            fix: None,
            key_fragments: vec![
                worker_name.clone(),
                worker_shared_state_layer_tag(reason).to_string(),
                worker_shared_state_actual(reason),
            ],
        },
        // ── §5.D C2-β cross-resolution: link-rx + outbox ref ──
        ValidationError::WorkerLinkRxRefUnknown {
            worker_name,
            ref_name,
            candidates,
            candidates_list: _,
        } => DiagnosticPayload {
            code: DiagnosticCode::WorkerLinkRxRefUnknown,
            stage: Stage::Validation,
            // `actual` carries the offending ref; closed candidate
            // list (sorted kind=link import aliases) rides
            // `Fix::ReplaceOneOf`. η-precedent: LinkClassUnsupportedOnTarget.
            expected: None,
            actual: Some(ref_name.clone()),
            fix: Some(Fix::ReplaceOneOf {
                candidates: candidates.clone(),
            }),
            key_fragments: vec![worker_name.clone(), ref_name.clone()],
        },
        // ── §5.I C2-β SPSC inbox ordering ──
        ValidationError::WorkerInboxOrderingUnspecified {
            worker_name,
        } => DiagnosticPayload {
            code: DiagnosticCode::WorkerInboxOrderingUnspecified,
            stage: Stage::Validation,
            // No `actual` value to surface — the violation is the
            // absence of the `ordering` attribute. NeutralOrDeterministic
            // non_overlap_class with `fix: None`; author picks
            // `acq_rel` or `relaxed` based on placement (not a closed
            // candidate set today — semantics of the choice ride the
            // diagnostic message, not the wire payload).
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![worker_name.clone()],
        },
        ValidationError::WorkerInboxOrderingRelaxedAcrossCores {
            worker_name,
            producer_core,
            consumer_core,
        } => DiagnosticPayload {
            code: DiagnosticCode::WorkerInboxOrderingRelaxedAcrossCores,
            stage: Stage::Validation,
            // Codegen-invariant: relaxed declared, but placement pins
            // producer and consumer on different cores. `actual`
            // carries the offending ordering string verbatim so the
            // wire surface names what was declared rather than what
            // was inferred. NeutralOrDeterministic — author picks
            // between flipping ordering or re-pinning placement;
            // neither axis is a closed candidate today.
            expected: None,
            actual: Some("relaxed".to_string()),
            fix: None,
            key_fragments: vec![
                worker_name.clone(),
                producer_core.to_string(),
                consumer_core.to_string(),
            ],
        },
        ValidationError::WorkerSchedulerUnsupported {
            worker_name,
            machine,
        } => DiagnosticPayload {
            code: DiagnosticCode::WorkerSchedulerUnsupported,
            stage: Stage::Validation,
            // Forge-side anchor for spec §5.D line 912. Per-doc miss
            // against the resolved machine's `workers` map.
            // NeutralOrDeterministic — author either declares the
            // worker in deploy.yaml or removes the Worker doc.
            expected: None,
            actual: Some(worker_name.clone()),
            fix: None,
            key_fragments: vec![worker_name.clone(), machine.clone()],
        },
        // ── §5.D C2 follow-up Atomic B outbox cross-resolution ──
        ValidationError::WorkerOutboxRefUnknown {
            worker_name,
            outbox_value,
            owner,
            candidates,
            candidates_list: _,
        } => DiagnosticPayload {
            code: DiagnosticCode::WorkerOutboxRefUnknown,
            stage: Stage::Validation,
            // `actual` carries the full authored ref so the diagnostic
            // surfaces what was written (owner segment alone hides the
            // suffix); closed candidate list (sorted statechart +
            // worker `.inbox` set) rides `Fix::ReplaceOneOf`.
            // η-precedent: `WorkerLinkRxRefUnknown` carries candidates
            // the same way.
            expected: None,
            actual: Some(outbox_value.clone()),
            fix: Some(Fix::ReplaceOneOf {
                candidates: candidates.clone(),
            }),
            key_fragments: vec![worker_name.clone(), owner.clone()],
        },
        ValidationError::WorkerOutboxTargetWrongKind {
            worker_name,
            outbox_value,
            owner,
            actual_kind,
            candidates,
            candidates_list: _,
        } => DiagnosticPayload {
            code: DiagnosticCode::WorkerOutboxTargetWrongKind,
            stage: Stage::Validation,
            // `actual` carries the full authored ref so the diagnostic
            // names what was written; the resolved kind rides
            // `key_fragments` for byte-stable test discrimination
            // between "unknown" and "wrong-kind" failure axes.
            expected: None,
            actual: Some(outbox_value.clone()),
            fix: Some(Fix::ReplaceOneOf {
                candidates: candidates.clone(),
            }),
            key_fragments: vec![
                worker_name.clone(),
                owner.clone(),
                actual_kind.clone(),
            ],
        },
        ValidationError::WorkerOutboxTargetSuffixInvalid {
            worker_name,
            outbox_value,
            owner,
            suffix,
        } => DiagnosticPayload {
            code: DiagnosticCode::WorkerOutboxTargetSuffixInvalid,
            stage: Stage::Validation,
            // Single-value deterministic repair: keep the authored
            // owner, replace suffix with `inbox`. `Fix::ReplaceWith`
            // carries `{owner}.inbox` so an agent applies the fix
            // without rewriting the prefix. NeutralOrDeterministic
            // non_overlap_class.
            expected: None,
            actual: Some(outbox_value.clone()),
            fix: Some(Fix::ReplaceWith {
                to: format!("{owner}.inbox"),
            }),
            key_fragments: vec![
                worker_name.clone(),
                owner.clone(),
                suffix.clone(),
            ],
        },
        // ── §5.L Bounded-collection parse-time structure validators (C6-α) ──
        ValidationError::CollectionOrderingSortedRequiresIndexBy {
            collection_name,
        } => DiagnosticPayload {
            code: DiagnosticCode::CollectionOrderingSortedRequiresIndexBy,
            stage: Stage::Validation,
            // No closed candidate set — the repair requires authoring a
            // field name from the element-type struct, which is author-
            // domain knowledge (cf. non_overlap_class entry's reasoning).
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![collection_name.clone()],
        },
        ValidationError::CollectionOverflowPolicyOldestWinsRequiresOrderingInsertion {
            collection_name,
        } => DiagnosticPayload {
            code: DiagnosticCode::CollectionOverflowPolicyOldestWinsRequiresOrderingInsertion,
            stage: Stage::Validation,
            // Per non_overlap_class reasoning: two equally valid repairs
            // (change ordering to `insertion`, or change policy off
            // `oldest-wins`) means no single canonical candidate →
            // NeutralOrDeterministic without a `Fix`.
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![collection_name.clone()],
        },
        // ── §5.M Fragment-reassembly variant parse-time structure validators (C9-α) ──
        ValidationError::MemReassemblyPoolVariantMissingMaxFragments { pool_name } => {
            DiagnosticPayload {
                code: DiagnosticCode::MemReassemblyPoolVariantMissingMaxFragments,
                stage: Stage::Validation,
                // Per non_overlap_class reasoning: the concrete fragment
                // count is author-domain knowledge (wire framer's per-
                // message maximum), so no closed candidate set.
                expected: None,
                actual: None,
                fix: None,
                key_fragments: vec![pool_name.clone()],
            }
        }
        ValidationError::MemReassemblyPoolVariantMissingTimeout { pool_name } => {
            DiagnosticPayload {
                code: DiagnosticCode::MemReassemblyPoolVariantMissingTimeout,
                stage: Stage::Validation,
                // Per non_overlap_class reasoning: the concrete timeout
                // value is author-domain (link latency budget + acceptable
                // hold time), so no closed candidate set.
                expected: None,
                actual: None,
                fix: None,
                key_fragments: vec![pool_name.clone()],
            }
        }
        // ── §5.M Fragment-reassembly cross-doc validators (C13-α-2 + C9-β) ──
        ValidationError::MemReassemblySlotSizeBelowDeclaredMtu {
            pool_name,
            slot_size,
            mtu_bytes,
            machine,
            link_name,
        } => DiagnosticPayload {
            code: DiagnosticCode::MemReassemblySlotSizeBelowDeclaredMtu,
            stage: Stage::Validation,
            // `actual` = the offending slot_size; `expected` = the
            // mtu lower bound. NeutralOrDeterministic: multi-axis
            // repair (raise slot_size / lower mtu / bind different
            // pool) — author chooses.
            actual: Some(slot_size.to_string()),
            expected: Some(vec![mtu_bytes.to_string()]),
            fix: None,
            key_fragments: vec![
                pool_name.clone(),
                slot_size.to_string(),
                mtu_bytes.to_string(),
                machine.clone(),
                link_name.clone(),
            ],
        },
        ValidationError::ReassemblyMaxFragmentsInsufficientForMtu {
            pool_name,
            slot_size,
            max_fragments_per_message,
            mtu_bytes,
            required,
            machine,
            link_name,
        } => DiagnosticPayload {
            code: DiagnosticCode::ReassemblyMaxFragmentsInsufficientForMtu,
            stage: Stage::Validation,
            actual: Some(slot_size.to_string()),
            expected: Some(vec![required.to_string()]),
            fix: None,
            key_fragments: vec![
                pool_name.clone(),
                slot_size.to_string(),
                max_fragments_per_message.to_string(),
                mtu_bytes.to_string(),
                required.to_string(),
                machine.clone(),
                link_name.clone(),
            ],
        },
        ValidationError::ReassemblyExpectedFragmentationRateHigh {
            pool_name,
            slot_size,
            expected_p99_bytes,
            rate_percent,
            machine,
            link_name,
        } => DiagnosticPayload {
            code: DiagnosticCode::ReassemblyExpectedFragmentationRateHigh,
            stage: Stage::Validation,
            // `actual` = the computed rate percent; `expected` = the
            // 25% threshold (default per spec, suppressible via §5.K
            // pool_defaults C13-γ scope).
            actual: Some(rate_percent.to_string()),
            expected: Some(vec!["25".to_string()]),
            fix: None,
            key_fragments: vec![
                pool_name.clone(),
                slot_size.to_string(),
                expected_p99_bytes.to_string(),
                rate_percent.to_string(),
                machine.clone(),
                link_name.clone(),
            ],
        },
        ValidationError::ReassemblyUntrustedLinkBinding {
            pool_name,
            trust_class,
            machine,
            link_name,
        } => DiagnosticPayload {
            code: DiagnosticCode::ReassemblyUntrustedLinkBinding,
            stage: Stage::Validation,
            // `actual` = the offending trust_class; expected omitted
            // (single permitted value `established_session` would
            // collapse to a FixCarriesCandidates shape, but
            // Q-C13-α2-7 (a) keeps the second valid repair "remove
            // binding entirely" in scope so the code stays
            // NeutralOrDeterministic and the message carries the
            // canonical replacement value).
            actual: Some(trust_class.clone()),
            expected: None,
            fix: None,
            key_fragments: vec![
                pool_name.clone(),
                trust_class.clone(),
                machine.clone(),
                link_name.clone(),
            ],
        },
        ValidationError::ReassemblyTrustClassMissingOnFragmentingLink {
            pool_name,
            machine,
            link_name,
        } => DiagnosticPayload {
            code: DiagnosticCode::ReassemblyTrustClassMissingOnFragmentingLink,
            stage: Stage::Validation,
            // No `actual` (the field is absent); message text carries
            // the canonical "declare trust_class: established_session"
            // guidance — but the second valid repair (remove the pool
            // binding) keeps this in NeutralOrDeterministic.
            actual: None,
            expected: None,
            fix: None,
            key_fragments: vec![
                pool_name.clone(),
                machine.clone(),
                link_name.clone(),
            ],
        },
        ValidationError::ReassemblyStageCopyWcetExceedsSlotBudget {
            machine,
            link_name,
            expected_p99_bytes,
            memcpy_cycles_per_byte,
            clock_freq_mhz,
            worker_slot_budget_us,
            stage_copy_wcet_us,
        } => DiagnosticPayload {
            code: DiagnosticCode::ReassemblyStageCopyWcetExceedsSlotBudget,
            stage: Stage::Validation,
            // `actual` = computed stage-copy WCET; `expected` = the
            // slot budget ceiling. The four inputs feeding the formula
            // ride key_fragments so a downstream renderer can quote
            // them verbatim.
            actual: Some(stage_copy_wcet_us.to_string()),
            expected: Some(vec![worker_slot_budget_us.to_string()]),
            fix: None,
            key_fragments: vec![
                machine.clone(),
                link_name.clone(),
                expected_p99_bytes.to_string(),
                // f32 normalized to a stable string form so
                // key_fragments hash is reproducible across runs.
                format!("{memcpy_cycles_per_byte}"),
                clock_freq_mhz.to_string(),
                worker_slot_budget_us.to_string(),
                stage_copy_wcet_us.to_string(),
            ],
        },
        // ── §5.M C9-γ codegen self-check ──
        ValidationError::ReassemblyPeerIdNotZidOnEstablishedSession {
            pool_name,
            language,
        } => DiagnosticPayload {
            code: DiagnosticCode::ReassemblyPeerIdNotZidOnEstablishedSession,
            stage: Stage::Validation,
            // Codegen-internal invariant — author-side `actual` /
            // `expected` / `fix` carry no useful information (the
            // "fix" is to file an upstream bug, not to edit the
            // SCXML). Mirrors `BufferPoolInterPoolPaddingNotEmitted`
            // shape (diagnostic.rs:3921, generator.rs:10225).
            actual: None,
            expected: None,
            fix: None,
            key_fragments: vec![pool_name.clone(), language.clone()],
        },
        // ── §5.C C10-α codegen self-check ──
        ValidationError::LinkListenerLinkNotPairedWithEstablishedSibling {
            link_name,
            language,
        } => DiagnosticPayload {
            code: DiagnosticCode::LinkListenerLinkNotPairedWithEstablishedSibling,
            stage: Stage::Validation,
            // Codegen-internal invariant — same shape as C9-γ
            // `ReassemblyPeerIdNotZidOnEstablishedSession`. Author-
            // side `actual` / `expected` / `fix` carry no useful
            // information.
            actual: None,
            expected: None,
            fix: None,
            key_fragments: vec![link_name.clone(), language.clone()],
        },
        // ── §5.M C10-α reassembly-binding-on-unpaired-listener ──
        ValidationError::ReassemblyBindingOnUnpairedListener {
            pool_name,
            machine,
            link_name,
        } => DiagnosticPayload {
            code: DiagnosticCode::MeshDeployReassemblyBindingOnUnpairedListener,
            stage: Stage::Validation,
            // Two valid repair paths (add `Accepting.*` vs remove
            // binding) — NeutralOrDeterministic. Mirrors
            // `ReassemblyTrustClassMissingOnFragmentingLink` shape:
            // no `actual` / `expected`; message text carries both
            // structural repairs.
            actual: None,
            expected: None,
            fix: None,
            key_fragments: vec![
                pool_name.clone(),
                machine.clone(),
                link_name.clone(),
            ],
        },
        // ── §5.N C10-β link/inbound-event-queue-unsized ──
        ValidationError::LinkInboundEventQueueUnsized {
            machine,
            link_name,
            inbound_event_count,
        } => DiagnosticPayload {
            code: DiagnosticCode::LinkInboundEventQueueUnsized,
            stage: Stage::Validation,
            // `actual` = the link's declared inbound event count,
            // making the surface volume visible to authors. Two-axis
            // repair (per-instance sce:capacity vs per-machine deploy
            // default) — NeutralOrDeterministic, no closed candidate
            // set, message text carries both structural repairs.
            actual: Some(inbound_event_count.to_string()),
            expected: None,
            fix: None,
            key_fragments: vec![
                machine.clone(),
                link_name.clone(),
                inbound_event_count.to_string(),
            ],
        },
        // ── §5.K C13-γ stage-copy policy promotion + opt-out rejection ──
        ValidationError::PoolStageCopyPolicyError {
            pool_name,
            slot_size,
            expected_p99_bytes,
            rate_percent,
            machine,
            link_name,
            policy,
        } => DiagnosticPayload {
            code: DiagnosticCode::PoolStageCopyPolicyError,
            stage: Stage::Validation,
            // `actual` = computed rate %; `expected` = 25% threshold.
            // NeutralOrDeterministic: multi-axis repair, no closed
            // candidate set.
            actual: Some(rate_percent.to_string()),
            expected: Some(vec!["25".to_string()]),
            fix: None,
            key_fragments: vec![
                pool_name.clone(),
                slot_size.to_string(),
                expected_p99_bytes.to_string(),
                rate_percent.to_string(),
                machine.clone(),
                link_name.clone(),
                policy.clone(),
            ],
        },
        ValidationError::PoolStageCopyAcceptRejectedUnderForbid {
            machine,
            link_name,
        } => DiagnosticPayload {
            code: DiagnosticCode::PoolStageCopyAcceptRejectedUnderForbid,
            stage: Stage::Validation,
            // `actual` carries the offending element's name so the
            // wire payload surfaces what was rejected. Per
            // non_overlap_class: two valid repair paths (remove
            // opt-out vs change policy), no closed candidate set.
            actual: Some("<sce:accept-stage-copy-rate>".to_string()),
            expected: None,
            fix: None,
            key_fragments: vec![machine.clone(), link_name.clone()],
        },
        // ── §5.L Bounded-collection cross-doc resolution (C6-β) ──
        ValidationError::CollectionElementTypeNotAKind {
            collection_name,
            element_type,
            candidates,
            candidates_list: _,
        } => DiagnosticPayload {
            code: DiagnosticCode::CollectionElementTypeNotAKind,
            stage: Stage::Validation,
            // `actual` carries the authored element-type body text so the
            // diagnostic surfaces what was written; closed candidate list
            // (sorted codec + procedure name union) rides
            // `Fix::ReplaceOneOf`. η-precedent: `WorkerOutboxRefUnknown`
            // carries the same shape.
            expected: None,
            actual: Some(element_type.clone()),
            fix: Some(Fix::ReplaceOneOf {
                candidates: candidates.clone(),
            }),
            key_fragments: vec![collection_name.clone(), element_type.clone()],
        },
        ValidationError::CollectionIndexByFieldMissing {
            collection_name,
            field,
            element_type,
            element_kind,
            candidates,
            candidates_list: _,
        } => DiagnosticPayload {
            code: DiagnosticCode::CollectionIndexByFieldMissing,
            stage: Stage::Validation,
            // `actual` carries the authored field name (the failing
            // reference); resolved element-type + kind ride
            // `key_fragments` for byte-stable test discrimination from
            // sibling cross-doc failures. FixCarriesCandidates over the
            // sorted field name list of the resolved kind.
            expected: None,
            actual: Some(field.clone()),
            fix: Some(Fix::ReplaceOneOf {
                candidates: candidates.clone(),
            }),
            key_fragments: vec![
                collection_name.clone(),
                element_type.clone(),
                element_kind.clone(),
                field.clone(),
            ],
        },
        ValidationError::CollectionMultiWriterWithoutAtomics {
            collection_name,
        } => DiagnosticPayload {
            code: DiagnosticCode::CollectionMultiWriterWithoutAtomics,
            stage: Stage::Validation,
            // No closed candidate set — atomic family is too large
            // (100+ symbols across load/store/cas/fetch × widths ×
            // orderings) for `Fix::ReplaceOneOf` to be useful. Author
            // picks width + ordering + op from the §5.I baseline per
            // their use case. NeutralOrDeterministic.
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![collection_name.clone()],
        },
        // ── §5.L Bounded-collection deploy-time capacity resolution (C6-γ1) ──
        ValidationError::CollectionCapacityUnresolved {
            collection_name,
            key,
            machine,
            limit,
            candidates,
            candidates_list: _,
        } => DiagnosticPayload {
            code: DiagnosticCode::CollectionCapacityUnresolved,
            stage: Stage::Validation,
            // `actual` carries the full authored key so the diagnostic
            // surfaces what was written; resolved machine + limit
            // ride `key_fragments` for byte-stable test discrimination
            // from sibling C6 codes. FixCarriesCandidates over the
            // sorted declared limit names from
            // `target_machine.limits`.
            expected: None,
            actual: Some(key.clone()),
            fix: Some(Fix::ReplaceOneOf {
                candidates: candidates.clone(),
            }),
            key_fragments: vec![
                collection_name.clone(),
                machine.clone(),
                limit.clone(),
            ],
        },
        ValidationError::TimerPeriodBelowTickRate {
            timer_name,
            machine,
            period_us,
            tick_period_us,
        } => DiagnosticPayload {
            code: DiagnosticCode::TimerPeriodBelowTickRate,
            stage: Stage::Validation,
            // Forge-side anchor for spec §5.D line 909. Per-doc check
            // when a Timer doc resolves against a cooperative scheduler.
            // NeutralOrDeterministic — author raises period or lowers
            // tick rate; no closed candidate.
            expected: Some(vec![tick_period_us.to_string()]),
            actual: Some(period_us.to_string()),
            fix: None,
            key_fragments: vec![
                timer_name.clone(),
                machine.clone(),
                period_us.to_string(),
                tick_period_us.to_string(),
            ],
        },
        // ── §5.I `<sce:extern>` whitelist rejection (Atomic A) ──
        ValidationError::ExternSymbolNotInWhitelist {
            name,
            candidates,
            candidates_list: _,
        } => DiagnosticPayload {
            code: DiagnosticCode::ExternSymbolNotInWhitelist,
            stage: Stage::Validation,
            // `actual` carries the offending symbol name; closed-set
            // candidates ride `Fix::ReplaceOneOf` so consumers see
            // closest-match suggestions without paging through 101
            // baseline entries.
            expected: None,
            actual: Some(name.clone()),
            fix: Some(Fix::ReplaceOneOf {
                candidates: candidates.clone(),
            }),
            key_fragments: vec![name.clone()],
        },
        ValidationError::ExternAbiMismatch {
            name,
            expected,
            actual,
        } => DiagnosticPayload {
            code: DiagnosticCode::ExternAbiMismatch,
            stage: Stage::Validation,
            // `expected` carries the registry's canonical ABI; the
            // closed two-element repair set rides `Fix::ReplaceOneOf`
            // so consumers picking via the wire format see both
            // legal values (`["c", "rust"]`) regardless of which one
            // the registry entry expected.
            expected: Some(vec![expected.clone()]),
            actual: Some(actual.clone()),
            fix: Some(Fix::ReplaceOneOf {
                candidates: vec!["c".to_string(), "rust".to_string()],
            }),
            key_fragments: vec![name.clone(), actual.clone()],
        },
        ValidationError::ExternSignatureMismatch {
            name,
            expected,
            actual,
        } => DiagnosticPayload {
            code: DiagnosticCode::ExternSignatureMismatch,
            stage: Stage::Validation,
            // Deterministic fix — the registry holds the canonical
            // sig; `Fix::ReplaceWith` carries it verbatim.
            // NeutralOrDeterministic non_overlap_class.
            expected: Some(vec![expected.clone()]),
            actual: Some(actual.clone()),
            fix: Some(Fix::ReplaceWith {
                to: expected.clone(),
            }),
            key_fragments: vec![name.clone(), actual.clone()],
        },
        ValidationError::ExternOrderingUnspecified {
            base,
            candidates,
            candidates_list: _,
        } => DiagnosticPayload {
            code: DiagnosticCode::ExternOrderingUnspecified,
            stage: Stage::Validation,
            // `actual` carries the suffix-less base name; suffix-
            // bearing completions ride `Fix::ReplaceOneOf`.
            expected: None,
            actual: Some(base.clone()),
            fix: Some(Fix::ReplaceOneOf {
                candidates: candidates.clone(),
            }),
            key_fragments: vec![base.clone()],
        },
        // ── §5.I target-plugin baseline-shadowing (Atomic B) ──
        ValidationError::ExternTargetPluginSymbolConflict { name, plugin_path } => {
            DiagnosticPayload {
                code: DiagnosticCode::ExternTargetPluginSymbolConflict,
                stage: Stage::Validation,
                // `actual` carries the conflicting symbol name; the
                // wire payload also exposes the plugin path through
                // the message body so consumers can surface both
                // axes without parsing the message text.
                expected: None,
                actual: Some(name.clone()),
                // Q-Call-6 (a): repair is "rename the plugin entry";
                // SCE cannot synthesize a non-baseline name. The
                // diagnostic stays advisory (`fix: None`).
                fix: None,
                key_fragments: vec![name.clone(), plugin_path.clone()],
            }
        }
        // ── §5.O Atomic 0 — IR provenance pre-emit guard ─────────
        //    Codegen-internal invariant: an IR node eligible for
        //    SCE-MAP marker emission reached the pre-emit walker
        //    with `source_location: None`. No author repair (the
        //    fix lives in the parser site that produced the node).
        //    `node_kind` + `node_id` ride `key_fragments` so the
        //    wire payload is uniquely keyed per offending parser
        //    site without leaking through `expected` / `actual`
        //    (which carry no useful author-facing data here).
        ValidationError::TraceabilityScxmlLineRangeMissing {
            node_kind,
            node_id,
        } => DiagnosticPayload {
            code: DiagnosticCode::TraceabilityScxmlLineRangeMissing,
            stage: Stage::Generate,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![(*node_kind).into(), node_id.clone()],
        },
        // ── §5.O Atomic 1 — symbol mangling collision detector. The
        //    dual-location payload rides `Fix::ReplaceOneOf` with the
        //    two `<file>:<line>` strings as the closed candidate set:
        //    the author picks whichever site to rename to break the
        //    clash. `actual` carries the colliding mangled symbol so
        //    the diagnostic message is self-describing.
        ValidationError::TraceabilityStateIdCollision {
            mangled,
            first_file,
            first_line,
            second_file,
            second_line,
        } => DiagnosticPayload {
            code: DiagnosticCode::TraceabilityStateIdCollision,
            stage: Stage::Generate,
            expected: None,
            actual: Some(mangled.clone()),
            fix: Some(Fix::ReplaceOneOf {
                candidates: vec![
                    format!("{first_file}:{first_line}"),
                    format!("{second_file}:{second_line}"),
                ],
            }),
            key_fragments: vec![mangled.clone()],
        },
        // ── §5.O Atomic 1 — mangled symbol exceeds C99 identifier
        //    limit. NeutralOrDeterministic: the author has multiple
        //    repair axes (shorten machine id, shorten state id,
        //    shorten artifact suffix, or relax the strict flag) so
        //    no single canonical candidate list.
        ValidationError::TraceabilitySymbolNameExceedsCIdentifierLimit {
            mangled,
            actual_len,
            over_by,
        } => DiagnosticPayload {
            code: DiagnosticCode::TraceabilitySymbolNameExceedsCIdentifierLimit,
            stage: Stage::Generate,
            expected: None,
            actual: Some(mangled.clone()),
            fix: None,
            key_fragments: vec![
                mangled.clone(),
                actual_len.to_string(),
                over_by.to_string(),
            ],
        },
        // ── §5.O Atomic 1 — sourcemap source_hash drift against
        //    §6.2.6 header. NeutralOrDeterministic: regenerate via
        //    `sce-codegen generate` is the only repair.
        ValidationError::TraceabilitySourcemapSourceHashMismatch {
            file,
            sourcemap_hash,
            header_hash,
        } => DiagnosticPayload {
            code: DiagnosticCode::TraceabilitySourcemapSourceHashMismatch,
            stage: Stage::Generate,
            expected: None,
            actual: Some(sourcemap_hash.clone()),
            fix: None,
            key_fragments: vec![
                file.clone(),
                sourcemap_hash.clone(),
                header_hash.clone(),
            ],
        },
        // ── §5.O Atomic 1 — Rust SCE-MAP `#[doc]` preservation guard
        //    (OQ-W16 b). NeutralOrDeterministic: the dual-emit
        //    fallback `// SCE-MAP:` line comment already covers the
        //    strip, so this is a heads-up rather than a hard error.
        ValidationError::TraceabilitySceMapAttributeStripped {
            crate_name,
            function,
            profile,
        } => DiagnosticPayload {
            code: DiagnosticCode::TraceabilitySceMapAttributeStripped,
            stage: Stage::Generate,
            expected: None,
            actual: Some(function.clone()),
            fix: None,
            key_fragments: vec![
                crate_name.clone(),
                function.clone(),
                profile.clone(),
            ],
        },
        // ── §5.O Atomic 1 follow-up — codegen-internal traceability
        //    invariant: SCE-emitted file lacks an SCE-MAP marker.
        //    NeutralOrDeterministic. No author repair — the fix is in
        //    a template `tools/codegen/templates/` upstream.
        ValidationError::TraceabilityMetaGeneratedSourceLineMarkerMissing { file } => {
            DiagnosticPayload {
                code: DiagnosticCode::TraceabilityMetaGeneratedSourceLineMarkerMissing,
                stage: Stage::Generate,
                expected: None,
                actual: Some(file.clone()),
                fix: None,
                key_fragments: vec![file.clone()],
            }
        }
        // ── Round F-α (watching-zenoh RFC §5.2) — `<sce:driver href>`
        //    resolution failure. `actual` carries the verbatim author-
        //    written href so the diagnostic message round-trips the
        //    original string; `key_fragments` include both href and
        //    resolved_dir so two identical-named misses under different
        //    search roots hash distinct wire-ids. Stage = Validation —
        //    the diagnostic fires before codegen, at compile-model
        //    time, matching the §5.O Atomic 1 stage for similar
        //    compile-model-time codes.
        ValidationError::McuDriverHeaderNotFound { href, resolved_dir } => DiagnosticPayload {
            code: DiagnosticCode::McuDriverHeaderNotFound,
            stage: Stage::Validation,
            expected: None,
            actual: Some(href.clone()),
            fix: None,
            key_fragments: vec![href.clone(), resolved_dir.clone()],
        },
    }
}

/// Stable wire-form tag for a [`CallbackPathReason`] arm. Used as a
/// `key_fragments` discriminator so two bad paths with the same
/// `state_id` + `link` + `callback` triple but different reason arms
/// hash to distinct wire ids. (Theoretical — same callback string
/// can only fail one way today — but the discipline keeps id
/// stability robust against future reason-arm growth.)
fn callback_reason_tag(reason: &crate::forge::error::CallbackPathReason) -> &'static str {
    use crate::forge::error::CallbackPathReason;
    match reason {
        CallbackPathReason::EmptyPath => "empty-path",
        CallbackPathReason::UnknownLanguagePrefix { .. } => "unknown-language-prefix",
        CallbackPathReason::MalformedPath => "malformed-path",
        CallbackPathReason::MalformedSegment { .. } => "malformed-segment",
    }
}

/// Stable layer-tag string for [`WorkerSharedStateReason`] used in the
/// `worker/shared-mutable-state` payload's `key_fragments`. Identical
/// rationale to [`callback_reason_tag`]: keeps wire ids distinct when
/// the same worker carries violations from multiple layers, and lets
/// downstream test fixtures key on the layer without parsing the
/// per-instance message. (C2-α has two reachable layers; future C4-
/// composition hardening grows the enum without disturbing existing
/// ids.)
fn worker_shared_state_layer_tag(
    reason: &crate::forge::error::WorkerSharedStateReason,
) -> &'static str {
    use crate::forge::error::WorkerSharedStateReason;
    match reason {
        WorkerSharedStateReason::WorkerImportForbidden { .. } => "worker-import-forbidden",
        WorkerSharedStateReason::BodyForeignNamespace { .. } => "body-foreign-namespace",
    }
}

/// Stable `actual`-field synthesis for [`WorkerSharedStateReason`]:
/// layer 1 surfaces the offending `<sce:import as="X" src="Y" kind="worker"/>`
/// pair; layer 2 surfaces the `element.attr="value"` triple. Used by
/// the payload builder so the wire format carries the most specific
/// fragment the diagnostic could quote.
fn worker_shared_state_actual(
    reason: &crate::forge::error::WorkerSharedStateReason,
) -> String {
    use crate::forge::error::WorkerSharedStateReason;
    match reason {
        WorkerSharedStateReason::WorkerImportForbidden {
            imported_alias,
            imported_src,
        } => format!("<sce:import as=\"{imported_alias}\" src=\"{imported_src}\" kind=\"worker\"/>"),
        WorkerSharedStateReason::BodyForeignNamespace {
            element,
            attr,
            value,
            foreign_prefix: _,
        } => format!("<{element} {attr}=\"{value}\"/>"),
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
        GenerateError::CodegenMcuClassKindOnNonMcuLanguage { kind, language } => DiagnosticPayload {
            code: DiagnosticCode::CodegenMcuClassKindOnNonMcuLanguage,
            stage: Stage::Generate,
            expected: None,
            actual: Some(language.clone()),
            fix: None,
            key_fragments: vec![kind.clone(), language.clone()],
        },
        GenerateError::CodegenGenericKindBackendEmitMissing { kind, language } => DiagnosticPayload {
            code: DiagnosticCode::CodegenGenericKindBackendEmitMissing,
            stage: Stage::Generate,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![kind.clone(), language.clone()],
        },
        // ── Round F-α (watching-zenoh RFC §5.2) — non-MCU backend
        //    refuses `platform.c11_section_attribute` (Q-Round-F-D3).
        //    `actual` carries the offending backend name (`cpp` /
        //    `rust` / `kotlin` / `go` / `python`); `key_fragments`
        //    use the same single value so the wire-id is stable per
        //    backend across runs. Stage = Generate — the reject fires
        //    inside the codegen-matrix walker, matching the existing
        //    Q-Call-7 sibling.
        GenerateError::McuSectionAttributeOnNonMcuTarget { backend } => DiagnosticPayload {
            code: DiagnosticCode::McuSectionAttributeOnNonMcuTarget,
            stage: Stage::Generate,
            expected: None,
            actual: Some(backend.clone()),
            fix: None,
            key_fragments: vec![backend.clone()],
        },
        GenerateError::CodegenNoStdScriptNotSupported { document, locations } => DiagnosticPayload {
            code: DiagnosticCode::CodegenNoStdScriptNotSupported,
            stage: Stage::Generate,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![document.clone(), locations.clone()],
        },
        GenerateError::CodegenNoStdHttpNotSupported { document, locations } => DiagnosticPayload {
            code: DiagnosticCode::CodegenNoStdHttpNotSupported,
            stage: Stage::Generate,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![document.clone(), locations.clone()],
        },
        GenerateError::CodegenNoStdFsLoadNotSupported { document, locations } => DiagnosticPayload {
            code: DiagnosticCode::CodegenNoStdFsLoadNotSupported,
            stage: Stage::Generate,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![document.clone(), locations.clone()],
        },
        GenerateError::CodegenNoStdInvokeNotSupported { document, locations } => DiagnosticPayload {
            code: DiagnosticCode::CodegenNoStdInvokeNotSupported,
            stage: Stage::Generate,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![document.clone(), locations.clone()],
        },
        GenerateError::ConstNotFoldable {
            algorithm,
            const_name,
            detail,
        } => DiagnosticPayload {
            code: DiagnosticCode::AlgorithmConstNotFoldable,
            stage: Stage::Generate,
            expected: None,
            actual: Some(const_name.clone()),
            fix: None,
            key_fragments: vec![algorithm.clone(), const_name.clone(), detail.clone()],
        },
        GenerateError::ConstFoldBudgetExceeded {
            algorithm,
            const_name,
            budget,
        } => DiagnosticPayload {
            code: DiagnosticCode::AlgorithmConstFoldBudgetExceeded,
            stage: Stage::Generate,
            expected: None,
            actual: const_name.clone(),
            fix: None,
            key_fragments: {
                let mut k = vec![algorithm.clone()];
                if let Some(n) = const_name {
                    k.push(n.clone());
                }
                k.push(budget.to_string());
                k
            },
        },
        GenerateError::ConstYieldTypeMismatch {
            algorithm,
            const_name,
            expected,
            actual,
        } => DiagnosticPayload {
            code: DiagnosticCode::AlgorithmConstYieldTypeMismatch,
            stage: Stage::Generate,
            // The declared element / scalar type rides `key_fragments`
            // rather than the wire `expected` field — `expected` is
            // reserved for the `ExpectedIsMetadata` bucket
            // (`expression/parse-mismatch`-style parser expectations);
            // type-coercion failures sit in the `NeutralOrDeterministic`
            // bucket alongside `validation/numeric-parse`.
            expected: None,
            actual: Some(actual.clone()),
            fix: None,
            key_fragments: vec![
                algorithm.clone(),
                const_name.clone(),
                format!("{expected:?}"),
                actual.clone(),
            ],
        },
    }
}

/// SCXML semantic-validation field mapping (RFC §W5 D2).
///
/// Three of the four variants reuse existing `validation/*` wire codes
/// per the W4 D4 fold precedent — concept identity over namespace
/// duplication. Only `TopLevelScriptUnloaded` introduces a NEW wire
/// code (`scxml/top-level-script-unloaded`) because W3C SCXML §5.8
/// has no forge analog.
///
/// Stage stays `Stage::Validation` for all four (RFC §W5 D2 reverse-
/// reverse-default): SCXML semantic-validation IS post-parse semantic
/// validation, the same analytical stage as forge `validation/*`. A
/// future production consumer that needs separate-stage routing can
/// drive the addition of `Stage::ScxmlSemantic` then; pre-emptive
/// addition violates `feedback_built_but_unconsumed.md`.
fn scxml_semantic_fields(e: &crate::scxml_semantic::ScxmlSemanticError) -> DiagnosticPayload {
    use crate::scxml_semantic::{InitialStateScope, ScxmlSemanticError};
    match e {
        ScxmlSemanticError::InitialStateUnknown {
            state_id,
            scope,
            available,
        } => DiagnosticPayload {
            // REUSE — same wire code as forge `ValidationError::InvalidReference`.
            // Concept identity: "name X did not resolve to declared symbol Y".
            code: DiagnosticCode::ValidationInvalidReference,
            stage: Stage::Validation,
            expected: None,
            actual: Some(state_id.clone()),
            fix: if available.is_empty() {
                None
            } else {
                Some(Fix::ReplaceOneOf {
                    candidates: available.clone(),
                })
            },
            // key_fragments: scope-string + "state" + state_id keeps
            // root-vs-compound distinction in the content-hash id so
            // two different sites referencing the same bad id yield
            // distinct fnv1a ids. Mirrors forge `InvalidReference`'s
            // `[kind, what, name]` layout.
            key_fragments: vec![
                match scope {
                    InitialStateScope::DocumentRoot => "scxml-root".to_string(),
                    InitialStateScope::CompoundState { parent_id } => {
                        format!("scxml-compound:{parent_id}")
                    }
                },
                "initial-state".to_string(),
                state_id.clone(),
            ],
        },
        ScxmlSemanticError::TransitionTargetUnknown {
            state,
            target,
            available,
        } => DiagnosticPayload {
            code: DiagnosticCode::ValidationInvalidReference,
            stage: Stage::Validation,
            expected: None,
            actual: Some(target.clone()),
            fix: if available.is_empty() {
                None
            } else {
                Some(Fix::ReplaceOneOf {
                    candidates: available.clone(),
                })
            },
            // The owning state goes into `key_fragments[0]` so a
            // document with the same bad target appearing in two
            // different states yields two distinct ids.
            key_fragments: vec![
                format!("scxml-state:{state}"),
                "transition-target".to_string(),
                target.clone(),
            ],
        },
        ScxmlSemanticError::NoStates => DiagnosticPayload {
            // REUSE — same wire code as forge `ValidationError::EmptyCollection`.
            // Concept identity: "kind requires at least one X".
            code: DiagnosticCode::ValidationEmptyCollection,
            stage: Stage::Validation,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec!["scxml".to_string(), "state".to_string()],
        },
        ScxmlSemanticError::TopLevelScriptUnloaded { index, src } => DiagnosticPayload {
            // NEW — W3C SCXML §5.8 has no forge analog. The 1 NEW
            // wire code RFC §W5 D2 introduces.
            code: DiagnosticCode::ScxmlTopLevelScriptUnloaded,
            stage: Stage::Validation,
            expected: None,
            // `actual` carries the failing src (when known) so repair
            // tools can locate the offending element. Empty when
            // analyzer.rs path emits without parser-captured detail.
            actual: src.clone(),
            fix: None,
            // Index + src in key_fragments keep two failing scripts
            // in the same document distinguishable in the content-hash
            // id. Both can be empty (analyzer path).
            key_fragments: {
                let mut k = Vec::new();
                if let Some(i) = index {
                    k.push(i.to_string());
                }
                if let Some(s) = src {
                    k.push(s.clone());
                }
                k
            },
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
        use crate::forge::error::{
            CallbackPathReason, ExprError, GenerateError, ImportError, ManifestError,
            WorkerSharedStateReason, XmlError,
        };
        vec![
            (
                "forge/xml-parse",
                ForgeError::Xml(XmlError::Parse("unexpected end tag </scxml>".into())),
                r#"{"v":1,"id":"fnv1a:16e2e2901e2b9b96","code":"xml/parse","stage":"xml","message":"XML parse error: unexpected end tag </scxml>"}"#,
            ),
            (
                "forge/xml-file-not-found",
                ForgeError::Xml(XmlError::FileNotFound {
                    path: "/nonexistent/path.scxml".into(),
                }),
                r#"{"v":1,"id":"fnv1a:b4a2c55cf61bb593","code":"xml/file-not-found","stage":"xml","message":"SCXML file not found: /nonexistent/path.scxml","actual":"/nonexistent/path.scxml"}"#,
            ),
            (
                "forge/xml-wrong-root-element",
                ForgeError::Xml(XmlError::WrongRootElement {
                    found: "html".into(),
                }),
                r#"{"v":1,"id":"fnv1a:648ce2d13ccab5eb","code":"xml/wrong-root-element","stage":"xml","message":"Root element is not <scxml>, found: <html>","expected":["scxml"],"actual":"html"}"#,
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
                r#"{"v":1,"id":"fnv1a:812898e1a23fda4d","code":"validation/unsupported-kind","stage":"validation","spec":"SCE Forge §3.2","message":"unsupported sce:kind value: 'bogus'","actual":"bogus","fix":{"kind":"replace_one_of","candidates":["statechart","transform","lookup","condition","codec","procedure","validator","filter","interpolation","timer","observer","algorithm","link","buffer-pool","worker","bounded-collection"]}}"#,
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
                "forge/bytes-max-size-violation",
                ValidationError::BytesMaxSizeViolation {
                    procedure: "security_access".into(),
                    detail: "<send sce:service=\"SecurityAccess\"> sce:response-max-size=128 exceeds destination slot 'seed' sce:max-size=64".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:45992402abee5c5d","code":"validation/bytes-max-size-violation","stage":"validation","message":"security_access: <send sce:service=\"SecurityAccess\"> sce:response-max-size=128 exceeds destination slot 'seed' sce:max-size=64"}"#,
            ),
            (
                // RFC §W5: SCXML semantic family — TopLevelScriptUnloaded
                // is the 1 NEW wire code (others reuse `validation/*`).
                // Golden uses the parser-path shape (index + src
                // populated) so the wire payload exercises both
                // optional fields. Analyzer-path emits with both
                // None — covered by the unit test in scxml_semantic.rs.
                "forge/scxml-top-level-script-unloaded",
                crate::scxml_semantic::ScxmlSemanticError::TopLevelScriptUnloaded {
                    index: Some(2),
                    src: Some("init.js".into()),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:60cc8f4eef6d11ca","code":"scxml/top-level-script-unloaded","stage":"validation","spec":"W3C SCXML §5.8","message":"Top-level <script> rejected per W3C SCXML 5.8","actual":"init.js"}"#,
            ),
            (
                // watching-zenoh RFC §5.E B7-η' Q-OnSample-2 (a)
                "forge/scxml-on-sample-invalid-parent",
                ValidationError::OnSampleInvalidParent {
                    path: "scxml > onentry".into(),
                    actual_parent: "onentry".into(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:90051ac7a8571e3a","code":"scxml/on-sample-invalid-parent","stage":"validation","spec":"watching-zenoh RFC §5.E","message":"<sce:on-sample> at scxml > onentry: must appear directly inside a <state> or <parallel>; found inside <onentry>. Move the element under a state or parallel ancestor.","actual":"onentry"}"#,
            ),
            (
                // watching-zenoh RFC §5.E B7-η' Q-OnSample-5 (a)
                "forge/scxml-on-sample-link-duplicate-in-state",
                ValidationError::OnSampleLinkDuplicateInState {
                    state_id: "running".into(),
                    link: "scout_link".into(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:ab4ec9e9ef9663b9","code":"scxml/on-sample-link-duplicate-in-state","stage":"validation","spec":"watching-zenoh RFC §5.E","message":"state 'running': duplicate <sce:on-sample link=\"scout_link\"> declarations. Each link is allowed at most one on-sample block per state; merge the duplicates or rename one of the link references.","actual":"scout_link"}"#,
            ),
            (
                // watching-zenoh RFC §5.E B7-η' Q-OnSample-7
                "forge/scxml-on-sample-event-name-conflict",
                ValidationError::OnSampleEventNameConflict {
                    event: "error.io".into(),
                    reserved_prefix: "error.".into(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:3b804e5f18b5b65a","code":"scxml/on-sample-event-name-conflict","stage":"validation","spec":"watching-zenoh RFC §5.E","message":"<sce:on-sample event=\"error.io\"> collides with the reserved W3C SCXML internal event prefix 'error.'. Pick an event name outside that family (e.g. 'sample.error.io') so dispatched samples stay distinct from built-in lifecycle events.","actual":"error.io"}"#,
            ),
            (
                // watching-zenoh RFC §5.E B7-η' Atomic B Q-OnSample-3
                // cross-ref pair — `not-declared` is reachable today.
                "forge/scxml-on-sample-link-not-declared",
                ValidationError::OnSampleLinkNotDeclared {
                    state_id: "running".into(),
                    link: "scout_link".into(),
                    candidates: vec!["status_link".into()],
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:b635927d2fa69152","code":"scxml/on-sample-link-not-declared","stage":"validation","spec":"watching-zenoh RFC §5.E","message":"state 'running': <sce:on-sample link=\"scout_link\"> references a name that no `.forge` file in the build declares as a link kind. Add a forge `<scxml sce:kind=\"link\" name=\"scout_link\">` document or fix the reference. See watching-zenoh RFC §5.E.","actual":"scout_link","fix":{"kind":"replace_one_of","candidates":["status_link"]}}"#,
            ),
            (
                // `wrong-kind` — forward-compat per stage_pool precedent.
                // Wired through full sync; unreachable in production
                // until a future cross-registry generalization grows
                // `ScxmlDocKind` with non-Link variants.
                "forge/scxml-on-sample-link-wrong-kind",
                ValidationError::OnSampleLinkWrongKind {
                    state_id: "running".into(),
                    link: "scout_codec".into(),
                    actual_kind: "codec".into(),
                    candidates: vec!["scout_link".into()],
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:38b424712554fe91","code":"scxml/on-sample-link-wrong-kind","stage":"validation","spec":"watching-zenoh RFC §5.E","message":"state 'running': <sce:on-sample link=\"scout_codec\"> resolves to a forge 'codec' kind, not 'link'. Only link kind documents back the on-sample subscriber contract. Repoint the reference at one of the build's link kind names. See watching-zenoh RFC §5.E.","actual":"codec","fix":{"kind":"replace_one_of","candidates":["scout_link"]}}"#,
            ),
            (
                // RFC §5.E B7-η' Atomic A1: on-sample subscriber on a
                // link whose forge document has no `<sce:stage-pool>` —
                // `Sample::take()` would route to the runtime's
                // `PanicOnTakeHook` default. Candidates pull from the
                // build's `ForgePoolRegistry` buffer-pool kind names.
                "forge/pool-sample-take-without-stage-pool",
                ValidationError::PoolSampleTakeWithoutStagePool {
                    state_id: "running".into(),
                    link: "scout_link".into(),
                    candidates: vec!["scout_stage_pool".into()],
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:1c55aabc0ddc8d36","code":"pool/sample-take-without-stage-pool","stage":"validation","spec":"watching-zenoh RFC §5.E","message":"state 'running': <sce:on-sample link=\"scout_link\"> targets a link kind whose forge document does not declare a `<sce:stage-pool>` element. Subscriber callbacks on this link cannot escape the borrow lifetime via `Sample::take()` because there is no stage-copy destination. Add `<sce:stage-pool ref=\"...\">` to the link's `.forge` document or restrict callbacks to borrow-only access. See watching-zenoh RFC §5.E.","actual":"scout_link","fix":{"kind":"replace_one_of","candidates":["scout_stage_pool"]}}"#,
            ),
            (
                // RFC §5.E B7-η' Atomic A2 callback-path syntax: an
                // `<sce:on-sample callback="rust:...">` value fails the
                // Q-Callback-3 Rust path subset. Today's reachable arms
                // are syntax failures (UnknownLanguagePrefix shown here);
                // future signature inspection extends the same code with
                // shape-mismatch arms. NeutralOrDeterministic non_overlap
                // class — no `Fix::ReplaceOneOf` surface (free-form
                // path; closed candidate set doesn't apply).
                "forge/pool-sample-callback-signature-non-borrow",
                ValidationError::PoolSampleCallbackSignatureNonBorrow {
                    state_id: "running".into(),
                    link: "scout_link".into(),
                    callback: "cpp:my_app::on_scout".into(),
                    reason: CallbackPathReason::UnknownLanguagePrefix {
                        prefix: "cpp".into(),
                    },
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:c1db04cc9e2b921b","code":"pool/sample-callback-signature-non-borrow","stage":"validation","spec":"watching-zenoh RFC §5.E","message":"state 'running': <sce:on-sample link=\"scout_link\" callback=\"cpp:my_app::on_scout\"> uses an unsupported language prefix `cpp` (only `rust:` is accepted today). The `callback` value must match `rust:crate::module::fn` (Q-Callback-3 Rust path subset). The borrow-mode contract is enforced at the dispatch site; rustc rejects owned-mode signatures at user-crate compile time. See watching-zenoh RFC §5.E.","actual":"cpp:my_app::on_scout"}"#,
            ),
            (
                // RFC §5.D line 911 C2-α: worker shared-state encapsulation.
                // Layer 1 reachable today (`<sce:import kind="worker">` is
                // the structural author error a parse-time guard catches).
                // Layer 2 (body SCXML cross-namespace data-refs) lands with
                // the same code in this atomic. Layer 3 (`<sce:extern>`
                // non-inbox symbol use in body) defers to a tracked
                // follow-up atomic gated on C4 intrinsic-registry
                // composition surface. NeutralOrDeterministic non_overlap
                // class — no `Fix::ReplaceOneOf` surface (the offending
                // path may be removed, refactored through the inbox, or
                // replaced with an `<sce:outbox>` ref; SCE cannot
                // synthesize the choice).
                "forge/worker-shared-mutable-state",
                ValidationError::WorkerSharedMutableState {
                    worker_name: "rx_loop".into(),
                    reason: WorkerSharedStateReason::WorkerImportForbidden {
                        imported_alias: "tx_loop".into(),
                        imported_src: "tx_loop.scxml".into(),
                    },
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:f054d112eea16560","code":"worker/shared-mutable-state","stage":"validation","spec":"watching-zenoh RFC §5.D","message":"worker 'rx_loop': declares <sce:import as=\"tx_loop\" src=\"tx_loop.scxml\" kind=\"worker\"/>; workers cannot import other worker kinds. Workers must communicate with other workers only through their own inbox (consume) and the recipient's inbox via <sce:outbox ref=\"...\"> (produce); all other paths to another worker's state are forbidden per RFC §5.D line 911 (\"any non-inbox access to another worker's state\").","actual":"<sce:import as=\"tx_loop\" src=\"tx_loop.scxml\" kind=\"worker\"/>"}"#,
            ),
            // ── §5.D C2-β worker cross-resolution: link-rx + outbox ref ──
            (
                // RFC §5.D C2-β: `<sce:link-rx ref="X">` must reference
                // an alias imported as `kind="link"`. Mirrors η-precedent
                // `validate_link_pool_framer_resolution` resolution shape.
                // FixCarriesCandidates with the sorted kind=link import
                // alias set on this worker doc.
                "forge/worker-link-rx-ref-unknown",
                ValidationError::WorkerLinkRxRefUnknown {
                    worker_name: "rx_loop".into(),
                    ref_name: "udp_scout".into(),
                    candidates: vec!["status_link".into()],
                    candidates_list: "status_link".into(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:60c2fe22d1085b50","code":"worker/link-rx-ref-unknown","stage":"validation","spec":"watching-zenoh RFC §5.D","message":"worker 'rx_loop': <sce:link-rx ref=\"udp_scout\"> references a name that is not imported as a link kind. Declare the link via <sce:import as=\"udp_scout\" src=\"...\" kind=\"link\"/> on this worker document, or replace the ref with one of the imported link-kind aliases (closest matches: status_link).","actual":"udp_scout","fix":{"kind":"replace_one_of","candidates":["status_link"]}}"#,
            ),
            // ── §5.I C2-β SPSC inbox ordering ──
            (
                // RFC §5.I lines 1757-1758 C2-β: `<sce:inbox>` declared
                // without an `ordering` attribute. SCE's error-only wire
                // realizes the spec "warning" as a required-when-worker-
                // exists error so authors get a load-bearing choice
                // before codegen emits ambiguous atomic ops.
                "forge/worker-inbox-ordering-unspecified",
                ValidationError::WorkerInboxOrderingUnspecified {
                    worker_name: "rx_loop".into(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:066432600b5950a2","code":"worker/inbox-ordering-unspecified","stage":"validation","spec":"watching-zenoh RFC §5.I","message":"worker 'rx_loop': <sce:inbox> declared without an `ordering` attribute. Pick `ordering=\"acq_rel\"` (safe default; producer and consumer pair head/tail with acquire+release on every push/pop) or `ordering=\"relaxed\"` (single-core fast-path; cross-core placement raises `worker/inbox-ordering-relaxed-across-cores`). Spec §5.I line 1752-1758 mandates one of these two for every SPSC inbox."}"#,
            ),
            (
                // RFC §5.I lines 1755-1756 C2-β: codegen-invariant
                // guard. Silent-skip when `ForgeCompileOptions.worker_
                // placement` is `None` (deploy-unaware path); fires
                // only when explicit cross-core placement coexists with
                // `relaxed` ordering. NeutralOrDeterministic — both
                // repair axes (flip ordering or co-locate) are author
                // judgment, not a closed candidate.
                "forge/worker-inbox-ordering-relaxed-across-cores",
                ValidationError::WorkerInboxOrderingRelaxedAcrossCores {
                    worker_name: "rx_loop".into(),
                    producer_core: 0,
                    consumer_core: 1,
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:b605193320262358","code":"worker/inbox-ordering-relaxed-across-cores","stage":"validation","spec":"watching-zenoh RFC §5.I","message":"worker 'rx_loop': <sce:inbox ordering=\"relaxed\"> declared but deploy.placement pins producer on core 0 and consumer on core 1. Cross-core SPSC inboxes require acquire/release pairing on head/tail (per spec §5.I lines 1752-1758). Replace with `ordering=\"acq_rel\"` or co-locate producer + consumer on the same core via deploy.placement.","actual":"relaxed"}"#,
            ),
            (
                // RFC §5.D line 912 C2-γ: forge-side anchor for scheduler
                // capacity violations. Fires when a Worker doc compiles
                // against a machine that did not list it in
                // `machines.<m>.workers`. NeutralOrDeterministic — author
                // adds the worker to deploy.yaml or removes the Worker
                // doc; no closed candidate set.
                "forge/worker-scheduler-unsupported",
                ValidationError::WorkerSchedulerUnsupported {
                    worker_name: "rx_loop".into(),
                    machine: "mcu_node".into(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:39f753b9c4918241","code":"worker/scheduler-unsupported","stage":"validation","spec":"watching-zenoh RFC §5.D","message":"worker 'rx_loop': not declared in deploy.yaml under `machines.mcu_node.workers`. watching-zenoh RFC §5.D line 912 (`worker/scheduler-unsupported`) — the cooperative scheduler tracks one tick slot per declared worker; an undeclared worker has no slot. Repair: add `rx_loop:` under `machines.mcu_node.workers:` in deploy.yaml, or remove the Worker doc from the build.","actual":"rx_loop"}"#,
            ),
            // ── §5.D C2 follow-up Atomic B worker outbox cross-resolution ──
            (
                // RFC §5.D C2 follow-up Atomic B: owner segment not in
                // SceCrossDocRegistry. FixCarriesCandidates — sorted
                // statechart + worker `.inbox` set rides Fix::ReplaceOneOf.
                "forge/worker-outbox-ref-unknown",
                ValidationError::WorkerOutboxRefUnknown {
                    worker_name: "rx_loop".into(),
                    outbox_value: "sesion_fsm.inbox".into(),
                    owner: "sesion_fsm".into(),
                    candidates: vec![
                        "session_fsm.inbox".into(),
                        "tx_loop.inbox".into(),
                    ],
                    candidates_list:
                        "session_fsm.inbox, tx_loop.inbox".into(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:99378501ddb9866e","code":"worker/outbox-ref-unknown","stage":"validation","spec":"watching-zenoh RFC §5.D","message":"worker 'rx_loop': <sce:outbox ref=\"sesion_fsm.inbox\"> names owner 'sesion_fsm' which is not a registered statechart or worker. Declare the recipient as a separate `.scxml` document in this build (statechart: `<scxml name=\"sesion_fsm\">`; worker: `<scxml sce:kind=\"worker\" name=\"sesion_fsm\">`), or replace the ref with one of the registered recipients: session_fsm.inbox, tx_loop.inbox.","actual":"sesion_fsm.inbox","fix":{"kind":"replace_one_of","candidates":["session_fsm.inbox","tx_loop.inbox"]}}"#,
            ),
            (
                // RFC §5.D C2 follow-up Atomic B: owner found but kind not
                // in {statechart, worker} — today's only other kind in the
                // registry is `link`. FixCarriesCandidates — same sorted
                // union shape as outbox-ref-unknown.
                "forge/worker-outbox-target-wrong-kind",
                ValidationError::WorkerOutboxTargetWrongKind {
                    worker_name: "rx_loop".into(),
                    outbox_value: "udp_scout.inbox".into(),
                    owner: "udp_scout".into(),
                    actual_kind: "link".into(),
                    candidates: vec![
                        "session_fsm.inbox".into(),
                        "tx_loop.inbox".into(),
                    ],
                    candidates_list:
                        "session_fsm.inbox, tx_loop.inbox".into(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:1efb5e754cd03d3d","code":"worker/outbox-target-wrong-kind","stage":"validation","spec":"watching-zenoh RFC §5.D","message":"worker 'rx_loop': <sce:outbox ref=\"udp_scout.inbox\"> names 'udp_scout' which is registered as a link kind, not a statechart or worker. Outbox refs may only target statechart or worker inboxes (RFC §5.D line 911 \"any non-inbox access\" by negation admits inbox access on statechart + worker kinds). Replace with one of: session_fsm.inbox, tx_loop.inbox.","actual":"udp_scout.inbox","fix":{"kind":"replace_one_of","candidates":["session_fsm.inbox","tx_loop.inbox"]}}"#,
            ),
            (
                // RFC §5.D C2 follow-up Atomic B: suffix !=  `inbox` per
                // Q-Outbox-6 (a) strict-suffix lock. Deterministic
                // single-value Fix::ReplaceWith.
                // NeutralOrDeterministic non_overlap_class.
                "forge/worker-outbox-target-suffix-invalid",
                ValidationError::WorkerOutboxTargetSuffixInvalid {
                    worker_name: "rx_loop".into(),
                    outbox_value: "session_fsm.inbx".into(),
                    owner: "session_fsm".into(),
                    suffix: "inbx".into(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:a8a2cabe814a473a","code":"worker/outbox-target-suffix-invalid","stage":"validation","spec":"watching-zenoh RFC §5.D","message":"worker 'rx_loop': <sce:outbox ref=\"session_fsm.inbx\"> declares suffix 'inbx' but the only legal suffix is 'inbox' (RFC §5.D line 895 example: `<owner>.inbox`; spec line 1998 codegen table fixes the recipient queue name to `inbox`). Replace with `session_fsm.inbox`.","actual":"session_fsm.inbx","fix":{"kind":"replace_with","to":"session_fsm.inbox"}}"#,
            ),
            (
                // RFC §5.D line 909 C1: forge-side anchor. Per-doc
                // check fires when a Timer doc resolves against a
                // cooperative-scheduler machine whose
                // `tick_period_us` exceeds the doc's `period_us`.
                "forge/timer-period-below-tick-rate",
                ValidationError::TimerPeriodBelowTickRate {
                    timer_name: "keepalive".into(),
                    machine: "mcu_node".into(),
                    period_us: 500,
                    tick_period_us: 1000,
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:a84c33faf2495d72","code":"timer/period-below-tick-rate","stage":"validation","spec":"watching-zenoh RFC §5.D","message":"timer 'keepalive': <sce:period> = 500 us is shorter than scheduler.tick_period_us = 1000 us on machine 'mcu_node'. watching-zenoh RFC §5.D line 909 (`timer/period-below-tick-rate`) — the cooperative scheduler dispatches at most one timer per tick, so a period below the tick rate would miss every other deadline. Repair: raise `<sce:period>` to >= 1000us, or lower `scheduler.tick_period_us` (warning: lowering tick rate increases scheduler overhead), or switch the target machine to `scheduler.kind: tokio` / `rt` (preemptive).","expected":["1000"],"actual":"500"}"#,
            ),
            // ── §5.L Bounded-collection parse-time structure validators (C6-α) ──
            (
                // RFC §5.L line 2559: sorted-by ordering with no
                // <sce:index-by> field — codegen has no comparator
                // to lower. NeutralOrDeterministic, no Fix payload.
                "forge/collection-ordering-sorted-requires-index-by",
                ValidationError::CollectionOrderingSortedRequiresIndexBy {
                    collection_name: "local_sub_table".into(),
                }
                .into(),
                // Hash placeholder — byte-stability assertion patches.
                r#"{"v":1,"id":"fnv1a:e42ff124e4f1930b","code":"collection/ordering-sorted-requires-index-by","stage":"validation","spec":"watching-zenoh RFC §5.L","message":"bounded-collection 'local_sub_table': <sce:ordering>sorted-by(index-by)</sce:ordering> declared without <sce:index-by field=\"...\"/>. watching-zenoh RFC §5.L line 2559 fixes sorted iteration to the `index-by` field; without it the codegen has no comparator to lower. Repair: add an `<sce:index-by field=\"FIELD\"/>` element naming a field of the element-type struct, or change `<sce:ordering>` to `insertion`."}"#,
            ),
            (
                // RFC §5.L line 2655: oldest-wins policy paired with
                // sorted-by ordering — "oldest" has no meaning when
                // iteration order is comparator-derived. Two equally
                // valid repairs → NeutralOrDeterministic, no Fix.
                "forge/collection-overflow-policy-oldest-wins-requires-ordering-insertion",
                ValidationError::CollectionOverflowPolicyOldestWinsRequiresOrderingInsertion {
                    collection_name: "local_sub_table".into(),
                }
                .into(),
                // Hash placeholder — byte-stability assertion patches.
                r#"{"v":1,"id":"fnv1a:31723798e4ccd410","code":"collection/overflow-policy-oldest-wins-requires-ordering-insertion","stage":"validation","spec":"watching-zenoh RFC §5.L","message":"bounded-collection 'local_sub_table': <sce:on-overflow>oldest-wins</sce:on-overflow> requires <sce:ordering>insertion</sce:ordering>, but ordering is `sorted-by(index-by)`. watching-zenoh RFC §5.L line 2655 lists this combination as the explicit anti-pattern: `oldest-wins` presumes a temporal ordering that `sorted-by` replaces with the `index-by` field comparator. Repair: change `<sce:ordering>` to `insertion` (keeps the oldest-wins policy), or change `<sce:on-overflow>` to `reject` / `diagnostic-event`."}"#,
            ),
            // ── §5.M Fragment-reassembly variant parse-time structure validators (C9-α) ──
            (
                // RFC §5.M line 2944: <sce:variant>reassembly with no
                // <sce:max-fragments-per-message> sibling — codegen has
                // no fragment-index bitmap width to lower per spec line
                // 2688. NeutralOrDeterministic, no Fix payload.
                "forge/mem-reassembly-pool-variant-missing-max-fragments",
                ValidationError::MemReassemblyPoolVariantMissingMaxFragments {
                    pool_name: "rx_reassembly_pool".into(),
                }
                .into(),
                // Hash placeholder — byte-stability assertion patches.
                r#"{"v":1,"id":"fnv1a:02053e3d69a3f3f4","code":"mem/reassembly-pool-variant-missing-max-fragments","stage":"validation","spec":"watching-zenoh RFC §5.M","message":"buffer-pool 'rx_reassembly_pool': <sce:variant>reassembly</sce:variant> declared without <sce:max-fragments-per-message>N</sce:max-fragments-per-message>. watching-zenoh RFC §5.M line 2688 fixes the per-slot fragment-index bitmap width to this value; without it codegen has no upper bound on the per-slot fragment-ID tracking. Repair: add an `<sce:max-fragments-per-message>N</sce:max-fragments-per-message>` element with a positive integer N derived from the wire framer's per-message maximum."}"#,
            ),
            (
                // RFC §5.M line 2945: <sce:variant>reassembly with no
                // <sce:reassembly-timeout-ms> sibling — the reassembly
                // FSM has no Receiving → TimedOut edge timer per spec
                // line 2689. NeutralOrDeterministic, no Fix payload.
                "forge/mem-reassembly-pool-variant-missing-timeout",
                ValidationError::MemReassemblyPoolVariantMissingTimeout {
                    pool_name: "rx_reassembly_pool".into(),
                }
                .into(),
                // Hash placeholder — byte-stability assertion patches.
                r#"{"v":1,"id":"fnv1a:1c6a61294467efab","code":"mem/reassembly-pool-variant-missing-timeout","stage":"validation","spec":"watching-zenoh RFC §5.M","message":"buffer-pool 'rx_reassembly_pool': <sce:variant>reassembly</sce:variant> declared without <sce:reassembly-timeout-ms>N</sce:reassembly-timeout-ms>. watching-zenoh RFC §5.M line 2689 fixes the per-slot deadline field to this value; without it the reassembly FSM has no `Receiving → TimedOut` edge timer (`docs/reassembly-fsm.md` §2.4.5). Repair: add an `<sce:reassembly-timeout-ms>N</sce:reassembly-timeout-ms>` element with a positive integer N (milliseconds) derived from link latency budget and acceptable hold time."}"#,
            ),
            // ── §5.M Fragment-reassembly cross-doc validators (C13-α-2 + C9-β) ──
            (
                // RFC §5.M line 2946: rx-pool slot_size < link mtu_bytes.
                // NeutralOrDeterministic, no Fix payload (multi-axis
                // repair: raise slot_size, lower mtu, bind different pool).
                "forge/mem-reassembly-slot-size-below-declared-mtu",
                ValidationError::MemReassemblySlotSizeBelowDeclaredMtu {
                    pool_name: "rx_data_pool".into(),
                    slot_size: 256,
                    mtu_bytes: 512,
                    machine: "mcu_node".into(),
                    link_name: "udp_data".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:eba6a0d43209c75a","code":"mem/reassembly-slot-size-below-declared-mtu","stage":"validation","spec":"watching-zenoh RFC §5.M","message":"buffer-pool 'rx_data_pool' is bound as RX pool for link 'udp_data' on machine 'mcu_node', but `<sce:slot-size>256</sce:slot-size>` is smaller than the link's `mtu_bytes: 512`. watching-zenoh RFC §5.M line 2946 — the slot cannot admit a single full-MTU datagram, so even the non-fragmented happy path fails. Repair: raise `<sce:slot-size>` on pool 'rx_data_pool' to >= 512, lower `mtu_bytes` on link 'udp_data', or bind a different (larger) pool.","expected":["512"],"actual":"256"}"#,
            ),
            (
                // RFC §5.M line 2947-2949: reassembly slot_size < max-fragments × mtu.
                // Hard error; multi-axis repair (raise slot_size / lower
                // max-fragments / lower mtu). NeutralOrDeterministic.
                "forge/reassembly-max-fragments-insufficient-for-mtu",
                ValidationError::ReassemblyMaxFragmentsInsufficientForMtu {
                    pool_name: "rx_reassembly_pool".into(),
                    slot_size: 1024,
                    max_fragments_per_message: 8,
                    mtu_bytes: 512,
                    required: 4096,
                    machine: "mcu_node".into(),
                    link_name: "udp_data".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:3460f145fdcb9f41","code":"reassembly/max-fragments-insufficient-for-mtu","stage":"validation","spec":"watching-zenoh RFC §5.M","message":"reassembly-variant buffer-pool 'rx_reassembly_pool' is bound to link 'udp_data' on machine 'mcu_node', but `<sce:slot-size>1024</sce:slot-size>` cannot hold the worst-case reassembled message: `<sce:max-fragments-per-message>8</sce:max-fragments-per-message> × link.mtu_bytes (512) = 4096` bytes required. watching-zenoh RFC §5.M line 2947-2949 verbatim: `slot_size >= max-fragments-per-message × mtu_bytes` — worst-case message must complete reassembly within declared bounds. Repair: raise `<sce:slot-size>` on pool 'rx_reassembly_pool' to >= 4096, lower `<sce:max-fragments-per-message>`, or lower link `mtu_bytes`.","expected":["4096"],"actual":"1024"}"#,
            ),
            (
                // RFC §5.M line 2950-2952: expected_p99 vs rx_pool slot_size
                // implies > 25% stage-copy rate. Warning (multi-axis repair).
                // NeutralOrDeterministic.
                "forge/reassembly-expected-fragmentation-rate-high",
                ValidationError::ReassemblyExpectedFragmentationRateHigh {
                    pool_name: "rx_data_pool".into(),
                    slot_size: 700,
                    expected_p99_bytes: 1024,
                    rate_percent: 31,
                    machine: "mcu_node".into(),
                    link_name: "udp_data".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:79cd242c82f712e0","code":"reassembly/expected-fragmentation-rate-high","stage":"validation","spec":"watching-zenoh RFC §5.M","message":"link 'udp_data' on machine 'mcu_node': `expected_p99_bytes: 1024` exceeds RX pool 'rx_data_pool' `<sce:slot-size>700</sce:slot-size>` by more than the 25% default stage-copy threshold (rate = 31%). watching-zenoh RFC §5.M line 2950-2952 — `(expected_p99_bytes - rx_pool.slot_size) / expected_p99_bytes > 0.25` triggers the warning. Repair: raise `<sce:slot-size>` on pool 'rx_data_pool', lower `expected_p99_bytes` (with justification), or add `<sce:accept-stage-copy-rate>` on the link source (C13-γ scope).","expected":["25"],"actual":"31"}"#,
            ),
            (
                // RFC §5.M line 2964-2969: reassembly pool bound to a
                // link with trust_class != established_session. Hard error.
                // NeutralOrDeterministic (per Q-C13-α2-7 a — two valid
                // repairs: change trust_class OR remove binding).
                "forge/reassembly-untrusted-link-binding",
                ValidationError::ReassemblyUntrustedLinkBinding {
                    pool_name: "rx_reassembly_pool".into(),
                    trust_class: "session_arming".into(),
                    machine: "mcu_node".into(),
                    link_name: "udp_listener".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:9f94349333d3b9ad","code":"reassembly/untrusted-link-binding","stage":"validation","spec":"watching-zenoh RFC §5.M","message":"reassembly-variant buffer-pool 'rx_reassembly_pool' is bound to link 'udp_listener' on machine 'mcu_node', but the link declares `trust_class: session_arming`. watching-zenoh RFC §5.M line 2964-2969 — only `trust_class: established_session` links may carry fragmented traffic; reassembly on `untrusted` / `session_arming` links exposes the per-peer quota space to source-IP spoofing. Repair: change link 'udp_listener' to `trust_class: established_session` (only if the link is in fact post-handshake), or remove the reassembly-pool binding.","actual":"session_arming"}"#,
            ),
            (
                // RFC §5.M line 2970-2975: domain_attrs absent on a
                // link with reassembly-pool binding. Hard error.
                // NeutralOrDeterministic (two valid repairs).
                "forge/reassembly-trust-class-missing-on-fragmenting-link",
                ValidationError::ReassemblyTrustClassMissingOnFragmentingLink {
                    pool_name: "rx_reassembly_pool".into(),
                    machine: "mcu_node".into(),
                    link_name: "udp_data".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:d09ad5b8d7cdc514","code":"reassembly/trust-class-missing-on-fragmenting-link","stage":"validation","spec":"watching-zenoh RFC §5.M","message":"reassembly-variant buffer-pool 'rx_reassembly_pool' is bound to link 'udp_data' on machine 'mcu_node', but the link does not declare `domain_attrs.trust_class`. watching-zenoh RFC §5.M line 2970-2975 — build cannot decide whether the binding is safe without a declared trust class. Repair: declare `domain_attrs: { trust_class: established_session }` on link 'udp_data' (data-plane links), or remove the reassembly-pool binding (control-plane links)."}"#,
            ),
            (
                // RFC §5.M line 2995-2999: stage-copy WCET vs slot budget.
                // NeutralOrDeterministic (multi-axis repair).
                "forge/reassembly-stage-copy-wcet-exceeds-slot-budget",
                ValidationError::ReassemblyStageCopyWcetExceedsSlotBudget {
                    machine: "mcu_node".into(),
                    link_name: "udp_data".into(),
                    expected_p99_bytes: 16384,
                    memcpy_cycles_per_byte: 4.0,
                    clock_freq_mhz: 48,
                    worker_slot_budget_us: 200,
                    stage_copy_wcet_us: 1365,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:c41a9b3397849183","code":"reassembly/stage-copy-wcet-exceeds-slot-budget","stage":"validation","spec":"watching-zenoh RFC §5.M","message":"link 'udp_data' on machine 'mcu_node': stage-copy WCET (1365 µs) exceeds `scheduler.worker_slot_budget_us: 200`. watching-zenoh RFC §5.M line 2995-2999 — `expected_p99_bytes (16384) × memcpy_cycles_per_byte (4) / clock_freq_mhz (48) > worker_slot_budget_us`. The stage copy alone starves Keepalive and parallel-region timers (ARCHITECTURE §9.3 + §3.4). Repair: raise `worker_slot_budget_us` (and re-validate every algorithm), lower `expected_p99_bytes` so stage copy is never invoked at that size, or raise the bound pool's `<sce:slot-size>` to absorb p99 without invoking stage copy.","expected":["200"],"actual":"1365"}"#,
            ),
            (
                // RFC §5.M line 2976-2981: codegen self-check —
                // reassembly variant must emit ZID-shaped per-slot
                // peer-id. NeutralOrDeterministic; pure template-
                // regression guard (mirrors
                // `mem/inter-pool-padding-not-emitted` shape).
                "forge/reassembly-peer-id-not-zid-on-established-session",
                ValidationError::ReassemblyPeerIdNotZidOnEstablishedSession {
                    pool_name: "rx_reassembly_pool".into(),
                    language: "rust".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:c88b0fe4fe0b1ff7","code":"reassembly/peer-id-not-zid-on-established-session","stage":"validation","spec":"watching-zenoh RFC §5.M","message":"reassembly-variant buffer-pool 'rx_reassembly_pool' (rust backend): emitted per-slot peer-id is not the 16-byte ZID signature required for `trust_class: established_session` bindings. watching-zenoh RFC §5.M line 2976-2981 — codegen invariant violation: per-peer quota check must use the handshake-derived ZID as the peer key, not the wire source address (defends against UDP source-IP spoofing on `established_session` links). In well-formed templates the reassembly variant always emits the 16-byte ZID typedef (the cross-doc validator `reassembly/untrusted-link-binding` gates non-`established_session` bindings upstream), so this diagnostic fires only on template regression; report at https://github.com/newmassrael/scxml-core-engine/issues"}"#,
            ),
            (
                // RFC §5.C lines 849-856: codegen self-check —
                // listener-link must emit both Listener + Sibling
                // halves. NeutralOrDeterministic; pure template-
                // regression guard (mirrors C9-γ
                // `reassembly/peer-id-not-zid-on-established-session`).
                "forge/link-listener-link-not-paired-with-established-sibling",
                ValidationError::LinkListenerLinkNotPairedWithEstablishedSibling {
                    link_name: "udp_listener".into(),
                    language: "rust".into(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:1018399a1345bb35","code":"link/listener-link-not-paired-with-established-sibling","stage":"validation","spec":"watching-zenoh RFC §5.C","message":"link 'udp_listener' (rust backend): listener-link sibling emission missing the `established_session` half. watching-zenoh RFC §5.C lines 849-856 — codegen invariant violation: every `session_arming` listener must emit its paired `established_session` sibling so per-peer dispatch retains a stable codegen-time identity (re-introduces OQ-W22 if dropped). In well-formed templates the diagnostic never fires (the per-language link template emits both halves unconditionally when `listener_links` contains this name); report at https://github.com/newmassrael/scxml-core-engine/issues"}"#,
            ),
            (
                // RFC §5.M lines 2982-2994: reassembly binding on
                // session_arming link without paired sibling.
                // NeutralOrDeterministic; two valid repair paths
                // (add `Accepting.*` substate vs remove binding).
                "forge/reassembly-binding-on-unpaired-listener",
                ValidationError::ReassemblyBindingOnUnpairedListener {
                    pool_name: "rx_reassembly_pool".into(),
                    machine: "mcu_node".into(),
                    link_name: "udp_listener".into(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:c9d90099f8ed9a01","code":"reassembly/binding-on-unpaired-listener","stage":"validation","spec":"watching-zenoh RFC §5.M","message":"reassembly-variant buffer-pool 'rx_reassembly_pool' is bound to link 'udp_listener' on machine 'mcu_node'; the link declares `trust_class: session_arming` but its machine source SCXML has no `Accepting.*` substate, so codegen cannot synthesize the paired `established_session` sibling. watching-zenoh RFC §5.M lines 2982-2994 — only listeners (machine source SCXML carrying `Accepting.*`) auto-rebind a `session_arming` reassembly binding to the `established_session` sibling; without that pairing the binding has no valid landing site. Repair: add an `Accepting.*` substate to machine 'mcu_node's source SCXML (making link 'udp_listener' a real listener so the sibling auto-synthesizes), or remove the reassembly-pool binding from link 'udp_listener'."}"#,
            ),
            (
                // RFC §5.N line 3062: cross-doc link has inbound
                // events but no FSM event-queue capacity reaches it.
                // NeutralOrDeterministic — two-axis repair (per-
                // instance sce:capacity vs per-machine
                // default_event_queue_capacity).
                "forge/link-inbound-event-queue-unsized",
                ValidationError::LinkInboundEventQueueUnsized {
                    machine: "mcu_node".into(),
                    link_name: "udp_listener".into(),
                    inbound_event_count: 3,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:d927379bfa06a12a","code":"link/inbound-event-queue-unsized","stage":"validation","spec":"watching-zenoh RFC §5.N","message":"link 'udp_listener' on machine 'mcu_node': declares 3 inbound event(s) but no downstream FSM event-queue capacity is bound. watching-zenoh RFC §5.N line 3062 — link declared but downstream FSM inbox depth unset. Repair: add `<scxml sce:capacity=\"N\">` to machine 'mcu_node's source SCXML (per-instance), or add `scheduler.default_event_queue_capacity: N` under `machines.mcu_node` (per-machine fallback).","actual":"3"}"#,
            ),
            // ── §5.K C13-γ stage-copy policy promotion + opt-out rejection ──
            (
                // RFC §5.K line 2504-2511: warning promoted to hard error.
                // NeutralOrDeterministic; multi-axis repair.
                "forge/pool-stage-copy-policy-error",
                ValidationError::PoolStageCopyPolicyError {
                    pool_name: "rx_data_pool".into(),
                    slot_size: 700,
                    expected_p99_bytes: 1024,
                    rate_percent: 31,
                    machine: "mcu_node".into(),
                    link_name: "udp_data".into(),
                    policy: "error".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:c91f1db81740d284","code":"pool/stage-copy-policy-error","stage":"validation","spec":"watching-zenoh RFC §5.K","message":"link 'udp_data' on machine 'mcu_node': `expected_p99_bytes: 1024` vs RX pool 'rx_data_pool' `<sce:slot-size>700</sce:slot-size>` triggers stage-copy rate 31% (> 25% threshold), promoted to hard error under `pool_defaults.stage_copy_policy: error`. watching-zenoh RFC §5.K line 2504-2511 — author resolution: raise `<sce:slot-size>` on pool 'rx_data_pool', lower `expected_p99_bytes`, or add `<sce:accept-stage-copy-rate>` on link 'udp_data' (last option unavailable under `forbid`).","expected":["25"],"actual":"31"}"#,
            ),
            (
                // RFC §5.K line 2512-2516: forbid rejects the opt-out outright.
                // NeutralOrDeterministic; two valid repair paths.
                "forge/pool-stage-copy-accept-rejected-under-forbid",
                ValidationError::PoolStageCopyAcceptRejectedUnderForbid {
                    machine: "mcu_node".into(),
                    link_name: "udp_data".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:c4ce58defa7ffd7f","code":"pool/stage-copy-accept-rejected-under-forbid","stage":"validation","spec":"watching-zenoh RFC §5.K","message":"link 'udp_data' on machine 'mcu_node': `<sce:accept-stage-copy-rate>` declared but `pool_defaults.stage_copy_policy: forbid` rejects the opt-out outright. watching-zenoh RFC §5.K line 2512-2516 — only structural fixes (raise `<sce:slot-size>` or lower `expected_p99_bytes`) are accepted under `forbid`. Repair: remove `<sce:accept-stage-copy-rate>` from link 'udp_data', or change `pool_defaults.stage_copy_policy` to `error` (which permits the opt-out).","actual":"<sce:accept-stage-copy-rate>"}"#,
            ),
            // ── §5.L Bounded-collection cross-doc resolution (C6-β) ──
            (
                // RFC §5.L lines 2566-2567: element-type body text does
                // not resolve to a codec-kind struct or procedure-kind
                // state record. FixCarriesCandidates — sorted codec +
                // procedure name union rides Fix::ReplaceOneOf.
                "forge/collection-element-type-not-a-kind",
                ValidationError::CollectionElementTypeNotAKind {
                    collection_name: "local_sub_table".into(),
                    element_type: "subscrription_entry".into(),
                    candidates: vec![
                        "router_handle".into(),
                        "subscription_entry".into(),
                    ],
                    candidates_list:
                        "router_handle, subscription_entry".into(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:653bda4f395fa2f4","code":"collection/element-type-not-a-kind","stage":"validation","spec":"watching-zenoh RFC §5.L","message":"bounded-collection 'local_sub_table': <sce:element-type>subscrription_entry</sce:element-type> does not name a codec-kind struct or procedure-kind state record in this build. watching-zenoh RFC §5.L line 2566-2567 — element types must reference another forge kind by name (codec for byte-encoded structs, procedure for stateful records). Declare the element type as a separate `.scxml` document (codec: `<scxml sce:kind=\"codec\" name=\"subscrription_entry\">`; procedure: `<scxml sce:kind=\"procedure\" name=\"subscrription_entry\">`), or replace the body text with one of the registered candidates: router_handle, subscription_entry.","actual":"subscrription_entry","fix":{"kind":"replace_one_of","candidates":["router_handle","subscription_entry"]}}"#,
            ),
            (
                // RFC §5.L line 2615: index-by field absent from the
                // resolved element-type struct (codec.fields[].id or
                // procedure.inputs[].id + internals[].id enumeration).
                // FixCarriesCandidates over the sorted field name list.
                "forge/collection-index-by-field-missing",
                ValidationError::CollectionIndexByFieldMissing {
                    collection_name: "local_sub_table".into(),
                    field: "key_id".into(),
                    element_type: "subscription_entry".into(),
                    element_kind: "codec".into(),
                    candidates: vec![
                        "callback_id".into(),
                        "key_expr_id".into(),
                    ],
                    candidates_list:
                        "callback_id, key_expr_id".into(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:4a9fd47c8aae2381","code":"collection/index-by-field-missing","stage":"validation","spec":"watching-zenoh RFC §5.L","message":"bounded-collection 'local_sub_table': <sce:index-by field=\"key_id\"/> names a field that does not exist on element-type 'subscription_entry' (codec kind). watching-zenoh RFC §5.L line 2615 — the `index-by` field enables `find_by_index(IndexKey)` and must name an actual struct field of the element type. Replace `field=\"key_id\"` with one of the subscription_entry's declared fields: callback_id, key_expr_id.","actual":"key_id","fix":{"kind":"replace_one_of","candidates":["callback_id","key_expr_id"]}}"#,
            ),
            (
                // RFC §5.L lines 2560-2562: multi-writer concurrency
                // declared without any §5.I atomic intrinsic imported
                // via `<sce:extern>` anywhere in the build. No closed
                // candidate set — the C4 baseline atomic family is too
                // large for `Fix::ReplaceOneOf`; author chooses width +
                // ordering + op. NeutralOrDeterministic, no Fix.
                "forge/collection-multi-writer-without-atomics",
                ValidationError::CollectionMultiWriterWithoutAtomics {
                    collection_name: "local_sub_table".into(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:b944cb27eabbfac8","code":"collection/multi-writer-without-atomics","stage":"validation","spec":"watching-zenoh RFC §5.L","message":"bounded-collection 'local_sub_table': <sce:concurrency>multi-writer</sce:concurrency> requires at least one §5.I atomic intrinsic to be declared via <sce:extern> somewhere in this build. watching-zenoh RFC §5.L lines 2560-2562 — multi-writer codegen lowers to acquire/release atomics on head/tail; the build's <sce:extern> trust-surface must acknowledge atomic intrinsics for codegen to emit them. Repair: either declare an atomic intrinsic via <sce:extern> (e.g. `<sce:extern name=\"sce_atomic_load_acquire_u32\" sig=\"(*const u32) -> u32\" abi=\"c\"/>` in any forge doc in this build), or change `<sce:concurrency>` to `single-writer`."}"#,
            ),
            // ── §5.L Bounded-collection deploy-time capacity resolution (C6-γ1) ──
            (
                // RFC §5.L lines 2583-2585: `<sce:capacity source="deploy"
                // key=…/>` references an undeclared limit under
                // `machines.<machine>.limits:`. FixCarriesCandidates —
                // sorted declared limit names ride Fix::ReplaceOneOf.
                "forge/collection-capacity-unresolved",
                ValidationError::CollectionCapacityUnresolved {
                    collection_name: "local_sub_table".into(),
                    key: "machines.mcu_node.limits.local_subscriptions".into(),
                    machine: "mcu_node".into(),
                    limit: "local_subscriptions".into(),
                    candidates: vec![
                        "in_flight_reassembly".into(),
                        "subscription_table".into(),
                    ],
                    candidates_list:
                        "in_flight_reassembly, subscription_table".into(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:4604decf96012397","code":"collection/capacity-unresolved","stage":"validation","spec":"watching-zenoh RFC §5.L","message":"bounded-collection 'local_sub_table': <sce:capacity source=\"deploy\" key=\"machines.mcu_node.limits.local_subscriptions\"/> references limit 'local_subscriptions' on machine 'mcu_node', but deploy.yaml does not declare `machines.mcu_node.limits.local_subscriptions`. watching-zenoh RFC §5.L lines 2583-2585 — `<sce:capacity source=\"deploy\">` resolves at codegen time to a per-language compile-time constant from `machines.<machine>.limits.<limit>:`; an unresolved limit blocks emission. Repair: declare `local_subscriptions: <count>` under `machines.mcu_node.limits:` in deploy.yaml (declared limits today: in_flight_reassembly, subscription_table), or switch the BC's `<sce:capacity>` to `const=\"N\"`.","actual":"machines.mcu_node.limits.local_subscriptions","fix":{"kind":"replace_one_of","candidates":["in_flight_reassembly","subscription_table"]}}"#,
            ),
            // ── §5.I `<sce:extern>` whitelisted intrinsic registry (Atomic A) ──
            (
                // RFC §5.I line 1847: symbol absent from the §5.I baseline
                // registry. Closest-match candidates ride `Fix::ReplaceOneOf`
                // so authors see suggestions without paging through the
                // 101-symbol baseline.
                "forge/extern-symbol-not-in-whitelist",
                ValidationError::ExternSymbolNotInWhitelist {
                    name: "sce_atomic_compare_exchange_u32".into(),
                    candidates: vec![
                        "sce_atomic_cas_strong_acq_rel_u32".into(),
                        "sce_atomic_cas_weak_acq_rel_u32".into(),
                    ],
                    candidates_list:
                        "sce_atomic_cas_strong_acq_rel_u32, sce_atomic_cas_weak_acq_rel_u32".into(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:173e706863ec82de","code":"extern/symbol-not-in-whitelist","stage":"validation","spec":"watching-zenoh RFC §5.I","message":"<sce:extern name=\"sce_atomic_compare_exchange_u32\"> references a symbol that is not on the §5.I baseline whitelist. Choose a registry-listed name (closest matches: sce_atomic_cas_strong_acq_rel_u32, sce_atomic_cas_weak_acq_rel_u32) or extend the whitelist via a target plugin (deploy.yaml `extern_symbols.target_plugin`).","actual":"sce_atomic_compare_exchange_u32","fix":{"kind":"replace_one_of","candidates":["sce_atomic_cas_strong_acq_rel_u32","sce_atomic_cas_weak_acq_rel_u32"]}}"#,
            ),
            (
                // RFC §5.I line 1848: ABI mismatch — closed two-element
                // repair set [`c`, `rust`] rides `Fix::ReplaceOneOf`.
                "forge/extern-abi-mismatch",
                ValidationError::ExternAbiMismatch {
                    name: "sce_atomic_load_acquire_u32".into(),
                    expected: "c".into(),
                    actual: "rust".into(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:c41d4f3a5258039c","code":"extern/abi-mismatch","stage":"validation","spec":"watching-zenoh RFC §5.I","message":"<sce:extern name=\"sce_atomic_load_acquire_u32\" abi=\"rust\"> uses a non-canonical ABI; the registry entry requires `abi=\"c\"`. The accepted set is [\"c\", \"rust\"].","expected":["c"],"actual":"rust","fix":{"kind":"replace_one_of","candidates":["c","rust"]}}"#,
            ),
            (
                // RFC §5.I line 1849: signature mismatch — `Fix::ReplaceWith`
                // carries the canonical sig (registry is source of truth).
                "forge/extern-signature-mismatch",
                ValidationError::ExternSignatureMismatch {
                    name: "sce_atomic_load_acquire_u32".into(),
                    expected: "(*const u32) -> u32".into(),
                    actual: "(*const u32) -> u64".into(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:36a8a1a0fbe59da6","code":"extern/signature-mismatch","stage":"validation","spec":"watching-zenoh RFC §5.I","message":"<sce:extern name=\"sce_atomic_load_acquire_u32\" sig=\"(*const u32) -> u64\"> declares a signature that does not match the registry entry. Replace with `sig=\"(*const u32) -> u32\"`.","expected":["(*const u32) -> u32"],"actual":"(*const u32) -> u64","fix":{"kind":"replace_with","to":"(*const u32) -> u32"}}"#,
            ),
            (
                // RFC §5.I line 1850: atomic-family base without ordering
                // suffix (e.g. `sce_atomic_load`). Suffix-bearing
                // completions ride `Fix::ReplaceOneOf`.
                "forge/extern-ordering-unspecified",
                ValidationError::ExternOrderingUnspecified {
                    base: "sce_atomic_load".into(),
                    candidates: vec![
                        "sce_atomic_load_acquire_u32".into(),
                        "sce_atomic_load_relaxed_u32".into(),
                    ],
                    candidates_list:
                        "sce_atomic_load_acquire_u32, sce_atomic_load_relaxed_u32".into(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:4e511900e9aed2c8","code":"extern/ordering-unspecified","stage":"validation","spec":"watching-zenoh RFC §5.I","message":"<sce:extern name=\"sce_atomic_load\"> is an atomic-family base without an explicit ordering + width suffix. Pick one of: sce_atomic_load_acquire_u32, sce_atomic_load_relaxed_u32.","actual":"sce_atomic_load","fix":{"kind":"replace_one_of","candidates":["sce_atomic_load_acquire_u32","sce_atomic_load_relaxed_u32"]}}"#,
            ),
            (
                // RFC §5.I line 1852 (Atomic B): target plugin redefines
                // a baseline whitelist symbol. Q-Call-6 (a) additive
                // composition lock — plugins extend, never override.
                // Repair is non-algorithmic (`fix: None`); plugin author
                // renames to a non-baseline name.
                "forge/extern-target-plugin-symbol-conflict",
                ValidationError::ExternTargetPluginSymbolConflict {
                    name: "sce_atomic_load_acquire_u32".into(),
                    plugin_path: "configs/target_extensions_stm32h7.yaml".into(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:0133c811a527ab82","code":"extern/target-plugin-symbol-conflict","stage":"validation","spec":"watching-zenoh RFC §5.I","message":"target plugin configs/target_extensions_stm32h7.yaml redefines core whitelist symbol `sce_atomic_load_acquire_u32`. Plugin entries extend the §5.I baseline registry but cannot override it (Q-Call-6 additive-composition lock). Rename the plugin entry to a name not already in the §5.I baseline; for a platform-specific impl, declare the entry under a vendor-prefixed name (e.g. `sce_hw_<symbol>`) and route through the registry entry's `crate` field.","actual":"sce_atomic_load_acquire_u32"}"#,
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
            // ── Watching-zenoh RFC §5.J.4 / §5.J.5 codegen matrix shells.
            //    Producer constructors are reachable; matrix walker that
            //    invokes them lands with the algorithm kind in Phase A3. ──
            (
                "forge/codegen-mcu-class-kind-on-non-mcu-language",
                GenerateError::CodegenMcuClassKindOnNonMcuLanguage {
                    kind: "link".into(),
                    language: "kotlin".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:0e78c9c56b3c4d51","code":"codegen/mcu-class-kind-on-non-mcu-language","stage":"generate","message":"MCU-class kind 'link' cannot be lowered to language 'kotlin': only rust and c11 have MCU substrate (watching-zenoh RFC §5.J.4)","actual":"kotlin"}"#,
            ),
            (
                "forge/codegen-generic-kind-backend-emit-missing",
                GenerateError::CodegenGenericKindBackendEmitMissing {
                    kind: "algorithm".into(),
                    language: "python".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:d54c90195c019259","code":"codegen/generic-kind-backend-emit-missing","stage":"generate","message":"generic-class kind 'algorithm': template missing for language 'python' (watching-zenoh RFC §5.J.4 expects all six backends to emit)"}"#,
            ),
            // ── Watching-zenoh RFC §5.J.2 Rust no_std variant rejections
            //    (C3 Atomic B-β). Author-side `--no-std` gate on
            //    `sce-codegen generate -l rust`. ──
            (
                "forge/codegen-no-std-script-not-supported",
                GenerateError::CodegenNoStdScriptNotSupported {
                    document: "demo".into(),
                    locations: "<script> in state 'init'".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:99d806fa3e160004","code":"codegen/no-std-script-not-supported","stage":"generate","spec":"watching-zenoh RFC §5.J.2","message":"Rust no_std variant rejects `<script>`: document 'demo' uses ECMAScript at <script> in state 'init' (watching-zenoh RFC §5.J.2; sce-rust-runtime no_std feature is incompatible with `script-engine-lua` and `script-engine-quickjs`)"}"#,
            ),
            (
                "forge/codegen-no-std-http-not-supported",
                GenerateError::CodegenNoStdHttpNotSupported {
                    document: "demo".into(),
                    locations: "<send target=\"http://localhost\"> in state 'send_step'".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:1e0ad94889d9bb0c","code":"codegen/no-std-http-not-supported","stage":"generate","spec":"watching-zenoh RFC §5.J.2","message":"Rust no_std variant rejects HTTP send: document 'demo' uses BasicHTTPEventProcessor at <send target=\"http://localhost\"> in state 'send_step' (watching-zenoh RFC §5.J.2; sce-rust-runtime no_std feature is incompatible with `http-send`)"}"#,
            ),
            // ── Watching-zenoh RFC §5.J.2 Rust no_std variant rejections
            //    (C3 Atomic B-γ2c). Helper runtime cfg-gate companion
            //    pair: filesystem load + invoke. ──
            (
                "forge/codegen-no-std-fs-load-not-supported",
                GenerateError::CodegenNoStdFsLoadNotSupported {
                    document: "demo".into(),
                    locations: "<data id='cfg' src='file:cfg.json'>".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:69b3678acabbba8d","code":"codegen/no-std-fs-load-not-supported","stage":"generate","spec":"watching-zenoh RFC §5.J.2","message":"Rust no_std variant rejects external `<data src>`: document 'demo' loads file content at <data id='cfg' src='file:cfg.json'> (watching-zenoh RFC §5.J.2; filesystem helpers are gated to !no_std and unreachable from emitted code)"}"#,
            ),
            (
                "forge/codegen-no-std-invoke-not-supported",
                GenerateError::CodegenNoStdInvokeNotSupported {
                    document: "demo".into(),
                    locations: "<invoke type='scxml' src='child.scxml'>".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:d46720a4cb2f313e","code":"codegen/no-std-invoke-not-supported","stage":"generate","spec":"watching-zenoh RFC §5.J.2","message":"Rust no_std variant rejects `<invoke>`: document 'demo' invokes child sessions at <invoke type='scxml' src='child.scxml'> (watching-zenoh RFC §5.J.2; invoke processing is gated to !no_std and unreachable from emitted code)"}"#,
            ),
            // ── Algorithm kind sema (watching-zenoh RFC §5.A) ───────
            (
                "forge/algorithm-local-shadows-param",
                ValidationError::AlgorithmLocalShadowsParam {
                    name: "data".into(),
                    what: "param".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:438800138df5f1c2","code":"algorithm/local-shadows-param","stage":"validation","spec":"watching-zenoh RFC §5.A","message":"algorithm: identifier 'data' shadows param","actual":"data"}"#,
            ),
            (
                "forge/algorithm-lvalue-unsupported",
                ValidationError::AlgorithmLvalueUnsupported {
                    target: "data".into(),
                    restriction: "algorithm parameters are read-only in v1".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:397b595293daf8ef","code":"algorithm/lvalue-unsupported","stage":"validation","spec":"watching-zenoh RFC §5.A","message":"<sce:assign target=\"data\">: algorithm parameters are read-only in v1","actual":"data"}"#,
            ),
            (
                "forge/algorithm-return-missing",
                ValidationError::AlgorithmReturnMissing.into(),
                r#"{"v":1,"id":"fnv1a:e2582bd483bbf621","code":"algorithm/return-missing","stage":"validation","spec":"watching-zenoh RFC §5.A","message":"algorithm: signature declares return type but body's last statement is not <sce:return>"}"#,
            ),
            // ── C7-lowering: algorithm-over-BC dispatch goldens
            //    (RFC §5.A line 311 + §5.L lines 2611-2618 + 2642-2647).
            //    Hash placeholders — patched by byte-stability assertion.
            (
                "forge/algorithm-foreach-source-not-iterable",
                ValidationError::AlgorithmForeachSourceNotIterable {
                    src: "missing_bc".into(),
                    candidates: vec!["data".into(), "subs".into()],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:8e088992b058a56d","code":"algorithm/foreach-source-not-iterable","stage":"validation","spec":"watching-zenoh RFC §5.A + §5.L","message":"algorithm: <sce:foreach in=\"missing_bc\">: source does not resolve to a bytes param or a bounded-collection import alias","actual":"missing_bc"}"#,
            ),
            (
                "forge/algorithm-call-target-unknown",
                ValidationError::AlgorithmCallTargetUnknown {
                    target: "missing_alias.find_by_index".into(),
                    alias: "missing_alias".into(),
                    candidates: vec!["subs".into()],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:7ca80fadd3ca6709","code":"algorithm/call-target-unknown","stage":"validation","spec":"watching-zenoh RFC §5.A + §5.L","message":"algorithm: <sce:call target=\"missing_alias.find_by_index\">: alias 'missing_alias' is not a declared import","actual":"missing_alias","fix":{"kind":"replace_one_of","candidates":["subs"]}}"#,
            ),
            (
                "forge/algorithm-call-target-method-unknown",
                ValidationError::AlgorithmCallTargetMethodUnknown {
                    target: "subs.unknown_method".into(),
                    alias: "subs".into(),
                    method: "unknown_method".into(),
                    kind: "bounded-collection".into(),
                    candidates: vec![
                        "capacity".into(),
                        "find_by_index".into(),
                        "get".into(),
                        "get_by_slot".into(),
                        "len".into(),
                    ],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:18c90b6a637ace44","code":"algorithm/call-target-method-unknown","stage":"validation","spec":"watching-zenoh RFC §5.A + §5.L","message":"algorithm: <sce:call target=\"subs.unknown_method\">: method 'unknown_method' is not callable on import 'subs' (kind=bounded-collection)","actual":"unknown_method","fix":{"kind":"replace_one_of","candidates":["capacity","find_by_index","get","get_by_slot","len"]}}"#,
            ),
            (
                "forge/algorithm-bc-mutation-forbidden",
                ValidationError::AlgorithmBcMutationForbidden {
                    target: "subs.insert".into(),
                    method: "insert".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:7fa7fc026ed6fad4","code":"algorithm/bc-mutation-forbidden","stage":"validation","spec":"watching-zenoh RFC §5.A + §5.L","message":"algorithm: <sce:call target=\"subs.insert\">: mutating bounded-collection method 'insert' is forbidden from algorithm body (algorithms are pure per RFC §5.A)","actual":"insert"}"#,
            ),
            (
                "forge/algorithm-foreach-source-bc-with-bytes-item-type",
                ValidationError::AlgorithmForeachSourceBcWithBytesItemType {
                    src: "subs".into(),
                    var_name: "b".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:6b9c5ec0f8cf743f","code":"algorithm/foreach-source-bc-with-bytes-item-type","stage":"validation","spec":"watching-zenoh RFC §5.A + §5.L","message":"algorithm: <sce:foreach in=\"subs\"> over bounded-collection: body's <sce:var name=\"b\" type=\"uint8\"> uses the bytes-iteration pattern but 'subs' is a bounded-collection (item carries element-type)","actual":"b"}"#,
            ),
            (
                "forge/algorithm-call-arg-count-mismatch",
                ValidationError::AlgorithmCallArgCountMismatch {
                    target: "subs.find_by_index".into(),
                    actual: 2,
                    expected: 1,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:bd464011584cd3c9","code":"algorithm/call-arg-count-mismatch","stage":"validation","spec":"watching-zenoh RFC §5.A + §5.L","message":"algorithm: <sce:call target=\"subs.find_by_index\">: argument count 2 does not match callable's arity 1"}"#,
            ),
            // ── §5.F build-time const-fold (watching-zenoh RFC §5.F) ─
            (
                "forge/algorithm-const-not-foldable",
                GenerateError::ConstNotFoldable {
                    algorithm: "crc16".into(),
                    const_name: "table".into(),
                    detail: "arithmetic on non-numeric operand".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:f005de566be48738","code":"algorithm/const-not-foldable","stage":"generate","spec":"watching-zenoh RFC §5.F","message":"algorithm 'crc16': <sce:const name=\"table\">: const-not-foldable: arithmetic on non-numeric operand","actual":"table"}"#,
            ),
            (
                "forge/algorithm-const-fold-budget-exceeded",
                GenerateError::ConstFoldBudgetExceeded {
                    algorithm: "crc16".into(),
                    const_name: Some("table".into()),
                    budget: 1_000_000,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:d1dee18e4cb7eeba","code":"algorithm/const-fold-budget-exceeded","stage":"generate","spec":"watching-zenoh RFC §5.F","message":"algorithm 'crc16': <sce:const name=\"table\">: const-fold-budget-exceeded: total iteration count exceeded the configured budget of 1000000 (RFC §5.F bound 1; override with --const-fold-budget=N)","actual":"table"}"#,
            ),
            (
                "forge/algorithm-const-yield-type-mismatch",
                GenerateError::ConstYieldTypeMismatch {
                    algorithm: "crc16".into(),
                    const_name: "table".into(),
                    expected: crate::forge::model::SceType::Uint16,
                    actual: "float".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:5d993dd11e2f6ddb","code":"algorithm/const-yield-type-mismatch","stage":"generate","spec":"watching-zenoh RFC §5.F","message":"algorithm 'crc16': <sce:const name=\"table\">: const-yield-type-mismatch: cannot coerce float to Uint16","actual":"float"}"#,
            ),
            // ── §5.B variant primitive (watching-zenoh RFC §5.B, B1-β) ─
            (
                "forge/codec-variant-arm-unreachable",
                ValidationError::CodecVariantArmUnreachable {
                    codec: "session_envelope".into(),
                    tag_field: "msg_id".into(),
                    tag_type: "uint8".into(),
                    arm_count: 3,
                    domain_size: Some(256),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:fb2f4b7fefc04603","code":"codec/variant-arm-unreachable","stage":"validation","spec":"watching-zenoh RFC §5.B","message":"codec 'session_envelope': variant on tag 'msg_id' (type uint8) has 3 arm(s) but no <sce:default> declared (tag type domain has 256 values) — at least one tag value would have no matching arm at runtime; add <sce:default type=\"...\"/> or enumerate the missing values explicitly","actual":"msg_id"}"#,
            ),
            // ── RFC variant-default-uniformity Atomic α — duplicate default-arm marker ─
            (
                "forge/codec-variant-duplicate-default-arm",
                ValidationError::CodecVariantDuplicateDefaultArm {
                    codec: "session_envelope".into(),
                    first_arm_value: 0x01,
                    second_arm_value: 0x02,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:e0a8b8b88b736596","code":"codec/variant-duplicate-default-arm","stage":"validation","spec":"watching-zenoh RFC §5.B","message":"codec 'session_envelope': <sce:variant> declares more than one <sce:arm default=\"true\"/> (first arm value=0x1, second arm value=0x2) — only one arm may be marked the Default-trait starting value; remove default=\"true\" from all but the intended arm. (The catch-all <sce:default> element is unrelated and still permitted once.)","actual":"session_envelope"}"#,
            ),
            // ── RFC variant-default-uniformity Atomic γ-1 — outer arm vs inner flag mismatch ─
            (
                "forge/codec-variant-default-arm-mid-mismatch",
                ValidationError::CodecVariantDefaultArmMidMismatch {
                    codec: "session_envelope".into(),
                    arm_value: 0x02,
                    inner_codec: "session_put".into(),
                    inner_flag: "mid".into(),
                    inner_flag_value: 0x01,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:290bd7d238ce9d7c","code":"codec/variant-default-arm-mid-mismatch","stage":"validation","spec":"watching-zenoh RFC §5.B","message":"codec 'session_envelope': default <sce:arm value=0x2/> selects inner codec 'session_put' but that codec declares <sce:flag name='mid' value=0x1/> on its dispatch field — outer arm value and inner flag value must match for round-trip dispatch to resolve to the same arm; align one to the other","expected":["0x2"],"actual":"0x1"}"#,
            ),
            // ── RFC variant-default-uniformity Atomic γ-1 — inner codec missing wire-MID ─
            (
                "forge/codec-variant-arm-inner-mid-undeclared",
                ValidationError::CodecVariantArmInnerMidUndeclared {
                    codec: "session_envelope".into(),
                    arm_value: 0x02,
                    inner_codec: "session_put".into(),
                    expected_flag: "mid".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:0bf3318e19f672aa","code":"codec/variant-arm-inner-mid-undeclared","stage":"validation","spec":"watching-zenoh RFC §5.B","message":"codec 'session_envelope': default <sce:arm value=0x2/> selects inner codec 'session_put', but 'session_put' does not declare a <sce:flag value=\"...\"/> constant on its dispatch field — the inner's Default would zero-fill the wire byte and break round-trip; add <sce:flag name='mid' value=0x2/> to 'session_put'","expected":["0x2"],"actual":"session_put"}"#,
            ),
            // ── §5.B present-if primitive (watching-zenoh RFC §5.B, B1-δ) ─
            (
                "forge/codec-present-if-refs-later-field",
                ValidationError::CodecPresentIfRefsLaterField {
                    codec: "session_envelope".into(),
                    field: "key".into(),
                    refers_to: "trailer_flags".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:f6f6bd818f8e28d9","code":"codec/present-if-refs-later-field","stage":"validation","spec":"watching-zenoh RFC §5.B","message":"codec 'session_envelope': field 'key' has sce:present-if=\"trailer_flags.…\" but 'trailer_flags' is not declared earlier in this codec — present-if predicates must reference a flags-bearing carrier that the streaming decoder has already consumed; reorder the fields so the carrier comes first, or correct the predicate","actual":"key"}"#,
            ),
            // ── §5.B repeat primitive (watching-zenoh RFC §5.B, B2) ─
            (
                "forge/codec-repeat-count-refs-later-field",
                ValidationError::CodecRepeatCountRefsLaterField {
                    codec: "fragment_burst".into(),
                    field: "frags".into(),
                    refers_to: "num_frags".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:f1e97717ebdba39a","code":"codec/repeat-count-refs-later-field","stage":"validation","spec":"watching-zenoh RFC §5.B","message":"codec 'fragment_burst': repeat field 'frags' has sce:count=\"num_frags\" but 'num_frags' is not declared earlier in this codec — repeat count references must resolve to a sibling integer field that the streaming decoder has already consumed; reorder the fields so the count comes first, or correct the attribute","actual":"frags"}"#,
            ),
            // ── §5.B test-vector primitive (watching-zenoh RFC §5.B, B2 + B5-θ) ─
            (
                "forge/algorithm-test-vector-unsupported-kind",
                ValidationError::TestVectorUnsupportedKind {
                    name: "session_filter".into(),
                    kind: ForgeKind::Filter,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:b10188077e684de4","code":"algorithm/test-vector-unsupported-kind","stage":"validation","spec":"watching-zenoh RFC §5.B","message":"<sce:test-vector> is only supported on sce:kind=\"algorithm\" (B2) and sce:kind=\"codec\" (B5-θ), but 'session_filter' declares sce:kind=\"Filter\" — move the test vector to an algorithm/codec file or use the kind-specific harness oracle","actual":"session_filter"}"#,
            ),
            // ── §5.B B3 TLV chain primitive (watching-zenoh RFC §5.B) ─
            (
                "forge/codec-tlv-chain-depth-unspecified",
                ValidationError::CodecTlvChainDepthUnspecified {
                    codec: "session_envelope".into(),
                    field: "extensions".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:569cbc9e420b26e4","code":"codec/tlv-chain-depth-unspecified","stage":"validation","spec":"watching-zenoh RFC §5.B","message":"codec 'session_envelope': tlv-chain field 'extensions' is missing the required `max-depth` attribute — TLV chain decoders need a build-time bound to size their working set and enforce iterative-only parse (RFC §5.B line 488); add `max-depth=\"N\"` for some N > 0","actual":"extensions"}"#,
            ),
            // ── §5.B B3 DMA alignment primitive (watching-zenoh RFC §5.B) ─
            (
                "forge/codec-dma-alignment-unsatisfiable",
                ValidationError::CodecDmaAlignmentUnsatisfiable {
                    codec: "session_envelope".into(),
                    field: "aligned_payload".into(),
                    burst_align: 32,
                    reason: "preceding field 'value' has bit-size 'vle' (variable-length); static padding cannot honor sce:dma-burst-align when any prior field's wire size depends on runtime values (RFC §5.B \"fixed-offset positions only — no VLE-following alignment\")".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:98584368c0e405fa","code":"codec/dma-alignment-unsatisfiable","stage":"validation","spec":"watching-zenoh RFC §5.B","message":"codec 'session_envelope': field 'aligned_payload' with sce:dma-burst-align=\"32\" cannot be honored — preceding field 'value' has bit-size 'vle' (variable-length); static padding cannot honor sce:dma-burst-align when any prior field's wire size depends on runtime values (RFC §5.B \"fixed-offset positions only — no VLE-following alignment\")","actual":"aligned_payload"}"#,
            ),
            // ── §5.B B5-γ parent-flags dependency (watching-zenoh RFC §5.B) ─
            (
                "forge/codec-parent-flag-mismatch",
                ValidationError::CodecParentFlagMismatch {
                    body_codec: "codec_init_syn_body".into(),
                    parent_codec: "codec_init_envelope".into(),
                    reason: "body declares <sce:flag name=\"S\" bit=\"6\"/> but parent codec's <sce:flags id=\"header\"> has 'S' at bit=5 (Zenoh transport header layout: S-flag is bit 6 — fix one side to align)".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:a724cb80443a21ac","code":"codec/parent-flag-mismatch","stage":"validation","spec":"watching-zenoh RFC §5.B","message":"codec 'codec_init_syn_body' (body): requires-parent-flags layout mismatch against parent codec 'codec_init_envelope' — body declares <sce:flag name=\"S\" bit=\"6\"/> but parent codec's <sce:flags id=\"header\"> has 'S' at bit=5 (Zenoh transport header layout: S-flag is bit 6 — fix one side to align)","actual":"codec_init_syn_body"}"#,
            ),
            // ── §5.C B6-α link kind (watching-zenoh RFC §5.C) ─
            (
                "forge/link-framer-missing",
                ValidationError::LinkFramerMissing {
                    name: "udp_scout".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:293d71926c63758a","code":"link/framer-missing","stage":"validation","spec":"watching-zenoh RFC §5.C","message":"link 'udp_scout': missing required <sce:framer ref=\"...\"/> child — `sce:kind=\"link\"` requires a framer codec reference so RX bytes can be decoded and TX events can be encoded; add a <sce:framer ref=\"<codec_name>\"/> child","actual":"udp_scout"}"#,
            ),
            // ── §5.C B6-γ link kind negative coverage parse-time pair ─
            (
                "forge/link-link-class-unknown",
                ValidationError::LinkLinkClassUnknown {
                    name: "udp_scout".into(),
                    value: "udpx".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:cb784501438f471c","code":"link/link-class-unknown","stage":"validation","spec":"watching-zenoh RFC §5.C","message":"link 'udp_scout': <sce:link-class> body text \"udpx\" is not in the closed enum {`udp`, `tcp`, `serial`, `websocket`, `raw_eth`} per RFC §5.C lines 765-771; replace with one of the listed candidates (OS-specific classes such as `unix_socket` or `qnx_msg` land additively in later phases)","actual":"udpx","fix":{"kind":"replace_one_of","candidates":["udp","tcp","serial","websocket","raw_eth"]}}"#,
            ),
            (
                "forge/link-backpressure-undeclared",
                ValidationError::LinkBackpressureUndeclared {
                    name: "udp_scout".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:d9cf8dea54e29a00","code":"link/backpressure-undeclared","stage":"validation","spec":"watching-zenoh RFC §5.C","message":"link 'udp_scout': missing required <sce:backpressure> child — `sce:kind=\"link\"` requires an explicit backpressure policy declaration per RFC §5.C; add a <sce:backpressure>drop|block|signal-event</sce:backpressure> child","actual":"udp_scout"}"#,
            ),
            // ── §5.C B6-η OS-axis validate-time diagnostic ───────
            (
                "forge/link-class-unsupported-on-target",
                ValidationError::LinkClassUnsupportedOnTarget {
                    name: "udp_scout".into(),
                    class: "serial".into(),
                    target_os: "linux".into(),
                    candidates: vec!["bare_metal".into()],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:5b4e6cb8ccacd634","code":"link/class-unsupported-on-target","stage":"validation","spec":"watching-zenoh RFC §5.C","message":"link 'udp_scout': link-class `serial` cannot run on target OS `linux` per RFC §5.C lines 765-771; the matrix admits `serial` on [\"bare_metal\"] only — change either the <sce:link-class> body or the deploy.yaml `machines.<id>.platform.os` for the target machine","actual":"linux","fix":{"kind":"replace_one_of","candidates":["bare_metal"]}}"#,
            ),
            // ── §5.C B6-α' link↔pool cross-resolution diagnostic ─
            (
                "forge/link-pool-slot-smaller-than-framer-max",
                ValidationError::LinkPoolSlotSmallerThanFramerMax {
                    link_name: "udp_scout".into(),
                    pool_side: "rx",
                    pool_alias: "rx_pool_sram1".into(),
                    pool_slot_size: 64,
                    framer_alias: "scout_frame_codec".into(),
                    framer_max_bytes: 256,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:b02945fcd497fac4","code":"link/pool-slot-smaller-than-framer-max","stage":"validation","spec":"watching-zenoh RFC §5.C","message":"link 'udp_scout': rx-pool 'rx_pool_sram1' slot-size 64 bytes is smaller than framer 'scout_frame_codec' worst-case encoded size 256 bytes — raise <sce:slot-size> on the bound pool or shrink the codec's worst-case body","actual":"64"}"#,
            ),
            // ── §5.E B7-α buffer-pool placement validate-time diagnostic ─
            (
                "forge/mem-pool-section-conflict",
                ValidationError::BufferPoolSectionConflict {
                    name: "rx_pool_sram1".into(),
                    machine: "mcu_node".into(),
                    section: "sram1".into(),
                    candidates: vec!["dtcm".into(), "sram2".into()],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:12ceca0bfb0b761a","code":"mem/pool-section-conflict","stage":"validation","spec":"watching-zenoh RFC §5.E","message":"buffer-pool 'rx_pool_sram1': section `sram1` is not declared in deploy.yaml `machines.mcu_node.memory.sram_regions` — extend the memory map or rename the pool's <sce:section> body to one of [\"dtcm\", \"sram2\"]","actual":"sram1","fix":{"kind":"replace_one_of","candidates":["dtcm","sram2"]}}"#,
            ),
            // ── §5.E B7-β buffer-pool size validate-time diagnostic ──
            (
                "forge/mem-pool-too-large",
                ValidationError::BufferPoolTooLarge {
                    name: "rx_pool_sram1".into(),
                    machine: "mcu_node".into(),
                    section: "sram1".into(),
                    slot_count: 32,
                    slot_size: 4096,
                    bytes_required: 131072,
                    region_size: 65536,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:8eb07e1d20f16ce2","code":"mem/pool-too-large","stage":"validation","spec":"watching-zenoh RFC §5.E","message":"buffer-pool 'rx_pool_sram1': storage footprint 131072 bytes (32 × 4096) does not fit in deploy.yaml `machines.mcu_node.memory.sram_regions.sram1` of size 65536 bytes — raise the region size or shrink slot-count/slot-size","actual":"131072"}"#,
            ),
            // ── §5.E B7-β linker fragment codegen self-check ─────
            (
                "forge/mem-inter-pool-padding-not-emitted",
                ValidationError::BufferPoolInterPoolPaddingNotEmitted {
                    name: "rx_pool_sram1".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:f3d92b158c62567b","code":"mem/inter-pool-padding-not-emitted","stage":"validation","spec":"watching-zenoh RFC §5.E","message":"buffer-pool 'rx_pool_sram1': linker fragment is missing the inter-pool `. = ALIGN(N);` sentinel — codegen invariant violation per RFC §5.E lines 1059-1064; report at https://github.com/newmassrael/scxml-core-engine/issues"}"#,
            ),
            // ── §5.E B7-ε pool header Layer 1 ownership pull-through self-check ─
            (
                "forge/pool-sample-typestate-attributes-disabled",
                ValidationError::BufferPoolSampleTypestateAttributesDisabled {
                    name: "rx_pool_sram1".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:43b2c0bb267f73f5","code":"pool/sample-typestate-attributes-disabled","stage":"validation","spec":"watching-zenoh RFC §5.E","message":"buffer-pool 'rx_pool_sram1': generated C11 header is missing the `#include <sce/sample.h>` directive — Layer 1 typestate attributes will be unavailable on consumer builds, codegen invariant violation per RFC §5.E lines 1276-1346; report at https://github.com/newmassrael/scxml-core-engine/issues"}"#,
            ),
            // ── §5.E C5 cache-maintenance validation: alignment vs platform.dcache_line_size ─
            (
                "forge/mem-cache-line-alignment",
                ValidationError::BufferPoolCacheLineAlignment {
                    name: "rx_pool_sram1".into(),
                    machine: "mcu_node".into(),
                    pool_alignment: 16,
                    dcache_line_size: 32,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:9871be15958d973d","code":"mem/cache-line-alignment","stage":"validation","spec":"watching-zenoh RFC §5.E","message":"buffer-pool 'rx_pool_sram1': alignment 16 is smaller than target platform's `dcache_line_size` 32 on machine 'mcu_node' under `cache-policy: maintain`. Partial-line cache_invalidate_by_addr corrupts adjacent slot data on the start side. Raise <sce:alignment> to at least 32.","actual":"16"}"#,
            ),
            // ── §5.E C5 cache-maintenance validation: slot_size vs platform.dcache_line_size ─
            (
                "forge/mem-slot-size-not-cache-line-multiple",
                ValidationError::BufferPoolSlotSizeNotCacheLineMultiple {
                    name: "rx_pool_sram1".into(),
                    machine: "mcu_node".into(),
                    slot_size: 100,
                    dcache_line_size: 32,
                    remainder: 4,
                    next_multiple: 128,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:ad64c9aa97912ef0","code":"mem/slot-size-not-cache-line-multiple","stage":"validation","spec":"watching-zenoh RFC §5.E","message":"buffer-pool 'rx_pool_sram1': slot-size 100 is not a whole-number multiple of target platform's `dcache_line_size` 32 on machine 'mcu_node' (remainder 4) under `cache-policy: maintain`. The boundary cache line is shared with the adjacent slot — cache_invalidate_by_addr after RX would corrupt it. Round slot-size up to 128 (next cache-line multiple).","actual":"100"}"#,
            ),
            // ── §5.E C5 cache-policy on no-dcache core (Fix::ReplaceOneOf [\"none\"]) ─
            (
                "forge/mem-cache-policy-unsupported-on-no-dcache-core",
                ValidationError::BufferPoolCachePolicyUnsupportedOnNoDcacheCore {
                    name: "rx_pool_sram1".into(),
                    machine: "mcu_node".into(),
                    declared_policy: "maintain".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:3144879193ab848a","code":"mem/cache-policy-unsupported-on-no-dcache-core","stage":"validation","spec":"watching-zenoh RFC §5.E","message":"buffer-pool 'rx_pool_sram1': `cache-policy: maintain` declared on machine 'mcu_node' which has `platform.has_dcache: false`. Cache maintenance is meaningless on a core without a data cache. Switch to `cache-policy: none`.","actual":"maintain","fix":{"kind":"replace_one_of","candidates":["none"]}}"#,
            ),
            // ── §5.E C5 author guard: <sce:extern> for cache trio rejected per spec lines 1222-1227 ─
            (
                "forge/pool-cache-maintenance-misplaced",
                ValidationError::PoolCacheMaintenanceMisplaced {
                    attempted_symbol: "sce_dcache_clean_by_addr".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:dd94b2147bfbcdf3","code":"pool/cache-maintenance-misplaced","stage":"validation","spec":"watching-zenoh RFC §5.E","message":"<sce:extern name=\"sce_dcache_clean_by_addr\">: cache-maintenance intrinsics are FSM-driven and authored automatically by the buffer-pool kind under `cache-policy: maintain` (RFC §5.E lines 1222-1227). Author <sce:extern> for the cache trio is forbidden — remove the declaration; codegen emits the calls on lifecycle edges.","actual":"sce_dcache_clean_by_addr"}"#,
            ),
            // ── §5.E C5 config-completeness: has_dcache=true requires has_speculative_prefetch ─
            (
                "forge/pool-speculative-prefetch-flag-missing",
                ValidationError::PoolSpeculativePrefetchFlagMissing {
                    machine: "mcu_node".into(),
                    pool_name: "rx_pool_sram1".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:101a5c631d3b5278","code":"pool/speculative-prefetch-flag-missing","stage":"validation","spec":"watching-zenoh RFC §5.E","message":"machine 'mcu_node': `platform.has_dcache: true` is set but `platform.has_speculative_prefetch` is not. Buffer-pool 'rx_pool_sram1' uses `cache-policy: maintain` and codegen cannot decide whether to emit the pre-DMA-RX invalidate edge. Declare `has_speculative_prefetch` per the SoC datasheet (M7+/A-class = true, M3/M4 = false)."}"#,
            ),
            // ── §5.E C5 codegen self-check: pre-arm cache-invalidate edge missing on speculative core ─
            (
                "forge/pool-cache-pre-arm-invalidate-missing-on-speculative-core",
                ValidationError::PoolCachePreArmInvalidateMissingOnSpeculativeCore {
                    name: "rx_pool_sram1".into(),
                    backend: "rust".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:e043d4876ee880bd","code":"pool/cache-pre-arm-invalidate-missing-on-speculative-core","stage":"validation","spec":"watching-zenoh RFC §5.E","message":"buffer-pool 'rx_pool_sram1': generated source for backend `rust` is missing the `sce_dcache_invalidate_by_addr` call on the `free → dma-armed-rx` edge despite `cache-policy: maintain` + `platform.has_speculative_prefetch: true` — codegen invariant violation per RFC §5.E lines 1186-1198 + 1552; report at https://github.com/newmassrael/scxml-core-engine/issues"}"#,
            ),
            // ── §5.O Atomic 0 IR provenance pre-emit guard ─────────
            //    Codegen-internal invariant: a node eligible for SCE-
            //    MAP marker emission carries `source_location: None`.
            //    Author-facing fields stay empty (`actual: None`,
            //    `expected: None`, `fix: None`); `node_kind` + `node_id`
            //    ride `key_fragments` and surface through `message`.
            (
                "traceability/scxml-line-range-missing",
                ValidationError::TraceabilityScxmlLineRangeMissing {
                    node_kind: "<state>",
                    node_id: "S0".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:c468c384e2af3fcf","code":"traceability/scxml-line-range-missing","stage":"generate","spec":"watching-zenoh RFC §5.O","message":"<state> 'S0': source_location not populated — §5.O Atomic 0 pre-emit guard (parser site missed)"}"#,
            ),
            // ── §5.O Atomic 1 — symbol mangling + sourcemap contract goldens ──
            //    Dual-location collision report. `actual` is the
            //    colliding mangled symbol; the `Fix::ReplaceOneOf`
            //    candidate list names the two offending sites so the
            //    agent / human picks which to rename.
            (
                "traceability/state-id-collision",
                ValidationError::TraceabilityStateIdCollision {
                    mangled: "motor__armed___state_body".into(),
                    first_file: "motor.scxml".into(),
                    first_line: 10,
                    second_file: "imports/armed.scxml".into(),
                    second_line: 4,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:acb9750dd735a718","code":"traceability/state-id-collision","stage":"generate","spec":"watching-zenoh RFC §5.O","message":"symbol collision: 'motor__armed___state_body' maps to two IR nodes — motor.scxml:10 and imports/armed.scxml:4. Repair: rename one of the colliding ids so the mangled symbols differ","actual":"motor__armed___state_body","fix":{"kind":"replace_one_of","candidates":["motor.scxml:10","imports/armed.scxml:4"]}}"#,
            ),
            //    Symbol length cap (C99 §5.2.4.1, 31 chars). `actual`
            //    is the offending mangled id; the key_fragments triple
            //    keys the wire-id off the id + over_by count so two
            //    distinct overflows hash to distinct records.
            (
                "traceability/symbol-name-exceeds-c-identifier-limit",
                ValidationError::TraceabilitySymbolNameExceedsCIdentifierLimit {
                    mangled: "very_long_machine__nested_state_path___state_body".into(),
                    actual_len: 49,
                    over_by: 18,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:9af98c119a0b235a","code":"traceability/symbol-name-exceeds-c-identifier-limit","stage":"generate","spec":"watching-zenoh RFC §5.O","message":"mangled symbol 'very_long_machine__nested_state_path___state_body' exceeds C99 external identifier limit by 18 char(s) (got 49, max 31). Repair: shorten one of the contributing names (machine id, state id, or artifact suffix) or enable `platform.strict_c99_identifiers: false` in deploy.yaml to suppress this warning","actual":"very_long_machine__nested_state_path___state_body"}"#,
            ),
            //    Sourcemap source_hash drift (codegen-invariant).
            //    `actual` carries the sourcemap-recorded hex hash;
            //    `header_hash` lives in the message body so the wire
            //    payload doesn't violate the actual/expected non-overlap.
            (
                "traceability/sourcemap-source-hash-mismatch",
                ValidationError::TraceabilitySourcemapSourceHashMismatch {
                    file: "out/rust/sce_sourcemap.json".into(),
                    sourcemap_hash: "abc123def456".into(),
                    header_hash: "789aaaabbbb0".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:d7a5b77c40fb62ef","code":"traceability/sourcemap-source-hash-mismatch","stage":"generate","spec":"watching-zenoh RFC §5.O","message":"sourcemap source_hash drift: sourcemap recorded 'abc123def456' but §6.2.6 header recorded '789aaaabbbb0' on out/rust/sce_sourcemap.json. Repair: regenerate via `sce-codegen generate` to rebuild both sides from the same inputs","actual":"abc123def456"}"#,
            ),
            //    Rust SCE-MAP `#[doc]` preservation heads-up (OQ-W16 b).
            //    `actual` is the function name; `crate_name` + `profile`
            //    ride `key_fragments` so the wire-id distinguishes
            //    distinct strip sites.
            (
                "traceability/sce-map-attribute-stripped",
                ValidationError::TraceabilitySceMapAttributeStripped {
                    crate_name: "sce_rust_tests".into(),
                    function: "test144::on_entry_s0_0".into(),
                    profile: "release".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:eefcdcd7864bca4d","code":"traceability/sce-map-attribute-stripped","stage":"generate","spec":"watching-zenoh RFC §5.O","message":"SCE-MAP `#[doc]` marker stripped from 'test144::on_entry_s0_0' in sce_rust_tests (release); falling back to `// SCE-MAP:` line comments. Repair: re-emit with the dual-marker form (default since §5.O Atomic 0c) or upstream the rustdoc fix","actual":"test144::on_entry_s0_0"}"#,
            ),
            //    SCE-MAP marker missing on a §6.2.6-headered file
            //    (codegen-internal invariant per ARCHITECTURE.md
            //    "Traceability Ownership Boundary"). `actual` carries
            //    the offending file path; `key_fragments` reuse the
            //    same path so the wire-id is path-stable.
            (
                "traceability/meta-generated-source-line-marker-missing",
                ValidationError::TraceabilityMetaGeneratedSourceLineMarkerMissing {
                    file: "out/test144/test144_sm.rs".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:052d4d7f20b67021","code":"traceability/meta-generated-source-line-marker-missing","stage":"generate","spec":"watching-zenoh RFC §5.O","message":"emitted file 'out/test144/test144_sm.rs' carries a §6.2.6 drift header but no `SCE-MAP:` marker line. Per ARCHITECTURE.md \"Traceability Ownership Boundary\", every SCE-emitted file must carry at least one marker. Repair: a template under `tools/codegen/templates/` is missing its `sce_map_marker` macro call — report upstream","actual":"out/test144/test144_sm.rs"}"#,
            ),
            // ── Round F-α (watching-zenoh RFC §5.2) — driver header
            //    reference cannot be resolved against the platform
            //    driver_root (or SCXML file's parent). `actual` carries
            //    the verbatim href; `key_fragments` add both href and
            //    resolved_dir so identical-named misses under distinct
            //    roots hash distinct wire-ids.
            (
                "mcu/driver-header-not-found",
                ValidationError::McuDriverHeaderNotFound {
                    href: "missing.h".into(),
                    resolved_dir: "/tmp/round_f_alpha".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:a887dfe8c01e0694","code":"mcu/driver-header-not-found","stage":"validation","spec":"watching-zenoh RFC §5.2","message":"driver header reference 'missing.h' could not be resolved (searched under '/tmp/round_f_alpha'). Repair: correct the `<sce:driver href=\"...\"/>` value, add the missing header, or set `platform.driver_root` in deploy.yaml so the relative path resolves.","actual":"missing.h"}"#,
            ),
            // ── Round F-α (watching-zenoh RFC §5.2) — non-MCU backend
            //    refuses `platform.c11_section_attribute` (Q-Round-F-D3,
            //    mirrors Q-Call-7). `actual` carries the offending
            //    backend; `key_fragments` reuse the same single value
            //    so the wire-id is stable per backend across runs.
            (
                "mcu/section-attribute-on-non-mcu-target",
                crate::forge::error::GenerateError::McuSectionAttributeOnNonMcuTarget {
                    backend: "rust".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:d9ee1c8383fc6b8f","code":"mcu/section-attribute-on-non-mcu-target","stage":"generate","spec":"watching-zenoh RFC §5.2","message":"platform.c11_section_attribute is set in deploy.yaml but the target backend is 'rust', not 'c11'. The section attribute injects `__attribute__((section(...)))` which only the C11 backend emits. Repair: remove the section attribute, switch the backend to 'c11', or split deploy configurations per target.","actual":"rust"}"#,
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
                "mesh/deploy-stage-pool-not-declared",
                DeployError::StagePoolNotDeclared {
                    machine: "mcu_node".into(),
                    binding: "#sub".into(),
                    stage_pool: "rx_pool_sram1".into(),
                    candidates: vec!["scout_rx_pool".into()],
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion
                // on first run. Shape + message are the contract.
                r#"{"v":1,"id":"fnv1a:43c3861fc17b431c","code":"mesh/deploy-stage-pool-not-declared","stage":"mesh-deploy","spec":"watching-zenoh RFC §5.E","message":"machine 'mcu_node': binding '#sub' references stage_pool 'rx_pool_sram1' but no `.forge` file in the build declares a pool by that name. Add a forge `<scxml sce:kind=\"buffer-pool\" name=\"rx_pool_sram1\">` document or fix the reference. See watching-zenoh RFC §5.E.","actual":"rx_pool_sram1","fix":{"kind":"replace_one_of","candidates":["scout_rx_pool"]}}"#,
            ),
            (
                "mesh/deploy-stage-pool-wrong-kind",
                DeployError::StagePoolWrongKind {
                    machine: "mcu_node".into(),
                    binding: "#sub".into(),
                    stage_pool: "scout_codec".into(),
                    actual_kind: "codec".into(),
                    candidates: vec!["scout_rx_pool".into()],
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:e05f280783ac494b","code":"mesh/deploy-stage-pool-wrong-kind","stage":"mesh-deploy","spec":"watching-zenoh RFC §5.E","message":"machine 'mcu_node': binding '#sub' references stage_pool 'scout_codec' which resolves to a forge 'codec' kind, not 'buffer-pool'. Only buffer-pool kind documents back the `Sample::take()` slot contract. Repoint the reference at one of the build's buffer-pool kind names. See watching-zenoh RFC §5.E.","actual":"codec","fix":{"kind":"replace_one_of","candidates":["scout_rx_pool"]}}"#,
            ),
            (
                "mesh/deploy-stage-pool-transport-mismatch",
                DeployError::StagePoolTransportMismatch {
                    machine: "mcu_node".into(),
                    binding: "#sub".into(),
                    stage_pool: "scout_rx_pool".into(),
                    transport: "zenoh".into(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:5e3fe77a13dd48d3","code":"mesh/deploy-stage-pool-transport-mismatch","stage":"mesh-deploy","spec":"watching-zenoh RFC §5.E","message":"machine 'mcu_node': binding '#sub' declares stage_pool 'scout_rx_pool' on transport 'zenoh', which has no buffer-pool RX staging surface. The `stage_pool` field is meaningful only for transports that bind a forge buffer-pool kind on their RX path. Drop the field or change the transport. See watching-zenoh RFC §5.E.","actual":"zenoh","fix":{"kind":"remove_fields","location":"topology.*.machines.mcu_node.bindings.#sub","fields":["stage_pool"]}}"#,
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
                "mesh/deploy-partition-name-not-identifier",
                DeployError::PartitionNameNotIdentifier {
                    partition: "motor-left".into(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion
                // on first run; shape + message are the contract.
                r#"{"v":1,"id":"fnv1a:f34df70fd07c3a96","code":"mesh/deploy-partition-name-not-identifier","stage":"mesh-deploy","spec":"SCE Mesh §14","message":"partition 'motor-left' is not a valid C++ identifier: must start with a letter or underscore and contain only letters, digits, and underscores. Codegen bakes this name into `SCE::Generated::<machine>::P_motor-left` (SCE_MESH.md §14 arch-debt #4 closure) — non-identifier characters would emit non-compiling C++. Rename the partition in deploy.yaml.","actual":"motor-left"}"#,
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
                "mesh/deploy-someip-liveness-service-id-overflow",
                DeployError::SomeipLivenessServiceIdOverflow {
                    participant_count: 129,
                    ceiling: 128,
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion
                // on first run; shape + message are the contract.
                r#"{"v":1,"id":"fnv1a:9c5db6329d9656ab","code":"mesh/deploy-someip-liveness-service-id-overflow","stage":"mesh-deploy","spec":"SCE Mesh §16.4","message":"§16.4 SOME/IP region-liveness service-ID overflow: 129 partitions exceed the 128-slot sub-range ceiling [0x8180, 0x81FF] (RFC F.X-3 subsystem range partitioning reserves the upper half of the SCE-reserved space for region-liveness, disjoint from §9.6 invoke's [0x8100, 0x817F]). Reduce the §16.4 SOMEIP partition count or split deploy.yaml across multi-OEM domains.","actual":"129"}"#,
            ),
            (
                "mesh/deploy-someip-liveness-service-id-pin-out-of-range",
                DeployError::SomeipLivenessServiceIdPinOutOfRange {
                    partition_key: "brake__P__left".into(),
                    pinned_id: 0x817F,
                    range_lo: 0x8180,
                    range_hi: 0x81FF,
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion
                // on first run; shape + message are the contract.
                r#"{"v":1,"id":"fnv1a:806588814d3ea3e2","code":"mesh/deploy-someip-liveness-service-id-pin-out-of-range","stage":"mesh-deploy","spec":"SCE Mesh §16.4","message":"partition 'brake__P__left': pinned `someip_liveness_service_id: 0x817f` is outside the §16.4 SOMEIP region-liveness sub-range [0x8180, 0x81ff] (RFC F.X-3). The lower half of the SCE-reserved range is reserved for §9.6 scxml-invoke; pins outside the SCE-reserved range collide with OEM-owned service space. Pick a value inside [0x8180, 0x81ff] or drop the pin to use the auto-assigner.","actual":"0x817f"}"#,
            ),
            (
                "mesh/deploy-someip-liveness-service-id-pin-collision",
                DeployError::SomeipLivenessServiceIdPinCollision {
                    partition_keys: vec![
                        "alpha__P__l".into(),
                        "beta__P__r".into(),
                    ],
                    pinned_id: 0x8185,
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion
                // on first run; shape + message are the contract.
                r#"{"v":1,"id":"fnv1a:48369888c142d0ed","code":"mesh/deploy-someip-liveness-service-id-pin-collision","stage":"mesh-deploy","spec":"SCE Mesh §16.4","message":"§16.4 SOME/IP region-liveness service-ID pin collision at 0x8185: partitions ['alpha__P__l', 'beta__P__r'] all pin the same value via deploy.yaml `someip_liveness_service_id:`. Each pin must be unique inside the [0x8180, 0x81FF] sub-range. Repick the pin on one of the listed partitions or drop a pin to fall back to the counter auto-assigner.","actual":"0x8185"}"#,
            ),
            (
                "mesh/deploy-someip-machine-liveness-service-id-overflow",
                DeployError::SomeipMachineLivenessServiceIdOverflow {
                    participant_count: 129,
                    ceiling: 128,
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion
                // on first run; shape + message are the contract.
                r#"{"v":1,"id":"fnv1a:00af94fa6eca6527","code":"mesh/deploy-someip-machine-liveness-service-id-overflow","stage":"mesh-deploy","spec":"SCE Mesh §16.7","message":"§16.7 row 8 SOME/IP machine-liveness service-ID overflow: 129 machines exceed the 128-slot sub-range ceiling [0x8280, 0x82FF] (RFC F.X-4 subsystem range partitioning reserves a third disjoint 128-slot sub-range for machine-level liveness, disjoint from §9.6 invoke's [0x8100, 0x817F] and §16.4 region-liveness's [0x8180, 0x81FF]). Drop `liveliness:` from some SOME/IP machines, switch them to Zenoh transport, or split deploy.yaml across multi-OEM domains.","actual":"129"}"#,
            ),
            (
                "mesh/deploy-someip-machine-liveness-service-id-pin-out-of-range",
                DeployError::SomeipMachineLivenessServiceIdPinOutOfRange {
                    machine: "brake_ctrl".into(),
                    pinned_id: 0x827F,
                    range_lo: 0x8280,
                    range_hi: 0x82FF,
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion
                // on first run; shape + message are the contract.
                r#"{"v":1,"id":"fnv1a:2262f524a063ef22","code":"mesh/deploy-someip-machine-liveness-service-id-pin-out-of-range","stage":"mesh-deploy","spec":"SCE Mesh §16.7","message":"machine 'brake_ctrl': pinned `someip_machine_liveness_service_id: 0x827f` is outside the §16.7 row 8 SOME/IP machine-liveness sub-range [0x8280, 0x82ff] (RFC F.X-4). The lower SCE-reserved sub-ranges are reserved for §9.6 scxml-invoke and §16.4 region-liveness; pins outside the SCE-reserved namespace collide with OEM-owned service space. Pick a value inside [0x8280, 0x82ff] or drop the pin to use the auto-assigner.","actual":"0x827f"}"#,
            ),
            (
                "mesh/deploy-someip-machine-liveness-service-id-pin-collision",
                DeployError::SomeipMachineLivenessServiceIdPinCollision {
                    machines: vec!["alpha".into(), "beta".into()],
                    pinned_id: 0x8285,
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion
                // on first run; shape + message are the contract.
                r#"{"v":1,"id":"fnv1a:4a0d4b54f2c7ffe4","code":"mesh/deploy-someip-machine-liveness-service-id-pin-collision","stage":"mesh-deploy","spec":"SCE Mesh §16.7","message":"§16.7 row 8 SOME/IP machine-liveness service-ID pin collision at 0x8285: machines ['alpha', 'beta'] all pin the same value via deploy.yaml `someip_machine_liveness_service_id:`. Each pin must be unique inside the [0x8280, 0x82FF] sub-range. Repick the pin on one of the listed machines or drop a pin to fall back to the counter auto-assigner.","actual":"0x8285"}"#,
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
                "mesh/deploy-platform-class-os-mismatch",
                DeployError::PlatformClassOsMismatch {
                    machine: "mcu_node".into(),
                    class: "mcu",
                    os: "linux",
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:9195176fda736ee8","code":"mesh/deploy-platform-class-os-mismatch","stage":"mesh-deploy","spec":"SCE Mesh §14","message":"machine 'mcu_node': platform.class 'mcu' is not compatible with platform.os 'linux'. SCE Mesh §14 (RFC §5.K) admits 'mcu' with os ∈ {bare_metal, rtos} and 'ap' with os ∈ {linux, qnx, macos, freebsd, windows}. Repair: change either field so the pair becomes admissible, or drop the platform: section to leave the machine unclassified.","actual":"linux"}"#,
            ),
            (
                "deploy/worker-stack-budget-missing",
                DeployError::SchedulerCooperativeMissingStackBudget {
                    machine: "mcu_node".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:936f4785a68e36b5","code":"deploy/worker-stack-budget-missing","stage":"mesh-deploy","spec":"watching-zenoh RFC §5.K","message":"machine 'mcu_node': scheduler.kind 'cooperative' requires scheduler.worker_stack_budget (bytes). watching-zenoh RFC §5.K line 2426 (`deploy/worker-stack-budget-missing`) — cooperative drives the `<send>` queue inside a fixed stack frame; a missing budget would let TLV-decode recursion silently overflow. Repair: add `worker_stack_budget: <bytes>` under `scheduler:` (e.g. 4096), or change `kind:` to `tokio` / `rt` to inherit the host runtime's stack defaults.","actual":"mcu_node"}"#,
            ),
            (
                "deploy/worker-slot-budget-missing",
                DeployError::SchedulerCooperativeMissingSlotBudget {
                    machine: "mcu_node".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:c7e272ab44356ef1","code":"deploy/worker-slot-budget-missing","stage":"mesh-deploy","spec":"watching-zenoh RFC §5.K","message":"machine 'mcu_node': scheduler.kind 'cooperative' requires scheduler.worker_slot_budget_us (microseconds). watching-zenoh RFC §5.K line 2428-2429 (`deploy/worker-slot-budget-missing`) — per-slot WCET ceiling drives the §5.B aggregate WCET check and the cooperative slot-count derivation. Repair: add `worker_slot_budget_us: <us>` under `scheduler:` (e.g. 200), or change `kind:` to `tokio` / `rt` to skip the WCET check.","actual":"mcu_node"}"#,
            ),
            (
                "deploy/keepalive-jitter-budget-missing",
                DeployError::SchedulerCooperativeMissingKeepaliveJitterBudget {
                    machine: "mcu_node".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:79db8ea5ee156731","code":"deploy/keepalive-jitter-budget-missing","stage":"mesh-deploy","spec":"watching-zenoh RFC §5.K","message":"machine 'mcu_node': scheduler.kind 'cooperative' requires scheduler.keepalive_jitter_budget_us (microseconds). watching-zenoh RFC §5.K line 2430-2431 (`deploy/keepalive-jitter-budget-missing`) — sum of worst-case slot budgets in one tick window must fit inside this bound. Repair: add `keepalive_jitter_budget_us: <us>` under `scheduler:` (recommended default: 0.5 × min lease), or change `kind:` to `tokio` / `rt` to inherit host runtime jitter.","actual":"mcu_node"}"#,
            ),
            (
                "deploy/scheduler-incompatible-with-worker-count",
                DeployError::SchedulerIncompatibleWithWorkerCount {
                    machine: "mcu_node".into(),
                    worker_count: 5,
                    slot_count: 3,
                    tick_period_us: 1000,
                    worker_slot_budget_us: 300,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:f26a225afadc7a9a","code":"deploy/scheduler-incompatible-with-worker-count","stage":"mesh-deploy","spec":"watching-zenoh RFC §5.K","message":"machine 'mcu_node': declared 5 workers under machines.mcu_node.workers, but cooperative scheduler can host only 3 per tick window (derived from tick_period_us 1000 / worker_slot_budget_us 300). watching-zenoh RFC §5.K line 2423 (`deploy/scheduler-incompatible-with-worker-count`). Repair: raise `tick_period_us`, lower `worker_slot_budget_us`, remove excess workers, or switch `scheduler.kind:` to a preemptive host (`tokio` / `rt`).","expected":["3"],"actual":"5"}"#,
            ),
            // ── C13-α-1 §5.K `links:` block (RFC §5.K lines 2232-2540) ──
            (
                "deploy/link-driver-unknown",
                DeployError::LinkDriverUnknown {
                    machine: "mcu_node".into(),
                    link_name: "udp_data".into(),
                    driver: "foo_udp".into(),
                    candidates: vec!["lwip_tcp".into(), "lwip_udp".into()],
                    candidates_list: "lwip_tcp, lwip_udp".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:29690d741179d6f1","code":"deploy/link-driver-unknown","stage":"mesh-deploy","spec":"watching-zenoh RFC §5.K","message":"machine 'mcu_node': link 'udp_data' declares driver 'foo_udp' which is unknown. watching-zenoh RFC §5.K line 2421 (`deploy/link-driver-unknown`) — the build's closed-allowlist + forge `<sce:link>` cross-doc registry union does not contain this driver. Repair: pick one of [lwip_tcp, lwip_udp].","actual":"foo_udp","fix":{"kind":"replace_one_of","candidates":["lwip_tcp","lwip_udp"]}}"#,
            ),
            (
                "deploy/link-mtu-missing-on-fragmenting-link",
                DeployError::LinkMtuMissingOnFragmentingLink {
                    machine: "mcu_node".into(),
                    link_name: "udp_data".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:863c8c160c2fe018","code":"deploy/link-mtu-missing-on-fragmenting-link","stage":"mesh-deploy","spec":"watching-zenoh RFC §5.K","message":"machine 'mcu_node': link 'udp_data' declares `domain_attrs.trust_class: established_session` but `mtu_bytes:` is absent. watching-zenoh RFC §5.K line 2440-2442 (`deploy/link-mtu-missing-on-fragmenting-link`) — only `established_session` trust class carries Fragment traffic (RFC §5.M line 2731) and the build cannot size reassembly pool slots without the link-layer MTU. Repair: add `mtu_bytes: <bytes>` under this link entry (e.g. 1472 for UDP/IPv4 over Ethernet)."}"#,
            ),
            (
                "deploy/link-mtu-below-driver-floor",
                DeployError::LinkMtuBelowDriverFloor {
                    machine: "mcu_node".into(),
                    link_name: "udp_data".into(),
                    driver: "lwip_udp".into(),
                    declared_mtu: 20,
                    driver_floor: 28,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:b923c818f30343df","code":"deploy/link-mtu-below-driver-floor","stage":"mesh-deploy","spec":"watching-zenoh RFC §5.K","message":"machine 'mcu_node': link 'udp_data' declares `mtu_bytes: 20` which is below driver 'lwip_udp's minimum payload floor (28). watching-zenoh RFC §5.K line 2443-2445 (`deploy/link-mtu-below-driver-floor`) — the driver's default minimum would override the declared value silently. Repair: raise `mtu_bytes` to >= 28, or change the driver to one with a smaller header floor.","expected":["28"],"actual":"20"}"#,
            ),
            (
                // C11-WebSocket follow-up sibling — driver↔class
                // cross-validator. Forge declares `websocket` but
                // deploy binds `lwip_tcp` whose class is `tcp`;
                // the validator surfaces the mismatch with the
                // candidate driver list (single-element: the
                // driver implementing `websocket` = `websocket_tcp`).
                "deploy/link-driver-class-mismatch",
                DeployError::LinkDriverClassMismatch {
                    machine: "mcu_node".into(),
                    link_name: "ws_control".into(),
                    driver: "lwip_tcp".into(),
                    declared_class: "websocket".into(),
                    expected_class: "tcp".into(),
                    driver_candidates: vec!["websocket_tcp".into()],
                    driver_candidates_list: "websocket_tcp".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:4ac5fbe980bb4938","code":"deploy/link-driver-class-mismatch","stage":"mesh-deploy","spec":"watching-zenoh RFC §5.K","message":"machine 'mcu_node': link 'ws_control' declares forge `<sce:link-class>websocket</sce:link-class>` but deploy.yaml binds `driver: lwip_tcp` which implements class 'tcp'. watching-zenoh RFC §5.C lines 765-771 + §8 Q8 line 3747 (`deploy/link-driver-class-mismatch`) — each core driver implements exactly one protocol class. Repair: change `driver:` to the entry matching the declared class, or change `<sce:link-class>` to match the bound driver.","expected":["tcp"],"actual":"websocket","fix":{"kind":"replace_one_of","candidates":["websocket_tcp"]}}"#,
            ),
            (
                "deploy/link-expected-p99-exceeds-mtu",
                DeployError::LinkExpectedP99ExceedsMtu {
                    machine: "mcu_node".into(),
                    link_name: "udp_data".into(),
                    expected_p99_bytes: 2048,
                    mtu_bytes: 1472,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:04a20137542a8e63","code":"deploy/link-expected-p99-exceeds-mtu","stage":"mesh-deploy","spec":"watching-zenoh RFC §5.K","message":"machine 'mcu_node': link 'udp_data' declares `expected_p99_bytes: 2048` which exceeds `mtu_bytes: 1472`. watching-zenoh RFC §5.K line 2446-2448 (`deploy/link-expected-p99-exceeds-mtu`) — the p99 message would always fragment. Repair: lower `expected_p99_bytes` to <= `mtu_bytes`, or raise `mtu_bytes` (driver permitting), or bind a reassembly pool to this link via a forge `<sce:link>` declaration.","expected":["1472"],"actual":"2048"}"#,
            ),
            (
                "deploy/link-burst-pps-missing-on-isr-dispatch",
                DeployError::LinkBurstPpsMissingOnIsrDispatch {
                    machine: "mcu_node".into(),
                    link_name: "udp_data".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:ae5e6142a2e8c826","code":"deploy/link-burst-pps-missing-on-isr-dispatch","stage":"mesh-deploy","spec":"watching-zenoh RFC §5.K","message":"machine 'mcu_node': link 'udp_data' resolves to `rx_dispatch: isr_to_pool` but `burst_pps` is not declared. watching-zenoh RFC §5.K line 2501-2503 (`deploy/link-burst-pps-missing-on-isr-dispatch`) — ISR fast-path requires `burst_pps` to size the descriptor ring and validate the stack budget. Repair: declare `burst_pps: <pps>`, or explicitly set `rx_dispatch: worker_tick` to opt into the slower cooperative-tick path."}"#,
            ),
            (
                // C13-α-2 cross-doc: declared burst_pps overruns RX
                // pool drain capacity within one cooperative tick.
                "deploy/link-burst-absorption-insufficient",
                DeployError::LinkBurstAbsorptionInsufficient {
                    machine: "mcu_node".into(),
                    link_name: "udp_data".into(),
                    pool_name: "rx_data_pool".into(),
                    slot_count: 16,
                    burst_pps: 50_000,
                    tick_period_us: 1000,
                    drain_per_second: 16_000,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:149ee464a43d20f6","code":"deploy/link-burst-absorption-insufficient","stage":"mesh-deploy","spec":"watching-zenoh RFC §5.K","message":"machine 'mcu_node': link 'udp_data' declares `burst_pps: 50000` against RX pool 'rx_data_pool' with `<sce:slot-count>16</sce:slot-count>` and scheduler `tick_period_us: 1000`. Effective drain capacity is 16000 pps (with the 2.0 safety factor required by watching-zenoh RFC §5.K line 2489-2495), insufficient for the declared burst. Repair: raise `<sce:slot-count>` on pool 'rx_data_pool', lower `scheduler.tick_period_us`, or switch `rx_dispatch: isr_to_pool` when currently `worker_tick`.","expected":["16000"],"actual":"50000"}"#,
            ),
            (
                // C13-α-2 cross-doc: rx_dispatch: worker_tick overruns
                // RX pool in one tick window.
                "deploy/link-rx-dispatch-worker-tick-on-high-burst",
                DeployError::LinkRxDispatchWorkerTickOnHighBurst {
                    machine: "mcu_node".into(),
                    link_name: "udp_data".into(),
                    pool_name: "rx_data_pool".into(),
                    slot_count: 16,
                    burst_pps: 100_000,
                    tick_period_us: 1000,
                    arrivals_per_tick: 100,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:f396cb2184d51ea2","code":"deploy/link-rx-dispatch-worker-tick-on-high-burst","stage":"mesh-deploy","spec":"watching-zenoh RFC §5.K","message":"machine 'mcu_node': link 'udp_data' resolves to `rx_dispatch: worker_tick` but one tick window of arrivals overruns RX pool 'rx_data_pool'. `burst_pps × tick_period_us / 1_000_000 = 100` exceeds `<sce:slot-count>16</sce:slot-count>`. watching-zenoh RFC §5.K line 2496-2500 (`deploy/link-rx-dispatch-worker-tick-on-high-burst`). Repair: switch `rx_dispatch: isr_to_pool` (descriptor-ring re-arm absorbs the burst), raise `<sce:slot-count>` on pool 'rx_data_pool' to admit the per-tick arrivals, or lower `scheduler.tick_period_us` so each window admits fewer arrivals.","expected":["16"],"actual":"100"}"#,
            ),
            (
                "deploy/link-not-declared-in-deploy",
                DeployError::LinkNotDeclaredInDeploy {
                    link_name: "udp_data".into(),
                    candidates: vec!["udp_scout".into()],
                    candidates_list: "udp_scout".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:7769451379390caa","code":"deploy/link-not-declared-in-deploy","stage":"mesh-deploy","spec":"watching-zenoh RFC §5.K","message":"forge `<sce:link name=\"udp_data\">` declared but no `deploy.yaml::machines.<n>.links.udp_data` entry exists. C13-α-1 cross-doc validator (`deploy/link-not-declared-in-deploy`) per Q-C13-5 (a) lock. Repair: add the deploy entry under one of [udp_scout] or another machine, or remove the forge link doc.","actual":"udp_data","fix":{"kind":"replace_one_of","candidates":["udp_scout"]}}"#,
            ),
            (
                "deploy/link-not-declared-in-forge",
                DeployError::LinkNotDeclaredInForge {
                    machine: "mcu_node".into(),
                    link_name: "udp_data".into(),
                    candidates: vec!["udp_scout".into()],
                    candidates_list: "udp_scout".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:68d69cab5e950a11","code":"deploy/link-not-declared-in-forge","stage":"mesh-deploy","spec":"watching-zenoh RFC §5.K","message":"machine 'mcu_node': link 'udp_data' declared in deploy.yaml but no forge `<scxml sce:kind=\"link\" name=\"udp_data\">` document was declared/imported. C13-α-1 cross-doc validator (`deploy/link-not-declared-in-forge`) per Q-C13-5 (a) lock. Repair: declare the forge link doc and import it from a statechart/worker on this machine, or pick one of [udp_scout] (forge link doc names known to this build), or remove the orphan deploy entry.","actual":"udp_data","fix":{"kind":"replace_one_of","candidates":["udp_scout"]}}"#,
            ),
            (
                // C13-γ parse-time typo guard for the policy enum.
                // FixCarriesCandidates over StageCopyPolicy::ALL.
                "deploy/stage-copy-policy-unknown",
                DeployError::StageCopyPolicyUnknown {
                    machine: "mcu_node".into(),
                    value: "errr".into(),
                    candidates: vec![
                        "warn".into(),
                        "error".into(),
                        "forbid".into(),
                    ],
                    candidates_list: "warn, error, forbid".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:5ca8faec9e11c481","code":"deploy/stage-copy-policy-unknown","stage":"mesh-deploy","spec":"watching-zenoh RFC §5.K","message":"machine 'mcu_node': `pool_defaults.stage_copy_policy: errr` is not a known policy. watching-zenoh RFC §5.K line 2517-2519 (`deploy/stage-copy-policy-unknown`) — closed-set typo guard. Repair: pick one of [warn, error, forbid].","actual":"errr","fix":{"kind":"replace_one_of","candidates":["warn","error","forbid"]}}"#,
            ),
            // ── C13-β anti-flood + stateless_accept (RFC §5.K lines 2272-2349 + 2449-2473) ──
            (
                "deploy/session-arming-quota-missing",
                DeployError::SessionArmingQuotaMissing {
                    machine: "mcu_node".into(),
                    link_name: "udp_listener".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:f5f597205aa5b0e5","code":"deploy/session-arming-quota-missing","stage":"mesh-deploy","spec":"watching-zenoh RFC §5.K","message":"machine 'mcu_node': link 'udp_listener' declares `trust_class: session_arming` but no `session_arming_quota`. watching-zenoh RFC §5.K line 2449-2451 — without a cap an attacker can fill every `Accepting.*` slot. Repair: declare `session_arming_quota: <count>` (MCU recommended 8, AP recommended 32 per spec line 2282)."}"#,
            ),
            (
                "deploy/accept-rate-config-missing",
                DeployError::AcceptRateConfigMissing {
                    machine: "mcu_node".into(),
                    link_name: "udp_listener".into(),
                    missing_fields: "accept_rate_per_sec, accept_rate_burst".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:6fbd647f1bd1aeb1","code":"deploy/accept-rate-config-missing","stage":"mesh-deploy","spec":"watching-zenoh RFC §5.K","message":"machine 'mcu_node': link 'udp_listener' declares `trust_class: session_arming` but missing accept-rate config: accept_rate_per_sec, accept_rate_burst. watching-zenoh RFC §5.K line 2452-2453 — token-bucket rate-limit is required to prevent half-open quota saturation. Repair: declare both `accept_rate_per_sec` and `accept_rate_burst` (spec line 2290-2302 recommends defaults `accept_rate_per_sec: 4` MCU / `16` AP and `accept_rate_burst: 2 × accept_rate_per_sec`).","actual":"accept_rate_per_sec, accept_rate_burst"}"#,
            ),
            (
                "deploy/session-arming-fields-on-non-arming-link",
                DeployError::SessionArmingFieldsOnNonArmingLink {
                    machine: "mcu_node".into(),
                    link_name: "udp_data".into(),
                    trust_class: "established_session".into(),
                    offending_fields: "session_arming_quota".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:a981ec0d75389842","code":"deploy/session-arming-fields-on-non-arming-link","stage":"mesh-deploy","spec":"watching-zenoh RFC §5.K","message":"machine 'mcu_node': link 'udp_data' declares `trust_class: established_session` but anti-flood / stateless_accept fields are present (session_arming_quota). watching-zenoh RFC §5.K line 2454-2459 — `Accepting.*` is never instantiated on this trust class so the fields are dead config (suggests author confusion about which link is the listener). Repair: change `trust_class` to `session_arming` on link 'udp_data' if it is in fact the listener, or remove the dead fields.","actual":"established_session"}"#,
            ),
            (
                "deploy/stateless-accept-required-on-untrusted-source",
                DeployError::StatelessAcceptRequiredOnUntrustedSource {
                    machine: "mcu_node".into(),
                    link_name: "udp_listener".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:06e17061b74e6486","code":"deploy/stateless-accept-required-on-untrusted-source","stage":"mesh-deploy","spec":"watching-zenoh RFC §5.K","message":"machine 'mcu_node': link 'udp_listener' declares `domain_attrs.untrusted_source: true` but no `stateless_accept` block. watching-zenoh RFC §5.K line 2463-2465 — links exposed to networks the deployment does not control must use HMAC cookies to prevent stateful spoofing. Repair: add a `stateless_accept:` block with `mode`, `cookie_lifetime_ms`, `key_rotation_s`, `hmac_extern`, `rng_extern` per spec line 2320-2349, or set `untrusted_source: false` if the link is on a controlled network."}"#,
            ),
            (
                "deploy/stateless-accept-key-rotation-shorter-than-lifetime",
                DeployError::StatelessAcceptKeyRotationShorterThanLifetime {
                    machine: "mcu_node".into(),
                    link_name: "udp_listener".into(),
                    key_rotation_s: 30,
                    cookie_lifetime_ms: 30_000,
                    rotation_ms: 30_000,
                    lifetime_doubled: 60_000,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:16e83b54ae5813fa","code":"deploy/stateless-accept-key-rotation-shorter-than-lifetime","stage":"mesh-deploy","spec":"watching-zenoh RFC §5.K","message":"machine 'mcu_node': link 'udp_listener' `stateless_accept.key_rotation_s: 30` × 1000 ≤ 2 × `cookie_lifetime_ms: 30000` (30000 ≤ 60000). watching-zenoh RFC §5.K line 2470-2473 — the previous-key honor window cannot bridge a rotation, so handshakes near rotation boundaries get spurious cookie rejection. Repair: raise `key_rotation_s` to > `2 × cookie_lifetime_ms / 1000`, or lower `cookie_lifetime_ms` to < `key_rotation_s × 500`.","expected":["60000"],"actual":"30000"}"#,
            ),
            (
                "deploy/session-arming-quota-vs-peer-table-invariant-violated",
                DeployError::SessionArmingQuotaVsPeerTableInvariantViolated {
                    machine: "mcu_node".into(),
                    link_name: "udp_listener".into(),
                    session_arming_quota: 8,
                    max_handshake_time_s: 2,
                    peer_table_capacity: 8,
                    product: 16,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:4fea7d1c968b2879","code":"deploy/session-arming-quota-vs-peer-table-invariant-violated","stage":"mesh-deploy","spec":"watching-zenoh RFC §5.K","message":"machine 'mcu_node': link 'udp_listener' `session_arming_quota: 8` × `stateless_accept.max_handshake_time_s: 2` > `stateless_accept.peer_table.capacity: 8` (16 > 8). watching-zenoh RFC §5.K line 2460-2462 — a slow legitimate handshake can be evicted under attack when the attacker churns the quota faster than the per-peer table can absorb. Repair: raise `peer_table.capacity` to ≥ 16, or lower `session_arming_quota` or `max_handshake_time_s` so the product fits the table.","expected":["8"],"actual":"16"}"#,
            ),
            (
                "deploy/stateless-accept-extern-not-whitelisted",
                DeployError::StatelessAcceptExternNotWhitelisted {
                    machine: "mcu_node".into(),
                    link_name: "udp_listener".into(),
                    extern_name: "my_custom_hmac".into(),
                    role: "hmac".into(),
                    candidates: vec![
                        "__sce_intrinsic_cookie_hmac_sha256".to_string(),
                        "__sce_intrinsic_csprng".to_string(),
                    ],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:023288f561f5a592","code":"deploy/stateless-accept-extern-not-whitelisted","stage":"mesh-deploy","spec":"watching-zenoh RFC §5.K","message":"machine 'mcu_node': link 'udp_listener' `stateless_accept.hmac_extern: my_custom_hmac` not present in the §5.I baseline intrinsics whitelist AND not declared in any loaded `target_plugin`. watching-zenoh RFC §5.K line 2466-2469 — `hmac_extern` and `rng_extern` symbols must come from the baseline registry or a target-plugin entry. Repair: spell the symbol exactly as it appears in the baseline registry or add the symbol to a loaded target-plugin file.","actual":"my_custom_hmac","fix":{"kind":"replace_one_of","candidates":["__sce_intrinsic_cookie_hmac_sha256","__sce_intrinsic_csprng"]}}"#,
            ),
            (
                "timer/slot-overflow",
                DeployError::TimerSlotOverflow {
                    machine: "mcu_node".into(),
                    timer_count: 5,
                    wheel_depth: 4,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:f5757abc38261115","code":"timer/slot-overflow","stage":"mesh-deploy","spec":"watching-zenoh RFC §5.D","message":"machine 'mcu_node': declared 5 timers under machines.mcu_node.timers, but scheduler.timer_wheel_depth = 4 slots cannot accommodate them. watching-zenoh RFC §5.D line 910 (`timer/slot-overflow`) — the static timer wheel is sized at compile time. Repair: raise `scheduler.timer_wheel_depth`, remove excess timers, or switch to `scheduler.kind: tokio` / `rt` to inherit host runtime timer scheduling.","expected":["4"],"actual":"5"}"#,
            ),
            (
                // RFC §5.N line 3060: MCU cooperative scheduler slot
                // overrun. NeutralOrDeterministic — three-axis repair
                // (raise per_link_budget, lower tick_period, drop a
                // link).
                "link/concurrent-count-exceeds-scheduler-slots",
                DeployError::LinkConcurrentCountExceedsSchedulerSlots {
                    machine: "mcu_node".into(),
                    link_count: 4,
                    slot_count: 2,
                    tick_period_us: 1000,
                    per_link_budget_us: 500,
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:e23d9c6761cfacc7","code":"link/concurrent-count-exceeds-scheduler-slots","stage":"mesh-deploy","spec":"watching-zenoh RFC §5.N","message":"machine 'mcu_node' (MCU): 4 links declared but the cooperative scheduler accommodates only 2 per-tick slots (`floor(tick_period_us 1000 / per_link_budget_us 500) = 2`). watching-zenoh RFC §5.N line 3060 — more links than the cooperative scheduler can accommodate. Repair: raise `per_link_budget_us`, lower `tick_period_us`, or remove a link declaration from `machines.<m>.links`.","expected":["2"],"actual":"4"}"#,
            ),
            (
                // RFC §5.N line 3061: per-link budget can't fit one
                // tick. NeutralOrDeterministic — two-axis repair.
                "link/per-link-budget-exceeds-tick-period",
                DeployError::LinkPerLinkBudgetExceedsTickPeriod {
                    machine: "mcu_node".into(),
                    per_link_budget_us: 2000,
                    tick_period_us: 1000,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:b3df0736b5b45af5","code":"link/per-link-budget-exceeds-tick-period","stage":"mesh-deploy","spec":"watching-zenoh RFC §5.N","message":"machine 'mcu_node': `scheduler.per_link_budget_us: 2000` exceeds `scheduler.tick_period_us: 1000`. watching-zenoh RFC §5.N line 3061 — a single link's budget cannot exceed the entire cooperative tick. Repair: lower `per_link_budget_us` to ≤ `tick_period_us`, or raise `tick_period_us`.","expected":["1000"],"actual":"2000"}"#,
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
            // ── §6.2.6 generated-source drift (B9, 2026-05-14) ──
            //    Single code covers both axes (source-hash + template-hash).
            //    `axis` field carries `"source"` or `"template"` to
            //    disambiguate the drifted half; `actual` field embeds the
            //    axis label + embedded value so consumers parsing the
            //    wire can identify the drift axis without re-reading the
            //    file.
            (
                "forge/source-hash-mismatch",
                CliError::VerifySourceHashMismatch {
                    path: "out/foo_sm.rs".into(),
                    axis: "source",
                    expected_hex:
                        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                            .into(),
                    actual_hex:
                        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                            .into(),
                },
                r#"{"v":1,"id":"fnv1a:6bbb966dd3008e84","code":"forge/source-hash-mismatch","stage":"cli","spec":"watching-zenoh RFC §6.2.6","message":"out/foo_sm.rs: §6.2.6 source-hash mismatch (embedded=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, recomputed=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa) — regenerate via sce-codegen","actual":"source-hash=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}"#,
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
            | LinkLinkClassUnknown
            | LinkClassUnsupportedOnTarget
            | MemPoolSectionConflict
            | MeshDeployStagePoolNotDeclared
            | MeshDeployStagePoolWrongKind
            | PoolSampleTakeWithoutStagePool
            | ScxmlOnSampleLinkNotDeclared
            | ScxmlOnSampleLinkWrongKind
            | MeshDeployUnsupportedVersion
            | MeshTopologyMachineNotFound
            | MeshTopologySubscriptionSourceUnbound
            | MeshCodegenUnsupportedLanguage
            | MeshCodegenUnsupportedTransport
            | CliUnknownLanguage
            | CliInvalidFormatOption
            // `<sce:extern>` whitelist rejection: 3 of the 4 codes
            // ride `Fix::ReplaceOneOf` (NotInWhitelist closest names,
            // AbiMismatch closed `[c, rust]` set, OrderingUnspecified
            // suffix-bearing completions). SignatureMismatch sits in
            // NeutralOrDeterministic below — it carries the canonical
            // sig as a single `Fix::Replace` value, not a candidate list.
            | ExternSymbolNotInWhitelist
            | ExternAbiMismatch
            | ExternOrderingUnspecified
            // C5 cache-policy on no-dcache core: closed candidate
            // list = `["none"]` (the only legal policy on a core
            // without D-cache). `Fix::ReplaceOneOf` carries the
            // single repair axis; the other 5 C5 codes have
            // multi-axis or codegen-invariant repairs (Fix::None)
            // and sit in NeutralOrDeterministic below.
            | MemCachePolicyUnsupportedOnNoDcacheCore
            // C2-β worker cross-resolution: link-rx ref-unknown rides
            // `Fix::ReplaceOneOf` with sorted alias lists from
            // `parsed.imports` filtered to kind=link. η-precedent:
            // `LinkClassUnsupportedOnTarget` carries closed candidates
            // the same way. (Outbox cross-resolution against statechart
            // / worker docs landed in C2 follow-up Atomic B alongside
            // the SCXML-side `compile_scxml_with_imports` orchestrator
            // that builds the cross-doc registry the validator
            // consumes — see `WorkerOutboxRefUnknown` +
            // `WorkerOutboxTargetWrongKind` below.)
            | WorkerLinkRxRefUnknown
            // ── C2 follow-up Atomic B outbox cross-resolution ──
            //   Two of the three outbox axes carry a closed candidate
            //   list (sorted statechart + worker `.inbox` set);
            //   suffix-invalid is deterministic (`{owner}.inbox` is the
            //   unique repair) and rides `NeutralOrDeterministic`
            //   below.
            | WorkerOutboxRefUnknown
            | WorkerOutboxTargetWrongKind
            // ── C6-β Bounded-collection cross-doc resolution ──
            //   Two of the three β codes carry a closed candidate
            //   list (sorted codec + procedure name union for element-
            //   type-not-a-kind; sorted field-name list for index-by-
            //   field-missing). The third (multi-writer-without-atomics)
            //   has no useful closed set across the C4 baseline's
            //   atomic family and rides `NeutralOrDeterministic` below.
            | CollectionElementTypeNotAKind
            | CollectionIndexByFieldMissing
            // C6-γ1 deploy-time capacity resolution: sorted set of
            // declared limit names under `machines.<machine>.limits:`
            // rides `Fix::ReplaceOneOf`. Mirrors the
            // `BufferPoolSectionConflict` precedent for sorted-
            // declared-name candidate sets.
            | CollectionCapacityUnresolved
            // C7-lowering algorithm-over-BC dispatch (RFC §5.A line 311
            // + §5.L line 2611-2618 + 2642-2647). Two of the six
            // codes carry a closed candidate set:
            //   - `algorithm/call-target-unknown`: sorted alias roster
            //     from the algorithm doc's `<sce:import>` list.
            //   - `algorithm/call-target-method-unknown`: the import's
            //     public-method roster per kind (BC = closed
            //     `{find_by_index, get, get_by_slot, len, capacity}`;
            //     algorithm = the imported algorithm name itself).
            // The other four sit in NeutralOrDeterministic below.
            | AlgorithmCallTargetUnknown
            | AlgorithmCallTargetMethodUnknown
            // C13-α-1 `links:` block cross-doc + driver-unknown
            // (RFC §5.K lines 2421, Q-C13-5 a, Q-C13-8 a). All three
            // ride `Fix::ReplaceOneOf`:
            //   - `deploy/link-driver-unknown`: closed candidate set =
            //     known-driver baseline + forge link-doc names (sorted).
            //   - `deploy/link-not-declared-in-deploy`: closed candidate
            //     set = deploy-side link-name set on the same machine.
            //   - `deploy/link-not-declared-in-forge`: closed candidate
            //     set = forge-side link-name set (across the build).
            // The other 6 C13-α-1 link codes have multi-axis or
            // author-domain repairs and sit in NeutralOrDeterministic
            // below.
            | MeshDeployLinkDriverUnknown
            | MeshDeployLinkNotDeclaredInDeploy
            | MeshDeployLinkNotDeclaredInForge
            // C11-WebSocket follow-up sibling — driver↔class
            // cross-validator. `Fix::ReplaceOneOf` 1-element
            // candidate set = the driver name whose KNOWN_DRIVERS
            // class matches the declared forge `<sce:link-class>`.
            // Single-axis deploy-side repair; the parallel forge-
            // side class swap is named in prose but stays out of
            // the structured Fix per non-overlap invariant.
            | MeshDeployLinkDriverClassMismatch
            // C13-γ deploy/stage-copy-policy-unknown — closed set
            // {warn, error, forbid} (RFC §5.K line 2351 + 2517-2519
            // verbatim, single source of truth at
            // `StageCopyPolicy::ALL`). FixCarriesCandidates over the
            // three values.
            | MeshDeployStageCopyPolicyUnknown
            // C13 deferred-2 deploy/stateless-accept-extern-not-
            // whitelisted (RFC §5.K line 2466-2469). Closed set =
            // sorted union of §5.I baseline intrinsics names + any
            // target-plugin-loaded symbol names; Fix::ReplaceOneOf
            // carries the union so authors get a single canonical
            // candidate list independent of which registry the
            // symbol originated in.
            | MeshDeployStatelessAcceptExternNotWhitelisted
            // ── §5.O Atomic 1 — symbol collision dual-location report.
            //    The two `<file>:<line>` strings ride `Fix::ReplaceOneOf`
            //    as the closed candidate set; the agent / author picks
            //    which site to rename to break the clash. Two-element
            //    closed set is the smallest legal FixCarriesCandidates
            //    case but the choice surface (which of two sites to
            //    rename) is real, not a degenerate dropdown.
            | TraceabilityStateIdCollision => FixCarriesCandidates,

            // ── `expected` carries non-repair metadata ────────
            ExpressionParseMismatch | MeshExternalAmbiguousEventGroup => ExpectedIsMetadata,

            // ── Deterministic fix or no fix; expected=None ────
            XmlParse
            | XmlSchemaValidation
            | XmlFileNotFound
            | XmlWrongRootElement
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
            | ValidationBytesMaxSizeViolation
            | MemPoolTooLarge
            | MemInterPoolPaddingNotEmitted
            // C5 cache-maintenance + author-guard + codegen-invariant
            // (5 codes; the 6th — MemCachePolicyUnsupportedOnNoDcacheCore
            // — sits in FixCarriesCandidates above with `["none"]`):
            //   - alignment / slot-size: multi-axis author repair
            //     (raise alignment OR shrink slot OR adjust deploy
            //     dcache_line_size); the `MemPoolTooLarge` precedent
            //     argues `Fix::None`
            //   - cache-maintenance-misplaced: author removes the
            //     `<sce:extern>`; no closed candidate set
            //   - speculative-prefetch-flag-missing: author sets the
            //     deploy.yaml field per SoC datasheet; the message
            //     prose names both M7+/M3 axes
            //   - cache-pre-arm-invalidate-missing-on-speculative-core:
            //     codegen-invariant violation, no author repair
            | MemCacheLineAlignment
            | MemSlotSizeNotCacheLineMultiple
            | PoolCacheMaintenanceMisplaced
            | PoolSpeculativePrefetchFlagMissing
            | PoolCachePreArmInvalidateMissingOnSpeculativeCore
            | PoolSampleTypestateAttributesDisabled
            | PoolSampleCallbackSignatureNonBorrow
            // Worker kind shared-state encapsulation (RFC §5.D line 911,
            // C2-α): author repair is "remove the offending
            // `<sce:import>` or refactor body XML to inbox-only access".
            // No closed candidate set — the foreign-namespace path is
            // arbitrary, so `fix: None` ⇒ NeutralOrDeterministic.
            | WorkerSharedMutableState
            // C2-β inbox ordering codes: author chooses `acq_rel` vs
            // `relaxed` based on placement; codegen-invariant fires
            // when relaxed coexists with cross-core placement. Both
            // axes are author-judgment (not closed candidate), so
            // `fix: None`.
            | WorkerInboxOrderingUnspecified
            | WorkerInboxOrderingRelaxedAcrossCores
            // C2-γ worker scheduler-capacity forge-side anchor
            // (RFC §5.D line 912). Author repair is either adding the
            // worker to `deploy.machines.<m>.workers` or removing the
            // Worker doc; no closed candidate list.
            | WorkerSchedulerUnsupported
            // C2 follow-up Atomic B outbox suffix-invalid axis: spec
            // §5.D line 895 + line 1998 fix the recipient queue name
            // to `inbox`, so the repair is deterministic
            // (`{owner}.inbox`) and rides `Fix::ReplaceWith`. The other
            // two outbox axes (unknown / wrong-kind) carry a closed
            // candidate set and live in FixCarriesCandidates above.
            | WorkerOutboxTargetSuffixInvalid
            // C6-α Bounded-collection parse-time structure validators
            // (RFC §5.L lines 2559 + 2655). Neither carries a closed
            // candidate set:
            // - `ordering-sorted-requires-index-by`: the repair is to
            //   author an `<sce:index-by>` element naming a field of
            //   the element-type struct, but the field name is author-
            //   domain knowledge → no enumeration possible.
            // - `overflow-policy-oldest-wins-requires-ordering-insertion`:
            //   the repair is deterministic (`<sce:ordering>insertion`)
            //   so it could ride `Fix::ReplaceWith`, but per spec the
            //   author could also keep `ordering=sorted-by` and change
            //   the policy — two equally valid repair paths means no
            //   single canonical candidate → NeutralOrDeterministic.
            | CollectionOrderingSortedRequiresIndexBy
            | CollectionOverflowPolicyOldestWinsRequiresOrderingInsertion
            // C9-α Fragment-reassembly variant parse-time structure
            // validators (RFC §5.M lines 2944-2945). Neither carries a
            // closed candidate set:
            // - `mem/reassembly-pool-variant-missing-max-fragments`: the
            //   repair is to add `<sce:max-fragments-per-message>N</sce:max-fragments-per-message>`
            //   but the concrete N is author-domain knowledge (depends
            //   on the wire framer's per-message maximum and the worst-
            //   case message size); no enumeration possible.
            // - `mem/reassembly-pool-variant-missing-timeout`: similar —
            //   the repair is to add `<sce:reassembly-timeout-ms>N</sce:reassembly-timeout-ms>`
            //   with N derived from link latency budget and acceptable
            //   hold time; author-domain knowledge.
            | MemReassemblyPoolVariantMissingMaxFragments
            | MemReassemblyPoolVariantMissingTimeout
            // C13-α-2 + C9-β Fragment-reassembly cross-doc validators
            // (RFC §5.M lines 2946-2995). Per Q-C13-α2-7 (a) lock: all
            // six ride NeutralOrDeterministic. Every code has multi-axis
            // repair (raise slot_size / lower expected_p99 / change
            // max-fragments-per-message / lower mtu_bytes / change
            // trust_class / raise worker_slot_budget_us); author picks
            // the appropriate axis based on the deployment's hot path.
            // `reassembly/untrusted-link-binding` could conceptually ride
            // `Fix::ReplaceWith` "established_session" (single-element
            // closed set), but the second repair "remove the pool
            // binding entirely" is equally valid per spec line 2973-2975,
            // so the two-path repair shape lives in
            // NeutralOrDeterministic alongside its siblings.
            | MemReassemblySlotSizeBelowDeclaredMtu
            | ReassemblyMaxFragmentsInsufficientForMtu
            | ReassemblyExpectedFragmentationRateHigh
            | ReassemblyUntrustedLinkBinding
            | ReassemblyTrustClassMissingOnFragmentingLink
            | ReassemblyStageCopyWcetExceedsSlotBudget
            // C9-γ codegen self-check (RFC §5.M lines 2976-2981). Pure
            // template-regression guard with no author-domain repair —
            // "report the bug upstream" is the only path forward.
            // NeutralOrDeterministic mirrors the
            // `mem/inter-pool-padding-not-emitted` precedent for
            // codegen-internal invariants where author-side `actual` /
            // `expected` / `fix` carry no useful information.
            | ReassemblyPeerIdNotZidOnEstablishedSession
            // C10-α listener-pair codegen self-check (RFC §5.C lines
            // 849-856). Pure template-regression guard — mirrors
            // `reassembly/peer-id-not-zid-on-established-session`
            // shape. NeutralOrDeterministic (Q-C10-7 a).
            | LinkListenerLinkNotPairedWithEstablishedSibling
            // C10-α reassembly-binding-on-unpaired-listener (RFC §5.M
            // lines 2982-2994). Two valid repair paths (add
            // `Accepting.*` substate vs remove the binding) — no
            // closed candidate set. NeutralOrDeterministic (Q-C10-7 a).
            | MeshDeployReassemblyBindingOnUnpairedListener
            // C10-β multi-link concurrency codes (RFC §5.N lines
            // 3060-3062). All three are multi-axis author-domain
            // repairs with no closed candidate sets per Q-C10-β-8 (a):
            //   `concurrent-count-exceeds-scheduler-slots`: raise
            //   per_link_budget_us / lower tick_period_us / drop a
            //   link.
            //   `per-link-budget-exceeds-tick-period`: lower
            //   per_link_budget_us / raise tick_period_us.
            //   `inbound-event-queue-unsized`: add SCXML
            //   `sce:capacity="N"` per-instance OR
            //   `default_event_queue_capacity` per-machine.
            | LinkConcurrentCountExceedsSchedulerSlots
            | LinkPerLinkBudgetExceedsTickPeriod
            | LinkInboundEventQueueUnsized
            // C6-β multi-writer without atomic imports: the C4 baseline
            // atomic family spans 100+ symbols (load/store/cas/fetch ×
            // 5 widths × multiple orderings) so a `Fix::ReplaceOneOf`
            // candidate list would be neither useful nor compact —
            // author chooses width + ordering + op based on their use
            // case. `fix: None` ⇒ NeutralOrDeterministic.
            | CollectionMultiWriterWithoutAtomics
            // C1 Timer kind diagnostics (RFC §5.D lines 909-910).
            // Both are author-judgment repairs: raise the period
            // above the tick rate, or rebalance the timer count
            // against the wheel depth. No closed candidate set.
            | TimerPeriodBelowTickRate
            | TimerSlotOverflow
            // `<sce:extern>` signature mismatch: deterministic
            // `Fix::Replace` with the canonical sig. The other three
            // codes sit in FixCarriesCandidates above.
            | ExternSignatureMismatch
            // `<sce:extern>` target-plugin baseline-shadowing (Atomic B,
            // spec line 1852): plugin author must rename the conflicting
            // entry; SCE cannot synthesize a candidate name. `fix: None`
            // ⇒ NeutralOrDeterministic non_overlap_class.
            | ExternTargetPluginSymbolConflict
            | AlgorithmLocalShadowsParam
            | AlgorithmLvalueUnsupported
            | AlgorithmReturnMissing
            // C7-lowering algorithm-over-BC dispatch — four of the six
            // codes ride `NeutralOrDeterministic`:
            //   - `algorithm/foreach-source-not-iterable`: multi-axis
            //     repair (rename source OR add BC import); `Fix::None`.
            //   - `algorithm/bc-mutation-forbidden`: repair is either
            //     move the mutation to a non-algorithm host or delete
            //     the call; both repairs sit outside the algorithm
            //     body so no `Fix::Replace` value applies.
            //   - `algorithm/foreach-source-bc-with-bytes-item-type`:
            //     repair is "delete the bytes-pattern <sce:var>"; a
            //     deletion, not a replacement.
            //   - `algorithm/call-arg-count-mismatch`: numeric arity
            //     mismatch; arg expressions are author-domain so no
            //     `Fix::Replace` candidate. Matches the
            //     `ValidationCountMismatch` precedent.
            | AlgorithmForeachSourceNotIterable
            | AlgorithmBcMutationForbidden
            | AlgorithmForeachSourceBcWithBytesItemType
            | AlgorithmCallArgCountMismatch
            | ScxmlTopLevelScriptUnloaded
            | ScxmlOnSampleInvalidParent
            | ScxmlOnSampleLinkDuplicateInState
            | ScxmlOnSampleEventNameConflict
            // ScxmlOnSampleLinkNotDeclared + ScxmlOnSampleLinkWrongKind
            // sit in FixCarriesCandidates above (cross-ref ride
            // `Fix::ReplaceOneOf` with the registry's name list).
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
            | CodegenMcuClassKindOnNonMcuLanguage
            | CodegenGenericKindBackendEmitMissing
            // C3 Atomic B-β + B-γ2c no_std rejections: author repair is
            // "drop --no-std" or "remove the offending construct"
            // (`<script>` / HTTP send / `<data src>` / `<invoke>`).
            // No closed candidate set, so `fix: None` ⇒
            // NeutralOrDeterministic.
            | CodegenNoStdScriptNotSupported
            | CodegenNoStdHttpNotSupported
            | CodegenNoStdFsLoadNotSupported
            | CodegenNoStdInvokeNotSupported
            | AlgorithmConstNotFoldable
            | AlgorithmConstFoldBudgetExceeded
            | AlgorithmConstYieldTypeMismatch
            | CodecVariantArmUnreachable
            | CodecVariantDuplicateDefaultArm
            | CodecVariantDefaultArmMidMismatch
            | CodecVariantArmInnerMidUndeclared
            | CodecPresentIfRefsLaterField
            | CodecRepeatCountRefsLaterField
            | AlgorithmTestVectorUnsupportedKind
            | CodecTlvChainDepthUnspecified
            | CodecDmaAlignmentUnsatisfiable
            | CodecParentFlagMismatch
            | LinkFramerMissing
            | LinkBackpressureUndeclared
            | LinkPoolSlotSmallerThanFramerMax
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
            | MeshDeployStagePoolTransportMismatch
            | MeshDeployScxmlInvokeTargetConflict
            | MeshDeployPartitionDuplicateName
            | MeshDeployPartitionMultiDevice
            | MeshDeployPartitionUnitDuplicate
            | MeshDeployPartitionMachineNotListed
            | MeshDeployPartitionEmpty
            | MeshDeployPartitionNameNotIdentifier
            | MeshDeployPartitionSynthInfixCollision
            | MeshDeployPartitionUncoveredUnit
            | MeshDeployPartitionPartialCoverageRequiresDefault
            | MeshDeployPartitionPoolMachine
            | MeshDeployPartitionTransportBindingUnsupported
            | MeshDeployScxmlInvokeCrossDeviceTransport
            | MeshDeploySomeipScxmlInvokeServiceIdOverflow
            | MeshDeploySomeipScxmlInvokeServiceIdPinOutOfRange
            | MeshDeploySomeipScxmlInvokeServiceIdPinCollision
            | MeshDeploySomeipLivenessServiceIdOverflow
            | MeshDeploySomeipLivenessServiceIdPinOutOfRange
            | MeshDeploySomeipLivenessServiceIdPinCollision
            | MeshDeploySomeipMachineLivenessServiceIdOverflow
            | MeshDeploySomeipMachineLivenessServiceIdPinOutOfRange
            | MeshDeploySomeipMachineLivenessServiceIdPinCollision
            | MeshDeployPartitionBarrierTimeoutInvalid
            | MeshPartitionParallelRootUndesignated
            | MeshPartitionParallelRootAmbiguous
            | MeshPartitionParallelRootNotInMachines
            | MeshPartitionParallelRootNonHost
            | MeshPartitionBarrierTimeoutWithoutRoot
            | MeshPartitionWire21CustomTcpUnimplemented
            | MeshDistributabilityR1SharedWrite
            | MeshDistributabilityR2CrossRegionTransition
            | MeshDeployPlatformClassOsMismatch
            | MeshDeploySchedulerCooperativeMissingStackBudget
            // C2-γ scheduler-capacity deploy-side anchors (RFC §5.K
            // lines 2423 / 2428-9 / 2430-1). Author repair = add the
            // missing field or rebalance worker count; no closed
            // candidate list, so `fix: None` ⇒ NeutralOrDeterministic.
            | MeshDeploySchedulerCooperativeMissingSlotBudget
            | MeshDeploySchedulerCooperativeMissingKeepaliveJitterBudget
            | MeshDeploySchedulerIncompatibleWithWorkerCount
            // C13-α-1 `links:` block parse-level + multi-axis repairs
            // (RFC §5.K lines 2440-2503). 6 of the 9 C13-α-1 codes
            // carry author-domain or multi-axis repairs with no closed
            // candidate set — driver MTU floor is the LOWER bound (not
            // the exact author value), expected_p99 vs mtu has two
            // equally valid repair paths (lower p99 or raise mtu),
            // burst-absorption-insufficient has three structural fixes
            // (raise slot_count / lower tick_period_us / switch
            // dispatch), etc. The other 3 link codes carry closed
            // candidate sets and sit in FixCarriesCandidates above.
            | MeshDeployLinkMtuMissingOnFragmentingLink
            | MeshDeployLinkMtuBelowDriverFloor
            | MeshDeployLinkExpectedP99ExceedsMtu
            | MeshDeployLinkBurstPpsMissingOnIsrDispatch
            // C13-α-2 cross-doc RX-pool burst invariants. Both ride
            // NeutralOrDeterministic — repair is multi-axis per spec
            // (raise slot_count, lower tick_period_us, switch rx_dispatch
            // mode); author chooses the axis fitting the deployment
            // budget. No closed candidate set.
            | MeshDeployLinkBurstAbsorptionInsufficient
            | MeshDeployLinkRxDispatchWorkerTickOnHighBurst
            // C13-γ stage-copy promotion + opt-out rejection. Both
            // ride NeutralOrDeterministic:
            //   - pool/stage-copy-policy-error: multi-axis repair
            //     (raise slot_size / lower expected_p99 / add
            //     accept-stage-copy-rate / change policy to warn).
            //   - pool/stage-copy-accept-rejected-under-forbid: two
            //     valid repair paths (remove the opt-out element vs
            //     change policy to error). Single-element closed-set
            //     repair would collapse to Fix::ReplaceWith but the
            //     two-path repair surface keeps it in
            //     NeutralOrDeterministic.
            | PoolStageCopyPolicyError
            | PoolStageCopyAcceptRejectedUnderForbid
            // C13-β anti-flood + stateless_accept. All five ride
            // NeutralOrDeterministic — author-domain numeric values
            // for the *-missing codes; two-axis repair for the
            // key-rotation-shorter-than-lifetime invariant; remove-
            // or-change-trust-class repair for the
            // session-arming-fields-on-non-arming-link; full-block
            // authoring for the stateless-accept-required code.
            | MeshDeploySessionArmingQuotaMissing
            | MeshDeployAcceptRateConfigMissing
            | MeshDeploySessionArmingFieldsOnNonArmingLink
            | MeshDeployStatelessAcceptRequiredOnUntrustedSource
            | MeshDeployStatelessAcceptKeyRotationShorterThanLifetime
            // C13 deferred-2 peer-table invariant. NeutralOrDeterministic
            // — three-axis repair (raise peer_table.capacity, lower
            // session_arming_quota, or lower max_handshake_time_s).
            // The wire payload's `expected` carries the bound
            // (peer_table.capacity); no closed candidate set exists
            // because the repair axes are independent author choices.
            | MeshDeploySessionArmingQuotaVsPeerTableInvariantViolated
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
            | MeshIo
            // ── §6.2.6 generated-source drift (B9). Repair is the
            //    deterministic `sce-codegen <regen-command>` — no
            //    candidate set across multiple repair paths. ──
            | ForgeSourceHashMismatch
            // ── §5.O Atomic 0 — IR provenance guard. Codegen-internal
            //    invariant: an empty source_location means the parser
            //    site that produced this node failed to attach a
            //    SourceLocation. No author repair — the fix lives in
            //    the parser site, not the document. ──
            | TraceabilityScxmlLineRangeMissing
            // ── §5.O Atomic 1 — three of the four codes ride
            //    NeutralOrDeterministic. Symbol-length: multi-axis
            //    repair (shorten any of three contributing names OR
            //    relax the strict flag). Sourcemap-source-hash drift:
            //    regenerate via `sce-codegen generate`. SCE-MAP-attribute-
            //    stripped: dual-emit fallback covers; the diagnostic is
            //    a heads-up not a hard repair. ──
            | TraceabilitySymbolNameExceedsCIdentifierLimit
            | TraceabilitySourcemapSourceHashMismatch
            | TraceabilitySceMapAttributeStripped
            // ── §5.O Atomic 1 follow-up — ownership-boundary walker.
            //    Codegen-internal invariant: no author repair, the
            //    fix is in the template that lost its SCE-MAP macro
            //    call. NeutralOrDeterministic since the diagnostic is
            //    informational toward upstream pipeline-bug repair. ──
            | TraceabilityMetaGeneratedSourceLineMarkerMissing
            // ── Round F-α (watching-zenoh RFC §5.2) — both MCU
            //    driver/class boundary codes ride NeutralOrDeterministic.
            //    `mcu/driver-header-not-found`: author-domain repair
            //    (fix href / add file / set driver_root) — no closed
            //    candidate set. `mcu/section-attribute-on-non-mcu-target`:
            //    multi-axis repair (remove the section / switch backend
            //    to c11 / split deploys) — also open-ended. Both fall
            //    outside FixCarriesCandidates by design.
            | McuDriverHeaderNotFound
            | McuSectionAttributeOnNonMcuTarget => NeutralOrDeterministic,
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
                | XmlFileNotFound | XmlWrongRootElement
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
                | ValidationBytesMaxSizeViolation
                | AlgorithmLocalShadowsParam
                | AlgorithmLvalueUnsupported
                | AlgorithmReturnMissing
                | AlgorithmForeachSourceNotIterable
                | AlgorithmCallTargetUnknown
                | AlgorithmCallTargetMethodUnknown
                | AlgorithmBcMutationForbidden
                | AlgorithmForeachSourceBcWithBytesItemType
                | AlgorithmCallArgCountMismatch
                | ScxmlTopLevelScriptUnloaded
                | ScxmlOnSampleInvalidParent
                | ScxmlOnSampleLinkDuplicateInState
                | ScxmlOnSampleEventNameConflict
                | ScxmlOnSampleLinkNotDeclared
                | ScxmlOnSampleLinkWrongKind
                | ExpressionEmpty | ExpressionLex
                | ExpressionUnsupportedConstruct | ExpressionStrictEquality
                | ExpressionParseMismatch | ExpressionUnexpectedToken
                | ExpressionInvalidLvalue | ExpressionTypeCoercion
                | ExpressionGoTernaryUnsupported | ImportFileNotFound
                | ImportKindMismatch | ImportNotForge | ImportReadError
                | ManifestCircularDependency | ManifestIo | GenerateInvalidConfig
                | GenerateTemplateLoad | GenerateTemplateRender
                | GenerateUnsupportedFeature
                | CodegenMcuClassKindOnNonMcuLanguage
                | CodegenGenericKindBackendEmitMissing
                | CodegenNoStdScriptNotSupported
                | CodegenNoStdHttpNotSupported
                | CodegenNoStdFsLoadNotSupported
                | CodegenNoStdInvokeNotSupported
                | AlgorithmConstNotFoldable
                | AlgorithmConstFoldBudgetExceeded
                | AlgorithmConstYieldTypeMismatch
                | CodecVariantArmUnreachable
                | CodecVariantDuplicateDefaultArm
                | CodecVariantDefaultArmMidMismatch
                | CodecVariantArmInnerMidUndeclared
                | CodecPresentIfRefsLaterField
                | CodecRepeatCountRefsLaterField
                | AlgorithmTestVectorUnsupportedKind
                | CodecTlvChainDepthUnspecified
                | CodecDmaAlignmentUnsatisfiable
                | CodecParentFlagMismatch
                | LinkFramerMissing
                | LinkLinkClassUnknown
                | LinkBackpressureUndeclared
                | LinkClassUnsupportedOnTarget
                | LinkPoolSlotSmallerThanFramerMax
                | MemPoolSectionConflict
                | MemPoolTooLarge
                | MemInterPoolPaddingNotEmitted
                | MemCacheLineAlignment
                | MemSlotSizeNotCacheLineMultiple
                | MemCachePolicyUnsupportedOnNoDcacheCore
                | PoolCacheMaintenanceMisplaced
                | PoolSpeculativePrefetchFlagMissing
                | PoolCachePreArmInvalidateMissingOnSpeculativeCore
                | PoolSampleTypestateAttributesDisabled
                | PoolSampleTakeWithoutStagePool
                | PoolSampleCallbackSignatureNonBorrow
                | WorkerSharedMutableState
                | WorkerLinkRxRefUnknown
                | WorkerInboxOrderingUnspecified
                | WorkerInboxOrderingRelaxedAcrossCores
                | WorkerSchedulerUnsupported
                | WorkerOutboxRefUnknown
                | WorkerOutboxTargetWrongKind
                | WorkerOutboxTargetSuffixInvalid
                | CollectionOrderingSortedRequiresIndexBy
                | CollectionOverflowPolicyOldestWinsRequiresOrderingInsertion
                | CollectionElementTypeNotAKind
                | CollectionIndexByFieldMissing
                | CollectionMultiWriterWithoutAtomics
                | CollectionCapacityUnresolved
                | MemReassemblyPoolVariantMissingMaxFragments
                | MemReassemblyPoolVariantMissingTimeout
                | MemReassemblySlotSizeBelowDeclaredMtu
                | ReassemblyMaxFragmentsInsufficientForMtu
                | ReassemblyExpectedFragmentationRateHigh
                | ReassemblyUntrustedLinkBinding
                | ReassemblyTrustClassMissingOnFragmentingLink
                | ReassemblyStageCopyWcetExceedsSlotBudget
                | ReassemblyPeerIdNotZidOnEstablishedSession
                | LinkListenerLinkNotPairedWithEstablishedSibling
                | MeshDeployReassemblyBindingOnUnpairedListener
                | LinkConcurrentCountExceedsSchedulerSlots
                | LinkPerLinkBudgetExceedsTickPeriod
                | LinkInboundEventQueueUnsized
                | TimerPeriodBelowTickRate
                | TimerSlotOverflow
                | ExternSymbolNotInWhitelist
                | ExternAbiMismatch
                | ExternSignatureMismatch
                | ExternOrderingUnspecified
                | ExternTargetPluginSymbolConflict
                | IoFilesystem
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
                | MeshDeployStagePoolNotDeclared
                | MeshDeployStagePoolWrongKind
                | MeshDeployStagePoolTransportMismatch
                | MeshDeployScxmlInvokeTargetConflict
                | MeshDeployPartitionDuplicateName
                | MeshDeployPartitionMultiDevice
                | MeshDeployPartitionUnitDuplicate
                | MeshDeployPartitionMachineNotListed
                | MeshDeployPartitionEmpty
                | MeshDeployPartitionNameNotIdentifier
                | MeshDeployPartitionSynthInfixCollision
                | MeshDeployPartitionUncoveredUnit
                | MeshDeployPartitionPartialCoverageRequiresDefault
                | MeshDeployPartitionPoolMachine
                | MeshDeployPartitionTransportBindingUnsupported
                | MeshDeployScxmlInvokeCrossDeviceTransport
                | MeshDeploySomeipScxmlInvokeServiceIdOverflow
                | MeshDeploySomeipScxmlInvokeServiceIdPinOutOfRange
                | MeshDeploySomeipScxmlInvokeServiceIdPinCollision
                | MeshDeploySomeipLivenessServiceIdOverflow
                | MeshDeploySomeipLivenessServiceIdPinOutOfRange
                | MeshDeploySomeipLivenessServiceIdPinCollision
                | MeshDeploySomeipMachineLivenessServiceIdOverflow
                | MeshDeploySomeipMachineLivenessServiceIdPinOutOfRange
                | MeshDeploySomeipMachineLivenessServiceIdPinCollision
                | MeshDeployPartitionBarrierTimeoutInvalid
                | MeshPartitionParallelRootUndesignated
                | MeshPartitionParallelRootAmbiguous
                | MeshPartitionParallelRootNotInMachines
                | MeshPartitionParallelRootNonHost
                | MeshPartitionBarrierTimeoutWithoutRoot
                | MeshPartitionWire21CustomTcpUnimplemented
                | MeshDistributabilityR1SharedWrite
                | MeshDistributabilityR2CrossRegionTransition
                | MeshDeployPlatformClassOsMismatch
                | MeshDeploySchedulerCooperativeMissingStackBudget
                | MeshDeploySchedulerCooperativeMissingSlotBudget
                | MeshDeploySchedulerCooperativeMissingKeepaliveJitterBudget
                | MeshDeploySchedulerIncompatibleWithWorkerCount
                | MeshDeployLinkDriverUnknown
                | MeshDeployLinkMtuMissingOnFragmentingLink
                | MeshDeployLinkMtuBelowDriverFloor
                | MeshDeployLinkExpectedP99ExceedsMtu
                | MeshDeployLinkBurstPpsMissingOnIsrDispatch
                | MeshDeployLinkNotDeclaredInDeploy
                | MeshDeployLinkNotDeclaredInForge
                | MeshDeployLinkBurstAbsorptionInsufficient
                | MeshDeployLinkRxDispatchWorkerTickOnHighBurst
                | MeshDeployLinkDriverClassMismatch
                | PoolStageCopyPolicyError
                | PoolStageCopyAcceptRejectedUnderForbid
                | MeshDeployStageCopyPolicyUnknown
                | MeshDeploySessionArmingQuotaMissing
                | MeshDeployAcceptRateConfigMissing
                | MeshDeploySessionArmingFieldsOnNonArmingLink
                | MeshDeployStatelessAcceptRequiredOnUntrustedSource
                | MeshDeployStatelessAcceptKeyRotationShorterThanLifetime
                | MeshDeploySessionArmingQuotaVsPeerTableInvariantViolated
                | MeshDeployStatelessAcceptExternNotWhitelisted
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
                | MeshIo
                // B9 §6.2.6 generated-source drift detection
                | ForgeSourceHashMismatch
                // §5.O Atomic 0 IR provenance pre-emit guard
                | TraceabilityScxmlLineRangeMissing
                // §5.O Atomic 1 — symbol mangling + sourcemap contract
                | TraceabilityStateIdCollision
                | TraceabilitySymbolNameExceedsCIdentifierLimit
                | TraceabilitySourcemapSourceHashMismatch
                | TraceabilitySceMapAttributeStripped
                // §5.O Atomic 1 follow-up — boundary walker
                | TraceabilityMetaGeneratedSourceLineMarkerMissing
                // Round F-α (watching-zenoh RFC §5.2) — MCU driver/class
                // boundary codes; both ride NeutralOrDeterministic per
                // the non_overlap_class match above.
                | McuDriverHeaderNotFound
                | McuSectionAttributeOnNonMcuTarget => true,
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
            277,
            "ALL_DIAGNOSTIC_CODES has duplicates or missing entries —\
             expected 263 distinct variants to match the DiagnosticCode \
             enum (watching-zenoh RFC §5.B B3 added the MCU-class TLV \
             chain v1 gate: CodecTlvChainDepthUnspecified; 168 → 169, \
             then DMA alignment v1 gate: CodecDmaAlignmentUnsatisfiable; \
             169 → 170, then B5-γ parent-flags dependency: \
             CodecParentFlagMismatch; 170 → 171; then watching-zenoh \
             RFC §5.C B6-α first link kind diagnostic LinkFramerMissing; \
             171 → 172; then B6-γ parse-time pair LinkLinkClassUnknown \
             + LinkBackpressureUndeclared; 172 → 174; then B6-η \
             OS-axis validate-time LinkClassUnsupportedOnTarget; \
             174 → 175; then watching-zenoh RFC §5.E B7-α first \
             buffer-pool kind diagnostic MemPoolSectionConflict \
             — η-second-consumer pattern on `compile_forge_with_deploy` \
             section validation; 175 → 176; then B7-β c11 parity \
             pair: MemPoolTooLarge — η-third-consumer extension that \
             checks `slot_count × slot_size` fits the resolved region's \
             `size` after section validation; and \
             MemInterPoolPaddingNotEmitted — codegen self-check for \
             the §5.E lines 1059-1064 inter-pool `. = ALIGN(N);` \
             sentinel artifact; 176 → 178; then watching-zenoh RFC \
             §5.C B6-α' link↔pool cross-resolution \
             LinkPoolSlotSmallerThanFramerMax — fourth consumer of the \
             `compile_forge_with_imports` enrichment infra (after the \
             three codec-side codec_max_bytes / requires_parent_flags / \
             first_flags consumers), pairing the B6-side \
             `<sce:rx-pool>` / `<sce:tx-pool>` schema (B7-α) with the \
             B7-side slot-size against the framer codec's recursive \
             worst-case bytes; 178 → 179; then watching-zenoh RFC §5.E \
             B7-ε Layer 1 ownership pull-through codegen self-check \
             PoolSampleTypestateAttributesDisabled — buffer-pool C11 \
             header must `#include <sce/sample.h>` so the runtime \
             header's `SCE_CONSUMABLE` / `SCE_CALLABLE_WHEN` / \
             `SCE_SET_TYPESTATE` / `SCE_PARAM_TYPESTATE` / \
             `SCE_WARN_UNUSED` family reaches downstream consumer builds; \
             β `mem/inter-pool-padding-not-emitted` codegen-invariant \
             precedent — diagnostic exists so a future template edit \
             that drops the include surfaces; 179 → 180; then watching-zenoh \
             RFC §5.E B7-η' deploy.yaml `binding.stage_pool:` cross-reference \
             into the forge pool registry — three companion diagnostics \
             (`mesh/deploy-stage-pool-not-declared`, \
             `mesh/deploy-stage-pool-wrong-kind`, \
             `mesh/deploy-stage-pool-transport-mismatch`) opening the \
             cross-schema reference resolution surface for `Sample::take()` \
             stage destinations. The first two ride `Fix::ReplaceOneOf` over \
             the `ForgePoolRegistry` declared-name candidate set; the third \
             rides `Fix::RemoveFields` to drop the misapplied field on a \
             non-staging-capable transport; 180 → 183; then watching-zenoh \
             RFC §5.E B7-η' Q-OnSample atomic A — the SCXML `<sce:on-sample>` \
             extension's three structural diagnostics \
             (`scxml/on-sample-invalid-parent`, \
             `scxml/on-sample-link-duplicate-in-state`, \
             `scxml/on-sample-event-name-conflict`) gating placement, \
             per-state link uniqueness, and W3C internal-event-prefix \
             collision; 183 → 186; then watching-zenoh RFC §5.E B7-η' \
             Q-OnSample atomic B — `<sce:on-sample link=\"X\">` cross-reference \
             into the forge link registry pair \
             (`scxml/on-sample-link-not-declared`, \
             `scxml/on-sample-link-wrong-kind`) closing the η' codegen \
             prereq chain. Both ride `Fix::ReplaceOneOf` over the \
             `SceCrossDocRegistry`-declared link-name candidate set; \
             `wrong-kind` is wired forward-compat per the stage_pool \
             `wrong-kind` precedent — the single-variant `ScxmlDocKind` \
             registry today only stores Link kinds, so the validator's \
             match never reaches the `Some(non-Link)` arm in \
             production until a future cross-registry generalization; \
             186 → 188); then watching-zenoh RFC §5.E B7-η' Atomic A1 \
             schema-locality fix — `pool/sample-take-without-stage-pool` \
             surfaces the gap when SCXML `<sce:on-sample link=\"X\">` \
             targets a registered link kind whose forge document does \
             not declare a `<sce:stage-pool>` (rx_pool/tx_pool sibling \
             pattern, single source of truth on the link kind). Diagnostic \
             rides `Fix::ReplaceOneOf` over the `ForgePoolRegistry` \
             buffer-pool kind candidates so authors picking a stage pool \
             reference see legal options at hand; 188 → 189; then watching-zenoh \
             RFC §5.E B7-η' Atomic A2 callback-path syntax — \
             `pool/sample-callback-signature-non-borrow` surfaces the gap \
             when `<sce:on-sample callback=\"rust:...\">` carries an \
             authoring path that fails the Q-Callback-3 Rust path subset \
             (unknown language prefix, leading/trailing `::`, malformed \
             segment, empty path). Diagnostic name preserves spec wording \
             verbatim per `feedback_spec_mirror_parity.md`; the per-instance \
             reason field disambiguates the specific path-syntax mistake. \
             Forward-compat for future signature inspection (β-extension on \
             top of α) extending the same code with shape-mismatch arms; \
             189 → 190; then watching-zenoh RFC §5.I `<sce:extern>` \
             whitelisted intrinsic registry Atomic A — four spec-verbatim \
             rejection codes (`extern/symbol-not-in-whitelist`, \
             `extern/abi-mismatch`, `extern/signature-mismatch`, \
             `extern/ordering-unspecified`) firing at parse-time on \
             `<sce:extern>` declarations against the 101-symbol baseline \
             registry (atomics × per-width × per-ordering + fences + cache \
             maintenance + IRQ control); 190 → 194. Atomic B then adds \
             `extern/target-plugin-symbol-conflict` (spec line 1852 \
             verbatim: 'target plugin redefines a core whitelist symbol'), \
             firing at plugin-load time when a target_plugin YAML \
             (`extern_symbols.target_plugin: <path>`) tries to redeclare \
             a baseline symbol. Q-Call-6 (a) lock: plugin entries are \
             additive — same name = conflict, regardless of sig parity, \
             because SCE-shipped baseline is canonical and platform-specific \
             impls plug in via the entry's `crate` field on a \
             differently-named symbol; 194 → 195. The remaining §5.C codes \
             defer to B6-δ (listener self-check, gated on §5.K + §5.M \
             SCE-side prerequisites), D.2 (`link/link-class-incompatible-with-os` \
             alongside OS-specific classes), and B7 (`mem/alignment-violation` \
             deferred until codec field placement lands a consumer surface, \
             B7-γ FSM family). The §5.I plugin-extension axes superseded \
             by Atomic B's spec-verbatim reduction \
             (`extern/target-plugin-not-loaded` is subsumed by Atomic A's \
             `extern/symbol-not-in-whitelist` since a non-loaded plugin \
             leaves missing symbols outside the registry; \
             `extern/target-plugin-shadows-baseline` is the same semantic \
             as the spec-verbatim `extern/target-plugin-symbol-conflict` \
             above). Plugin-axis codes still deferred to Atomic C of \
             `rfc-sce-call-intrinsic-registry.md`: \
             `extern/linker-flavor-unsupported`, \
             `extern/linker-flavor-os-managed-without-cmake-import`, \
             `extern/ordering-insufficient-for-cross-core`. Then \
             watching-zenoh RFC §5.E C5 cache-maintenance intrinsics \
             wired into §5.E codegen — six spec-named codes from \
             RFC §5.E lines 1543-1545 + 1548 + 1552-1553 covering the \
             cache-policy=maintain enforcement surface: \
             `mem/cache-line-alignment` (pool alignment < platform \
             dcache_line_size), \
             `mem/slot-size-not-cache-line-multiple` (slot_size % \
             dcache_line_size != 0), \
             `mem/cache-policy-unsupported-on-no-dcache-core` \
             (maintain/non-cacheable on platform.has_dcache=false; \
             FixCarriesCandidates with `[\"none\"]`), \
             `pool/cache-maintenance-misplaced` (parse-time author \
             guard: <sce:extern> for cache trio rejected per spec \
             lines 1222-1227 author-must-not), \
             `pool/speculative-prefetch-flag-missing` (config-\
             completeness: has_dcache=true requires has_speculative_\
             prefetch when at least one cache-policy=maintain pool \
             exists), \
             `pool/cache-pre-arm-invalidate-missing-on-speculative-\
             core` (codegen-invariant guard against template \
             regression that would silently drop the pre-arm \
             cache-invalidate edge on M7+ cores). Auto-injects 3 \
             cache extern declarations at parse time (atomic C \
             sidecar emit picks them up automatically); 195 → 201. \
             Then watching-zenoh RFC §5.D Worker kind C2-α — one \
             spec-named code `worker/shared-mutable-state` from RFC \
             §5.D line 911 covering the worker encapsulation surface: \
             layer 1 rejects `<sce:import kind=\"worker\">` siblings \
             inside a worker document (workers cannot import other \
             worker kinds); layer 2 rejects body SCXML data-refs whose \
             namespace prefix names a foreign owner (not in the inbox-\
             only allowlist of `[<self-name>, _event, _data, _name, \
             _iolocation, <outbox-target>]`). Layer 3 (`<sce:extern>` \
             non-inbox symbol use in worker body) defers to a tracked \
             follow-up atomic gated on C4 intrinsic-registry composition \
             surface per Q-C2-7 (a)+(b) lock; spec line 911 phrasing \
             \"any non-inbox access to another worker's state\" covers \
             all three layers together; 201 → 202. Then watching-zenoh \
             RFC §5.D + §5.I C2-β Worker codegen + inbox ordering — \
             three codes (count narrowed from the original 4 after Gate \
             B preflight surfaced `parse_imports` rejects \
             `kind=\"statechart\"` imports as a long-standing forge \
             invariant): `worker/link-rx-ref-unknown` for `<sce:link-rx \
             ref>` not matching a `kind=\"link\"` import alias follows \
             the η-precedent `validate_link_pool_framer_resolution` \
             shape, validating against `parsed.imports` within \
             `compile_forge_with_imports` and riding `Fix::ReplaceOneOf` \
             over the sorted alias candidate set; two spec-verbatim \
             SPSC inbox ordering codes from RFC §5.I lines 1752-1758 \
             (`worker/inbox-ordering-unspecified` parse-time error when \
             `<sce:inbox>` lacks the required `ordering` attribute — \
             SCE's error-only wire realizes the spec \"warning\" as \
             required-when-worker-exists, and \
             `worker/inbox-ordering-relaxed-across-cores` codegen-invariant \
             when explicit `ordering=\"relaxed\"` coexists with deploy \
             placement pinning producer and consumer on different cores). \
             Outbox cross-resolution (`worker/outbox-ref-unknown`) defers \
             to a follow-up atomic that places the validator on the \
             SCXML-side build tier (where statechart docs are first-class) \
             — `parse_imports`'s long-standing statechart-rejection \
             prevents the η-precedent's parsed.imports-direct shape from \
             applying to outbox refs without a deeper architectural \
             change; 202 → 205. Then watching-zenoh RFC §5.D + §5.K \
             C2-γ scheduler-capacity axis adds four spec-named codes: \
             `worker/scheduler-unsupported` (§5.D line 912 — forge-side \
             anchor for compile_forge_with_deploy when a Worker doc is \
             not declared in `machines.<m>.workers`), \
             `deploy/worker-slot-budget-missing` (§5.K line 2428-2429 — \
             cooperative scheduler missing `worker_slot_budget_us`), \
             `deploy/keepalive-jitter-budget-missing` (§5.K line 2430-2431 \
             — cooperative scheduler missing `keepalive_jitter_budget_us`), \
             and `deploy/scheduler-incompatible-with-worker-count` (§5.K \
             line 2423 — `workers.len() > floor(tick_period_us / \
             worker_slot_budget_us)`). The C2-γ landing also renames the \
             pre-existing wire `mesh/deploy-scheduler-cooperative-missing-stack-budget` \
             to spec-verbatim `deploy/worker-stack-budget-missing` (§5.K \
             line 2426). Rename is wire-only; variant ident retained. \
             205 → 209. Then watching-zenoh RFC §5.D C1 Timer kind \
             migration adds two spec-named codes: \
             `timer/period-below-tick-rate` (line 909, period < \
             scheduler.tick_period_us on cooperative) and \
             `timer/slot-overflow` (line 910, total Timer doc count \
             for a machine exceeds scheduler.timer_wheel_depth). \
             C1 also migrates the legacy multi-timer-per-doc \
             `<datamodel>/<data sce:timer=\"periodic|timeout|delayed\">` \
             shape to the spec-mandated single-timer-per-doc shape \
             with body-text `<sce:period>` (unit suffix us/ms/s/m) + \
             event-driven `<sce:reset-on>` / state-exit-driven \
             `<sce:cancel-on>` / required `<sce:fire-event>` \
             lifecycle. 209 → 211. Then watching-zenoh RFC §5.D C2 \
             follow-up Atomic B adds the SCXML-side `<sce:outbox ref>` \
             cross-resolution surface (the outbox piece C2-β deferred \
             pending the cross-doc registry foundation Atomic A landed): \
             three non-spec codes per Q-Outbox-8 (c) lock 2026-05-12 \
             splitting the failure axes by repair surface — \
             `worker/outbox-ref-unknown` for an owner not in the \
             `SceCrossDocRegistry` (statechart + worker union per \
             Q-Outbox-3 (b)), `worker/outbox-target-wrong-kind` for an \
             owner registered as an incompatible kind (today only \
             link — buffer-pool / algorithm / codec / timer / extern \
             never enter the registry), and `worker/outbox-target-\
             suffix-invalid` for any suffix !=  `inbox` per Q-Outbox-6 \
             (a) strict-suffix lock (RFC §5.D line 895 example + line \
             1998 codegen table jointly fix the recipient queue name). \
             The first two ride `Fix::ReplaceOneOf` over the sorted \
             union of statechart + worker `.inbox` candidates; the \
             third rides `Fix::ReplaceWith` (deterministic single \
             repair `{{owner}}.inbox`) and sits in `NeutralOrDeterministic`. \
             Validator `validate_worker_outbox_references` consumes the \
             registry from `compile_scxml_with_imports` (Atomic A \
             foundation) and runs after statechart-name registration \
             so worker→statechart and worker→worker outboxes resolve \
             symmetrically. 211 → 214; then watching-zenoh RFC §5.J.2 \
             C3 Atomic B-β Rust no_std variant rejection pair: \
             `codegen/no-std-script-not-supported` fires when an SCXML \
             document that contains `<script>` is generated with \
             `sce-codegen generate -l rust --no-std` (the `sce-rust-runtime` \
             `no_std` Cargo feature is mutually exclusive with the \
             `script-engine-lua` / `script-engine-quickjs` features per \
             spec line 1989 zero-alloc mandate), and \
             `codegen/no-std-http-not-supported` fires when the same \
             document carries a `<send type=\"BasicHTTPEventProcessor\">` \
             or `<send target=\"http://...\">` (the runtime crate's \
             `http-send` feature is std-coupled to tokio/reqwest, so \
             `no_std + http-send` is rejected at the cfg-assert layer in \
             the runtime crate's `lib.rs`). Both ride \
             `NeutralOrDeterministic` non_overlap_class — author repair \
             is to drop `--no-std` or to remove the incompatible \
             construct, no closed candidate set. 214 → 216. Then \
             watching-zenoh RFC §5.J.2 C3 Atomic B-γ2c closes the helper \
             runtime cfg-gate with a second no_std rejection pair: \
             `codegen/no-std-fs-load-not-supported` fires when the document \
             contains `<data src=\"...\">` (the filesystem helpers in \
             `sce-rust-runtime/src/helpers/datamodel_init.rs` need `PathBuf` \
             plus `std::fs::read_to_string`, both alloc/OS-coupled per spec \
             line 1989-1994), and `codegen/no-std-invoke-not-supported` \
             fires when the document contains `<invoke>` (the invoke \
             machinery uses `Arc<Mutex<Vec<...>>>` plus `HashMap` and is \
             whole-module gated to `!no_std`). Both ride \
             `NeutralOrDeterministic` non_overlap_class for the same \
             reason as the B-β pair — drop `--no-std` or remove the \
             construct, no closed candidate set. 216 → 218. Then \
             watching-zenoh RFC §5.L C6 Atomic α bounded-collection kind \
             — two parse-time structure validators from spec lines 2559 \
             + 2655: `collection/ordering-sorted-requires-index-by` \
             (sorted-by ordering declared without an accompanying \
             `<sce:index-by>` element — codegen has no comparator) and \
             `collection/overflow-policy-oldest-wins-requires-ordering-\
             insertion` (oldest-wins overflow paired with sorted-by \
             ordering — \"oldest\" has no temporal meaning when iteration \
             order is comparator-derived). Both ride \
             `NeutralOrDeterministic` non_overlap_class (sorted: author \
             must name a field from author-domain knowledge; \
             oldest-wins: two equally valid repair paths means no single \
             canonical candidate). C6-α schema lock ships \
             `<sce:element-type>` / `<sce:capacity source=deploy|const>` / \
             `<sce:index-by field>` / `<sce:on-overflow>` / `<sce:ordering>` \
             / `<sce:concurrency>` body with cross-doc resolution + \
             deploy-time + codegen surfaces deferred to β/γ; 218 → 220. \
             Then watching-zenoh RFC §5.L C6 Atomic β bounded-collection \
             cross-doc resolution — three codes from spec lines 2566-2567 \
             + 2615 + 2560-2562 closing the cross-doc layer C6-α \
             deferred: `collection/element-type-not-a-kind` for \
             `<sce:element-type>NAME` body text that does not resolve to \
             a codec-kind struct or procedure-kind state record \
             anywhere in the build, and `collection/index-by-field-\
             missing` for `<sce:index-by field=\"X\"/>` naming a field \
             absent from the resolved element-type's struct (codec \
             `.fields[].id` or procedure `.inputs[].id + .internals[].id` \
             enumeration mirroring `discover_stateful_member_fields`). \
             Both ride `Fix::ReplaceOneOf` over the sorted candidate \
             union — codec + procedure name set for element-type, \
             declared field-name set for index-by — so \
             FixCarriesCandidates non_overlap_class follows the C2-\
             outbox B `worker/outbox-ref-unknown` precedent for sorted-\
             closed-candidate diagnostics. Third code \
             `collection/multi-writer-without-atomics` (RFC §5.L lines \
             2560-2562 — multi-writer codegen lowers to acquire/release \
             atomics on head/tail; the build's `<sce:extern>` trust-\
             surface must acknowledge atomic intrinsics) is build-wide \
             cross-doc per user direction: pass-1 of \
             `compile_scxml_with_imports` aggregates every parsed forge \
             doc's `extern_declarations` into a single slice; the \
             validator scans for any entry whose registry-resolved \
             purpose starts with `\"atomic-\"`. NeutralOrDeterministic \
             non_overlap_class — the C4 baseline atomic family spans \
             100+ symbols (load/store/cas/fetch × 5 widths × multiple \
             orderings) so a useful candidate list is impossible; author \
             judgment chooses width + ordering + op. Cross-doc validator \
             consumes a separate forge-doc map per Gate B finding (the \
             `SceCrossDocRegistry` reserves SCXML-cross-reference \
             semantics for Link / Statechart / Worker kinds, while \
             codec / procedure participate only in forge→forge cross-\
             references); 220 → 223. Then watching-zenoh RFC §5.L \
             C6 Atomic γ1 bounded-collection deploy-time capacity \
             resolution — one spec-named code from spec lines 2583-2585 \
             + 2649 closing the codegen-time prereq C6-α deferred: \
             `collection/capacity-unresolved` fires on the \
             `compile_forge_with_deploy` path when `<sce:capacity \
             source=\"deploy\" key=\"machines.<m>.limits.<k>\"/>` \
             names a limit that is not declared under \
             `machines.<m>.limits:` in deploy.yaml. New `MachineConfig.\
             limits: HashMap<String, u32>` field (mirrors the \
             `workers`/`timers` per-machine registry precedent — \
             keyed by limit name, value is the codegen-lowered slot \
             count). Validator silent-skips when deploy or \
             target_machine is None (single-file compile paths) or \
             when the key's machine segment does not equal \
             target_machine (BC doc designed for a different machine) \
             per the Q-η5 (a) precedent. Closed candidate set (sorted \
             declared limit names) rides `Fix::ReplaceOneOf` ⇒ \
             FixCarriesCandidates non_overlap_class — mirrors the \
             `BufferPoolSectionConflict` precedent for sorted-\
             declared-name candidate sets. γ1 deliberately defers \
             the Handle bit-allocation contract (slot index + \
             generation counter per spec lines 2621-2622) to γ2 \
             alongside the first-backend (Rust) template emit per \
             `[[feedback-silently-broken-hooks]]` — the Handle is \
             purely codegen-time and has no in-atomic consumer at the \
             foundation tier; 223 → 224. Then watching-zenoh RFC §5.A \
             line 311 + §5.L lines 2611-2618 + 2642-2647 C7-lowering \
             algorithm-over-BC dispatch — six spec-named codes wiring \
             the `<sce:foreach in=\"<bc>\">` + `<sce:call \
             target=\"alias.method\">` lowering surface so an algorithm \
             body iterates a bounded-collection import and dispatches \
             into its read-only method set. Two ride \
             `Fix::ReplaceOneOf` ⇒ FixCarriesCandidates: \
             `algorithm/call-target-unknown` (sorted alias roster from \
             the algorithm doc's `<sce:import>` list) and \
             `algorithm/call-target-method-unknown` (per-kind public-\
             method roster — BC closed `{{find_by_index, get, \
             get_by_slot, len, capacity}}`, algorithm singleton = the \
             imported algorithm name itself). Four ride \
             `NeutralOrDeterministic`: `algorithm/foreach-source-not-\
             iterable` (multi-axis: rename source OR add BC import), \
             `algorithm/bc-mutation-forbidden` (repair sits outside the \
             algorithm body — algorithms are pure per RFC §5.A line \
             333), `algorithm/foreach-source-bc-with-bytes-item-type` \
             (deletion-style repair), and `algorithm/call-arg-count-\
             mismatch` (numeric arity mismatch — arg expressions are \
             author-domain, mirrors `ValidationCountMismatch` precedent). \
             224 → 230. Then watching-zenoh RFC §5.M lines 2944-2945 \
             land the C9-α fragment-reassembly variant parse-time \
             structure validators — \
             `mem/reassembly-pool-variant-missing-max-fragments` + \
             `mem/reassembly-pool-variant-missing-timeout` fire when \
             `<sce:variant>reassembly</sce:variant>` is declared without \
             one of its three required sibling elements (spec line 2688 \
             max-fragments-per-message + spec line 2689 reassembly-\
             timeout-ms). Both ride `NeutralOrDeterministic` — repair \
             requires authoring a concrete u32 value derived from \
             author-domain knowledge (wire framer's per-message maximum, \
             link latency budget) with no closed candidate set. The \
             third reassembly-only element `<sce:per-peer-quota>` reuses \
             the generic `ValidationError::MissingElement` rather than \
             minting a third reassembly-specific code per spec — line \
             2944-2945 names only these two. Cross-doc / cross-link \
             validators that reference §5.K `links.<name>.{{mtu_bytes, \
             expected_p99_bytes, domain_attrs.trust_class}}` (6-8 codes) \
             defer to C9-β co-landing with C13 §5.K. Codegen-side \
             per-slot bitmap/deadline/peer-id emission + 1 codegen-\
             template-regression guard defer to C9-γ. Listener-link \
             sibling-split codes (2) belong to C10/C11 per spec line \
             2820-2824. 230 → 232. Then watching-zenoh RFC §5.K lines \
             2232-2540 lands the C13-α-1 `links:` block schema with 9 \
             new spec-named codes: 7 deploy-side parse-time + cross-doc \
             validators (`deploy/link-driver-unknown` at spec line \
             2421, `deploy/link-mtu-missing-on-fragmenting-link` at \
             2440-2442, `deploy/link-mtu-below-driver-floor` at \
             2443-2445, `deploy/link-expected-p99-exceeds-mtu` at \
             2446-2448, `deploy/link-burst-absorption-insufficient` \
             at 2489-2495, `deploy/link-rx-dispatch-worker-tick-on-\
             high-burst` at 2496-2500, `deploy/link-burst-pps-missing-\
             on-isr-dispatch` at 2501-2503) plus 2 cross-doc validators \
             (Q-C13-5 a lock; `deploy/link-not-declared-in-deploy` + \
             `deploy/link-not-declared-in-forge` pair forge link doc \
             names against deploy.yaml `machines.<n>.links.<name>` \
             entries). Three ride `FixCarriesCandidates` (driver-\
             unknown closed candidate = known-driver baseline + forge \
             link-doc names; both not-declared codes ride opposite-side \
             link-name set per Q-C13-5 a). Four ride `NeutralOrDeterministic` \
             (multi-axis or author-domain repairs: mtu-missing requires \
             author concrete value, p99-exceeds-mtu has two-path repair, \
             burst-pps-missing-on-isr-dispatch carries 2 structural \
             fixes). 2 spec codes (`deploy/link-burst-absorption-\
             insufficient` at 2489-2495 + `deploy/link-rx-dispatch-\
             worker-tick-on-high-burst` at 2496-2500) defer to C13-α-2 \
             per [[feedback-silently-broken-hooks]] — both require RX \
             pool slot_count to be cross-doc-resolved against the \
             forge `<sce:link>` document's `<sce:rx-pool ref=\"X\">` \
             and the `ForgePoolRegistry` entry for `X`, infrastructure \
             that lands in C13-α-2. 6 C9-β reassembly cross-doc codes \
             (mem/reassembly-slot-size-below-declared-mtu + 5 \
             reassembly/*) also defer to C13-α-2. 232 → 239. Then \
             watching-zenoh RFC §5.M lines 2946-2995 + §5.K lines \
             2489-2500 land the C13-α-2 cross-doc validators that \
             consume `compile_scxml_with_imports`'s newly captured \
             forge LinkModel + BufferPoolModel maps. 2 deploy-side codes \
             (`deploy/link-burst-absorption-insufficient` + \
             `deploy/link-rx-dispatch-worker-tick-on-high-burst`) ride \
             the shared cross-doc resolver `resolve_link_rx_pool_slot_count` \
             that joins `deploy.links.<X>` → forge `<sce:link name=X>` → \
             `<sce:rx-pool ref=Y>` → ForgePoolRegistry's BufferPoolModel \
             for Y; both fire when the cooperative tick window cannot \
             drain the declared burst. 6 forge-side reassembly cross-doc \
             codes (`mem/reassembly-slot-size-below-declared-mtu` + \
             `reassembly/max-fragments-insufficient-for-mtu` + \
             `reassembly/expected-fragmentation-rate-high` + \
             `reassembly/untrusted-link-binding` + \
             `reassembly/trust-class-missing-on-fragmenting-link` + \
             `reassembly/stage-copy-wcet-exceeds-slot-budget`) consume \
             the same resolver to check `BufferPoolVariant::Reassembly` \
             bindings against link `mtu_bytes` / `expected_p99_bytes` / \
             `domain_attrs.trust_class` / PlatformConfig WCET fields \
             (C13-α-1 schema landed). All 8 codes ride \
             NeutralOrDeterministic (multi-axis repair; per Q-C13-α2-7 a). \
             239 → 247. Then watching-zenoh RFC §5.K lines 2350-2369 + \
             2504-2519 land the C13-γ `pool_defaults.stage_copy_policy` \
             promotion family — 3 spec-named codes: \
             `pool/stage-copy-policy-error` (NeutralOrDeterministic — \
             warning under `warn` promoted to hard error under `error` \
             / `forbid`, multi-axis repair) + \
             `pool/stage-copy-accept-rejected-under-forbid` \
             (NeutralOrDeterministic — `forbid` rejects the per-link \
             `<sce:accept-stage-copy-rate>` opt-out, two valid repair \
             paths: remove opt-out vs change policy) + \
             `deploy/stage-copy-policy-unknown` (FixCarriesCandidates \
             over the closed-set `StageCopyPolicy::ALL` = {{warn, \
             error, forbid}}). 247 → 250. Then watching-zenoh RFC \
             §5.K lines 2272-2349 + 2449-2473 land the C13-β \
             anti-flood + stateless_accept family — 5 spec-named \
             codes (`deploy/session-arming-quota-missing` + \
             `deploy/accept-rate-config-missing` + \
             `deploy/session-arming-fields-on-non-arming-link` + \
             `deploy/stateless-accept-required-on-untrusted-source` \
             + `deploy/stateless-accept-key-rotation-shorter-than-\
             lifetime`), all NeutralOrDeterministic. Two additional \
             spec codes (`deploy/session-arming-quota-vs-peer-table-\
             invariant-violated` at line 2460-2462 + \
             `deploy/stateless-accept-extern-not-whitelisted` at \
             line 2466-2469) defer per \
             `[[feedback-silently-broken-hooks]]` — the former \
             references `peer_table.capacity` + \
             `max_handshake_time_s` schema fields the spec uses in \
             invariants but does not declare as deploy.yaml schema \
             entries; the latter requires cross-doc resolution \
             against the §5.I baseline whitelist + loaded \
             target_plugin symbols, which lives on \
             `compile_forge_with_deploy` not parse-time. 250 → 255. \
             Then watching-zenoh RFC §5.M lines 2976-2981 land the \
             C9-γ codegen self-check \
             `reassembly/peer-id-not-zid-on-established-session` — a \
             template-regression guard fired by post-render substring \
             inspection inside `render_buffer_pool_rust` / \
             `render_buffer_pool_c` when the resolved variant is \
             `BufferPoolVariant::Reassembly` and the emitted output \
             does not contain the 16-byte ZID peer-id signature. In \
             well-formed templates the diagnostic never fires (the \
             cross-doc validator `reassembly/untrusted-link-binding` \
             gates non-`established_session` bindings upstream); \
             NeutralOrDeterministic mirrors the \
             `mem/inter-pool-padding-not-emitted` precedent. 255 → 256. \
             Then watching-zenoh RFC §5.K lines 2460-2462 + 2466-2469 \
             close the two C13-β-deferred codes \
             (`deploy/session-arming-quota-vs-peer-table-invariant-violated` \
             + `deploy/stateless-accept-extern-not-whitelisted`) via \
             the C13 deferred-2 atomic: the first lands the \
             `peer_table` + `max_handshake_time_s` schema fields on \
             the `stateless_accept` block (NeutralOrDeterministic \
             three-axis repair); the second wires a sorted-union \
             baseline + target-plugin allowlist check at the \
             orchestrator level (FixCarriesCandidates over the \
             union). 256 → 258. Then watching-zenoh RFC §5.C lines \
             849-856 + §5.M lines 2982-2994 land the C10-α \
             listener-link sibling-pair: two NeutralOrDeterministic \
             codes (`link/listener-link-not-paired-with-established-\
             sibling` codegen self-check + `reassembly/binding-on-\
             unpaired-listener` author-facing cross-doc). The \
             orchestrator-resolved `listener_links: BTreeSet<String>` \
             is the single source of truth — populated by joining \
             deploy `trust_class: session_arming` × machine source \
             SCXML `Accepting.*` substate-present (Q-C10-5 a) and \
             consumed by both (a) `validate_reassembly_cross_doc`'s \
             new session-arming branch which fires `binding-on-\
             unpaired-listener` when the link is not a listener \
             (Q-C10-4 a, narrowing the historic \
             `reassembly/untrusted-link-binding` to Untrusted-only \
             after C10-α) and (b) `render_link_rust` + \
             `render_link_c` template extensions emitting a \
             durable-suffix Sibling half (`EstablishedSession` Rust \
             struct / `_established_session_t` C11 typedef per \
             Q-C10-3 + Q-C10-5). Post-render substring grep mirrors \
             the C9-γ `reassembly/peer-id-not-zid-on-established-\
             session` precedent (generator.rs:10225). 258 → 260. Then \
             watching-zenoh RFC §5.N lines 3031-3062 land the C10-β \
             multi-link concurrency contract — three \
             NeutralOrDeterministic codes (`link/concurrent-count-\
             exceeds-scheduler-slots` MCU-only via \
             `floor(tick_period_us / per_link_budget_us)` slot \
             derivation mirroring C2-γ \
             `validate_machine_scheduler_worker_capacity`; \
             `link/per-link-budget-exceeds-tick-period` literal \
             code-name reading per Q-C10-β-3 a; \
             `link/inbound-event-queue-unsized` extending \
             `compile_scxml_with_imports` pass-2 cross-doc to verify \
             SCXML `event_queue_capacity` OR deploy \
             `default_event_queue_capacity` source for any link \
             carrying inbound events per Q-C10-β-4 a). New \
             `MachineSchedulerConfig.per_link_budget_us: Option<u32>` \
             schema field per spec line 3056-3057 verbatim drives the \
             first two codes; the third closes the §5.J.2 + §5.N \
             event-queue-size axis the C3 Atomic B-γ1 field-only \
             landing deliberately left unconnected. AP `LinkBus` + MCU \
             round-robin templates emit per-machine sibling artifacts \
             (Rust + C11 only, matching the C10-α link.* footprint \
             per Q-C10-β-7 a) via a new orchestrator pipeline path \
             that iterates `deploy.machines` and pushes \
             `(<machine>_link_bus.rs / <machine>_scheduler.{{rs,c}}, \
             GeneratedOutput)` entries alongside the existing \
             basename-keyed per-doc outputs. 260 → 263. Then \
             watching-zenoh RFC §6.2.6 B9 generated-source drift \
             detection ForgeSourceHashMismatch — single code covers \
             both axes (source-hash + template-hash) per Q-§6.2.6-5 \
             lock; emitted from `sce-codegen verify` when embedded \
             header hash diverges from recomputed state. 263 → 264. \
             Then watching-zenoh RFC §5.O Atomic 0 IR provenance \
             pre-emit guard TraceabilityScxmlLineRangeMissing — \
             codegen-internal invariant firing when a node eligible \
             for SCE-MAP marker emission carries `source_location: \
             None`. Spec lines 3289-3290 verbatim: \"Codegen failure \
             ... surfaced via `traceability/scxml-line-range-missing` \
             (codegen-internal)\". NeutralOrDeterministic non_overlap_\
             class — there is no author repair because the fix lives \
             in the parser site that produced the IR node. The pre-\
             emit walker that fires this code is `validate_emission_\
             provenance` (forge/provenance.rs); it lands together \
             with the diagnostic so the IR-field-as-only-consumer \
             (parser-side populates) gains a consumer the same atomic \
             ([[feedback-silently-broken-hooks]]). Atomic 0 part B \
             will extend the validator surface alongside the per-\
             backend marker emission templates and the forge_\
             conformance + sce-rust-tests goldens regen. 264 → 265. \
             Then watching-zenoh RFC §5.O Atomic 1 four codes lock the \
             full per-symbol attribution + sourcemap JSON contract \
             across all 6 backends. `traceability/state-id-collision` \
             (FixCarriesCandidates over the two colliding `<file>:<line>` \
             sites) fires when the cross-IR symbol-table walker finds \
             two distinct nodes whose `<machine>__<state_path>__<artifact>` \
             triples mangle to the same C identifier — typically \
             XInclude or sce:template composition importing a state \
             fragment whose id collides with a top-level state. \
             `traceability/symbol-name-exceeds-c-identifier-limit` \
             (NeutralOrDeterministic; multi-axis repair) fires when \
             the mangled id exceeds 31 chars per C99 §5.2.4.1; default \
             rendering is warn, escalated to hard-error by \
             `platform.strict_c99_identifiers: true` in deploy.yaml. \
             `traceability/sourcemap-source-hash-mismatch` \
             (NeutralOrDeterministic; regenerate to repair) is the \
             codegen-invariant check that the sourcemap JSON's \
             `source_hash` field is byte-equal to the per-file §6.2.6 \
             header's `source-hash` value (spec lines 3321-3324). \
             `traceability/sce-map-attribute-stripped` \
             (NeutralOrDeterministic; dual-emit fallback covers the \
             strip) is the OQ-W16 (b) empirical preservation guard — \
             fires from `sce-codegen addr2sce` when a rustdoc JSON dump \
             contains no `#[doc = \"SCE-MAP: ...\"]` for a function \
             whose sourcemap entry says one should exist. The \
             `// SCE-MAP:` line-comment dual-emit path (default since \
             §5.O Atomic 0c) is the fallback the diagnostic signals \
             toward, not a hard failure. 265 → 269. Then the §5.O \
             Atomic 1 follow-up adds the traceability ownership \
             boundary walker: `traceability/meta-generated-source-line- \
             marker-missing` (NeutralOrDeterministic; codegen-internal, \
             empty fix) fires from \
             `forge::sourcemap::validate_emitted_files_have_markers` \
             when an SCE-emitted file (identified by a §6.2.6 drift \
             header) carries no `SCE-MAP:` marker — the textbook \
             boundary contract per ARCHITECTURE.md \"Traceability \
             Ownership Boundary\" (external meta-generator output is \
             silently out-of-scope by virtue of carrying no drift \
             header). 269 → 270. Then watching-zenoh RFC §5.2 Round F-α \
             lands the MCU driver/class boundary pair: \
             `mcu/driver-header-not-found` (NeutralOrDeterministic; \
             author-domain repair) fires at compile-model time when a \
             `<sce:driver href=\"...\"/>` reference cannot be resolved \
             against `deploy.yaml`'s `platform.driver_root` (or the \
             SCXML file's parent directory as fallback). \
             `mcu/section-attribute-on-non-mcu-target` \
             (NeutralOrDeterministic; multi-axis repair) fires at \
             codegen entry when `platform.c11_section_attribute` is \
             present but the target backend is not C11 — mirrors the \
             Q-Call-7 non-MCU reject pattern (Q-Round-F-D3). Together \
             they pin the §5.2 driver/class boundary policy on the \
             C11 backend: SCE emits the statechart class + reference; \
             cross-TU signature verification stays the C compiler's \
             job (Q-Round-F-D2). 270 → 272. Then RFC variant-default-\
             uniformity Atomic α (claudedocs/rfc-variant-default-\
             uniformity.md) lands the parse-time duplicate-default-arm \
             guard: `codec/variant-duplicate-default-arm` \
             (NeutralOrDeterministic; both offending arm values ride \
             `key_fragments`) fires inside `parse_codec_variant` when \
             two `<sce:arm default=\"true\"/>` are declared on the \
             same `<sce:variant>`. Atomic α surface is schema-only — \
             the `FlagDef.value: Option<u64>` + `VariantArm.is_default: \
             bool` model fields are parsed but no cross-doc validation \
             or codegen change yet; Atomic β/γ build on this baseline. \
             274 → 275. \
             Then RFC variant-default-uniformity Atomic γ-1 added two \
             cross-doc validators: CodecVariantDefaultArmMidMismatch fires \
             when the outer marked-default arm's value differs from the \
             inner codec's matching peek-byte <sce:flag value=>, and \
             CodecVariantArmInnerMidUndeclared fires when the inner codec \
             selected by a marked-default arm declares no wire-MID flag \
             at all. Both gate emission of the new β-chain Default \
             contracts on round-trip safety. 275 → 277.",
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
