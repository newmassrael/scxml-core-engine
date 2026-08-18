// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Machine-readable diagnostic schema for SCE Forge errors.
//
// Upstream consumers (LangGraph-style triage, IDEs, CI) consume this
// format. The design leans on four invariants:
//
//   1. `code` is a closed enum (`DiagnosticCode`), not a free string —
//      consumers dispatch by variant, not by parsing text.
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
    /// The call site a preprocessor substituted from, when the
    /// rejected value was synthesised. Default `None` — overridden by
    /// `Located<E>`, the only wrapper that carries one.
    fn diagnostic_expanded_from(&self) -> Option<Location> {
        None
    }

    fn to_single_diagnostic(&self) -> Diagnostic {
        let payload = self.diagnostic_payload();
        let location = self.diagnostic_location();
        let expanded_from = self.diagnostic_expanded_from();
        // Read before the record takes ownership of the field.
        let synthesised = expanded_from.is_some();
        let id = compute_id(
            payload.code,
            payload.stage,
            location.as_ref().map(|l| l.file.as_str()),
            &payload.key_fragments,
        );
        Diagnostic {
            schema_version: SCHEMA_VERSION,
            id,
            generator: crate::GENERATOR_COMMIT,
            code: payload.code,
            stage: payload.stage,
            spec: payload.code.spec_anchor(),
            message: self.to_string(),
            location,
            expanded_from,
            expected: payload.expected,
            actual: payload.actual,
            // One place decides whether a repair proposal survives to
            // the wire, so the ~100 sites that build one only have to
            // say what they know. Two rules apply here:
            //
            //  * a choice variant with an empty set is no repair at
            //    all (§3.1), and
            //  * a substitution proposal against a synthesised value
            //    cannot be performed — `actual` is not in the file
            //    `location` names, and rewriting the template row it
            //    does name changes every other expansion of that
            //    template. `expanded_from` travels instead, which
            //    says what happened and where the parameters came
            //    from without proposing an edit that would be wrong.
            fix: payload
                .fix
                .and_then(Fix::with_a_choice_to_offer)
                .and_then(|fix| fix.applicable_to_a_synthesised_value(synthesised)),
            spec_provenance: Vec::new(),
            question_kind: None,
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
/// `"stable"` by the procedure in `SCE_ERROR_CONTRACT.md` §8.1, which
/// is a deliberate editorial act and explicitly *not* an automated
/// threshold: a maintainer decides the schema has settled — an external
/// consumer having committed to the format, or churn having stopped —
/// and lands the const, the schema header and the
/// `SCE_WIRE_CONTRACTS.md` row in one commit. (An earlier spelling of
/// this doc cited "30 consecutive days at HEAD" as the criterion §8.1
/// sets; §8.1 sets no such number, and quoting a threshold the contract
/// does not carry would have let a flip claim an authority it never
/// had.) Emitted as `x-sce-schema-status`
/// at the top of `schemas/sce-diagnostic.v1.schema.json` so downstream
/// consumers can read the signal without linking this crate. The
/// `schema_file_declares_status` test guards the two declarations
/// against drift.
///
/// This constant plus that drift test are what realise the RFC
/// §synth-6.1 diagnostic contract: a code lands in the contract doc,
/// the accepted-subset appendix and the schema simultaneously, and the
/// declaration cannot drift between the Rust source and the schema file.
pub const SCHEMA_STATUS: &str = "pre-release";

/// A single machine-readable diagnostic, one record per NDJSON line.
///
/// Serialized field order is fixed: `v` first so any consumer can
/// version-gate before reading anything else, `id` second so streams
/// can dedup without a full parse, `generator` third so attribution
/// is available without reading the payload.
#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    /// Wire-format version. Always present, always first. Consumers that
    /// see a higher value than they were built against should fall
    /// back to a best-effort parse rather than crash.
    #[serde(rename = "v")]
    pub schema_version: u32,

    /// Content-hash id. Prefix names the algorithm so future migration
    /// (e.g. to blake3) can be rolled out without breaking consumers
    /// that pattern-match the format.
    pub id: String,

    /// Commit of the generator that emitted this record — the same value
    /// the stdout manifest carries as `generator` and `--version` reports
    /// in parentheses. `"unknown"` on a build with no git checkout to
    /// read; see [`crate::GENERATOR_COMMIT`].
    ///
    /// Present on **every** record because a rejected run writes no
    /// manifest at all: stdout stays empty and the exit code carries the
    /// failure, so on the path a repair loop actually iterates on, this
    /// record is the only thing the consumer receives. §8.1 tells
    /// consumers to pin a specific commit rather than rely on `v1` while
    /// the schema is `pre-release` — an instruction that cannot be
    /// followed if the payload does not name the commit it came from.
    ///
    /// Per-record rather than once per stream: a consumer that quotes a
    /// single NDJSON line in a bug report must carry the attribution
    /// with it, and a record that needs a sibling line to be interpreted
    /// is not self-describing.
    ///
    /// Third rather than first so `v` and `id` keep the positions the
    /// version-gate and stream-dedup below rely on.
    pub generator: &'static str,

    /// Closed enum, serialized as a slash-path string. Consumers dispatch
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

    /// The `<sce:use>` whose parameters synthesised the rejected
    /// value, present only when a preprocessor assembled it.
    ///
    /// `location` names the row the value occupies — after template
    /// expansion, a row in the *template*. That row shows the shape
    /// (`target="tick_{$n}"`), not the value that was rejected
    /// (`tick_1`), and editing it rewrites every expansion rather
    /// than the one that failed. This names the call site that chose
    /// the parameters, which is the coordinate that tells the
    /// expansions apart. A consumer holding both can describe the
    /// rejection completely; a consumer holding only `location` sees
    /// a row that does not contain `actual` and has no way to learn
    /// why.
    ///
    /// Its presence is also the signal that the value is synthetic,
    /// which is why no substitution `fix` accompanies it — see
    /// [`Fix::with_a_choice_to_offer`]'s call site.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expanded_from: Option<Location>,

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
    /// signals — when populated, consumers apply (or choose) based on
    /// the variant; when absent, no structured repair exists and
    /// there is no fallback to `expected`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<Fix>,

    /// NL→IR Mapping Roadmap Item 6 — spec-document anchors that
    /// justify the rejected node. SCE never infers this; IR
    /// generators (NL→IR pipelines, ARXML transcoders) populate it
    /// when they know the spec origin. Pass-through field on the
    /// diagnostic wire — `Vec::is_empty()` skips serialisation so
    /// the existing byte-stable goldens stay byte-stable for any
    /// diagnostic the upstream did not populate.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub spec_provenance: Vec<crate::provenance::SpecProvenance>,

    /// NL→IR Mapping Roadmap Item 6 — coarse routing label so IDE
    /// integrations and triage tooling can dispatch errors by
    /// *kind* (structural / unit-missing / ambiguous-mapping / …)
    /// rather than memorising the full code catalogue. Absent on
    /// purely structural rejections that map cleanly onto
    /// [`Self::code`] alone.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub question_kind: Option<QuestionKind>,
}

/// NL→IR Mapping Roadmap Item 6 — diagnostic routing label.
///
/// Extensible: consumers must treat unknown values as
/// `Structural` (the fallback bucket) per the schema's
/// "ignore unknown" rule. New variants are additive within v1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionKind {
    /// The author left a value at SCE's implicit default and the
    /// rejection rule asks them to make the choice explicit.
    ImplicitDefault,
    /// Multiple NL→IR mapping candidates were equally plausible —
    /// the IR generator could not pick a single answer.
    AmbiguousMapping,
    /// Two spec documents disagree at the spec source — neither
    /// the author nor SCE can pick without a domain decision.
    CrossDocConflict,
    /// A numeric or measurement value was left dimensionless
    /// where a unit is required (Roadmap Item 4 territory).
    UnitUnspecified,
    /// The author used a vocabulary term SCE does not recognise
    /// and no closed candidate set narrows the choice.
    UnknownVocabulary,
    /// Pure structural well-formedness — fallback for any
    /// rejection that does not fit the more specific buckets.
    Structural,
}

impl QuestionKind {
    /// Stable snake_case string used by drift guards and hash
    /// inputs. Matches the serde rename produced by
    /// `#[serde(rename_all = "snake_case")]`.
    pub fn as_str(&self) -> &'static str {
        match self {
            QuestionKind::ImplicitDefault => "implicit_default",
            QuestionKind::AmbiguousMapping => "ambiguous_mapping",
            QuestionKind::CrossDocConflict => "cross_doc_conflict",
            QuestionKind::UnitUnspecified => "unit_unspecified",
            QuestionKind::UnknownVocabulary => "unknown_vocabulary",
            QuestionKind::Structural => "structural",
        }
    }
}

/// Exhaustive list of [`QuestionKind`] variants in declaration
/// order. The drift guard
/// [`tests::json_schema_enums_match_rust_source_of_truth`] reads
/// the JSON schema and asserts byte equality against this slice.
pub const ALL_QUESTION_KINDS: &[QuestionKind] = &[
    QuestionKind::ImplicitDefault,
    QuestionKind::AmbiguousMapping,
    QuestionKind::CrossDocConflict,
    QuestionKind::UnitUnspecified,
    QuestionKind::UnknownVocabulary,
    QuestionKind::Structural,
];

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
/// match). Consumers depending on SCE can enumerate this type to build
/// a complete dispatch table.
#[derive(Debug, Clone, Copy, Serialize)]
pub enum DiagnosticCode {
    #[serde(rename = "xml/parse")]
    XmlParse,
    #[serde(rename = "xml/schema-validation")]
    XmlSchemaValidation,
    // ── Top-level parser-entry errors (§wire-W4 α-strict). The two
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
    //    repair shape so consumers can dispatch: missing-href gets a
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
    // ── sce:template preprocessing (AOT-only).
    //    Split by repair shape so consumers can dispatch without
    //    parsing message text: missing-attribute / missing-param
    //    carry deterministic add_attribute fixes, unknown-param
    //    lists declared names so consumers correct typos, cycle /
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
    /// The preprocessor pass never ran: a directive survived into
    /// parsing. Sits with the `template-*` codes because it is the
    /// ninth way template expansion can fail to deliver a row — the
    /// one where nothing was attempted.
    #[serde(rename = "xml/preprocessor-not-run")]
    XmlPreprocessorNotRun,

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
    #[serde(rename = "validation/native-action-placement")]
    ValidationNativeActionPlacement,
    #[serde(rename = "validation/native-action-argument")]
    ValidationNativeActionArgument,
    #[serde(rename = "validation/native-action-signature-conflict")]
    ValidationNativeActionSignatureConflict,
    #[serde(rename = "validation/mesh-rpc-reserved-param")]
    ValidationMeshRpcReservedParam,
    #[serde(rename = "validation/mesh-rpc-missing-target")]
    ValidationMeshRpcMissingTarget,
    #[serde(rename = "validation/mesh-rpc-duplicate-target")]
    ValidationMeshRpcDuplicateTarget,
    #[serde(rename = "validation/removed-attribute")]
    ValidationRemovedAttribute,
    // ── Forge bytes-typed slot capacity contract. The
    //    inconsistency is between two SCXML-declared caps (e.g.
    //    `sce:response-max-size` on a `<send>` exceeds
    //    `sce:max-size` on the destination `<data>` slot), caught
    //    at parse time before any backend codegen. ────────────
    #[serde(rename = "validation/bytes-max-size-violation")]
    ValidationBytesMaxSizeViolation,
    // ── NL→IR Mapping Roadmap Item 1: sce:req traceability attribute.
    //    Opaque token by design (SCE assigns no semantics to the id
    //    shape), but a duplicate token on a single node masks a
    //    missing-second-annotation as a phantom double-count in
    //    req-coverage NDJSON. Rejected at parse time so authors fix
    //    it locally. ─────────────────────────────────────────────
    #[serde(rename = "validation/duplicate-requirement-id")]
    ValidationDuplicateRequirementId,

    // ── NL→IR Mapping Roadmap Item 5: sce:unresolved placeholder.
    //    Default builds carry the marker silently (the model + the
    //    `sce-codegen unresolved` NDJSON report expose it for IDE
    //    / linter / CI consumers). `--strict-unresolved` lifts the
    //    marker to this build-failing rejection so production CI
    //    cannot merge unresolved IR. ─────────────────────────────
    #[serde(rename = "validation/unresolved-placeholder")]
    ValidationUnresolvedPlaceholder,

    // ── NL→IR Mapping Roadmap Item 2: cross-kind typed binding.
    //    Three diagnostics for the silent-broken pattern where an
    //    importing kind's expression references `<alias>.<field>` on
    //    an imported kind: field-not-found (with closed
    //    `Fix::ReplaceOneOf` candidate set), type-mismatch against the
    //    enclosing use-site contract, and defensive circular import
    //    detection. Today wired only on the Forge→Forge path; the
    //    codes are kind-agnostic so a future Statechart→Forge binding
    //    extends the wiring without renaming. ────────────────────
    #[serde(rename = "validation/cross-kind-field-not-found")]
    ValidationCrossKindFieldNotFound,
    #[serde(rename = "validation/cross-kind-type-mismatch")]
    ValidationCrossKindTypeMismatch,
    #[serde(rename = "validation/cross-kind-circular-dependency")]
    ValidationCrossKindCircularDependency,

    // ── Algorithm kind (SCE Protocol-Synthesis RFC §synth-5-A, item A3).
    //    Parser-stage sema for the pure-function kind. Three of
    //    the six RFC §synth-5-A diagnostics are implemented; the rest
    //    (`return-type-mismatch`, `while-unbounded`,
    //    `call-cycle`) need typed expression flow / deploy-yaml
    //    MCU detection / cross-file import resolution and wait
    //    until a consumer needs them. ─────────────────────────
    #[serde(rename = "algorithm/local-shadows-param")]
    AlgorithmLocalShadowsParam,
    #[serde(rename = "algorithm/lvalue-unsupported")]
    AlgorithmLvalueUnsupported,
    #[serde(rename = "algorithm/return-missing")]
    AlgorithmReturnMissing,

    // ── Algorithm-over-BC dispatch (RFC §synth-5-A line 311 + §synth-5-L lines
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

    // ── Byte-buffer-build (SCE byte-buffer-build, SCE_FORGE.md §4.12). The
    //    `<sce:append>` statement's two semantic failures: a target that is
    //    not a declared `bytes` buffer (rides `FixCarriesCandidates` — the
    //    visible buffer names) and an RHS whose static type is neither
    //    `uint8` nor `bytes` (rides `ExpectedIsMetadata` — expected = the
    //    accepted type set). Attribute-shape rules (capacity required on a
    //    bytes buffer / forbidden on a scalar, returns-max-size required on a
    //    bytes return, capacity == returns-max-size) reuse the generic
    //    `validation/missing-attribute` / `validation/invalid-attribute`
    //    codes. ───────────────────────────────────────────────────────────
    #[serde(rename = "algorithm/append-target-not-buffer")]
    AlgorithmAppendTargetNotBuffer,
    #[serde(rename = "algorithm/append-type-mismatch")]
    AlgorithmAppendTypeMismatch,

    // ── SCXML semantic-validation (§wire-W5). Three of the four
    //    SCXML semantic failures fold into existing `validation/*`
    //    codes per the W4 D4 fold precedent — concept identity:
    //    "name does not resolve to declared symbol" is the same
    //    failure shape regardless of which document type produces
    //    it. Only this code is W3C-SCXML-specific (top-level
    //    `<script>` rejection per §5.8 has no forge analog). ──
    #[serde(rename = "scxml/top-level-script-unloaded")]
    ScxmlTopLevelScriptUnloaded,
    // ── §scxml-3.2 `datamodel` attribute. The attribute names the
    //    languages a document's expressions are written in; SCE read it
    //    into a string that no decision consulted, so `xpath` and any
    //    invented token compiled and were evaluated by whichever engine
    //    the deployment injected. `scxml/unsupported-datamodel` refuses
    //    the value, and `scxml/null-datamodel-forbids-construct` refuses
    //    a construct whose language the declared model does not have
    //    (§scxml-B-1). ────────────────────────────────────────────────
    #[serde(rename = "scxml/unsupported-datamodel")]
    ScxmlUnsupportedDatamodel,
    #[serde(rename = "scxml/null-datamodel-forbids-construct")]
    ScxmlNullDatamodelForbidsConstruct,
    // ── NL→IR Mapping Roadmap Item 3 — Statechart graph
    //    reachability. BFS from the document `initial` (plus the
    //    parallel-all-children, compound-initial-cascade, and history
    //    default-target entry rules) computes the design-time reach
    //    set; a state outside the set is reported as
    //    `scxml/unreachable-state`. For an unreachable state that
    //    carries `<transition>` elements, the per-transition variant
    //    `scxml/dead-transition` is emitted in preference so the author
    //    sees the concrete element to repair. ───────────────────────
    #[serde(rename = "scxml/unreachable-state")]
    ScxmlUnreachableState,
    #[serde(rename = "scxml/dead-transition")]
    ScxmlDeadTransition,
    // ── NL→IR Mapping Roadmap Item 3 — event-set
    //    exhaustiveness. Fires when a compound `<state>` has sibling
    //    children that disagree on whether a given event is handled,
    //    with no parent-level fallthrough. Narrow heuristic (requires
    //    a shared event-vocabulary `common ground` across siblings)
    //    keeps W3C IRP / conformance / downstream-consumer at zero false
    //    positives; `sce:unhandled` on the child that leaves the event
    //    unhandled escapes genuine intent-gap cases. The two companion
    //    codes police that declaration so it cannot decay into prose:
    //    `contradictory-unhandled-declaration` when the declaring state
    //    does handle the event, `stale-unhandled-declaration` when the
    //    declared event is not a gap under its parent. All three
    //    NeutralOrDeterministic: repair is author-domain (add the
    //    transition, add a fallthrough, declare the absence, or delete
    //    the declaration). ───────────────────────────────────────────
    #[serde(rename = "scxml/non-exhaustive-event-handling")]
    ScxmlNonExhaustiveEventHandling,
    #[serde(rename = "scxml/contradictory-unhandled-declaration")]
    ScxmlContradictoryUnhandledDeclaration,
    #[serde(rename = "scxml/stale-unhandled-declaration")]
    ScxmlStaleUnhandledDeclaration,
    // ── NL→IR Mapping Roadmap Item 3 — guard analysis.
    //    `scxml/always-false-guard` fires when a transition's
    //    `cond` expression is statically determinable as false
    //    (literal `false`, numeric `0`, `N==M` with differing
    //    numeric literals, `N!=N`). `scxml/shadowed-transition`
    //    fires when an unconditional transition precedes a guarded
    //    sibling with the same event descriptor — per W3C SCXML
    //    §5.10 the first matches and the later is dead. Both
    //    NeutralOrDeterministic: repair is author-domain (remove,
    //    rewrite, or reorder the transition). Language-prefixed
    //    conditions (`cpp:`, `kotlin:`, `rust:`) stay opaque to
    //    keep the false-positive surface at zero across the W3C
    //    IRP / conformance / downstream-consumer corpora. ──────────
    #[serde(rename = "scxml/always-false-guard")]
    ScxmlAlwaysFalseGuard,
    #[serde(rename = "scxml/shadowed-transition")]
    ScxmlShadowedTransition,
    // ── SCE Protocol-Synthesis RFC §synth-5-E sample-callback SCXML on-sample family ──
    // Author-facing rules for `<sce:on-sample>` SCE extension: the
    // structural diagnostics (placement, uniqueness,
    // event-name-conflict) here, plus the cross-ref diagnostics
    // (link-not-declared, link-wrong-kind) that resolve against the
    // SceCrossDocRegistry.
    #[serde(rename = "scxml/on-sample-invalid-parent")]
    ScxmlOnSampleInvalidParent,
    #[serde(rename = "scxml/on-sample-link-duplicate-in-state")]
    ScxmlOnSampleLinkDuplicateInState,
    #[serde(rename = "scxml/on-sample-event-name-conflict")]
    ScxmlOnSampleEventNameConflict,
    // ── SCE Protocol-Synthesis RFC §synth-5-E sample-callback cross-ref family ──
    // Cross-reference resolution surface for `<sce:on-sample link="X">`
    // against the build's `SceCrossDocRegistry`, complementing the
    // structural codes above.
    // `link-wrong-kind` is wired forward-compat — today
    // `ScxmlDocKind` has the `Link` variant only, so the validator's
    // match never reaches the `Some(non-Link)` arm in production.
    #[serde(rename = "scxml/on-sample-link-not-declared")]
    ScxmlOnSampleLinkNotDeclared,
    #[serde(rename = "scxml/on-sample-link-wrong-kind")]
    ScxmlOnSampleLinkWrongKind,

    // ── Listener-role declaration — top-level `<sce:session-role
    //    kind="..."/>` SCXML extension structural validators: the
    //    parse-time kind-value family. The three cross-doc
    //    partial-claim codes below join this set with deploy.yaml
    //    `LinkConfig.role`.
    #[serde(rename = "scxml/unknown-session-role-kind")]
    ScxmlUnknownSessionRoleKind,
    #[serde(rename = "scxml/duplicate-session-role-declaration")]
    ScxmlDuplicateSessionRoleDeclaration,
    // ── Listener-role typed-per-direction cross-doc partial-claim
    //    family + role × trust-class matrix validator.
    //    Three NeutralOrDeterministic codes covering each direction
    //    of the listener-role declaration cross-claim. The legacy
    //    silent-skip discipline at `lib.rs:3210-3220` becomes a typed
    //    diagnostic with `key_fragments` on `(machine, link_name)` or
    //    `(machine, source)`.
    #[serde(rename = "link/deploy-role-listener-without-scxml-accept-side-role")]
    LinkDeployRoleListenerWithoutScxmlAcceptSideRole,
    #[serde(rename = "scxml/accept-side-role-without-listener-link")]
    ScxmlAcceptSideRoleWithoutListenerLink,
    #[serde(rename = "link/role-listener-with-non-session-arming-trust-class")]
    LinkRoleListenerWithNonSessionArmingTrustClass,
    // ── Listener-role migration-helper — fires when an SCXML
    //    carries reserved `Accepting.*` state
    //    ids but no `<sce:session-role kind="accept-side"/>`
    //    declaration. Repurposes the legacy `accepting_substate_
    //    present` walker into a typed parser-time author diagnostic
    //    rather than deleting it outright.
    #[serde(rename = "scxml/accept-side-states-without-role-declaration")]
    ScxmlAcceptSideStatesWithoutRoleDeclaration,
    // ── Declared-consumption invariant (SCE Protocol-Synthesis RFC §synth-5-M
    //    lines 2841-2861) — `peer_table.capacity × per_peer_quota ≥
    //    slot_count` invariant. Forge buffer-pool declares its quota
    //    + slot count; deploy.yaml link declares its peer-table
    //    capacity; cross-doc validator catches violations. Closes
    //    the declared-consumption coverage gap an earlier
    //    placeholder had deferred.
    #[serde(rename = "reassembly/per-peer-quota-build-invariant-violated")]
    ReassemblyPerPeerQuotaBuildInvariantViolated,

    #[serde(rename = "expression/empty")]
    ExpressionEmpty,
    #[serde(rename = "expression/lex")]
    ExpressionLex,
    #[serde(rename = "expression/unsupported-construct")]
    ExpressionUnsupportedConstruct,
    // A standard-library *name* the ECMAScript datamodel does not
    // carry — `words.map(...)`, `JSON.serialize(...)`. Distinct from
    // `unsupported-construct`, which is about grammar and has no
    // alternative to name: here the vocabulary that does exist is a
    // closed set, so it rides `Fix::ReplaceOneOf`.
    #[serde(rename = "expression/unsupported-builtin")]
    ExpressionUnsupportedBuiltin,
    // A free identifier nothing declares — `conut + 1` beside a
    // `<data id="count">`. Distinct from `unsupported-builtin`, which
    // names something the *language* has: here the repair comes from
    // the document's own declarations, so the candidates are the near
    // misses among them and there may be none.
    #[serde(rename = "expression/unknown-identifier")]
    ExpressionUnknownIdentifier,
    // A name this datamodel provides, written as a call — `t.length()`,
    // `Math.PI()`. Distinct from `unsupported-builtin`, which says the
    // name is absent: here it is present, so there is nothing to offer
    // in its place and the repair is to drop the call.
    #[serde(rename = "expression/property-not-callable")]
    ExpressionPropertyNotCallable,
    // A namespace this datamodel installs, written as the call itself —
    // `Math()`, `new Object()`. Distinct from `property-not-callable`,
    // whose repair is to drop the call: a namespace is not a value
    // either, so what is left after dropping it is a second refusal.
    // The members that may stand there ride `expected` as metadata,
    // because choosing one is choosing its arguments too.
    #[serde(rename = "expression/namespace-not-callable")]
    ExpressionNamespaceNotCallable,
    // The same namespace read as a value — `v = Math`, `Math[key]`.
    // Distinct from `namespace-not-callable` because the positions are
    // distinct and so are the members that may stand in them: a read may
    // name `Math.PI`, a call may not.
    #[serde(rename = "expression/namespace-not-a-value")]
    ExpressionNamespaceNotAValue,
    // A literal written as the thing being called — `1()`, `null()`.
    // Distinct from `property-not-callable` because that code's repair
    // is to keep the name and drop the call, and there is no name here
    // worth keeping: what is left is a literal nobody wrote the
    // expression to obtain.
    #[serde(rename = "expression/literal-not-callable")]
    ExpressionLiteralNotCallable,
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

    // ── Codegen matrix invariants (SCE Protocol-Synthesis RFC §synth-5-J-4 / §synth-5-J-5).
    //    The variants shipped first as shells (item A1) so downstream
    //    consumers could pin the wire IDs; the producer + matrix walker
    //    live in `forge/codegen_matrix.rs` (landed with the §synth-5-A
    //    algorithm kind, item A3). Stage = Generate
    //    (codegen-time errors share the existing repair-routing key).
    //    See `docs/rfc-sce-protocol-synthesis.md` §synth-5-J-4 commitment. ──
    #[serde(rename = "codegen/mcu-class-kind-on-non-mcu-language")]
    CodegenMcuClassKindOnNonMcuLanguage,
    #[serde(rename = "codegen/generic-kind-backend-emit-missing")]
    CodegenGenericKindBackendEmitMissing,

    // ── §synth-5-J-2 Rust no_std variant rejection (item C3).
    //    Producer: `cmd_generate` walks the parsed SCXML model when
    //    `--no-std` is passed to `sce-codegen generate -l rust` and
    //    rejects documents that depend on std-coupled runtime
    //    features. SCE Protocol-Synthesis RFC §synth-5-J-2 line 1989 prescribes
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
    // Item C3: <data src="..."> requires PathBuf + std::fs::read_to_string,
    // both alloc/OS-coupled. RFC §synth-5-J-2 lines 1989-1994 forbid alloc paths in
    // generated no_std code.
    #[serde(rename = "codegen/no-std-fs-load-not-supported")]
    CodegenNoStdFsLoadNotSupported,
    // Item C3: <invoke> binds child-session lifecycle via
    // Arc<Mutex<Vec<...>>> + HashMap, all alloc-coupled. Same RFC §synth-5-J-2 rule.
    #[serde(rename = "codegen/no-std-invoke-not-supported")]
    CodegenNoStdInvokeNotSupported,

    // ── §synth-5-F build-time const-fold (SCE Protocol-Synthesis RFC §synth-5-F, item A4).
    //    The host interpreter (`forge::const_fold`) emits these
    //    codegen-time errors when a `<sce:fold>` body — or a scalar
    //    `<sce:const init=...>` — fails the foldable substrate, blows
    //    the iteration budget, or produces a value the declared
    //    element / scalar type cannot hold. Stage = Generate (the
    //    interpreter runs inside `lower_algorithm_consts` during
    //    template rendering). Formerly `generate/unsupported-feature`
    //    slug payloads, now first-class wire codes. ──
    #[serde(rename = "algorithm/const-not-foldable")]
    AlgorithmConstNotFoldable,
    #[serde(rename = "algorithm/const-fold-budget-exceeded")]
    AlgorithmConstFoldBudgetExceeded,
    #[serde(rename = "algorithm/const-yield-type-mismatch")]
    AlgorithmConstYieldTypeMismatch,

    // ── §synth-5-B variant primitive (SCE Protocol-Synthesis RFC §synth-5-B, item B1).
    //    Build-time check on `<sce:variant>` codec suffix: the
    //    enumerated `<sce:arm value=...>` set must cover the tag
    //    field's value domain when no `<sce:default>` is declared,
    //    otherwise some incoming tag value would have no matching
    //    branch at runtime. v1 considers uint8 (256) and uint16
    //    (65536) practically enumerable; uint32 / uint64 always
    //    require a default. Stage = Validation. ──
    #[serde(rename = "codec/variant-arm-unreachable")]
    CodecVariantArmUnreachable,

    // ── RFC variant-default-uniformity. Parse-time check on
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

    // ── RFC variant-default-uniformity: cross-doc check
    //    that the outer `<sce:arm default="true" value="X"/>` and
    //    the inner codec's matching peek-byte `<sce:flag value="Y"/>`
    //    declare the same wire constant. Mismatch lands the wrong
    //    arm at decode time. Stage = Validation. ──
    #[serde(rename = "codec/variant-arm-mid-mismatch")]
    CodecVariantArmMidMismatch,

    // ── RFC variant-default-uniformity: cross-doc check
    //    that the inner codec selected by a default-marked outer
    //    arm declares a wire-MID constant via `<sce:flag value="..."/>`.
    //    Absence means the inner's Default zero-fills the dispatch
    //    byte and round-trip breaks. Stage = Validation. ──
    #[serde(rename = "codec/variant-arm-inner-mid-undeclared")]
    CodecVariantArmInnerMidUndeclared,

    // ── Caller-tag variant shape: a variant arm
    //    body resolves to a codec whose `<sce:variant>` is itself in
    //    caller-tag shape (no `tag=` attribute). Caller-tag shape requires the
    //    caller to supply the dispatch tag positionally; in a
    //    variant-arm context there is no natural source for that tag
    //    (the parent dispatcher's own tag selects WHICH arm, not what
    //    to forward), and codegen would emit a call missing the tag
    //    arg → downstream rustc/g++/etc. arity mismatch. Reject
    //    upstream with a typed diagnostic so the author sees the
    //    constraint at codegen time. Stage = Validation. ──
    #[serde(rename = "codec/variant-arm-body-caller-tag-unsupported")]
    CodecVariantArmBodyCallerTagUnsupported,

    // ── RFC variant-default-overlay: deploy.yaml
    //    `variant_defaults:` names a codec + arm value, but the codec
    //    has no <sce:variant> or its declared arms do not include the
    //    overlay value. Stage = Validation. Fix carries the declared
    //    arm value list as candidates so authors see what the legal
    //    choices are. ──
    #[serde(rename = "codec/variant-default-overlay-arm-not-declared")]
    CodecVariantDefaultOverlayArmNotDeclared,

    // ── RFC variant-default-uniformity: every
    //    `<sce:variant>` must declare an `<sce:arm default="true"/>`
    //    marker. Without it codegen would silently re-introduce the
    //    "first declared arm" implicit fallback that the RFC closes.
    //    Stage = Validation. ──
    #[serde(rename = "codec/variant-no-default-arm")]
    CodecVariantNoDefaultArm,

    // ── Parent-tag dispatch: a parent codec's
    //    `<sce:variant-dispatch flag="X.Y">` does not resolve — either
    //    the carrier X or the flag Y is not declared on the parent.
    //    Candidates list supplies available carriers (or flags on
    //    resolved carrier) for typo repair. Stage = Validation. ──
    #[serde(rename = "codec/variant-dispatch-flag-not-resolved")]
    CodecVariantDispatchFlagNotResolved,

    // ── Parent-tag dispatch: a parent codec's
    //    `<sce:variant-dispatch>` names a flag whose `width` cannot
    //    encode the imported codec's arm count. Dispatch domain is
    //    `1 << width`; the imported variant has more arms than that.
    //    Stage = Validation. ──
    #[serde(rename = "codec/variant-dispatch-bit-width-mismatch")]
    CodecVariantDispatchBitWidthMismatch,

    // ── Parent-tag dispatch: a parent codec imports a variant
    //    codec without `<sce:variant-dispatch>` AND the imported codec
    //    has no `default="true"` arm. Decode cannot pick an arm
    //    deterministically. Author adds either a dispatch declaration
    //    or a default arm. Stage = Validation. ──
    #[serde(rename = "codec/variant-dispatch-arms-not-distinguishable-without-default")]
    CodecVariantDispatchArmsNotDistinguishableWithoutDefault,

    // ── Parent-tag dispatch: a parent codec's
    //    `<sce:variant-dispatch>` targets a flag that ALSO carries a
    //    static `value=` constant. Derived and static cannot coexist
    //    on the same bit. Stage = Validation. ──
    #[serde(rename = "codec/variant-dispatch-flag-has-static-value")]
    CodecVariantDispatchFlagHasStaticValue,

    // ── Parent-tag dispatch: a parent codec declares a field
    //    with `<sce:variant-dispatch>` BEFORE the carrier field that
    //    the dispatch flag belongs to. Readable declaration order is
    //    carrier-first; matches wire order. Stage = Validation. ──
    #[serde(rename = "codec/variant-dispatch-carrier-after-embed")]
    CodecVariantDispatchCarrierAfterEmbed,

    // ── Flag inversion: parent's `<sce:flag-bind input="X" ...>`
    //    references a leaf-side input name that the imported codec does
    //    not declare in its `<sce:flag-inputs>` block. Either the leaf
    //    renamed the input or the parent's bind has a typo. Candidates
    //    list supplies the leaf's available input names for typo repair.
    //    Stage = Validation. ──
    #[serde(rename = "codec/flag-bind-input-not-declared")]
    CodecFlagBindInputNotDeclared,

    // ── Flag inversion: parent's `<sce:flag-bind source="...">`
    //    references a source that resolves to neither a local flags-
    //    carrier flag (dotted `<carrier>.<flag>` form) nor one of the
    //    parent's own `<sce:flag-input>` declarations (bare-name chain-
    //    forwarder form). Stage = Validation. ──
    #[serde(rename = "codec/flag-bind-source-not-resolved")]
    CodecFlagBindSourceNotResolved,

    // ── Flag inversion: parent's `<sce:flag-bind>` source width
    //    does not match the leaf-side input's declared width. v1 fixes
    //    flag-input width at 1; multi-bit inputs defer to a reachable
    //    consumer. Stage = Validation. ──
    #[serde(rename = "codec/flag-bind-width-mismatch")]
    CodecFlagBindWidthMismatch,

    // ── Flag inversion: the imported leaf codec declares a
    //    `<sce:flag-input name="X" .../>` but the parent's `<sce:import>`
    //    does not supply a matching `<sce:flag-bind input="X" .../>`.
    //    The leaf would receive an undefined value for that input —
    //    binding-completeness invariant. Stage = Validation. ──
    #[serde(rename = "codec/flag-input-unbound")]
    CodecFlagInputUnbound,

    // ── Flag inversion: a parent's `<sce:import>` declares two
    //    `<sce:flag-bind>` children with the same `input=` attribute.
    //    Each leaf-side input must be bound at most once. Stage =
    //    Validation. ──
    #[serde(rename = "codec/flag-bind-duplicate-input")]
    CodecFlagBindDuplicateInput,

    // ── Flag inversion: a parent's `<sce:flag-bind source="X.Y">`
    //    names a local carrier whose field is declared AFTER the embed
    //    that consumes the bound input. Streaming-order requires
    //    carrier-first. Mirrors the legacy carrier-after-embed ordering
    //    constraint for the inverted shape. Stage = Validation. ──
    #[serde(rename = "codec/flag-bind-carrier-after-embed")]
    CodecFlagBindCarrierAfterEmbed,

    // ── §synth-5-B present-if primitive (SCE Protocol-Synthesis RFC §synth-5-B, item B1).
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

    // ── §synth-5-B repeat primitive (SCE Protocol-Synthesis RFC §synth-5-B, B2). Build-
    //    time check on `<sce:repeat sce:count="X"/>`: the referenced
    //    count field `X` must be declared earlier in the same codec
    //    so the streaming decoder has already consumed it by the time
    //    the repeat loop reads N. A forward reference (count field
    //    declared after the repeat) is rejected here; non-integer
    //    count target and shape mismatches reuse the generic
    //    `validation/invalid-attribute` code. Stage = Validation. ──
    #[serde(rename = "codec/repeat-count-refs-later-field")]
    CodecRepeatCountRefsLaterField,

    // ── §synth-5-B test-vector primitive (SCE Protocol-Synthesis RFC §synth-5-B,
    //    items B2 + B5) ── `<sce:test-vector hex value/>` is
    //    supported on `sce:kind="algorithm"` and `sce:kind="codec"`;
    //    any other kind is rejected here (the kind-specific harness
    //    oracle is the route for everything else). Stage =
    //    Validation. ──
    #[serde(rename = "algorithm/test-vector-unsupported-kind")]
    AlgorithmTestVectorUnsupportedKind,

    // ── §synth-5-B B3 TLV chain primitive (SCE Protocol-Synthesis RFC §synth-5-B) ──
    //    `<sce:tlv-chain>` is MCU-class and the runtime decoder needs a
    //    build-time bound to size its working set; missing `max-depth`
    //    is rejected here. RFC line 488 "max-depth MUST be specified
    //    for MCU targets" + line 533 "Iterative parse only; max-depth
    //    lowers to a max-iter on the chain traversal loop". Repair is
    //    structural — add `max-depth="N"` (N > 0). Stage = Validation. ──
    #[serde(rename = "codec/tlv-chain-depth-unspecified")]
    CodecTlvChainDepthUnspecified,
    /// `on-overflow="truncate"` declared with `terminate-on="entry-flag"`:
    /// a dropped entry's bytes stay under the cursor, where the field
    /// after the chain reads them.
    #[serde(rename = "codec/tlv-chain-truncate-under-entry-flag")]
    CodecTlvChainTruncateUnderEntryFlag,

    // ── §synth-5-B B3 DMA alignment primitive (SCE Protocol-Synthesis RFC §synth-5-B) ──
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

    /// RFC §synth-5-B peek-byte cross-codec contract — a
    /// parent variant's `<sce:peek-byte>` flag layout and an arm body
    /// codec's first `<sce:flags>` field must agree exactly on bit and
    /// width for each named flag. Mismatch surfaces at variant arm
    /// wire-up; repair is structural (align one side). Stage =
    /// Validation.
    #[serde(rename = "codec/peek-byte-flag-layout-mismatch")]
    CodecPeekByteFlagLayoutMismatch,

    // ── §synth-5-C link kind (SCE Protocol-Synthesis RFC §synth-5-C, item B6). MCU-class
    //    byte-stream link endpoint. The parse-time trio
    //    (`link/framer-missing`, `link/link-class-unknown`,
    //    `link/backpressure-undeclared`) fires from the forge parser;
    //    the OS-axis check (`link/class-unsupported-on-target`) fires
    //    via the forge × deploy.yaml integration (`platform.os` lives
    //    per-machine in deploy.yaml per RFC §synth-5-C lines 702-704); the
    //    link↔pool cross-resolution (`link/pool-slot-smaller-than-
    //    framer-max`) fires from `compile_forge_with_imports`. The
    //    listener self-check (`link/listener-link-not-paired-with-
    //    established-sibling`) is declared with the §synth-5-M reassembly
    //    family below. Stage = Validation. ──
    /// `<sce:framer ref="..."/>` is required on `sce:kind="link"`;
    /// absence is rejected at parse time so the codegen never
    /// reaches the missing-codec branch. Repair: add the framer ref.
    #[serde(rename = "link/framer-missing")]
    LinkFramerMissing,

    /// `<sce:link-class>` body text is not in the closed enum
    /// (`udp` / `tcp` / `serial` / `websocket` / `raw_eth` per RFC
    /// §synth-5-C lines 765-771). Promotes the generic
    /// `validation/invalid-attribute` to a dedicated link-kind code so
    /// authors and downstream consumers key on the link-class violation
    /// directly. Repair: replace the value with one of the listed
    /// candidates (`fix: ReplaceOneOf`).
    #[serde(rename = "link/link-class-unknown")]
    LinkLinkClassUnknown,

    /// `<sce:backpressure>` element is required on `sce:kind="link"`
    /// declarations — the policy is load-bearing for the runtime
    /// crate's RX queue behavior under load (RFC §synth-5-C body). The
    /// absence is a hard error (no silent `default-to-drop`) so
    /// authors must declare the policy intentionally. Repair: add a `<sce:backpressure>`
    /// child whose body is `drop`, `block`, or `signal-event`.
    #[serde(rename = "link/backpressure-undeclared")]
    LinkBackpressureUndeclared,

    /// Declared `<sce:link-class>` cannot run on the deploy-resolved
    /// `platform.os` per RFC §synth-5-C lines 765-771 / 838. The matrix
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
    /// `Fix::None`. RFC §synth-5-C lines 793-794 (rx-pool / tx-pool inherit
    /// the §synth-5-E pool model on both sides of the byte-stream link).
    #[serde(rename = "link/pool-slot-smaller-than-framer-max")]
    LinkPoolSlotSmallerThanFramerMax,

    /// RFC §synth-5-C / §synth-5-E cross-resolution: a
    /// `<sce:rx-pool ref>` / `<sce:tx-pool ref>` / `<sce:stage-pool
    /// ref>` on a link kind names no buffer-pool document reachable
    /// from the build. Fires only from the multi-document entry point,
    /// which is handed the closed world and can therefore separate a
    /// typo from a partial topology; the single-document paths keep
    /// their tolerance. Closes the silent-skip that let a
    /// one-character slip in a pool ref switch the deploy-time
    /// burst-absorption and reassembly validators off — those join on
    /// the same name and `continue` when it misses. Candidate set is
    /// the build's declared buffer-pool names ⇒ `Fix::ReplaceOneOf`.
    #[serde(rename = "link/pool-ref-not-declared")]
    LinkPoolRefNotDeclared,

    /// RFC §synth-5-C link framer cross-doc resolution: `<sce:framer
    /// ref>` on a link kind names no codec document reachable from the
    /// build. The pool-ref diagnostic's twin, one join later, and fired
    /// from the same multi-document entry point for the same reason —
    /// only the closed world separates a typo from a partial topology.
    /// Closes the silent-skip that let a one-character slip in the
    /// framer ref switch `link/pool-slot-smaller-than-framer-max` off:
    /// that check reads the codec's worst-case encoded size through
    /// this very name and returns `Ok(())` when it misses, so the typo
    /// shipped a pool slot too small to hold a framed message.
    /// Candidate set is the build's declared codec names ⇒
    /// `Fix::ReplaceOneOf`.
    #[serde(rename = "link/framer-ref-not-declared")]
    LinkFramerRefNotDeclared,

    /// RFC §synth-5-E buffer-pool placement validation (item B7): declared
    /// `<sce:section>` body is not in deploy.yaml `machines.<m>.memory.
    /// sram_regions`. Validate-time — fires only via
    /// [`compile_forge_with_deploy`] when both `deploy` and
    /// `target_machine` resolve and the machine has a `memory` block;
    /// missing pieces skip silently per the deploy-unaware
    /// silent-skip discipline. Repair:
    /// `Fix::ReplaceOneOf` carries the section-name axis (the list of
    /// regions the resolved machine declares) so the author can rename
    /// the pool's `<sce:section>` body or extend the deploy.yaml memory
    /// map. RFC §synth-5-E lines 1000-1023 + 1537 spec anchor.
    #[serde(rename = "mem/pool-section-conflict")]
    MemPoolSectionConflict,

    /// RFC §synth-5-E buffer-pool size validation (item B7): storage footprint
    /// (`slot_count × slot_size`) does not fit inside the resolved
    /// region's `size` field. Validate-time — fires only via
    /// [`compile_forge_with_deploy`] after `mem/pool-section-conflict`
    /// passes (the section must resolve before its size matters); same
    /// silent-skip when deploy.yaml is unavailable. No
    /// candidate set — the repair is to raise the region size in
    /// deploy.yaml or shrink `slot_count` / `slot_size`, both of which
    /// are author choices. RFC §synth-5-E lines 1031-1086 spec anchor.
    #[serde(rename = "mem/pool-too-large")]
    MemPoolTooLarge,

    /// RFC §synth-5-E codegen self-check (item B7): the rendered linker fragment
    /// is missing the explicit `. = ALIGN(<n>);` inter-pool sentinel.
    /// Codegen-invariant violation, not an authoring mistake — fires
    /// only when the buffer-pool linker fragment template itself drops
    /// the sentinel. The artifact makes the inter-pool boundary
    /// diff-visible and protects the post-pool boundary from
    /// master-script INCLUDE re-ordering. RFC §synth-5-E lines 1059-1064.
    #[serde(rename = "mem/inter-pool-padding-not-emitted")]
    MemInterPoolPaddingNotEmitted,

    /// RFC §synth-5-E C5 cache-maintenance validation: pool `<sce:alignment>`
    /// is smaller than the resolved target's `platform.dcache_line_size`
    /// while `cache-policy: maintain` is in effect. Validate-time —
    /// fires only via [`compile_forge_with_deploy`] after section
    /// validation passes (silent-skip when deploy.yaml is
    /// unavailable). Partial-line cache_invalidate_by_addr corrupts
    /// adjacent slot data on the start side. RFC §synth-5-E line 1544 +
    /// §synth-5-I lines 1742-1744 spec anchor.
    #[serde(rename = "mem/cache-line-alignment")]
    MemCacheLineAlignment,

    /// RFC §synth-5-E C5 cache-maintenance validation:
    /// `platform.dcache_line_size` is not a power of two. Validate-time
    /// — fires only via [`compile_forge_with_deploy`]. Load-bearing for
    /// the rules around it, which divide by this value and rely on a
    /// line size no larger than an alignment dividing it. RFC
    /// §synth-5-E line 1544 + §synth-5-I lines 1742-1744 spec anchor.
    #[serde(rename = "mem/dcache-line-size-not-power-of-two")]
    MemDcacheLineSizeNotPowerOfTwo,

    /// RFC §synth-5-E slot-table layout: `<sce:alignment>` is not a
    /// power of two. Both backends lower it to a language alignment
    /// specifier (`_Alignas` / `#[repr(align)]`) and neither language
    /// admits anything else. Parse-time — fires without a deploy.
    /// RFC §synth-5-E lines 1024-1073 spec anchor.
    #[serde(rename = "mem/alignment-not-power-of-two")]
    MemAlignmentNotPowerOfTwo,

    /// RFC §synth-5-E slot-table layout: `<sce:slot-size>` is not a
    /// whole-number multiple of `<sce:alignment>`. The slot size is
    /// the stride between slots, so only the first slot would start
    /// on the declared DMA boundary. Distinct from
    /// [`Self::MemCacheLineAlignment`], which compares the pool's
    /// alignment against the deploy-resolved
    /// `platform.dcache_line_size` under `cache-policy: maintain`;
    /// this one is the pool's own declared boundary and holds under
    /// every policy, with or without a deploy. Parse-time.
    /// RFC §synth-5-E lines 1024-1073 spec anchor.
    #[serde(rename = "mem/slot-size-not-alignment-multiple")]
    MemSlotSizeNotAlignmentMultiple,

    /// RFC §synth-5-E C5 cache-maintenance validation: pool declares
    /// `cache-policy: maintain` (or `non-cacheable`) while the
    /// resolved target platform has `has_dcache: false`. Cache
    /// maintenance call sites are meaningless on a core without a
    /// data cache. Validate-time — fires only via
    /// [`compile_forge_with_deploy`]. Repair: `Fix::ReplaceOneOf`
    /// candidates = `["none"]`. RFC §synth-5-E line 1543 spec anchor.
    #[serde(rename = "mem/cache-policy-unsupported-on-no-dcache-core")]
    MemCachePolicyUnsupportedOnNoDcacheCore,

    /// RFC §synth-5-E C5 cache-maintenance + §synth-5-I author-guard: an
    /// `<sce:extern>` declaration tries to author one of the cache-
    /// maintenance trio (`sce_dcache_clean_by_addr`,
    /// `sce_dcache_invalidate_by_addr`,
    /// `sce_dcache_clean_invalidate_by_addr`). Per spec lines
    /// 1222-1227, cache maintenance is FSM-driven; codegen auto-
    /// injects the externs and emits the calls on the buffer-pool
    /// lifecycle edges. Author authoring is forbidden because it
    /// silently invites the class of bugs ("the maintenance call
    /// sits in the wrong place") the FSM-driven design prevents.
    /// Parse-time — fires before the §synth-5-I baseline whitelist
    /// validator. RFC §synth-5-E line 1548 + lines 1222-1227 spec anchor.
    #[serde(rename = "pool/cache-maintenance-misplaced")]
    PoolCacheMaintenanceMisplaced,

    /// RFC §synth-5-E C5 cache-maintenance config-completeness diagnostic:
    /// a target machine declares `platform.has_dcache: true` without
    /// setting `platform.has_speculative_prefetch`. Validate-time —
    /// fires only via [`compile_forge_with_deploy`] when at least
    /// one buffer-pool with `cache-policy: maintain` exists in the
    /// build. Codegen cannot decide whether to emit the
    /// `free → dma-armed-rx` pre-arm cache-invalidate edge. Author
    /// resolution: declare `has_speculative_prefetch` per the SoC
    /// datasheet (M7+/A-class = true, M3/M4 = false). RFC §synth-5-E line
    /// 1553 spec anchor.
    #[serde(rename = "pool/speculative-prefetch-flag-missing")]
    PoolSpeculativePrefetchFlagMissing,

    /// RFC §synth-5-E C5 cache-maintenance codegen self-check:
    /// `cache-policy: maintain` + `platform.has_speculative_prefetch:
    /// true` resolved, but the rendered buffer-pool template did not
    /// emit a `sce_dcache_invalidate_by_addr` call inside the
    /// `link_arm_rx` body. Codegen-invariant violation, not an
    /// authoring mistake — fires only when the
    /// `tools/codegen/templates/forge/{rust,c}/buffer_pool` template
    /// itself drops the pre-arm invalidate edge. The diagnostic
    /// guards against template regression that would silently
    /// corrupt RX data on M7+ cores. RFC §synth-5-E line 1552 spec anchor.
    #[serde(rename = "pool/cache-pre-arm-invalidate-missing-on-speculative-core")]
    PoolCachePreArmInvalidateMissingOnSpeculativeCore,

    // ── §synth-5-E pool kind ownership contract (SCE Protocol-Synthesis RFC §synth-5-E,
    //    item B7). The attribute family is exposed to
    //    consumer builds through `backends/c/runtime/include/sce/sample.h`,
    //    pulled in by the generated pool header. The diagnostic catches
    //    a future template edit that drops the `#include` — the
    //    coverage would silently disappear without it. Stage =
    //    Validation. Of the related `pool/...` family,
    //    `pool/sample-take-without-stage-pool`,
    //    `pool/sample-callback-signature-non-borrow`, and
    //    `pool/cache-maintenance-misplaced` are declared below;
    //    `pool/ownership-violation` and `pool/slot-leak-on-error-path`
    //    stay unimplemented pending the §synth-5-I `<sce:call>`
    //    intrinsic registry. `pool/clang-tidy-not-configured` is NOT
    //    pending — it was withdrawn from the spec: Clang-Tidy reads the
    //    same frontend that drops the typestate attributes in C, and
    //    the macros expand to nothing there, so it has no annotation to
    //    parse and cannot report what the diagnostic would have
    //    mandated it for. The defensive layer covers that gap by
    //    defaulting on instead. ──
    /// `<sce/sample.h>` runtime header pull-through (the producer of
    /// the `SCE_CONSUMABLE` / `SCE_CALLABLE_WHEN` /
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

    /// SCE Protocol-Synthesis RFC §synth-5-E sample-callback application-layer
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
    /// Schema-locality choice: the stage pool is a *link*
    /// property co-located with rx_pool/tx_pool on the link kind
    /// document, not a deploy-yaml binding property. The
    /// `BindingConfig.stage_pool` field is a deploy-time override
    /// mechanism — orthogonal to this diagnostic.
    #[serde(rename = "pool/sample-take-without-stage-pool")]
    PoolSampleTakeWithoutStagePool,

    /// SCE Protocol-Synthesis RFC §synth-5-E sample-callback callback-path
    /// diagnostic (spec lines 1516-1519): an
    /// `<sce:on-sample callback="rust:...">` attribute carries an
    /// authoring path that fails the `rust:crate::module::fn` path
    /// subset. SCE-side reachable arms today are path-syntax (unknown
    /// language prefix, leading/trailing `::`, malformed segment,
    /// empty path); signature-inspection shape-mismatch arms stay
    /// absent until a consumer needs them — they would extend the
    /// same code.
    /// Diagnostic name preserves spec wording verbatim per
    /// `feedback_spec_mirror_parity.md`; the per-instance message's
    /// reason clause names the specific path-syntax mistake.
    #[serde(rename = "pool/sample-callback-signature-non-borrow")]
    PoolSampleCallbackSignatureNonBorrow,

    // ── §synth-5-D Worker kind (SCE Protocol-Synthesis RFC §synth-5-D, item C2). The
    //    worker primitive is a concurrent execution context driven
    //    by a `<sce:link-rx>` source; it owns an SPSC inbox and
    //    communicates only through that channel + an optional outbox.
    //    Spec line 911 enforces encapsulation: "any non-inbox access
    //    to another worker's state" is a diagnostic. Static
    //    recognition covers two layers (sibling `<sce:import
    //    kind="worker">` + body SCXML data-refs); the `<sce:extern>`
    //    non-inbox-symbol path (item C4 composition) stays
    //    unimplemented until a consumer needs it. Diagnostic names
    //    are spec verbatim per `feedback_spec_mirror_parity.md`. ──
    /// `<sce:body>` or sibling-document scope reaches another
    /// worker's state through a path other than its own inbox + a
    /// recipient's inbox (via `<sce:outbox ref>`). Covered layers:
    /// `<sce:import kind="worker">` rejection (layer 1) and body
    /// SCXML cross-namespace data-ref rejection (layer 2). Layer 3
    /// (`<sce:extern>` non-inbox symbol use in body) stays
    /// unimplemented until a consumer needs the §synth-5-I
    /// intrinsic-registry composition surface.
    ///
    /// Diagnostic name preserves spec wording verbatim
    /// (`worker/shared-mutable-state`) per
    /// `feedback_spec_mirror_parity.md`; the per-instance message's
    /// reason clause names the specific path that crossed the
    /// encapsulation boundary so authors can locate the offending
    /// declaration without grepping. RFC §synth-5-D line 911 spec anchor.
    #[serde(rename = "worker/shared-mutable-state")]
    WorkerSharedMutableState,

    // ── §synth-5-D + §synth-5-I worker cross-resolution + inbox ordering ──────
    //    Worker docs reference (a) the driving link kind via
    //    `<sce:link-rx ref>` and (b) the recipient state machine's
    //    inbox via `<sce:outbox ref>`. Both refs cross-resolve against
    //    the worker doc's own `<sce:import>` declarations
    //    (precedent: `validate_link_pool_framer_resolution` resolves
    //    framer codec aliases the same way). The 2 ordering codes
    //    cover §synth-5-I lines 1752-1758: every SPSC inbox must declare
    //    acquire/release vs relaxed; relaxed-across-cores is a
    //    codegen-invariant guard against unsafe cross-cache-coherency
    //    pairing. Diagnostic names preserve spec wording verbatim per
    //    `feedback_spec_mirror_parity.md`. ──
    /// `<sce:link-rx ref>` names an alias not imported as kind=link.
    /// Repair surface = `Fix::ReplaceOneOf` over the sorted set of
    /// link-kind import aliases on this worker doc. Non-spec
    /// diagnostic; cross-resolution is
    /// SCE's per-doc strengthening of the spec example's elided import
    /// shape.
    #[serde(rename = "worker/link-rx-ref-unknown")]
    WorkerLinkRxRefUnknown,

    /// `<sce:inbox>` declared without an explicit `ordering` attribute.
    /// Spec §synth-5-I lines 1757-1758 verbatim: "no ordering chosen, codegen
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
    /// pins inbox producer and consumer on different cores. Spec §synth-5-I
    /// lines 1755-1756 verbatim: relaxed on cross-worker shared state
    /// is insufficient. Codegen-invariant guard: silent-skip when
    /// `ForgeCompileOptions.worker_placement` is absent (deploy-unaware
    /// silent-skip precedent), fires when explicit cross-core placement coexists
    /// with `relaxed` ordering. Repair surface = no closed candidate
    /// list (author either changes ordering to `acq_rel` or co-locates
    /// the worker on a single core); `fix: None`. Spec-verbatim name
    /// (`worker/inbox-ordering-relaxed-across-cores`).
    #[serde(rename = "worker/inbox-ordering-relaxed-across-cores")]
    WorkerInboxOrderingRelaxedAcrossCores,

    /// Worker doc compiles against a target machine that did not list it
    /// in `deploy.machines.<m>.workers`. Spec §synth-5-D line 912 verbatim
    /// (`worker/scheduler-unsupported` — "worker count exceeds scheduler
    /// slot count"). The deploy-side anchor for the sum check is
    /// [`MeshDeploySchedulerIncompatibleWithWorkerCount`] (spec §synth-5-K
    /// line 2423); this forge-side code fires when
    /// [`crate::compile_forge_with_deploy`] sees a Worker doc whose
    /// `name` is absent from the resolved machine's `workers` map,
    /// signaling the worker was not budgeted into the cooperative
    /// scheduler's tick window. Repair surface = no closed candidate
    /// list (author either adds the worker to deploy.yaml or removes
    /// the Worker doc); `fix: None` per NeutralOrDeterministic class.
    #[serde(rename = "worker/scheduler-unsupported")]
    WorkerSchedulerUnsupported,

    // ── §synth-5-D worker outbox: SCXML-side `<sce:outbox ref>` cross-
    //    resolution against the build-wide
    //    [`crate::forge::cross_doc_registry::SceCrossDocRegistry`]
    //    (orchestrator + registry + 3-kind variant landed in
    //    `3e5e26e9`; these codes are its validator consumer).
    //    Strict-suffix rule (`.inbox`) + recipient kinds
    //    (statechart + worker) + 3-code split per repair axis
    //    (unknown / wrong-kind / suffix-invalid).
    /// `<sce:outbox ref>`'s owner segment does not match any
    /// statechart or worker doc in the build's cross-doc registry.
    /// Distinct from [`Self::WorkerOutboxTargetWrongKind`] (owner
    /// found but kind not in {statechart, worker}); distinct from
    /// [`Self::WorkerOutboxTargetSuffixInvalid`] (syntactic suffix
    /// failure independent of registry state). Repair surface =
    /// `Fix::ReplaceOneOf` over the sorted union of statechart +
    /// worker doc names (each suffixed with `.inbox` so candidates
    /// are drop-in replacements). Non-spec diagnostic splitting the
    /// failure axes by repair surface.
    #[serde(rename = "worker/outbox-ref-unknown")]
    WorkerOutboxRefUnknown,

    /// `<sce:outbox ref>`'s owner segment resolves in the cross-doc
    /// registry but to a kind not in {statechart, worker}. Today the
    /// only other kind the registry holds is `link` (forge link
    /// imports), so a wrong-kind hit usually means the author confused
    /// a link import alias with a statechart name. Repair surface =
    /// `Fix::ReplaceOneOf` over the same sorted statechart + worker
    /// `.inbox` set as [`Self::WorkerOutboxRefUnknown`]. Non-spec
    /// diagnostic splitting the failure axes by repair surface.
    #[serde(rename = "worker/outbox-target-wrong-kind")]
    WorkerOutboxTargetWrongKind,

    /// `<sce:outbox ref>` declares a suffix other than `inbox`
    /// (including missing dot entirely). Spec §synth-5-D line 895 example
    /// writes `session_fsm.inbox`; spec line 1998 codegen table fixes
    /// the recipient queue name to `inbox`. Repair is deterministic:
    /// keep the authored owner segment, replace the suffix with
    /// `inbox`. `Fix::ReplaceWith` carries `"{owner}.inbox"`. Single-
    /// value repair → `NeutralOrDeterministic` non-overlap class.
    /// Non-spec diagnostic; the strict-suffix rule splits this
    /// syntactic axis from the registry-dependent pair.
    #[serde(rename = "worker/outbox-target-suffix-invalid")]
    WorkerOutboxTargetSuffixInvalid,

    // ── §synth-5-M Fragment-reassembly buffer-pool variant diagnostics
    //    (SCE Protocol-Synthesis RFC §synth-5-M lines 2944-2945, item C9). The two
    //    parse-level structure codes live here; the cross-doc
    //    validators that reference §synth-5-K `links.<name>.{mtu_bytes,
    //    expected_p99_bytes, domain_attrs.trust_class}`
    //    (`reassembly/max-fragments-insufficient-for-mtu`,
    //    `reassembly/untrusted-link-binding`, etc.) and the
    //    `reassembly/per-peer-quota-build-invariant-violated`
    //    invariant (peer_table.capacity × per-peer-quota ≥
    //    slot_count, declared above) join the §synth-5-K `links:` block
    //    cross-doc family. Codegen-side per-slot
    //    bitmap/deadline/peer-id emission is guarded by the
    //    `reassembly/peer-id-not-zid-on-established-session`
    //    template-regression check. Listener-link sibling-split
    //    (`link/listener-link-not-paired-with-established-sibling` +
    //    `reassembly/binding-on-unpaired-listener`) is a §synth-5-C codegen
    //    contract (spec line 2820-2824). Backend coverage reuses
    //    `codegen/mcu-class-kind-on-non-mcu-language` per spec line 2664
    //    verbatim — no new MCU-class code minted here. ──
    /// `<sce:variant>reassembly</sce:variant>` declared on a buffer-pool
    /// without an accompanying `<sce:max-fragments-per-message>` sibling.
    /// RFC §synth-5-M line 2944 names this code. Spec line 2688 fixes the
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
    /// RFC §synth-5-M line 2945 names this code. Spec line 2689 + line 2696
    /// fix the per-slot deadline field to this value; without it the
    /// reassembly FSM has no `Receiving → TimedOut` edge timer
    /// (`docs/reassembly-fsm.md` §2.4.5). The single recoverable
    /// repair is to add the missing element with a concrete millisecond
    /// value — author-domain knowledge (depends on link latency budget
    /// and acceptable hold time), so the non-overlap class is
    /// `NeutralOrDeterministic` (no closed candidate set).
    #[serde(rename = "mem/reassembly-pool-variant-missing-timeout")]
    MemReassemblyPoolVariantMissingTimeout,

    // ── §synth-5-M Fragment-reassembly cross-doc validators (items C9 +
    //    C13, SCE Protocol-Synthesis RFC §synth-5-M lines 2946-2995).
    //    Each fires from a cross-doc resolver that walks
    //    `deploy.links.<X>` → forge `<sce:link name=X>` → its
    //    `<sce:rx-pool ref=Y>` → `ForgePoolRegistry`'s BufferPoolModel
    //    for Y. Silent-skip on any join-step failure per the
    //    deploy-unaware silent-skip discipline. All six ride
    //    NeutralOrDeterministic — every
    //    code has multi-axis repair paths (raise slot_size, lower
    //    expected_p99, change max-fragments-per-message, lower
    //    mtu_bytes, change trust_class, raise worker_slot_budget_us)
    //    that are author-domain decisions rather than closed
    //    candidate sets. ──
    /// `<sce:rx-pool ref>` bound to a link whose `mtu_bytes` exceeds
    /// the pool's `<sce:slot-size>`. RFC §synth-5-M line 2946 names this
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
    /// `mtu_bytes`. RFC §synth-5-M line 2947-2949 verbatim: `slot_size <
    /// max-fragments-per-message × mtu_bytes`. Hard error — worst-case
    /// message cannot complete reassembly within declared bounds.
    /// Repair: raise `<sce:slot-size>`, lower
    /// `<sce:max-fragments-per-message>`, or lower link `mtu_bytes`.
    #[serde(rename = "reassembly/max-fragments-insufficient-for-mtu")]
    ReassemblyMaxFragmentsInsufficientForMtu,

    /// Build-time stage-copy rate gate. RFC §synth-5-M line 2950-2952
    /// verbatim: `(expected_p99_bytes - rx_pool.slot_size) /
    /// expected_p99_bytes > 0.25`. The 25% threshold is the spec's
    /// default warning point — beyond it, the link runs the
    /// ARCHITECTURE §9.3 stage-copy path on >¼ of inbound traffic.
    /// Warning, not hard error; suppressible via
    /// `<sce:accept-stage-copy-rate>` on the link source; promotable
    /// to `pool/stage-copy-policy-error` via §synth-5-K
    /// `pool_defaults.stage_copy_policy: error`. Silent-skip when no
    /// regular RX pool is bound (the "regular RX pool" the formula
    /// references doesn't exist).
    #[serde(rename = "reassembly/expected-fragmentation-rate-high")]
    ReassemblyExpectedFragmentationRateHigh,

    /// Reassembly pool bound to a link whose
    /// `domain_attrs.trust_class` is `untrusted` or `session_arming`.
    /// RFC §synth-5-M line 2964-2969 verbatim: hard error. Fragmentation
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
    /// undeclared). RFC §synth-5-M line 2970-2975 verbatim: hard error;
    /// build cannot decide whether the binding is safe. Absence of
    /// the `domain_attrs` block is
    /// the trigger — when `domain_attrs` is declared without
    /// `trust_class`, the deploy parser rejection
    /// (`LinkDomainAttrs.trust_class` is required-when-block-declared)
    /// catches it earlier. Repair: declare `trust_class:
    /// established_session` for data-plane links, or remove the
    /// reassembly-pool binding for control-plane links.
    #[serde(rename = "reassembly/trust-class-missing-on-fragmenting-link")]
    ReassemblyTrustClassMissingOnFragmentingLink,

    /// Stage-copy WCET vs cooperative slot budget. RFC §synth-5-M line
    /// 2995-2999 verbatim: `expected_p99_bytes ×
    /// memcpy_cycles_per_byte / clock_freq_mhz >
    /// worker_slot_budget_us`. When triggered, the implicit memcpy in
    /// the stage-copy path alone blows the cooperative slot, starving
    /// Keepalive and other parallel-region timers (ARCHITECTURE §9.3 +
    /// §3.4). Silent-skip when any of the four platform/scheduler
    /// inputs are absent (deploy-unaware silent-skip precedent). Repair: raise
    /// `worker_slot_budget_us`, lower `expected_p99_bytes` so stage
    /// copy is never invoked at that size, or raise the bound pool's
    /// `<sce:slot-size>` to absorb p99 without invoking stage copy.
    #[serde(rename = "reassembly/stage-copy-wcet-exceeds-slot-budget")]
    ReassemblyStageCopyWcetExceedsSlotBudget,

    /// Codegen self-check: the emitted reassembly-variant pool's
    /// per-slot peer-id is not the 16-byte ZID signature mandated for
    /// `trust_class: established_session` bindings. RFC §synth-5-M line
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
    /// `established_session` Sibling half. SCE Protocol-Synthesis RFC §synth-5-C
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
    /// NeutralOrDeterministic — pure template-regression
    /// guard with no closed candidate set.
    #[serde(rename = "link/listener-link-not-paired-with-established-sibling")]
    LinkListenerLinkNotPairedWithEstablishedSibling,

    /// Author-facing hard error: a reassembly-pool binding has
    /// resolved to a `session_arming` link instance whose paired
    /// `established_session` sibling does not exist. SCE Protocol-Synthesis
    /// RFC §synth-5-M lines 2982-2994 verbatim: "a reassembly-pool binding
    /// has resolved to a `session_arming` link instance whose paired
    /// `established_session` sibling does not exist. Hard error. In
    /// well-formed codegen this is unreachable (the listener-link
    /// sibling emission contract in §synth-5-C guarantees pairing); the
    /// diagnostic guards SCXML that explicitly targets the
    /// `session_arming` half (bypassing the auto-resolution) and any
    /// future schema evolution that introduces non-listener
    /// `session_arming` instances. Distinct from
    /// `reassembly/untrusted-link-binding` (which rejects bindings to
    /// `untrusted` and to standalone `session_arming` non-listeners)
    /// and from `link/listener-link-not-paired-with-established-
    /// sibling` (which is the §synth-5-C link-side codegen self-check)."
    ///
    /// Wired into
    /// [`crate::mesh::deploy::validate_reassembly_cross_doc`]: when
    /// the bound link's resolved
    /// `trust_class` is `session_arming` AND the orchestrator-resolved
    /// listener-link set does NOT contain the link name (i.e. no
    /// `Accepting.*` substate on the machine's source SCXML), the
    /// validator fires this code in place of the historic
    /// `reassembly/untrusted-link-binding` for the session-arming
    /// subcase. NeutralOrDeterministic — two valid repair
    /// paths: add an `Accepting.*` substate to the machine's source
    /// SCXML (making the link a real listener so the sibling
    /// auto-synthesizes), or remove the reassembly-pool binding.
    #[serde(rename = "reassembly/binding-on-unpaired-listener")]
    MeshDeployReassemblyBindingOnUnpairedListener,

    /// SCE Protocol-Synthesis RFC §synth-5-N line 3060 verbatim
    /// (`link/concurrent-count-exceeds-scheduler-slots`) — MCU-only
    /// cooperative-scheduler accounting: more links than the
    /// scheduler can accommodate within one tick. Hard error.
    ///
    /// Slot ceiling derivation: `floor(tick_period_us
    /// / per_link_budget_us)` mirrors the
    /// `validate_machine_scheduler_worker_capacity` precedent at
    /// mesh/deploy.rs `worker_slot_budget_us`. Validator silent-
    /// skips when `platform.class != mcu`, `scheduler.kind !=
    /// cooperative`, `tick_period_us` absent, or `per_link_budget_us`
    /// absent (deploy-unaware silent-skip precedent). Repair: raise
    /// `per_link_budget_us`, lower `tick_period_us`, or remove a
    /// link declaration from `machines.<m>.links`.
    #[serde(rename = "link/concurrent-count-exceeds-scheduler-slots")]
    LinkConcurrentCountExceedsSchedulerSlots,

    /// SCE Protocol-Synthesis RFC §synth-5-N line 3061 verbatim
    /// (`link/per-link-budget-exceeds-tick-period`). Per-link budget
    /// must fit inside one cooperative tick:
    /// `per_link_budget_us > tick_period_us` is the single-link
    /// sanity check (the diagnostic name is read literally). Hard
    /// error. NeutralOrDeterministic — two-axis repair (lower
    /// `per_link_budget_us` or raise `tick_period_us`). Validator
    /// silent-skips when either input absent or scheduler is not
    /// cooperative.
    #[serde(rename = "link/per-link-budget-exceeds-tick-period")]
    LinkPerLinkBudgetExceedsTickPeriod,

    /// SCE Protocol-Synthesis RFC §synth-5-N line 3062 verbatim
    /// (`link/inbound-event-queue-unsized`). A `<sce:link>` declares
    /// at least one `<sce:inbound>` event but the downstream FSM's
    /// event queue depth is undeclared. Hard error.
    ///
    /// Two acceptable size sources: SCXML
    /// per-instance `<scxml sce:capacity="N">` (preferred — pins
    /// the FSM-side spsc capacity to the machine's actual event
    /// volume) or deploy
    /// `machines.<m>.scheduler.default_event_queue_capacity`
    /// (fallback — single default applied to every undeclared
    /// machine on the deploy). Validator extends
    /// `compile_scxml_with_imports` pass-2 (the orchestrator-level
    /// cross-doc precedent). Silent-skip when the link has
    /// no inbound events declared or when no SCXML imports the link
    /// (no FSM downstream to size). NeutralOrDeterministic —
    /// two-axis repair (per-instance vs per-machine source).
    #[serde(rename = "link/inbound-event-queue-unsized")]
    LinkInboundEventQueueUnsized,

    // ── §synth-5-L Bounded-collection kind diagnostics (SCE Protocol-Synthesis RFC
    //    §synth-5-L lines 2540-2655, item C6). Two structure-only parse-time
    //    codes, three cross-doc codes (element-type-not-a-kind /
    //    index-by-field-missing / multi-writer-without-atomics), and
    //    one deploy-time code (capacity-unresolved). ──
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
    /// restricts element types to these two kinds. The cross-doc
    /// validator consumes the orchestrator-assembled element-type
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
    /// without any §synth-5-I atomic intrinsic having been imported via
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
    /// (deploy-unaware silent-skip precedent). Closed candidate set (sorted declared
    /// limit names under target_machine.limits) rides
    /// `Fix::ReplaceOneOf` ⇒ FixCarriesCandidates non_overlap_class.
    #[serde(rename = "collection/capacity-unresolved")]
    CollectionCapacityUnresolved,

    // ── §synth-5-D Timer kind diagnostics (SCE Protocol-Synthesis RFC §synth-5-D
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
    /// `timer_wheel_depth` is absent (deploy-
    /// unaware paths don't have the wheel sizing information).
    #[serde(rename = "timer/slot-overflow")]
    TimerSlotOverflow,

    // ── §synth-5-I `<sce:extern>` whitelisted intrinsic registry
    //    (SCE Protocol-Synthesis RFC §synth-5-I, item C4). Four spec-verbatim
    //    codes (lines 1847-1850) that fire at parse-time on
    //    `<sce:extern>` declarations. The 101-symbol baseline lives
    //    in `crate::forge::intrinsic_registry::BASELINE_SYMBOLS`;
    //    closed-set lookup is in `extern_validator::validate_extern`.
    //    Diagnostic names are spec verbatim per
    //    `feedback_spec_mirror_parity.md`. The target-plugin
    //    extension axis (`extern/target-plugin-symbol-conflict`) is
    //    declared below; further plugin axes (`extern/linker-flavor-*`)
    //    stay unimplemented until a consumer needs them. ──
    /// `<sce:extern name>` references a symbol absent from the
    /// §synth-5-I baseline registry. Repair surface = `Fix::ReplaceOneOf`
    /// over closest baseline names (bounded top-8). Parse-time;
    /// mirrors the `LinkLinkClassUnknown` closed-enum precedent.
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
    /// declares a symbol whose `name` already exists in the §synth-5-I
    /// baseline registry. Spec line 1852 verbatim semantic: "target
    /// plugin redefines a core whitelist symbol". Additive-
    /// composition rule — plugin entries extend, never
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
    #[serde(rename = "cli/invalid-suite-package")]
    CliInvalidSuitePackage,
    /// The generator binary was not built from the sources it is being
    /// run against.
    ///
    /// Nothing in the C++ build graph produces `sce-codegen`; CMake finds
    /// whatever binary is in `target/` and uses it, and `target/` does not
    /// travel to a build machine. So a tree whose sources are current can
    /// generate with a binary that is arbitrarily old, and the acceptance
    /// suites cannot see it because they run *against* the generator.
    ///
    /// `expected` carries the digest of the tree, `actual` the digest the
    /// binary was stamped with. Repair is a rebuild, not an edit — see
    /// [`crate::generator_witness`].
    #[serde(rename = "cli/generator-source-drift")]
    CliGeneratorSourceDrift,
    /// Freshness could not be established, which is not the same claim as
    /// drift and must not be reported as one.
    ///
    /// The binary carries [`crate::generator_witness::DIGEST_UNAVAILABLE`]
    /// because the build that produced it could not read the witness set —
    /// a vendored crate or a release tarball with no workspace
    /// `Cargo.lock` beside it. Reporting that as drift would blame the
    /// tree for the tool's own missing input, which is the misattribution
    /// `sce_gate_cannot_run` exists to prevent elsewhere.
    #[serde(rename = "cli/generator-source-unverifiable")]
    CliGeneratorSourceUnverifiable,
    /// The invocation itself did not parse: a required argument is
    /// absent, a flag is unknown, a value is outside its enumeration,
    /// or two mutually-exclusive modes were named together.
    ///
    /// Exists because the argument parser's own failure path is still a
    /// failure of this process, and [§6](SCE_ERROR_CONTRACT.md) makes a
    /// non-zero exit carrying no record a contract violation. Without
    /// this code the most common failure a machine caller produces —
    /// a malformed command line — arrived as prose under the exit
    /// status reserved for `xml/*`, which tells a repair consumer that
    /// the *document* is broken.
    #[serde(rename = "cli/usage")]
    CliUsage,
    /// A well-formed query that matched nothing: no such symbol in the
    /// sourcemap, no symbol at the requested coordinates, no frame that
    /// resolved.
    ///
    /// Distinct from [`DiagnosticCode::CliReadInput`] because nothing
    /// failed — the tool ran, looked, and found no answer. It is the
    /// one CLI code that does not exit 20: the query tools document a
    /// dedicated status for a miss so a build gate can branch on it
    /// without a JSON parser, and §6 registers that status.
    #[serde(rename = "cli/query-no-match")]
    CliQueryNoMatch,

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
    /// A machine's `dedup:` section declares a window that cannot filter
    /// (§mesh-10.5). Parse-time reject; no closed candidate set.
    #[serde(rename = "mesh/deploy-invalid-dedup-window")]
    MeshDeployInvalidDedupWindow,
    /// A device's `transports.custom_tcp:` socket-layer field declares a
    /// value that would disable what it configures (§mesh-16.8.3).
    /// Parse-time reject; the legal set is open, so no candidates.
    #[serde(rename = "mesh/deploy-invalid-custom-tcp-socket")]
    MeshDeployInvalidCustomTcpSocket,
    /// A device's `transports.dds.qos:` overlay declares a DDS default
    /// (§mesh-8.2). Parse-time reject; the legal set is open.
    #[serde(rename = "mesh/deploy-invalid-dds-qos")]
    MeshDeployInvalidDdsQos,
    #[serde(rename = "mesh/deploy-invalid-liveliness")]
    MeshDeployInvalidLiveliness,
    #[serde(rename = "mesh/deploy-invalid-server-response-deadline")]
    MeshDeployInvalidServerResponseDeadline,
    #[serde(rename = "mesh/deploy-invalid-outbound-buffer")]
    MeshDeployInvalidOutboundBuffer,
    #[serde(rename = "mesh/deploy-invalid-retry-policy")]
    MeshDeployInvalidRetryPolicy,
    #[serde(rename = "mesh/deploy-invalid-auth-policy")]
    MeshDeployInvalidAuthPolicy,
    #[serde(rename = "mesh/deploy-discovery-not-supported")]
    MeshDeployDiscoveryNotSupported,
    #[serde(rename = "mesh/deploy-pool-not-supported-by-transport")]
    MeshDeployPoolNotSupportedByTransport,
    #[serde(rename = "mesh/deploy-pool-missing-member-list")]
    MeshDeployPoolMissingMemberList,
    #[serde(rename = "mesh/deploy-pool-empty-member-list")]
    MeshDeployPoolEmptyMemberList,
    #[serde(rename = "mesh/deploy-pool-binding-field-not-supported")]
    MeshDeployPoolBindingFieldNotSupported,
    #[serde(rename = "mesh/deploy-pool-dispatch-without-member")]
    MeshDeployPoolDispatchWithoutMember,
    #[serde(rename = "mesh/deploy-pool-invalid-placeholder")]
    MeshDeployPoolInvalidPlaceholder,
    #[serde(rename = "mesh/deploy-server-pool-not-supported")]
    MeshDeployServerPoolNotSupported,
    #[serde(rename = "mesh/deploy-cross-target-reply-not-supported")]
    MeshDeployCrossTargetReplyNotSupported,
    #[serde(rename = "mesh/deploy-invalid-reply-from")]
    MeshDeployInvalidReplyFrom,
    /// A binding key that neither the typed schema nor the bound
    /// transport reads. Parse-time typo guard; FixCarriesCandidates
    /// over the transport's legal binding-key set, closest-first.
    #[serde(rename = "mesh/deploy-unknown-binding-field")]
    MeshDeployUnknownBindingField,
    // ── SCE Protocol-Synthesis RFC §synth-5-E sample-callback deploy.yaml stage_pool family ──
    // These diagnostics validate the deploy.yaml `binding.stage_pool:`
    // cross-reference into the forge buffer-pool registry. Distinct
    // family from the `mesh/deploy-pool-*` SOME/IP-instance-routing
    // diagnostics above: those concern routing-pool placeholders for
    // RPC bindings, while these concern buffer-pool kind references
    // for `Sample::take()` stage copies (§synth-5-E Sample API contract).
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
    // §14 per-machine platform/scheduler schema (RFC §synth-5-K, item A2)
    #[serde(rename = "mesh/deploy-platform-class-os-mismatch")]
    MeshDeployPlatformClassOsMismatch,
    /// SCE Protocol-Synthesis RFC §synth-5-K line 2426 verbatim
    /// (`deploy/worker-stack-budget-missing`). Cooperative scheduler
    /// declared without `worker_stack_budget`. Renamed from the
    /// SCE-Mesh-prefix wire (`mesh/deploy-scheduler-cooperative-missing-stack-budget`)
    /// to the deploy-prefix wire; SCE Mesh §14
    /// continues to anchor to the same variant.
    #[serde(rename = "deploy/worker-stack-budget-missing")]
    MeshDeploySchedulerCooperativeMissingStackBudget,
    /// SCE Protocol-Synthesis RFC §synth-5-K line 2428-2429 verbatim
    /// (`deploy/worker-slot-budget-missing`). Cooperative scheduler
    /// declared without `worker_slot_budget_us`. Validator
    /// [`crate::mesh::deploy::validate_worker_slot_budget_required_when_cooperative`]
    /// fires at deploy.yaml parse time.
    #[serde(rename = "deploy/worker-slot-budget-missing")]
    MeshDeploySchedulerCooperativeMissingSlotBudget,
    /// SCE Protocol-Synthesis RFC §synth-5-K line 2430-2431 verbatim
    /// (`deploy/keepalive-jitter-budget-missing`). Cooperative
    /// scheduler declared without `keepalive_jitter_budget_us`.
    /// Validator
    /// [`crate::mesh::deploy::validate_keepalive_jitter_required_when_cooperative`]
    /// fires at deploy.yaml parse time.
    #[serde(rename = "deploy/keepalive-jitter-budget-missing")]
    MeshDeploySchedulerCooperativeMissingKeepaliveJitterBudget,
    /// SCE Protocol-Synthesis RFC §synth-5-K line 2423 verbatim
    /// (`deploy/scheduler-incompatible-with-worker-count`). Cooperative
    /// scheduler's derived slot count (`floor(tick_period_us /
    /// worker_slot_budget_us)`) is less than the number of workers
    /// declared under `machines.<m>.workers`. Validator
    /// [`crate::mesh::deploy::validate_machine_scheduler_worker_capacity`]
    /// fires at deploy.yaml parse time after the three "*-missing"
    /// validators have confirmed the budget fields are present. The
    /// forge-side anchor for the same axis is [`WorkerSchedulerUnsupported`]
    /// (spec §synth-5-D line 912).
    #[serde(rename = "deploy/scheduler-incompatible-with-worker-count")]
    MeshDeploySchedulerIncompatibleWithWorkerCount,

    // ── §synth-5-K `links:` block parse-time + cross-doc validators
    //    (SCE Protocol-Synthesis RFC §synth-5-K lines 2232-2540, item C13). 9 codes:
    //    7 spec-named (`link-driver-unknown` at line 2421, `link-mtu-*`
    //    at lines 2440-2448, `link-burst-*` at lines 2489-2503) plus 2
    //    cross-doc joins that pair forge `<sce:link name>`
    //    documents against deploy.yaml `machines.<n>.links.<name>`
    //    entries. The anti-flood / stateless_accept codes and the 6
    //    reassembly cross-doc codes (`mem/reassembly-slot-size-
    //    below-declared-mtu`, `reassembly/{max-fragments-insufficient-
    //    for-mtu, expected-fragmentation-rate-high, untrusted-link-
    //    binding, trust-class-missing-on-fragmenting-link, stage-copy-
    //    wcet-exceeds-slot-budget}`) are declared in their own
    //    sections. ──
    /// SCE Protocol-Synthesis RFC §synth-5-K line 2421 verbatim
    /// (`deploy/link-driver-unknown`). `machines.<n>.links.<name>.driver`
    /// value is not in the known-driver baseline (currently
    /// `{lwip_udp, lwip_tcp}`; extends as new forge link-kind docs
    /// ship) AND not declared as a forge `<sce:link>` document name in
    /// the build's [`crate::forge::cross_doc_registry::SceCrossDocRegistry`].
    /// `Fix::ReplaceOneOf` over the known-driver + forge-link-doc-name
    /// union, sorted. Enum-shape kept as `String` so
    /// forge-side driver authoring extends organically; closed-allowlist
    /// validator is the gate, not the type system.
    #[serde(rename = "deploy/link-driver-unknown")]
    MeshDeployLinkDriverUnknown,

    /// SCE Protocol-Synthesis RFC §synth-5-K line 2440-2442 verbatim
    /// (`deploy/link-mtu-missing-on-fragmenting-link`). A
    /// `machines.<n>.links.<name>` entry is bound to a forge
    /// `<sce:link name="...">` whose FSM emits/consumes Fragment codec
    /// events, but `mtu_bytes` is absent. Without it the build cannot
    /// size reassembly pool slots per §synth-5-M. This parse-time detector
    /// uses a conservative under-approximation
    /// — it fires for every `domain_attrs.trust_class:
    /// established_session` link missing `mtu_bytes`, since
    /// `established_session` is the only trust class permitted to carry
    /// Fragment traffic per RFC §synth-5-M line 2731. The §synth-5-M
    /// reassembly-pool-bound-to-link cross-doc validator performs the
    /// precise Fragment-FSM detection; the parse-time
    /// under-approximation surfaces the same author error class earlier.
    #[serde(rename = "deploy/link-mtu-missing-on-fragmenting-link")]
    MeshDeployLinkMtuMissingOnFragmentingLink,

    /// SCE Protocol-Synthesis RFC §synth-5-K line 2443-2445 verbatim
    /// (`deploy/link-mtu-below-driver-floor`). `mtu_bytes` declared
    /// smaller than the driver's minimum payload (e.g. UDP/IPv6's
    /// 56-byte floor); driver default would override silently. The
    /// known-driver baseline carries each driver's floor (currently
    /// `{lwip_udp: 28, lwip_tcp: 40}` from IPv4 minimum-header
    /// arithmetic); unknown drivers fall back to silent-skip until
    /// their floor is registered.
    #[serde(rename = "deploy/link-mtu-below-driver-floor")]
    MeshDeployLinkMtuBelowDriverFloor,

    /// SCE Protocol-Synthesis RFC §synth-5-C lines 765-771 + §synth-8 Q8 line 3747
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
    /// `LinkClassUnsupportedOnTarget` precedent at §synth-5-C uses the
    /// same single-axis Fix discipline).
    #[serde(rename = "deploy/link-driver-class-mismatch")]
    MeshDeployLinkDriverClassMismatch,

    /// SCE Protocol-Synthesis RFC §synth-5-K line 2446-2448 verbatim
    /// (`deploy/link-expected-p99-exceeds-mtu`). `expected_p99_bytes >
    /// mtu_bytes` AND no reassembly pool is bound to the link (this
    /// parse-time detector fires the warning whenever
    /// `expected_p99_bytes > mtu_bytes` regardless of pool binding,
    /// matching the spec's "the p99 message would always fragment but
    /// no reassembly path exists" intent — authors with a reassembly
    /// pool already see the `reassembly/expected-fragmentation-
    /// rate-high` consumer, the two diagnostics complement).
    #[serde(rename = "deploy/link-expected-p99-exceeds-mtu")]
    MeshDeployLinkExpectedP99ExceedsMtu,

    /// SCE Protocol-Synthesis RFC §synth-5-K line 2489-2495 verbatim
    /// (`deploy/link-burst-absorption-insufficient`). `burst_pps × 1s`
    /// of worst-case inbound exceeds the RX pool's drain rate within
    /// one cooperative tick window: `slot_count × ticks_per_second /
    /// burst_pps < 1.0` with safety factor 2.0 (i.e. the check fires
    /// when `slot_count × 1_000_000 / tick_period_us < burst_pps × 2`).
    /// Pool will deplete during burst and drop packets. Fires
    /// from a cross-doc resolver that joins `deploy.links.<X>` → forge
    /// `<sce:link name=X>` → `<sce:rx-pool ref>` → ForgePoolRegistry's
    /// `BufferPoolModel.slot_count`. Silent-skip on any join failure
    /// or missing scheduler.tick_period_us. Multi-axis
    /// repair: raise `<sce:slot-count>` on the pool, lower
    /// `scheduler.tick_period_us`, or switch `rx_dispatch` to
    /// `isr_to_pool` when currently `worker_tick`.
    #[serde(rename = "deploy/link-burst-absorption-insufficient")]
    MeshDeployLinkBurstAbsorptionInsufficient,

    /// SCE Protocol-Synthesis RFC §synth-5-K line 2496-2500 verbatim
    /// (`deploy/link-rx-dispatch-worker-tick-on-high-burst`).
    /// `rx_dispatch: worker_tick` declared but `burst_pps ×
    /// tick_period_us / 1_000_000 > slot_count` (one tick window of
    /// arrivals overruns the pool). Hard error unless author justifies
    /// via `<sce:accept-burst-drop-rate>` on the link source (a
    /// forge-side opt-out that stays unimplemented until a consumer
    /// needs it; the detector treats the opt-out as absent). The
    /// detector silent-skips when join steps or `tick_period_us` are missing.
    /// Multi-axis repair: switch `rx_dispatch` to `isr_to_pool`, raise
    /// `<sce:slot-count>` to absorb the per-tick burst, or lower
    /// `tick_period_us` so each window admits fewer arrivals.
    #[serde(rename = "deploy/link-rx-dispatch-worker-tick-on-high-burst")]
    MeshDeployLinkRxDispatchWorkerTickOnHighBurst,

    // ── §synth-5-K pool_defaults.stage_copy_policy (RFC §synth-5-K lines
    //    2350-2369 + 2504-2519, item C13). Three codes wire the policy enum
    //    plus the per-link `<sce:accept-stage-copy-rate>` opt-out
    //    semantics. ──
    /// SCE Protocol-Synthesis RFC §synth-5-K line 2504-2511 verbatim
    /// (`pool/stage-copy-policy-error`). `pool_defaults.stage_copy_policy:
    /// error` (or `forbid`) AND the §synth-5-M / ARCHITECTURE §9.3
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

    /// SCE Protocol-Synthesis RFC §synth-5-K line 2512-2516 verbatim
    /// (`pool/stage-copy-accept-rejected-under-forbid`).
    /// `pool_defaults.stage_copy_policy: forbid` AND a link source
    /// carries `<sce:accept-stage-copy-rate>`. The opt-out is rejected
    /// outright; only structural fixes (raise `<sce:slot-size>` or
    /// lower `expected_p99_bytes`) are accepted under `forbid`.
    /// NeutralOrDeterministic — two valid repair paths (remove the
    /// opt-out vs change policy to `error`).
    #[serde(rename = "pool/stage-copy-accept-rejected-under-forbid")]
    PoolStageCopyAcceptRejectedUnderForbid,

    /// SCE Protocol-Synthesis RFC §synth-5-K line 2517-2519 verbatim
    /// (`deploy/stage-copy-policy-unknown`).
    /// `pool_defaults.stage_copy_policy` declared with a value other
    /// than `warn` / `error` / `forbid`. Hard error (typo guard).
    /// FixCarriesCandidates over the closed set
    /// [`crate::mesh::deploy::StageCopyPolicy::ALL`].
    #[serde(rename = "deploy/stage-copy-policy-unknown")]
    MeshDeployStageCopyPolicyUnknown,

    // ── §synth-5-K anti-flood + stateless_accept (RFC §synth-5-K lines
    //    2272-2349 + 2449-2473, item C13). Five codes wire the
    //    conditional requirement, dead-config rejection, opt-out
    //    requirement, and key-rotation invariant, plus two siblings:
    //      - `deploy/session-arming-quota-vs-peer-table-invariant-violated`
    //        (line 2460-2462) — rides the `peer_table` +
    //        `max_handshake_time_s` schema fields on the
    //        `stateless_accept` block (validator wired into
    //        `validate_links` per per-link invariant scope).
    //      - `deploy/stateless-accept-extern-not-whitelisted`
    //        (line 2466-2469) — lives at the orchestrator level
    //        where the §synth-5-I baseline + loaded `target_plugin`
    //        symbols converge, mirroring the
    //        `extern/target-plugin-symbol-conflict` precedent.
    /// SCE Protocol-Synthesis RFC §synth-5-K line 2449-2451 verbatim
    /// (`deploy/session-arming-quota-missing`). Link declares
    /// `trust_class: session_arming` but no `session_arming_quota`.
    /// Hard error; without a cap an attacker can fill every
    /// `Accepting.*` slot. Repair: declare a concrete u32 value
    /// (MCU default 8, AP default 32 per spec line 2282).
    /// NeutralOrDeterministic — author-domain value, no closed
    /// candidate set.
    #[serde(rename = "deploy/session-arming-quota-missing")]
    MeshDeploySessionArmingQuotaMissing,

    /// SCE Protocol-Synthesis RFC §synth-5-K line 2452-2453 verbatim
    /// (`deploy/accept-rate-config-missing`). `trust_class:
    /// session_arming` link missing `accept_rate_per_sec` or
    /// `accept_rate_burst`. Hard error.
    /// NeutralOrDeterministic — author-domain values.
    #[serde(rename = "deploy/accept-rate-config-missing")]
    MeshDeployAcceptRateConfigMissing,

    /// SCE Protocol-Synthesis RFC §synth-5-K line 2454-2459 verbatim
    /// (`deploy/session-arming-fields-on-non-arming-link`). Anti-
    /// flood / stateless_accept fields declared on a `trust_class:
    /// untrusted` or `established_session` link where `Accepting.*`
    /// is never instantiated. Dead config; suggests author confusion
    /// about which link is the listener. Hard error.
    /// NeutralOrDeterministic — two valid repair paths (change
    /// trust_class to session_arming vs remove the dead fields).
    #[serde(rename = "deploy/session-arming-fields-on-non-arming-link")]
    MeshDeploySessionArmingFieldsOnNonArmingLink,

    /// SCE Protocol-Synthesis RFC §synth-5-K line 2463-2465 verbatim
    /// (`deploy/stateless-accept-required-on-untrusted-source`).
    /// Link with `domain_attrs.untrusted_source: true` but no
    /// `stateless_accept` block. Hard error.
    /// NeutralOrDeterministic — author must author the full block.
    #[serde(rename = "deploy/stateless-accept-required-on-untrusted-source")]
    MeshDeployStatelessAcceptRequiredOnUntrustedSource,

    /// SCE Protocol-Synthesis RFC §synth-5-K line 2470-2473 verbatim
    /// (`deploy/stateless-accept-key-rotation-shorter-than-lifetime`).
    /// `key_rotation_s × 1000 ≤ 2 × cookie_lifetime_ms`. The
    /// previous-key honor window cannot bridge a rotation, so
    /// handshakes near rotation boundaries get spurious cookie
    /// rejection. Hard error.
    /// NeutralOrDeterministic — two-axis repair (raise key_rotation_s
    /// vs lower cookie_lifetime_ms).
    #[serde(rename = "deploy/stateless-accept-key-rotation-shorter-than-lifetime")]
    MeshDeployStatelessAcceptKeyRotationShorterThanLifetime,

    /// SCE Protocol-Synthesis RFC §synth-5-K line 2460-2462 verbatim
    /// (`deploy/session-arming-quota-vs-peer-table-invariant-violated`).
    /// `session_arming_quota × max_handshake_time_s > peer_table.capacity`.
    /// A slow legitimate handshake can be evicted under attack when
    /// an attacker churns the quota faster than the per-peer table
    /// absorbs. Hard error.
    /// NeutralOrDeterministic — three-axis repair (raise
    /// `peer_table.capacity`, lower `session_arming_quota`, or lower
    /// `max_handshake_time_s`); the wire payload carries the
    /// violating product in `actual` and the bound in `expected`.
    /// The consuming validator lives inside
    /// `validate_links` alongside the other anti-flood checks.
    #[serde(rename = "deploy/session-arming-quota-vs-peer-table-invariant-violated")]
    MeshDeploySessionArmingQuotaVsPeerTableInvariantViolated,

    /// SCE Protocol-Synthesis RFC §synth-5-K line 2466-2469 verbatim
    /// (`deploy/stateless-accept-extern-not-whitelisted`).
    /// `hmac_extern` or `rng_extern` symbol not present in the
    /// `sce_intrinsics_runtime` core whitelist AND not declared in
    /// any loaded `target_plugin`. Hard error.
    /// FixCarriesCandidates — closed-set candidates is the sorted
    /// union of §synth-5-I baseline names + target-plugin-loaded names
    /// (`Fix::ReplaceOneOf`). The consuming
    /// validator lives at the orchestrator level
    /// (`compile_scxml_with_imports` + `compile_forge_with_deploy`)
    /// because target-plugin loading is deploy-driven, mirroring
    /// the `extern/target-plugin-symbol-conflict` precedent.
    #[serde(rename = "deploy/stateless-accept-extern-not-whitelisted")]
    MeshDeployStatelessAcceptExternNotWhitelisted,

    /// SCE Protocol-Synthesis RFC §synth-5-K line 2501-2503 verbatim
    /// (`deploy/link-burst-pps-missing-on-isr-dispatch`). The resolved
    /// `rx_dispatch` (per [`super::super::mesh::deploy::LinkConfig::resolved_rx_dispatch`])
    /// is `IsrToPool` but `burst_pps` is not declared. ISR fast-path
    /// requires the rate to size descriptor ring + validate stack
    /// budget. Repair: declare `burst_pps`, or set `rx_dispatch:
    /// worker_tick` explicitly. The detector fires at parse-time
    /// since the resolution is purely intra-link-config.
    #[serde(rename = "deploy/link-burst-pps-missing-on-isr-dispatch")]
    MeshDeployLinkBurstPpsMissingOnIsrDispatch,

    /// Cross-doc validator pair (item C13). A forge
    /// `<scxml sce:kind="link" name="X">` document was imported by
    /// some statechart/worker on this machine, but no
    /// `deploy.yaml::machines.<n>.links.<X>` entry exists. Build
    /// cannot resolve the link's `bind` address, `mtu_bytes`, or
    /// `domain_attrs`. `Fix::ReplaceOneOf` over the deploy-side
    /// link-name set for this machine (sorted). Mirrors the
    /// `validate_stage_pool_references` +
    /// `validate_worker_outbox_references` precedents.
    #[serde(rename = "deploy/link-not-declared-in-deploy")]
    MeshDeployLinkNotDeclaredInDeploy,

    /// Cross-doc validator pair (item C13). A
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

    // ── Forge generated-source drift detection (SCE Protocol-Synthesis RFC
    //    §synth-6.2.6, item B9). Single code covers both axes
    //    of `sce-codegen verify`'s recomputed-vs-embedded comparison:
    //    `source-hash` mismatch (input SCXML or deploy.yaml drifted
    //    since generation) and `template-hash` mismatch (codegen
    //    template tree or Cargo.lock drifted). The
    //    `actual` field carries the axis label (`source`|`template`)
    //    plus the embedded hex; `expected` carries the recomputed
    //    hex. Repair is `sce-codegen <regen-command>` (deterministic,
    //    no candidate set), hence NeutralOrDeterministic. ─────────
    #[serde(rename = "forge/source-hash-mismatch")]
    ForgeSourceHashMismatch,

    // ── §synth-6.2.6 source-set coverage guard. The `source-hash` is
    //    folded from every `**/*.scxml` under the input root, so a root
    //    that resolves to nothing still yields a well-formed sha256 —
    //    the empty-input digest — which is indistinguishable on the wire
    //    from a successful hash. Fired at emit time when the digest about
    //    to be embedded would not describe the input that produced it:
    //    an empty set always, and additionally a set that omits the input
    //    when the root was inferred from that input's own location.
    //    Repair is a single deterministic move: point `--input-root` at
    //    a directory that contains the input. ─────────────────────────
    #[serde(rename = "forge/source-hash-input-uncovered")]
    ForgeSourceHashInputUncovered,

    // ── §synth-6.2.6 source-set enumeration did not terminate within
    //    its descent ceiling. A directory symlink naming a sibling
    //    contributes under every name that reaches it, so nested levels
    //    of such links name a path count exponential in the depth.
    //    Refused rather than truncated: a digest folded over the prefix
    //    the walk happened to reach describes a subset of the input, and
    //    a header carrying one is unauditable in the same way the
    //    empty-input digest above was. Repair is the input layout —
    //    re-point `--input-root` below the aliasing, or remove it. ─────
    #[serde(rename = "forge/source-hash-walk-unbounded")]
    ForgeSourceHashWalkUnbounded,

    // ── Traceability §synth-5-O IR provenance pre-emit guard
    //    (SCE Protocol-Synthesis RFC §synth-5-O lines 3289-3290 verbatim:
    //    "XInclude / sce:template composition MUST track per-element
    //    (file_id, line, column) and attach it to every IR node.
    //    Codegen failure ... surfaced via
    //    `traceability/scxml-line-range-missing` (codegen-internal)").
    //    Fires at the compile-pipeline pre-emit pass when a node
    //    eligible for marker emission carries `source_location: None`
    //    — guarantees the per-backend SCE-MAP marker family
    //    (function-level markers + per-symbol attribution) is never
    //    silently dropped on an unpopulated IR record. Author repair
    //    is empty: this is a codegen-internal invariant, so authors
    //    never see it in practice; the diagnostic exists so a future
    //    parser edit that creates an IR node without populating
    //    `source_location` surfaces immediately rather than producing
    //    silently-broken codegen.
    #[serde(rename = "traceability/scxml-line-range-missing")]
    TraceabilityScxmlLineRangeMissing,

    // ── Traceability §synth-5-O symbol mangling collision
    //    detector. Spec lines 3055-3057 fix the per-symbol mangling
    //    pattern (`<machine>__<state_path>__<artifact>`); XInclude or
    //    `sce:template` composition can produce two distinct IR nodes
    //    whose triples mangle to the same C identifier. The dual-
    //    location payload pins both sites so authors can rename one
    //    of the two states to break the collision.
    #[serde(rename = "traceability/state-id-collision")]
    TraceabilityStateIdCollision,

    // ── Traceability §synth-5-O — mangled symbol exceeds the C99
    //    §5.2.4.1 external identifier limit (31 chars). Default
    //    rendering is warn; `platform.strict_c99_identifiers: true`
    //    in deploy.yaml escalates to hard-error. The diagnostic
    //    carries the offending mangled id + excess-char count so
    //    authors see exactly what overflowed.
    #[serde(rename = "traceability/symbol-name-exceeds-c-identifier-limit")]
    TraceabilitySymbolNameExceedsCIdentifierLimit,

    // ── Traceability §synth-5-O — sourcemap `source_hash` drift
    //    against the §synth-6.2.6 header. Spec lines 3321-3324 require
    //    byte-equality between the sourcemap JSON's `source_hash`
    //    field and the per-file `// source-hash:` header value;
    //    drift indicates the sourcemap was emitted from a stale
    //    snapshot or hand-edited.
    #[serde(rename = "traceability/sourcemap-source-hash-mismatch")]
    TraceabilitySourcemapSourceHashMismatch,

    // ── Traceability §synth-5-O — Rust SCE-MAP marker preservation
    //    guard (OQ-W16 (b)). Empirical: rustdoc may strip `#[doc]`
    //    attributes under specific profile / no_std combinations.
    //    Fires from `sce-codegen addr2sce` when the rustdoc JSON dump
    //    lacks the expected `SCE-MAP:` `#[doc]` line; the `// SCE-MAP:`
    //    line-comment fallback (the default dual-emit form) covers the
    //    miss. Diagnostic signals the fallback was needed.
    #[serde(rename = "traceability/sce-map-attribute-stripped")]
    TraceabilitySceMapAttributeStripped,

    // ── Traceability §synth-5-O — codegen-internal
    //    invariant: every SCE-emitted file (one carrying a §synth-6.2.6
    //    drift header) MUST contain at least one `SCE-MAP:` marker.
    //    Walker `forge::sourcemap::validate_emitted_files_have_markers`
    //    fires this from cmd_generate / cmd_generate_w3c success paths.
    //    Files without a drift header (external meta-generator output)
    //    are silently skipped per ARCHITECTURE.md "Traceability
    //    Ownership Boundary". Author repair is empty — the fix lives
    //    in the template that lost its `sce_map_marker` macro call.
    #[serde(rename = "traceability/meta-generated-source-line-marker-missing")]
    TraceabilityMetaGeneratedSourceLineMarkerMissing,

    // ── MCU driver/class boundary on the C11 backend (SCE Protocol-Synthesis
    //    RFC §5.2). The two codes split by repair surface:
    //    `mcu/driver-header-not-found` fires at compile-model time
    //    when `<sce:driver href="..."/>` cannot be resolved against
    //    `deploy.yaml`'s `platform.driver_root` (or the SCXML file's
    //    parent directory as fallback). Cross-TU signature checking
    //    is delegated to the C compiler; SCE only
    //    confirms the file exists.
    //    `mcu/section-attribute-on-non-mcu-target` fires at codegen
    //    entry when `platform.c11_section_attribute` is set but the
    //    target backend is not C11 — mirrors the
    //    `codegen/mcu-class-kind-on-non-mcu-language` non-MCU
    //    reject pattern so the section directive does
    //    not silently disappear on non-MCU compiles.
    #[serde(rename = "mcu/driver-header-not-found")]
    McuDriverHeaderNotFound,
    #[serde(rename = "mcu/section-attribute-on-non-mcu-target")]
    McuSectionAttributeOnNonMcuTarget,
    /// `platform.c11_section_attribute.class` names a section the C11
    /// emitter cannot place into a string literal — see
    /// [`GenerateError::McuSectionAttributeNameInvalid`].
    #[serde(rename = "mcu/section-attribute-name-invalid")]
    McuSectionAttributeNameInvalid,

    // ── NL→IR Item C1 Path A: Enum kind invariants ───
    /// Enum document declares no `<sce:variant>` children — see
    /// [`ValidationError::EnumNoVariants`].
    #[serde(rename = "validation/enum-no-variants")]
    ValidationEnumNoVariants,
    /// Two variants share an identifier — see
    /// [`ValidationError::EnumVariantDuplicateName`].
    #[serde(rename = "validation/enum-variant-duplicate-name")]
    ValidationEnumVariantDuplicateName,
    /// Two variants share an underlying integer value (Path A's
    /// bijectivity invariant) — see
    /// [`ValidationError::EnumVariantDuplicateValue`].
    #[serde(rename = "validation/enum-variant-duplicate-value")]
    ValidationEnumVariantDuplicateValue,
    /// A variant's `value` overflows the declared
    /// `sce:underlying-type` — see
    /// [`ValidationError::EnumVariantValueOverflowsUnderlying`].
    #[serde(rename = "validation/enum-variant-value-overflows-underlying")]
    ValidationEnumVariantValueOverflowsUnderlying,
    /// `sce:underlying-type` is not one of the supported unsigned
    /// integer carriers — see
    /// [`ValidationError::EnumUnsupportedUnderlyingType`].
    #[serde(rename = "validation/enum-unsupported-underlying-type")]
    ValidationEnumUnsupportedUnderlyingType,

    // ── NL→IR Item C1 Path A: EventSchema kind ───────
    //    Three new codes for the 18th Forge kind:
    //    `validation/event-schema-on-builtin-event` (DL-9'; rejects
    //    schema declarations against the W3C SCXML reserved event
    //    namespaces `error.*` / `done.invoke.*` / `done.state.*`),
    //    `validation/event-payload-field-unknown` (DL-4' send-side;
    //    rejects `<send>/<param name="F">` whose `F` is not declared
    //    on the imported EventSchema), and `mesh/event-schema-mismatch`
    //    (DL-7' cross-machine; rejects `<send target="#...">` whose
    //    sender and receiver disagree on the event's typed payload
    //    contract). Field-not-found + type-mismatch on the receive-
    //    side reuse the existing `validation/cross-kind-field-not-found`
    //    + `validation/cross-kind-type-mismatch` codes per Item 4
    //    reuse precedent — the type-mismatch code is also reused on
    //    the send-side per the same precedent. ────────────────────
    #[serde(rename = "validation/event-schema-on-builtin-event")]
    ValidationEventSchemaOnBuiltinEvent,
    #[serde(rename = "validation/event-payload-field-unknown")]
    ValidationEventPayloadFieldUnknown,
    // RFC `rfc-eventschema-bytes-guard.md` §bytesguard-3 B3 — an ordering operator
    // (`<`/`>`/`<=`/`>=`) applied to a bytes-typed `_event.data.<field>`
    // in a transition guard. Distinct operator-domain rule (no existing
    // code fits without semantic stretch), so one new variant per
    // `diagnostic_code_edit_checklist`.
    #[serde(rename = "validation/bytes-comparison-not-equality")]
    ValidationBytesComparisonNotEquality,
    #[serde(rename = "mesh/event-schema-mismatch")]
    MeshEventSchemaMismatch,
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
        XmlPreprocessorNotRun,
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
        ValidationNativeActionPlacement,
        ValidationNativeActionArgument,
        ValidationNativeActionSignatureConflict,
        ValidationMeshRpcReservedParam,
        ValidationMeshRpcMissingTarget,
        ValidationMeshRpcDuplicateTarget,
        ValidationRemovedAttribute,
        ValidationBytesMaxSizeViolation,
        // NL→IR Mapping Roadmap Item 1: sce:req traceability
        ValidationDuplicateRequirementId,
        // NL→IR Mapping Roadmap Item 5: sce:unresolved placeholder
        ValidationUnresolvedPlaceholder,
        // NL→IR Mapping Roadmap Item 2: cross-kind typed binding
        ValidationCrossKindFieldNotFound,
        ValidationCrossKindTypeMismatch,
        ValidationCrossKindCircularDependency,
        // Algorithm (SCE Protocol-Synthesis RFC §synth-5-A, item A3)
        AlgorithmLocalShadowsParam,
        AlgorithmLvalueUnsupported,
        AlgorithmReturnMissing,
        // Algorithm-over-BC dispatch (RFC §synth-5-A line 311 + §synth-5-L lines
        // 2611-2618 + 2642-2647, C7-lowering 2026-05-13)
        AlgorithmForeachSourceNotIterable,
        AlgorithmCallTargetUnknown,
        AlgorithmCallTargetMethodUnknown,
        AlgorithmBcMutationForbidden,
        AlgorithmForeachSourceBcWithBytesItemType,
        AlgorithmCallArgCountMismatch,
        // Byte-buffer-build (SCE_FORGE.md §4.12)
        AlgorithmAppendTargetNotBuffer,
        AlgorithmAppendTypeMismatch,
        // SCXML semantic (§wire-W5)
        ScxmlTopLevelScriptUnloaded,
        // §scxml-3.2 datamodel attribute + §scxml-B-1 Null data model
        ScxmlUnsupportedDatamodel,
        ScxmlNullDatamodelForbidsConstruct,
        // NL→IR Mapping Roadmap Item 3 — Statechart graph reachability
        ScxmlUnreachableState,
        ScxmlDeadTransition,
        // NL→IR Mapping Roadmap Item 3 — event-set exhaustiveness
        ScxmlNonExhaustiveEventHandling,
        ScxmlContradictoryUnhandledDeclaration,
        ScxmlStaleUnhandledDeclaration,
        // NL→IR Mapping Roadmap Item 3 — guard analysis
        ScxmlAlwaysFalseGuard,
        ScxmlShadowedTransition,
        ScxmlOnSampleInvalidParent,
        ScxmlOnSampleLinkDuplicateInState,
        ScxmlOnSampleEventNameConflict,
        ScxmlOnSampleLinkNotDeclared,
        ScxmlOnSampleLinkWrongKind,
        // Listener-role — top-level `<sce:session-role>` structural codes
        ScxmlUnknownSessionRoleKind,
        ScxmlDuplicateSessionRoleDeclaration,
        // Listener-role — cross-doc partial-claim + matrix validators
        LinkDeployRoleListenerWithoutScxmlAcceptSideRole,
        ScxmlAcceptSideRoleWithoutListenerLink,
        LinkRoleListenerWithNonSessionArmingTrustClass,
        // Listener-role — migration-helper parser diagnostic
        ScxmlAcceptSideStatesWithoutRoleDeclaration,
        // Declared-consumption — reassembly cross-doc invariant
        ReassemblyPerPeerQuotaBuildInvariantViolated,
        // Expression
        ExpressionEmpty,
        ExpressionLex,
        ExpressionUnsupportedConstruct,
        ExpressionUnsupportedBuiltin,
        ExpressionUnknownIdentifier,
        ExpressionPropertyNotCallable,
        ExpressionNamespaceNotCallable,
        ExpressionNamespaceNotAValue,
        ExpressionLiteralNotCallable,
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
        // Codegen matrix shells (SCE Protocol-Synthesis RFC §synth-5-J-4 / §synth-5-J-5)
        CodegenMcuClassKindOnNonMcuLanguage,
        CodegenGenericKindBackendEmitMissing,
        // Codegen Rust no_std variant rejection (SCE Protocol-Synthesis RFC §synth-5-J-2,
        // item C3)
        CodegenNoStdScriptNotSupported,
        CodegenNoStdHttpNotSupported,
        CodegenNoStdFsLoadNotSupported,
        CodegenNoStdInvokeNotSupported,
        // Algorithm §synth-5-F const-fold (SCE Protocol-Synthesis RFC §synth-5-F, item A4)
        AlgorithmConstNotFoldable,
        AlgorithmConstFoldBudgetExceeded,
        AlgorithmConstYieldTypeMismatch,
        // Codec §synth-5-B variant primitive (SCE Protocol-Synthesis RFC §synth-5-B, item B1)
        CodecVariantArmUnreachable,
        // RFC variant-default-uniformity — duplicate default-arm marker
        CodecVariantDuplicateDefaultArm,
        // RFC variant-default-uniformity — cross-doc MID mismatch
        CodecVariantArmMidMismatch,
        // RFC variant-default-uniformity — inner codec missing wire-MID constant
        CodecVariantArmInnerMidUndeclared,
        // Caller-tag variant shape — variant arm body
        // is a caller-tag dispatcher with no natural tag source
        CodecVariantArmBodyCallerTagUnsupported,
        // RFC variant-default-uniformity — every variant must mark a default arm
        CodecVariantNoDefaultArm,
        // RFC variant-default-overlay — deploy.yaml overlay names an undeclared arm
        CodecVariantDefaultOverlayArmNotDeclared,
        // Parent-tag dispatch — parent-side variant-dispatch (5 codes)
        CodecVariantDispatchFlagNotResolved,
        CodecVariantDispatchBitWidthMismatch,
        CodecVariantDispatchArmsNotDistinguishableWithoutDefault,
        CodecVariantDispatchFlagHasStaticValue,
        CodecVariantDispatchCarrierAfterEmbed,
        // Flag inversion — parent-side flag-bind (6 codes)
        CodecFlagBindInputNotDeclared,
        CodecFlagBindSourceNotResolved,
        CodecFlagBindWidthMismatch,
        CodecFlagInputUnbound,
        CodecFlagBindDuplicateInput,
        CodecFlagBindCarrierAfterEmbed,
        // Codec §synth-5-B present-if primitive (SCE Protocol-Synthesis RFC §synth-5-B, item B1)
        CodecPresentIfRefsLaterField,
        // Codec §synth-5-B repeat primitive (SCE Protocol-Synthesis RFC §synth-5-B, B2)
        CodecRepeatCountRefsLaterField,
        // Algorithm §synth-5-B test-vector primitive (SCE Protocol-Synthesis RFC §synth-5-B, items B2 + B5)
        AlgorithmTestVectorUnsupportedKind,
        // Codec §synth-5-B B3 TLV chain primitive (SCE Protocol-Synthesis RFC §synth-5-B)
        CodecTlvChainDepthUnspecified,
        CodecTlvChainTruncateUnderEntryFlag,
        // Codec §synth-5-B B3 DMA alignment primitive (SCE Protocol-Synthesis RFC §synth-5-B)
        CodecDmaAlignmentUnsatisfiable,
        // Codec §synth-5-B peek-byte cross-codec layout match
        CodecPeekByteFlagLayoutMismatch,
        // Link §synth-5-C byte-stream link endpoint (SCE Protocol-Synthesis RFC §synth-5-C, item B6)
        LinkFramerMissing,
        LinkLinkClassUnknown,
        LinkBackpressureUndeclared,
        LinkClassUnsupportedOnTarget,
        LinkPoolSlotSmallerThanFramerMax,
        LinkPoolRefNotDeclared,
        LinkFramerRefNotDeclared,
        // BufferPool §synth-5-E DMA-aligned slot table (SCE Protocol-Synthesis RFC §synth-5-E, item B7)
        MemPoolSectionConflict,
        MemPoolTooLarge,
        MemInterPoolPaddingNotEmitted,
        // BufferPool §synth-5-E C5 cache-maintenance validation + codegen self-checks (SCE Protocol-Synthesis RFC §synth-5-E + §synth-5-I)
        MemCacheLineAlignment,
        MemDcacheLineSizeNotPowerOfTwo,
        MemAlignmentNotPowerOfTwo,
        MemSlotSizeNotAlignmentMultiple,
        MemCachePolicyUnsupportedOnNoDcacheCore,
        PoolCacheMaintenanceMisplaced,
        PoolSpeculativePrefetchFlagMissing,
        PoolCachePreArmInvalidateMissingOnSpeculativeCore,
        // BufferPool §synth-5-E ownership pull-through (SCE Protocol-Synthesis RFC §synth-5-E, item B7)
        PoolSampleTypestateAttributesDisabled,
        // Sample API §synth-5-E sample-callback application-layer ownership (SCE Protocol-Synthesis RFC §synth-5-E)
        PoolSampleTakeWithoutStagePool,
        // Sample API §synth-5-E sample-callback callback-path syntax (SCE Protocol-Synthesis RFC §synth-5-E)
        PoolSampleCallbackSignatureNonBorrow,
        // Worker kind shared-state encapsulation (SCE Protocol-Synthesis RFC §synth-5-D, item C2)
        WorkerSharedMutableState,
        // Worker kind cross-resolution + inbox ordering (SCE Protocol-Synthesis RFC §synth-5-D + §synth-5-I, item C2)
        WorkerLinkRxRefUnknown,
        WorkerInboxOrderingUnspecified,
        WorkerInboxOrderingRelaxedAcrossCores,
        // Worker kind scheduler-capacity forge-side anchor (SCE Protocol-Synthesis RFC §synth-5-D, item C2)
        WorkerSchedulerUnsupported,
        // Worker kind SCXML-side outbox cross-resolution (SCE Protocol-Synthesis RFC §synth-5-D, item C2)
        WorkerOutboxRefUnknown,
        WorkerOutboxTargetWrongKind,
        WorkerOutboxTargetSuffixInvalid,
        // Fragment-reassembly buffer-pool variant parse-time structure validators (SCE Protocol-Synthesis RFC §synth-5-M, item C9)
        MemReassemblyPoolVariantMissingMaxFragments,
        MemReassemblyPoolVariantMissingTimeout,
        // Fragment-reassembly cross-doc validators (SCE Protocol-Synthesis RFC §synth-5-M, items C9 + C13)
        MemReassemblySlotSizeBelowDeclaredMtu,
        ReassemblyMaxFragmentsInsufficientForMtu,
        ReassemblyExpectedFragmentationRateHigh,
        ReassemblyUntrustedLinkBinding,
        ReassemblyTrustClassMissingOnFragmentingLink,
        ReassemblyStageCopyWcetExceedsSlotBudget,
        // Fragment-reassembly codegen self-check (SCE Protocol-Synthesis RFC §synth-5-M, item C9)
        ReassemblyPeerIdNotZidOnEstablishedSession,
        // Listener-link sibling-pair (SCE Protocol-Synthesis RFC §synth-5-C + §synth-5-M, item C10)
        LinkListenerLinkNotPairedWithEstablishedSibling,
        MeshDeployReassemblyBindingOnUnpairedListener,
        // Multi-link concurrency contract (SCE Protocol-Synthesis RFC §synth-5-N, item C10)
        LinkConcurrentCountExceedsSchedulerSlots,
        LinkPerLinkBudgetExceedsTickPeriod,
        LinkInboundEventQueueUnsized,
        // Bounded-collection kind parse-time structure validators (SCE Protocol-Synthesis RFC §synth-5-L, item C6)
        CollectionOrderingSortedRequiresIndexBy,
        CollectionOverflowPolicyOldestWinsRequiresOrderingInsertion,
        // Bounded-collection kind cross-doc resolution (SCE Protocol-Synthesis RFC §synth-5-L, item C6)
        CollectionElementTypeNotAKind,
        CollectionIndexByFieldMissing,
        CollectionMultiWriterWithoutAtomics,
        // Bounded-collection kind deploy-time capacity resolution (SCE Protocol-Synthesis RFC §synth-5-L, item C6)
        CollectionCapacityUnresolved,
        // Timer kind diagnostics (SCE Protocol-Synthesis RFC §synth-5-D, C1)
        TimerPeriodBelowTickRate,
        TimerSlotOverflow,
        // `<sce:extern>` whitelisted intrinsic registry (SCE Protocol-Synthesis RFC §synth-5-I, item C4)
        ExternSymbolNotInWhitelist,
        ExternAbiMismatch,
        ExternSignatureMismatch,
        ExternOrderingUnspecified,
        // `<sce:extern>` target-plugin extension (SCE Protocol-Synthesis RFC §synth-5-I, item C4)
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
        CliInvalidSuitePackage,
        CliGeneratorSourceDrift,
        CliGeneratorSourceUnverifiable,
        CliUsage,
        CliQueryNoMatch,
        // Mesh Deploy
        MeshDeployRead,
        MeshDeployParse,
        MeshDeployUnsupportedVersion,
        MeshDeployDuplicateMachine,
        MeshDeployInvalidOrderingTimings,
        MeshDeployInvalidDedupWindow,
        MeshDeployInvalidCustomTcpSocket,
        MeshDeployInvalidDdsQos,
        MeshDeployInvalidLiveliness,
        MeshDeployInvalidServerResponseDeadline,
        MeshDeployInvalidOutboundBuffer,
        MeshDeployInvalidRetryPolicy,
        MeshDeployInvalidAuthPolicy,
        MeshDeployDiscoveryNotSupported,
        MeshDeployPoolNotSupportedByTransport,
        MeshDeployPoolMissingMemberList,
        MeshDeployPoolEmptyMemberList,
        MeshDeployPoolBindingFieldNotSupported,
        MeshDeployPoolDispatchWithoutMember,
        MeshDeployPoolInvalidPlaceholder,
        MeshDeployServerPoolNotSupported,
        MeshDeployCrossTargetReplyNotSupported,
        MeshDeployInvalidReplyFrom,
        MeshDeployUnknownBindingField,
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
        // Scheduler-capacity axis (SCE Protocol-Synthesis RFC §synth-5-K lines 2423/2428-9/2430-1)
        MeshDeploySchedulerCooperativeMissingSlotBudget,
        MeshDeploySchedulerCooperativeMissingKeepaliveJitterBudget,
        MeshDeploySchedulerIncompatibleWithWorkerCount,
        // §synth-5-K `links:` block schema + parse-time + cross-doc (RFC §synth-5-K lines 2232-2540)
        MeshDeployLinkDriverUnknown,
        MeshDeployLinkMtuMissingOnFragmentingLink,
        MeshDeployLinkMtuBelowDriverFloor,
        MeshDeployLinkDriverClassMismatch,
        MeshDeployLinkExpectedP99ExceedsMtu,
        MeshDeployLinkBurstPpsMissingOnIsrDispatch,
        MeshDeployLinkNotDeclaredInDeploy,
        MeshDeployLinkNotDeclaredInForge,
        // Cross-doc RX pool burst invariants (RFC §synth-5-K lines 2489-2500)
        MeshDeployLinkBurstAbsorptionInsufficient,
        MeshDeployLinkRxDispatchWorkerTickOnHighBurst,
        // pool_defaults.stage_copy_policy (RFC §synth-5-K lines 2350-2369 + 2504-2519)
        PoolStageCopyPolicyError,
        PoolStageCopyAcceptRejectedUnderForbid,
        MeshDeployStageCopyPolicyUnknown,
        // Anti-flood + stateless_accept (RFC §synth-5-K lines 2272-2349 + 2449-2473)
        MeshDeploySessionArmingQuotaMissing,
        MeshDeployAcceptRateConfigMissing,
        MeshDeploySessionArmingFieldsOnNonArmingLink,
        MeshDeployStatelessAcceptRequiredOnUntrustedSource,
        MeshDeployStatelessAcceptKeyRotationShorterThanLifetime,
        // Peer-table invariant + extern allowlist (RFC §synth-5-K lines 2460-2462 + 2466-2469)
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
        // Forge generated-source drift detection (SCE Protocol-Synthesis RFC §synth-6.2.6)
        ForgeSourceHashMismatch,
        ForgeSourceHashInputUncovered,
        ForgeSourceHashWalkUnbounded,
        // Traceability §synth-5-O — IR provenance pre-emit guard
        // (SCE Protocol-Synthesis RFC §synth-5-O lines 3289-3290).
        TraceabilityScxmlLineRangeMissing,
        // Traceability §synth-5-O — symbol mangling + sourcemap
        // contract (SCE Protocol-Synthesis RFC §synth-5-O lines 3055-3057, 3219-3243,
        // 3321-3324, OQ-W16 a/b locks).
        TraceabilityStateIdCollision,
        TraceabilitySymbolNameExceedsCIdentifierLimit,
        TraceabilitySourcemapSourceHashMismatch,
        TraceabilitySceMapAttributeStripped,
        // Traceability §synth-5-O — ownership-boundary
        // walker fires this when a drift-headered file is missing
        // its SCE-MAP marker. ARCHITECTURE.md "Traceability Ownership
        // Boundary" defines the scope.
        TraceabilityMetaGeneratedSourceLineMarkerMissing,
        // MCU driver/class boundary on the C11 backend (SCE Protocol-Synthesis
        // RFC §5.2). `mcu/driver-header-not-found`
        // covers `<sce:driver href="..."/>` resolution failure;
        // `mcu/section-attribute-on-non-mcu-target` covers the
        // non-MCU backend reject of `platform.c11_section_attribute`.
        McuDriverHeaderNotFound,
        McuSectionAttributeOnNonMcuTarget,
        McuSectionAttributeNameInvalid,
        // ── NL→IR Item C1 Path A: Enum kind invariants ──
        ValidationEnumNoVariants,
        ValidationEnumVariantDuplicateName,
        ValidationEnumVariantDuplicateValue,
        ValidationEnumVariantValueOverflowsUnderlying,
        ValidationEnumUnsupportedUnderlyingType,
        // ── NL→IR Item C1 Path A: EventSchema kind ────
        ValidationEventSchemaOnBuiltinEvent,
        ValidationEventPayloadFieldUnknown,
        ValidationBytesComparisonNotEquality,
        MeshEventSchemaMismatch,
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
            generator: crate::GENERATOR_COMMIT,
            code,
            stage,
            // A meta failure has no document behind it, so nothing
            // was expanded to produce it.
            expanded_from: None,
            spec: code.spec_anchor(),
            message,
            location: None,
            expected: None,
            actual: None,
            fix: None,
            spec_provenance: Vec::new(),
            question_kind: None,
        }
    }

    /// Message carried by the terminal-fallback record.
    pub const TERMINAL_FALLBACK_MESSAGE: &'static str = "double serialization failure";

    /// Last-resort NDJSON line for the case where serializing a
    /// diagnostic fails twice.
    ///
    /// A literal, because at that point serde is the thing that
    /// failed. It lives here rather than at the emitter so the tests
    /// that own the wire contract can hold it to that contract: one
    /// line, valid against `schemas/sce-diagnostic.v1.schema.json`,
    /// and byte-identical to what [`Diagnostic::meta_failure`] would
    /// have produced for the same message.
    ///
    /// A hand-built literal no test compares against the schema is the
    /// drift the rest of this module exists to prevent. The earlier
    /// spelling of this line carried `"id":"fnv1a:0"`, which the
    /// schema's `^fnv1a:[0-9a-f]{16}$` pattern rejects — a record
    /// documented as "the shortest legal NDJSON record" that no
    /// consumer's validator would have accepted.
    ///
    /// `generator` is spliced with `concat!`/`env!` rather than read
    /// from [`crate::GENERATOR_COMMIT`] because `concat!` takes
    /// literals, not constants. Both read the same build-script
    /// variable in the same compilation, and
    /// [`terminal_fallback_line_matches_what_serde_would_emit`] pins
    /// this line to what the struct serializes to, so the two spellings
    /// cannot disagree without a test failing. The stamp matters most
    /// here: this is the record a consumer receives when everything
    /// else about the diagnostic pipeline has already failed.
    pub const TERMINAL_FALLBACK_NDJSON: &'static str = concat!(
        "{\"v\":1,\"id\":\"fnv1a:2d6d674d44088d0d\",\"generator\":\"",
        env!("SCE_GIT_COMMIT"),
        "\",\"code\":\"io/filesystem\",\"stage\":\"io\",\"message\":\"double serialization failure\"}"
    );
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
    /// leaving it empty, because consumers would ground hallucinated
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
            | MeshDeployUnknownBindingField
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

            // ── SCXML §5.8 top-level script (§wire-W5) ─────────────
            ScxmlTopLevelScriptUnloaded => Some("W3C SCXML §5.8"),

            // ── SCXML data models ──────────────────────────────────
            // The attribute is defined in §3.2 and the data models it
            // selects in Appendix B, so the two codes anchor apart: one
            // is about the value being unusable, the other about a
            // construct the selected model has no language for.
            ScxmlUnsupportedDatamodel => Some("W3C SCXML §3.2"),
            ScxmlNullDatamodelForbidsConstruct => Some("W3C SCXML §B.1"),
            // The name the author reached for is one Appendix B.2's
            // datamodel defines and SCE does not implement, so it
            // anchors on the ECMAScript data model rather than on
            // Forge's expression language: a Forge document cannot
            // reach it, and the author fixing it is reading B.2.
            ExpressionUnsupportedBuiltin => Some("W3C SCXML §B.2"),
            // Same anchor, same reason: B.2 is where the ECMAScript data
            // model says the datamodel's variables are the ones the
            // document declares, so a name outside them is answered
            // there and not in Forge's expression language.
            ExpressionUnknownIdentifier => Some("W3C SCXML §B.2"),
            // Same anchor again: B.2 is what makes ECMAScript's own
            // rules the datamodel's rules, and calling a value is one
            // ECMAScript answers for itself.
            ExpressionPropertyNotCallable => Some("W3C SCXML §B.2"),
            // And once more: the appendix is what obliges the namespaces
            // to exist at all, so it is also what decides that reaching
            // one without naming a member is not an expression this
            // datamodel evaluates.
            ExpressionNamespaceNotCallable => Some("W3C SCXML §B.2"),
            // The read half answers to the same appendix, and to the
            // same sentence in it: the datamodel is ECMAScript's, and
            // what SCE hands out for these names is a member set.
            ExpressionNamespaceNotAValue => Some("W3C SCXML §B.2"),
            // The same appendix decides this one too: a call on a value
            // that is not a function is what ECMAScript answers with a
            // TypeError, and the appendix is what makes ECMAScript's
            // answers this datamodel's.
            ExpressionLiteralNotCallable => Some("W3C SCXML §B.2"),

            // ── Algorithm kind (SCE Protocol-Synthesis RFC §synth-5-A) ──────────
            AlgorithmLocalShadowsParam
            | AlgorithmLvalueUnsupported
            | AlgorithmReturnMissing => Some("SCE Protocol-Synthesis RFC §5.A"),

            // ── Algorithm-over-BC dispatch (C7-lowering: RFC §synth-5-A
            //    line 311 + §synth-5-L lines 2611-2618 + 2642-2647). Six
            //    codes share the cross-section anchor that names both
            //    the algorithm `Call`/`Foreach` IR shape and the BC
            //    public-method roster. ─────────────────────────────
            AlgorithmForeachSourceNotIterable
            | AlgorithmCallTargetUnknown
            | AlgorithmCallTargetMethodUnknown
            | AlgorithmBcMutationForbidden
            | AlgorithmForeachSourceBcWithBytesItemType
            | AlgorithmCallArgCountMismatch => Some("SCE Protocol-Synthesis RFC §5.A + §5.L"),

            // ── Algorithm byte-buffer-build (SCE-owned, SCE_FORGE.md §4.12) ─
            AlgorithmAppendTargetNotBuffer | AlgorithmAppendTypeMismatch => {
                Some("SCE Forge §4.12")
            }

            // ── Algorithm §synth-5-F build-time const-fold ─────────────
            AlgorithmConstNotFoldable
            | AlgorithmConstFoldBudgetExceeded
            | AlgorithmConstYieldTypeMismatch => Some("SCE Protocol-Synthesis RFC §5.F"),

            // ── Codec §synth-5-B variant + present-if + repeat + tlv-chain + dma-align primitives (items B1 + B2 + B3) ─
            CodecVariantArmUnreachable
            | CodecVariantDuplicateDefaultArm
            | CodecVariantArmMidMismatch
            | CodecVariantArmInnerMidUndeclared
            | CodecVariantArmBodyCallerTagUnsupported
            | CodecVariantNoDefaultArm
            | CodecVariantDefaultOverlayArmNotDeclared
            | CodecVariantDispatchFlagNotResolved
            | CodecVariantDispatchBitWidthMismatch
            | CodecVariantDispatchArmsNotDistinguishableWithoutDefault
            | CodecVariantDispatchFlagHasStaticValue
            | CodecVariantDispatchCarrierAfterEmbed
            | CodecFlagBindInputNotDeclared
            | CodecFlagBindSourceNotResolved
            | CodecFlagBindWidthMismatch
            | CodecFlagInputUnbound
            | CodecFlagBindDuplicateInput
            | CodecFlagBindCarrierAfterEmbed
            | CodecPresentIfRefsLaterField
            | CodecRepeatCountRefsLaterField
            | AlgorithmTestVectorUnsupportedKind
            | CodecTlvChainDepthUnspecified
            | CodecTlvChainTruncateUnderEntryFlag
            | CodecDmaAlignmentUnsatisfiable
            | CodecPeekByteFlagLayoutMismatch => Some("SCE Protocol-Synthesis RFC §5.B"),

            // ── Link §synth-5-C byte-stream link endpoint (B6) ─────────
            LinkFramerMissing
            | LinkLinkClassUnknown
            | LinkBackpressureUndeclared
            | LinkClassUnsupportedOnTarget
            | LinkPoolSlotSmallerThanFramerMax
            | LinkPoolRefNotDeclared
            | LinkFramerRefNotDeclared => Some("SCE Protocol-Synthesis RFC §5.C"),

            // ── BufferPool §synth-5-E DMA-aligned slot table + the
            //    ownership pull-through + sample-callback Sample API
            //    application-layer ownership (item B7) ──────────────
            MemPoolSectionConflict
            | MemPoolTooLarge
            | MemInterPoolPaddingNotEmitted
            | MemCacheLineAlignment
            | MemDcacheLineSizeNotPowerOfTwo
            | MemAlignmentNotPowerOfTwo
            | MemSlotSizeNotAlignmentMultiple
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
            | ScxmlOnSampleLinkWrongKind => Some("SCE Protocol-Synthesis RFC §5.E"),

            // ── §synth-5-I `<sce:extern>` whitelisted intrinsic registry
            //    (baseline + target-plugin extension, item C4) ──
            ExternSymbolNotInWhitelist
            | ExternAbiMismatch
            | ExternSignatureMismatch
            | ExternOrderingUnspecified
            | ExternTargetPluginSymbolConflict => Some("SCE Protocol-Synthesis RFC §5.I"),

            // ── §synth-5-D Worker kind encapsulation (item C2) ───────────
            WorkerSharedMutableState => Some("SCE Protocol-Synthesis RFC §5.D"),

            // ── Worker cross-resolution (RFC §synth-5-D) + SPSC inbox
            //    ordering (RFC §synth-5-I lines 1752-1758) ──
            //    Cross-ref codes carry §synth-5-D spec anchor (worker schema
            //    is §synth-5-D's domain). Ordering codes carry §synth-5-I anchor
            //    (the SPSC/MPSC ordering contract is §synth-5-I's domain).
            WorkerLinkRxRefUnknown => Some("SCE Protocol-Synthesis RFC §5.D"),
            WorkerInboxOrderingUnspecified
            | WorkerInboxOrderingRelaxedAcrossCores => Some("SCE Protocol-Synthesis RFC §5.I"),

            // ── Worker scheduler-capacity axis (RFC §synth-5-D + §synth-5-K) ──
            //    Forge-side anchor (line 912) lives in §synth-5-D worker
            //    domain; the three deploy-side anchors (line 2423 /
            //    2428-9 / 2430-1) live in §synth-5-K deploy.yaml domain.
            WorkerSchedulerUnsupported
            | TimerPeriodBelowTickRate
            | TimerSlotOverflow => Some("SCE Protocol-Synthesis RFC §5.D"),

            // ── Worker SCXML-side outbox cross-resolution (RFC §synth-5-D,
            //    item C2). All three axes live in §synth-5-D
            //    worker domain (the worker schema's `<sce:outbox>` is
            //    §synth-5-D's; the recipient codegen contract is §synth-5-D's
            //    inbox lowering).
            WorkerOutboxRefUnknown
            | WorkerOutboxTargetWrongKind
            | WorkerOutboxTargetSuffixInvalid => Some("SCE Protocol-Synthesis RFC §5.D"),

            // ── §synth-5-L Bounded-collection kind parse-time structure
            //    validators (SCE Protocol-Synthesis RFC §synth-5-L lines 2540-2655,
            //    item C6). The parse-time codes are XML-structure-only
            //    — the sorted ordering requires an explicit
            //    `<sce:index-by>` field (spec line 2559), and
            //    `oldest-wins` overflow requires `insertion` ordering
            //    (spec line 2655). The cross-doc
            //    codes cover element-type kind resolution
            //    (lines 2566-2567), index-by field enumeration
            //    (line 2615), multi-writer atomic-import surface
            //    (lines 2560-2562). All five sit on §synth-5-L.
            CollectionOrderingSortedRequiresIndexBy
            | CollectionOverflowPolicyOldestWinsRequiresOrderingInsertion
            | CollectionElementTypeNotAKind
            | CollectionIndexByFieldMissing
            | CollectionMultiWriterWithoutAtomics
            | CollectionCapacityUnresolved => {
                Some("SCE Protocol-Synthesis RFC §5.L")
            }

            // ── §synth-5-M Fragment-reassembly variant parse-time structure
            //    validators (SCE Protocol-Synthesis RFC §synth-5-M lines 2944-2945,
            //    item C9). Both codes fire when
            //    `<sce:variant>reassembly</sce:variant>` is declared
            //    without one of its required sibling elements
            //    (max-fragments-per-message at spec line 2688,
            //    reassembly-timeout-ms at spec line 2689). The
            //    cross-doc / cross-link / codegen-side reassembly
            //    diagnostics are anchored below.
            MemReassemblyPoolVariantMissingMaxFragments
            | MemReassemblyPoolVariantMissingTimeout
            // Cross-doc validators (RFC §synth-5-M lines 2946-2995).
            // The 6 cross-doc reassembly codes share §synth-5-M spec anchor.
            | MemReassemblySlotSizeBelowDeclaredMtu
            | ReassemblyMaxFragmentsInsufficientForMtu
            | ReassemblyExpectedFragmentationRateHigh
            | ReassemblyUntrustedLinkBinding
            | ReassemblyTrustClassMissingOnFragmentingLink
            | ReassemblyStageCopyWcetExceedsSlotBudget
            // Codegen self-check (RFC §synth-5-M lines 2976-2981).
            | ReassemblyPeerIdNotZidOnEstablishedSession
            // Reassembly side of the listener-pair (RFC §synth-5-M
            // lines 2982-2994). Shares §synth-5-M anchor with the
            // codegen self-check + the 6 cross-doc
            // reassembly codes.
            | MeshDeployReassemblyBindingOnUnpairedListener => {
                Some("SCE Protocol-Synthesis RFC §5.M")
            }
            // Listener-pair codegen self-check (RFC §synth-5-C lines
            // 849-856). The §synth-5-C anchor matches the existing item B6
            // link family.
            LinkListenerLinkNotPairedWithEstablishedSibling => {
                Some("SCE Protocol-Synthesis RFC §5.C")
            }

            // Multi-link concurrency contract (RFC §synth-5-N lines
            // 3031-3062, item C10). All three codes share the §synth-5-N anchor —
            // distinct §synth-5 section from §synth-5-C (B6 link kind) and §synth-5-M
            // (reassembly variant) so the spec table-of-contents stays
            // readable.
            LinkConcurrentCountExceedsSchedulerSlots
            | LinkPerLinkBudgetExceedsTickPeriod
            | LinkInboundEventQueueUnsized => Some("SCE Protocol-Synthesis RFC §5.N"),

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
            MeshDeployInvalidDedupWindow => Some("SCE Mesh §10.5"),
            MeshDeployInvalidCustomTcpSocket => Some("SCE Mesh §16.8.3"),
            MeshDeployInvalidDdsQos => Some("SCE Mesh §8.2"),

            // ── Mesh communication errors (SCE_MESH.md §16.7) ────
            MeshDeployInvalidLiveliness => Some("SCE Mesh §16.7"),

            // ── Mesh server-side lifecycle (SCE_MESH.md §9.5) ────
            MeshDeployInvalidServerResponseDeadline => Some("SCE Mesh §9.5"),
            MeshDeployInvalidOutboundBuffer => Some("SCE Mesh §10.10"),
            MeshDeployInvalidRetryPolicy => Some("SCE Mesh §16.7"),
            MeshDeployInvalidAuthPolicy => Some("SCE Mesh §16.7"),

            // ── Discovery invariant (SCE_MESH.md §3.3) ──────────
            MeshDeployDiscoveryNotSupported => Some("SCE Mesh §3.3"),

            // ── Mesh remote invoke codegen-shape exclusivity (SCE_MESH.md §9.6) ──
            MeshDeployScxmlInvokeTargetConflict => Some("SCE Mesh §9.6"),

            // ── Mesh cross-device scxml-remote transport (SCE_MESH.md §9.6) ──
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
            | MeshDeployPoolMissingMemberList
            | MeshDeployPoolEmptyMemberList
            | MeshDeployPoolBindingFieldNotSupported
            | MeshDeployPoolDispatchWithoutMember
            | MeshDeployPoolInvalidPlaceholder
            | MeshDeployServerPoolNotSupported
            | MeshTopologyPoolParamNameMissing => Some("SCE Mesh §14.4"),

            // ── Mesh RPC responder set (SCE_MESH.md §14.6) ──
            MeshDeployCrossTargetReplyNotSupported | MeshDeployInvalidReplyFrom => Some("SCE Mesh §14.6"),

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

            // ── Scheduler-capacity deploy-side anchors
            //    (SCE Protocol-Synthesis RFC §synth-5-K lines 2423 / 2426 / 2428-9 / 2430-1).
            //    The stack-budget variant was renamed
            //    from the SCE-Mesh-prefix wire to the deploy-prefix
            //    wire (`deploy/worker-stack-budget-missing`); the spec
            //    anchor follows the rename. The three sibling variants
            //    follow the same anchor. ──
            MeshDeploySchedulerCooperativeMissingStackBudget
            | MeshDeploySchedulerCooperativeMissingSlotBudget
            | MeshDeploySchedulerCooperativeMissingKeepaliveJitterBudget
            | MeshDeploySchedulerIncompatibleWithWorkerCount
            // §synth-5-K `links:` block (RFC §synth-5-K lines 2232-2540) +
            // cross-doc RX-pool burst invariants (RFC §synth-5-K
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
            // pool_defaults.stage_copy_policy (RFC §synth-5-K lines
            // 2350-2369 + 2504-2519). Three codes share §synth-5-K anchor.
            | PoolStageCopyPolicyError
            | PoolStageCopyAcceptRejectedUnderForbid
            | MeshDeployStageCopyPolicyUnknown
            // Anti-flood + stateless_accept (RFC §synth-5-K lines
            // 2272-2349 + 2449-2473). Five codes share §synth-5-K anchor.
            | MeshDeploySessionArmingQuotaMissing
            | MeshDeployAcceptRateConfigMissing
            | MeshDeploySessionArmingFieldsOnNonArmingLink
            | MeshDeployStatelessAcceptRequiredOnUntrustedSource
            | MeshDeployStatelessAcceptKeyRotationShorterThanLifetime
            // Peer-table invariant (RFC §synth-5-K line
            // 2460-2462) + extern allowlist (RFC §synth-5-K line 2466-2469).
            | MeshDeploySessionArmingQuotaVsPeerTableInvariantViolated
            | MeshDeployStatelessAcceptExternNotWhitelisted
                => Some("SCE Protocol-Synthesis RFC §5.K"),

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

            // ── §synth-5-J-2 Rust no_std variant (item C3) ─────────────
            //    Both rejections anchor on the SCE Protocol-Synthesis RFC
            //    section that defines the no_std variant of the Rust
            //    backend; the script-engine incompatibility is line
            //    1989 ("zero alloc dependency") and the http-send
            //    incompatibility is line 1983 ("`sce-rust-runtime`
            //    grows a `no_std` feature gate").
            CodegenNoStdScriptNotSupported
            | CodegenNoStdHttpNotSupported
            | CodegenNoStdFsLoadNotSupported
            | CodegenNoStdInvokeNotSupported => Some("SCE Protocol-Synthesis RFC §5.J.2"),

            // ── §synth-6.2.6 generated-source drift detection (B9, 2026-05-14) ──
            //    The single mismatch code covers both axes (source-hash
            //    + template-hash); the spec section defines the header
            //    contract + `sce-build verify` recompute pipeline.
            ForgeSourceHashMismatch => Some("SCE Protocol-Synthesis RFC §6.2.6"),
            //    Same section: it defines the source set the header's
            //    `source-hash` folds, which is the invariant this code
            //    guards at emit time rather than at verify time.
            ForgeSourceHashInputUncovered => Some("SCE Protocol-Synthesis RFC §6.2.6"),

            // ── Same section, enumeration side rather than coverage
            //    side: §6.2.6 defines the source set as every `*.scxml`
            //    under the root, and this fires when that set cannot be
            //    enumerated within the walk's descent ceiling. ──
            ForgeSourceHashWalkUnbounded => Some("SCE Protocol-Synthesis RFC §6.2.6"),

            // ── §synth-5-O traceability IR provenance pre-emit
            //    guard. Spec lines 3289-3290 verbatim: "Codegen failure
            //    ... surfaced via `traceability/scxml-line-range-
            //    missing` (codegen-internal)". Anchors the diagnostic
            //    against the per-IR-node `(file_id, line, column)`
            //    invariant the SCE-MAP markers + sourcemap
            //    both consume.
            TraceabilityScxmlLineRangeMissing => Some("SCE Protocol-Synthesis RFC §5.O"),

            // ── §synth-5-O symbol mangling + sourcemap contract.
            //    All 4 anchor at SCE Protocol-Synthesis RFC §synth-5-O (the same
            //    section heading): spec lines 3055-3057
            //    (mangling pattern), 3219-3243 (sourcemap JSON shape),
            //    3321-3324 (source_hash byte-equality), OQ-W16 a/b
            //    (escape encoding + `#[doc]` preservation).
            TraceabilityStateIdCollision
            | TraceabilitySymbolNameExceedsCIdentifierLimit
            | TraceabilitySourcemapSourceHashMismatch
            | TraceabilitySceMapAttributeStripped
            | TraceabilityMetaGeneratedSourceLineMarkerMissing => Some("SCE Protocol-Synthesis RFC §5.O"),

            // ── MCU driver/class boundary on the C11 backend
            //    (SCE Protocol-Synthesis RFC §5.2). Both anchor at §5.2
            //    (driver header reference
            //    + non-MCU section attribute reject).
            McuDriverHeaderNotFound
            | McuSectionAttributeOnNonMcuTarget
            | McuSectionAttributeNameInvalid => {
                Some("SCE Protocol-Synthesis RFC §5.2")
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
            | XmlPreprocessorNotRun
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
            | ValidationNativeActionPlacement
            | ValidationNativeActionArgument
            | ValidationNativeActionSignatureConflict
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
            | CliInvalidSuitePackage
            | CliGeneratorSourceDrift
            | CliGeneratorSourceUnverifiable
            | CliUsage
            | CliQueryNoMatch
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
            | MeshIo
            // The listener-role contract is
            // SCE-internal, not a synthesis-RFC spec section. The
            // codes describe an SCE-only cross-doc role contract;
            // spec_anchor stays None until / unless a future spec
            // section adopts the same vocabulary.
            | ScxmlUnknownSessionRoleKind
            | ScxmlDuplicateSessionRoleDeclaration
            | LinkDeployRoleListenerWithoutScxmlAcceptSideRole
            | ScxmlAcceptSideRoleWithoutListenerLink
            | LinkRoleListenerWithNonSessionArmingTrustClass
            | ScxmlAcceptSideStatesWithoutRoleDeclaration
            // NL→IR Mapping Roadmap Items 1 + 5 — sce:req and
            // sce:unresolved are SCE-internal extensions
            // (`nl_to_ir_mapping_roadmap.md` §"Item 1" / §"Item 5").
            // No external spec defines the rejection rules, so
            // spec_anchor stays None.
            | ValidationDuplicateRequirementId
            | ValidationUnresolvedPlaceholder
            // NL→IR Mapping Roadmap Item 2 — cross-kind typed binding
            // diagnostics also originate from
            // `nl_to_ir_mapping_roadmap.md` (Item 2). No external spec
            // anchor; the rejection contract is SCE-internal.
            | ValidationCrossKindFieldNotFound
            | ValidationCrossKindTypeMismatch
            | ValidationCrossKindCircularDependency
            // NL→IR Mapping Roadmap Item 3 — reachability is
            // implied by §scxml-3 entry semantics (the design-time
            // BFS over `initial`, parallel cascade, history default
            // targets, and transition `target` edges), but no spec
            // section names "unreachable state" as a rejection. Treat
            // as SCE-internal hygiene; spec_anchor stays None.
            | ScxmlUnreachableState
            | ScxmlDeadTransition
            // NL→IR Mapping Roadmap Item 3 — event-set
            // exhaustiveness. Heuristic over §scxml-5.10 event
            // matching, but no spec section names "non-exhaustive
            // event handling" as a rejection. SCE-internal hygiene.
            // The two `sce:unhandled` declaration codes police an SCE
            // extension attribute, so they have no spec anchor either.
            | ScxmlNonExhaustiveEventHandling
            | ScxmlContradictoryUnhandledDeclaration
            | ScxmlStaleUnhandledDeclaration
            // NL→IR Mapping Roadmap Item 3 — guard analysis.
            // §scxml-5.10 transition selection implies that an
            // always-false guard makes the transition unreachable
            // and a shadowed transition cannot fire, but the spec
            // does not name these as rejections. SCE-internal
            // hygiene.
            | ScxmlAlwaysFalseGuard
            | ScxmlShadowedTransition => None,
            // The reassembly declared-consumption invariant carries the spec anchor
            // that lived in the diagnostic.rs:1170 placeholder comment.
            ReassemblyPerPeerQuotaBuildInvariantViolated => {
                Some("SCE Protocol-Synthesis RFC §5.M")
            }
            // NL→IR Item C1 Path A: Enum kind invariants live in the
            // design RFC and prescoping-reopen RFC, not in an
            // external spec. Internal hygiene — same stance as
            // SCE-internal SCXML hygiene codes above.
            ValidationEnumNoVariants
            | ValidationEnumVariantDuplicateName
            | ValidationEnumVariantDuplicateValue
            | ValidationEnumVariantValueOverflowsUnderlying
            | ValidationEnumUnsupportedUnderlyingType
            // NL→IR Item C1 Path A: EventSchema kind — the
            // three rejections (built-in-event schema, send-side
            // payload field-unknown, mesh cross-machine schema
            // mismatch) are SCE-internal hygiene with no external
            // spec section naming them — the W3C-defined platform
            // contract on `error.*` etc. drives DL-9' but does not
            // name the rejection; DL-4' and DL-7' are SCE typed-
            // binding and mesh contracts respectively. spec_anchor
            // stays None.
            | ValidationEventSchemaOnBuiltinEvent
            | ValidationEventPayloadFieldUnknown
            | ValidationBytesComparisonNotEquality
            | MeshEventSchemaMismatch => None,
        }
    }

    /// Slash-path string form used in the content hash. Must match the
    /// serde `rename` on each variant exactly.
    ///
    /// Public because this string *is* the wire `code` — it already
    /// leaves the crate on every NDJSON record, and a consumer that has
    /// the typed value in hand (the `check` manifest's per-backend
    /// verdict) must be able to name it without re-serialising a whole
    /// diagnostic to read one field back out.
    pub fn as_str(&self) -> &'static str {
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
            XmlPreprocessorNotRun => "xml/preprocessor-not-run",
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
            ValidationNativeActionPlacement => "validation/native-action-placement",
            ValidationNativeActionArgument => "validation/native-action-argument",
            ValidationNativeActionSignatureConflict => {
                "validation/native-action-signature-conflict"
            }
            ValidationMeshRpcReservedParam => "validation/mesh-rpc-reserved-param",
            ValidationMeshRpcMissingTarget => "validation/mesh-rpc-missing-target",
            ValidationMeshRpcDuplicateTarget => "validation/mesh-rpc-duplicate-target",
            ValidationRemovedAttribute => "validation/removed-attribute",
            ValidationBytesMaxSizeViolation => "validation/bytes-max-size-violation",
            ValidationDuplicateRequirementId => "validation/duplicate-requirement-id",
            ValidationUnresolvedPlaceholder => "validation/unresolved-placeholder",
            ValidationCrossKindFieldNotFound => "validation/cross-kind-field-not-found",
            ValidationCrossKindTypeMismatch => "validation/cross-kind-type-mismatch",
            ValidationCrossKindCircularDependency => "validation/cross-kind-circular-dependency",
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
            AlgorithmAppendTargetNotBuffer => "algorithm/append-target-not-buffer",
            AlgorithmAppendTypeMismatch => "algorithm/append-type-mismatch",
            ScxmlTopLevelScriptUnloaded => "scxml/top-level-script-unloaded",
            ScxmlUnsupportedDatamodel => "scxml/unsupported-datamodel",
            ScxmlNullDatamodelForbidsConstruct => "scxml/null-datamodel-forbids-construct",
            ScxmlUnreachableState => "scxml/unreachable-state",
            ScxmlDeadTransition => "scxml/dead-transition",
            ScxmlNonExhaustiveEventHandling => "scxml/non-exhaustive-event-handling",
            ScxmlContradictoryUnhandledDeclaration => "scxml/contradictory-unhandled-declaration",
            ScxmlStaleUnhandledDeclaration => "scxml/stale-unhandled-declaration",
            ScxmlAlwaysFalseGuard => "scxml/always-false-guard",
            ScxmlShadowedTransition => "scxml/shadowed-transition",
            ScxmlOnSampleInvalidParent => "scxml/on-sample-invalid-parent",
            ScxmlOnSampleLinkDuplicateInState => "scxml/on-sample-link-duplicate-in-state",
            ScxmlOnSampleEventNameConflict => "scxml/on-sample-event-name-conflict",
            ScxmlOnSampleLinkNotDeclared => "scxml/on-sample-link-not-declared",
            ScxmlOnSampleLinkWrongKind => "scxml/on-sample-link-wrong-kind",
            ScxmlUnknownSessionRoleKind => "scxml/unknown-session-role-kind",
            ScxmlDuplicateSessionRoleDeclaration => "scxml/duplicate-session-role-declaration",
            LinkDeployRoleListenerWithoutScxmlAcceptSideRole => {
                "link/deploy-role-listener-without-scxml-accept-side-role"
            }
            ScxmlAcceptSideRoleWithoutListenerLink => {
                "scxml/accept-side-role-without-listener-link"
            }
            LinkRoleListenerWithNonSessionArmingTrustClass => {
                "link/role-listener-with-non-session-arming-trust-class"
            }
            ScxmlAcceptSideStatesWithoutRoleDeclaration => {
                "scxml/accept-side-states-without-role-declaration"
            }
            ReassemblyPerPeerQuotaBuildInvariantViolated => {
                "reassembly/per-peer-quota-build-invariant-violated"
            }
            ExpressionEmpty => "expression/empty",
            ExpressionLex => "expression/lex",
            ExpressionUnsupportedConstruct => "expression/unsupported-construct",
            ExpressionUnsupportedBuiltin => "expression/unsupported-builtin",
            ExpressionUnknownIdentifier => "expression/unknown-identifier",
            ExpressionPropertyNotCallable => "expression/property-not-callable",
            ExpressionNamespaceNotCallable => "expression/namespace-not-callable",
            ExpressionNamespaceNotAValue => "expression/namespace-not-a-value",
            ExpressionLiteralNotCallable => "expression/literal-not-callable",
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
            CodecVariantArmMidMismatch => "codec/variant-arm-mid-mismatch",
            CodecVariantArmInnerMidUndeclared => "codec/variant-arm-inner-mid-undeclared",
            CodecVariantArmBodyCallerTagUnsupported => {
                "codec/variant-arm-body-caller-tag-unsupported"
            }
            CodecVariantNoDefaultArm => "codec/variant-no-default-arm",
            CodecVariantDefaultOverlayArmNotDeclared => {
                "codec/variant-default-overlay-arm-not-declared"
            }
            CodecVariantDispatchFlagNotResolved => "codec/variant-dispatch-flag-not-resolved",
            CodecVariantDispatchBitWidthMismatch => "codec/variant-dispatch-bit-width-mismatch",
            CodecVariantDispatchArmsNotDistinguishableWithoutDefault => {
                "codec/variant-dispatch-arms-not-distinguishable-without-default"
            }
            CodecVariantDispatchFlagHasStaticValue => {
                "codec/variant-dispatch-flag-has-static-value"
            }
            CodecVariantDispatchCarrierAfterEmbed => "codec/variant-dispatch-carrier-after-embed",
            CodecFlagBindInputNotDeclared => "codec/flag-bind-input-not-declared",
            CodecFlagBindSourceNotResolved => "codec/flag-bind-source-not-resolved",
            CodecFlagBindWidthMismatch => "codec/flag-bind-width-mismatch",
            CodecFlagInputUnbound => "codec/flag-input-unbound",
            CodecFlagBindDuplicateInput => "codec/flag-bind-duplicate-input",
            CodecFlagBindCarrierAfterEmbed => "codec/flag-bind-carrier-after-embed",
            CodecPresentIfRefsLaterField => "codec/present-if-refs-later-field",
            CodecRepeatCountRefsLaterField => "codec/repeat-count-refs-later-field",
            AlgorithmTestVectorUnsupportedKind => "algorithm/test-vector-unsupported-kind",
            CodecTlvChainDepthUnspecified => "codec/tlv-chain-depth-unspecified",
            CodecTlvChainTruncateUnderEntryFlag => "codec/tlv-chain-truncate-under-entry-flag",
            CodecDmaAlignmentUnsatisfiable => "codec/dma-alignment-unsatisfiable",
            CodecPeekByteFlagLayoutMismatch => "codec/peek-byte-flag-layout-mismatch",
            LinkFramerMissing => "link/framer-missing",
            LinkLinkClassUnknown => "link/link-class-unknown",
            LinkBackpressureUndeclared => "link/backpressure-undeclared",
            LinkClassUnsupportedOnTarget => "link/class-unsupported-on-target",
            LinkPoolSlotSmallerThanFramerMax => "link/pool-slot-smaller-than-framer-max",
            LinkPoolRefNotDeclared => "link/pool-ref-not-declared",
            LinkFramerRefNotDeclared => "link/framer-ref-not-declared",
            MemPoolSectionConflict => "mem/pool-section-conflict",
            MemPoolTooLarge => "mem/pool-too-large",
            MemInterPoolPaddingNotEmitted => "mem/inter-pool-padding-not-emitted",
            MemCacheLineAlignment => "mem/cache-line-alignment",
            MemDcacheLineSizeNotPowerOfTwo => "mem/dcache-line-size-not-power-of-two",
            MemAlignmentNotPowerOfTwo => "mem/alignment-not-power-of-two",
            MemSlotSizeNotAlignmentMultiple => "mem/slot-size-not-alignment-multiple",
            MemCachePolicyUnsupportedOnNoDcacheCore => {
                "mem/cache-policy-unsupported-on-no-dcache-core"
            }
            PoolCacheMaintenanceMisplaced => "pool/cache-maintenance-misplaced",
            PoolSpeculativePrefetchFlagMissing => "pool/speculative-prefetch-flag-missing",
            PoolCachePreArmInvalidateMissingOnSpeculativeCore => {
                "pool/cache-pre-arm-invalidate-missing-on-speculative-core"
            }
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
            MemReassemblyPoolVariantMissingMaxFragments => {
                "mem/reassembly-pool-variant-missing-max-fragments"
            }
            MemReassemblyPoolVariantMissingTimeout => "mem/reassembly-pool-variant-missing-timeout",
            MemReassemblySlotSizeBelowDeclaredMtu => "mem/reassembly-slot-size-below-declared-mtu",
            ReassemblyMaxFragmentsInsufficientForMtu => {
                "reassembly/max-fragments-insufficient-for-mtu"
            }
            ReassemblyExpectedFragmentationRateHigh => {
                "reassembly/expected-fragmentation-rate-high"
            }
            ReassemblyUntrustedLinkBinding => "reassembly/untrusted-link-binding",
            ReassemblyTrustClassMissingOnFragmentingLink => {
                "reassembly/trust-class-missing-on-fragmenting-link"
            }
            ReassemblyStageCopyWcetExceedsSlotBudget => {
                "reassembly/stage-copy-wcet-exceeds-slot-budget"
            }
            ReassemblyPeerIdNotZidOnEstablishedSession => {
                "reassembly/peer-id-not-zid-on-established-session"
            }
            LinkListenerLinkNotPairedWithEstablishedSibling => {
                "link/listener-link-not-paired-with-established-sibling"
            }
            MeshDeployReassemblyBindingOnUnpairedListener => {
                "reassembly/binding-on-unpaired-listener"
            }
            LinkConcurrentCountExceedsSchedulerSlots => {
                "link/concurrent-count-exceeds-scheduler-slots"
            }
            LinkPerLinkBudgetExceedsTickPeriod => "link/per-link-budget-exceeds-tick-period",
            LinkInboundEventQueueUnsized => "link/inbound-event-queue-unsized",
            CollectionOrderingSortedRequiresIndexBy => {
                "collection/ordering-sorted-requires-index-by"
            }
            CollectionOverflowPolicyOldestWinsRequiresOrderingInsertion => {
                "collection/overflow-policy-oldest-wins-requires-ordering-insertion"
            }
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
            CliInvalidSuitePackage => "cli/invalid-suite-package",
            CliGeneratorSourceDrift => "cli/generator-source-drift",
            CliGeneratorSourceUnverifiable => "cli/generator-source-unverifiable",
            CliUsage => "cli/usage",
            CliQueryNoMatch => "cli/query-no-match",
            MeshDeployRead => "mesh/deploy-read",
            MeshDeployParse => "mesh/deploy-parse",
            MeshDeployUnsupportedVersion => "mesh/deploy-unsupported-version",
            MeshDeployDuplicateMachine => "mesh/deploy-duplicate-machine",
            MeshDeployInvalidOrderingTimings => "mesh/deploy-invalid-ordering-timings",
            MeshDeployInvalidDedupWindow => "mesh/deploy-invalid-dedup-window",
            MeshDeployInvalidCustomTcpSocket => "mesh/deploy-invalid-custom-tcp-socket",
            MeshDeployInvalidDdsQos => "mesh/deploy-invalid-dds-qos",
            MeshDeployInvalidLiveliness => "mesh/deploy-invalid-liveliness",
            MeshDeployInvalidServerResponseDeadline => {
                "mesh/deploy-invalid-server-response-deadline"
            }
            MeshDeployInvalidOutboundBuffer => "mesh/deploy-invalid-outbound-buffer",
            MeshDeployInvalidRetryPolicy => "mesh/deploy-invalid-retry-policy",
            MeshDeployInvalidAuthPolicy => "mesh/deploy-invalid-auth-policy",
            MeshDeployDiscoveryNotSupported => "mesh/deploy-discovery-not-supported",
            MeshDeployPoolNotSupportedByTransport => "mesh/deploy-pool-not-supported-by-transport",
            MeshDeployPoolMissingMemberList => "mesh/deploy-pool-missing-member-list",
            MeshDeployPoolEmptyMemberList => "mesh/deploy-pool-empty-member-list",
            MeshDeployPoolBindingFieldNotSupported => {
                "mesh/deploy-pool-binding-field-not-supported"
            }
            MeshDeployPoolDispatchWithoutMember => "mesh/deploy-pool-dispatch-without-member",
            MeshDeployPoolInvalidPlaceholder => "mesh/deploy-pool-invalid-placeholder",
            MeshDeployServerPoolNotSupported => "mesh/deploy-server-pool-not-supported",
            MeshDeployCrossTargetReplyNotSupported => {
                "mesh/deploy-cross-target-reply-not-supported"
            }
            MeshDeployInvalidReplyFrom => "mesh/deploy-invalid-reply-from",
            MeshDeployUnknownBindingField => "mesh/deploy-unknown-binding-field",
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
            MeshDeployPartitionPartialCoverageRequiresDefault => {
                "mesh/deploy-partition-partial-coverage-requires-default"
            }
            MeshDeployPartitionPoolMachine => "mesh/deploy-partition-pool-machine",
            MeshDeployPartitionTransportBindingUnsupported => {
                "mesh/deploy-partition-transport-binding-unsupported"
            }
            MeshDeployScxmlInvokeCrossDeviceTransport => {
                "mesh/deploy-scxml-invoke-cross-device-transport"
            }
            MeshDeploySomeipScxmlInvokeServiceIdOverflow => {
                "mesh/deploy-someip-scxml-invoke-service-id-overflow"
            }
            MeshDeploySomeipScxmlInvokeServiceIdPinOutOfRange => {
                "mesh/deploy-someip-scxml-invoke-service-id-pin-out-of-range"
            }
            MeshDeploySomeipScxmlInvokeServiceIdPinCollision => {
                "mesh/deploy-someip-scxml-invoke-service-id-pin-collision"
            }
            MeshDeploySomeipLivenessServiceIdOverflow => {
                "mesh/deploy-someip-liveness-service-id-overflow"
            }
            MeshDeploySomeipLivenessServiceIdPinOutOfRange => {
                "mesh/deploy-someip-liveness-service-id-pin-out-of-range"
            }
            MeshDeploySomeipLivenessServiceIdPinCollision => {
                "mesh/deploy-someip-liveness-service-id-pin-collision"
            }
            MeshDeploySomeipMachineLivenessServiceIdOverflow => {
                "mesh/deploy-someip-machine-liveness-service-id-overflow"
            }
            MeshDeploySomeipMachineLivenessServiceIdPinOutOfRange => {
                "mesh/deploy-someip-machine-liveness-service-id-pin-out-of-range"
            }
            MeshDeploySomeipMachineLivenessServiceIdPinCollision => {
                "mesh/deploy-someip-machine-liveness-service-id-pin-collision"
            }
            MeshDeployPartitionBarrierTimeoutInvalid => {
                "mesh/deploy-partition-barrier-timeout-invalid"
            }
            MeshPartitionParallelRootUndesignated => "mesh/partition-parallel-root-undesignated",
            MeshPartitionParallelRootAmbiguous => "mesh/partition-parallel-root-ambiguous",
            MeshPartitionParallelRootNotInMachines => {
                "mesh/partition-parallel-root-not-in-machines"
            }
            MeshPartitionParallelRootNonHost => "mesh/partition-parallel-root-non-host",
            MeshPartitionBarrierTimeoutWithoutRoot => "mesh/partition-barrier-timeout-without-root",
            MeshPartitionWire21CustomTcpUnimplemented => {
                "mesh/partition-wire21-custom-tcp-unimplemented"
            }
            MeshDistributabilityR1SharedWrite => "mesh/distributability-r1-shared-write",
            MeshDistributabilityR2CrossRegionTransition => {
                "mesh/distributability-r2-cross-region-transition"
            }
            MeshDeployPlatformClassOsMismatch => "mesh/deploy-platform-class-os-mismatch",
            MeshDeploySchedulerCooperativeMissingStackBudget => {
                "deploy/worker-stack-budget-missing"
            }
            MeshDeploySchedulerCooperativeMissingSlotBudget => "deploy/worker-slot-budget-missing",
            MeshDeploySchedulerCooperativeMissingKeepaliveJitterBudget => {
                "deploy/keepalive-jitter-budget-missing"
            }
            MeshDeploySchedulerIncompatibleWithWorkerCount => {
                "deploy/scheduler-incompatible-with-worker-count"
            }
            MeshDeployLinkDriverUnknown => "deploy/link-driver-unknown",
            MeshDeployLinkMtuMissingOnFragmentingLink => {
                "deploy/link-mtu-missing-on-fragmenting-link"
            }
            MeshDeployLinkMtuBelowDriverFloor => "deploy/link-mtu-below-driver-floor",
            MeshDeployLinkDriverClassMismatch => "deploy/link-driver-class-mismatch",
            MeshDeployLinkExpectedP99ExceedsMtu => "deploy/link-expected-p99-exceeds-mtu",
            MeshDeployLinkBurstPpsMissingOnIsrDispatch => {
                "deploy/link-burst-pps-missing-on-isr-dispatch"
            }
            MeshDeployLinkNotDeclaredInDeploy => "deploy/link-not-declared-in-deploy",
            MeshDeployLinkNotDeclaredInForge => "deploy/link-not-declared-in-forge",
            MeshDeployLinkBurstAbsorptionInsufficient => {
                "deploy/link-burst-absorption-insufficient"
            }
            MeshDeployLinkRxDispatchWorkerTickOnHighBurst => {
                "deploy/link-rx-dispatch-worker-tick-on-high-burst"
            }
            PoolStageCopyPolicyError => "pool/stage-copy-policy-error",
            PoolStageCopyAcceptRejectedUnderForbid => {
                "pool/stage-copy-accept-rejected-under-forbid"
            }
            MeshDeployStageCopyPolicyUnknown => "deploy/stage-copy-policy-unknown",
            MeshDeploySessionArmingQuotaMissing => "deploy/session-arming-quota-missing",
            MeshDeployAcceptRateConfigMissing => "deploy/accept-rate-config-missing",
            MeshDeploySessionArmingFieldsOnNonArmingLink => {
                "deploy/session-arming-fields-on-non-arming-link"
            }
            MeshDeployStatelessAcceptRequiredOnUntrustedSource => {
                "deploy/stateless-accept-required-on-untrusted-source"
            }
            MeshDeployStatelessAcceptKeyRotationShorterThanLifetime => {
                "deploy/stateless-accept-key-rotation-shorter-than-lifetime"
            }
            MeshDeploySessionArmingQuotaVsPeerTableInvariantViolated => {
                "deploy/session-arming-quota-vs-peer-table-invariant-violated"
            }
            MeshDeployStatelessAcceptExternNotWhitelisted => {
                "deploy/stateless-accept-extern-not-whitelisted"
            }
            MeshExternalRead => "mesh/external-read",
            MeshExternalParse => "mesh/external-parse",
            MeshExternalUnresolvedNames => "mesh/external-unresolved-names",
            MeshExternalAmbiguousEventGroup => "mesh/external-ambiguous-event-group",
            MeshExternalEmptyEventGroup => "mesh/external-empty-event-group",
            MeshExternalNamedReferenceWithoutConfig => {
                "mesh/external-named-reference-without-config"
            }
            MeshExternalReservedSomeipIdKeys => "mesh/external-reserved-someip-id-keys",
            MeshExternalSomeipFieldOnNonSomeipTransport => {
                "mesh/external-someip-field-on-non-someip-transport"
            }
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
            MeshTopologyMachineLifetimeSubscriptionUnsupported => {
                "mesh/topology-machine-lifetime-subscription-unsupported"
            }
            MeshCodegenUnsupportedLanguage => "mesh/codegen-unsupported-language",
            MeshCodegenUnsupportedTransport => "mesh/codegen-unsupported-transport",
            MeshCodegenTemplateRead => "mesh/codegen-template-read",
            MeshCodegenTemplateRender => "mesh/codegen-template-render",
            MeshCodegenEventNameCollision => "mesh/codegen-event-name-collision",
            MeshCodegenPoolWithRpcClientUnsupported => {
                "mesh/codegen-pool-with-rpc-client-unsupported"
            }
            MeshIo => "mesh/io",
            ForgeSourceHashMismatch => "forge/source-hash-mismatch",
            ForgeSourceHashInputUncovered => "forge/source-hash-input-uncovered",
            ForgeSourceHashWalkUnbounded => "forge/source-hash-walk-unbounded",
            TraceabilityScxmlLineRangeMissing => "traceability/scxml-line-range-missing",
            TraceabilityStateIdCollision => "traceability/state-id-collision",
            TraceabilitySymbolNameExceedsCIdentifierLimit => {
                "traceability/symbol-name-exceeds-c-identifier-limit"
            }
            TraceabilitySourcemapSourceHashMismatch => {
                "traceability/sourcemap-source-hash-mismatch"
            }
            TraceabilitySceMapAttributeStripped => "traceability/sce-map-attribute-stripped",
            TraceabilityMetaGeneratedSourceLineMarkerMissing => {
                "traceability/meta-generated-source-line-marker-missing"
            }
            McuDriverHeaderNotFound => "mcu/driver-header-not-found",
            McuSectionAttributeOnNonMcuTarget => "mcu/section-attribute-on-non-mcu-target",
            McuSectionAttributeNameInvalid => "mcu/section-attribute-name-invalid",
            // ── NL→IR Item C1 Path A: Enum kind invariants ──
            ValidationEnumNoVariants => "validation/enum-no-variants",
            ValidationEnumVariantDuplicateName => "validation/enum-variant-duplicate-name",
            ValidationEnumVariantDuplicateValue => "validation/enum-variant-duplicate-value",
            ValidationEnumVariantValueOverflowsUnderlying => {
                "validation/enum-variant-value-overflows-underlying"
            }
            ValidationEnumUnsupportedUnderlyingType => {
                "validation/enum-unsupported-underlying-type"
            }
            // ── NL→IR Item C1 Path A: EventSchema kind ──
            ValidationEventSchemaOnBuiltinEvent => "validation/event-schema-on-builtin-event",
            ValidationEventPayloadFieldUnknown => "validation/event-payload-field-unknown",
            ValidationBytesComparisonNotEquality => "validation/bytes-comparison-not-equality",
            MeshEventSchemaMismatch => "mesh/event-schema-mismatch",
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
/// constraint, the payload is attached here. Consumers therefore inspect
/// `fix` and `fix` only to drive repair — `expected` carries a
/// different kind of information (see `Diagnostic::expected`) and the
/// two fields never overlap.
///
/// The variant encodes the *shape* of the repair:
///
/// * Deterministic: `AddAttribute`, `RenameDuplicate`, `RemoveFields`,
///   `ReplaceWith` — applicable without further judgment.
/// * Choice-based: `ReplaceOneOf`, `AddOneOf` — the producer lists the
///   closed candidate set and the consumer (or the human) picks.
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
    RemoveFields {
        location: String,
        fields: Vec<String>,
    },

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
    /// single answer. The consumer must choose which candidate fits the
    /// surrounding context.
    ReplaceOneOf { candidates: Vec<String> },

    /// Add one of several legal attributes to the named element. Used
    /// for "require either X or Y" constraints (e.g. `<send>` needs
    /// `event` or `eventexpr`). The consumer must choose which of `attrs`
    /// to emit based on surrounding context.
    AddOneOf { element: String, attrs: Vec<String> },
}

impl Fix {
    /// Drop a choice-shaped proposal whose choice set came out empty.
    ///
    /// `replace_one_of` / `add_one_of` are defined by
    /// SCE_ERROR_CONTRACT.md §3.1 as "the consumer or human chooses
    /// from the list", and there is no choosing from nothing. A
    /// producer reaching that state has not found a degenerate repair,
    /// it has found no repair — which §3 already has a spelling for:
    /// `fix` absent. Two sites had settled on opposite readings of
    /// this (the statechart reference validator collapsed the empty
    /// case to `None`, the cross-kind field validator shipped
    /// `candidates: []` as an "honest degenerate closed set"), which
    /// put both shapes on one wire field for consumers to
    /// discriminate with nothing to discriminate on.
    ///
    /// Applied once, at the single point where a payload becomes a
    /// record, rather than at each construction site — the sites know
    /// their candidate set, they should not each have to remember what
    /// an empty one means.
    fn with_a_choice_to_offer(self) -> Option<Fix> {
        match &self {
            Fix::ReplaceOneOf { candidates } if candidates.is_empty() => None,
            Fix::AddOneOf { attrs, .. } if attrs.is_empty() => None,
            _ => Some(self),
        }
    }

    /// Drop a substitution proposal when the value it would replace
    /// was assembled by a preprocessor.
    ///
    /// `replace_with` / `replace_one_of` are both defined as replacing
    /// the value in `actual` (§3.1), which presumes the consumer can
    /// find that value in the document `location` names. For a
    /// synthesised value it is not there: the authored row holds the
    /// template's parameterised shape, and substituting into *that*
    /// rewrites every expansion rather than the failing one. The
    /// record still carries `expanded_from`, so the consumer is not
    /// left guessing — it is told the value is synthetic and where
    /// its parameters came from, which is the honest form of "no
    /// local edit repairs this" (§3).
    ///
    /// The other variants are untouched: they name an element and an
    /// attribute rather than a replacement for `actual`, so a
    /// synthesised value does not make them unperformable.
    fn applicable_to_a_synthesised_value(self, synthesised: bool) -> Option<Fix> {
        if !synthesised {
            return Some(self);
        }
        match self {
            Fix::ReplaceWith { .. } | Fix::ReplaceOneOf { .. } => None,
            other => Some(other),
        }
    }
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

/// An expression the ECMAScript frontend refused, reported without
/// stopping the build.
///
/// The payload is the inner [`ExprError`]'s — the same `expression/*`
/// code, `actual` and key fragments a fatal rejection of the same
/// construct carries, so one construct has one wire identity whichever
/// dialect met it. What differs is only when the record is emitted:
/// the spec obliges the generated machine to raise `error.execution`
/// rather than be refused at build time, so this record accompanies a
/// successful run instead of replacing one.
///
/// `exit_code` is the expression stage's, unused on the reporting path
/// and read only when `--lint` promotes the refusal to fatal — an
/// authored document has no §5.9.1 excuse for carrying one.
impl ToDiagnostics for crate::ecmascript_acceptance::RefusedExpression {
    fn exit_code(&self) -> i32 {
        ForgeError::Expression(ExprError::UnsupportedConstruct {
            construct: String::new(),
        })
        .exit_code()
    }

    fn to_diagnostics(&self) -> Vec<Diagnostic> {
        vec![self.to_single_diagnostic()]
    }
}

impl SingleDiagnostic for crate::ecmascript_acceptance::RefusedExpression {
    fn diagnostic_payload(&self) -> DiagnosticPayload {
        expression_fields(&self.error)
    }

    fn diagnostic_location(&self) -> Option<Location> {
        self.location.as_ref().map(|at| Location {
            file: at.file.clone(),
            line: at.line,
            col: at.col,
        })
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

    fn diagnostic_expanded_from(&self) -> Option<Location> {
        self.expanded_from.as_ref().map(|at| Location {
            file: at.file.clone(),
            line: at.line,
            col: at.col,
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
        XmlError::Parse(_) => DiagnosticPayload {
            code: DiagnosticCode::XmlParse,
            stage: Stage::Xml,
            expected: None,
            actual: None,
            fix: None,
            // No fragments. The only value this variant carries is the
            // XML engine's own parse text, and a key fragment has to be
            // a value SCE determined: engine text differs between
            // roxmltree here and pugixml in the C++ runtime parser, and
            // shifts when either is upgraded, so hashing it makes `id`
            // unreproducible for the same document across producers and
            // across dependency bumps. `code|stage|file` identifies the
            // failure on its own — parsing stops at the first error, so
            // a document has one of these.
            key_fragments: Vec::new(),
        },
        XmlError::SchemaValidation(_) => DiagnosticPayload {
            code: DiagnosticCode::XmlSchemaValidation,
            stage: Stage::Xml,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: Vec::new(),
        },
        // Top-level parser-entry errors (§wire-W4 α-strict). `actual`
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
        // consumers can dispatch without text parsing. `actual` carries
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
        XmlError::XInclude(XIncludeError::Malformed { href, .. }) => DiagnosticPayload {
            code: DiagnosticCode::XmlXIncludeMalformed,
            stage: Stage::Xml,
            expected: None,
            actual: Some(href.clone()),
            fix: None,
            // `detail` is the XML engine's parse text and stays out of
            // the key for the reason `XmlError::Parse` carries none —
            // the href identifies which include failed, which is what a
            // consumer dedups on.
            key_fragments: vec![href.clone()],
        },
        XmlError::XInclude(XIncludeError::Unsupported { href, feature }) => DiagnosticPayload {
            code: DiagnosticCode::XmlXIncludeUnsupported,
            stage: Stage::Xml,
            expected: None,
            actual: Some(feature.clone()),
            fix: None,
            key_fragments: vec![href.clone(), feature.clone()],
        },
        // `sce:template` failure modes. Parallel to XInclude: the
        // leaf variant drives the code so consumers can dispatch
        // without parsing text; `actual` carries the offending
        // template path (or parameter name, for the param-shaped
        // variants) so repair bots can act without re-parsing the
        // message; `key_fragments` tie into the `id` hash.
        XmlError::Template(TemplateError::NotFound { template, searched }) => DiagnosticPayload {
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
        XmlError::Template(TemplateError::Malformed { template, .. }) => DiagnosticPayload {
            code: DiagnosticCode::XmlTemplateMalformed,
            stage: Stage::Xml,
            expected: None,
            actual: Some(template.clone()),
            fix: None,
            // `detail` is SCE-authored for most of this variant's
            // branches but is the XML engine's parse text for two of
            // them, and a variant's key shape has to hold for every
            // instance of it — so the template name is the key, and the
            // reason travels in `message`.
            key_fragments: vec![template.clone()],
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
        // No `fix`: the repair is a change to how the caller drives the
        // pipeline, not an edit to the document the diagnostic points
        // at. Naming the surviving element in `actual` is what lets an
        // consumer tell the two directives apart without re-reading the
        // source.
        XmlError::PreprocessorNotRun { element } => DiagnosticPayload {
            code: DiagnosticCode::XmlPreprocessorNotRun,
            stage: Stage::Xml,
            expected: None,
            actual: Some(element.clone()),
            fix: None,
            key_fragments: vec![element.clone()],
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
            // The list is authoritative, so consumers get a structured
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
        ValidationError::DuplicateRequirementId { element, id } => DiagnosticPayload {
            code: DiagnosticCode::ValidationDuplicateRequirementId,
            stage: Stage::Validation,
            expected: None,
            actual: Some(id.clone()),
            // No closed candidate set — the repair is "drop the
            // duplicate token". `Fix::None` carries that via the
            // `NeutralOrDeterministic` non-overlap class.
            fix: None,
            key_fragments: vec![element.clone(), id.clone()],
        },
        ValidationError::UnresolvedPlaceholder {
            element,
            id,
            reason,
        } => DiagnosticPayload {
            code: DiagnosticCode::ValidationUnresolvedPlaceholder,
            stage: Stage::Validation,
            expected: None,
            actual: Some(id.clone()),
            fix: None,
            key_fragments: {
                let mut k = vec![element.clone(), id.clone()];
                if let Some(r) = reason {
                    k.push(r.clone());
                }
                k
            },
        },
        ValidationError::CrossKindFieldNotFound {
            importing_kind,
            importing_name,
            alias,
            field,
            imported_kind,
            imported_name,
            candidates,
        } => DiagnosticPayload {
            code: DiagnosticCode::ValidationCrossKindFieldNotFound,
            stage: Stage::Validation,
            expected: None,
            // `actual` carries the dotted source spelling so the
            // diagnostic message reproduces the offending token
            // verbatim; the structured candidate list lives on `fix`.
            actual: Some(format!("{alias}.{field}")),
            fix: Some(Fix::ReplaceOneOf {
                // Candidates are pre-sorted by the caller (the symbol
                // table builder returns a deduplicated `Vec`); when the
                // imported kind exposes zero fields the list is empty
                // and the wire still carries a (degenerate) closed set
                // — `Fix::ReplaceOneOf { candidates: [] }` is honest
                // about there being no legal replacement.
                candidates: candidates.iter().map(|c| format!("{alias}.{c}")).collect(),
            }),
            key_fragments: vec![
                importing_kind.to_string(),
                importing_name.clone(),
                alias.clone(),
                field.clone(),
                imported_kind.to_string(),
                imported_name.clone(),
            ],
        },
        ValidationError::CrossKindTypeMismatch {
            importing_kind,
            importing_name,
            alias,
            field,
            actual,
            expected,
        } => DiagnosticPayload {
            code: DiagnosticCode::ValidationCrossKindTypeMismatch,
            stage: Stage::Validation,
            expected: Some(vec![expected.clone()]),
            actual: Some(actual.clone()),
            fix: None,
            key_fragments: vec![
                importing_kind.to_string(),
                importing_name.clone(),
                alias.clone(),
                field.clone(),
            ],
        },
        ValidationError::CrossKindCircularDependency { cycle } => DiagnosticPayload {
            code: DiagnosticCode::ValidationCrossKindCircularDependency,
            stage: Stage::Validation,
            expected: None,
            actual: Some(cycle.join(" → ")),
            fix: None,
            // The cycle path itself is the canonical identity — two
            // distinct cycles in the same build will have different
            // `cycle` vectors and thus different IDs.
            key_fragments: cycle.clone(),
        },
        ValidationError::QuantityUnitMismatch {
            kind,
            name,
            op,
            left_unit,
            right_unit,
            expr,
        } => DiagnosticPayload {
            // NL→IR Mapping Roadmap Item 4 — reuse the same
            // DiagnosticCode as cross-kind type-mismatch (concept
            // identity: "two values whose types are incompatible meet
            // in an expression"). The typed `ValidationError` variant
            // diverges so the payload renders the right *kind* of
            // mismatch without burning a new slot in the
            // DiagnosticCode enum.
            code: DiagnosticCode::ValidationCrossKindTypeMismatch,
            stage: Stage::Validation,
            expected: Some(vec![left_unit.clone()]),
            actual: Some(right_unit.clone()),
            fix: None,
            key_fragments: vec![
                kind.to_string(),
                name.clone(),
                op.clone(),
                left_unit.clone(),
                right_unit.clone(),
                expr.clone(),
            ],
        },
        ValidationError::DuplicateContextObject { id } => DiagnosticPayload {
            code: DiagnosticCode::ValidationDuplicateContextObject,
            stage: Stage::Validation,
            expected: None,
            actual: Some(id.clone()),
            // `<sce:context>` is a document-wide scope, so the repair
            // surface is identical to any other duplicate id — rename
            // one of the declarations. `what` names the namespace so
            // consumers can disambiguate from state/field/event id reuse.
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
            // `actual` carries the offending expression so consumers can
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
        ValidationError::WrongPipeline { kind, .. } => DiagnosticPayload {
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
            // `actual` carries the specific blocker so consumers route
            // between Interpreter fallback (dynamic invoke) and
            // document rewrite (missing initial); no closed candidate
            // set exists for the repair, so `fix` stays `None`.
            expected: None,
            actual: Some(reason.clone()),
            fix: None,
            key_fragments: vec![name.clone(), reason.clone()],
        },
        ValidationError::NativeActionPlacement { name, detail } => DiagnosticPayload {
            code: DiagnosticCode::ValidationNativeActionPlacement,
            stage: Stage::Validation,
            // The repair is to MOVE the element to a `<transition>`, not to
            // substitute a value, so no closed candidate set exists and `fix`
            // stays `None`; `actual` carries the offending placement detail.
            expected: None,
            actual: Some(detail.clone()),
            fix: None,
            key_fragments: vec![name.clone(), detail.clone()],
        },
        ValidationError::NativeActionArgument { name, detail } => DiagnosticPayload {
            code: DiagnosticCode::ValidationNativeActionArgument,
            stage: Stage::Validation,
            expected: None,
            actual: Some(detail.clone()),
            fix: None,
            key_fragments: vec![name.clone(), detail.clone()],
        },
        ValidationError::NativeActionSignatureConflict { name, detail } => DiagnosticPayload {
            code: DiagnosticCode::ValidationNativeActionSignatureConflict,
            stage: Stage::Validation,
            expected: None,
            actual: Some(detail.clone()),
            fix: None,
            key_fragments: vec![name.clone(), detail.clone()],
        },
        ValidationError::MeshRpcReservedParam { param, detail } => DiagnosticPayload {
            code: DiagnosticCode::ValidationMeshRpcReservedParam,
            stage: Stage::Validation,
            // `actual` carries the offending reserved param name so
            // consumers can locate the element to repair; `detail` flows
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
            // its `sce:` prefix) so consumers can target the exact
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
        ValidationError::AlgorithmForeachSourceNotIterable { src, candidates } => {
            DiagnosticPayload {
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
            }
        }
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
        ValidationError::AlgorithmAppendTargetNotBuffer { target, candidates } => {
            DiagnosticPayload {
                code: DiagnosticCode::AlgorithmAppendTargetNotBuffer,
                stage: Stage::Validation,
                // Multi-axis repair (declare a `<sce:var type="bytes">` OR
                // retarget an existing buffer); the visible buffer-name
                // union rides `key_fragments` for content-hash stability,
                // no single `Fix::Replace`. Mirrors
                // `AlgorithmForeachSourceNotIterable`.
                expected: None,
                actual: Some(target.clone()),
                fix: None,
                key_fragments: {
                    let mut k = vec![target.clone()];
                    k.extend(candidates.iter().cloned());
                    k
                },
            }
        }
        ValidationError::AlgorithmAppendTypeMismatch { target, got } => DiagnosticPayload {
            code: DiagnosticCode::AlgorithmAppendTypeMismatch,
            stage: Stage::Validation,
            // The accepted RHS type set is fixed metadata, not a
            // structured repair candidate; narrowing is author-domain.
            expected: Some(vec!["uint8".into(), "bytes".into()]),
            actual: Some(got.clone()),
            fix: None,
            key_fragments: vec![target.clone(), got.clone()],
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
        ValidationError::CodecVariantArmMidMismatch {
            codec,
            arm_value,
            inner_codec,
            inner_flag,
            inner_flag_value,
        } => DiagnosticPayload {
            code: DiagnosticCode::CodecVariantArmMidMismatch,
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
        ValidationError::CodecVariantNoDefaultArm { codec } => DiagnosticPayload {
            code: DiagnosticCode::CodecVariantNoDefaultArm,
            stage: Stage::Validation,
            // Repair is "add default=\"true\" to the intended arm" —
            // author-domain (which arm is the intended default), not
            // a closed candidate set, so `fix` stays `None`. The
            // codec name alone keys the violation; one variant per
            // codec means at most one occurrence per source.
            expected: None,
            actual: Some(codec.clone()),
            fix: None,
            key_fragments: vec![codec.clone()],
        },
        ValidationError::CodecVariantDefaultOverlayArmNotDeclared {
            codec,
            overlay_arm_value,
            declared_arms,
        } => DiagnosticPayload {
            code: DiagnosticCode::CodecVariantDefaultOverlayArmNotDeclared,
            stage: Stage::Validation,
            // Repair is "pick one of the declared arm values for the
            // overlay entry" — closed candidate set (FixCarriesCandidates
            // class). The candidate axis is the codec's declared arm
            // values sorted, surfaced via `fix: Some(ReplaceOneOf)` so
            // tooling can offer them as a typed pick list. Empty list
            // when the codec has no variant at all — in that case the
            // candidate set degenerates to "remove the overlay entry".
            expected: Some(vec![format!("{overlay_arm_value:#x}")]),
            actual: Some(codec.clone()),
            fix: if declared_arms.is_empty() {
                None
            } else {
                Some(Fix::ReplaceOneOf {
                    candidates: declared_arms.iter().map(|v| format!("{v:#x}")).collect(),
                })
            },
            key_fragments: vec![codec.clone(), format!("{overlay_arm_value:#x}")],
        },
        ValidationError::CodecVariantDispatchFlagNotResolved {
            parent_codec,
            embedded_alias,
            flag_source,
            detail: _,
            candidates,
        } => DiagnosticPayload {
            code: DiagnosticCode::CodecVariantDispatchFlagNotResolved,
            stage: Stage::Validation,
            expected: Some(vec![flag_source.clone()]),
            actual: Some(parent_codec.clone()),
            fix: if candidates.is_empty() {
                None
            } else {
                Some(Fix::ReplaceOneOf {
                    candidates: candidates.clone(),
                })
            },
            key_fragments: vec![
                parent_codec.clone(),
                embedded_alias.clone(),
                flag_source.clone(),
            ],
        },
        ValidationError::CodecVariantDispatchBitWidthMismatch {
            parent_codec,
            embedded_alias,
            embedded_codec,
            carrier,
            flag,
            flag_width,
            max_values,
            arm_count,
        } => DiagnosticPayload {
            code: DiagnosticCode::CodecVariantDispatchBitWidthMismatch,
            stage: Stage::Validation,
            expected: Some(vec![format!("flag width ≥ ceil(log2({arm_count}))")]),
            actual: Some(format!(
                "width={flag_width} (max {max_values} values) vs {arm_count} arms"
            )),
            fix: None,
            key_fragments: vec![
                parent_codec.clone(),
                embedded_alias.clone(),
                embedded_codec.clone(),
                carrier.clone(),
                flag.clone(),
            ],
        },
        ValidationError::CodecVariantDispatchArmsNotDistinguishableWithoutDefault {
            parent_codec,
            embedded_alias,
            embedded_codec,
        } => DiagnosticPayload {
            code: DiagnosticCode::CodecVariantDispatchArmsNotDistinguishableWithoutDefault,
            stage: Stage::Validation,
            expected: Some(vec!["<sce:variant-dispatch flag=\"...\"/> on the import \
                 OR <sce:arm default=\"true\"/> in the imported codec"
                .into()]),
            actual: Some(embedded_codec.clone()),
            fix: None,
            key_fragments: vec![
                parent_codec.clone(),
                embedded_alias.clone(),
                embedded_codec.clone(),
            ],
        },
        ValidationError::CodecVariantDispatchFlagHasStaticValue {
            parent_codec,
            embedded_alias,
            carrier,
            flag,
            static_value,
        } => DiagnosticPayload {
            code: DiagnosticCode::CodecVariantDispatchFlagHasStaticValue,
            stage: Stage::Validation,
            expected: None,
            actual: Some(format!("{parent_codec}.{carrier}.{flag}={static_value:#x}")),
            fix: None,
            key_fragments: vec![
                parent_codec.clone(),
                embedded_alias.clone(),
                carrier.clone(),
                flag.clone(),
            ],
        },
        ValidationError::CodecVariantDispatchCarrierAfterEmbed {
            parent_codec,
            embedded_alias,
            embedded_field,
            carrier,
            flag,
            carrier_index,
            embedded_index,
        } => DiagnosticPayload {
            code: DiagnosticCode::CodecVariantDispatchCarrierAfterEmbed,
            stage: Stage::Validation,
            expected: Some(vec![format!(
                "carrier '{carrier}' before field '{embedded_field}'"
            )]),
            actual: Some(format!(
                "carrier at index {carrier_index}, embed at index {embedded_index}"
            )),
            fix: None,
            key_fragments: vec![
                parent_codec.clone(),
                embedded_alias.clone(),
                embedded_field.clone(),
                carrier.clone(),
                flag.clone(),
            ],
        },
        ValidationError::CodecFlagBindInputNotDeclared {
            parent_codec,
            embedded_alias,
            embedded_codec,
            input,
            available_inputs,
        } => DiagnosticPayload {
            code: DiagnosticCode::CodecFlagBindInputNotDeclared,
            stage: Stage::Validation,
            expected: Some(vec![input.clone()]),
            actual: Some(format!(
                "declared on {embedded_codec}: [{available_inputs}]"
            )),
            fix: if available_inputs.is_empty() {
                None
            } else {
                Some(Fix::ReplaceOneOf {
                    candidates: available_inputs
                        .split(", ")
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string())
                        .collect(),
                })
            },
            key_fragments: vec![parent_codec.clone(), embedded_alias.clone(), input.clone()],
        },
        ValidationError::CodecFlagBindSourceNotResolved {
            parent_codec,
            embedded_alias,
            input,
            bind_source,
            detail: _,
        } => DiagnosticPayload {
            code: DiagnosticCode::CodecFlagBindSourceNotResolved,
            stage: Stage::Validation,
            expected: Some(vec![bind_source.clone()]),
            actual: Some(parent_codec.clone()),
            fix: None,
            key_fragments: vec![
                parent_codec.clone(),
                embedded_alias.clone(),
                input.clone(),
                bind_source.clone(),
            ],
        },
        ValidationError::CodecFlagBindWidthMismatch {
            parent_codec,
            embedded_alias,
            input,
            bind_source,
            source_width,
            input_width,
        } => DiagnosticPayload {
            code: DiagnosticCode::CodecFlagBindWidthMismatch,
            stage: Stage::Validation,
            expected: Some(vec![format!("input width {input_width}")]),
            actual: Some(format!("source width {source_width}")),
            fix: None,
            key_fragments: vec![
                parent_codec.clone(),
                embedded_alias.clone(),
                input.clone(),
                bind_source.clone(),
            ],
        },
        ValidationError::CodecFlagInputUnbound {
            parent_codec,
            embedded_alias,
            embedded_codec,
            input,
        } => DiagnosticPayload {
            code: DiagnosticCode::CodecFlagInputUnbound,
            stage: Stage::Validation,
            expected: Some(vec![format!(
                "<sce:flag-bind input=\"{input}\" source=\"...\"/>"
            )]),
            actual: Some(format!(
                "{embedded_codec} declares <sce:flag-input name=\"{input}\"/>"
            )),
            fix: Some(Fix::AddAttribute {
                element: format!("<sce:import as=\"{embedded_alias}\">"),
                attr: format!("<sce:flag-bind input=\"{input}\" source=\"...\"/>"),
            }),
            key_fragments: vec![parent_codec.clone(), embedded_alias.clone(), input.clone()],
        },
        ValidationError::CodecFlagBindDuplicateInput {
            parent_codec,
            embedded_alias,
            input,
        } => DiagnosticPayload {
            code: DiagnosticCode::CodecFlagBindDuplicateInput,
            stage: Stage::Validation,
            expected: None,
            actual: Some(format!("{embedded_alias}.{input}")),
            fix: None,
            key_fragments: vec![parent_codec.clone(), embedded_alias.clone(), input.clone()],
        },
        ValidationError::CodecFlagBindCarrierAfterEmbed {
            parent_codec,
            embedded_alias,
            embedded_field,
            input,
            carrier,
            flag,
            carrier_index,
            embedded_index,
        } => DiagnosticPayload {
            code: DiagnosticCode::CodecFlagBindCarrierAfterEmbed,
            stage: Stage::Validation,
            expected: Some(vec![format!(
                "carrier '{carrier}' before field '{embedded_field}'"
            )]),
            actual: Some(format!(
                "carrier at index {carrier_index}, embed at index {embedded_index}"
            )),
            fix: None,
            key_fragments: vec![
                parent_codec.clone(),
                embedded_alias.clone(),
                embedded_field.clone(),
                input.clone(),
                carrier.clone(),
                flag.clone(),
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
        ValidationError::CodecVariantArmBodyCallerTagUnsupported {
            parent_codec,
            arm_value,
            embedded_alias,
            embedded_codec,
        } => DiagnosticPayload {
            code: DiagnosticCode::CodecVariantArmBodyCallerTagUnsupported,
            stage: Stage::Validation,
            // Repair is structural — either add `tag=` to the inner
            // codec or move the import from variant arm to <sce:embed>.
            // Both alternatives are deterministic for the author, so
            // `fix: None` per NeutralOrDeterministic class.
            //
            // `arm_value` is `Some(v)` for enumerated arms and `None`
            // for the catch-all `<sce:default>` arm. The expected[]
            // slot renders `<default>` so authors with both a
            // value=0x00 arm AND a default arm see two distinct
            // diagnostics (key_fragments preserves the disambiguation
            // for the FNV1a id too).
            expected: Some(vec![arm_value
                .map(|v| format!("{v:#x}"))
                .unwrap_or_else(|| "<default>".to_string())]),
            actual: Some(embedded_codec.clone()),
            fix: None,
            key_fragments: vec![
                parent_codec.clone(),
                arm_value
                    .map(|v| format!("{v:#x}"))
                    .unwrap_or_else(|| "<default>".to_string()),
                embedded_alias.clone(),
                embedded_codec.clone(),
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
        ValidationError::CodecTlvChainTruncateUnderEntryFlag { codec, field } => {
            DiagnosticPayload {
                code: DiagnosticCode::CodecTlvChainTruncateUnderEntryFlag,
                stage: Stage::Validation,
                // The offending value is the `on-overflow` the author
                // wrote, and its repair is single-valued: `reject` is the
                // only policy that can hold under entry-flag termination.
                // Switching `terminate-on` instead redeclares what the
                // wire is, rather than repairing this declaration, so it
                // stays in the message and out of the fix.
                expected: None,
                actual: Some("truncate".to_string()),
                fix: Some(Fix::ReplaceWith {
                    to: "reject".to_string(),
                }),
                key_fragments: vec![codec.clone(), field.clone()],
            }
        }
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
        ValidationError::CodecPeekByteFlagLayoutMismatch {
            body_codec,
            parent_codec,
            reason: _,
        } => DiagnosticPayload {
            code: DiagnosticCode::CodecPeekByteFlagLayoutMismatch,
            stage: Stage::Validation,
            // Repair is structural (align one side's peek-byte vs body
            // flag layout); no closed candidate set. (body_codec,
            // parent_codec) keys the violation across multiple
            // peek-byte arm wire-ups.
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
            // Closed-enum candidate set (RFC §synth-5-C lines 765-771) —
            // emit `Fix::ReplaceOneOf` so consumers can mechanically
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
            // class admits per RFC §synth-5-C lines 765-771) — emit
            // `Fix::ReplaceOneOf` so consumers can mechanically pick a
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
        ValidationError::LinkPoolRefNotDeclared {
            link_name,
            pool_side,
            pool_ref,
            candidates,
        } => DiagnosticPayload {
            code: DiagnosticCode::LinkPoolRefNotDeclared,
            stage: Stage::Validation,
            // Closed candidate set — the build's declared buffer-pool
            // names — so the repair is machine-applicable. `actual`
            // carries the ref as written, which is what an author
            // greps for. The side rides `key_fragments` rather than
            // `actual` so one link declaring the same bad name on two
            // sides yields two distinct ids.
            expected: None,
            actual: Some(pool_ref.clone()),
            fix: Some(Fix::ReplaceOneOf {
                candidates: candidates.clone(),
            }),
            key_fragments: vec![
                link_name.clone(),
                (*pool_side).to_string(),
                pool_ref.clone(),
            ],
        },
        ValidationError::LinkFramerRefNotDeclared {
            link_name,
            framer_ref,
            candidates,
        } => DiagnosticPayload {
            code: DiagnosticCode::LinkFramerRefNotDeclared,
            stage: Stage::Validation,
            // Same shape as the pool-ref arm above: a closed candidate
            // set — the build's declared codec names — makes the repair
            // machine-applicable, and `actual` carries the ref as
            // written because that is the string an author greps for.
            // No side fragment here: a link declares exactly one
            // framer, so the link name and the ref already identify the
            // site uniquely.
            expected: None,
            actual: Some(framer_ref.clone()),
            fix: Some(Fix::ReplaceOneOf {
                candidates: candidates.clone(),
            }),
            key_fragments: vec![link_name.clone(), framer_ref.clone()],
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
            // carries the region-name axis so consumers can mechanically
            // pick a legal section. The author can alternately extend
            // the deploy.yaml memory map; the message prose names
            // both repair surfaces. RFC §synth-5-E lines 1000-1023 + 1537.
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
        ValidationError::DeployDcacheLineSizeNotPowerOfTwo {
            machine,
            dcache_line_size,
            previous_power,
            next_power,
        } => DiagnosticPayload {
            code: DiagnosticCode::MemDcacheLineSizeNotPowerOfTwo,
            stage: Stage::Validation,
            // Closed two-element repair, so the candidates ride the
            // fix rather than the prose — same shape as the two
            // §synth-5-E slot-table rules. Which one is right is a
            // datasheet question, not one codegen can settle.
            expected: None,
            actual: Some(dcache_line_size.to_string()),
            fix: Some(Fix::ReplaceOneOf {
                candidates: vec![previous_power.to_string(), next_power.to_string()],
            }),
            key_fragments: vec![
                machine.clone(),
                dcache_line_size.to_string(),
                previous_power.to_string(),
                next_power.to_string(),
            ],
        },
        ValidationError::BufferPoolAlignmentNotPowerOfTwo {
            name,
            alignment,
            previous_power,
            next_power,
        } => DiagnosticPayload {
            code: DiagnosticCode::MemAlignmentNotPowerOfTwo,
            stage: Stage::Validation,
            // `Fix::ReplaceOneOf` rather than prose: the repair is a
            // closed two-element choice, and which of the two is right
            // is the author's call — rounding down keeps the memory
            // budget, rounding up keeps the boundary the peripheral
            // may actually need.
            expected: None,
            actual: Some(alignment.to_string()),
            fix: Some(Fix::ReplaceOneOf {
                candidates: vec![previous_power.to_string(), next_power.to_string()],
            }),
            key_fragments: vec![
                name.clone(),
                alignment.to_string(),
                previous_power.to_string(),
                next_power.to_string(),
            ],
        },
        ValidationError::BufferPoolSlotSizeNotAlignmentMultiple {
            name,
            slot_size,
            alignment,
            remainder,
            previous_multiple,
            next_multiple,
        } => DiagnosticPayload {
            code: DiagnosticCode::MemSlotSizeNotAlignmentMultiple,
            stage: Stage::Validation,
            expected: None,
            actual: Some(slot_size.to_string()),
            // Same shape as the alignment repair above, and for the
            // same reason: rounding down keeps the SRAM budget,
            // rounding up keeps the authored payload capacity.
            fix: Some(Fix::ReplaceOneOf {
                candidates: vec![previous_multiple.to_string(), next_multiple.to_string()],
            }),
            key_fragments: vec![
                name.clone(),
                slot_size.to_string(),
                alignment.to_string(),
                remainder.to_string(),
                previous_multiple.to_string(),
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
            key_fragments: vec![name.clone(), machine.clone(), declared_policy.clone()],
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
        ValidationError::ScxmlUnknownSessionRoleKind { kind, allowed } => DiagnosticPayload {
            // Listener-role — closed-set kind vocabulary
            // rides `Fix::ReplaceOneOf` per FixCarriesCandidates
            // non-overlap class. `expected` stays None (the candidate
            // list lives in fix, not expected, per the non-overlap
            // contract).
            code: DiagnosticCode::ScxmlUnknownSessionRoleKind,
            stage: Stage::Validation,
            expected: None,
            actual: Some(kind.clone()),
            fix: Some(Fix::ReplaceOneOf {
                candidates: allowed.clone(),
            }),
            key_fragments: vec![kind.clone()],
        },
        ValidationError::ScxmlDuplicateSessionRoleDeclaration { kind } => DiagnosticPayload {
            // Listener-role — single-axis repair (delete
            // the duplicate); no closed candidate list. `actual`
            // surfaces the offending kind so CLI consumers see which
            // kind was duplicated without parsing the message body.
            code: DiagnosticCode::ScxmlDuplicateSessionRoleDeclaration,
            stage: Stage::Validation,
            expected: None,
            actual: Some(kind.clone()),
            fix: None,
            key_fragments: vec![kind.clone()],
        },
        ValidationError::LinkDeployRoleListenerWithoutScxmlAcceptSideRole {
            machine,
            link_name,
        } => {
            DiagnosticPayload {
                // Listener-role — typed partial-claim.
                // 2-axis repair (add SCXML role OR remove deploy role);
                // NeutralOrDeterministic.
                // `actual` carries the link name; `key_fragments`
                // mirror the `reassembly/binding-on-unpaired-
                // listener` `(machine, link_name)` shape so external
                // consumers can join diagnostic streams without
                // re-parsing.
                code: DiagnosticCode::LinkDeployRoleListenerWithoutScxmlAcceptSideRole,
                stage: Stage::Validation,
                expected: None,
                actual: Some(link_name.clone()),
                fix: None,
                key_fragments: vec![machine.clone(), link_name.clone()],
            }
        }
        ValidationError::ScxmlAcceptSideRoleWithoutListenerLink {
            machine,
            scxml_source,
        } => DiagnosticPayload {
            // Listener-role — typed partial-claim mirror
            // direction. 2-axis repair. `actual` carries the SCXML
            // source basename so consumers can navigate to the
            // offending file; `key_fragments` carry both fields.
            code: DiagnosticCode::ScxmlAcceptSideRoleWithoutListenerLink,
            stage: Stage::Validation,
            expected: None,
            actual: Some(scxml_source.clone()),
            fix: None,
            key_fragments: vec![machine.clone(), scxml_source.clone()],
        },
        ValidationError::LinkRoleListenerWithNonSessionArmingTrustClass {
            machine,
            link_name,
            trust_class,
        } => DiagnosticPayload {
            // Listener-role — role × trust-class matrix check.
            // 2-axis repair (change trust_class OR remove role);
            // NeutralOrDeterministic. `actual` carries the wrong
            // trust_class wire-form value so the violation is visible
            // without re-reading deploy.yaml.
            code: DiagnosticCode::LinkRoleListenerWithNonSessionArmingTrustClass,
            stage: Stage::Validation,
            expected: None,
            actual: Some(trust_class.clone()),
            fix: None,
            key_fragments: vec![machine.clone(), link_name.clone(), trust_class.clone()],
        },
        ValidationError::ScxmlAcceptSideStatesWithoutRoleDeclaration { offending_ids } => {
            DiagnosticPayload {
                // Listener-role migration-helper. 2-axis
                // repair (add role declaration OR rename states); no
                // closed candidate set so `fix: None`. `actual`
                // serializes the offending id list in document order
                // — joined with comma so the wire format is a single
                // string (per the DiagnosticPayload `actual: Option<
                // String>` contract).
                code: DiagnosticCode::ScxmlAcceptSideStatesWithoutRoleDeclaration,
                stage: Stage::Validation,
                expected: None,
                actual: Some(offending_ids.join(",")),
                fix: None,
                key_fragments: offending_ids.clone(),
            }
        }
        ValidationError::ReassemblyPerPeerQuotaBuildInvariantViolated {
            pool_name,
            slot_count,
            machine,
            link_name,
            peer_table_capacity,
            per_peer_quota,
            product,
        } => DiagnosticPayload {
            // Declared-consumption — peer_table.capacity × per_peer_quota
            // >= slot_count. 3-axis repair; NeutralOrDeterministic.
            // `actual` carries the violating product so consumers can
            // see the shortfall without parsing the message body.
            code: DiagnosticCode::ReassemblyPerPeerQuotaBuildInvariantViolated,
            stage: Stage::Validation,
            expected: None,
            actual: Some(format!(
                "{product} < {slot_count}",
                product = product,
                slot_count = slot_count
            )),
            fix: None,
            key_fragments: vec![
                pool_name.clone(),
                machine.clone(),
                link_name.clone(),
                slot_count.to_string(),
                peer_table_capacity.to_string(),
                per_peer_quota.to_string(),
            ],
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
        // ── §synth-5-D worker cross-resolution: link-rx + outbox ref ──
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
            // `Fix::ReplaceOneOf`. Precedent: `LinkClassUnsupportedOnTarget`.
            expected: None,
            actual: Some(ref_name.clone()),
            fix: Some(Fix::ReplaceOneOf {
                candidates: candidates.clone(),
            }),
            key_fragments: vec![worker_name.clone(), ref_name.clone()],
        },
        // ── §synth-5-I SPSC inbox ordering ──
        ValidationError::WorkerInboxOrderingUnspecified { worker_name } => DiagnosticPayload {
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
            // Forge-side anchor for spec §synth-5-D line 912. Per-doc miss
            // against the resolved machine's `workers` map.
            // NeutralOrDeterministic — author either declares the
            // worker in deploy.yaml or removes the Worker doc.
            expected: None,
            actual: Some(worker_name.clone()),
            fix: None,
            key_fragments: vec![worker_name.clone(), machine.clone()],
        },
        // ── §synth-5-D worker outbox cross-resolution ──
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
            // Precedent: `WorkerLinkRxRefUnknown` carries candidates
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
            key_fragments: vec![worker_name.clone(), owner.clone(), actual_kind.clone()],
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
            // carries `{owner}.inbox` so a consumer applies the fix
            // without rewriting the prefix. NeutralOrDeterministic
            // non_overlap_class.
            expected: None,
            actual: Some(outbox_value.clone()),
            fix: Some(Fix::ReplaceWith {
                to: format!("{owner}.inbox"),
            }),
            key_fragments: vec![worker_name.clone(), owner.clone(), suffix.clone()],
        },
        // ── §synth-5-L Bounded-collection parse-time structure validators (item C6) ──
        ValidationError::CollectionOrderingSortedRequiresIndexBy { collection_name } => {
            DiagnosticPayload {
                code: DiagnosticCode::CollectionOrderingSortedRequiresIndexBy,
                stage: Stage::Validation,
                // No closed candidate set — the repair requires authoring a
                // field name from the element-type struct, which is author-
                // domain knowledge (cf. non_overlap_class entry's reasoning).
                expected: None,
                actual: None,
                fix: None,
                key_fragments: vec![collection_name.clone()],
            }
        }
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
        // ── §synth-5-M Fragment-reassembly variant parse-time structure validators (item C9) ──
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
        // ── §synth-5-M Fragment-reassembly cross-doc validators (items C9 + C13) ──
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
            // 25% threshold (default per spec, suppressible via §synth-5-K
            // `pool_defaults.stage_copy_policy`).
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
            // the second valid repair "remove
            // binding entirely" stays in scope so the code stays
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
            key_fragments: vec![pool_name.clone(), machine.clone(), link_name.clone()],
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
        // ── §synth-5-M reassembly codegen self-check ──
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
        // ── §synth-5-C listener-pair codegen self-check ──
        ValidationError::LinkListenerLinkNotPairedWithEstablishedSibling {
            link_name,
            language,
        } => DiagnosticPayload {
            code: DiagnosticCode::LinkListenerLinkNotPairedWithEstablishedSibling,
            stage: Stage::Validation,
            // Codegen-internal invariant — same shape as
            // `ReassemblyPeerIdNotZidOnEstablishedSession`. Author-
            // side `actual` / `expected` / `fix` carry no useful
            // information.
            actual: None,
            expected: None,
            fix: None,
            key_fragments: vec![link_name.clone(), language.clone()],
        },
        // ── §synth-5-M reassembly-binding-on-unpaired-listener ──
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
            key_fragments: vec![pool_name.clone(), machine.clone(), link_name.clone()],
        },
        // ── §synth-5-N link/inbound-event-queue-unsized ──
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
        // ── §synth-5-K stage-copy policy promotion + opt-out rejection ──
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
        ValidationError::PoolStageCopyAcceptRejectedUnderForbid { machine, link_name } => {
            DiagnosticPayload {
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
            }
        }
        // ── §synth-5-L Bounded-collection cross-doc resolution (item C6) ──
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
            // `Fix::ReplaceOneOf`. Precedent: `WorkerOutboxRefUnknown`
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
        ValidationError::CollectionMultiWriterWithoutAtomics { collection_name } => {
            DiagnosticPayload {
                code: DiagnosticCode::CollectionMultiWriterWithoutAtomics,
                stage: Stage::Validation,
                // No closed candidate set — atomic family is too large
                // (100+ symbols across load/store/cas/fetch × widths ×
                // orderings) for `Fix::ReplaceOneOf` to be useful. Author
                // picks width + ordering + op from the §synth-5-I baseline per
                // their use case. NeutralOrDeterministic.
                expected: None,
                actual: None,
                fix: None,
                key_fragments: vec![collection_name.clone()],
            }
        }
        // ── §synth-5-L Bounded-collection deploy-time capacity resolution (item C6) ──
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
            key_fragments: vec![collection_name.clone(), machine.clone(), limit.clone()],
        },
        ValidationError::TimerPeriodBelowTickRate {
            timer_name,
            machine,
            period_us,
            tick_period_us,
        } => DiagnosticPayload {
            code: DiagnosticCode::TimerPeriodBelowTickRate,
            stage: Stage::Validation,
            // Forge-side anchor for spec §synth-5-D line 909. Per-doc check
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
        // ── §synth-5-I `<sce:extern>` whitelist rejection ──
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
        // ── §synth-5-I target-plugin baseline-shadowing ──
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
                // Repair is "rename the plugin entry";
                // SCE cannot synthesize a non-baseline name. The
                // diagnostic stays advisory (`fix: None`).
                fix: None,
                key_fragments: vec![name.clone(), plugin_path.clone()],
            }
        }
        // ── §synth-5-O IR provenance pre-emit guard ────────────────────
        //    Codegen-internal invariant: an IR node eligible for
        //    SCE-MAP marker emission reached the pre-emit walker
        //    with `source_location: None`. No author repair (the
        //    fix lives in the parser site that produced the node).
        //    `node_kind` + `node_id` ride `key_fragments` so the
        //    wire payload is uniquely keyed per offending parser
        //    site without leaking through `expected` / `actual`
        //    (which carry no useful author-facing data here).
        ValidationError::TraceabilityScxmlLineRangeMissing { node_kind, node_id } => {
            DiagnosticPayload {
                code: DiagnosticCode::TraceabilityScxmlLineRangeMissing,
                stage: Stage::Generate,
                expected: None,
                actual: None,
                fix: None,
                key_fragments: vec![(*node_kind).into(), node_id.clone()],
            }
        }
        // ── §synth-5-O symbol mangling collision detector. The
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
        // ── §synth-5-O — mangled symbol exceeds C99 identifier
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
            key_fragments: vec![mangled.clone(), actual_len.to_string(), over_by.to_string()],
        },
        // ── §synth-5-O — sourcemap source_hash drift against
        //    §synth-6.2.6 header. NeutralOrDeterministic: regenerate via
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
            key_fragments: vec![file.clone(), sourcemap_hash.clone(), header_hash.clone()],
        },
        // ── §synth-5-O — Rust SCE-MAP `#[doc]` preservation guard
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
            key_fragments: vec![crate_name.clone(), function.clone(), profile.clone()],
        },
        // ── §synth-5-O — codegen-internal traceability
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
        // ── MCU `<sce:driver href>` (SCE Protocol-Synthesis RFC §5.2) —
        //    resolution failure. `actual` carries the verbatim author-
        //    written href so the diagnostic message round-trips the
        //    original string; `key_fragments` include both href and
        //    resolved_dir so two identical-named misses under different
        //    search roots hash distinct wire-ids. Stage = Validation —
        //    the diagnostic fires before codegen, at compile-model
        //    time, matching the stage of similar
        //    compile-model-time codes.
        ValidationError::McuDriverHeaderNotFound { href, resolved_dir } => DiagnosticPayload {
            code: DiagnosticCode::McuDriverHeaderNotFound,
            stage: Stage::Validation,
            expected: None,
            actual: Some(href.clone()),
            fix: None,
            key_fragments: vec![href.clone(), resolved_dir.clone()],
        },
        // ── NL→IR Item C1 Path A: Enum kind invariants ──
        ValidationError::EnumNoVariants { name } => DiagnosticPayload {
            code: DiagnosticCode::ValidationEnumNoVariants,
            stage: Stage::Validation,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![name.clone()],
        },
        ValidationError::EnumVariantDuplicateName { enum_name, name } => DiagnosticPayload {
            code: DiagnosticCode::ValidationEnumVariantDuplicateName,
            stage: Stage::Validation,
            expected: None,
            actual: Some(name.clone()),
            fix: None,
            key_fragments: vec![enum_name.clone(), name.clone()],
        },
        ValidationError::EnumVariantDuplicateValue {
            enum_name,
            value,
            first_name,
            second_name,
        } => DiagnosticPayload {
            code: DiagnosticCode::ValidationEnumVariantDuplicateValue,
            stage: Stage::Validation,
            expected: None,
            actual: Some(value.to_string()),
            fix: None,
            key_fragments: vec![
                enum_name.clone(),
                value.to_string(),
                first_name.clone(),
                second_name.clone(),
            ],
        },
        ValidationError::EnumVariantValueOverflowsUnderlying {
            enum_name,
            variant_name,
            value,
            underlying,
        } => DiagnosticPayload {
            code: DiagnosticCode::ValidationEnumVariantValueOverflowsUnderlying,
            stage: Stage::Validation,
            expected: Some(vec![underlying.clone()]),
            actual: Some(value.to_string()),
            fix: None,
            key_fragments: vec![
                enum_name.clone(),
                variant_name.clone(),
                value.to_string(),
                underlying.clone(),
            ],
        },
        ValidationError::EnumUnsupportedUnderlyingType { name, declared } => DiagnosticPayload {
            code: DiagnosticCode::ValidationEnumUnsupportedUnderlyingType,
            stage: Stage::Validation,
            expected: None,
            actual: Some(declared.clone()),
            fix: None,
            key_fragments: vec![name.clone(), declared.clone()],
        },
        ValidationError::EventSchemaOnBuiltinEvent { event_name } => DiagnosticPayload {
            // NL→IR Mapping Roadmap Item C1 Path A (DL-9'): EventSchema
            // declared against a W3C built-in event namespace
            // (`error.*`, `done.invoke.*`, `done.state.*`). No
            // `fix` candidate set — the legal repair is to rename the
            // schema's `sce:event-name` to a non-reserved value or
            // delete the schema document; neither is enumerable from
            // the offending input.
            code: DiagnosticCode::ValidationEventSchemaOnBuiltinEvent,
            stage: Stage::Validation,
            expected: None,
            actual: Some(event_name.clone()),
            fix: None,
            // The reserved-event name is the canonical identity —
            // two distinct authored schemas pointing at the same
            // built-in event collapse to the same diagnostic ID.
            key_fragments: vec![event_name.clone()],
        },
        ValidationError::EventPayloadFieldUnknown {
            importing_kind,
            importing_name,
            event_name,
            field,
            imported_kind: _,
            imported_name: _,
            candidates,
        } => DiagnosticPayload {
            // NL→IR Mapping Roadmap Item C1 Path A (DL-4' send-side):
            // the `<param name="F">` on a `<send event="X">` /
            // `<raise event="X">` references an undeclared field on
            // the imported EventSchema for `X`. Mirrors
            // `CrossKindFieldNotFound`'s closed candidate set so
            // consumers see `did_you_mean`-style typo repair.
            code: DiagnosticCode::ValidationEventPayloadFieldUnknown,
            stage: Stage::Validation,
            // `expected` stays absent: the declared-field surface is
            // a substitution candidate list, and §3.2 gives that role
            // to `fix` alone — "the candidate list is never duplicated
            // across both fields". The `FixCarriesCandidates`
            // non-overlap class this code belongs to says the same
            // thing from the other side; it names where the candidates
            // ride, not a second place to copy them to.
            expected: None,
            actual: Some(field.clone()),
            fix: if candidates.is_empty() {
                None
            } else {
                Some(Fix::ReplaceOneOf {
                    candidates: candidates.clone(),
                })
            },
            // Statechart + event + offending field name form the
            // canonical identity — two unrelated bad params in the
            // same statechart on different events stay distinct.
            key_fragments: vec![
                importing_kind.to_string(),
                importing_name.clone(),
                event_name.clone(),
                field.clone(),
            ],
        },
        ValidationError::BytesComparisonNotEquality {
            importing_kind,
            importing_name,
            field,
            op,
        } => DiagnosticPayload {
            // RFC `rfc-eventschema-bytes-guard.md` §bytesguard-3 B3: an ordering
            // operator on a bytes payload. No `fix` candidate set —
            // the repair is author-domain (switch to `===`/`!==`, or
            // compare a different field); deterministic, not a closed
            // choice. `expected` names the only legal operator class so
            // the message and the wire payload agree.
            code: DiagnosticCode::ValidationBytesComparisonNotEquality,
            stage: Stage::Validation,
            expected: Some(vec!["=== or !==".to_string()]),
            actual: Some(op.clone()),
            // Statechart + offending field + operator form the
            // canonical identity — two distinct bad guards on the same
            // field with different operators stay distinct.
            fix: None,
            key_fragments: vec![
                importing_kind.to_string(),
                importing_name.clone(),
                field.clone(),
                op.clone(),
            ],
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
/// per-instance message. (Two layers are reachable today; additional
/// layers grow the enum without disturbing existing
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
fn worker_shared_state_actual(reason: &crate::forge::error::WorkerSharedStateReason) -> String {
    use crate::forge::error::WorkerSharedStateReason;
    match reason {
        WorkerSharedStateReason::WorkerImportForbidden {
            imported_alias,
            imported_src,
        } => {
            format!("<sce:import as=\"{imported_alias}\" src=\"{imported_src}\" kind=\"worker\"/>")
        }
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
        // The vocabulary that exists is the answer, so it rides `fix`
        // as the candidate set a consumer can offer directly. `expected`
        // stays absent: non-overlap gives the candidates one home, and
        // this producer's home is `fix` (`non_overlap_class` places the
        // code in `FixCarriesCandidates`).
        ExprError::UnsupportedBuiltin { name, available } => DiagnosticPayload {
            code: DiagnosticCode::ExpressionUnsupportedBuiltin,
            stage: Stage::Expression,
            expected: None,
            actual: Some(name.clone()),
            fix: Some(Fix::ReplaceOneOf {
                candidates: available.clone(),
            }),
            key_fragments: vec![name.clone()],
        },
        // The near misses are the answer, and there may be none. An
        // empty `candidates` reaches `with_a_choice_to_offer` and becomes
        // an absent `fix` — the contract's spelling for "no local edit
        // repairs this" — rather than a choice from nothing.
        ExprError::UnknownIdentifier { name, candidates } => DiagnosticPayload {
            code: DiagnosticCode::ExpressionUnknownIdentifier,
            stage: Stage::Expression,
            expected: None,
            actual: Some(name.clone()),
            fix: Some(Fix::ReplaceOneOf {
                candidates: candidates.clone(),
            }),
            // The candidates are derived from the document, not from the
            // mistake, so two documents misspelling the same name the
            // same way are one diagnostic identity.
            key_fragments: vec![name.clone()],
        },
        // The call is what `actual` names, because the call is what the
        // consumer edits: `.length` occurs on the line either way, and a
        // record pointing at it would leave the consumer to work out
        // that the parentheses beside it are the part to remove. A call
        // carrying arguments has no single replacement — `.length` is
        // still the name, but what becomes of the arguments is the
        // author's decision — so the record names the property and
        // carries no `fix`, which is §3's spelling for that.
        ExprError::PropertyNotCallable { name, arguments } => DiagnosticPayload {
            code: DiagnosticCode::ExpressionPropertyNotCallable,
            stage: Stage::Expression,
            expected: None,
            actual: Some(if *arguments == 0 {
                format!("{name}()")
            } else {
                name.clone()
            }),
            fix: (*arguments == 0).then(|| Fix::ReplaceWith { to: name.clone() }),
            key_fragments: vec![name.clone()],
        },
        // The namespace is what `actual` names, and the members that may
        // stand in its place ride `expected` rather than `fix`. Dropping
        // the call is not the repair here — `Math` alone is refused too —
        // and naming a member without its arguments would be an edit the
        // consumer cannot apply, so the producer states the position and
        // stops there.
        ExprError::NamespaceNotCallable { namespace, members } => DiagnosticPayload {
            code: DiagnosticCode::ExpressionNamespaceNotCallable,
            stage: Stage::Expression,
            expected: Some(members.clone()),
            actual: Some(format!("{namespace}()")),
            fix: None,
            key_fragments: vec![namespace.clone()],
        },
        // `actual` is the bare name this time, because the bare name is
        // what the consumer edits: there are no parentheses beside it to
        // remove, and what has to appear is a member the document has
        // not written yet.
        // No `expected` and no `fix`: the producer knows what was
        // written and nothing about what should have been. The record's
        // work is the location and the sentence.
        ExprError::LiteralNotCallable { what } => DiagnosticPayload {
            code: DiagnosticCode::ExpressionLiteralNotCallable,
            stage: Stage::Expression,
            expected: None,
            actual: Some(what.clone()),
            fix: None,
            key_fragments: vec![what.clone()],
        },
        ExprError::NamespaceNotAValue { namespace, members } => DiagnosticPayload {
            code: DiagnosticCode::ExpressionNamespaceNotAValue,
            stage: Stage::Expression,
            expected: Some(members.clone()),
            actual: Some(namespace.clone()),
            fix: None,
            key_fragments: vec![namespace.clone()],
        },
        ExprError::StrictEquality { operator, strict } => DiagnosticPayload {
            code: DiagnosticCode::ExpressionStrictEquality,
            stage: Stage::Expression,
            // Single legal replacement (`==` → `===`, `!=` → `!==`).
            // It rides `fix` as a deterministic `ReplaceWith`;
            // duplicating it in `expected` would violate non-overlap.
            expected: None,
            actual: Some((*operator).to_string()),
            fix: Some(Fix::ReplaceWith {
                to: (*strict).to_string(),
            }),
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
        GenerateError::TemplateLoad(_) => DiagnosticPayload {
            code: DiagnosticCode::GenerateTemplateLoad,
            stage: Stage::Generate,
            expected: None,
            actual: None,
            fix: None,
            // No fragments. The only value this variant carries is
            // minijinja's own error text with a template name glued to
            // the front, and a key fragment must be a value SCE itself
            // determined (SCE_ERROR_CONTRACT.md §2.1.1): a renderer
            // upgrade would reword it and move every id. Generation
            // aborts at the first template failure, so `code|stage|file`
            // identifies the one a run can produce.
            key_fragments: Vec::new(),
        },
        GenerateError::TemplateRender(_) => DiagnosticPayload {
            code: DiagnosticCode::GenerateTemplateRender,
            stage: Stage::Generate,
            expected: None,
            actual: None,
            fix: None,
            // No fragments. The only value this variant carries is
            // minijinja's own error text with a template name glued to
            // the front, and a key fragment must be a value SCE itself
            // determined (SCE_ERROR_CONTRACT.md §2.1.1): a renderer
            // upgrade would reword it and move every id. Generation
            // aborts at the first template failure, so `code|stage|file`
            // identifies the one a run can produce.
            key_fragments: Vec::new(),
        },
        GenerateError::UnsupportedFeature(detail) => DiagnosticPayload {
            code: DiagnosticCode::GenerateUnsupportedFeature,
            stage: Stage::Generate,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![detail.clone()],
        },
        GenerateError::CodegenMcuClassKindOnNonMcuLanguage { kind, language } => {
            DiagnosticPayload {
                code: DiagnosticCode::CodegenMcuClassKindOnNonMcuLanguage,
                stage: Stage::Generate,
                expected: None,
                actual: Some(language.clone()),
                fix: None,
                key_fragments: vec![kind.clone(), language.clone()],
            }
        }
        GenerateError::CodegenGenericKindBackendEmitMissing { kind, language } => {
            DiagnosticPayload {
                code: DiagnosticCode::CodegenGenericKindBackendEmitMissing,
                stage: Stage::Generate,
                expected: None,
                actual: None,
                fix: None,
                key_fragments: vec![kind.clone(), language.clone()],
            }
        }
        // ── Non-MCU backend refuses `platform.c11_section_attribute`
        //    (SCE Protocol-Synthesis RFC §5.2).
        //    `actual` carries the offending backend name (`cpp` /
        //    `rust` / `kotlin` / `go` / `python`); `key_fragments`
        //    use the same single value so the wire-id is stable per
        //    backend across runs. Stage = Generate — the reject fires
        //    inside the codegen-matrix walker, matching the existing
        //    `codegen/mcu-class-kind-on-non-mcu-language` sibling.
        GenerateError::McuSectionAttributeOnNonMcuTarget { backend } => DiagnosticPayload {
            code: DiagnosticCode::McuSectionAttributeOnNonMcuTarget,
            stage: Stage::Generate,
            expected: None,
            actual: Some(backend.clone()),
            fix: None,
            key_fragments: vec![backend.clone()],
        },
        // The repair is a rename the author has to make in two places
        // (deploy.yaml and the linker script that places the section),
        // so no candidate set is derivable and `fix` stays `None`. The
        // name rides `actual` verbatim; the per-character reason is in
        // the message rather than in `expected`, which would otherwise
        // be read as a suggested replacement.
        GenerateError::McuSectionAttributeNameInvalid { name, reason } => DiagnosticPayload {
            code: DiagnosticCode::McuSectionAttributeNameInvalid,
            stage: Stage::Generate,
            expected: None,
            actual: Some(name.clone()),
            fix: None,
            key_fragments: vec![name.clone(), reason.clone()],
        },
        GenerateError::CodegenNoStdScriptNotSupported {
            document,
            locations,
        } => DiagnosticPayload {
            code: DiagnosticCode::CodegenNoStdScriptNotSupported,
            stage: Stage::Generate,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![document.clone(), locations.clone()],
        },
        GenerateError::CodegenNoStdHttpNotSupported {
            document,
            locations,
        } => DiagnosticPayload {
            code: DiagnosticCode::CodegenNoStdHttpNotSupported,
            stage: Stage::Generate,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![document.clone(), locations.clone()],
        },
        GenerateError::CodegenNoStdFsLoadNotSupported {
            document,
            locations,
        } => DiagnosticPayload {
            code: DiagnosticCode::CodegenNoStdFsLoadNotSupported,
            stage: Stage::Generate,
            expected: None,
            actual: None,
            fix: None,
            key_fragments: vec![document.clone(), locations.clone()],
        },
        GenerateError::CodegenNoStdInvokeNotSupported {
            document,
            locations,
        } => DiagnosticPayload {
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

/// SCXML semantic-validation field mapping (§wire-W5 D2).
///
/// Three of the four variants reuse existing `validation/*` wire codes
/// per the W4 D4 fold precedent — concept identity over namespace
/// duplication. Only `TopLevelScriptUnloaded` introduces a NEW wire
/// code (`scxml/top-level-script-unloaded`) because §scxml-5.8
/// has no forge analog.
///
/// Stage stays `Stage::Validation` for all four (§wire-W5 D2 reverse-
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
        ScxmlSemanticError::HistoryDefaultTransitionMissing {
            history_id,
            parent_id,
            available: _,
        } => DiagnosticPayload {
            // REUSE — same wire code as forge
            // `ValidationError::MissingElement`. Concept identity: "a
            // required child element is absent".
            code: DiagnosticCode::ValidationMissingElement,
            stage: Stage::Validation,
            expected: None,
            // `actual` names the offending element so repair tooling
            // locates it; the missing child has no value to substitute,
            // which is why `validation/missing-element` carries no
            // `fix` (SCE_ERROR_CONTRACT §3.1 has no add-child-element
            // variant — the legal default targets travel in `message`).
            actual: Some(history_id.clone()),
            fix: None,
            // Parent + history id keep two defaulted-less histories in
            // one document distinguishable in the content-hash id.
            key_fragments: vec![
                format!("scxml-state:{parent_id}"),
                "history-default-transition".to_string(),
                history_id.clone(),
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
        ScxmlSemanticError::UnsupportedDatamodel {
            declared,
            kind,
            supported,
        } => DiagnosticPayload {
            code: DiagnosticCode::ScxmlUnsupportedDatamodel,
            stage: Stage::Validation,
            // `expected` is the closed vocabulary, `actual` what the
            // document wrote — the pair a consumer needs to render
            // "did you mean" without re-deriving SCE's support matrix.
            expected: Some(supported.clone()),
            actual: Some(declared.clone()),
            fix: Some(Fix::ReplaceOneOf {
                candidates: supported.clone(),
            }),
            // `kind` rides the id so a document that moves from `xpath`
            // to a typo does not reuse the previous record's identity.
            key_fragments: vec![
                "datamodel".to_string(),
                declared.clone(),
                match kind {
                    crate::scxml_semantic::UnsupportedDatamodelKind::Unimplemented => {
                        "unimplemented".to_string()
                    }
                    crate::scxml_semantic::UnsupportedDatamodelKind::Undefined => {
                        "undefined".to_string()
                    }
                },
            ],
        },
        ScxmlSemanticError::NullDatamodelForbidsConstruct {
            construct,
            needs,
            rule,
            state,
        } => DiagnosticPayload {
            code: DiagnosticCode::ScxmlNullDatamodelForbidsConstruct,
            stage: Stage::Validation,
            // What the Null data model offers for this construct is
            // nothing, and saying so is the point — `expected` names the
            // missing language rather than a value to substitute.
            expected: Some(vec![format!("{needs} (W3C SCXML {rule})")]),
            actual: Some(construct.clone()),
            fix: None,
            key_fragments: {
                let mut k = vec!["null-datamodel".to_string(), construct.clone()];
                if !state.is_empty() {
                    k.push(state.clone());
                }
                k
            },
        },
        ScxmlSemanticError::TopLevelScriptUnloaded { index, src } => DiagnosticPayload {
            // NEW — §scxml-5.8 has no forge analog. The 1 NEW
            // wire code §wire-W5 D2 introduces.
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
        ScxmlSemanticError::UnreachableState { state_id } => DiagnosticPayload {
            // NEW — NL→IR Mapping Roadmap Item 3. Reachability
            // is a Statechart-graph rule with no forge analog (Forge
            // kinds carry no control-flow surface), so the wire code
            // sits in the `scxml/*` namespace.
            code: DiagnosticCode::ScxmlUnreachableState,
            stage: Stage::Validation,
            expected: None,
            actual: Some(state_id.clone()),
            // NeutralOrDeterministic — author repair is "delete the
            // orphan or wire a transition into it"; no closed
            // candidate set would be honest (see `non_overlap_class`
            // commentary on this code).
            fix: None,
            key_fragments: vec!["scxml-unreachable-state".to_string(), state_id.clone()],
        },
        ScxmlSemanticError::DeadTransition { state, target } => DiagnosticPayload {
            // NEW — paired with `UnreachableState` above; emitted in
            // preference whenever the orphan state carries at least
            // one `<transition>` so the diagnostic surfaces a concrete
            // (source, target) edge to repair.
            code: DiagnosticCode::ScxmlDeadTransition,
            stage: Stage::Validation,
            expected: None,
            actual: Some(target.clone()),
            fix: None,
            // (state, target) keep two orphan edges in the same
            // document distinguishable in the content-hash id.
            key_fragments: vec![
                format!("scxml-state:{state}"),
                "dead-transition-target".to_string(),
                target.clone(),
            ],
        },
        ScxmlSemanticError::AlwaysFalseGuard { state, cond } => DiagnosticPayload {
            // NEW — NL→IR Mapping Roadmap Item 3 guard analysis. The
            // `actual` slot carries the raw guard text so consumers
            // can quote it back to the author; `key_fragments`
            // distinguish two always-false guards in the same state.
            code: DiagnosticCode::ScxmlAlwaysFalseGuard,
            stage: Stage::Validation,
            expected: None,
            actual: Some(cond.clone()),
            fix: None,
            key_fragments: vec![
                format!("scxml-state:{state}"),
                "always-false-guard".to_string(),
                cond.clone(),
            ],
        },
        ScxmlSemanticError::ShadowedTransition {
            state,
            event,
            shadowing_index,
            shadowed_index,
        } => DiagnosticPayload {
            // NEW — paired with `AlwaysFalseGuard` above. The
            // `actual` slot carries the event descriptor verbatim;
            // both transition indices ride in `key_fragments` so two
            // shadowed-transition diagnostics in the same state on
            // the same event remain distinguishable.
            code: DiagnosticCode::ScxmlShadowedTransition,
            stage: Stage::Validation,
            expected: None,
            actual: Some(event.clone()),
            fix: None,
            key_fragments: vec![
                format!("scxml-state:{state}"),
                "shadowed-transition".to_string(),
                event.clone(),
                format!("shadow-by:{shadowing_index}"),
                format!("shadowed:{shadowed_index}"),
            ],
        },
        ScxmlSemanticError::NonExhaustiveEventHandling {
            parent,
            event,
            handlers: _,
            non_handlers,
            // Not a key fragment: the other gaps are context for the
            // author, not part of what makes THIS record distinct, and
            // folding them in would move the id whenever an unrelated
            // sibling gap appeared or was repaired.
            also: _,
        } => DiagnosticPayload {
            // NEW — NL→IR Mapping Roadmap Item 3 event-set exhaustiveness. The
            // `actual` slot carries the unhandled event so consumers
            // dispatching on (code, actual) can route the diagnostic
            // even when the parent id is verbose. handlers /
            // non_handlers ids ride in `key_fragments` to keep the
            // FNV1a content-hash id distinct across multiple
            // non-exhaustive gaps in the same document.
            code: DiagnosticCode::ScxmlNonExhaustiveEventHandling,
            stage: Stage::Validation,
            expected: None,
            actual: Some(event.clone()),
            fix: None,
            key_fragments: {
                let mut k = vec![
                    format!("scxml-parent:{parent}"),
                    "non-exhaustive-event".to_string(),
                    event.clone(),
                ];
                for nh in non_handlers {
                    k.push(format!("non-handler:{nh}"));
                }
                k
            },
        },
        ScxmlSemanticError::ContradictoryUnhandledDeclaration { state, event } => {
            DiagnosticPayload {
                // The declaring state and the event it contradicts
                // itself about are the whole identity — there is no
                // sibling context in this record, by construction.
                code: DiagnosticCode::ScxmlContradictoryUnhandledDeclaration,
                stage: Stage::Validation,
                expected: None,
                actual: Some(event.clone()),
                fix: None,
                key_fragments: vec![
                    format!("scxml-state:{state}"),
                    "contradictory-unhandled".to_string(),
                    event.clone(),
                ],
            }
        }
        ScxmlSemanticError::StaleUnhandledDeclaration {
            state,
            parent,
            event,
            // Not a key fragment for the same reason `also` is not one
            // on the gap record: the parent's other gaps are context
            // for the author, and folding them in would move the id
            // whenever an unrelated gap appeared or was repaired.
            gaps: _,
        } => DiagnosticPayload {
            code: DiagnosticCode::ScxmlStaleUnhandledDeclaration,
            stage: Stage::Validation,
            expected: None,
            actual: Some(event.clone()),
            fix: None,
            key_fragments: vec![
                format!("scxml-parent:{parent}"),
                format!("scxml-state:{state}"),
                "stale-unhandled".to_string(),
                event.clone(),
            ],
        },
    }
}

// ── Helpers ────────────────────────────────────────────────────

/// Split a human-readable "expected" list ("foo, bar | baz") into a
/// vector of individual tokens. Several validation errors carry this
/// as free text for the Display impl; consumers need it structured.
fn split_expected(s: &str) -> Vec<String> {
    s.split([',', '|'])
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
        assert!(
            d.expected.is_none(),
            "expected must not duplicate fix.candidates"
        );
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
                r#"{"v":1,"id":"fnv1a:414d301b635e7145","code":"xml/parse","stage":"xml","message":"XML parse error: unexpected end tag </scxml>"}"#,
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
                r#"{"v":1,"id":"fnv1a:812898e1a23fda4d","code":"validation/unsupported-kind","stage":"validation","spec":"SCE Forge §3.2","message":"unsupported sce:kind value: 'bogus'","actual":"bogus","fix":{"kind":"replace_one_of","candidates":["statechart","transform","lookup","condition","codec","procedure","validator","filter","interpolation","timer","observer","algorithm","link","buffer-pool","worker","bounded-collection","enum","event-schema"]}}"#,
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
                "forge/duplicate-requirement-id",
                ValidationError::DuplicateRequirementId {
                    element: "<state id=\"armed\">".into(),
                    id: "REQ_001".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:cb11c8b9b1171851","code":"validation/duplicate-requirement-id","stage":"validation","message":"<state id=\"armed\">: duplicate sce:req id 'REQ_001'","actual":"REQ_001"}"#,
            ),
            (
                "forge/unresolved-placeholder",
                ValidationError::UnresolvedPlaceholder {
                    element: "<state id=\"armed\">".into(),
                    id: "tbd_threshold".into(),
                    reason: Some("waiting on calibration data".into()),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:bb99ccd9698c8f58","code":"validation/unresolved-placeholder","stage":"validation","message":"<state id=\"armed\">: unresolved placeholder id='tbd_threshold' reason='waiting on calibration data'","actual":"tbd_threshold"}"#,
            ),
            (
                "forge/cross-kind-field-not-found",
                ValidationError::CrossKindFieldNotFound {
                    importing_kind: ForgeKind::Algorithm,
                    importing_name: "keyexpr_match".into(),
                    alias: "subs".into(),
                    field: "callbackid".into(),
                    imported_kind: ForgeKind::BoundedCollection,
                    imported_name: "sub_table".into(),
                    candidates: vec!["callback_id".into(), "topic_id".into()],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:dea0fc18e5b53274","code":"validation/cross-kind-field-not-found","stage":"validation","message":"algorithm 'keyexpr_match': 'subs.callbackid' references an undeclared field on imported bounded-collection 'sub_table' (declared fields: callback_id, topic_id)","actual":"subs.callbackid","fix":{"kind":"replace_one_of","candidates":["subs.callback_id","subs.topic_id"]}}"#,
            ),
            (
                "forge/cross-kind-type-mismatch",
                ValidationError::CrossKindTypeMismatch {
                    importing_kind: ForgeKind::Algorithm,
                    importing_name: "keyexpr_match".into(),
                    alias: "subs".into(),
                    field: "callback_id".into(),
                    actual: "uint32".into(),
                    expected: "bool".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:f19b566002125832","code":"validation/cross-kind-type-mismatch","stage":"validation","message":"algorithm 'keyexpr_match': 'subs.callback_id' has type 'uint32' but context expects 'bool'","expected":["bool"],"actual":"uint32"}"#,
            ),
            (
                "forge/cross-kind-circular-dependency",
                ValidationError::CrossKindCircularDependency {
                    cycle: vec!["a.scxml".into(), "b.scxml".into(), "a.scxml".into()],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:9316da7f260e4553","code":"validation/cross-kind-circular-dependency","stage":"validation","message":"circular <sce:import> dependency: a.scxml → b.scxml → a.scxml","actual":"a.scxml → b.scxml → a.scxml"}"#,
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
                    pipeline: crate::Pipeline::Forge,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:71f73dde2407223e","code":"validation/wrong-pipeline","stage":"validation","spec":"SCE Forge §4","message":"statechart kind cannot be processed by the forge pipeline","actual":"statechart"}"#,
            ),
            // The mirror direction, which had no producer until the
            // statechart entries started asking the router: same code and
            // same `actual`, different message, so a consumer keyed on the
            // code still routes and an author reads the right pipeline.
            (
                "forge/wrong-pipeline-into-scxml",
                ValidationError::WrongPipeline {
                    kind: ForgeKind::Algorithm,
                    pipeline: crate::Pipeline::Scxml,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:cbb2daf4e235a91e","code":"validation/wrong-pipeline","stage":"validation","spec":"SCE Forge §4","message":"algorithm kind cannot be processed by the SCXML pipeline","actual":"algorithm"}"#,
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
                "forge/native-action-placement",
                ValidationError::NativeActionPlacement {
                    name: "do_effect".into(),
                    detail: "supported only as a direct <transition> child".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:1473c2ac552ffeb4","code":"validation/native-action-placement","stage":"validation","message":"<sce:action name=\"do_effect\">: supported only as a direct <transition> child","actual":"supported only as a direct <transition> child"}"#,
            ),
            (
                "forge/native-action-argument",
                ValidationError::NativeActionArgument {
                    name: "append".into(),
                    detail: "argument '42' must be a bare `_event.data.<field>` reference".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:670341f082aeea0c","code":"validation/native-action-argument","stage":"validation","message":"<sce:action name=\"append\">: argument '42' must be a bare `_event.data.<field>` reference","actual":"argument '42' must be a bare `_event.data.<field>` reference"}"#,
            ),
            (
                "forge/native-action-signature-conflict",
                ValidationError::NativeActionSignatureConflict {
                    name: "append".into(),
                    detail: "argument types (bytes) here disagree with (uint32) on another transition".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:2f5ea10b3dcdc557","code":"validation/native-action-signature-conflict","stage":"validation","message":"<sce:action name=\"append\">: argument types (bytes) here disagree with (uint32) on another transition","actual":"argument types (bytes) here disagree with (uint32) on another transition"}"#,
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
                // §wire-W5: SCXML semantic family — TopLevelScriptUnloaded
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
                // §scxml-3.2 — a data model SCE cannot honor. Golden
                // uses `xpath`, the Unimplemented half, because it is
                // the case where the document is valid W3C SCXML and
                // SCE is the limitation; the Undefined half differs
                // only in the key fragment that rides the id.
                "forge/scxml-unsupported-datamodel-xpath",
                crate::scxml_semantic::ScxmlSemanticError::UnsupportedDatamodel {
                    declared: "xpath".into(),
                    kind: crate::scxml_semantic::UnsupportedDatamodelKind::Unimplemented,
                    supported: vec!["null".into(), "ecmascript".into()],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:975893e56914aa49","code":"scxml/unsupported-datamodel","stage":"validation","spec":"W3C SCXML §3.2","message":"datamodel=\"xpath\" is a W3C SCXML data model that SCE does not implement","expected":["null","ecmascript"],"actual":"xpath","fix":{"kind":"replace_one_of","candidates":["null","ecmascript"]}}"#,
            ),
            (
                // §scxml-B-1 — a construct whose language the declared
                // model does not have. Golden uses the `<param expr=…>`
                // shape because it is the one the repository's own
                // fixtures walked into: `<param>` is a §5 element
                // (B-1-7) and its `expr` needs the value expression
                // language (B-1-4), so the element rule is what fires.
                "forge/scxml-null-datamodel-forbids-construct",
                crate::scxml_semantic::ScxmlSemanticError::NullDatamodelForbidsConstruct {
                    construct: "<param>".into(),
                    needs: "the data model its §5 semantics operate on".into(),
                    rule: "B.1.7".into(),
                    state: "idle".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:18b3e668983fd6d6","code":"scxml/null-datamodel-forbids-construct","stage":"validation","spec":"W3C SCXML §B.1","message":"<param> is not available under datamodel=\"null\": it needs the data model its §5 semantics operate on, which W3C SCXML B.1.7 withholds — declare the data model this document actually uses, or remove the construct","expected":["the data model its §5 semantics operate on (W3C SCXML B.1.7)"],"actual":"<param>"}"#,
            ),
            (
                // NL→IR Mapping Roadmap Item 3 — Statechart
                // reachability. State-level form fires only when an
                // orphan state has no `<transition>` children (the
                // per-transition variant outranks it otherwise).
                "forge/scxml-unreachable-state",
                crate::scxml_semantic::ScxmlSemanticError::UnreachableState {
                    state_id: "ghost_branch".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:c62a2a67930417c8","code":"scxml/unreachable-state","stage":"validation","message":"State 'ghost_branch' is unreachable from the document initial configuration","actual":"ghost_branch"}"#,
            ),
            (
                // NL→IR Mapping Roadmap Item 3 — per-transition
                // form. Source is the unreachable state, target is the
                // transition's `target` attribute verbatim.
                "forge/scxml-dead-transition",
                crate::scxml_semantic::ScxmlSemanticError::DeadTransition {
                    state: "ghost_branch".into(),
                    target: "armed".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:c6afb255ec2689b7","code":"scxml/dead-transition","stage":"validation","message":"Transition in unreachable state 'ghost_branch' targets 'armed' — source state is never entered","actual":"armed"}"#,
            ),
            (
                // NL→IR Mapping Roadmap Item 3 — non-exhaustive
                // event handling. The parent compound state has three
                // children that share `cmd.stop` as common ground;
                // `cmd.start` is handled by `idle` + `stopped` but
                // not by `active`, so the validator flags the gap.
                "forge/scxml-non-exhaustive-event-handling",
                crate::scxml_semantic::ScxmlSemanticError::NonExhaustiveEventHandling {
                    parent: "dispatch".into(),
                    event: "cmd.start".into(),
                    handlers: vec!["idle".into(), "stopped".into()],
                    non_handlers: vec!["active".into()],
                    also: Vec::new(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:7072cc6c1038cfb6","code":"scxml/non-exhaustive-event-handling","stage":"validation","message":"Compound state 'dispatch' has children handling event 'cmd.start' inconsistently — handlers: [\"idle\", \"stopped\"], non-handlers: [\"active\"]. Add the missing transition, add a parent-level fallthrough, or declare the gap on the non-handling child with sce:unhandled=\"cmd.start\" if it is intentional.","actual":"cmd.start"}"#,
            ),
            (
                // NL→IR Mapping Roadmap Item 3 — a state declaring
                // `sce:unhandled` for an event it in fact handles.
                // Local contradiction; no sibling context involved.
                "forge/scxml-contradictory-unhandled-declaration",
                crate::scxml_semantic::ScxmlSemanticError::ContradictoryUnhandledDeclaration {
                    state: "active".into(),
                    event: "cmd.start".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:f85013ee355dc5a9","code":"scxml/contradictory-unhandled-declaration","stage":"validation","message":"State 'active' declares sce:unhandled=\"cmd.start\" but has a transition that handles 'cmd.start'. Remove the event from sce:unhandled, or remove the transition — the document currently asserts both.","actual":"cmd.start"}"#,
            ),
            (
                // NL→IR Mapping Roadmap Item 3 — a declaration that
                // exempts nothing. `gaps` empty is the harder half of
                // the message: the state has no gaps at all, so the
                // declaration has outlived its subject.
                "forge/scxml-stale-unhandled-declaration",
                crate::scxml_semantic::ScxmlSemanticError::StaleUnhandledDeclaration {
                    state: "active".into(),
                    parent: "dispatch".into(),
                    event: "cmd.start".into(),
                    gaps: Vec::new(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:0f6a02f53c7d0276","code":"scxml/stale-unhandled-declaration","stage":"validation","message":"State 'active' declares sce:unhandled=\"cmd.start\" but 'cmd.start' is not a gap under parent 'dispatch' — that state has no inconsistently-handled events at all. Remove the event from sce:unhandled.","actual":"cmd.start"}"#,
            ),
            (
                // NL→IR Mapping Roadmap Item 3 — trivially
                // false guard. The validator stops at structural
                // false literals (`false`, `0`, differing numeric
                // equality, equal numeric inequality) and leaves
                // language-prefixed conds (`cpp:`, `kotlin:`,
                // `rust:`) opaque.
                "forge/scxml-always-false-guard",
                crate::scxml_semantic::ScxmlSemanticError::AlwaysFalseGuard {
                    state: "armed".into(),
                    cond: "false".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:9e9dbc120c0f1890","code":"scxml/always-false-guard","stage":"validation","message":"Transition in state 'armed' carries guard 'false' that is statically false — the transition can never fire. Remove the transition or change the guard expression.","actual":"false"}"#,
            ),
            (
                // NL→IR Mapping Roadmap Item 3 — shadowed
                // transition. Document-order #0 (unconditional)
                // shadows #1 (guarded) on the same event descriptor.
                "forge/scxml-shadowed-transition",
                crate::scxml_semantic::ScxmlSemanticError::ShadowedTransition {
                    state: "armed".into(),
                    event: "fire".into(),
                    shadowing_index: 0,
                    shadowed_index: 1,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:d4a8c789490ede2b","code":"scxml/shadowed-transition","stage":"validation","message":"Transition #1 in state 'armed' (event 'fire') is shadowed by an earlier unconditional transition #0 with the same event descriptor. The shadowed transition can never fire. Reorder the transitions, add a guard to the shadowing transition, or remove the shadowed transition.","actual":"fire"}"#,
            ),
            (
                // SCE Protocol-Synthesis RFC §synth-5-E sample-callback placement rule
                "forge/scxml-on-sample-invalid-parent",
                ValidationError::OnSampleInvalidParent {
                    path: "scxml > onentry".into(),
                    actual_parent: "onentry".into(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:90051ac7a8571e3a","code":"scxml/on-sample-invalid-parent","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.E","message":"<sce:on-sample> at scxml > onentry: must appear directly inside a <state> or <parallel>; found inside <onentry>. Move the element under a state or parallel ancestor.","actual":"onentry"}"#,
            ),
            (
                // SCE Protocol-Synthesis RFC §synth-5-E sample-callback per-state uniqueness rule
                "forge/scxml-on-sample-link-duplicate-in-state",
                ValidationError::OnSampleLinkDuplicateInState {
                    state_id: "running".into(),
                    link: "scout_link".into(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:ab4ec9e9ef9663b9","code":"scxml/on-sample-link-duplicate-in-state","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.E","message":"state 'running': duplicate <sce:on-sample link=\"scout_link\"> declarations. Each link is allowed at most one on-sample block per state; merge the duplicates or rename one of the link references.","actual":"scout_link"}"#,
            ),
            (
                // SCE Protocol-Synthesis RFC §synth-5-E sample-callback reserved-event-name rule
                "forge/scxml-on-sample-event-name-conflict",
                ValidationError::OnSampleEventNameConflict {
                    event: "error.io".into(),
                    reserved_prefix: "error.".into(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:3b804e5f18b5b65a","code":"scxml/on-sample-event-name-conflict","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.E","message":"<sce:on-sample event=\"error.io\"> collides with the reserved W3C SCXML internal event prefix 'error.'. Pick an event name outside that family (e.g. 'sample.error.io') so dispatched samples stay distinct from built-in lifecycle events.","actual":"error.io"}"#,
            ),
            (
                // SCE Protocol-Synthesis RFC §synth-5-E sample-callback
                // cross-ref pair — `not-declared` is reachable today.
                "forge/scxml-on-sample-link-not-declared",
                ValidationError::OnSampleLinkNotDeclared {
                    state_id: "running".into(),
                    link: "scout_link".into(),
                    candidates: vec!["status_link".into()],
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:b635927d2fa69152","code":"scxml/on-sample-link-not-declared","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.E","message":"state 'running': <sce:on-sample link=\"scout_link\"> references a name that no `.forge` file in the build declares as a link kind. Add a forge `<scxml sce:kind=\"link\" name=\"scout_link\">` document or fix the reference. See SCE Protocol-Synthesis RFC §5.E.","actual":"scout_link","fix":{"kind":"replace_one_of","candidates":["status_link"]}}"#,
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
                r#"{"v":1,"id":"fnv1a:38b424712554fe91","code":"scxml/on-sample-link-wrong-kind","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.E","message":"state 'running': <sce:on-sample link=\"scout_codec\"> resolves to a forge 'codec' kind, not 'link'. Only link kind documents back the on-sample subscriber contract. Repoint the reference at one of the build's link kind names. See SCE Protocol-Synthesis RFC §5.E.","actual":"codec","fix":{"kind":"replace_one_of","candidates":["scout_link"]}}"#,
            ),
            (
                // Listener-role — `<sce:session-role kind="X"/>`
                // unknown-kind structural diagnostic. Carries the
                // closed-set vocabulary via `Fix::ReplaceOneOf` so
                // authors get a closed picker even when v1 has only
                // one variant. Hash placeholder — patched by byte-
                // stability assertion.
                "forge/scxml-unknown-session-role-kind",
                ValidationError::ScxmlUnknownSessionRoleKind {
                    kind: "listener".into(),
                    allowed: vec!["accept-side".into()],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:1a3bf2135f63c092","code":"scxml/unknown-session-role-kind","stage":"validation","message":"<sce:session-role kind=\"listener\"/>: unknown session-role kind. v1 vocabulary: [\"accept-side\"]. Repair: change `kind` to one of the listed values or remove the element if no session-FSM role applies.","actual":"listener","fix":{"kind":"replace_one_of","candidates":["accept-side"]}}"#,
            ),
            (
                // Listener-role — duplicate
                // `<sce:session-role>` declaration on one document.
                // NeutralOrDeterministic non_overlap class; no
                // candidate list. Hash placeholder — patched by byte-
                // stability assertion.
                "forge/scxml-duplicate-session-role-declaration",
                ValidationError::ScxmlDuplicateSessionRoleDeclaration {
                    kind: "accept-side".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:ebc5cc71107bd7e9","code":"scxml/duplicate-session-role-declaration","stage":"validation","message":"<sce:session-role kind=\"accept-side\"/>: declared more than once on this SCXML document. Each session-role kind may appear at most once per document. Repair: delete the duplicate `<sce:session-role kind=\"accept-side\"/>` element.","actual":"accept-side"}"#,
            ),
            (
                // Listener-role — deploy declares listener
                // role but SCXML lacks accept-side declaration. Hash
                // placeholder — patched by byte-stability assertion.
                "forge/link-deploy-role-listener-without-scxml-accept-side-role",
                ValidationError::LinkDeployRoleListenerWithoutScxmlAcceptSideRole {
                    machine: "mcu_node".into(),
                    link_name: "udp_listener".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:c2f9a854621bdaa3","code":"link/deploy-role-listener-without-scxml-accept-side-role","stage":"validation","message":"deploy machine 'mcu_node' link 'udp_listener': declares `role: listener` but its source SCXML carries no `<sce:session-role kind=\"accept-side\"/>` top-level declaration. Repair: add `<sce:session-role kind=\"accept-side\"/>` to the SCXML root if it implements the session-FSM accept-side, OR remove `role: listener` from the deploy link if the link is not a listener half.","actual":"udp_listener"}"#,
            ),
            (
                // Listener-role — SCXML declares accept-side
                // but no deploy link has listener role. Hash placeholder.
                "forge/scxml-accept-side-role-without-listener-link",
                ValidationError::ScxmlAcceptSideRoleWithoutListenerLink {
                    machine: "mcu_node".into(),
                    scxml_source: "session_fsm.scxml".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:126e3b7664c66f4a","code":"scxml/accept-side-role-without-listener-link","stage":"validation","message":"SCXML machine 'mcu_node' (source `session_fsm.scxml`): declares `<sce:session-role kind=\"accept-side\"/>` but no deploy link on this machine has `role: listener`. Repair: add `role: listener` to the deploy link that hosts the accept-side handshake, OR remove the `<sce:session-role>` element from the SCXML if it does not serve as the accept-side FSM.","actual":"session_fsm.scxml"}"#,
            ),
            (
                // Listener-role matrix — listener
                // role with wrong trust_class. Hash placeholder.
                "forge/link-role-listener-with-non-session-arming-trust-class",
                ValidationError::LinkRoleListenerWithNonSessionArmingTrustClass {
                    machine: "mcu_node".into(),
                    link_name: "udp_listener".into(),
                    trust_class: "untrusted".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:df7bcfc9e6b3e2ec","code":"link/role-listener-with-non-session-arming-trust-class","stage":"validation","message":"deploy machine 'mcu_node' link 'udp_listener': declares `role: listener` but `trust_class: untrusted` (not `session_arming`). The listener-role declaration applies only to pre-handshake traffic, which lives on the `session_arming` trust tier. Repair: change `trust_class` to `session_arming`, OR remove `role: listener`.","actual":"untrusted"}"#,
            ),
            (
                // Listener-role migration-helper — Accepting.*
                // state ids without role declaration. Hash placeholder.
                "forge/scxml-accept-side-states-without-role-declaration",
                ValidationError::ScxmlAcceptSideStatesWithoutRoleDeclaration {
                    offending_ids: vec![
                        "Accepting".to_string(),
                        "Accepting.AwaitingInitSyn".to_string(),
                    ],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:9c490c868c1407cc","code":"scxml/accept-side-states-without-role-declaration","stage":"validation","message":"SCXML doc carries state ids matching the reserved `Accepting.*` prefix ([\"Accepting\", \"Accepting.AwaitingInitSyn\"]) but no top-level `<sce:session-role kind=\"accept-side\"/>` declaration. The canonical session-FSM accept-side state names are reserved for documents that claim the accept-side role. Repair: add `<sce:session-role kind=\"accept-side\"/>` to the SCXML root if the doc implements the session-FSM accept-side, OR rename the offending state ids to avoid the `Accepting.*` reservation.","actual":"Accepting,Accepting.AwaitingInitSyn"}"#,
            ),
            (
                // Declared-consumption — reassembly per-peer-quota peer-table
                // build invariant violated. Hash placeholder.
                "forge/reassembly-per-peer-quota-build-invariant-violated",
                ValidationError::ReassemblyPerPeerQuotaBuildInvariantViolated {
                    pool_name: "rx_reassembly_pool".into(),
                    slot_count: 16,
                    machine: "mcu_node".into(),
                    link_name: "udp_listener".into(),
                    peer_table_capacity: 2,
                    per_peer_quota: 4,
                    product: 8,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:3c94c473897424b8","code":"reassembly/per-peer-quota-build-invariant-violated","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.M","message":"reassembly-variant buffer-pool 'rx_reassembly_pool' (slot_count=16) bound to machine 'mcu_node' link 'udp_listener' violates the per-peer-quota build invariant: `peer_table.capacity (2) × per_peer_quota (4) = 8` < `slot_count (16)`. RFC §5.M lines 2841-2861 — without this bound a peer storm can occupy more slots than the per-peer cap permits, silently degrading per-peer accounting into shared-pool contention. Repair: raise `peer_table.capacity` on the link's `stateless_accept`, raise `per_peer_quota` on the pool, or lower `slot_count` on the pool.","actual":"8 < 16"}"#,
            ),
            (
                // RFC §synth-5-E sample-callback: on-sample subscriber on a
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
                r#"{"v":1,"id":"fnv1a:1c55aabc0ddc8d36","code":"pool/sample-take-without-stage-pool","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.E","message":"state 'running': <sce:on-sample link=\"scout_link\"> targets a link kind whose forge document does not declare a `<sce:stage-pool>` element. Subscriber callbacks on this link cannot escape the borrow lifetime via `Sample::take()` because there is no stage-copy destination. Add `<sce:stage-pool ref=\"...\">` to the link's `.forge` document or restrict callbacks to borrow-only access. See SCE Protocol-Synthesis RFC §5.E.","actual":"scout_link","fix":{"kind":"replace_one_of","candidates":["scout_stage_pool"]}}"#,
            ),
            (
                // RFC §synth-5-E sample-callback callback-path syntax: an
                // `<sce:on-sample callback="rust:...">` value fails the
                // `rust:crate::module::fn` path subset. Today's reachable
                // arms are syntax failures (UnknownLanguagePrefix shown
                // here); signature-inspection shape-mismatch arms stay
                // absent until a consumer needs them. NeutralOrDeterministic non_overlap
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
                r#"{"v":1,"id":"fnv1a:c1db04cc9e2b921b","code":"pool/sample-callback-signature-non-borrow","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.E","message":"state 'running': <sce:on-sample link=\"scout_link\" callback=\"cpp:my_app::on_scout\"> uses an unsupported language prefix `cpp` (accepted: `rust:`, `c:`). The `callback` value must match `rust:crate::module::fn` or `c:identifier`. The borrow-mode contract is enforced at the dispatch site; rustc rejects owned-mode signatures at user-crate compile time, and the C11 backend emits a prototype the host definition must match. See SCE Protocol-Synthesis RFC §5.E.","actual":"cpp:my_app::on_scout"}"#,
            ),
            (
                // RFC §synth-5-D line 911: worker shared-state encapsulation.
                // Layer 1 (`<sce:import kind="worker">` is
                // the structural author error a parse-time guard catches)
                // and layer 2 (body SCXML cross-namespace data-refs)
                // share this code. Layer 3 (`<sce:extern>`
                // non-inbox symbol use in body) stays unimplemented
                // until a consumer needs the §synth-5-I intrinsic-registry
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
                r#"{"v":1,"id":"fnv1a:f054d112eea16560","code":"worker/shared-mutable-state","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.D","message":"worker 'rx_loop': declares <sce:import as=\"tx_loop\" src=\"tx_loop.scxml\" kind=\"worker\"/>; workers cannot import other worker kinds. Workers must communicate with other workers only through their own inbox (consume) and the recipient's inbox via <sce:outbox ref=\"...\"> (produce); all other paths to another worker's state are forbidden per RFC §5.D line 911 (\"any non-inbox access to another worker's state\").","actual":"<sce:import as=\"tx_loop\" src=\"tx_loop.scxml\" kind=\"worker\"/>"}"#,
            ),
            // ── §synth-5-D worker cross-resolution: link-rx + outbox ref ──
            (
                // RFC §synth-5-D: `<sce:link-rx ref="X">` must reference
                // an alias imported as `kind="link"`. Mirrors the
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
                r#"{"v":1,"id":"fnv1a:60c2fe22d1085b50","code":"worker/link-rx-ref-unknown","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.D","message":"worker 'rx_loop': <sce:link-rx ref=\"udp_scout\"> references a name that is not imported as a link kind. Declare the link via <sce:import as=\"udp_scout\" src=\"...\" kind=\"link\"/> on this worker document, or replace the ref with one of the imported link-kind aliases (closest matches: status_link).","actual":"udp_scout","fix":{"kind":"replace_one_of","candidates":["status_link"]}}"#,
            ),
            // ── §synth-5-I SPSC inbox ordering ──
            (
                // RFC §synth-5-I lines 1757-1758: `<sce:inbox>` declared
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
                r#"{"v":1,"id":"fnv1a:066432600b5950a2","code":"worker/inbox-ordering-unspecified","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.I","message":"worker 'rx_loop': <sce:inbox> declared without an `ordering` attribute. Pick `ordering=\"acq_rel\"` (safe default; producer and consumer pair head/tail with acquire+release on every push/pop) or `ordering=\"relaxed\"` (single-core fast-path; cross-core placement raises `worker/inbox-ordering-relaxed-across-cores`). Spec §5.I line 1752-1758 mandates one of these two for every SPSC inbox."}"#,
            ),
            (
                // RFC §synth-5-I lines 1755-1756: codegen-invariant
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
                r#"{"v":1,"id":"fnv1a:b605193320262358","code":"worker/inbox-ordering-relaxed-across-cores","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.I","message":"worker 'rx_loop': <sce:inbox ordering=\"relaxed\"> declared but deploy.placement pins producer on core 0 and consumer on core 1. Cross-core SPSC inboxes require acquire/release pairing on head/tail (per spec §5.I lines 1752-1758). Replace with `ordering=\"acq_rel\"` or co-locate producer + consumer on the same core via deploy.placement.","actual":"relaxed"}"#,
            ),
            (
                // RFC §synth-5-D line 912: forge-side anchor for scheduler
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
                r#"{"v":1,"id":"fnv1a:39f753b9c4918241","code":"worker/scheduler-unsupported","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.D","message":"worker 'rx_loop': not declared in deploy.yaml under `machines.mcu_node.workers`. SCE Protocol-Synthesis RFC §5.D line 912 (`worker/scheduler-unsupported`) — the cooperative scheduler tracks one tick slot per declared worker; an undeclared worker has no slot. Repair: add `rx_loop:` under `machines.mcu_node.workers:` in deploy.yaml, or remove the Worker doc from the build.","actual":"rx_loop"}"#,
            ),
            // ── §synth-5-D worker outbox cross-resolution ──
            (
                // RFC §synth-5-D worker outbox: owner segment not in
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
                r#"{"v":1,"id":"fnv1a:99378501ddb9866e","code":"worker/outbox-ref-unknown","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.D","message":"worker 'rx_loop': <sce:outbox ref=\"sesion_fsm.inbox\"> names owner 'sesion_fsm' which is not a registered statechart or worker. Declare the recipient as a separate `.scxml` document in this build (statechart: `<scxml name=\"sesion_fsm\">`; worker: `<scxml sce:kind=\"worker\" name=\"sesion_fsm\">`), or replace the ref with one of the registered recipients: session_fsm.inbox, tx_loop.inbox.","actual":"sesion_fsm.inbox","fix":{"kind":"replace_one_of","candidates":["session_fsm.inbox","tx_loop.inbox"]}}"#,
            ),
            (
                // RFC §synth-5-D worker outbox: owner found but kind not
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
                r#"{"v":1,"id":"fnv1a:1efb5e754cd03d3d","code":"worker/outbox-target-wrong-kind","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.D","message":"worker 'rx_loop': <sce:outbox ref=\"udp_scout.inbox\"> names 'udp_scout' which is registered as a link kind, not a statechart or worker. Outbox refs may only target statechart or worker inboxes (RFC §5.D line 911 \"any non-inbox access\" by negation admits inbox access on statechart + worker kinds). Replace with one of: session_fsm.inbox, tx_loop.inbox.","actual":"udp_scout.inbox","fix":{"kind":"replace_one_of","candidates":["session_fsm.inbox","tx_loop.inbox"]}}"#,
            ),
            (
                // RFC §synth-5-D worker outbox: suffix !=  `inbox` per
                // the strict-suffix rule. Deterministic
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
                r#"{"v":1,"id":"fnv1a:a8a2cabe814a473a","code":"worker/outbox-target-suffix-invalid","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.D","message":"worker 'rx_loop': <sce:outbox ref=\"session_fsm.inbx\"> declares suffix 'inbx' but the only legal suffix is 'inbox' (RFC §5.D line 895 example: `<owner>.inbox`; spec line 1998 codegen table fixes the recipient queue name to `inbox`). Replace with `session_fsm.inbox`.","actual":"session_fsm.inbx","fix":{"kind":"replace_with","to":"session_fsm.inbox"}}"#,
            ),
            (
                // RFC §synth-5-D line 909 C1: forge-side anchor. Per-doc
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
                r#"{"v":1,"id":"fnv1a:a84c33faf2495d72","code":"timer/period-below-tick-rate","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.D","message":"timer 'keepalive': <sce:period> = 500 us is shorter than scheduler.tick_period_us = 1000 us on machine 'mcu_node'. SCE Protocol-Synthesis RFC §5.D line 909 (`timer/period-below-tick-rate`) — the cooperative scheduler dispatches at most one timer per tick, so a period below the tick rate would miss every other deadline. Repair: raise `<sce:period>` to >= 1000us, or lower `scheduler.tick_period_us` (warning: lowering tick rate increases scheduler overhead), or switch the target machine to `scheduler.kind: tokio` / `rt` (preemptive).","expected":["1000"],"actual":"500"}"#,
            ),
            // ── §synth-5-L Bounded-collection parse-time structure validators (item C6) ──
            (
                // RFC §synth-5-L line 2559: sorted-by ordering with no
                // <sce:index-by> field — codegen has no comparator
                // to lower. NeutralOrDeterministic, no Fix payload.
                "forge/collection-ordering-sorted-requires-index-by",
                ValidationError::CollectionOrderingSortedRequiresIndexBy {
                    collection_name: "local_sub_table".into(),
                }
                .into(),
                // Hash placeholder — byte-stability assertion patches.
                r#"{"v":1,"id":"fnv1a:e42ff124e4f1930b","code":"collection/ordering-sorted-requires-index-by","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.L","message":"bounded-collection 'local_sub_table': <sce:ordering>sorted-by(index-by)</sce:ordering> declared without <sce:index-by field=\"...\"/>. SCE Protocol-Synthesis RFC §5.L line 2559 fixes sorted iteration to the `index-by` field; without it the codegen has no comparator to lower. Repair: add an `<sce:index-by field=\"FIELD\"/>` element naming a field of the element-type struct, or change `<sce:ordering>` to `insertion`."}"#,
            ),
            (
                // RFC §synth-5-L line 2655: oldest-wins policy paired with
                // sorted-by ordering — "oldest" has no meaning when
                // iteration order is comparator-derived. Two equally
                // valid repairs → NeutralOrDeterministic, no Fix.
                "forge/collection-overflow-policy-oldest-wins-requires-ordering-insertion",
                ValidationError::CollectionOverflowPolicyOldestWinsRequiresOrderingInsertion {
                    collection_name: "local_sub_table".into(),
                }
                .into(),
                // Hash placeholder — byte-stability assertion patches.
                r#"{"v":1,"id":"fnv1a:31723798e4ccd410","code":"collection/overflow-policy-oldest-wins-requires-ordering-insertion","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.L","message":"bounded-collection 'local_sub_table': <sce:on-overflow>oldest-wins</sce:on-overflow> requires <sce:ordering>insertion</sce:ordering>, but ordering is `sorted-by(index-by)`. SCE Protocol-Synthesis RFC §5.L line 2655 lists this combination as the explicit anti-pattern: `oldest-wins` presumes a temporal ordering that `sorted-by` replaces with the `index-by` field comparator. Repair: change `<sce:ordering>` to `insertion` (keeps the oldest-wins policy), or change `<sce:on-overflow>` to `reject` / `diagnostic-event`."}"#,
            ),
            // ── §synth-5-M Fragment-reassembly variant parse-time structure validators (item C9) ──
            (
                // RFC §synth-5-M line 2944: <sce:variant>reassembly with no
                // <sce:max-fragments-per-message> sibling — codegen has
                // no fragment-index bitmap width to lower per spec line
                // 2688. NeutralOrDeterministic, no Fix payload.
                "forge/mem-reassembly-pool-variant-missing-max-fragments",
                ValidationError::MemReassemblyPoolVariantMissingMaxFragments {
                    pool_name: "rx_reassembly_pool".into(),
                }
                .into(),
                // Hash placeholder — byte-stability assertion patches.
                r#"{"v":1,"id":"fnv1a:02053e3d69a3f3f4","code":"mem/reassembly-pool-variant-missing-max-fragments","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.M","message":"buffer-pool 'rx_reassembly_pool': <sce:variant>reassembly</sce:variant> declared without <sce:max-fragments-per-message>N</sce:max-fragments-per-message>. SCE Protocol-Synthesis RFC §5.M line 2688 fixes the per-slot fragment-index bitmap width to this value; without it codegen has no upper bound on the per-slot fragment-ID tracking. Repair: add an `<sce:max-fragments-per-message>N</sce:max-fragments-per-message>` element with a positive integer N derived from the wire framer's per-message maximum."}"#,
            ),
            (
                // RFC §synth-5-M line 2945: <sce:variant>reassembly with no
                // <sce:reassembly-timeout-ms> sibling — the reassembly
                // FSM has no Receiving → TimedOut edge timer per spec
                // line 2689. NeutralOrDeterministic, no Fix payload.
                "forge/mem-reassembly-pool-variant-missing-timeout",
                ValidationError::MemReassemblyPoolVariantMissingTimeout {
                    pool_name: "rx_reassembly_pool".into(),
                }
                .into(),
                // Hash placeholder — byte-stability assertion patches.
                r#"{"v":1,"id":"fnv1a:1c6a61294467efab","code":"mem/reassembly-pool-variant-missing-timeout","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.M","message":"buffer-pool 'rx_reassembly_pool': <sce:variant>reassembly</sce:variant> declared without <sce:reassembly-timeout-ms>N</sce:reassembly-timeout-ms>. SCE Protocol-Synthesis RFC §5.M line 2689 fixes the per-slot deadline field to this value; without it the reassembly FSM has no `Receiving → TimedOut` edge timer (`docs/reassembly-fsm.md` §2.4.5). Repair: add an `<sce:reassembly-timeout-ms>N</sce:reassembly-timeout-ms>` element with a positive integer N (milliseconds) derived from link latency budget and acceptable hold time."}"#,
            ),
            // ── §synth-5-M Fragment-reassembly cross-doc validators (items C9 + C13) ──
            (
                // RFC §synth-5-M line 2946: rx-pool slot_size < link mtu_bytes.
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
                r#"{"v":1,"id":"fnv1a:eba6a0d43209c75a","code":"mem/reassembly-slot-size-below-declared-mtu","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.M","message":"buffer-pool 'rx_data_pool' is bound as RX pool for link 'udp_data' on machine 'mcu_node', but `<sce:slot-size>256</sce:slot-size>` is smaller than the link's `mtu_bytes: 512`. SCE Protocol-Synthesis RFC §5.M line 2946 — the slot cannot admit a single full-MTU datagram, so even the non-fragmented happy path fails. Repair: raise `<sce:slot-size>` on pool 'rx_data_pool' to >= 512, lower `mtu_bytes` on link 'udp_data', or bind a different (larger) pool.","expected":["512"],"actual":"256"}"#,
            ),
            (
                // RFC §synth-5-M line 2947-2949: reassembly slot_size < max-fragments × mtu.
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
                r#"{"v":1,"id":"fnv1a:3460f145fdcb9f41","code":"reassembly/max-fragments-insufficient-for-mtu","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.M","message":"reassembly-variant buffer-pool 'rx_reassembly_pool' is bound to link 'udp_data' on machine 'mcu_node', but `<sce:slot-size>1024</sce:slot-size>` cannot hold the worst-case reassembled message: `<sce:max-fragments-per-message>8</sce:max-fragments-per-message> × link.mtu_bytes (512) = 4096` bytes required. SCE Protocol-Synthesis RFC §5.M line 2947-2949 verbatim: `slot_size >= max-fragments-per-message × mtu_bytes` — worst-case message must complete reassembly within declared bounds. Repair: raise `<sce:slot-size>` on pool 'rx_reassembly_pool' to >= 4096, lower `<sce:max-fragments-per-message>`, or lower link `mtu_bytes`.","expected":["4096"],"actual":"1024"}"#,
            ),
            (
                // RFC §synth-5-M line 2950-2952: expected_p99 vs rx_pool slot_size
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
                r#"{"v":1,"id":"fnv1a:79cd242c82f712e0","code":"reassembly/expected-fragmentation-rate-high","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.M","message":"link 'udp_data' on machine 'mcu_node': `expected_p99_bytes: 1024` exceeds RX pool 'rx_data_pool' `<sce:slot-size>700</sce:slot-size>` by more than the 25% default stage-copy threshold (rate = 31%). SCE Protocol-Synthesis RFC §5.M line 2950-2952 — `(expected_p99_bytes - rx_pool.slot_size) / expected_p99_bytes > 0.25` triggers the warning. Repair: raise `<sce:slot-size>` on pool 'rx_data_pool', lower `expected_p99_bytes` (with justification), or add `<sce:accept-stage-copy-rate>` on the link source.","expected":["25"],"actual":"31"}"#,
            ),
            (
                // RFC §synth-5-M line 2964-2969: reassembly pool bound to a
                // link with trust_class != established_session. Hard error.
                // NeutralOrDeterministic (two valid
                // repairs: change trust_class OR remove binding).
                "forge/reassembly-untrusted-link-binding",
                ValidationError::ReassemblyUntrustedLinkBinding {
                    pool_name: "rx_reassembly_pool".into(),
                    trust_class: "session_arming".into(),
                    machine: "mcu_node".into(),
                    link_name: "udp_listener".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:9f94349333d3b9ad","code":"reassembly/untrusted-link-binding","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.M","message":"reassembly-variant buffer-pool 'rx_reassembly_pool' is bound to link 'udp_listener' on machine 'mcu_node', but the link declares `trust_class: session_arming`. SCE Protocol-Synthesis RFC §5.M line 2964-2969 — only `trust_class: established_session` links may carry fragmented traffic; reassembly on `untrusted` / `session_arming` links exposes the per-peer quota space to source-IP spoofing. Repair: change link 'udp_listener' to `trust_class: established_session` (only if the link is in fact post-handshake), or remove the reassembly-pool binding.","actual":"session_arming"}"#,
            ),
            (
                // RFC §synth-5-M line 2970-2975: domain_attrs absent on a
                // link with reassembly-pool binding. Hard error.
                // NeutralOrDeterministic (two valid repairs).
                "forge/reassembly-trust-class-missing-on-fragmenting-link",
                ValidationError::ReassemblyTrustClassMissingOnFragmentingLink {
                    pool_name: "rx_reassembly_pool".into(),
                    machine: "mcu_node".into(),
                    link_name: "udp_data".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:d09ad5b8d7cdc514","code":"reassembly/trust-class-missing-on-fragmenting-link","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.M","message":"reassembly-variant buffer-pool 'rx_reassembly_pool' is bound to link 'udp_data' on machine 'mcu_node', but the link does not declare `domain_attrs.trust_class`. SCE Protocol-Synthesis RFC §5.M line 2970-2975 — build cannot decide whether the binding is safe without a declared trust class. Repair: declare `domain_attrs: { trust_class: established_session }` on link 'udp_data' (data-plane links), or remove the reassembly-pool binding (control-plane links)."}"#,
            ),
            (
                // RFC §synth-5-M line 2995-2999: stage-copy WCET vs slot budget.
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
                r#"{"v":1,"id":"fnv1a:c41a9b3397849183","code":"reassembly/stage-copy-wcet-exceeds-slot-budget","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.M","message":"link 'udp_data' on machine 'mcu_node': stage-copy WCET (1365 µs) exceeds `scheduler.worker_slot_budget_us: 200`. SCE Protocol-Synthesis RFC §5.M line 2995-2999 — `expected_p99_bytes (16384) × memcpy_cycles_per_byte (4) / clock_freq_mhz (48) > worker_slot_budget_us`. The stage copy alone starves Keepalive and parallel-region timers (ARCHITECTURE §9.3 + §3.4). Repair: raise `worker_slot_budget_us` (and re-validate every algorithm), lower `expected_p99_bytes` so stage copy is never invoked at that size, or raise the bound pool's `<sce:slot-size>` to absorb p99 without invoking stage copy.","expected":["200"],"actual":"1365"}"#,
            ),
            (
                // RFC §synth-5-M line 2976-2981: codegen self-check —
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
                r#"{"v":1,"id":"fnv1a:c88b0fe4fe0b1ff7","code":"reassembly/peer-id-not-zid-on-established-session","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.M","message":"reassembly-variant buffer-pool 'rx_reassembly_pool' (rust backend): emitted per-slot peer-id is not the 16-byte ZID signature required for `trust_class: established_session` bindings. SCE Protocol-Synthesis RFC §5.M line 2976-2981 — codegen invariant violation: per-peer quota check must use the handshake-derived ZID as the peer key, not the wire source address (defends against UDP source-IP spoofing on `established_session` links). In well-formed templates the reassembly variant always emits the 16-byte ZID typedef (the cross-doc validator `reassembly/untrusted-link-binding` gates non-`established_session` bindings upstream), so this diagnostic fires only on template regression; report at https://github.com/newmassrael/scxml-core-engine/issues"}"#,
            ),
            (
                // RFC §synth-5-C lines 849-856: codegen self-check —
                // listener-link must emit both Listener + Sibling
                // halves. NeutralOrDeterministic; pure template-
                // regression guard (mirrors
                // `reassembly/peer-id-not-zid-on-established-session`).
                "forge/link-listener-link-not-paired-with-established-sibling",
                ValidationError::LinkListenerLinkNotPairedWithEstablishedSibling {
                    link_name: "udp_listener".into(),
                    language: "rust".into(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:1018399a1345bb35","code":"link/listener-link-not-paired-with-established-sibling","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.C","message":"link 'udp_listener' (rust backend): listener-link sibling emission missing the `established_session` half. SCE Protocol-Synthesis RFC §5.C lines 849-856 — codegen invariant violation: every `session_arming` listener must emit its paired `established_session` sibling so per-peer dispatch retains a stable codegen-time identity (re-introduces OQ-W22 if dropped). In well-formed templates the diagnostic never fires (the per-language link template emits both halves unconditionally when `listener_links` contains this name); report at https://github.com/newmassrael/scxml-core-engine/issues"}"#,
            ),
            (
                // RFC §synth-5-M lines 2982-2994: reassembly binding on
                // session_arming link without paired sibling.
                // NeutralOrDeterministic; two valid repair paths under
                // the explicit-role contract (declare explicit role on both sides
                // OR remove the binding).
                "forge/reassembly-binding-on-unpaired-listener",
                ValidationError::ReassemblyBindingOnUnpairedListener {
                    pool_name: "rx_reassembly_pool".into(),
                    machine: "mcu_node".into(),
                    link_name: "udp_listener".into(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:c9d90099f8ed9a01","code":"reassembly/binding-on-unpaired-listener","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.M","message":"reassembly-variant buffer-pool 'rx_reassembly_pool' is bound to link 'udp_listener' on machine 'mcu_node'; the link declares `trust_class: session_arming` but its machine source SCXML did not pair with a listener-role declaration (deploy `role: listener` + SCXML `<sce:session-role kind=\"accept-side\"/>`), so codegen cannot synthesize the paired `established_session` sibling. SCE Protocol-Synthesis RFC §5.M lines 2982-2994 — only listeners (the explicit deploy/SCXML role pair) auto-rebind a `session_arming` reassembly binding to the `established_session` sibling; without that pairing the binding has no valid landing site. Repair: declare `role: listener` on the deploy link AND add `<sce:session-role kind=\"accept-side\"/>` to machine 'mcu_node's source SCXML (making link 'udp_listener' a real listener so the sibling auto-synthesizes), or remove the reassembly-pool binding from link 'udp_listener'."}"#,
            ),
            (
                // RFC §synth-5-N line 3062: cross-doc link has inbound
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
                r#"{"v":1,"id":"fnv1a:d927379bfa06a12a","code":"link/inbound-event-queue-unsized","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.N","message":"link 'udp_listener' on machine 'mcu_node': declares 3 inbound event(s) but no downstream FSM event-queue capacity is bound. SCE Protocol-Synthesis RFC §5.N line 3062 — link declared but downstream FSM inbox depth unset. Repair: add `<scxml sce:capacity=\"N\">` to machine 'mcu_node's source SCXML (per-instance), or add `scheduler.default_event_queue_capacity: N` under `machines.mcu_node` (per-machine fallback).","actual":"3"}"#,
            ),
            // ── §synth-5-K stage-copy policy promotion + opt-out rejection ──
            (
                // RFC §synth-5-K line 2504-2511: warning promoted to hard error.
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
                r#"{"v":1,"id":"fnv1a:c91f1db81740d284","code":"pool/stage-copy-policy-error","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.K","message":"link 'udp_data' on machine 'mcu_node': `expected_p99_bytes: 1024` vs RX pool 'rx_data_pool' `<sce:slot-size>700</sce:slot-size>` triggers stage-copy rate 31% (> 25% threshold), promoted to hard error under `pool_defaults.stage_copy_policy: error`. SCE Protocol-Synthesis RFC §5.K line 2504-2511 — author resolution: raise `<sce:slot-size>` on pool 'rx_data_pool', lower `expected_p99_bytes`, or add `<sce:accept-stage-copy-rate>` on link 'udp_data' (last option unavailable under `forbid`).","expected":["25"],"actual":"31"}"#,
            ),
            (
                // RFC §synth-5-K line 2512-2516: forbid rejects the opt-out outright.
                // NeutralOrDeterministic; two valid repair paths.
                "forge/pool-stage-copy-accept-rejected-under-forbid",
                ValidationError::PoolStageCopyAcceptRejectedUnderForbid {
                    machine: "mcu_node".into(),
                    link_name: "udp_data".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:c4ce58defa7ffd7f","code":"pool/stage-copy-accept-rejected-under-forbid","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.K","message":"link 'udp_data' on machine 'mcu_node': `<sce:accept-stage-copy-rate>` declared but `pool_defaults.stage_copy_policy: forbid` rejects the opt-out outright. SCE Protocol-Synthesis RFC §5.K line 2512-2516 — only structural fixes (raise `<sce:slot-size>` or lower `expected_p99_bytes`) are accepted under `forbid`. Repair: remove `<sce:accept-stage-copy-rate>` from link 'udp_data', or change `pool_defaults.stage_copy_policy` to `error` (which permits the opt-out).","actual":"<sce:accept-stage-copy-rate>"}"#,
            ),
            // ── §synth-5-L Bounded-collection cross-doc resolution (item C6) ──
            (
                // RFC §synth-5-L lines 2566-2567: element-type body text does
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
                r#"{"v":1,"id":"fnv1a:653bda4f395fa2f4","code":"collection/element-type-not-a-kind","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.L","message":"bounded-collection 'local_sub_table': <sce:element-type>subscrription_entry</sce:element-type> does not name a codec-kind struct or procedure-kind state record in this build. SCE Protocol-Synthesis RFC §5.L line 2566-2567 — element types must reference another forge kind by name (codec for byte-encoded structs, procedure for stateful records). Declare the element type as a separate `.scxml` document (codec: `<scxml sce:kind=\"codec\" name=\"subscrription_entry\">`; procedure: `<scxml sce:kind=\"procedure\" name=\"subscrription_entry\">`), or replace the body text with one of the registered candidates: router_handle, subscription_entry.","actual":"subscrription_entry","fix":{"kind":"replace_one_of","candidates":["router_handle","subscription_entry"]}}"#,
            ),
            (
                // RFC §synth-5-L line 2615: index-by field absent from the
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
                r#"{"v":1,"id":"fnv1a:4a9fd47c8aae2381","code":"collection/index-by-field-missing","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.L","message":"bounded-collection 'local_sub_table': <sce:index-by field=\"key_id\"/> names a field that does not exist on element-type 'subscription_entry' (codec kind). SCE Protocol-Synthesis RFC §5.L line 2615 — the `index-by` field enables `find_by_index(IndexKey)` and must name an actual struct field of the element type. Replace `field=\"key_id\"` with one of the subscription_entry's declared fields: callback_id, key_expr_id.","actual":"key_id","fix":{"kind":"replace_one_of","candidates":["callback_id","key_expr_id"]}}"#,
            ),
            (
                // RFC §synth-5-L lines 2560-2562: multi-writer concurrency
                // declared without any §synth-5-I atomic intrinsic imported
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
                r#"{"v":1,"id":"fnv1a:b944cb27eabbfac8","code":"collection/multi-writer-without-atomics","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.L","message":"bounded-collection 'local_sub_table': <sce:concurrency>multi-writer</sce:concurrency> requires at least one §5.I atomic intrinsic to be declared via <sce:extern> somewhere in this build. SCE Protocol-Synthesis RFC §5.L lines 2560-2562 — multi-writer codegen lowers to acquire/release atomics on head/tail; the build's <sce:extern> trust-surface must acknowledge atomic intrinsics for codegen to emit them. Repair: either declare an atomic intrinsic via <sce:extern> (e.g. `<sce:extern name=\"sce_atomic_load_acquire_u32\" sig=\"(*const u32) -> u32\" abi=\"c\"/>` in any forge doc in this build), or change `<sce:concurrency>` to `single-writer`."}"#,
            ),
            // ── §synth-5-L Bounded-collection deploy-time capacity resolution (item C6) ──
            (
                // RFC §synth-5-L lines 2583-2585: `<sce:capacity source="deploy"
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
                r#"{"v":1,"id":"fnv1a:4604decf96012397","code":"collection/capacity-unresolved","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.L","message":"bounded-collection 'local_sub_table': <sce:capacity source=\"deploy\" key=\"machines.mcu_node.limits.local_subscriptions\"/> references limit 'local_subscriptions' on machine 'mcu_node', but deploy.yaml does not declare `machines.mcu_node.limits.local_subscriptions`. SCE Protocol-Synthesis RFC §5.L lines 2583-2585 — `<sce:capacity source=\"deploy\">` resolves at codegen time to a per-language compile-time constant from `machines.<machine>.limits.<limit>:`; an unresolved limit blocks emission. Repair: declare `local_subscriptions: <count>` under `machines.mcu_node.limits:` in deploy.yaml (declared limits today: in_flight_reassembly, subscription_table), or switch the BC's `<sce:capacity>` to `const=\"N\"`.","actual":"machines.mcu_node.limits.local_subscriptions","fix":{"kind":"replace_one_of","candidates":["in_flight_reassembly","subscription_table"]}}"#,
            ),
            // ── §synth-5-I `<sce:extern>` whitelisted intrinsic registry ──
            (
                // RFC §synth-5-I line 1847: symbol absent from the §synth-5-I baseline
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
                r#"{"v":1,"id":"fnv1a:173e706863ec82de","code":"extern/symbol-not-in-whitelist","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.I","message":"<sce:extern name=\"sce_atomic_compare_exchange_u32\"> references a symbol that is not on the §5.I baseline whitelist. Choose a registry-listed name (closest matches: sce_atomic_cas_strong_acq_rel_u32, sce_atomic_cas_weak_acq_rel_u32) or extend the whitelist via a target plugin (deploy.yaml `extern_symbols.target_plugin`).","actual":"sce_atomic_compare_exchange_u32","fix":{"kind":"replace_one_of","candidates":["sce_atomic_cas_strong_acq_rel_u32","sce_atomic_cas_weak_acq_rel_u32"]}}"#,
            ),
            (
                // RFC §synth-5-I line 1848: ABI mismatch — closed two-element
                // repair set [`c`, `rust`] rides `Fix::ReplaceOneOf`.
                "forge/extern-abi-mismatch",
                ValidationError::ExternAbiMismatch {
                    name: "sce_atomic_load_acquire_u32".into(),
                    expected: "c".into(),
                    actual: "rust".into(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:c41d4f3a5258039c","code":"extern/abi-mismatch","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.I","message":"<sce:extern name=\"sce_atomic_load_acquire_u32\" abi=\"rust\"> uses a non-canonical ABI; the registry entry requires `abi=\"c\"`. The accepted set is [\"c\", \"rust\"].","expected":["c"],"actual":"rust","fix":{"kind":"replace_one_of","candidates":["c","rust"]}}"#,
            ),
            (
                // RFC §synth-5-I line 1849: signature mismatch — `Fix::ReplaceWith`
                // carries the canonical sig (registry is source of truth).
                "forge/extern-signature-mismatch",
                ValidationError::ExternSignatureMismatch {
                    name: "sce_atomic_load_acquire_u32".into(),
                    expected: "(*const u32) -> u32".into(),
                    actual: "(*const u32) -> u64".into(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:36a8a1a0fbe59da6","code":"extern/signature-mismatch","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.I","message":"<sce:extern name=\"sce_atomic_load_acquire_u32\" sig=\"(*const u32) -> u64\"> declares a signature that does not match the registry entry. Replace with `sig=\"(*const u32) -> u32\"`.","expected":["(*const u32) -> u32"],"actual":"(*const u32) -> u64","fix":{"kind":"replace_with","to":"(*const u32) -> u32"}}"#,
            ),
            (
                // RFC §synth-5-I line 1850: atomic-family base without ordering
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
                r#"{"v":1,"id":"fnv1a:4e511900e9aed2c8","code":"extern/ordering-unspecified","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.I","message":"<sce:extern name=\"sce_atomic_load\"> is an atomic-family base without an explicit ordering + width suffix. Pick one of: sce_atomic_load_acquire_u32, sce_atomic_load_relaxed_u32.","actual":"sce_atomic_load","fix":{"kind":"replace_one_of","candidates":["sce_atomic_load_acquire_u32","sce_atomic_load_relaxed_u32"]}}"#,
            ),
            (
                // RFC §synth-5-I line 1852: target plugin redefines
                // a baseline whitelist symbol. Additive-
                // composition rule — plugins extend, never override.
                // Repair is non-algorithmic (`fix: None`); plugin author
                // renames to a non-baseline name.
                "forge/extern-target-plugin-symbol-conflict",
                ValidationError::ExternTargetPluginSymbolConflict {
                    name: "sce_atomic_load_acquire_u32".into(),
                    plugin_path: "configs/target_extensions_stm32h7.yaml".into(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:0133c811a527ab82","code":"extern/target-plugin-symbol-conflict","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.I","message":"target plugin configs/target_extensions_stm32h7.yaml redefines core whitelist symbol `sce_atomic_load_acquire_u32`. Plugin entries extend the §5.I baseline registry but cannot override it (additive composition — extend, never override). Rename the plugin entry to a name not already in the §5.I baseline; for a platform-specific impl, declare the entry under a vendor-prefixed name (e.g. `sce_hw_<symbol>`) and route through the registry entry's `crate` field.","actual":"sce_atomic_load_acquire_u32"}"#,
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
                // A standard method name the datamodel has no
                // implementation for. The candidate set is what the
                // emitter can lower, so a consumer repairing this does
                // not need Appendix B.2 open.
                "forge/expression-unsupported-builtin",
                ExprError::UnsupportedBuiltin {
                    name: ".map()".into(),
                    available: vec![".join()".into(), ".push()".into()],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:136a075615baf31b","code":"expression/unsupported-builtin","stage":"expression","spec":"W3C SCXML §B.2","message":".map() is not provided by SCE's ECMAScript datamodel. Available: .join(), .push()","actual":".map()","fix":{"kind":"replace_one_of","candidates":[".join()",".push()"]}}"#,
            ),
            (
                // A misspelling. The candidates come from the document
                // rather than from the language, which is the whole
                // difference from the entry above.
                "forge/expression-unknown-identifier",
                ExprError::UnknownIdentifier {
                    name: "conut".into(),
                    candidates: vec!["count".into()],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:30e4d0dbd04a0706","code":"expression/unknown-identifier","stage":"expression","spec":"W3C SCXML §B.2","message":"conut is not declared by this document. Did you mean: count?","actual":"conut","fix":{"kind":"replace_one_of","candidates":["count"]}}"#,
            ),
            (
                // The same code with nothing to offer: `candidates` came
                // out empty, so `fix` is absent rather than a choice
                // from nothing (SCE_ERROR_CONTRACT.md §3.1).
                "forge/expression-unknown-identifier-no-candidates",
                ExprError::UnknownIdentifier {
                    name: "wholesaleDistributor".into(),
                    candidates: Vec::new(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:1f905d2454137dec","code":"expression/unknown-identifier","stage":"expression","spec":"W3C SCXML §B.2","message":"wholesaleDistributor is not declared by this document","actual":"wholesaleDistributor"}"#,
            ),
            (
                // A name the datamodel provides, called. The repair is
                // the name itself, so it rides `Fix::ReplaceWith` — the
                // opposite of the two entries above, where the producer
                // had a set and could not pick from it.
                "forge/expression-property-not-callable",
                ExprError::PropertyNotCallable {
                    name: ".length".into(),
                    arguments: 0,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:b4ec9e3bee3acf8b","code":"expression/property-not-callable","stage":"expression","spec":"W3C SCXML §B.2","message":".length holds a value, not a function. Write .length without the call.","actual":".length()","fix":{"kind":"replace_with","to":".length"}}"#,
            ),
            (
                // The same code with the arguments that make the repair
                // the author's to make: `actual` names the property and
                // no `fix` rides, since dropping the call would discard
                // them (SCE_ERROR_CONTRACT.md §3).
                "forge/expression-property-not-callable-with-arguments",
                ExprError::PropertyNotCallable {
                    name: "Math.PI".into(),
                    arguments: 1,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:87ec1b18624df1da","code":"expression/property-not-callable","stage":"expression","spec":"W3C SCXML §B.2","message":"Math.PI holds a value, not a function. Write Math.PI without the call.","actual":"Math.PI"}"#,
            ),
            (
                // A namespace called. Neither of the two entries above
                // fits: the name is provided, so it is not a missing
                // builtin, and it holds nothing, so dropping the call
                // repairs nothing. `expected` carries the members and no
                // `fix` rides.
                "forge/expression-namespace-not-callable",
                ExprError::NamespaceNotCallable {
                    namespace: "JSON".into(),
                    members: vec!["JSON.parse".into(), "JSON.stringify".into()],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:dbf47c756e89d56b","code":"expression/namespace-not-callable","stage":"expression","spec":"W3C SCXML §B.2","message":"JSON is a namespace, not a function. Call one of its members: JSON.parse, JSON.stringify","expected":["JSON.parse","JSON.stringify"],"actual":"JSON()"}"#,
            ),
            (
                // The read half. `actual` is the bare name and the
                // members carry both halves of the vocabulary, which is
                // what separates this record from the one above.
                "forge/expression-namespace-not-a-value",
                ExprError::NamespaceNotAValue {
                    namespace: "Math".into(),
                    members: vec!["Math.PI".into(), "Math.abs".into()],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:234d162120386eee","code":"expression/namespace-not-a-value","stage":"expression","spec":"W3C SCXML §B.2","message":"Math is a namespace, not a value. Reach one of its members: Math.PI, Math.abs","expected":["Math.PI","Math.abs"],"actual":"Math"}"#,
            ),
            (
                // Neither field carries a set: the producer can name
                // what was written and nothing that should replace it.
                "forge/expression-literal-not-callable",
                ExprError::LiteralNotCallable {
                    what: "the number literal".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:9dd8d2ff05d2b95e","code":"expression/literal-not-callable","stage":"expression","spec":"W3C SCXML §B.2","message":"the number literal is not a function","actual":"the number literal"}"#,
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
                r#"{"v":1,"id":"fnv1a:0b54880b78b1e343","code":"expression/invalid-lvalue","stage":"expression","spec":"SCE Forge §3.4","message":"assign location 'call expression' is not an lvalue: cannot assign to a function call"}"#,
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
                r#"{"v":1,"id":"fnv1a:900f455ab62d2801","code":"generate/template-load","stage":"generate","message":"template load error: codec.cpp.jinja2 not found in template dir"}"#,
            ),
            (
                "forge/generate-template-render",
                GenerateError::TemplateRender("undefined variable `fields` at line 12".into())
                    .into(),
                r#"{"v":1,"id":"fnv1a:6fd2874e8cc9c725","code":"generate/template-render","stage":"generate","message":"template render error: undefined variable `fields` at line 12"}"#,
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
                r#"{"v":1,"id":"fnv1a:668662191385c78c","code":"xml/xinclude-malformed","stage":"xml","message":"<xi:include href=\"frag.xml\">: included file is malformed: unexpected end tag","actual":"frag.xml"}"#,
            ),
            (
                "forge/xinclude-unsupported",
                XmlError::XInclude(crate::xinclude::XIncludeError::Unsupported {
                    href: "frag.xml".into(),
                    feature: "parse=\"text\" (only parse=\"xml\" is supported)".to_string(),
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
                r#"{"v":1,"id":"fnv1a:85213b68a3102bc7","code":"xml/template-malformed","stage":"xml","message":"<sce:use template=\"bad.sce-template.xml\">: template is malformed: root element must be <sce:template>, got <not-a-template>","actual":"bad.sce-template.xml"}"#,
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
            (
                "forge/preprocessor-not-run",
                XmlError::PreprocessorNotRun {
                    element: "sce:use".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:91b4616df99fdbbc","code":"xml/preprocessor-not-run","stage":"xml","message":"<sce:use> survived into parsing: the document was never run through preprocessor expansion (expand_preprocessors)","actual":"sce:use"}"#,
            ),
            // ── SCE Protocol-Synthesis RFC §synth-5-J-4 / §synth-5-J-5 codegen matrix shells.
            //    Producer constructors are reachable; the matrix walker
            //    in `forge/codegen_matrix.rs` invokes them. ──
            (
                "forge/codegen-mcu-class-kind-on-non-mcu-language",
                GenerateError::CodegenMcuClassKindOnNonMcuLanguage {
                    kind: "link".into(),
                    language: "kotlin".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:0e78c9c56b3c4d51","code":"codegen/mcu-class-kind-on-non-mcu-language","stage":"generate","message":"MCU-class kind 'link' cannot be lowered to language 'kotlin': only rust and c11 have MCU substrate (SCE Protocol-Synthesis RFC §5.J.4)","actual":"kotlin"}"#,
            ),
            (
                "forge/codegen-generic-kind-backend-emit-missing",
                GenerateError::CodegenGenericKindBackendEmitMissing {
                    kind: "algorithm".into(),
                    language: "python".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:d54c90195c019259","code":"codegen/generic-kind-backend-emit-missing","stage":"generate","message":"generic-class kind 'algorithm': template missing for language 'python' (SCE Protocol-Synthesis RFC §5.J.4 expects all six backends to emit)"}"#,
            ),
            // ── SCE Protocol-Synthesis RFC §synth-5-J-2 Rust no_std variant rejections
            //    (item C3). Author-side `--no-std` gate on
            //    `sce-codegen generate -l rust`. ──
            (
                "forge/codegen-no-std-script-not-supported",
                GenerateError::CodegenNoStdScriptNotSupported {
                    document: "demo".into(),
                    locations: "<script> in state 'init'".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:99d806fa3e160004","code":"codegen/no-std-script-not-supported","stage":"generate","spec":"SCE Protocol-Synthesis RFC §5.J.2","message":"Rust no_std variant rejects `<script>`: document 'demo' uses ECMAScript at <script> in state 'init' (SCE Protocol-Synthesis RFC §5.J.2; sce-rust-runtime no_std feature is incompatible with `script-engine-lua` and `script-engine-quickjs`)"}"#,
            ),
            (
                "forge/codegen-no-std-http-not-supported",
                GenerateError::CodegenNoStdHttpNotSupported {
                    document: "demo".into(),
                    locations: "<send target=\"http://localhost\"> in state 'send_step'".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:1e0ad94889d9bb0c","code":"codegen/no-std-http-not-supported","stage":"generate","spec":"SCE Protocol-Synthesis RFC §5.J.2","message":"Rust no_std variant rejects HTTP send: document 'demo' uses BasicHTTPEventProcessor at <send target=\"http://localhost\"> in state 'send_step' (SCE Protocol-Synthesis RFC §5.J.2; sce-rust-runtime no_std feature is incompatible with `http-send`)"}"#,
            ),
            // ── SCE Protocol-Synthesis RFC §synth-5-J-2 Rust no_std variant rejections
            //    (item C3). Helper runtime cfg-gate companion
            //    pair: filesystem load + invoke. ──
            (
                "forge/codegen-no-std-fs-load-not-supported",
                GenerateError::CodegenNoStdFsLoadNotSupported {
                    document: "demo".into(),
                    locations: "<data id='cfg' src='file:cfg.json'>".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:69b3678acabbba8d","code":"codegen/no-std-fs-load-not-supported","stage":"generate","spec":"SCE Protocol-Synthesis RFC §5.J.2","message":"Rust no_std variant rejects external `<data src>`: document 'demo' loads file content at <data id='cfg' src='file:cfg.json'> (SCE Protocol-Synthesis RFC §5.J.2; filesystem helpers are gated to !no_std and unreachable from emitted code)"}"#,
            ),
            (
                "forge/codegen-no-std-invoke-not-supported",
                GenerateError::CodegenNoStdInvokeNotSupported {
                    document: "demo".into(),
                    locations: "<invoke type='scxml' src='child.scxml'>".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:d46720a4cb2f313e","code":"codegen/no-std-invoke-not-supported","stage":"generate","spec":"SCE Protocol-Synthesis RFC §5.J.2","message":"Rust no_std variant rejects `<invoke>`: document 'demo' invokes child sessions at <invoke type='scxml' src='child.scxml'> (SCE Protocol-Synthesis RFC §5.J.2; invoke processing is gated to !no_std and unreachable from emitted code)"}"#,
            ),
            // ── Algorithm kind sema (SCE Protocol-Synthesis RFC §synth-5-A) ───────
            (
                "forge/algorithm-local-shadows-param",
                ValidationError::AlgorithmLocalShadowsParam {
                    name: "data".into(),
                    what: "param".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:438800138df5f1c2","code":"algorithm/local-shadows-param","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.A","message":"algorithm: identifier 'data' shadows param","actual":"data"}"#,
            ),
            (
                "forge/algorithm-lvalue-unsupported",
                ValidationError::AlgorithmLvalueUnsupported {
                    target: "data".into(),
                    restriction: "algorithm parameters are read-only in v1".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:397b595293daf8ef","code":"algorithm/lvalue-unsupported","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.A","message":"<sce:assign target=\"data\">: algorithm parameters are read-only in v1","actual":"data"}"#,
            ),
            (
                "forge/algorithm-return-missing",
                ValidationError::AlgorithmReturnMissing.into(),
                r#"{"v":1,"id":"fnv1a:e2582bd483bbf621","code":"algorithm/return-missing","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.A","message":"algorithm: signature declares return type but body's last statement is not <sce:return>"}"#,
            ),
            // ── C7-lowering: algorithm-over-BC dispatch goldens
            //    (RFC §synth-5-A line 311 + §synth-5-L lines 2611-2618 + 2642-2647).
            //    Hash placeholders — patched by byte-stability assertion.
            (
                "forge/algorithm-foreach-source-not-iterable",
                ValidationError::AlgorithmForeachSourceNotIterable {
                    src: "missing_bc".into(),
                    candidates: vec!["data".into(), "subs".into()],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:8e088992b058a56d","code":"algorithm/foreach-source-not-iterable","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.A + §5.L","message":"algorithm: <sce:foreach in=\"missing_bc\">: source does not resolve to a bytes param or a bounded-collection import alias","actual":"missing_bc"}"#,
            ),
            (
                "forge/algorithm-call-target-unknown",
                ValidationError::AlgorithmCallTargetUnknown {
                    target: "missing_alias.find_by_index".into(),
                    alias: "missing_alias".into(),
                    candidates: vec!["subs".into()],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:7ca80fadd3ca6709","code":"algorithm/call-target-unknown","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.A + §5.L","message":"algorithm: <sce:call target=\"missing_alias.find_by_index\">: alias 'missing_alias' is not a declared import","actual":"missing_alias","fix":{"kind":"replace_one_of","candidates":["subs"]}}"#,
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
                r#"{"v":1,"id":"fnv1a:18c90b6a637ace44","code":"algorithm/call-target-method-unknown","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.A + §5.L","message":"algorithm: <sce:call target=\"subs.unknown_method\">: method 'unknown_method' is not callable on import 'subs' (kind=bounded-collection)","actual":"unknown_method","fix":{"kind":"replace_one_of","candidates":["capacity","find_by_index","get","get_by_slot","len"]}}"#,
            ),
            (
                "forge/algorithm-bc-mutation-forbidden",
                ValidationError::AlgorithmBcMutationForbidden {
                    target: "subs.insert".into(),
                    method: "insert".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:7fa7fc026ed6fad4","code":"algorithm/bc-mutation-forbidden","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.A + §5.L","message":"algorithm: <sce:call target=\"subs.insert\">: mutating bounded-collection method 'insert' is forbidden from algorithm body (algorithms are pure per RFC §5.A)","actual":"insert"}"#,
            ),
            (
                "forge/algorithm-foreach-source-bc-with-bytes-item-type",
                ValidationError::AlgorithmForeachSourceBcWithBytesItemType {
                    src: "subs".into(),
                    var_name: "b".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:6b9c5ec0f8cf743f","code":"algorithm/foreach-source-bc-with-bytes-item-type","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.A + §5.L","message":"algorithm: <sce:foreach in=\"subs\"> over bounded-collection: body's <sce:var name=\"b\" type=\"uint8\"> uses the bytes-iteration pattern but 'subs' is a bounded-collection (item carries element-type)","actual":"b"}"#,
            ),
            (
                "forge/algorithm-call-arg-count-mismatch",
                ValidationError::AlgorithmCallArgCountMismatch {
                    target: "subs.find_by_index".into(),
                    actual: 2,
                    expected: 1,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:bd464011584cd3c9","code":"algorithm/call-arg-count-mismatch","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.A + §5.L","message":"algorithm: <sce:call target=\"subs.find_by_index\">: argument count 2 does not match callable's arity 1"}"#,
            ),
            // ── Byte-buffer-build (SCE-owned, SCE_FORGE.md §4.12) ─
            (
                "forge/algorithm-append-target-not-buffer",
                ValidationError::AlgorithmAppendTargetNotBuffer {
                    target: "scratch".into(),
                    candidates: vec!["out".into()],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:43e6bffa846c22db","code":"algorithm/append-target-not-buffer","stage":"validation","spec":"SCE Forge §4.12","message":"algorithm: <sce:append target=\"scratch\">: 'scratch' is not a declared bytes buffer (declare it with <sce:var type=\"bytes\" capacity=\"N\"/>)","actual":"scratch"}"#,
            ),
            (
                "forge/algorithm-append-type-mismatch",
                ValidationError::AlgorithmAppendTypeMismatch {
                    target: "out".into(),
                    got: "uint16".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:e63c3542ddb12c1f","code":"algorithm/append-type-mismatch","stage":"validation","spec":"SCE Forge §4.12","message":"algorithm: <sce:append target=\"out\">: expr must be uint8 or bytes, got uint16","expected":["uint8","bytes"],"actual":"uint16"}"#,
            ),
            // ── §synth-5-F build-time const-fold (SCE Protocol-Synthesis RFC §synth-5-F) ─
            (
                "forge/algorithm-const-not-foldable",
                GenerateError::ConstNotFoldable {
                    algorithm: "crc16".into(),
                    const_name: "table".into(),
                    detail: "arithmetic on non-numeric operand".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:f005de566be48738","code":"algorithm/const-not-foldable","stage":"generate","spec":"SCE Protocol-Synthesis RFC §5.F","message":"algorithm 'crc16': <sce:const name=\"table\">: const-not-foldable: arithmetic on non-numeric operand","actual":"table"}"#,
            ),
            (
                "forge/algorithm-const-fold-budget-exceeded",
                GenerateError::ConstFoldBudgetExceeded {
                    algorithm: "crc16".into(),
                    const_name: Some("table".into()),
                    budget: 1_000_000,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:d1dee18e4cb7eeba","code":"algorithm/const-fold-budget-exceeded","stage":"generate","spec":"SCE Protocol-Synthesis RFC §5.F","message":"algorithm 'crc16': <sce:const name=\"table\">: const-fold-budget-exceeded: total iteration count exceeded the configured budget of 1000000 (RFC §5.F bound 1; override with --const-fold-budget=N)","actual":"table"}"#,
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
                r#"{"v":1,"id":"fnv1a:5d993dd11e2f6ddb","code":"algorithm/const-yield-type-mismatch","stage":"generate","spec":"SCE Protocol-Synthesis RFC §5.F","message":"algorithm 'crc16': <sce:const name=\"table\">: const-yield-type-mismatch: cannot coerce float to Uint16","actual":"float"}"#,
            ),
            // ── §synth-5-B variant primitive (SCE Protocol-Synthesis RFC §synth-5-B, item B1) ─
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
                r#"{"v":1,"id":"fnv1a:fb2f4b7fefc04603","code":"codec/variant-arm-unreachable","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.B","message":"codec 'session_envelope': variant on tag 'msg_id' (type uint8) has 3 arm(s) but no <sce:default> declared (tag type domain has 256 values) — at least one tag value would have no matching arm at runtime; add <sce:default type=\"...\"/> or enumerate the missing values explicitly","actual":"msg_id"}"#,
            ),
            // ── RFC variant-default-uniformity — duplicate default-arm marker ─
            (
                "forge/codec-variant-duplicate-default-arm",
                ValidationError::CodecVariantDuplicateDefaultArm {
                    codec: "session_envelope".into(),
                    first_arm_value: 0x01,
                    second_arm_value: 0x02,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:e0a8b8b88b736596","code":"codec/variant-duplicate-default-arm","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.B","message":"codec 'session_envelope': <sce:variant> declares more than one <sce:arm default=\"true\"/> (first arm value=0x1, second arm value=0x2) — only one arm may be marked the Default-trait starting value; remove default=\"true\" from all but the intended arm. (The catch-all <sce:default> element is unrelated and still permitted once.)","actual":"session_envelope"}"#,
            ),
            // ── RFC variant-default-uniformity — outer arm vs inner flag mismatch ─
            (
                "forge/codec-variant-arm-mid-mismatch",
                ValidationError::CodecVariantArmMidMismatch {
                    codec: "session_envelope".into(),
                    arm_value: 0x02,
                    inner_codec: "session_put".into(),
                    inner_flag: "mid".into(),
                    inner_flag_value: 0x01,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:27db9fad022a4f48","code":"codec/variant-arm-mid-mismatch","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.B","message":"codec 'session_envelope': <sce:arm value=0x2/> selects inner codec 'session_put' but that codec declares <sce:flag name='mid' value=0x1/> on its dispatch field — outer arm value and inner flag value must match for round-trip dispatch to resolve to the same arm; align one to the other","expected":["0x2"],"actual":"0x1"}"#,
            ),
            // ── RFC variant-default-uniformity — inner codec missing wire-MID ─
            (
                "forge/codec-variant-arm-inner-mid-undeclared",
                ValidationError::CodecVariantArmInnerMidUndeclared {
                    codec: "session_envelope".into(),
                    arm_value: 0x02,
                    inner_codec: "session_put".into(),
                    expected_flag: "mid".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:0bf3318e19f672aa","code":"codec/variant-arm-inner-mid-undeclared","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.B","message":"codec 'session_envelope': <sce:arm value=0x2/> selects inner codec 'session_put', but 'session_put' does not declare a <sce:flag value=\"...\"/> constant on its dispatch field — the inner's Default would zero-fill the wire byte and break round-trip; add <sce:flag name='mid' value=0x2/> to 'session_put'","expected":["0x2"],"actual":"session_put"}"#,
            ),
            // ── Caller-tag variant shape — variant arm body is caller-tag dispatcher ─
            (
                "forge/codec-variant-arm-body-caller-tag-unsupported",
                ValidationError::CodecVariantArmBodyCallerTagUnsupported {
                    parent_codec: "session_envelope".into(),
                    arm_value: Some(0x03),
                    embedded_alias: "session_inner".into(),
                    embedded_codec: "session_inner".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:8259656376e7288f","code":"codec/variant-arm-body-caller-tag-unsupported","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.B","message":"codec 'session_envelope': variant arm value=0x3 (alias 'session_inner') resolves to codec 'session_inner' whose <sce:variant> is in caller-tag shape (no tag= attribute) — there is no natural source for the inner tag in a variant-arm context. Either add tag=\"<field>\" to 'session_inner' so it reads its tag from its own wire bytes, or expose 'session_inner' via <sce:embed> + <sce:variant-dispatch> on a parent flag instead of as a variant arm body.","expected":["0x3"],"actual":"session_inner"}"#,
            ),
            // ── RFC variant-default-uniformity — no default arm declared ─
            (
                "forge/codec-variant-no-default-arm",
                ValidationError::CodecVariantNoDefaultArm {
                    codec: "session_envelope".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:9778351f6ba08771","code":"codec/variant-no-default-arm","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.B","message":"codec 'session_envelope': <sce:variant> declares no <sce:arm default=\"true\"/> — every variant must mark one arm as the deliberate Default-trait starting value so codegen does not implicitly pick the first declared arm; add default=\"true\" to the intended arm. (The catch-all <sce:default> element is a separate concept and does not satisfy this requirement.)","actual":"session_envelope"}"#,
            ),
            // ── RFC variant-default-overlay — deploy.yaml overlay ─
            (
                "forge/codec-variant-default-overlay-arm-not-declared",
                ValidationError::CodecVariantDefaultOverlayArmNotDeclared {
                    codec: "session_envelope".into(),
                    overlay_arm_value: 0xff,
                    declared_arms: vec![0x01, 0x02, 0x03],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:eeb51f3d4f6560b6","code":"codec/variant-default-overlay-arm-not-declared","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.B","message":"codec 'session_envelope': deploy.yaml variant_defaults names arm value 0xff, but the codec declares no matching <sce:arm value=...> — declared arms: [0x1, 0x2, 0x3]; align the overlay entry with one of the declared values or remove it from variant_defaults","expected":["0xff"],"actual":"session_envelope","fix":{"kind":"replace_one_of","candidates":["0x1","0x2","0x3"]}}"#,
            ),
            // ── Parent-tag dispatch — parent-side variant-dispatch (5 codes) ─
            (
                "forge/codec-variant-dispatch-flag-not-resolved",
                ValidationError::CodecVariantDispatchFlagNotResolved {
                    parent_codec: "codec_zenoh_push".into(),
                    embedded_alias: "key".into(),
                    flag_source: "header.X".into(),
                    detail: "flag 'X' is not declared on carrier 'header'".into(),
                    candidates: vec!["header.M".into(), "header.N".into()],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:70575bd712ca76c4","code":"codec/variant-dispatch-flag-not-resolved","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.B","message":"parent codec 'codec_zenoh_push': <sce:variant-dispatch flag=\"header.X\"/> on import 'key' does not resolve — flag 'X' is not declared on carrier 'header'. Correct the dotted reference to one of: [header.M, header.N].","expected":["header.X"],"actual":"codec_zenoh_push","fix":{"kind":"replace_one_of","candidates":["header.M","header.N"]}}"#,
            ),
            (
                "forge/codec-variant-dispatch-bit-width-mismatch",
                ValidationError::CodecVariantDispatchBitWidthMismatch {
                    parent_codec: "codec_zenoh_push".into(),
                    embedded_alias: "key".into(),
                    embedded_codec: "codec_zenoh_keyexpr".into(),
                    carrier: "header".into(),
                    flag: "M".into(),
                    flag_width: 1,
                    max_values: 2,
                    arm_count: 4,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:e63d7bb728560215","code":"codec/variant-dispatch-bit-width-mismatch","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.B","message":"parent codec 'codec_zenoh_push': <sce:variant-dispatch flag=\"header.M\"/> on import 'key' (codec 'codec_zenoh_keyexpr') — flag width 1 can encode at most 2 dispatch values, but the imported codec declares 4 arms. Widen the flag or reduce the arm count.","expected":["flag width ≥ ceil(log2(4))"],"actual":"width=1 (max 2 values) vs 4 arms"}"#,
            ),
            (
                "forge/codec-variant-dispatch-arms-not-distinguishable-without-default",
                ValidationError::CodecVariantDispatchArmsNotDistinguishableWithoutDefault {
                    parent_codec: "codec_zenoh_decl_kexpr".into(),
                    embedded_alias: "key".into(),
                    embedded_codec: "codec_zenoh_keyexpr".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:bbab0d15f3268277","code":"codec/variant-dispatch-arms-not-distinguishable-without-default","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.B","message":"parent codec 'codec_zenoh_decl_kexpr': import 'key' (codec 'codec_zenoh_keyexpr') is a variant codec but the import declares no <sce:variant-dispatch> and the imported codec has no <sce:arm default=\"true\"/> marker. Add <sce:variant-dispatch flag=\"...\"/> to the import, or mark one arm in 'codec_zenoh_keyexpr' as default=\"true\".","expected":["<sce:variant-dispatch flag=\"...\"/> on the import OR <sce:arm default=\"true\"/> in the imported codec"],"actual":"codec_zenoh_keyexpr"}"#,
            ),
            (
                "forge/codec-variant-dispatch-flag-has-static-value",
                ValidationError::CodecVariantDispatchFlagHasStaticValue {
                    parent_codec: "codec_zenoh_push".into(),
                    embedded_alias: "key".into(),
                    carrier: "header".into(),
                    flag: "M".into(),
                    static_value: 1,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:1fdcd474ff7c7fc7","code":"codec/variant-dispatch-flag-has-static-value","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.B","message":"parent codec 'codec_zenoh_push': flag 'header.M' has static <sce:flag value=0x1/>, but <sce:variant-dispatch flag=\"header.M\"/> on import 'key' would derive the same bit from the variant's arm choice — static and derived cannot coexist. Remove the value= constant or move the dispatch to a different flag.","actual":"codec_zenoh_push.header.M=0x1"}"#,
            ),
            (
                "forge/codec-variant-dispatch-carrier-after-embed",
                ValidationError::CodecVariantDispatchCarrierAfterEmbed {
                    parent_codec: "codec_zenoh_push".into(),
                    embedded_alias: "key".into(),
                    embedded_field: "key".into(),
                    carrier: "header".into(),
                    flag: "M".into(),
                    carrier_index: 1,
                    embedded_index: 0,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:ca182b36796032f0","code":"codec/variant-dispatch-carrier-after-embed","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.B","message":"parent codec 'codec_zenoh_push': field 'key' (import 'key') has <sce:variant-dispatch flag=\"header.M\"/>, but carrier 'header' is declared at field index 1 which is AFTER the embed field at index 0. Reorder fields so 'header' precedes 'key'.","expected":["carrier 'header' before field 'key'"],"actual":"carrier at index 1, embed at index 0"}"#,
            ),
            // ── Flag inversion — parent-side flag-bind validators ─
            (
                "forge/codec-flag-bind-input-not-declared",
                ValidationError::CodecFlagBindInputNotDeclared {
                    parent_codec: "codec_zenoh_request".into(),
                    embedded_alias: "key".into(),
                    embedded_codec: "codec_zenoh_wireexpr".into(),
                    input: "is_admin".into(),
                    available_inputs: "has_suffix".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:218d5e66e1cbff7a","code":"codec/flag-bind-input-not-declared","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.B","message":"codec 'codec_zenoh_request': <sce:flag-bind input=\"is_admin\"/> on <sce:import as=\"key\"> targets a leaf-side input that 'codec_zenoh_wireexpr' does not declare. Available inputs on the imported leaf: [has_suffix]. Align the bind's input= attribute with a declared <sce:flag-input name=\"…\">, or remove the bind if the leaf no longer needs that input.","expected":["is_admin"],"actual":"declared on codec_zenoh_wireexpr: [has_suffix]","fix":{"kind":"replace_one_of","candidates":["has_suffix"]}}"#,
            ),
            (
                "forge/codec-flag-bind-source-not-resolved",
                ValidationError::CodecFlagBindSourceNotResolved {
                    parent_codec: "codec_zenoh_request".into(),
                    embedded_alias: "key".into(),
                    input: "has_suffix".into(),
                    bind_source: "header.K".into(),
                    detail: "flag 'K' is not declared on local carrier 'header'".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:52e7c2d46ebfffe6","code":"codec/flag-bind-source-not-resolved","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.B","message":"codec 'codec_zenoh_request': <sce:flag-bind input=\"has_suffix\" source=\"header.K\"/> on <sce:import as=\"key\"> cannot be resolved against this codec's namespace. flag 'K' is not declared on local carrier 'header'. Use <carrier>.<flag> form to reference a local flags-carrier flag, or the bare input name to forward one of this codec's own <sce:flag-input> declarations.","expected":["header.K"],"actual":"codec_zenoh_request"}"#,
            ),
            (
                "forge/codec-flag-bind-width-mismatch",
                ValidationError::CodecFlagBindWidthMismatch {
                    parent_codec: "codec_zenoh_request".into(),
                    embedded_alias: "key".into(),
                    input: "has_suffix".into(),
                    bind_source: "header.priority".into(),
                    source_width: 3,
                    input_width: 1,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:29ec86e6e42266e2","code":"codec/flag-bind-width-mismatch","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.B","message":"codec 'codec_zenoh_request': <sce:flag-bind input=\"has_suffix\" source=\"header.priority\"/> on <sce:import as=\"key\"> has source width 3 but leaf-side input 'has_suffix' declares width 1. v1 lock-in fixes flag-input width at 1; multi-bit inputs defer to a reachable consumer.","expected":["input width 1"],"actual":"source width 3"}"#,
            ),
            (
                "forge/codec-flag-input-unbound",
                ValidationError::CodecFlagInputUnbound {
                    parent_codec: "codec_zenoh_request".into(),
                    embedded_alias: "key".into(),
                    embedded_codec: "codec_zenoh_wireexpr".into(),
                    input: "has_suffix".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:6f64a9b2f3e7bfd3","code":"codec/flag-input-unbound","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.B","message":"codec 'codec_zenoh_request': <sce:import as=\"key\"> imports 'codec_zenoh_wireexpr' which declares <sce:flag-input name=\"has_suffix\"/> but no matching <sce:flag-bind input=\"has_suffix\"/> is supplied. Bind the input to one of this codec's local flags-carrier flags (<sce:flag-bind input=\"has_suffix\" source=\"carrier.flag\"/>) or to one of this codec's own <sce:flag-input> declarations (<sce:flag-bind input=\"has_suffix\" source=\"local_input\"/>).","expected":["<sce:flag-bind input=\"has_suffix\" source=\"...\"/>"],"actual":"codec_zenoh_wireexpr declares <sce:flag-input name=\"has_suffix\"/>","fix":{"kind":"add_attribute","element":"<sce:import as=\"key\">","attr":"<sce:flag-bind input=\"has_suffix\" source=\"...\"/>"}}"#,
            ),
            (
                "forge/codec-flag-bind-duplicate-input",
                ValidationError::CodecFlagBindDuplicateInput {
                    parent_codec: "codec_zenoh_request".into(),
                    embedded_alias: "key".into(),
                    input: "has_suffix".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:171f17f551b318d7","code":"codec/flag-bind-duplicate-input","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.B","message":"codec 'codec_zenoh_request': <sce:import as=\"key\"> has duplicate <sce:flag-bind input=\"has_suffix\"/> declarations. Each leaf-side input may be bound at most once per import site.","actual":"key.has_suffix"}"#,
            ),
            (
                "forge/codec-flag-bind-carrier-after-embed",
                ValidationError::CodecFlagBindCarrierAfterEmbed {
                    parent_codec: "codec_zenoh_request".into(),
                    embedded_alias: "key".into(),
                    embedded_field: "key".into(),
                    input: "has_suffix".into(),
                    carrier: "header".into(),
                    flag: "N".into(),
                    carrier_index: 1,
                    embedded_index: 0,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:3bb9387760da5afb","code":"codec/flag-bind-carrier-after-embed","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.B","message":"codec 'codec_zenoh_request': <sce:flag-bind input=\"has_suffix\" source=\"header.N\"/> on <sce:import as=\"key\"> references a carrier 'header' declared at field-index 1 but the embed 'key' (which consumes the bound input) is at field-index 0. Streaming decode requires carrier to precede consumer — reorder the fields so 'header' is declared before 'key'.","expected":["carrier 'header' before field 'key'"],"actual":"carrier at index 1, embed at index 0"}"#,
            ),
            // ── §synth-5-B present-if primitive (SCE Protocol-Synthesis RFC §synth-5-B, item B1) ─
            (
                "forge/codec-present-if-refs-later-field",
                ValidationError::CodecPresentIfRefsLaterField {
                    codec: "session_envelope".into(),
                    field: "key".into(),
                    refers_to: "trailer_flags".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:f6f6bd818f8e28d9","code":"codec/present-if-refs-later-field","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.B","message":"codec 'session_envelope': field 'key' has sce:present-if=\"trailer_flags.…\" but 'trailer_flags' is not declared earlier in this codec — present-if predicates must reference a flags-bearing carrier that the streaming decoder has already consumed; reorder the fields so the carrier comes first, or correct the predicate","actual":"key"}"#,
            ),
            // ── §synth-5-B repeat primitive (SCE Protocol-Synthesis RFC §synth-5-B, B2) ─
            (
                "forge/codec-repeat-count-refs-later-field",
                ValidationError::CodecRepeatCountRefsLaterField {
                    codec: "fragment_burst".into(),
                    field: "frags".into(),
                    refers_to: "num_frags".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:f1e97717ebdba39a","code":"codec/repeat-count-refs-later-field","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.B","message":"codec 'fragment_burst': repeat field 'frags' has sce:count=\"num_frags\" but 'num_frags' is not declared earlier in this codec — repeat count references must resolve to a sibling integer field that the streaming decoder has already consumed; reorder the fields so the count comes first, or correct the attribute","actual":"frags"}"#,
            ),
            // ── §synth-5-B test-vector primitive (SCE Protocol-Synthesis RFC §synth-5-B, items B2 + B5) ─
            (
                "forge/algorithm-test-vector-unsupported-kind",
                ValidationError::TestVectorUnsupportedKind {
                    name: "session_filter".into(),
                    kind: ForgeKind::Filter,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:b10188077e684de4","code":"algorithm/test-vector-unsupported-kind","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.B","message":"<sce:test-vector> is only supported on sce:kind=\"algorithm\" and sce:kind=\"codec\", but 'session_filter' declares sce:kind=\"Filter\" — move the test vector to an algorithm/codec file or use the kind-specific harness oracle","actual":"session_filter"}"#,
            ),
            // ── §synth-5-B B3 TLV chain primitive (SCE Protocol-Synthesis RFC §synth-5-B) ─
            (
                "forge/codec-tlv-chain-depth-unspecified",
                ValidationError::CodecTlvChainDepthUnspecified {
                    codec: "session_envelope".into(),
                    field: "extensions".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:569cbc9e420b26e4","code":"codec/tlv-chain-depth-unspecified","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.B","message":"codec 'session_envelope': tlv-chain field 'extensions' is missing the required `max-depth` attribute — TLV chain decoders need a build-time bound to size their working set and enforce iterative-only parse (RFC §5.B line 488); add `max-depth=\"N\"` for some N > 0","actual":"extensions"}"#,
            ),
            (
                "forge/codec-tlv-chain-truncate-under-entry-flag",
                ValidationError::CodecTlvChainTruncateUnderEntryFlag {
                    codec: "session_envelope".into(),
                    field: "extensions".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:66aacd8d5b7ac12e","code":"codec/tlv-chain-truncate-under-entry-flag","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.B","message":"codec 'session_envelope': tlv-chain field 'extensions' declares `on-overflow=\"truncate\"` with `terminate-on=\"entry-flag\"` — entry-flag termination means the bytes after the chain belong to the fields that follow it, so silently dropping a post-cap entry leaves that entry's bytes where the next field reads them and the decode succeeds with corrupt values (RFC §5.B line 533); use `on-overflow=\"reject\"`, or `terminate-on=\"exhaust-or-depth\"` if the chain owns the rest of the wire","actual":"truncate","fix":{"kind":"replace_with","to":"reject"}}"#,
            ),
            // ── §synth-5-B B3 DMA alignment primitive (SCE Protocol-Synthesis RFC §synth-5-B) ─
            (
                "forge/codec-dma-alignment-unsatisfiable",
                ValidationError::CodecDmaAlignmentUnsatisfiable {
                    codec: "session_envelope".into(),
                    field: "aligned_payload".into(),
                    burst_align: 32,
                    reason: "preceding field 'value' has bit-size 'vle' (variable-length); static padding cannot honor sce:dma-burst-align when any prior field's wire size depends on runtime values (RFC §5.B \"fixed-offset positions only — no VLE-following alignment\")".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:98584368c0e405fa","code":"codec/dma-alignment-unsatisfiable","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.B","message":"codec 'session_envelope': field 'aligned_payload' with sce:dma-burst-align=\"32\" cannot be honored — preceding field 'value' has bit-size 'vle' (variable-length); static padding cannot honor sce:dma-burst-align when any prior field's wire size depends on runtime values (RFC §5.B \"fixed-offset positions only — no VLE-following alignment\")","actual":"aligned_payload"}"#,
            ),
            // ── §synth-5-B peek-byte cross-codec layout ─
            (
                "forge/codec-peek-byte-flag-layout-mismatch",
                ValidationError::CodecPeekByteFlagLayoutMismatch {
                    body_codec: "codec_peek_arm_a".into(),
                    parent_codec: "codec_variant_peek_basic".into(),
                    reason: "parent <sce:peek-byte id=\"peek\"> places flag 'kind' at bit=0 width=2 but arm body 'codec_peek_arm_a' header field 'header' places 'kind' at bit=1 width=2 — fix one side (the peeked byte and the arm body's own first byte are the same wire byte, so the two declarations MUST agree)".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:f143872e85c94270","code":"codec/peek-byte-flag-layout-mismatch","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.B","message":"codec 'codec_peek_arm_a' (arm body): peek-byte flag layout mismatch against parent codec 'codec_variant_peek_basic' — parent <sce:peek-byte id=\"peek\"> places flag 'kind' at bit=0 width=2 but arm body 'codec_peek_arm_a' header field 'header' places 'kind' at bit=1 width=2 — fix one side (the peeked byte and the arm body's own first byte are the same wire byte, so the two declarations MUST agree)","actual":"codec_peek_arm_a"}"#,
            ),
            // ── §synth-5-C link kind (SCE Protocol-Synthesis RFC §synth-5-C) ─
            (
                "forge/link-framer-missing",
                ValidationError::LinkFramerMissing {
                    name: "udp_scout".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:293d71926c63758a","code":"link/framer-missing","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.C","message":"link 'udp_scout': missing required <sce:framer ref=\"...\"/> child — `sce:kind=\"link\"` requires a framer codec reference so RX bytes can be decoded and TX events can be encoded; add a <sce:framer ref=\"<codec_name>\"/> child","actual":"udp_scout"}"#,
            ),
            // ── §synth-5-C link kind negative coverage parse-time pair ─
            (
                "forge/link-link-class-unknown",
                ValidationError::LinkLinkClassUnknown {
                    name: "udp_scout".into(),
                    value: "udpx".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:cb784501438f471c","code":"link/link-class-unknown","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.C","message":"link 'udp_scout': <sce:link-class> body text \"udpx\" is not in the closed enum {`udp`, `tcp`, `serial`, `websocket`, `raw_eth`} per RFC §5.C lines 765-771; replace with one of the listed candidates (OS-specific classes such as `unix_socket` or `qnx_msg` land additively in later phases)","actual":"udpx","fix":{"kind":"replace_one_of","candidates":["udp","tcp","serial","websocket","raw_eth"]}}"#,
            ),
            (
                "forge/link-backpressure-undeclared",
                ValidationError::LinkBackpressureUndeclared {
                    name: "udp_scout".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:d9cf8dea54e29a00","code":"link/backpressure-undeclared","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.C","message":"link 'udp_scout': missing required <sce:backpressure> child — `sce:kind=\"link\"` requires an explicit backpressure policy declaration per RFC §5.C; add a <sce:backpressure>drop|block|signal-event</sce:backpressure> child","actual":"udp_scout"}"#,
            ),
            // ── §synth-5-C OS-axis validate-time diagnostic ────────────
            (
                "forge/link-class-unsupported-on-target",
                ValidationError::LinkClassUnsupportedOnTarget {
                    name: "udp_scout".into(),
                    class: "serial".into(),
                    target_os: "linux".into(),
                    candidates: vec!["bare_metal".into()],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:5b4e6cb8ccacd634","code":"link/class-unsupported-on-target","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.C","message":"link 'udp_scout': link-class `serial` cannot run on target OS `linux` per RFC §5.C lines 765-771; the matrix admits `serial` on [\"bare_metal\"] only — change either the <sce:link-class> body or the deploy.yaml `machines.<id>.platform.os` for the target machine","actual":"linux","fix":{"kind":"replace_one_of","candidates":["bare_metal"]}}"#,
            ),
            // ── §synth-5-C link↔pool cross-resolution diagnostic ───────
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
                r#"{"v":1,"id":"fnv1a:b02945fcd497fac4","code":"link/pool-slot-smaller-than-framer-max","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.C","message":"link 'udp_scout': rx-pool 'rx_pool_sram1' slot-size 64 bytes is smaller than framer 'scout_frame_codec' worst-case encoded size 256 bytes — raise <sce:slot-size> on the bound pool or shrink the codec's worst-case body","actual":"64"}"#,
            ),
            (
                "forge/link-pool-ref-not-declared",
                ValidationError::LinkPoolRefNotDeclared {
                    link_name: "udp_scout".into(),
                    pool_side: "rx",
                    pool_ref: "rx_pool_sram".into(),
                    candidates: vec!["rx_pool_sram1".into(), "tx_pool_sram1".into()],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:297ea44f013980b0","code":"link/pool-ref-not-declared","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.C","message":"link 'udp_scout': <sce:rx-pool ref=\"rx_pool_sram\"/> names no `sce:kind=\"buffer-pool\"` document in this build — declare the pool document and pass it to the build, or correct the ref to one of [\"rx_pool_sram1\", \"tx_pool_sram1\"]","actual":"rx_pool_sram","fix":{"kind":"replace_one_of","candidates":["rx_pool_sram1","tx_pool_sram1"]}}"#,
            ),
            (
                "forge/link-framer-ref-not-declared",
                ValidationError::LinkFramerRefNotDeclared {
                    link_name: "udp_scout".into(),
                    framer_ref: "scout_frame_code".into(),
                    candidates: vec!["scout_frame_codec".into(), "telemetry_codec".into()],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:51559615ee15b545","code":"link/framer-ref-not-declared","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.C","message":"link 'udp_scout': <sce:framer ref=\"scout_frame_code\"/> names no `sce:kind=\"codec\"` document in this build — declare the codec document and pass it to the build, or correct the ref to one of [\"scout_frame_codec\", \"telemetry_codec\"]","actual":"scout_frame_code","fix":{"kind":"replace_one_of","candidates":["scout_frame_codec","telemetry_codec"]}}"#,
            ),
            // ── §synth-5-E buffer-pool placement validate-time diagnostic ─
            (
                "forge/mem-pool-section-conflict",
                ValidationError::BufferPoolSectionConflict {
                    name: "rx_pool_sram1".into(),
                    machine: "mcu_node".into(),
                    section: "sram1".into(),
                    candidates: vec!["dtcm".into(), "sram2".into()],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:12ceca0bfb0b761a","code":"mem/pool-section-conflict","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.E","message":"buffer-pool 'rx_pool_sram1': section `sram1` is not declared in deploy.yaml `machines.mcu_node.memory.sram_regions` — extend the memory map or rename the pool's <sce:section> body to one of [\"dtcm\", \"sram2\"]","actual":"sram1","fix":{"kind":"replace_one_of","candidates":["dtcm","sram2"]}}"#,
            ),
            // ── §synth-5-E buffer-pool size validate-time diagnostic ──
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
                r#"{"v":1,"id":"fnv1a:8eb07e1d20f16ce2","code":"mem/pool-too-large","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.E","message":"buffer-pool 'rx_pool_sram1': storage footprint 131072 bytes (32 × 4096) does not fit in deploy.yaml `machines.mcu_node.memory.sram_regions.sram1` of size 65536 bytes — raise the region size or shrink slot-count/slot-size","actual":"131072"}"#,
            ),
            // ── §synth-5-E linker fragment codegen self-check ──────────
            (
                "forge/mem-inter-pool-padding-not-emitted",
                ValidationError::BufferPoolInterPoolPaddingNotEmitted {
                    name: "rx_pool_sram1".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:f3d92b158c62567b","code":"mem/inter-pool-padding-not-emitted","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.E","message":"buffer-pool 'rx_pool_sram1': linker fragment is missing the inter-pool `. = ALIGN(N);` sentinel — codegen invariant violation per RFC §5.E lines 1059-1064; report at https://github.com/newmassrael/scxml-core-engine/issues"}"#,
            ),
            // ── §synth-5-E pool header ownership pull-through self-check ─
            (
                "forge/pool-sample-typestate-attributes-disabled",
                ValidationError::BufferPoolSampleTypestateAttributesDisabled {
                    name: "rx_pool_sram1".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:43b2c0bb267f73f5","code":"pool/sample-typestate-attributes-disabled","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.E","message":"buffer-pool 'rx_pool_sram1': generated C11 header is missing the `#include <sce/sample.h>` directive — the ownership contract the analyzer and defensive layers rest on will be absent from consumer builds, codegen invariant violation per RFC §5.E lines 1276-1346; report at https://github.com/newmassrael/scxml-core-engine/issues"}"#,
            ),
            // ── §synth-5-E C5 cache-maintenance validation: alignment vs platform.dcache_line_size ─
            (
                "forge/mem-cache-line-alignment",
                ValidationError::BufferPoolCacheLineAlignment {
                    name: "rx_pool_sram1".into(),
                    machine: "mcu_node".into(),
                    pool_alignment: 16,
                    dcache_line_size: 32,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:9871be15958d973d","code":"mem/cache-line-alignment","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.E","message":"buffer-pool 'rx_pool_sram1': alignment 16 is smaller than target platform's `dcache_line_size` 32 on machine 'mcu_node' under `cache-policy: maintain`. Partial-line cache_invalidate_by_addr corrupts adjacent slot data on the start side. Raise <sce:alignment> to at least 32.","actual":"16"}"#,
            ),
            // ── §synth-5-E C5 cache-maintenance validation: dcache_line_size must be a power of two ─
            (
                "forge/mem-dcache-line-size-not-power-of-two",
                ValidationError::DeployDcacheLineSizeNotPowerOfTwo {
                    machine: "mcu_node".into(),
                    dcache_line_size: 48,
                    previous_power: 32,
                    next_power: 64,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:612266b62e600c21","code":"mem/dcache-line-size-not-power-of-two","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.E","message":"machine 'mcu_node': `platform.dcache_line_size` 48 is not a power of two. Cache lines are powers of two on every core SCE targets, and the buffer-pool cache rules divide by this value. Nearest powers of two are 32 and 64.","actual":"48","fix":{"kind":"replace_one_of","candidates":["32","64"]}}"#,
            ),
            // ── §synth-5-E slot-table layout: alignment must be a power of two ─
            (
                "forge/mem-alignment-not-power-of-two",
                ValidationError::BufferPoolAlignmentNotPowerOfTwo {
                    name: "rx_pool_sram1".into(),
                    alignment: 3,
                    previous_power: 2,
                    next_power: 4,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:2e5362567f68b687","code":"mem/alignment-not-power-of-two","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.E","message":"buffer-pool 'rx_pool_sram1': alignment 3 is not a power of two. DMA and cache-line boundaries are powers of two, and both backends lower `<sce:alignment>` to a language alignment specifier that admits nothing else. Nearest powers of two are 2 and 4.","actual":"3","fix":{"kind":"replace_one_of","candidates":["2","4"]}}"#,
            ),
            // ── §synth-5-E slot-table layout: slot-size is the stride, so it must divide by the alignment ─
            (
                "forge/mem-slot-size-not-alignment-multiple",
                ValidationError::BufferPoolSlotSizeNotAlignmentMultiple {
                    name: "rx_pool_sram1".into(),
                    slot_size: 100,
                    alignment: 32,
                    remainder: 4,
                    previous_multiple: 96,
                    next_multiple: 128,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:5770bfd94be1cf4d","code":"mem/slot-size-not-alignment-multiple","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.E","message":"buffer-pool 'rx_pool_sram1': slot-size 100 is not a whole-number multiple of alignment 32 (remainder 4). The slot size is the stride between slots, so only the first slot would start on the declared DMA boundary — and the per-slot cache maintenance would reach into the neighbouring slot's line. Round slot-size to 96 or 128.","actual":"100","fix":{"kind":"replace_one_of","candidates":["96","128"]}}"#,
            ),
            // ── §synth-5-E C5 cache-policy on no-dcache core (Fix::ReplaceOneOf [\"none\"]) ─
            (
                "forge/mem-cache-policy-unsupported-on-no-dcache-core",
                ValidationError::BufferPoolCachePolicyUnsupportedOnNoDcacheCore {
                    name: "rx_pool_sram1".into(),
                    machine: "mcu_node".into(),
                    declared_policy: "maintain".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:3144879193ab848a","code":"mem/cache-policy-unsupported-on-no-dcache-core","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.E","message":"buffer-pool 'rx_pool_sram1': `cache-policy: maintain` declared on machine 'mcu_node' which has `platform.has_dcache: false`. Cache maintenance is meaningless on a core without a data cache. Switch to `cache-policy: none`.","actual":"maintain","fix":{"kind":"replace_one_of","candidates":["none"]}}"#,
            ),
            // ── §synth-5-E C5 author guard: <sce:extern> for cache trio rejected per spec lines 1222-1227 ─
            (
                "forge/pool-cache-maintenance-misplaced",
                ValidationError::PoolCacheMaintenanceMisplaced {
                    attempted_symbol: "sce_dcache_clean_by_addr".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:dd94b2147bfbcdf3","code":"pool/cache-maintenance-misplaced","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.E","message":"<sce:extern name=\"sce_dcache_clean_by_addr\">: cache-maintenance intrinsics are FSM-driven and authored automatically by the buffer-pool kind under `cache-policy: maintain` (RFC §5.E lines 1222-1227). Author <sce:extern> for the cache trio is forbidden — remove the declaration; codegen emits the calls on lifecycle edges.","actual":"sce_dcache_clean_by_addr"}"#,
            ),
            // ── §synth-5-E C5 config-completeness: has_dcache=true requires has_speculative_prefetch ─
            (
                "forge/pool-speculative-prefetch-flag-missing",
                ValidationError::PoolSpeculativePrefetchFlagMissing {
                    machine: "mcu_node".into(),
                    pool_name: "rx_pool_sram1".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:101a5c631d3b5278","code":"pool/speculative-prefetch-flag-missing","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.E","message":"machine 'mcu_node': `platform.has_dcache: true` is set but `platform.has_speculative_prefetch` is not. Buffer-pool 'rx_pool_sram1' uses `cache-policy: maintain` and codegen cannot decide whether to emit the pre-DMA-RX invalidate edge. Declare `has_speculative_prefetch` per the SoC datasheet (M7+/A-class = true, M3/M4 = false)."}"#,
            ),
            // ── §synth-5-E C5 codegen self-check: pre-arm cache-invalidate edge missing on speculative core ─
            (
                "forge/pool-cache-pre-arm-invalidate-missing-on-speculative-core",
                ValidationError::PoolCachePreArmInvalidateMissingOnSpeculativeCore {
                    name: "rx_pool_sram1".into(),
                    backend: "rust".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:e043d4876ee880bd","code":"pool/cache-pre-arm-invalidate-missing-on-speculative-core","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.E","message":"buffer-pool 'rx_pool_sram1': generated source for backend `rust` is missing the `sce_dcache_invalidate_by_addr` call on the `free → dma-armed-rx` edge despite `cache-policy: maintain` + `platform.has_speculative_prefetch: true` — codegen invariant violation per RFC §5.E lines 1186-1198 + 1552; report at https://github.com/newmassrael/scxml-core-engine/issues"}"#,
            ),
            // ── §synth-5-O IR provenance pre-emit guard ──────────────────
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
                r#"{"v":1,"id":"fnv1a:c468c384e2af3fcf","code":"traceability/scxml-line-range-missing","stage":"generate","spec":"SCE Protocol-Synthesis RFC §5.O","message":"<state> 'S0': source_location not populated — sourcemap pre-emit guard (parser site missed)"}"#,
            ),
            // ── §synth-5-O symbol mangling + sourcemap contract goldens ──
            //    Dual-location collision report. `actual` is the
            //    colliding mangled symbol; the `Fix::ReplaceOneOf`
            //    candidate list names the two offending sites so the
            //    consumer / human picks which to rename.
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
                r#"{"v":1,"id":"fnv1a:acb9750dd735a718","code":"traceability/state-id-collision","stage":"generate","spec":"SCE Protocol-Synthesis RFC §5.O","message":"symbol collision: 'motor__armed___state_body' maps to two IR nodes — motor.scxml:10 and imports/armed.scxml:4. Repair: rename one of the colliding ids so the mangled symbols differ","actual":"motor__armed___state_body","fix":{"kind":"replace_one_of","candidates":["motor.scxml:10","imports/armed.scxml:4"]}}"#,
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
                r#"{"v":1,"id":"fnv1a:9af98c119a0b235a","code":"traceability/symbol-name-exceeds-c-identifier-limit","stage":"generate","spec":"SCE Protocol-Synthesis RFC §5.O","message":"mangled symbol 'very_long_machine__nested_state_path___state_body' exceeds C99 external identifier limit by 18 char(s) (got 49, max 31). Repair: shorten one of the contributing names (machine id, state id, or artifact suffix) or enable `platform.strict_c99_identifiers: false` in deploy.yaml to suppress this warning","actual":"very_long_machine__nested_state_path___state_body"}"#,
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
                r#"{"v":1,"id":"fnv1a:d7a5b77c40fb62ef","code":"traceability/sourcemap-source-hash-mismatch","stage":"generate","spec":"SCE Protocol-Synthesis RFC §5.O","message":"sourcemap source_hash drift: sourcemap recorded 'abc123def456' but §6.2.6 header recorded '789aaaabbbb0' on out/rust/sce_sourcemap.json. Repair: regenerate via `sce-codegen generate` to rebuild both sides from the same inputs","actual":"abc123def456"}"#,
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
                r#"{"v":1,"id":"fnv1a:eefcdcd7864bca4d","code":"traceability/sce-map-attribute-stripped","stage":"generate","spec":"SCE Protocol-Synthesis RFC §5.O","message":"SCE-MAP `#[doc]` marker stripped from 'test144::on_entry_s0_0' in sce_rust_tests (release); falling back to `// SCE-MAP:` line comments. Repair: re-emit with the dual-marker form (the default dual-marker form) or upstream the rustdoc fix","actual":"test144::on_entry_s0_0"}"#,
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
                r#"{"v":1,"id":"fnv1a:052d4d7f20b67021","code":"traceability/meta-generated-source-line-marker-missing","stage":"generate","spec":"SCE Protocol-Synthesis RFC §5.O","message":"emitted file 'out/test144/test144_sm.rs' carries a §6.2.6 drift header but no `SCE-MAP:` marker line. Per ARCHITECTURE.md \"Traceability Ownership Boundary\", every SCE-emitted file must carry at least one marker. Repair: a template under `tools/codegen/templates/` is missing its `sce_map_marker` macro call — report upstream","actual":"out/test144/test144_sm.rs"}"#,
            ),
            // ── MCU driver/class boundary (SCE Protocol-Synthesis RFC §5.2) — driver header
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
                r#"{"v":1,"id":"fnv1a:a887dfe8c01e0694","code":"mcu/driver-header-not-found","stage":"validation","spec":"SCE Protocol-Synthesis RFC §5.2","message":"driver header reference 'missing.h' could not be resolved (searched under '/tmp/round_f_alpha'). Repair: correct the `<sce:driver href=\"...\"/>` value, add the missing header, or set `platform.driver_root` in deploy.yaml so the relative path resolves.","actual":"missing.h"}"#,
            ),
            // ── Non-MCU backend refuses `platform.c11_section_attribute`
            //    (SCE Protocol-Synthesis RFC §5.2; mirrors
            //    `codegen/mcu-class-kind-on-non-mcu-language`). `actual` carries the offending
            //    backend; `key_fragments` reuse the same single value
            //    so the wire-id is stable per backend across runs.
            (
                "mcu/section-attribute-on-non-mcu-target",
                crate::forge::error::GenerateError::McuSectionAttributeOnNonMcuTarget {
                    backend: "rust".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:d9ee1c8383fc6b8f","code":"mcu/section-attribute-on-non-mcu-target","stage":"generate","spec":"SCE Protocol-Synthesis RFC §5.2","message":"platform.c11_section_attribute is set in deploy.yaml but the target backend is 'rust', not 'c11'. The section attribute injects `__attribute__((section(...)))` which only the C11 backend emits. Repair: remove the section attribute, switch the backend to 'c11', or split deploy configurations per target.","actual":"rust"}"#,
            ),
            // ── Section name the C11 emitter cannot place into a
            //    string literal. The quote case is the one that
            //    matters: the name reaches a plain C string on
            //    GCC/Clang/Keil and a string-inside-a-string on IAR,
            //    and a `"` closes one of them.
            (
                "mcu/section-attribute-name-invalid",
                crate::forge::error::GenerateError::McuSectionAttributeNameInvalid {
                    name: ".app\"code".into(),
                    reason: "it contains a double quote, which would close the \
                             string literal the name is emitted into"
                        .into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:bf5be93d976d3ac3","code":"mcu/section-attribute-name-invalid","stage":"generate","spec":"SCE Protocol-Synthesis RFC §5.2","message":"platform.c11_section_attribute.class = '.app\"code' is not a usable section name: it contains a double quote, which would close the string literal the name is emitted into. Accepted characters are letters, digits, `.`, `_`, `$` and `-`, and the name must not be empty. Repair: rename the section in deploy.yaml and in the linker script that places it.","actual":".app\"code"}"#,
            ),
            // ── NL→IR Item C1 Path A: Enum kind invariants ──
            //   `id` hashes are placeholders; the goldens test prints
            //   the actual FNV1a on first run, copy them back here.
            (
                "validation/enum-no-variants",
                ValidationError::EnumNoVariants {
                    name: "result".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:88dbabe6c2556d66","code":"validation/enum-no-variants","stage":"validation","message":"enum 'result': declares no <sce:variant> — at least one variant required"}"#,
            ),
            (
                "validation/enum-variant-duplicate-name",
                ValidationError::EnumVariantDuplicateName {
                    enum_name: "result".into(),
                    name: "ok".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:cfcc7b94a1e9a594","code":"validation/enum-variant-duplicate-name","stage":"validation","message":"enum 'result': duplicate variant name 'ok'","actual":"ok"}"#,
            ),
            (
                "validation/enum-variant-duplicate-value",
                ValidationError::EnumVariantDuplicateValue {
                    enum_name: "result".into(),
                    value: 1,
                    first_name: "error".into(),
                    second_name: "timeout".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:ff9a05d509eaf5c8","code":"validation/enum-variant-duplicate-value","stage":"validation","message":"enum 'result': variants 'error' and 'timeout' both have value 1","actual":"1"}"#,
            ),
            (
                "validation/enum-variant-value-overflows-underlying",
                ValidationError::EnumVariantValueOverflowsUnderlying {
                    enum_name: "result".into(),
                    variant_name: "big".into(),
                    value: 256,
                    underlying: "uint8".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:30ab011184dea5cf","code":"validation/enum-variant-value-overflows-underlying","stage":"validation","message":"enum 'result' variant 'big': value 256 overflows underlying type 'uint8'","expected":["uint8"],"actual":"256"}"#,
            ),
            (
                "validation/enum-unsupported-underlying-type",
                ValidationError::EnumUnsupportedUnderlyingType {
                    name: "result".into(),
                    declared: "float32".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:1438d7bd57263c36","code":"validation/enum-unsupported-underlying-type","stage":"validation","message":"enum 'result': sce:underlying-type='float32' is not supported (supported: uint8 | uint16 | uint32 | uint64 | int8 | int16 | int32 | int64)","actual":"float32"}"#,
            ),
            // ── NL→IR Item C1 Path A: EventSchema kind ───
            //   `id` hashes are placeholders; the goldens test prints
            //   the actual FNV1a on first run, copy them back here.
            (
                "validation/event-schema-on-builtin-event",
                ValidationError::EventSchemaOnBuiltinEvent {
                    event_name: "error.execution".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:fdece2ea5d583d9c","code":"validation/event-schema-on-builtin-event","stage":"validation","message":"EventSchema cannot declare a schema for W3C built-in event 'error.execution' (reserved namespace: error., done.invoke., done.state.)","actual":"error.execution"}"#,
            ),
            (
                "validation/event-payload-field-unknown",
                ValidationError::EventPayloadFieldUnknown {
                    importing_kind: ForgeKind::Statechart,
                    importing_name: "demo".into(),
                    event_name: "job.completed".into(),
                    field: "stauts".into(),
                    imported_kind: ForgeKind::EventSchema,
                    imported_name: "job_completed_schema".into(),
                    candidates: vec!["count".into(), "status".into()],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:fa3df31661845cdb","code":"validation/event-payload-field-unknown","stage":"validation","message":"statechart 'demo': <send event=\"job.completed\"> declares <param name=\"stauts\"> not in the EventSchema for 'job.completed' (imported event-schema 'job_completed_schema') (declared fields: count, status)","actual":"stauts","fix":{"kind":"replace_one_of","candidates":["count","status"]}}"#,
            ),
            // ── RFC `rfc-eventschema-bytes-guard.md` §bytesguard-3 B3: ordering
            //   operator on a bytes payload. `id` is a placeholder; the
            //   goldens test prints the actual FNV1a on first run.
            (
                "validation/bytes-comparison-not-equality",
                ValidationError::BytesComparisonNotEquality {
                    importing_kind: ForgeKind::Statechart,
                    importing_name: "demo".into(),
                    field: "raw".into(),
                    op: "<".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:dd6f31b5df3a391c","code":"validation/bytes-comparison-not-equality","stage":"validation","message":"statechart 'demo': operator '<' is not defined on the bytes payload '_event.data.raw' — only equality ('===' / '!==') is supported on bytes","expected":["=== or !=="],"actual":"<"}"#,
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
        vec![(
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
        )]
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
                "mesh/deploy-invalid-dedup-window",
                DeployError::InvalidDedupWindow {
                    machine: "brake".into(),
                    reason: "window_size must be greater than zero — a zero-length window admits every duplicate, which is not a narrow filter but no filter. Omit the `dedup:` section to take the default of 256 entries.".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:e3f14d59a0d51e88","code":"mesh/deploy-invalid-dedup-window","stage":"mesh-deploy","spec":"SCE Mesh §10.5","message":"machine 'brake': invalid `dedup:` section in deploy.yaml — window_size must be greater than zero — a zero-length window admits every duplicate, which is not a narrow filter but no filter. Omit the `dedup:` section to take the default of 256 entries.","actual":"brake"}"#,
            ),
            (
                "mesh/deploy-invalid-custom-tcp-socket",
                DeployError::InvalidCustomTcpSocket {
                    device: "ecu1".into(),
                    reason: "backlog (0) must be at least 1 — omit the field to take the default rather than declaring a value that disables it".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:c3ab6607263bd89f","code":"mesh/deploy-invalid-custom-tcp-socket","stage":"mesh-deploy","spec":"SCE Mesh §16.8.3","message":"device 'ecu1': invalid `transports.custom_tcp:` socket setting — backlog (0) must be at least 1 — omit the field to take the default rather than declaring a value that disables it","actual":"ecu1"}"#,
            ),
            (
                "mesh/deploy-invalid-dds-qos",
                DeployError::InvalidDdsQos {
                    device: "ecu1".into(),
                    reason: "partition must not be empty — the empty string is the default partition, so declaring it changes nothing; omit the field instead".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:920c01994d026981","code":"mesh/deploy-invalid-dds-qos","stage":"mesh-deploy","spec":"SCE Mesh §8.2","message":"device 'ecu1': invalid `transports.dds.qos:` setting — partition must not be empty — the empty string is the default partition, so declaring it changes nothing; omit the field instead","actual":"ecu1"}"#,
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
                "mesh/deploy-invalid-server-response-deadline",
                DeployError::InvalidServerResponseDeadline {
                    machine: "motor".into(),
                    reason: "response_deadline_ms (5) must be >= 10 ms — values below this floor race typical engine macrostep latency and would expire every inbound request before the engine can respond".into(),
                }
                .into(),
                // Hash placeholder — the byte-stability assertion patches
                // it on first run. Shape + message are the contract.
                r#"{"v":1,"id":"fnv1a:57355b72c52cc9ad","code":"mesh/deploy-invalid-server-response-deadline","stage":"mesh-deploy","spec":"SCE Mesh §9.5","message":"machine 'motor': invalid `server.response_deadline_ms` in deploy.yaml — response_deadline_ms (5) must be >= 10 ms — values below this floor race typical engine macrostep latency and would expire every inbound request before the engine can respond. Either fix the value or omit the knob entirely to disable the server deadline.","actual":"motor"}"#,
            ),
            (
                "mesh/deploy-invalid-outbound-buffer",
                DeployError::InvalidOutboundBuffer {
                    machine: "brake".into(),
                    reason: "max_pending_per_target (0) must be >= 1 — a zero-capacity buffer cannot hold any envelope, which is indistinguishable from the pre-§10.10 silent-drop behaviour; omit the section entirely to opt out of buffering instead".into(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:a62483a5bfc65457","code":"mesh/deploy-invalid-outbound-buffer","stage":"mesh-deploy","spec":"SCE Mesh §10.10","message":"machine 'brake': invalid `outbound_buffer:` section in deploy.yaml — max_pending_per_target (0) must be >= 1 — a zero-capacity buffer cannot hold any envelope, which is indistinguishable from the pre-§10.10 silent-drop behaviour; omit the section entirely to opt out of buffering instead. Either correct the section or omit it entirely to opt out of §10.10 buffering.","actual":"brake"}"#,
            ),
            (
                "mesh/deploy-invalid-retry-policy",
                DeployError::InvalidRetryPolicy {
                    machine: "throttle".into(),
                    target: "#motor".into(),
                    reason: "max_retries (0) must be >= 1 — a zero-retry policy is semantically equivalent to omitting the section (the dispatcher would fast-fail every failure and SEND_FAILED would fire per Stage 1/2 behaviour); omit the section entirely to opt out of retries instead".into(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:a595b4da59d04f93","code":"mesh/deploy-invalid-retry-policy","stage":"mesh-deploy","spec":"SCE Mesh §16.7","message":"machine 'throttle', binding '#motor': invalid `retry:` section in deploy.yaml — max_retries (0) must be >= 1 — a zero-retry policy is semantically equivalent to omitting the section (the dispatcher would fast-fail every failure and SEND_FAILED would fire per Stage 1/2 behaviour); omit the section entirely to opt out of retries instead. Either fix the value or omit the section entirely to opt out of §16.7 row 3 retry-layer wrapping.","actual":"throttle"}"#,
            ),
            (
                "mesh/deploy-invalid-auth-policy",
                DeployError::InvalidAuthPolicy {
                    machine: "throttle".into(),
                    target: "#motor".into(),
                    reason: "transport 'custom_tcp' does not support §16.7 row 10 UNAUTHORIZED in this release — only `zenoh` (mTLS cert pinning) and `someip` (SD denial classification) are wired. Either move the binding to a supported transport or set `required: false`".into(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:d3592e1c5629d109","code":"mesh/deploy-invalid-auth-policy","stage":"mesh-deploy","spec":"SCE Mesh §16.7","message":"machine 'throttle', binding '#motor': invalid `auth:` section in deploy.yaml — transport 'custom_tcp' does not support §16.7 row 10 UNAUTHORIZED in this release — only `zenoh` (mTLS cert pinning) and `someip` (SD denial classification) are wired. Either move the binding to a supported transport or set `required: false`. Either fix the configuration or omit the section to opt out of §16.7 row 10 UNAUTHORIZED classification.","actual":"throttle"}"#,
            ),
            (
                "mesh/deploy-discovery-not-supported",
                DeployError::DiscoveryNotSupported {
                    content_kind: "object with keys [mode, resolution]".into(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:b515214da13d33fa","code":"mesh/deploy-discovery-not-supported","stage":"mesh-deploy","spec":"SCE Mesh §3.3","message":"deploy.yaml 'discovery:' top-level block is not supported (object with keys [mode, resolution]). SCE Mesh §3.3 invariant: transport-native routing is the source of truth for peer availability; SCE does not maintain a peer table (the §13 rejected list rejects SCE-maintained peer tables and a `discovery.mode: static | dynamic` deploy switch). For per-binding runtime target selection use value-field placeholders (§14.4). For transport-level peer discovery configure the external OEM config (zenoh.json5 scouting, vsomeip.json service-discovery).","actual":"object with keys [mode, resolution]"}"#,
            ),
            (
                "mesh/deploy-pool-not-supported-by-transport",
                DeployError::PoolNotSupportedByTransport {
                    machine: "brake".into(),
                    binding: "#logger".into(),
                    transport: "shm".into(),
                    // Read from the registry exactly as the raise site
                    // does, so a pool_shape change lands here as a
                    // golden diff rather than as stale repair advice.
                    realised_transports: crate::mesh::transport::pool_alternatives(),
                }
                .into(),
                // Hash placeholder — patched by byte-stability assertion.
                r#"{"v":1,"id":"fnv1a:d6c4a65cf22dfccc","code":"mesh/deploy-pool-not-supported-by-transport","stage":"mesh-deploy","spec":"SCE Mesh §14.4","message":"machine 'brake': binding '#logger' on transport 'shm' carries a '{name}' placeholder, but this transport does not support pool bindings (pool_shape = None). Transports that do: 'someip' (requires instances:), 'zenoh', 'dds' (requires members:). Move the binding to one of those, or drop the placeholder.","actual":"brake"}"#,
            ),
            (
                "mesh/deploy-pool-missing-member-list",
                DeployError::PoolMissingMemberList {
                    machine: "brake".into(),
                    binding: "#player".into(),
                    transport: "dds".into(),
                    // Carrier-derived: the DDS arm demands `members:`,
                    // the SOME/IP arm `instances:`, and the golden pins
                    // that the message quotes whichever the transport
                    // actually reads.
                    expected_field: "members".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:e38c83d89b136e7a","code":"mesh/deploy-pool-missing-member-list","stage":"mesh-deploy","spec":"SCE Mesh §14.4","message":"machine 'brake': binding '#player' on transport 'dds' requests a runtime pool but is missing the required `members:` list. This transport cannot discover members on demand; declare the expected set explicitly.","actual":"brake"}"#,
            ),
            (
                "mesh/deploy-pool-empty-member-list",
                DeployError::PoolEmptyMemberList {
                    machine: "brake".into(),
                    binding: "#player".into(),
                    declared_field: "instances".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:80e7e4b37861e924","code":"mesh/deploy-pool-empty-member-list","stage":"mesh-deploy","spec":"SCE Mesh §14.4","message":"machine 'brake': binding '#player' has an empty `instances: []` list. Declare at least one member or remove the list entirely.","actual":"brake"}"#,
            ),
            (
                "mesh/deploy-pool-binding-field-not-supported",
                DeployError::PoolBindingFieldNotSupported {
                    machine: "brake".into(),
                    binding: "#player".into(),
                    transport: "zenoh".into(),
                    declared_field: "instances".into(),
                    // Zenoh has a pool, so its carrier does describe a
                    // mechanism — the rejection is "other carrier's
                    // syntax", not "no pool at all".
                    expected_mechanism: crate::mesh::transport::lookup("zenoh")
                        .unwrap()
                        .pool_member_carrier
                        .selection_mechanism()
                        .map(str::to_string),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:5506845f9af45993","code":"mesh/deploy-pool-binding-field-not-supported","stage":"mesh-deploy","spec":"SCE Mesh §14.4","message":"machine 'brake': binding '#player' on transport 'zenoh' declares `instances:`, which this transport does not read. Its pool members are string segments of the binding address: embed a `{name}` placeholder naming the selecting <param>, and — on a bounded pool — enumerate the values in `members:`.","actual":"instances","fix":{"kind":"remove_fields","location":"topology.*.machines.brake.bindings.#player","fields":["instances"]}}"#,
            ),
            (
                "mesh/deploy-pool-dispatch-without-member",
                DeployError::PoolDispatchWithoutMember {
                    machine: "brake".into(),
                    binding: "#player".into(),
                    transport: "zenoh".into(),
                    feature: "outbound_buffer".into(),
                    reason: "the buffer holds sends until ONE readiness signal for this \
                             binding fires, and a pool has one address per member"
                        .into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:47d4496873dd770d","code":"mesh/deploy-pool-dispatch-without-member","stage":"mesh-deploy","spec":"SCE Mesh §14.4","message":"machine 'brake': binding '#player' on transport 'zenoh' declares a §14.4 pool, which cannot be combined with `outbound_buffer:` on the same machine — the buffer holds sends until ONE readiness signal for this binding fires, and a pool has one address per member. Remove `outbound_buffer:`, or move the pool binding to a machine without it.","actual":"outbound_buffer","fix":{"kind":"remove_fields","location":"topology.*.machines.brake","fields":["outbound_buffer"]}}"#,
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
                "mesh/deploy-cross-target-reply-not-supported",
                DeployError::CrossTargetReplyNotSupported {
                    machine: "brake".into(),
                    binding: "#alpha".into(),
                    transport: "zenoh".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:42bfa9b16bf8c23d","code":"mesh/deploy-cross-target-reply-not-supported","stage":"mesh-deploy","spec":"SCE Mesh §14.6","message":"machine 'brake': binding '#alpha' on transport 'zenoh' declares a `reply_from:` set wider than its own target, but this transport cannot carry a cross-target reply (supports_cross_target_reply = false). Either drop `reply_from:` to keep the same-target default, or move the binding to a transport that correlates replies through a lookup table (someip, local).","actual":"brake"}"#,
            ),
            (
                "mesh/deploy-invalid-reply-from",
                DeployError::InvalidReplyFrom {
                    machine: "brake".into(),
                    binding: "#alpha".into(),
                    reason: "names machine 'ghost', which the topology does not declare".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:2449562e1103c5ed","code":"mesh/deploy-invalid-reply-from","stage":"mesh-deploy","spec":"SCE Mesh §14.6","message":"machine 'brake': binding '#alpha' has an invalid `reply_from:` list — names machine 'ghost', which the topology does not declare. Omit the field entirely to keep the same-target default.","actual":"brake"}"#,
            ),
            (
                // Parse-time typo guard for the per-binding key surface.
                // FixCarriesCandidates over the transport's legal keys,
                // ordered closest-first so `candidates[0]` is the repair.
                "mesh/deploy-unknown-binding-field",
                DeployError::UnknownBindingField {
                    location: "topology.ecu1.machines.brake.bindings.#motor".into(),
                    transport: "zenoh".into(),
                    field: "orderng".into(),
                    candidates: vec!["ordering".into(), "key".into()],
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:8e0d02a6016f0b67","code":"mesh/deploy-unknown-binding-field","stage":"mesh-deploy","spec":"SCE Mesh §14","message":"deploy.yaml `topology.ecu1.machines.brake.bindings.#motor` (transport: zenoh) declares unknown key `orderng:` — closest legal key: `ordering`. No transport reads it, so it would be dropped between parse and codegen. Legal keys for a 'zenoh' binding: [ordering, key].","actual":"orderng","fix":{"kind":"replace_one_of","candidates":["ordering","key"]}}"#,
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
                r#"{"v":1,"id":"fnv1a:43c3861fc17b431c","code":"mesh/deploy-stage-pool-not-declared","stage":"mesh-deploy","spec":"SCE Protocol-Synthesis RFC §5.E","message":"machine 'mcu_node': binding '#sub' references stage_pool 'rx_pool_sram1' but no `.forge` file in the build declares a pool by that name. Add a forge `<scxml sce:kind=\"buffer-pool\" name=\"rx_pool_sram1\">` document or fix the reference. See SCE Protocol-Synthesis RFC §5.E.","actual":"rx_pool_sram1","fix":{"kind":"replace_one_of","candidates":["scout_rx_pool"]}}"#,
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
                r#"{"v":1,"id":"fnv1a:e05f280783ac494b","code":"mesh/deploy-stage-pool-wrong-kind","stage":"mesh-deploy","spec":"SCE Protocol-Synthesis RFC §5.E","message":"machine 'mcu_node': binding '#sub' references stage_pool 'scout_codec' which resolves to a forge 'codec' kind, not 'buffer-pool'. Only buffer-pool kind documents back the `Sample::take()` slot contract. Repoint the reference at one of the build's buffer-pool kind names. See SCE Protocol-Synthesis RFC §5.E.","actual":"codec","fix":{"kind":"replace_one_of","candidates":["scout_rx_pool"]}}"#,
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
                r#"{"v":1,"id":"fnv1a:5e3fe77a13dd48d3","code":"mesh/deploy-stage-pool-transport-mismatch","stage":"mesh-deploy","spec":"SCE Protocol-Synthesis RFC §5.E","message":"machine 'mcu_node': binding '#sub' declares stage_pool 'scout_rx_pool' on transport 'zenoh', which has no buffer-pool RX staging surface. The `stage_pool` field is meaningful only for transports that bind a forge buffer-pool kind on their RX path. Drop the field or change the transport. See SCE Protocol-Synthesis RFC §5.E.","actual":"zenoh","fix":{"kind":"remove_fields","location":"topology.*.machines.mcu_node.bindings.#sub","fields":["stage_pool"]}}"#,
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
                DeployError::ScxmlInvokeCrossDeviceTransport(Box::new(
                    crate::mesh::error::ScxmlInvokeCrossDeviceTransportPayload {
                        parent: "parent_x".into(),
                        peer: "worker_y".into(),
                        parent_device: "ecu_a".into(),
                        peer_device: "ecu_b".into(),
                        failure: crate::mesh::error::ScxmlInvokeCrossDeviceFailure::MissingBinding,
                    },
                ))
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
                r#"{"v":1,"id":"fnv1a:936f4785a68e36b5","code":"deploy/worker-stack-budget-missing","stage":"mesh-deploy","spec":"SCE Protocol-Synthesis RFC §5.K","message":"machine 'mcu_node': scheduler.kind 'cooperative' requires scheduler.worker_stack_budget (bytes). SCE Protocol-Synthesis RFC §5.K line 2426 (`deploy/worker-stack-budget-missing`) — cooperative drives the `<send>` queue inside a fixed stack frame; a missing budget would let TLV-decode recursion silently overflow. Repair: add `worker_stack_budget: <bytes>` under `scheduler:` (e.g. 4096), or change `kind:` to `tokio` / `rt` to inherit the host runtime's stack defaults.","actual":"mcu_node"}"#,
            ),
            (
                "deploy/worker-slot-budget-missing",
                DeployError::SchedulerCooperativeMissingSlotBudget {
                    machine: "mcu_node".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:c7e272ab44356ef1","code":"deploy/worker-slot-budget-missing","stage":"mesh-deploy","spec":"SCE Protocol-Synthesis RFC §5.K","message":"machine 'mcu_node': scheduler.kind 'cooperative' requires scheduler.worker_slot_budget_us (microseconds). SCE Protocol-Synthesis RFC §5.K line 2428-2429 (`deploy/worker-slot-budget-missing`) — per-slot WCET ceiling drives the §5.B aggregate WCET check and the cooperative slot-count derivation. Repair: add `worker_slot_budget_us: <us>` under `scheduler:` (e.g. 200), or change `kind:` to `tokio` / `rt` to skip the WCET check.","actual":"mcu_node"}"#,
            ),
            (
                "deploy/keepalive-jitter-budget-missing",
                DeployError::SchedulerCooperativeMissingKeepaliveJitterBudget {
                    machine: "mcu_node".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:79db8ea5ee156731","code":"deploy/keepalive-jitter-budget-missing","stage":"mesh-deploy","spec":"SCE Protocol-Synthesis RFC §5.K","message":"machine 'mcu_node': scheduler.kind 'cooperative' requires scheduler.keepalive_jitter_budget_us (microseconds). SCE Protocol-Synthesis RFC §5.K line 2430-2431 (`deploy/keepalive-jitter-budget-missing`) — sum of worst-case slot budgets in one tick window must fit inside this bound. Repair: add `keepalive_jitter_budget_us: <us>` under `scheduler:` (recommended default: 0.5 × min lease), or change `kind:` to `tokio` / `rt` to inherit host runtime jitter.","actual":"mcu_node"}"#,
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
                r#"{"v":1,"id":"fnv1a:f26a225afadc7a9a","code":"deploy/scheduler-incompatible-with-worker-count","stage":"mesh-deploy","spec":"SCE Protocol-Synthesis RFC §5.K","message":"machine 'mcu_node': declared 5 workers under machines.mcu_node.workers, but cooperative scheduler can host only 3 per tick window (derived from tick_period_us 1000 / worker_slot_budget_us 300). SCE Protocol-Synthesis RFC §5.K line 2423 (`deploy/scheduler-incompatible-with-worker-count`). Repair: raise `tick_period_us`, lower `worker_slot_budget_us`, remove excess workers, or switch `scheduler.kind:` to a preemptive host (`tokio` / `rt`).","expected":["3"],"actual":"5"}"#,
            ),
            // ── §synth-5-K `links:` block (RFC §synth-5-K lines 2232-2540) ──
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
                r#"{"v":1,"id":"fnv1a:29690d741179d6f1","code":"deploy/link-driver-unknown","stage":"mesh-deploy","spec":"SCE Protocol-Synthesis RFC §5.K","message":"machine 'mcu_node': link 'udp_data' declares driver 'foo_udp' which is unknown. SCE Protocol-Synthesis RFC §5.K line 2421 (`deploy/link-driver-unknown`) — the build's closed-allowlist + forge `<sce:link>` cross-doc registry union does not contain this driver. Repair: pick one of [lwip_tcp, lwip_udp].","actual":"foo_udp","fix":{"kind":"replace_one_of","candidates":["lwip_tcp","lwip_udp"]}}"#,
            ),
            (
                "deploy/link-mtu-missing-on-fragmenting-link",
                DeployError::LinkMtuMissingOnFragmentingLink {
                    machine: "mcu_node".into(),
                    link_name: "udp_data".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:863c8c160c2fe018","code":"deploy/link-mtu-missing-on-fragmenting-link","stage":"mesh-deploy","spec":"SCE Protocol-Synthesis RFC §5.K","message":"machine 'mcu_node': link 'udp_data' declares `domain_attrs.trust_class: established_session` but `mtu_bytes:` is absent. SCE Protocol-Synthesis RFC §5.K line 2440-2442 (`deploy/link-mtu-missing-on-fragmenting-link`) — only `established_session` trust class carries Fragment traffic (RFC §5.M line 2731) and the build cannot size reassembly pool slots without the link-layer MTU. Repair: add `mtu_bytes: <bytes>` under this link entry (e.g. 1472 for UDP/IPv4 over Ethernet)."}"#,
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
                r#"{"v":1,"id":"fnv1a:b923c818f30343df","code":"deploy/link-mtu-below-driver-floor","stage":"mesh-deploy","spec":"SCE Protocol-Synthesis RFC §5.K","message":"machine 'mcu_node': link 'udp_data' declares `mtu_bytes: 20` which is below driver 'lwip_udp's minimum payload floor (28). SCE Protocol-Synthesis RFC §5.K line 2443-2445 (`deploy/link-mtu-below-driver-floor`) — the driver's default minimum would override the declared value silently. Repair: raise `mtu_bytes` to >= 28, or change the driver to one with a smaller header floor.","expected":["28"],"actual":"20"}"#,
            ),
            (
                // C11-WebSocket follow-up sibling — driver↔class
                // cross-validator. Forge declares `websocket` but
                // deploy binds `lwip_tcp` whose class is `tcp`;
                // the validator surfaces the mismatch with the
                // candidate driver list (single-element: the
                // driver implementing `websocket` = `websocket_tcp`).
                "deploy/link-driver-class-mismatch",
                DeployError::LinkDriverClassMismatch(Box::new(
                    crate::mesh::error::LinkDriverClassMismatchPayload {
                        machine: "mcu_node".into(),
                        link_name: "ws_control".into(),
                        driver: "lwip_tcp".into(),
                        declared_class: "websocket".into(),
                        expected_class: "tcp".into(),
                        driver_candidates: vec!["websocket_tcp".into()],
                        driver_candidates_list: "websocket_tcp".into(),
                    },
                ))
                .into(),
                r#"{"v":1,"id":"fnv1a:4ac5fbe980bb4938","code":"deploy/link-driver-class-mismatch","stage":"mesh-deploy","spec":"SCE Protocol-Synthesis RFC §5.K","message":"machine 'mcu_node': link 'ws_control' declares forge `<sce:link-class>websocket</sce:link-class>` but deploy.yaml binds `driver: lwip_tcp` which implements class 'tcp'. SCE Protocol-Synthesis RFC §5.C lines 765-771 + §8 Q8 line 3747 (`deploy/link-driver-class-mismatch`) — each core driver implements exactly one protocol class. Repair: change `driver:` to the entry matching the declared class, or change `<sce:link-class>` to match the bound driver.","expected":["tcp"],"actual":"websocket","fix":{"kind":"replace_one_of","candidates":["websocket_tcp"]}}"#,
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
                r#"{"v":1,"id":"fnv1a:04a20137542a8e63","code":"deploy/link-expected-p99-exceeds-mtu","stage":"mesh-deploy","spec":"SCE Protocol-Synthesis RFC §5.K","message":"machine 'mcu_node': link 'udp_data' declares `expected_p99_bytes: 2048` which exceeds `mtu_bytes: 1472`. SCE Protocol-Synthesis RFC §5.K line 2446-2448 (`deploy/link-expected-p99-exceeds-mtu`) — the p99 message would always fragment. Repair: lower `expected_p99_bytes` to <= `mtu_bytes`, or raise `mtu_bytes` (driver permitting), or bind a reassembly pool to this link via a forge `<sce:link>` declaration.","expected":["1472"],"actual":"2048"}"#,
            ),
            (
                "deploy/link-burst-pps-missing-on-isr-dispatch",
                DeployError::LinkBurstPpsMissingOnIsrDispatch {
                    machine: "mcu_node".into(),
                    link_name: "udp_data".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:ae5e6142a2e8c826","code":"deploy/link-burst-pps-missing-on-isr-dispatch","stage":"mesh-deploy","spec":"SCE Protocol-Synthesis RFC §5.K","message":"machine 'mcu_node': link 'udp_data' resolves to `rx_dispatch: isr_to_pool` but `burst_pps` is not declared. SCE Protocol-Synthesis RFC §5.K line 2501-2503 (`deploy/link-burst-pps-missing-on-isr-dispatch`) — ISR fast-path requires `burst_pps` to size the descriptor ring and validate the stack budget. Repair: declare `burst_pps: <pps>`, or explicitly set `rx_dispatch: worker_tick` to opt into the slower cooperative-tick path."}"#,
            ),
            (
                // Cross-doc: declared burst_pps overruns RX
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
                r#"{"v":1,"id":"fnv1a:149ee464a43d20f6","code":"deploy/link-burst-absorption-insufficient","stage":"mesh-deploy","spec":"SCE Protocol-Synthesis RFC §5.K","message":"machine 'mcu_node': link 'udp_data' declares `burst_pps: 50000` against RX pool 'rx_data_pool' with `<sce:slot-count>16</sce:slot-count>` and scheduler `tick_period_us: 1000`. Effective drain capacity is 16000 pps (with the 2.0 safety factor required by SCE Protocol-Synthesis RFC §5.K line 2489-2495), insufficient for the declared burst. Repair: raise `<sce:slot-count>` on pool 'rx_data_pool', lower `scheduler.tick_period_us`, or switch `rx_dispatch: isr_to_pool` when currently `worker_tick`.","expected":["16000"],"actual":"50000"}"#,
            ),
            (
                // Cross-doc: rx_dispatch: worker_tick overruns
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
                r#"{"v":1,"id":"fnv1a:f396cb2184d51ea2","code":"deploy/link-rx-dispatch-worker-tick-on-high-burst","stage":"mesh-deploy","spec":"SCE Protocol-Synthesis RFC §5.K","message":"machine 'mcu_node': link 'udp_data' resolves to `rx_dispatch: worker_tick` but one tick window of arrivals overruns RX pool 'rx_data_pool'. `burst_pps × tick_period_us / 1_000_000 = 100` exceeds `<sce:slot-count>16</sce:slot-count>`. SCE Protocol-Synthesis RFC §5.K line 2496-2500 (`deploy/link-rx-dispatch-worker-tick-on-high-burst`). Repair: switch `rx_dispatch: isr_to_pool` (descriptor-ring re-arm absorbs the burst), raise `<sce:slot-count>` on pool 'rx_data_pool' to admit the per-tick arrivals, or lower `scheduler.tick_period_us` so each window admits fewer arrivals.","expected":["16"],"actual":"100"}"#,
            ),
            (
                "deploy/link-not-declared-in-deploy",
                DeployError::LinkNotDeclaredInDeploy {
                    link_name: "udp_data".into(),
                    candidates: vec!["udp_scout".into()],
                    candidates_list: "udp_scout".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:7769451379390caa","code":"deploy/link-not-declared-in-deploy","stage":"mesh-deploy","spec":"SCE Protocol-Synthesis RFC §5.K","message":"forge `<sce:link name=\"udp_data\">` declared but no `deploy.yaml::machines.<n>.links.udp_data` entry exists. Cross-doc validator (`deploy/link-not-declared-in-deploy`). Repair: add the deploy entry under one of [udp_scout] or another machine, or remove the forge link doc.","actual":"udp_data","fix":{"kind":"replace_one_of","candidates":["udp_scout"]}}"#,
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
                r#"{"v":1,"id":"fnv1a:68d69cab5e950a11","code":"deploy/link-not-declared-in-forge","stage":"mesh-deploy","spec":"SCE Protocol-Synthesis RFC §5.K","message":"machine 'mcu_node': link 'udp_data' declared in deploy.yaml but no forge `<scxml sce:kind=\"link\" name=\"udp_data\">` document was declared/imported. Cross-doc validator (`deploy/link-not-declared-in-forge`). Repair: declare the forge link doc and import it from a statechart/worker on this machine, or pick one of [udp_scout] (forge link doc names known to this build), or remove the orphan deploy entry.","actual":"udp_data","fix":{"kind":"replace_one_of","candidates":["udp_scout"]}}"#,
            ),
            (
                // Parse-time typo guard for the policy enum.
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
                r#"{"v":1,"id":"fnv1a:5ca8faec9e11c481","code":"deploy/stage-copy-policy-unknown","stage":"mesh-deploy","spec":"SCE Protocol-Synthesis RFC §5.K","message":"machine 'mcu_node': `pool_defaults.stage_copy_policy: errr` is not a known policy. SCE Protocol-Synthesis RFC §5.K line 2517-2519 (`deploy/stage-copy-policy-unknown`) — closed-set typo guard. Repair: pick one of [warn, error, forbid].","actual":"errr","fix":{"kind":"replace_one_of","candidates":["warn","error","forbid"]}}"#,
            ),
            // ── Anti-flood + stateless_accept (RFC §synth-5-K lines 2272-2349 + 2449-2473) ──
            (
                "deploy/session-arming-quota-missing",
                DeployError::SessionArmingQuotaMissing {
                    machine: "mcu_node".into(),
                    link_name: "udp_listener".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:f5f597205aa5b0e5","code":"deploy/session-arming-quota-missing","stage":"mesh-deploy","spec":"SCE Protocol-Synthesis RFC §5.K","message":"machine 'mcu_node': link 'udp_listener' declares `trust_class: session_arming` but no `session_arming_quota`. SCE Protocol-Synthesis RFC §5.K line 2449-2451 — without a cap an attacker can fill every `Accepting.*` slot. Repair: declare `session_arming_quota: <count>` (MCU recommended 8, AP recommended 32 per spec line 2282)."}"#,
            ),
            (
                "deploy/accept-rate-config-missing",
                DeployError::AcceptRateConfigMissing {
                    machine: "mcu_node".into(),
                    link_name: "udp_listener".into(),
                    missing_fields: "accept_rate_per_sec, accept_rate_burst".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:6fbd647f1bd1aeb1","code":"deploy/accept-rate-config-missing","stage":"mesh-deploy","spec":"SCE Protocol-Synthesis RFC §5.K","message":"machine 'mcu_node': link 'udp_listener' declares `trust_class: session_arming` but missing accept-rate config: accept_rate_per_sec, accept_rate_burst. SCE Protocol-Synthesis RFC §5.K line 2452-2453 — token-bucket rate-limit is required to prevent half-open quota saturation. Repair: declare both `accept_rate_per_sec` and `accept_rate_burst` (spec line 2290-2302 recommends defaults `accept_rate_per_sec: 4` MCU / `16` AP and `accept_rate_burst: 2 × accept_rate_per_sec`).","actual":"accept_rate_per_sec, accept_rate_burst"}"#,
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
                r#"{"v":1,"id":"fnv1a:a981ec0d75389842","code":"deploy/session-arming-fields-on-non-arming-link","stage":"mesh-deploy","spec":"SCE Protocol-Synthesis RFC §5.K","message":"machine 'mcu_node': link 'udp_data' declares `trust_class: established_session` but anti-flood / stateless_accept fields are present (session_arming_quota). SCE Protocol-Synthesis RFC §5.K line 2454-2459 — `Accepting.*` is never instantiated on this trust class so the fields are dead config (suggests author confusion about which link is the listener). Repair: change `trust_class` to `session_arming` on link 'udp_data' if it is in fact the listener, or remove the dead fields.","actual":"established_session"}"#,
            ),
            (
                "deploy/stateless-accept-required-on-untrusted-source",
                DeployError::StatelessAcceptRequiredOnUntrustedSource {
                    machine: "mcu_node".into(),
                    link_name: "udp_listener".into(),
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:06e17061b74e6486","code":"deploy/stateless-accept-required-on-untrusted-source","stage":"mesh-deploy","spec":"SCE Protocol-Synthesis RFC §5.K","message":"machine 'mcu_node': link 'udp_listener' declares `domain_attrs.untrusted_source: true` but no `stateless_accept` block. SCE Protocol-Synthesis RFC §5.K line 2463-2465 — links exposed to networks the deployment does not control must use HMAC cookies to prevent stateful spoofing. Repair: add a `stateless_accept:` block with `mode`, `cookie_lifetime_ms`, `key_rotation_s`, `hmac_extern`, `rng_extern` per spec line 2320-2349, or set `untrusted_source: false` if the link is on a controlled network."}"#,
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
                r#"{"v":1,"id":"fnv1a:16e83b54ae5813fa","code":"deploy/stateless-accept-key-rotation-shorter-than-lifetime","stage":"mesh-deploy","spec":"SCE Protocol-Synthesis RFC §5.K","message":"machine 'mcu_node': link 'udp_listener' `stateless_accept.key_rotation_s: 30` × 1000 ≤ 2 × `cookie_lifetime_ms: 30000` (30000 ≤ 60000). SCE Protocol-Synthesis RFC §5.K line 2470-2473 — the previous-key honor window cannot bridge a rotation, so handshakes near rotation boundaries get spurious cookie rejection. Repair: raise `key_rotation_s` to > `2 × cookie_lifetime_ms / 1000`, or lower `cookie_lifetime_ms` to < `key_rotation_s × 500`.","expected":["60000"],"actual":"30000"}"#,
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
                r#"{"v":1,"id":"fnv1a:4fea7d1c968b2879","code":"deploy/session-arming-quota-vs-peer-table-invariant-violated","stage":"mesh-deploy","spec":"SCE Protocol-Synthesis RFC §5.K","message":"machine 'mcu_node': link 'udp_listener' `session_arming_quota: 8` × `stateless_accept.max_handshake_time_s: 2` > `stateless_accept.peer_table.capacity: 8` (16 > 8). SCE Protocol-Synthesis RFC §5.K line 2460-2462 — a slow legitimate handshake can be evicted under attack when the attacker churns the quota faster than the per-peer table can absorb. Repair: raise `peer_table.capacity` to ≥ 16, or lower `session_arming_quota` or `max_handshake_time_s` so the product fits the table.","expected":["8"],"actual":"16"}"#,
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
                r#"{"v":1,"id":"fnv1a:023288f561f5a592","code":"deploy/stateless-accept-extern-not-whitelisted","stage":"mesh-deploy","spec":"SCE Protocol-Synthesis RFC §5.K","message":"machine 'mcu_node': link 'udp_listener' `stateless_accept.hmac_extern: my_custom_hmac` not present in the §5.I baseline intrinsics whitelist AND not declared in any loaded `target_plugin`. SCE Protocol-Synthesis RFC §5.K line 2466-2469 — `hmac_extern` and `rng_extern` symbols must come from the baseline registry or a target-plugin entry. Repair: spell the symbol exactly as it appears in the baseline registry or add the symbol to a loaded target-plugin file.","actual":"my_custom_hmac","fix":{"kind":"replace_one_of","candidates":["__sce_intrinsic_cookie_hmac_sha256","__sce_intrinsic_csprng"]}}"#,
            ),
            (
                "timer/slot-overflow",
                DeployError::TimerSlotOverflow {
                    machine: "mcu_node".into(),
                    timer_count: 5,
                    wheel_depth: 4,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:f5757abc38261115","code":"timer/slot-overflow","stage":"mesh-deploy","spec":"SCE Protocol-Synthesis RFC §5.D","message":"machine 'mcu_node': declared 5 timers under machines.mcu_node.timers, but scheduler.timer_wheel_depth = 4 slots cannot accommodate them. SCE Protocol-Synthesis RFC §5.D line 910 (`timer/slot-overflow`) — the static timer wheel is sized at compile time. Repair: raise `scheduler.timer_wheel_depth`, remove excess timers, or switch to `scheduler.kind: tokio` / `rt` to inherit host runtime timer scheduling.","expected":["4"],"actual":"5"}"#,
            ),
            (
                // RFC §synth-5-N line 3060: MCU cooperative scheduler slot
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
                r#"{"v":1,"id":"fnv1a:e23d9c6761cfacc7","code":"link/concurrent-count-exceeds-scheduler-slots","stage":"mesh-deploy","spec":"SCE Protocol-Synthesis RFC §5.N","message":"machine 'mcu_node' (MCU): 4 links declared but the cooperative scheduler accommodates only 2 per-tick slots (`floor(tick_period_us 1000 / per_link_budget_us 500) = 2`). SCE Protocol-Synthesis RFC §5.N line 3060 — more links than the cooperative scheduler can accommodate. Repair: raise `per_link_budget_us`, lower `tick_period_us`, or remove a link declaration from `machines.<m>.links`.","expected":["2"],"actual":"4"}"#,
            ),
            (
                // RFC §synth-5-N line 3061: per-link budget can't fit one
                // tick. NeutralOrDeterministic — two-axis repair.
                "link/per-link-budget-exceeds-tick-period",
                DeployError::LinkPerLinkBudgetExceedsTickPeriod {
                    machine: "mcu_node".into(),
                    per_link_budget_us: 2000,
                    tick_period_us: 1000,
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:b3df0736b5b45af5","code":"link/per-link-budget-exceeds-tick-period","stage":"mesh-deploy","spec":"SCE Protocol-Synthesis RFC §5.N","message":"machine 'mcu_node': `scheduler.per_link_budget_us: 2000` exceeds `scheduler.tick_period_us: 1000`. SCE Protocol-Synthesis RFC §5.N line 3061 — a single link's budget cannot exceed the entire cooperative tick. Repair: lower `per_link_budget_us` to ≤ `tick_period_us`, or raise `tick_period_us`.","expected":["1000"],"actual":"2000"}"#,
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
                    // Read from the registry exactly as the raise site
                    // does, so a flag flip lands here as a golden diff
                    // rather than as a message nobody re-read.
                    realised_transports:
                        crate::mesh::transport::machine_lifetime_subscribe_alternatives(),
                }
                .into(),
                r##"{"v":1,"id":"fnv1a:a66a86a130ed11be","code":"mesh/topology-machine-lifetime-subscription-unsupported","stage":"mesh-topology","spec":"SCE Mesh §13","message":"machine 'brake': subscription on source '#motor' for event 'event.notification.status' uses transport 'someip', which does not support the machine-lifetime subscription path in this build. Transports that do realise it: 'someip', 'zenoh', 'custom_tcp', 'dds'. Move the binding to one of those, or drop the subscription from machines.brake.subscriptions:.","actual":"someip"}"##,
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
                r#"{"v":1,"id":"fnv1a:aa145685cde035e6","code":"mesh/codegen-unsupported-transport","stage":"mesh-codegen","spec":"SCE Mesh §8","message":"transport 'carrier_pigeon' not yet supported (target '#motor')","actual":"carrier_pigeon","fix":{"kind":"replace_one_of","candidates":["local","shm","someip","zenoh","custom_tcp","dds"]}}"#,
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
            // ── NL→IR Item C1 Path A: EventSchema mesh DL-7' ──
            (
                "mesh/event-schema-mismatch",
                DeployError::EventSchemaMismatch {
                    event_name: "job.completed".into(),
                    sender_machine: "alpha".into(),
                    receiver_machine: "beta".into(),
                    reason: crate::mesh::error::EventSchemaMismatchReason::StructuralHashMismatch {
                        sender_hash: "a1b2".into(),
                        receiver_hash: "c3d4".into(),
                    },
                }
                .into(),
                r#"{"v":1,"id":"fnv1a:7cc311d0bc85d710","code":"mesh/event-schema-mismatch","stage":"mesh-deploy","message":"cross-machine schema mismatch on event 'job.completed' between sender machine 'alpha' and receiver machine 'beta': structural-hash mismatch (sender=a1b2, receiver=c3d4)","expected":["c3d4"],"actual":"a1b2"}"#,
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
                CliError::UnknownLanguage {
                    lang: "ruby".into(),
                    // A route with a restricted menu, so the golden pins
                    // that the candidates are the *route's* set rather
                    // than every backend the enum has: offering `c11`
                    // here would be a repair that fails again.
                    route: crate::cli_language::LanguageRoute::GenerateW3c,
                },
                r#"{"v":1,"id":"fnv1a:9ddf2f566ce27ec2","code":"cli/unknown-language","stage":"cli","message":"Unknown language: ruby. `generate-w3c` takes rust, cpp, kotlin, go, python.","actual":"ruby","fix":{"kind":"replace_one_of","candidates":["rust","cpp","kotlin","go","python"]}}"#,
            ),
            (
                "cli/unsupported-language",
                CliError::UnsupportedLanguage {
                    lang: "c11".into(),
                    route: crate::cli_language::LanguageRoute::GenerateW3c,
                },
                r#"{"v":1,"id":"fnv1a:d4ef6ce0e63e47bc","code":"cli/unsupported-language","stage":"cli","message":"`generate-w3c` does not target c11. It takes rust, cpp, kotlin, go, python — C11 is absent because no C11 W3C statechart emitter exists yet (RFC §5.J.1); single-document `generate -l c11` uses a different emitter and does work","actual":"c11","fix":{"kind":"replace_one_of","candidates":["rust","cpp","kotlin","go","python"]}}"#,
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
            (
                "cli/invalid-suite-package",
                CliError::InvalidSuitePackage {
                    detail: "'crate' is not a usable rust conformance suite name: its module \
                             path 'crate' is a Rust keyword, so generated tests could not name it"
                        .into(),
                },
                r#"{"v":1,"id":"fnv1a:c241dc46b96f9f91","code":"cli/invalid-suite-package","stage":"cli","message":"--suite-package: 'crate' is not a usable rust conformance suite name: its module path 'crate' is a Rust keyword, so generated tests could not name it"}"#,
            ),
            // ── Generator content witness ──
            //    Two codes rather than one because "the binary disagrees
            //    with this tree" and "the comparison could not be made"
            //    send the reader to different places, and a consumer
            //    branching on the code must be able to tell them apart
            //    without reading prose.
            (
                "cli/generator-source-drift",
                CliError::GeneratorSourceDrift {
                    root: "/home/dev/scxml-core-engine".into(),
                    embedded_hex:
                        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
                    recomputed_hex:
                        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
                },
                r#"{"v":1,"id":"fnv1a:6d7738e11a3600c2","code":"cli/generator-source-drift","stage":"cli","message":"this sce-codegen was built from different sources than /home/dev/scxml-core-engine holds (binary=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, tree=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb) — rebuild it with: cargo build --bin sce-codegen --features cli -p sce-build","actual":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}"#,
            ),
            (
                "cli/generator-source-unverifiable",
                CliError::GeneratorSourceUnverifiable {
                    root: "/home/dev/scxml-core-engine".into(),
                    reason: "this binary carries no source witness".into(),
                },
                r#"{"v":1,"id":"fnv1a:3e7c8b218f240010","code":"cli/generator-source-unverifiable","stage":"cli","message":"cannot establish whether this sce-codegen matches /home/dev/scxml-core-engine: this binary carries no source witness. Both halves need a full checkout — rebuild the binary with `cargo build --bin sce-codegen --features cli -p sce-build`, or point --root at the workspace it was built from"}"#,
            ),
            // ── §synth-6.2.6 generated-source drift (B9, 2026-05-14) ──
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
                        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
                    actual_hex: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                        .into(),
                },
                r#"{"v":1,"id":"fnv1a:6bbb966dd3008e84","code":"forge/source-hash-mismatch","stage":"cli","spec":"SCE Protocol-Synthesis RFC §6.2.6","message":"out/foo_sm.rs: §6.2.6 source-hash mismatch (embedded=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb, recomputed=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa) — regenerate via sce-codegen","actual":"source-hash=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}"#,
            ),
            // ── §synth-6.2.6 source-set coverage guard ──
            //    Emit-time counterpart to the mismatch code above: that one
            //    fires at `verify` when an embedded hash has gone stale,
            //    this one refuses to embed a hash that never described the
            //    input in the first place. `actual` carries the root and
            //    the collected count so a consumer can distinguish "root
            //    resolved to nothing" (`hashed=0`) from "input lives
            //    outside the root" without walking the tree itself.
            (
                "forge/source-hash-input-uncovered",
                CliError::SourceHashInputUncovered {
                    input: "src/scxml/pdc_reset.scxml".into(),
                    root: "src/scxml".into(),
                    hashed: 0,
                },
                r#"{"v":1,"id":"fnv1a:af571002d7ff22bd","code":"forge/source-hash-input-uncovered","stage":"cli","spec":"SCE Protocol-Synthesis RFC §6.2.6","message":"src/scxml/pdc_reset.scxml: §6.2.6 source-hash would not describe it — 0 file(s) collected from src/scxml; pass --input-root <DIR> containing the input","actual":"root=src/scxml hashed=0"}"#,
            ),
            // ── §synth-6.2.6 source-set enumeration ceiling ──
            //    The coverage guard above answers "does the set describe
            //    the input"; this one answers "can the set be enumerated
            //    at all". `actual` carries the ceiling rather than a
            //    traversal count, so one tree produces one record on every
            //    machine — a count would move with readdir ordering.
            (
                "forge/source-hash-walk-unbounded",
                CliError::SourceHashWalkUnbounded {
                    root: "src/scxml".into(),
                    limit: 1_000_000,
                },
                r#"{"v":1,"id":"fnv1a:a692356602400fc9","code":"forge/source-hash-walk-unbounded","stage":"cli","spec":"SCE Protocol-Synthesis RFC §6.2.6","message":"src/scxml: §6.2.6 source set exceeds 1000000 directories — a directory symlink reaching a sibling multiplies the paths under it; re-point --input-root at a tree without the aliasing, or remove it","actual":"root=src/scxml descent-limit=1000000"}"#,
            ),
            // ── The argument parser's own failure ──
            //    Pinned as a golden because the message is not SCE's
            //    prose: it is clap's rendering, carried verbatim. If a
            //    future refactor starts rewriting it, this entry moves
            //    and the change has to be deliberate.
            (
                "cli/usage",
                CliError::Usage {
                    detail: "unexpected argument '--frobnicate' found".into(),
                },
                r#"{"v":1,"id":"fnv1a:8428d1db7db324b9","code":"cli/usage","stage":"cli","message":"unexpected argument '--frobnicate' found"}"#,
            ),
            // ── A query that ran and found nothing ──
            //    The one CLI code whose exit status is not 20. `searched`
            //    rides the message but not the id, so the same miss in a
            //    tree checked out at another path is the same diagnostic.
            (
                "cli/query-no-match",
                CliError::QueryNoMatch {
                    tool: "addr2sce",
                    query: "symbol 'probe__s0__on_entry'".into(),
                    searched: "out/sce_sourcemap.json".into(),
                },
                r#"{"v":1,"id":"fnv1a:6b1e65365d0453bb","code":"cli/query-no-match","stage":"cli","message":"addr2sce: symbol 'probe__s0__on_entry' matched nothing in out/sce_sourcemap.json","actual":"symbol 'probe__s0__on_entry'"}"#,
            ),
        ]
    }

    /// Assert the generator stamp is present and correct, and return the
    /// line without it.
    ///
    /// The goldens pin the wire *shape*, which is a property of the
    /// source tree. The generator stamp is a property of the *build*:
    /// its value changes on every commit, so a golden that spelled it
    /// would have to be rewritten by every commit — and a golden table
    /// rewritten that often stops being read.
    ///
    /// Removing it here rather than dropping the field from golden
    /// coverage keeps the assertion strictly stronger than a byte match
    /// would have been: every entry in the table is checked for the
    /// stamp *and* for its exact value, against a value no golden could
    /// have written down.
    fn without_generator_stamp(line: &str) -> String {
        let stamp = format!(",\"generator\":\"{}\"", crate::GENERATOR_COMMIT);
        assert!(
            line.contains(&stamp),
            "every diagnostic record must carry {stamp}: {line}"
        );
        line.replacen(&stamp, "", 1)
    }

    /// Byte-stable goldens: each error variant in
    /// [`forge_golden_entries`] / [`mesh_golden_entries`] produces the
    /// exact JSON string pinned in the table. A byte mismatch means a
    /// consumer that dedup'd on `id` yesterday now sees a different
    /// record for the same semantic error — a wire-format regression.
    /// Update the goldens deliberately (alongside a schema-version
    /// bump, when appropriate), never silently.
    ///
    /// The build-varying `generator` stamp is asserted and stripped by
    /// [`without_generator_stamp`] before the comparison; see there for
    /// why it is not part of the pinned bytes.
    #[test]
    fn diagnostic_goldens_are_byte_stable() {
        let mut mismatches: Vec<String> = Vec::new();
        for (label, err, golden) in forge_golden_entries() {
            let actual = without_generator_stamp(&serde_json::to_string(&single(&err)).unwrap());
            if actual != golden {
                mismatches.push(format!(
                    "\n[{label}]\nexpected: {golden}\n  actual: {actual}"
                ));
            }
        }
        for (label, err, golden) in mesh_golden_entries() {
            let actual = without_generator_stamp(&serde_json::to_string(&single(&err)).unwrap());
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
            let actual = without_generator_stamp(&serde_json::to_string(&single(&err)).unwrap());
            if actual != golden {
                mismatches.push(format!(
                    "\n[{label}]\nexpected: {golden}\n  actual: {actual}"
                ));
            }
        }
        for (label, err, golden) in cli_golden_entries() {
            let actual = without_generator_stamp(&serde_json::to_string(&single(&err)).unwrap());
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
    /// via `ErrorFormat::Human`; upstream consumers consume
    /// `Diagnostic.message` via `--error-format=json`. If the two ever
    /// diverge, the same error gets described two different ways —
    /// operator pages the consumer, consumer's memory references a wording
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
            | LinkPoolRefNotDeclared
            | LinkFramerRefNotDeclared
            | MemPoolSectionConflict
            | MeshDeployStagePoolNotDeclared
            | MeshDeployStagePoolWrongKind
            | PoolSampleTakeWithoutStagePool
            | ScxmlOnSampleLinkNotDeclared
            | ScxmlOnSampleLinkWrongKind
            // Listener-role — `<sce:session-role kind="X"/>`
            // unknown-kind diagnostic rides `Fix::ReplaceOneOf` with
            // the v1 vocabulary (currently `["accept-side"]`). Future
            // kind variants extend the candidate list in lockstep
            // without changing the non-overlap bucket.
            | ScxmlUnknownSessionRoleKind
            // `datamodel="X"` for an X SCE cannot honor: the same shape
            // as the session-role kind above — an enumerated attribute
            // whose legal values are a closed set, so the fix is
            // `Fix::ReplaceOneOf` over that set. `xpath` and a typo take
            // the same repair even though they differ in why.
            | ScxmlUnsupportedDatamodel
            // A standard method name this datamodel does not implement.
            // What it *does* implement is a closed set — the emitter's
            // dispatch table, or the member list of a namespace this
            // repository installs — so the candidates ride `fix` and
            // `expected` stays absent, exactly as the unsupported
            // `datamodel` value above.
            | ExpressionUnsupportedBuiltin
            // A name nothing declares. The candidate set is open in
            // principle — any name could be declared — but what rides
            // `fix` is the closed part of it: the declarations already in
            // this document that are one or two edits away. Same bucket
            // for the same reason, and empty candidates become no `fix`
            // rather than an empty choice.
            | ExpressionUnknownIdentifier
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
            // Worker cross-resolution: link-rx ref-unknown rides
            // `Fix::ReplaceOneOf` with sorted alias lists from
            // `parsed.imports` filtered to kind=link. Precedent:
            // `LinkClassUnsupportedOnTarget` carries closed candidates
            // the same way. (Outbox cross-resolution against statechart
            // / worker docs rides
            // the SCXML-side `compile_scxml_with_imports` orchestrator
            // that builds the cross-doc registry the validator
            // consumes — see `WorkerOutboxRefUnknown` +
            // `WorkerOutboxTargetWrongKind` below.)
            | WorkerLinkRxRefUnknown
            // ── Worker outbox cross-resolution ──
            //   Two of the three outbox axes carry a closed candidate
            //   list (sorted statechart + worker `.inbox` set);
            //   suffix-invalid is deterministic (`{owner}.inbox` is the
            //   unique repair) and rides `NeutralOrDeterministic`
            //   below.
            | WorkerOutboxRefUnknown
            | WorkerOutboxTargetWrongKind
            // ── Bounded-collection cross-doc resolution ──
            //   Two of the three cross-doc codes carry a closed candidate
            //   list (sorted codec + procedure name union for element-
            //   type-not-a-kind; sorted field-name list for index-by-
            //   field-missing). The third (multi-writer-without-atomics)
            //   has no useful closed set across the C4 baseline's
            //   atomic family and rides `NeutralOrDeterministic` below.
            | CollectionElementTypeNotAKind
            | CollectionIndexByFieldMissing
            // Bounded-collection deploy-time capacity resolution: sorted set of
            // declared limit names under `machines.<machine>.limits:`
            // rides `Fix::ReplaceOneOf`. Mirrors the
            // `BufferPoolSectionConflict` precedent for sorted-
            // declared-name candidate sets.
            | CollectionCapacityUnresolved
            // C7-lowering algorithm-over-BC dispatch (RFC §synth-5-A line 311
            // + §synth-5-L line 2611-2618 + 2642-2647). Two of the six
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
            // §synth-5-K `links:` block cross-doc + driver-unknown
            // (RFC §synth-5-K line 2421). All three
            // ride `Fix::ReplaceOneOf`:
            //   - `deploy/link-driver-unknown`: closed candidate set =
            //     known-driver baseline + forge link-doc names (sorted).
            //   - `deploy/link-not-declared-in-deploy`: closed candidate
            //     set = deploy-side link-name set on the same machine.
            //   - `deploy/link-not-declared-in-forge`: closed candidate
            //     set = forge-side link-name set (across the build).
            // The other 6 `links:` block codes have multi-axis or
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
            // deploy/stage-copy-policy-unknown — closed set
            // {warn, error, forbid} (RFC §synth-5-K line 2351 + 2517-2519
            // verbatim, single source of truth at
            // `StageCopyPolicy::ALL`). FixCarriesCandidates over the
            // three values.
            | MeshDeployStageCopyPolicyUnknown
            // mesh/deploy-unknown-binding-field — closed set = the
            // transport's legal binding keys, ordered closest-first so
            // `candidates[0]` is the likely intent behind a typo.
            | MeshDeployUnknownBindingField
            // deploy/stateless-accept-extern-not-
            // whitelisted (RFC §synth-5-K line 2466-2469). Closed set =
            // sorted union of §synth-5-I baseline intrinsics names + any
            // target-plugin-loaded symbol names; Fix::ReplaceOneOf
            // carries the union so authors get a single canonical
            // candidate list independent of which registry the
            // symbol originated in.
            | MeshDeployStatelessAcceptExternNotWhitelisted
            // ── §synth-5-O symbol collision dual-location report.
            //    The two `<file>:<line>` strings ride `Fix::ReplaceOneOf`
            //    as the closed candidate set; the consumer / author picks
            //    which site to rename to break the clash. Two-element
            //    closed set is the smallest legal FixCarriesCandidates
            //    case but the choice surface (which of two sites to
            //    rename) is real, not a degenerate dropdown.
            | TraceabilityStateIdCollision
            // ── RFC variant-default-overlay — closed set =
            //    the codec's declared `<sce:arm value=...>` values
            //    (sorted, hex-formatted). Author replaces their
            //    overlay entry with one of these or removes the
            //    entry. Degenerate empty-set case (codec has no
            //    variant) drops `fix` to `None` per-instance; the
            //    code-level class stays FixCarriesCandidates since
            //    the dominant case carries the closed set.
            | CodecVariantDefaultOverlayArmNotDeclared
            // ── Parent-tag dispatch — parent's own carrier or flag
            //    names form the closed-set candidate list for the
            //    dotted reference repair (carrier-missing case lists
            //    parent fields; flag-missing-on-carrier case lists
            //    flags on the resolved carrier).
            | CodecVariantDispatchFlagNotResolved
            // ── Flag inversion — `flag-bind-input-not-declared`
            //    carries the leaf-side declared input names as a
            //    closed-set candidate list for typo repair.
            //    `flag-input-unbound` rides `Fix::AddAttribute` with the
            //    canonical `<sce:flag-bind input="X" source="..."/>`
            //    repair shape, also closed-form (deterministic), but
            //    `AddAttribute` is not a candidate-list fix and thus
            //    routes to NeutralOrDeterministic below.
            | CodecFlagBindInputNotDeclared
            // NL→IR Mapping Roadmap Item 2 — cross-kind field-not-found
            // carries the imported kind's full member surface as a
            // sorted closed-set `Fix::ReplaceOneOf`. Type-mismatch and
            // circular-dependency siblings ride NeutralOrDeterministic
            // (deterministic repair = author edits a single declared
            // type / removes a cyclic import; no closed candidate set).
            | ValidationCrossKindFieldNotFound
            // NL→IR Mapping Roadmap Item C1 Path A (DL-4' send-side) —
            // the send-side payload field-unknown mirror of cross-kind
            // field-not-found. Carries the EventSchema's declared
            // field set as the same closed-form `Fix::ReplaceOneOf`
            // candidate list so `did_you_mean`-style typo repair
            // surfaces identically on `<send>/<param>` and on
            // `_event.data.<field>` use sites.
            | ValidationEventPayloadFieldUnknown
            // Buffer-pool slot-table layout — both carry the two
            // legal roundings as a closed candidate set, and which one is
            // right is the author's call: rounding down keeps the SRAM
            // budget, rounding up keeps the boundary or the payload
            // capacity. Naming one in prose would be picking for them.
            | MemAlignmentNotPowerOfTwo
            | MemSlotSizeNotAlignmentMultiple => FixCarriesCandidates,

            // ── `expected` carries non-repair metadata ────────
            // `algorithm/append-type-mismatch`: `expected` is the fixed
            // accepted RHS type set ("uint8 or bytes"); the narrowing
            // repair is author-domain, no structured `Fix`.
            // `expression/namespace-not-callable`: `expected` is the
            // namespace's callable members. A repair means choosing one
            // *and* the arguments it takes, which is the author's
            // decision, so the set documents the position rather than
            // proposing an edit.
            ExpressionParseMismatch
            | ExpressionNamespaceNotCallable
            // `expression/namespace-not-a-value`: the same, one position
            // over. There is no edit to propose — the name has to become
            // a member reach, and which member is the document's meaning.
            | ExpressionNamespaceNotAValue
            | MeshExternalAmbiguousEventGroup
            | AlgorithmAppendTypeMismatch => ExpectedIsMetadata,

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
            | XmlPreprocessorNotRun
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
            | ValidationNativeActionPlacement
            | ValidationNativeActionArgument
            | ValidationNativeActionSignatureConflict
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
            | MemDcacheLineSizeNotPowerOfTwo
            | PoolCacheMaintenanceMisplaced
            | PoolSpeculativePrefetchFlagMissing
            | PoolCachePreArmInvalidateMissingOnSpeculativeCore
            | PoolSampleTypestateAttributesDisabled
            | PoolSampleCallbackSignatureNonBorrow
            // Worker kind shared-state encapsulation (RFC §synth-5-D line
            // 911): author repair is "remove the offending
            // `<sce:import>` or refactor body XML to inbox-only access".
            // No closed candidate set — the foreign-namespace path is
            // arbitrary, so `fix: None` ⇒ NeutralOrDeterministic.
            | WorkerSharedMutableState
            // Worker inbox ordering codes: author chooses `acq_rel` vs
            // `relaxed` based on placement; codegen-invariant fires
            // when relaxed coexists with cross-core placement. Both
            // axes are author-judgment (not closed candidate), so
            // `fix: None`.
            | WorkerInboxOrderingUnspecified
            | WorkerInboxOrderingRelaxedAcrossCores
            // Worker scheduler-capacity forge-side anchor
            // (RFC §synth-5-D line 912). Author repair is either adding the
            // worker to `deploy.machines.<m>.workers` or removing the
            // Worker doc; no closed candidate list.
            | WorkerSchedulerUnsupported
            // Worker outbox suffix-invalid axis: spec
            // §synth-5-D line 895 + line 1998 fix the recipient queue name
            // to `inbox`, so the repair is deterministic
            // (`{owner}.inbox`) and rides `Fix::ReplaceWith`. The other
            // two outbox axes (unknown / wrong-kind) carry a closed
            // candidate set and live in FixCarriesCandidates above.
            | WorkerOutboxTargetSuffixInvalid
            // Bounded-collection parse-time structure validators
            // (RFC §synth-5-L lines 2559 + 2655). Neither carries a closed
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
            // Fragment-reassembly variant parse-time structure
            // validators (RFC §synth-5-M lines 2944-2945). Neither carries a
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
            // Fragment-reassembly cross-doc validators
            // (RFC §synth-5-M lines 2946-2995). All
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
            // Reassembly codegen self-check (RFC §synth-5-M lines 2976-2981). Pure
            // template-regression guard with no author-domain repair —
            // "report the bug upstream" is the only path forward.
            // NeutralOrDeterministic mirrors the
            // `mem/inter-pool-padding-not-emitted` precedent for
            // codegen-internal invariants where author-side `actual` /
            // `expected` / `fix` carry no useful information.
            | ReassemblyPeerIdNotZidOnEstablishedSession
            // Listener-pair codegen self-check (RFC §synth-5-C lines
            // 849-856). Pure template-regression guard — mirrors
            // `reassembly/peer-id-not-zid-on-established-session`
            // shape. NeutralOrDeterministic.
            | LinkListenerLinkNotPairedWithEstablishedSibling
            // reassembly-binding-on-unpaired-listener (RFC §synth-5-M
            // lines 2982-2994). Two valid repair paths (add
            // `Accepting.*` substate vs remove the binding) — no
            // closed candidate set. NeutralOrDeterministic.
            | MeshDeployReassemblyBindingOnUnpairedListener
            // Multi-link concurrency codes (RFC §synth-5-N lines
            // 3060-3062). All three are multi-axis author-domain
            // repairs with no closed candidate sets:
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
            // Bounded-collection multi-writer without atomic imports: the item C4 baseline
            // atomic family spans 100+ symbols (load/store/cas/fetch ×
            // 5 widths × multiple orderings) so a `Fix::ReplaceOneOf`
            // candidate list would be neither useful nor compact —
            // author chooses width + ordering + op based on their use
            // case. `fix: None` ⇒ NeutralOrDeterministic.
            | CollectionMultiWriterWithoutAtomics
            // C1 Timer kind diagnostics (RFC §synth-5-D lines 909-910).
            // Both are author-judgment repairs: raise the period
            // above the tick rate, or rebalance the timer count
            // against the wheel depth. No closed candidate set.
            | TimerPeriodBelowTickRate
            | TimerSlotOverflow
            // `<sce:extern>` signature mismatch: deterministic
            // `Fix::Replace` with the canonical sig. The other three
            // codes sit in FixCarriesCandidates above.
            | ExternSignatureMismatch
            // `<sce:extern>` target-plugin baseline-shadowing (spec
            // line 1852): plugin author must rename the conflicting
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
            // `algorithm/append-target-not-buffer`: multi-axis repair
            // (declare a bytes buffer OR retarget an existing one); the
            // visible buffer-name union rides `key_fragments`, no
            // single `Fix::Replace`. Mirrors `AlgorithmForeachSourceNotIterable`.
            | AlgorithmAppendTargetNotBuffer
            | ScxmlTopLevelScriptUnloaded
            // Two-axis repair: declare the data model the document
            // actually uses, or drop the construct. Neither is the
            // author's obvious intent from the code alone, so no single
            // `Fix` is offered.
            | ScxmlNullDatamodelForbidsConstruct
            | ScxmlOnSampleInvalidParent
            | ScxmlOnSampleLinkDuplicateInState
            | ScxmlOnSampleEventNameConflict
            // ScxmlOnSampleLinkNotDeclared + ScxmlOnSampleLinkWrongKind
            // sit in FixCarriesCandidates above (cross-ref ride
            // `Fix::ReplaceOneOf` with the registry's name list).
            // Listener-role duplicate-declaration: repair
            // is single-axis (delete the duplicate); no closed
            // candidate list. NeutralOrDeterministic.
            | ScxmlDuplicateSessionRoleDeclaration
            // Listener-role partial-claim codes — all
            // three are NeutralOrDeterministic (2-axis repair,
            // no closed candidate set).
            | LinkDeployRoleListenerWithoutScxmlAcceptSideRole
            | ScxmlAcceptSideRoleWithoutListenerLink
            | LinkRoleListenerWithNonSessionArmingTrustClass
            // Listener-role migration-helper —
            // NeutralOrDeterministic (2-axis repair: add role or
            // rename states).
            | ScxmlAcceptSideStatesWithoutRoleDeclaration
            // Declared-consumption — 3-axis repair (raise
            // peer_table.capacity, raise per_peer_quota, or lower
            // slot_count); no closed candidate set.
            | ReassemblyPerPeerQuotaBuildInvariantViolated
            | ExpressionEmpty
            | ExpressionLex
            | ExpressionUnsupportedConstruct
            // The callee was typed by having been written, and nothing
            // that could stand in its place follows from that — so no
            // fix, and `expected` stays absent with it.
            | ExpressionLiteralNotCallable
            | ExpressionStrictEquality
            // The name that was called is the name that repairs it, so
            // there is one replacement rather than a set — and none at
            // all when the call carried arguments, since dropping it
            // would discard them.
            | ExpressionPropertyNotCallable
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
            // Item C3 no_std rejections: author repair is
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
            | CodecVariantArmMidMismatch
            | CodecVariantArmInnerMidUndeclared
            | CodecVariantArmBodyCallerTagUnsupported
            | CodecVariantNoDefaultArm
            | CodecVariantDispatchBitWidthMismatch
            | CodecVariantDispatchArmsNotDistinguishableWithoutDefault
            | CodecVariantDispatchFlagHasStaticValue
            | CodecVariantDispatchCarrierAfterEmbed
            | CodecPresentIfRefsLaterField
            | CodecRepeatCountRefsLaterField
            | AlgorithmTestVectorUnsupportedKind
            | CodecTlvChainDepthUnspecified
            | CodecTlvChainTruncateUnderEntryFlag
            | CodecDmaAlignmentUnsatisfiable
            | CodecPeekByteFlagLayoutMismatch
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
            // The refusal names one fact about the run — the backend
            // emits no suite name, or the name is unspellable, or the
            // tree is this repository's own. None of the three has a
            // candidate list to offer, and none carries an `expected`.
            | CliInvalidSuitePackage
            // The argument parser's message already carries the usage
            // line, and the repair is a different command line rather
            // than a substitution into this one — there is no token in
            // a record to replace.
            | CliUsage
            // Nothing was expected and nothing is offered: the query was
            // legal and the artifact simply holds no answer.
            | CliQueryNoMatch
            | MeshDeployRead
            | MeshDeployParse
            | MeshDeployDuplicateMachine
            | MeshDeployInvalidOrderingTimings
            | MeshDeployInvalidDedupWindow
            | MeshDeployInvalidCustomTcpSocket
            | MeshDeployInvalidDdsQos
            | MeshDeployInvalidLiveliness
            | MeshDeployInvalidServerResponseDeadline
            | MeshDeployInvalidOutboundBuffer
            | MeshDeployInvalidRetryPolicy
            | MeshDeployInvalidAuthPolicy
            | MeshDeployDiscoveryNotSupported
            | MeshDeployPoolNotSupportedByTransport
            | MeshDeployPoolMissingMemberList
            | MeshDeployPoolEmptyMemberList
            | MeshDeployPoolBindingFieldNotSupported
            | MeshDeployPoolDispatchWithoutMember
            | MeshDeployPoolInvalidPlaceholder
            | MeshDeployServerPoolNotSupported
            | MeshDeployCrossTargetReplyNotSupported
            | MeshDeployInvalidReplyFrom
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
            // Scheduler-capacity deploy-side anchors (RFC §synth-5-K
            // lines 2423 / 2428-9 / 2430-1). Author repair = add the
            // missing field or rebalance worker count; no closed
            // candidate list, so `fix: None` ⇒ NeutralOrDeterministic.
            | MeshDeploySchedulerCooperativeMissingSlotBudget
            | MeshDeploySchedulerCooperativeMissingKeepaliveJitterBudget
            | MeshDeploySchedulerIncompatibleWithWorkerCount
            // §synth-5-K `links:` block parse-level + multi-axis repairs
            // (RFC §synth-5-K lines 2440-2503). 6 of the 9 `links:` block codes
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
            // Cross-doc RX-pool burst invariants. Both ride
            // NeutralOrDeterministic — repair is multi-axis per spec
            // (raise slot_count, lower tick_period_us, switch rx_dispatch
            // mode); author chooses the axis fitting the deployment
            // budget. No closed candidate set.
            | MeshDeployLinkBurstAbsorptionInsufficient
            | MeshDeployLinkRxDispatchWorkerTickOnHighBurst
            // Stage-copy promotion + opt-out rejection. Both
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
            // Anti-flood + stateless_accept. All five ride
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
            // Peer-table invariant. NeutralOrDeterministic
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
            // ── §synth-6.2.6 generated-source drift (B9). Repair is the
            //    deterministic `sce-codegen <regen-command>` — no
            //    candidate set across multiple repair paths. ──
            | ForgeSourceHashMismatch
            // ── Generator content witness. Both repairs are single and
            //    deterministic — rebuild the binary, or build one from a
            //    tree it can read — so neither offers candidates, and the
            //    two digests ride `message` + `actual` rather than
            //    `expected`, which no CLI-family code carries. ──
            | CliGeneratorSourceDrift
            | CliGeneratorSourceUnverifiable
            // ── §synth-6.2.6 source-set coverage guard. One repair
            //    path — re-point `--input-root` at a directory that
            //    contains the input — so there is no candidate set. ──
            | ForgeSourceHashInputUncovered
            // ── §synth-6.2.6 source-set enumeration ceiling. One repair
            //    path — re-point `--input-root` below the directory-link
            //    aliasing, or remove it — so there is no candidate set. ──
            | ForgeSourceHashWalkUnbounded
            // ── §synth-5-O IR provenance guard. Codegen-internal
            //    invariant: an empty source_location means the parser
            //    site that produced this node failed to attach a
            //    SourceLocation. No author repair — the fix lives in
            //    the parser site, not the document. ──
            | TraceabilityScxmlLineRangeMissing
            // ── §synth-5-O — three of the four sourcemap-contract codes ride
            //    NeutralOrDeterministic. Symbol-length: multi-axis
            //    repair (shorten any of three contributing names OR
            //    relax the strict flag). Sourcemap-source-hash drift:
            //    regenerate via `sce-codegen generate`. SCE-MAP-attribute-
            //    stripped: dual-emit fallback covers; the diagnostic is
            //    a heads-up not a hard repair. ──
            | TraceabilitySymbolNameExceedsCIdentifierLimit
            | TraceabilitySourcemapSourceHashMismatch
            | TraceabilitySceMapAttributeStripped
            // ── §synth-5-O — ownership-boundary walker.
            //    Codegen-internal invariant: no author repair, the
            //    fix is in the template that lost its SCE-MAP macro
            //    call. NeutralOrDeterministic since the diagnostic is
            //    informational toward upstream pipeline-bug repair. ──
            | TraceabilityMetaGeneratedSourceLineMarkerMissing
            // ── MCU driver/class boundary (SCE Protocol-Synthesis RFC §5.2)
            //    — both codes ride NeutralOrDeterministic.
            //    `mcu/driver-header-not-found`: author-domain repair
            //    (fix href / add file / set driver_root) — no closed
            //    candidate set. `mcu/section-attribute-on-non-mcu-target`:
            //    multi-axis repair (remove the section / switch backend
            //    to c11 / split deploys) — also open-ended. Both fall
            //    outside FixCarriesCandidates by design.
            | McuDriverHeaderNotFound
            | McuSectionAttributeOnNonMcuTarget
            | McuSectionAttributeNameInvalid
            // ── Flag inversion — flag-bind sibling codes that
            //    don't carry candidate lists:
            //   - `flag-bind-source-not-resolved`: open-ended repair
            //     (author edits the source attribute or adds a missing
            //     local input); no closed candidate set.
            //   - `flag-bind-width-mismatch`: invariant repair (v1
            //     fixes width=1); deterministic single repair, no
            //     candidate list.
            //   - `flag-input-unbound`: deterministic `Fix::AddAttribute`
            //     with the canonical `<sce:flag-bind input="X"
            //     source="..."/>` repair shape — no candidate list.
            //   - `flag-bind-duplicate-input`: deterministic "delete
            //     one of the duplicates"; no candidate list.
            //   - `flag-bind-carrier-after-embed`: deterministic
            //     reorder repair (move carrier before embed); no
            //     candidate list. ──
            | CodecFlagBindSourceNotResolved
            | CodecFlagBindWidthMismatch
            | CodecFlagInputUnbound
            | CodecFlagBindDuplicateInput
            | CodecFlagBindCarrierAfterEmbed
            // NL→IR Mapping Roadmap Item 1 — duplicate sce:req id.
            // Deterministic repair (drop the second occurrence); no
            // closed candidate set, opaque token by design.
            | ValidationDuplicateRequirementId
            // NL→IR Mapping Roadmap Item 5 — unresolved placeholder
            // under `--strict-unresolved`. Deterministic repair
            // (resolve the marker and replace the value); no closed
            // candidate set unless the author populated
            // `sce:unresolved-candidates`, which lives in the model
            // for IDE / linter consumers rather than the diagnostic
            // wire (`Fix::ReplaceOneOf` would inflate every record
            // even when no candidates were declared).
            | ValidationUnresolvedPlaceholder
            // NL→IR Mapping Roadmap Item 2 — type-mismatch repair is
            // deterministic per use site (edit the field's declared
            // type or the use-site's expected type, single axis the
            // author chooses); circular-dependency repair removes one
            // edge from the cycle (named in `cycle` but the choice of
            // which edge is author-domain). Neither carries a closed
            // candidate set on the diagnostic wire.
            | ValidationCrossKindTypeMismatch
            | ValidationCrossKindCircularDependency
            // NL→IR Mapping Roadmap Item 3 — reachability codes
            // ship without a closed candidate set. Repair for an
            // unreachable state is author-domain (delete the orphan or
            // re-connect via a new transition); listing every reachable
            // state as a `ReplaceOneOf` candidate would mis-signal that
            // a rename is the expected fix, when the author almost
            // always meant to wire a transition rather than match an
            // existing state name. Same reasoning for dead-transition:
            // the source state's id is correct, the graph topology is
            // not.
            | ScxmlUnreachableState
            | ScxmlDeadTransition
            // NL→IR Mapping Roadmap Item 3 — non-exhaustive
            // event handling. Repair has three axes (add the
            // transition, add a parent-level fallthrough, or declare
            // the absence on the non-handling child with
            // `sce:unhandled`) none of which pick a fixed candidate
            // set; `ReplaceOneOf` would imply a rename when the author
            // almost always meant to add a missing handler or accept
            // the gap deliberately.
            | ScxmlNonExhaustiveEventHandling
            // The two declaration-hygiene codes: the repair is to
            // delete one of two things the document says, and which
            // one is the author's call, not a candidate list.
            | ScxmlContradictoryUnhandledDeclaration
            | ScxmlStaleUnhandledDeclaration
            // NL→IR Mapping Roadmap Item 3 — always-false
            // guards and shadowed transitions. Repair is deterministic
            // per use-site (remove the dead transition, rewrite the
            // guard, or reorder) with no closed candidate set the
            // validator can predict.
            | ScxmlAlwaysFalseGuard
            | ScxmlShadowedTransition
            // NL→IR Item C1 Path A: Enum kind invariants. The five
            // codes don't carry author-actionable closed candidate
            // lists (variant names/values are author-defined; the
            // overflow / duplicate diagnostics name the conflicting
            // pair without proposing a replacement). The unsupported-
            // underlying code could ride `Fix::ReplaceOneOf` with
            // the four legal carriers; that refinement stays out
            // until a consumer needs it.
            | ValidationEnumNoVariants
            | ValidationEnumVariantDuplicateName
            | ValidationEnumVariantDuplicateValue
            | ValidationEnumVariantValueOverflowsUnderlying
            | ValidationEnumUnsupportedUnderlyingType
            // NL→IR Item C1 Path A: EventSchema built-in-
            // event schema rejection — repair is author-domain
            // (rename `sce:event-name` to a non-reserved value or
            // delete the schema document). Reserved-prefix list is
            // closed but the legal replacement event name is open,
            // so no `ReplaceOneOf` set fits — NeutralOrDeterministic.
            | ValidationEventSchemaOnBuiltinEvent
            // RFC `rfc-eventschema-bytes-guard.md` §bytesguard-3 B3: ordering
            // operator on a bytes payload. Repair is author-domain
            // (switch to `===`/`!==`, or compare a different field) —
            // deterministic, no closed candidate set the validator can
            // predict, so NeutralOrDeterministic.
            | ValidationBytesComparisonNotEquality
            // NL→IR Item C1 Path A: mesh cross-machine
            // EventSchema mismatch — repair is two-axis (realign
            // sender ↔ receiver schemas by editing one side's field
            // declarations, or declare a schema on the side that is
            // missing it). Neither axis names a closed candidate set
            // the validator can predict — author-domain choice.
            | MeshEventSchemaMismatch => NeutralOrDeterministic,
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
            ExprError::NamespaceNotCallable {
                namespace: "Math".into(),
                members: vec!["Math.abs".into()],
            }
            .into(),
        ];

        let mesh_samples: Vec<MeshError> = vec![ExternalConfigError::AmbiguousEventGroup {
            machine: "ecu_a".into(),
            target: "#motor".into(),
            event_group: "overspeed".into(),
            count: 3,
            config_path: "vsomeip.json".into(),
        }
        .into()];

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
                | XmlPreprocessorNotRun
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
                | ValidationDynamicFeatures | ValidationNativeActionPlacement
                | ValidationNativeActionArgument
                | ValidationNativeActionSignatureConflict | ValidationMeshRpcReservedParam
                | ValidationMeshRpcMissingTarget
                | ValidationMeshRpcDuplicateTarget
                | ValidationRemovedAttribute
                | ValidationBytesMaxSizeViolation
                | ValidationDuplicateRequirementId
                | ValidationUnresolvedPlaceholder
                | ValidationCrossKindFieldNotFound
                | ValidationCrossKindTypeMismatch
                | ValidationCrossKindCircularDependency
                | AlgorithmLocalShadowsParam
                | AlgorithmLvalueUnsupported
                | AlgorithmReturnMissing
                | AlgorithmForeachSourceNotIterable
                | AlgorithmCallTargetUnknown
                | AlgorithmCallTargetMethodUnknown
                | AlgorithmBcMutationForbidden
                | AlgorithmForeachSourceBcWithBytesItemType
                | AlgorithmCallArgCountMismatch
                | AlgorithmAppendTargetNotBuffer
                | AlgorithmAppendTypeMismatch
                | ScxmlTopLevelScriptUnloaded
                | ScxmlUnsupportedDatamodel
                | ScxmlNullDatamodelForbidsConstruct
                | ScxmlUnreachableState
                | ScxmlDeadTransition
                | ScxmlNonExhaustiveEventHandling
                | ScxmlContradictoryUnhandledDeclaration
                | ScxmlStaleUnhandledDeclaration
                | ScxmlAlwaysFalseGuard
                | ScxmlShadowedTransition
                | ScxmlOnSampleInvalidParent
                | ScxmlOnSampleLinkDuplicateInState
                | ScxmlOnSampleEventNameConflict
                | ScxmlOnSampleLinkNotDeclared
                | ScxmlOnSampleLinkWrongKind
                | ScxmlUnknownSessionRoleKind
                | ScxmlDuplicateSessionRoleDeclaration
                | LinkDeployRoleListenerWithoutScxmlAcceptSideRole
                | ScxmlAcceptSideRoleWithoutListenerLink
                | LinkRoleListenerWithNonSessionArmingTrustClass
                | ScxmlAcceptSideStatesWithoutRoleDeclaration
                | ReassemblyPerPeerQuotaBuildInvariantViolated
                | ExpressionEmpty | ExpressionLex
                | ExpressionUnsupportedConstruct | ExpressionUnsupportedBuiltin
                | ExpressionUnknownIdentifier
                | ExpressionPropertyNotCallable
                | ExpressionNamespaceNotCallable
                | ExpressionNamespaceNotAValue
                | ExpressionLiteralNotCallable
                | ExpressionStrictEquality
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
                | CodecVariantArmMidMismatch
                | CodecVariantArmInnerMidUndeclared
                | CodecVariantArmBodyCallerTagUnsupported
                | CodecVariantNoDefaultArm
                | CodecVariantDefaultOverlayArmNotDeclared
                | CodecVariantDispatchFlagNotResolved
                | CodecVariantDispatchBitWidthMismatch
                | CodecVariantDispatchArmsNotDistinguishableWithoutDefault
                | CodecVariantDispatchFlagHasStaticValue
                | CodecVariantDispatchCarrierAfterEmbed
                | CodecPresentIfRefsLaterField
                | CodecRepeatCountRefsLaterField
                | AlgorithmTestVectorUnsupportedKind
                | CodecTlvChainDepthUnspecified
                | CodecTlvChainTruncateUnderEntryFlag
                | CodecDmaAlignmentUnsatisfiable
                | CodecPeekByteFlagLayoutMismatch
                | LinkFramerMissing
                | LinkLinkClassUnknown
                | LinkBackpressureUndeclared
                | LinkClassUnsupportedOnTarget
                | LinkPoolSlotSmallerThanFramerMax
                | LinkPoolRefNotDeclared
                | LinkFramerRefNotDeclared
                | MemPoolSectionConflict
                | MemPoolTooLarge
                | MemInterPoolPaddingNotEmitted
                | MemCacheLineAlignment
                | MemDcacheLineSizeNotPowerOfTwo
                | MemAlignmentNotPowerOfTwo
                | MemSlotSizeNotAlignmentMultiple
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
                | CliInvalidSuitePackage
                | CliGeneratorSourceDrift | CliGeneratorSourceUnverifiable
                | CliUsage | CliQueryNoMatch
                | MeshDeployRead | MeshDeployParse | MeshDeployUnsupportedVersion
                | MeshDeployDuplicateMachine | MeshDeployInvalidOrderingTimings
                | MeshDeployInvalidDedupWindow
                | MeshDeployInvalidCustomTcpSocket
                | MeshDeployInvalidDdsQos
                | MeshDeployInvalidLiveliness
                | MeshDeployInvalidServerResponseDeadline
                | MeshDeployInvalidOutboundBuffer
                | MeshDeployInvalidRetryPolicy
                | MeshDeployInvalidAuthPolicy
                | MeshDeployDiscoveryNotSupported
                | MeshDeployPoolNotSupportedByTransport
                | MeshDeployPoolMissingMemberList
                | MeshDeployPoolEmptyMemberList
                | MeshDeployPoolBindingFieldNotSupported
                | MeshDeployPoolDispatchWithoutMember
                | MeshDeployPoolInvalidPlaceholder
                | MeshDeployServerPoolNotSupported
                | MeshDeployCrossTargetReplyNotSupported
                | MeshDeployInvalidReplyFrom
                | MeshDeployUnknownBindingField
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
                // B9 §synth-6.2.6 generated-source drift detection
                | ForgeSourceHashMismatch
                // §synth-6.2.6 source-set coverage guard
                | ForgeSourceHashInputUncovered
                // §synth-6.2.6 source-set enumeration ceiling
                | ForgeSourceHashWalkUnbounded
                // §synth-5-O IR provenance pre-emit guard
                | TraceabilityScxmlLineRangeMissing
                // §synth-5-O symbol mangling + sourcemap contract
                | TraceabilityStateIdCollision
                | TraceabilitySymbolNameExceedsCIdentifierLimit
                | TraceabilitySourcemapSourceHashMismatch
                | TraceabilitySceMapAttributeStripped
                // §synth-5-O boundary walker
                | TraceabilityMetaGeneratedSourceLineMarkerMissing
                // MCU driver/class boundary (SCE Protocol-Synthesis RFC §5.2)
                // codes; both ride NeutralOrDeterministic per
                // the non_overlap_class match above.
                | McuDriverHeaderNotFound
                | McuSectionAttributeOnNonMcuTarget
                | McuSectionAttributeNameInvalid
                // Flag inversion — parent-side flag-bind cross-doc
                // validator codes. One in FixCarriesCandidates
                // (`flag-bind-input-not-declared`) and five in
                // NeutralOrDeterministic (the other five) per the
                // `non_overlap_class` match above.
                | CodecFlagBindInputNotDeclared
                | CodecFlagBindSourceNotResolved
                | CodecFlagBindWidthMismatch
                | CodecFlagInputUnbound
                | CodecFlagBindDuplicateInput
                | CodecFlagBindCarrierAfterEmbed
                // NL→IR Item C1 Path A: Enum kind invariants
                | ValidationEnumNoVariants
                | ValidationEnumVariantDuplicateName
                | ValidationEnumVariantDuplicateValue
                | ValidationEnumVariantValueOverflowsUnderlying
                | ValidationEnumUnsupportedUnderlyingType
                // NL→IR Item C1 Path A: EventSchema kind
                | ValidationEventSchemaOnBuiltinEvent
                | ValidationEventPayloadFieldUnknown
                | ValidationBytesComparisonNotEquality
                | MeshEventSchemaMismatch => true,
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
            356,
            "ALL_DIAGNOSTIC_CODES has duplicates or missing entries — \
             expected 356 distinct variants to match the DiagnosticCode \
             enum. When a commit adds or removes a variant, update this \
             count in the same commit and follow the variant checklist: \
             SCE_ERROR_CONTRACT.md plus the acceptance-doc appendix \
             (`acceptance_doc_covers_every_code` below pins the latter).",
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
    /// The other direction of `acceptance_doc_covers_every_code`: every
    /// diagnostic slug a **contract document** quotes must be a code that
    /// exists.
    ///
    /// The appendix gate proves code → doc (no variant ships
    /// undocumented). Nothing proved doc → code, so a document could name
    /// a slug that had been renamed, or one that was deleted along with
    /// the feature it belonged to, and no test disagreed. That is not
    /// hypothetical: `SCE_MESH.md` described
    /// `validate_someip_scxml_invoke_service_id_collisions` and the slug
    /// `mesh/deploy-someip-scxml-invoke-service-id-collision` across three
    /// paragraphs and a roadmap row marked LANDED, months after RFC F.X-1
    /// replaced the FNV-1a derivation with a counter allocator and deleted
    /// both. Three pool slugs were stale the same way, from a rename.
    ///
    /// **Scope is contract documents only**, and deliberately so. Design
    /// drafts and rejected-design ADRs quote slugs for codes that are
    /// *proposed* or *refused* — a phantom slug there is the document
    /// doing its job. The two files below are the ones that describe what
    /// SCE does today, so a phantom slug in them is a false claim about
    /// shipped behaviour.
    #[test]
    fn contract_docs_cite_only_real_codes() {
        const CONTRACT_DOCS: &[(&str, &str)] = &[
            ("SCE_MESH.md", include_str!("../../../SCE_MESH.md")),
            (
                "docs/SCE_ACCEPTED_SUBSET.md",
                include_str!("../../../docs/SCE_ACCEPTED_SUBSET.md"),
            ),
            // The wire contract itself. It was absent from this list for
            // as long as the list existed, and absent from the code → doc
            // direction too, which is how its §5 came to promise the full
            // enumeration while naming under a third of it. §5's tables
            // are generated now, so what this direction still guards is
            // the prose around them — every slug quoted in §2 through §11
            // is a claim about what SCE emits.
            (
                "SCE_ERROR_CONTRACT.md",
                include_str!("../../../SCE_ERROR_CONTRACT.md"),
            ),
        ];
        let real: std::collections::BTreeSet<&str> =
            ALL_DIAGNOSTIC_CODES.iter().map(|c| c.as_str()).collect();
        // Only tokens whose prefix is a stage SCE actually emits are
        // candidates — a backticked `foo/bar` that is a path, a URL
        // fragment, or a config key must not be read as a slug.
        let stages: std::collections::BTreeSet<&str> =
            real.iter().filter_map(|s| s.split('/').next()).collect();

        let mut phantom: Vec<String> = Vec::new();
        for (name, text) in CONTRACT_DOCS {
            for token in text.split('`') {
                let Some((stage, rest)) = token.split_once('/') else {
                    continue;
                };
                if !stages.contains(stage) || rest.is_empty() {
                    continue;
                }
                if !rest
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
                {
                    continue;
                }
                if !real.contains(token) {
                    phantom.push(format!("{name}: `{token}`"));
                }
            }
        }
        phantom.sort();
        phantom.dedup();
        assert!(
            phantom.is_empty(),
            "contract document(s) quote diagnostic slugs that do not exist:\n  {}\n\n\
             Either the code was renamed (update the document) or the feature was \
             removed (rewrite the paragraph — a slug is a claim that SCE emits it). \
             Only contract documents are in scope: they describe shipped behaviour, \
             while design drafts legitimately name proposed or refused codes.",
            phantom.join("\n  ")
        );
    }

    /// Markers bounding the generated code catalog in
    /// `SCE_ERROR_CONTRACT.md` §5.
    const CATALOG_BEGIN: &str = "<!-- BEGIN GENERATED: code catalog -->";
    const CATALOG_END: &str = "<!-- END GENERATED: code catalog -->";

    fn error_contract_path() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("sce-build's parent is the repo root")
            .join("SCE_ERROR_CONTRACT.md")
    }

    /// One catalog row, with every column taken from the record the
    /// code actually produces.
    ///
    /// `fix` and `stage` are collected as sets rather than single
    /// values because neither is a property of the *code* — a code is
    /// emitted from sites that may name different repairs. A row that
    /// picked one and dropped the rest would be a claim the goldens do
    /// not support, so a code with two answers renders both.
    #[derive(Default)]
    struct CatalogRow {
        stages: std::collections::BTreeSet<&'static str>,
        fixes: std::collections::BTreeSet<String>,
        specs: std::collections::BTreeSet<&'static str>,
    }

    fn fix_kind(fix: &Fix) -> String {
        let v = serde_json::to_value(fix).expect("Fix serialises");
        v.get("kind")
            .and_then(serde_json::Value::as_str)
            .expect("Fix is an internally tagged enum with a `kind`")
            .to_string()
    }

    fn catalog_rows(
        diags: impl IntoIterator<Item = Diagnostic>,
        into: &mut std::collections::BTreeMap<&'static str, CatalogRow>,
    ) {
        for d in diags {
            let row = into.entry(d.code.as_str()).or_default();
            row.stages.insert(d.stage.as_str());
            match &d.fix {
                Some(f) => {
                    row.fixes.insert(format!("`{}`", fix_kind(f)));
                }
                None => {
                    row.fixes.insert("no".to_string());
                }
            }
            if let Some(spec) = d.spec {
                row.specs.insert(spec);
            }
        }
    }

    fn render_catalog_table(rows: &std::collections::BTreeMap<&'static str, CatalogRow>) -> String {
        let mut out = String::from("| Code | Stage | Fix? | Spec |\n|---|---|---|---|\n");
        for (code, row) in rows {
            out.push_str(&format!(
                "| `{}` | {} | {} | {} |\n",
                code,
                row.stages
                    .iter()
                    .map(|s| format!("`{s}`"))
                    .collect::<Vec<_>>()
                    .join(" / "),
                row.fixes.iter().cloned().collect::<Vec<_>>().join(" / "),
                row.specs.iter().copied().collect::<Vec<_>>().join(" / "),
            ));
        }
        out
    }

    /// Render §5's catalog from the goldens, grouped by the pipeline
    /// that produces each code.
    fn render_code_catalog() -> String {
        let mut forge = std::collections::BTreeMap::new();
        catalog_rows(
            forge_golden_entries().iter().map(|(_, e, _)| single(e)),
            &mut forge,
        );
        catalog_rows(
            xsd_golden_entries()
                .iter()
                .flat_map(|(_, e, _)| e.to_diagnostics()),
            &mut forge,
        );
        let mut cli = std::collections::BTreeMap::new();
        catalog_rows(
            cli_golden_entries().iter().map(|(_, e, _)| single(e)),
            &mut cli,
        );
        let mut mesh = std::collections::BTreeMap::new();
        catalog_rows(
            mesh_golden_entries().iter().map(|(_, e, _)| single(e)),
            &mut mesh,
        );
        // A code produced by more than one pipeline is listed once, under
        // the first that claims it, so the row count below is the code
        // count and not a number larger than it.
        for code in forge.keys().copied().collect::<Vec<_>>() {
            cli.remove(code);
            mesh.remove(code);
        }
        for code in cli.keys().copied().collect::<Vec<_>>() {
            mesh.remove(code);
        }

        let mut out = String::new();
        for (title, rows) in [
            ("5.1 Forge", &forge),
            ("5.2 CLI", &cli),
            ("5.3 Mesh", &mesh),
        ] {
            out.push_str(&format!("### {title}\n\n"));
            out.push_str(&render_catalog_table(rows));
            out.push('\n');
        }
        out.pop();
        out
    }

    /// §5 says it is the full enumeration of `code` values. It was not:
    /// measured 2026-08-12, it named 96 of 346, with fifteen whole
    /// stages absent — and nothing read this file, in either direction.
    /// The sibling gates cover `docs/SCE_ACCEPTED_SUBSET.md` and
    /// `SCE_MESH.md`, so their green hid a document that promised more
    /// than any check asked of it.
    ///
    /// Hand-completing the table would have made a second enumeration
    /// to maintain, and this repository has measured what that costs
    /// more than once. The table is generated from the goldens instead:
    /// `ALL_DIAGNOSTIC_CODES` stays the one source, every column is a
    /// property of the record the code actually emits, and completeness
    /// stops being a claim anyone has to keep true by hand.
    ///
    /// Refresh with:
    ///   `UPDATE_EXPECT=1 cargo test -p sce-build error_contract_catalog`
    #[test]
    fn error_contract_catalog_is_the_full_enumeration() {
        let generated = render_code_catalog();
        let path = error_contract_path();
        let doc = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));

        let begin = doc
            .find(CATALOG_BEGIN)
            .unwrap_or_else(|| panic!("{CATALOG_BEGIN} missing from {}", path.display()))
            + CATALOG_BEGIN.len();
        let end = doc
            .find(CATALOG_END)
            .unwrap_or_else(|| panic!("{CATALOG_END} missing from {}", path.display()));
        assert!(
            begin < end,
            "the generated-catalog markers are inverted in {}",
            path.display()
        );

        // Asserted on what is about to be written, and BEFORE the refresh
        // path can write it. Checking the file afterwards would let a
        // generator that dropped a pipeline round-trip against its own
        // output: the refresh would make the document agree with the bug,
        // and the next plain run would pass. Caught exactly that way —
        // this assertion sat after the early return and a refresh sailed
        // through with three codes unaccounted for.
        let absent: Vec<&str> = ALL_DIAGNOSTIC_CODES
            .iter()
            .map(|c| c.as_str())
            .filter(|c| !generated.contains(&format!("| `{c}` |")))
            .collect();
        assert!(
            absent.is_empty(),
            "the catalog calls itself the full enumeration and the generator \
             omits {} code(s): {absent:#?}",
            absent.len(),
        );

        let wanted = format!("\n\n{generated}\n");
        if std::env::var_os("UPDATE_EXPECT").is_some() {
            let mut next = String::with_capacity(doc.len());
            next.push_str(&doc[..begin]);
            next.push_str(&wanted);
            next.push_str(&doc[end..]);
            std::fs::write(&path, next).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
            return;
        }

        assert_eq!(
            &doc[begin..end],
            wanted,
            "\nSCE_ERROR_CONTRACT.md \u{a7}5 is out of sync with the codes SCE \
             emits. Run:\n  \
             UPDATE_EXPECT=1 cargo test -p sce-build error_contract_catalog\n\
             and commit the regenerated section with the change that moved it.",
        );
    }

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
        let schema_bytes = include_str!("../../../schemas/sce-diagnostic.v1.schema.json");
        let parsed: serde_json::Value =
            serde_json::from_str(schema_bytes).expect("schema file must be valid JSON");
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
    /// of truth or consumers will reject (or accept) the wrong records.
    ///
    /// Loaded via `include_str!` so the test fires at compile time —
    /// no external filesystem assumptions — and parsed with `serde_json`
    /// rather than a schema validator because the guard's concern is
    /// source-of-truth parity, not self-consistency of the schema.
    #[test]
    fn json_schema_enums_match_rust_source_of_truth() {
        const SCHEMA_BYTES: &str = include_str!("../../../schemas/sce-diagnostic.v1.schema.json");
        let schema: serde_json::Value =
            serde_json::from_str(SCHEMA_BYTES).expect("diagnostic schema is valid JSON");

        let code_enum: Vec<String> = schema["properties"]["code"]["enum"]
            .as_array()
            .expect("code.enum is an array")
            .iter()
            .map(|v| {
                v.as_str()
                    .expect("code enum member is a string")
                    .to_string()
            })
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
            .map(|v| {
                v.as_str()
                    .expect("stage enum member is a string")
                    .to_string()
            })
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

        // NL→IR Mapping Roadmap Item 6 — question_kind enum drift.
        // The schema is the wire contract; the Rust constant
        // `ALL_QUESTION_KINDS` is the source of truth for the
        // `as_str` mapping. They must agree byte-for-byte.
        let question_kind_enum: Vec<String> = schema["properties"]["question_kind"]["enum"]
            .as_array()
            .expect("question_kind.enum is an array")
            .iter()
            .map(|v| {
                v.as_str()
                    .expect("question_kind enum member is a string")
                    .to_string()
            })
            .collect();
        let rust_question_kinds: Vec<String> = ALL_QUESTION_KINDS
            .iter()
            .map(|qk| qk.as_str().to_string())
            .collect();
        assert_eq!(
            question_kind_enum, rust_question_kinds,
            "schemas/sce-diagnostic.v1.schema.json question_kind.enum drifted \
             from ALL_QUESTION_KINDS. Update the schema (or the constant) \
             together so the wire contract matches the Rust source of truth.",
        );
    }

    /// Every golden record, as `(label, json)` — the bytes the producer
    /// actually emits.
    ///
    /// The four tables carry different error types, so a caller that
    /// needs the instances must walk them separately. A caller that
    /// needs only the wire bytes does not, and this collapses the
    /// four-arm walk to one place.
    ///
    /// Serialising here rather than handing back the pinned table entry
    /// is what keeps these callers looking at a whole record. The table
    /// entries deliberately omit the build-varying `generator` stamp
    /// (see [`without_generator_stamp`] for why it cannot be written
    /// down), so a schema check run against them would be certifying a
    /// shape no producer emits — and the negative cases below would be
    /// mutating a control that was already invalid.
    /// [`diagnostic_goldens_are_byte_stable`] is what holds these bytes
    /// to the table.
    fn all_golden_json() -> Vec<(&'static str, String)> {
        let mut out: Vec<(&'static str, String)> = Vec::new();
        for (label, err, _golden) in forge_golden_entries() {
            out.push((label, serde_json::to_string(&single(&err)).unwrap()));
        }
        for (label, err, _golden) in mesh_golden_entries() {
            out.push((label, serde_json::to_string(&single(&err)).unwrap()));
        }
        for (label, err, _golden) in xsd_golden_entries() {
            out.push((label, serde_json::to_string(&single(&err)).unwrap()));
        }
        for (label, err, _golden) in cli_golden_entries() {
            out.push((label, serde_json::to_string(&single(&err)).unwrap()));
        }
        out
    }

    /// The published schema, compiled as draft-07.
    fn diagnostic_schema() -> serde_json::Value {
        serde_json::from_str(include_str!(
            "../../../schemas/sce-diagnostic.v1.schema.json"
        ))
        .expect("diagnostic schema is valid JSON")
    }

    /// Schema violations for one record, as message strings.
    ///
    /// The instance is bound before the validator runs and the error
    /// iterator is drained into owned strings inside the match, so the
    /// borrow it holds ends before the instance drops.
    fn diagnostic_schema_violations(line: &str) -> Vec<String> {
        let instance: serde_json::Value =
            serde_json::from_str(line).expect("diagnostic record is JSON");
        let schema_value = diagnostic_schema();
        let validator = jsonschema::JSONSchema::options()
            .with_draft(jsonschema::Draft::Draft7)
            .compile(&schema_value)
            .expect("diagnostic schema compiles as draft-07");
        let msgs: Vec<String> = match validator.validate(&instance) {
            Ok(()) => Vec::new(),
            Err(errors) => errors.map(|e| e.to_string()).collect(),
        };
        msgs
    }

    /// Every diagnostic this crate can emit validates against the
    /// published schema.
    ///
    /// [`json_schema_enums_match_rust_source_of_truth`] compares two
    /// lists of strings. That guard cannot see a record carrying a
    /// field the schema forbids (`additionalProperties: false`), a
    /// required key the producer stopped emitting, or a type the
    /// schema narrows — those are properties of an instance, and no
    /// instance was ever run through a validator.
    ///
    /// The coverage is total rather than sampled: each golden is
    /// `serde_json::to_string` of a real diagnostic, pinned by
    /// [`diagnostic_goldens_are_byte_stable`], and
    /// [`every_code_has_a_golden`] proves the tables reach every
    /// [`DiagnosticCode`] variant. Validating the tables therefore
    /// certifies every code, not a chosen few.
    #[test]
    fn every_golden_record_validates_against_the_wire_schema() {
        let goldens = all_golden_json();
        assert!(
            goldens.len() >= ALL_DIAGNOSTIC_CODES.len(),
            "collected {} goldens for {} codes; a walk that reaches \
             fewer records than there are codes certifies nothing",
            goldens.len(),
            ALL_DIAGNOSTIC_CODES.len(),
        );

        let mut violations: Vec<String> = Vec::new();
        for (label, golden) in &goldens {
            let msgs = diagnostic_schema_violations(golden);
            if !msgs.is_empty() {
                violations.push(format!("\n[{label}] {msgs:?}\n  {golden}"));
            }
        }
        assert!(
            violations.is_empty(),
            "{} of {} golden records violate \
             schemas/sce-diagnostic.v1.schema.json:{}",
            violations.len(),
            goldens.len(),
            violations.join(""),
        );
    }

    /// Label of the golden every negative case mutates from.
    const NEGATIVE_BASE_GOLDEN: &str = "forge/xml-parse";

    fn golden_by_label(label: &str) -> serde_json::Value {
        let (_, json) = all_golden_json()
            .into_iter()
            .find(|(l, _)| *l == label)
            .unwrap_or_else(|| panic!("no golden labelled {label}"));
        serde_json::from_str(&json).expect("golden record is JSON")
    }

    /// A record the schema must reject, built by changing exactly one
    /// thing in a real producer record.
    ///
    /// The control assertion is what makes the negative mean anything.
    /// Hand-typed records reject for whichever constraint they trip
    /// first: an earlier revision of these cases used the placeholder
    /// `"id":"fnv1a:0"`, which fails the schema's
    /// `^fnv1a:[0-9a-f]{16}$` pattern, so all three rejected on the id
    /// and not one of them reached the constraint it was named after.
    /// Starting from a valid record and asserting its validity first
    /// pins the rejection to the single mutated field.
    fn assert_one_change_is_rejected(
        why: &str,
        mutate: impl FnOnce(&mut serde_json::Map<String, serde_json::Value>),
    ) {
        let mut record = golden_by_label(NEGATIVE_BASE_GOLDEN);
        let control = record.to_string();
        assert!(
            diagnostic_schema_violations(&control).is_empty(),
            "the control record must be valid before mutation, \
             otherwise the rejection below proves nothing: {control}",
        );
        mutate(record.as_object_mut().expect("record is an object"));
        let mutated = record.to_string();
        assert!(
            !diagnostic_schema_violations(&mutated).is_empty(),
            "schema must reject this record ({why}): {mutated}",
        );
    }

    /// The terminal fallback is the one record that never goes through
    /// serde, so nothing else in the suite would notice it drifting
    /// from the shape serde produces.
    #[test]
    fn terminal_fallback_line_matches_what_serde_would_emit() {
        let expected = serde_json::to_string(&Diagnostic::meta_failure(
            Diagnostic::TERMINAL_FALLBACK_MESSAGE,
        ))
        .expect("meta_failure serializes");
        assert_eq!(
            Diagnostic::TERMINAL_FALLBACK_NDJSON,
            expected,
            "the hand-built fallback line drifted from the shape \
             Diagnostic serializes to; update the constant",
        );
    }

    #[test]
    fn terminal_fallback_line_validates_against_the_wire_schema() {
        assert!(
            !Diagnostic::TERMINAL_FALLBACK_NDJSON.contains('\n'),
            "the fallback must be one NDJSON line",
        );
        let msgs = diagnostic_schema_violations(Diagnostic::TERMINAL_FALLBACK_NDJSON);
        assert!(
            msgs.is_empty(),
            "the terminal fallback record violates the wire schema \
             {msgs:?}: {}",
            Diagnostic::TERMINAL_FALLBACK_NDJSON,
        );
    }

    #[test]
    fn diagnostic_schema_rejects_an_unknown_code() {
        assert_one_change_is_rejected("code outside ALL_DIAGNOSTIC_CODES", |obj| {
            obj.insert(
                "code".to_string(),
                serde_json::Value::String("xml/no-such-code".to_string()),
            );
        });
    }

    #[test]
    fn diagnostic_schema_rejects_a_missing_required_field() {
        assert_one_change_is_rejected("message absent", |obj| {
            obj.remove("message");
        });
    }

    /// The generator stamp is required, not merely declared.
    ///
    /// Declaring `generator` under `properties` while leaving it out of
    /// `required` would let a producer that stopped emitting it ship
    /// records every consumer's validator still accepts — documented
    /// and unenforced. That gap is precisely the state this surface was
    /// in before the field existed: the contract told consumers to pin
    /// a commit, and nothing made the payload name one.
    #[test]
    fn diagnostic_schema_rejects_a_missing_generator_stamp() {
        assert_one_change_is_rejected("generator absent", |obj| {
            obj.remove("generator");
        });
    }

    /// The schema closes its object. A producer that starts emitting a
    /// field without declaring it would otherwise ship records every
    /// external validator rejects while this crate stays green.
    #[test]
    fn diagnostic_schema_rejects_an_undeclared_field() {
        assert_one_change_is_rejected("field not declared in properties", |obj| {
            obj.insert(
                "severity".to_string(),
                serde_json::Value::String("warning".to_string()),
            );
        });
    }
}
