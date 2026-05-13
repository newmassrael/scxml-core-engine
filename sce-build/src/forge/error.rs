// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Structured error hierarchy for the SCE Forge pipeline.
//
// Each variant maps to a pipeline stage:
//   Xml        → stages 1-2 (XML parsing, XSD schema validation)
//   Validation → stage 3   (kind-specific semantic validation in parser.rs)
//   Expression → stage 4   (expression transpilation in expr.rs)
//   Import     → stage 5   (cross-file resolution in lib.rs)
//   Manifest   → stage 6   (dependency graph in manifest.rs)
//   Generate   → stage 7   (template rendering in generator.rs)
//   Io         → cross-cutting filesystem errors

use crate::forge::model::{ForgeKind, SceType};
use std::path::PathBuf;

/// Source location attached to an error for machine-readable diagnostics.
///
/// Carried by the [`Located`] wrapper struct (below) to answer *where*
/// an error was raised. The leaf error enums stay focused on *what* is
/// wrong — identity, expected/actual values, stage — and remain
/// orthogonal to position.
#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub file: String,
    pub line: Option<u32>,
    pub col: Option<u32>,
}

/// Error + source-location context as a single unit.
///
/// Preferred over an enum variant on `ForgeError` because:
///   - the leaf error type (`E`) stays narrow — a function that only
///     produces `ValidationError` still advertises `Result<_,
///     Located<ValidationError>>`, not a widened `ForgeError`,
///   - nesting is structurally impossible (`Located<Located<E>>` is
///     a type error, not a runtime surprise),
///   - `exit_code()` / `to_diagnostic()` become plain delegates
///     without a recursive match arm,
///   - diagnostic emission (`to_diagnostic`) can key off the type,
///     not a runtime discriminant.
///
/// This mirrors the wrapper-struct pattern used by `syn::Spanned`,
/// `codespan`, and `miette::Report`.
#[derive(Debug, Clone)]
pub struct Located<E> {
    pub error: E,
    pub location: SourceLocation,
}

impl<E> Located<E> {
    /// Build a located error from its parts.
    ///
    /// `line` and `col` are optional so XSD-level errors (line only),
    /// file-scoped errors (neither), and node-precise errors (both)
    /// share a single constructor.
    pub fn new(
        error: E,
        file: impl Into<String>,
        line: Option<u32>,
        col: Option<u32>,
    ) -> Self {
        Self {
            error,
            location: SourceLocation {
                file: file.into(),
                line,
                col,
            },
        }
    }

}

impl<E: std::fmt::Display> std::fmt::Display for Located<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Delegate: the location rides on the outer record, not the
        // human-readable message. Rendering stays identical to the
        // wrapped error's own Display.
        self.error.fmt(f)
    }
}

impl<E: std::error::Error + 'static> std::error::Error for Located<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

/// Top-level error for the forge code-generation pipeline.
///
/// Variants correspond to pipeline stages so callers can react
/// programmatically (distinct CLI exit codes, IDE diagnostics, etc.)
/// without parsing error message strings.
#[derive(Debug, thiserror::Error)]
pub enum ForgeError {
    #[error(transparent)]
    Xml(#[from] XmlError),

    #[error(transparent)]
    Validation(#[from] ValidationError),

    #[error(transparent)]
    Expression(#[from] ExprError),

    #[error(transparent)]
    Import(#[from] ImportError),

    #[error(transparent)]
    Manifest(#[from] ManifestError),

    #[error(transparent)]
    Generate(#[from] GenerateError),

    /// SCXML semantic-validation failures — distinct from forge
    /// `ValidationError` because the rules come from W3C SCXML §3
    /// reference resolution, not forge-document structure rules.
    /// RFC §W5 D2 keeps `ScxmlSemanticError` as a parallel enum
    /// outside `forge::*` but routes it through `ForgeError` so the
    /// `Located<ForgeError>` plumbing and JSON wire layer apply
    /// uniformly. Wire codes mostly REUSE existing `validation/*`
    /// per W4 D4 fold (concept identity); only `TopLevelScriptUnloaded`
    /// is W3C-SCXML-specific and gets its own `scxml/*` code.
    #[error(transparent)]
    Scxml(#[from] crate::scxml_semantic::ScxmlSemanticError),

    #[error("I/O error on {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
}

// ── Stage 1-2: XML / XSD ───────────────────────────────────────

/// Syntactic errors from XML parsing and XSD schema validation.
#[derive(Debug, thiserror::Error)]
pub enum XmlError {
    #[error("XML parse error: {0}")]
    Parse(String),

    #[error("{0}")]
    SchemaValidation(#[from] crate::forge::xsd_validator::XsdErrors),

    /// SCXML source file not found at the resolved path. Distinct
    /// from generic `ForgeError::Io` so the wire dispatch can surface
    /// the parser-entry retry strategy (PATH_RETRY) without re-parsing
    /// `io::Error::kind()`. Raised by [`crate::parser::SCXMLParser::parse_file`]
    /// when `std::fs::read_to_string` returns
    /// `io::ErrorKind::NotFound`; other I/O failures (permission
    /// denied, etc.) keep flowing through `ForgeError::Io` so the
    /// distinction stays semantically meaningful.
    ///
    /// Mirrors C++ `SCE::parsing::ParseFileNotFound` (RFC §W4 D2).
    #[error("SCXML file not found: {path}")]
    FileNotFound { path: String },

    /// Document parsed as well-formed XML but the root element is not
    /// `<scxml>`. Catches a previously-silent failure mode where a
    /// non-SCXML document (after `classify_document` routed it into
    /// the SCXML pipeline) would parse to an empty model with
    /// downstream `parse_states` finding nothing to walk. Raised by
    /// [`crate::parser::SCXMLParser::parse_impl`] immediately after
    /// `roxmltree::Document::parse` succeeds, before any structural
    /// parsing.
    ///
    /// Mirrors C++ `SCE::parsing::ParseWrongRootElement` (RFC §W4 D2).
    #[error("Root element is not <scxml>, found: <{found}>")]
    WrongRootElement { found: String },

    /// W3C XInclude preprocessing failure. The Rust AOT pipeline
    /// rejects XInclude failures that the C++ runtime would warn
    /// and skip — the two parsers must yield the same effective
    /// document or AOT-generated code silently diverges from
    /// Interpreter-parsed behaviour. Raised by
    /// [`crate::xinclude::expand`] before roxmltree sees the
    /// document.
    #[error(transparent)]
    XInclude(#[from] crate::xinclude::XIncludeError),

    /// `<sce:use>` / `<sce:template>` preprocessing failure. AOT-only
    /// expansion per RFC §6.5 Phase A — the C++ runtime does not
    /// implement template expansion, so documents containing
    /// `<sce:use>` are accepted only through `sce-build`. Raised by
    /// [`crate::template::expand`] immediately after XInclude so
    /// templates see a post-XInclude document.
    #[error(transparent)]
    Template(#[from] crate::template::TemplateError),
}

// ── Stage 3: Semantic validation ───────────────────────────────

/// Semantic validation errors from kind-specific parsing.
///
/// These fire *after* XML is successfully parsed — the document
/// structure is well-formed but violates forge domain rules.
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    /// A required child element is missing.
    /// e.g. "Transform kind requires a <datamodel> element"
    #[error("{kind} kind requires a <{element}> element")]
    MissingElement { kind: ForgeKind, element: String },

    /// A required attribute is missing on an element.
    /// e.g. "Codec field must have an 'id' attribute"
    #[error("{element} must have an '{attr}' attribute")]
    MissingAttribute { element: String, attr: String },

    /// An attribute has a value that is not in the valid set.
    /// e.g. "Unknown sce:type 'blob' on field 'data'"
    #[error("{element}: unknown {attr} value '{value}' (expected: {expected})")]
    InvalidAttribute {
        element: String,
        attr: String,
        value: String,
        expected: String,
    },

    /// The sce:kind value is not recognised or supported.
    #[error("unsupported sce:kind value: '{0}'")]
    UnsupportedKind(String),

    /// A name, key, or state id appears more than once.
    /// e.g. "duplicate state id: 'armed'"
    #[error("{kind}: duplicate {what}: '{id}'")]
    DuplicateId {
        kind: ForgeKind,
        what: String,
        id: String,
    },

    /// Duplicate `<sce:context id="...">` declaration. Orthogonal to
    /// `DuplicateId` because `<sce:context>` is a document-wide
    /// extension scope that can appear in any forge kind (a codec, a
    /// procedure, a statechart) — keying its duplicate signal by
    /// `ForgeKind` would be a semantic lie, so it gets its own variant.
    #[error("duplicate <sce:context id=\"{id}\"> declaration")]
    DuplicateContextObject { id: String },

    /// `<sce:context id="...">` names an identifier that collides with
    /// a type alias the C++ codegen emits on the generated state-machine
    /// class — e.g. `id="policy"` would generate `using PolicyType = ...`
    /// alongside the pre-existing `using PolicyType = <PolicyInstance>;`.
    /// Rejected at parse time so the collision never reaches template
    /// rendering or C++ compilation. `reserved` carries the closed list
    /// of disallowed names so the message can quote it without
    /// duplicating the source of truth.
    #[error(
        "<sce:context id=\"{id}\"> uses reserved name; rename to any identifier not in: {}",
        reserved.join(", ")
    )]
    ReservedContextId {
        id: String,
        reserved: &'static [&'static str],
    },

    /// A required collection (fields, entries, states, …) is empty.
    /// e.g. "Codec kind requires at least one field with byte layout"
    #[error("{kind} kind requires at least one {what}")]
    EmptyCollection { kind: ForgeKind, what: String },

    /// A count doesn't match the expected value.
    /// e.g. "Linear interpolation: value count (5) must match axis breakpoints (4)"
    #[error("{kind}: {detail}")]
    CountMismatch { kind: ForgeKind, detail: String },

    /// Attributes that cannot coexist on the same element.
    /// e.g. "sce:on-miss='error' is incompatible with sce:default"
    #[error("{element}: {detail}")]
    IncompatibleAttributes { element: String, detail: String },

    /// Native-code blocks (`<cpp>`, `<kt>`, `cpp:`/`kt:` conditions)
    /// reference external objects but the document carries no
    /// matching `<sce:context>` declaration. `site` names the offending
    /// surface ("cpp: condition", "kt: condition", "<cpp> action",
    /// "<kt> action") so agents can route the repair at the generator
    /// that owns that site; `detail` carries the offending expression.
    #[error("{site}: {detail}")]
    MissingContext { site: String, detail: String },

    /// A reference (target, initial state, …) doesn't resolve.
    /// e.g. "Initial state 'armed' does not match any state"
    #[error("{kind}: {name} does not match any {what} (available: {available})")]
    InvalidReference {
        kind: ForgeKind,
        name: String,
        what: String,
        available: String,
    },

    /// A direction value is invalid for the context.
    /// e.g. "Transform kind does not support 'internal' direction"
    #[error("{kind} kind does not support '{direction}' direction{}", if field.is_empty() { String::new() } else { format!(" (field '{field}')") })]
    InvalidDirection {
        kind: ForgeKind,
        direction: String,
        field: String,
    },

    /// Numeric parsing failure on an attribute value.
    /// e.g. "Invalid sce:byte value '0xZZ' on field 'crc'"
    #[error("invalid {attr} value '{value}' on {element}: {detail}")]
    NumericParse {
        element: String,
        attr: String,
        value: String,
        detail: String,
    },

    /// An attribute value must not be empty.
    /// e.g. "<sce:helper> 'name' attribute must not be empty"
    #[error("{element} '{attr}' attribute must not be empty")]
    EmptyValue { element: String, attr: String },

    /// Only one instance of a specific attribute is allowed.
    /// e.g. "Only one sce:plausibility attribute allowed"
    #[error("only one {attr} attribute allowed per {kind} kind")]
    SingletonViolation { kind: ForgeKind, attr: String },

    /// At least one of the listed attributes must be present.
    /// e.g. "Timer 'diag' must have either 'sce:event' or 'sce:on-timeout'"
    #[error("{element} must have at least one of: {}", alternatives.join(", "))]
    RequireEither {
        element: String,
        alternatives: Vec<String>,
    },

    /// A forge document was routed to the wrong pipeline.
    /// e.g. statechart kind sent to the forge pipeline, or imported as forge
    #[error("{kind} kind cannot be processed by the forge pipeline")]
    WrongPipeline { kind: ForgeKind },

    /// Statechart uses features the static AOT generator cannot
    /// express (`<invoke srcexpr=...>`, missing initial state,
    /// `_event` metadata, …). `reason` carries which specific blocker
    /// the analyzer detected so agents know whether to rewrite the
    /// document (missing initial) or accept the Interpreter fallback
    /// (dynamic invoke).
    #[error("cannot generate static code for '{name}': {reason}")]
    DynamicFeatures { name: String, reason: String },

    /// Reserved `_mesh_*` `<param>` rule violation on
    /// `<invoke type="sce:mesh-rpc">` (SCE Mesh §9.5). Covers the four
    /// cases the spec calls out as hard build-time errors: the required
    /// `_mesh_event` is missing; `_mesh_event` (or any other reserved
    /// name) appears more than once; an unknown `_mesh_*` name is used
    /// (the prefix is reserved for future metadata); or
    /// `_mesh_deadline_ms` carries a non-integer value.
    ///
    /// `param` names the offending `<param name="...">` so agents can
    /// target the repair at the exact child element; `detail` explains
    /// which specific rule was broken. One variant covers the whole
    /// family because the repair surface is uniform — the author must
    /// rename or retype the param — and an unstructured detail string
    /// follows the existing precedent of `IncompatibleAttributes`.
    #[error("<invoke type=\"sce:mesh-rpc\">: {detail} (param '{param}')")]
    MeshRpcReservedParam { param: String, detail: String },

    /// An SCXML attribute removed by SCE Mesh §13 path B still appears
    /// on a `<send>`. The Stage 1 migration window (parse-tolerant
    /// warning) is closed in Session E1; presence of the attribute is
    /// now a hard build error.
    ///
    /// `attribute` names the offending attribute including its
    /// `sce:` prefix (e.g. `sce:qos`); `event` carries the `<send
    /// event="...">` value when present for locator context.
    #[error("deprecated attribute {attribute} on <send{}> was removed in SCE Mesh §13 path B; pattern is now inferred from event-name conventions and RPC reply pairing from topology structure. Remove the attribute.",
             match event { Some(ev) => format!(" event=\"{ev}\""), None => String::new() })]
    RemovedAttribute {
        attribute: String,
        event: Option<String>,
    },

    /// `<invoke type="sce:mesh-rpc">` is missing both `src` and
    /// `srcexpr`. Exactly one must be present (SCE_MESH.md §9.5).
    #[error("<invoke type=\"sce:mesh-rpc\"> must declare exactly one of `src` or `srcexpr` — both are missing. Add `src=\"#<machine>\"` for a build-time target, or `srcexpr=\"...\"` to pick among declared bindings at runtime.")]
    MeshRpcMissingTarget,

    /// `<invoke type="sce:mesh-rpc">` has both `src` and `srcexpr`. They
    /// are mutually exclusive (SCE_MESH.md §9.5).
    #[error("<invoke type=\"sce:mesh-rpc\"> declares both `src` and `srcexpr` — they are mutually exclusive. Keep only the one matching how the target is chosen (static vs runtime).")]
    MeshRpcDuplicateTarget,

    /// A bytes-typed slot's declared cap is exceeded by an upstream
    /// source's declared cap (helper return, `<send>` response). The
    /// inconsistency is static — the declarations themselves contradict
    /// each other before any runtime data flows. See
    /// `claudedocs/rfc-forge-bytes-bounded.md` §3 B1+B4.
    #[error("{procedure}: {detail}")]
    BytesMaxSizeViolation {
        procedure: String,
        detail: String,
    },

    /// RFC §5.A: a local `<sce:var>` or `<sce:foreach item>` reuses
    /// the name of a parameter (or another local) inside the same
    /// algorithm body. Read/write access becomes ambiguous in v1, so
    /// the parser rejects the reuse before lowering.
    #[error("algorithm: identifier '{name}' shadows {what}")]
    AlgorithmLocalShadowsParam { name: String, what: String },

    /// RFC §5.A: `<sce:assign target=...>` writes to an l-value the
    /// algorithm body cannot mutate. v1 forbids assigning to a
    /// parameter (parameters are read-only) and to the foreach loop
    /// variable. `target` is the offending l-value text;
    /// `restriction` names which rule was hit.
    #[error("<sce:assign target=\"{target}\">: {restriction}")]
    AlgorithmLvalueUnsupported {
        target: String,
        restriction: String,
    },

    /// RFC §5.A: an algorithm declares a non-void `<sce:return type>`
    /// in the signature but the body contains no terminal
    /// `<sce:return expr>` along every code path. v1 detects only the
    /// trivial case (last statement is not a return); flow-sensitive
    /// path tracking lands with §5.F (A4).
    #[error("algorithm: signature declares return type but body's last statement is not <sce:return>")]
    AlgorithmReturnMissing,

    /// RFC §5.A + §5.L line 2642-2647 (C7-lowering): `<sce:foreach
    /// in="X">` where X resolves to neither (a) an algorithm signature
    /// param of type `bytes` nor (b) an `<sce:import
    /// kind="bounded-collection" as="X">` alias. Codegen-time check,
    /// fires from `lower_algorithm_stmt::Foreach`'s source dispatch.
    /// `candidates` is the sorted union of bytes-typed params + BC
    /// import aliases visible at the foreach call site.
    #[error("algorithm: <sce:foreach in=\"{src}\">: source does not resolve to a bytes param or a bounded-collection import alias")]
    AlgorithmForeachSourceNotIterable {
        src: String,
        candidates: Vec<String>,
    },

    /// RFC §5.A line 311 + §5.L line 2642-2647 (C7-lowering): `<sce:call
    /// target="alias.method">` where `alias` does not match any
    /// `<sce:import as="...">` declared in the enclosing algorithm doc.
    /// `candidates` is the sorted list of declared import aliases.
    #[error("algorithm: <sce:call target=\"{target}\">: alias '{alias}' is not a declared import")]
    AlgorithmCallTargetUnknown {
        target: String,
        alias: String,
        candidates: Vec<String>,
    },

    /// RFC §5.A line 311 + §5.L line 2611-2618 (C7-lowering): dotted
    /// call's alias resolves but `method` is not in the kind's
    /// public-method set. For bounded-collection imports the closed
    /// callable set is `{find_by_index, get, get_by_slot, len,
    /// capacity}` (read-only — mutation is rejected separately via
    /// `algorithm/bc-mutation-forbidden`). For algorithm imports the
    /// only callable method name equals the imported algorithm's
    /// declared `name`.
    #[error("algorithm: <sce:call target=\"{target}\">: method '{method}' is not callable on import '{alias}' (kind={kind})")]
    AlgorithmCallTargetMethodUnknown {
        target: String,
        alias: String,
        method: String,
        kind: String,
        candidates: Vec<String>,
    },

    /// RFC §5.A line 333 (algorithms are pure: no heap allocation, no
    /// closures, no exceptions/panics) + §5.L line 2611 (BC mutation
    /// API is `insert`/`remove`). Algorithm-body dispatch into a BC
    /// alias is read-only; `<sce:call target="bc_alias.insert">` or
    /// `bc_alias.remove` violates the purity contract.
    #[error("algorithm: <sce:call target=\"{target}\">: mutating bounded-collection method '{method}' is forbidden from algorithm body (algorithms are pure per RFC §5.A)")]
    AlgorithmBcMutationForbidden {
        target: String,
        method: String,
    },

    /// RFC §5.A v1 + §5.L line 2642-2647 (C7-lowering): `<sce:foreach
    /// in="<bc-alias>">` body declares a `<sce:var name="..."
    /// type="uint8">` — the legacy bytes-iteration pattern where the
    /// loop item is a `u8`. BC iteration carries the element-type, not
    /// `uint8`; the body cannot rely on `u8` semantics.
    #[error("algorithm: <sce:foreach in=\"{src}\"> over bounded-collection: body's <sce:var name=\"{var_name}\" type=\"uint8\"> uses the bytes-iteration pattern but '{src}' is a bounded-collection (item carries element-type)")]
    AlgorithmForeachSourceBcWithBytesItemType {
        src: String,
        var_name: String,
    },

    /// RFC §5.A line 311 (C7-lowering): dotted `<sce:call
    /// target="alias.method">` argument count does not match the
    /// imported callable's signature arity. For algorithm imports the
    /// expected arity comes from the imported `<sce:signature>`'s
    /// `<sce:param>` count. BC methods have fixed arities (1-2 per
    /// §5.L), validated by the same path.
    #[error("algorithm: <sce:call target=\"{target}\">: argument count {actual} does not match callable's arity {expected}")]
    AlgorithmCallArgCountMismatch {
        target: String,
        actual: usize,
        expected: usize,
    },

    /// RFC §5.B variant primitive (B1-β): the variant's enumerated
    /// arms don't cover the tag field's value domain AND no
    /// `<sce:default>` arm catches the unenumerated values. At least
    /// one tag value would reach the runtime decoder with no matching
    /// branch — author resolves by adding `<sce:default type="..."/>`
    /// or by enumerating every missing tag value explicitly. The
    /// `domain_size` is `Some(N)` for practically enumerable tag types
    /// (uint8 = 256, uint16 = 65536); `None` for uint32 / uint64 whose
    /// domain is too large to enumerate.
    #[error(
        "codec '{codec}': variant on tag '{tag_field}' (type {tag_type}) has {arm_count} arm(s) but no <sce:default> declared{} — at least one tag value would have no matching arm at runtime; add <sce:default type=\"...\"/> or enumerate the missing values explicitly",
        match domain_size {
            Some(n) => format!(" (tag type domain has {n} values)"),
            None => String::new(),
        }
    )]
    CodecVariantArmUnreachable {
        codec: String,
        tag_field: String,
        tag_type: String,
        arm_count: usize,
        domain_size: Option<u64>,
    },

    /// RFC §5.B present-if primitive (B1-δ): the predicate on a
    /// `sce:present-if` attribute references a field that is **not**
    /// declared earlier in the same codec — either declared later
    /// (a forward reference, which would require a runtime peek the
    /// streaming decoder cannot perform) or never declared at all.
    /// Author resolves by reordering field declarations so the
    /// referenced flags carrier precedes every consumer, or by
    /// correcting a typo in the predicate's field id.
    #[error(
        "codec '{codec}': field '{field}' has sce:present-if=\"{refers_to}.…\" but '{refers_to}' is not declared earlier in this codec — present-if predicates must reference a flags-bearing carrier that the streaming decoder has already consumed; reorder the fields so the carrier comes first, or correct the predicate"
    )]
    CodecPresentIfRefsLaterField {
        codec: String,
        field: String,
        refers_to: String,
    },

    /// RFC §5.B repeat primitive (B2): the `sce:count` reference on a
    /// `<sce:repeat>` element points to a field that is **not**
    /// declared earlier in the same codec — either declared later (a
    /// forward reference the streaming decoder cannot resolve) or never
    /// declared at all. Author resolves by reordering the count field
    /// to precede the `<sce:repeat>`, or by correcting a typo in the
    /// `sce:count` attribute.
    #[error(
        "codec '{codec}': repeat field '{field}' has sce:count=\"{refers_to}\" but '{refers_to}' is not declared earlier in this codec — repeat count references must resolve to a sibling integer field that the streaming decoder has already consumed; reorder the fields so the count comes first, or correct the attribute"
    )]
    CodecRepeatCountRefsLaterField {
        codec: String,
        field: String,
        refers_to: String,
    },

    /// RFC §5.B test-vector primitive: a `<sce:test-vector>` element
    /// appears under a `sce:kind` other than `algorithm` (B2) or
    /// `codec` (B5-θ). Other kinds (transform / lookup / validator
    /// / etc.) cannot host a hex-bytes round-trip oracle in v1 —
    /// their wire shape is not byte-stable enough to anchor a single
    /// reference vector. Author resolves by moving the test vector
    /// onto a supported kind or expressing the round-trip in the
    /// kind-specific harness oracle.
    #[error(
        "<sce:test-vector> is only supported on sce:kind=\"algorithm\" (B2) and sce:kind=\"codec\" (B5-θ), but '{name}' declares sce:kind=\"{kind:?}\" — move the test vector to an algorithm/codec file or use the kind-specific harness oracle"
    )]
    TestVectorUnsupportedKind {
        /// Forge document name (root `name=` attribute).
        name: String,
        /// Actual kind that the test-vector was declared under.
        kind: ForgeKind,
    },

    /// RFC §5.B B3 TLV chain primitive: `<sce:tlv-chain>` declared
    /// without `max-depth`. The attribute is mandatory because the chain
    /// is MCU-class — the runtime decoder needs a build-time bound to
    /// size its working set and to enforce the iterative-only contract
    /// (RFC line 488 "max-depth MUST be specified for MCU targets" +
    /// line 533 "Iterative parse only; max-depth lowers to a max-iter on
    /// the chain traversal loop"). Author resolves by adding the
    /// attribute (e.g. `max-depth="8"`).
    #[error(
        "codec '{codec}': tlv-chain field '{field}' is missing the required `max-depth` attribute — TLV chain decoders need a build-time bound to size their working set and enforce iterative-only parse (RFC §5.B line 488); add `max-depth=\"N\"` for some N > 0"
    )]
    CodecTlvChainDepthUnspecified {
        codec: String,
        field: String,
    },

    /// RFC §5.B B3 DMA alignment primitive: a field with
    /// `sce:dma-burst-align="N"` cannot be honored at build time —
    /// either its authored `sce:byte` is not divisible by `N`, or one
    /// of its preceding fields is variable-length (vle / length-ref /
    /// tail / repeat / tlv-chain) so the field's wire offset is
    /// runtime-dependent. RFC line 558-583 "fixed-offset positions
    /// only — no VLE-following alignment". Repair is structural:
    /// reorder fields, lower the alignment requirement, or change the
    /// variable predecessor to a fixed-width carrier.
    #[error(
        "codec '{codec}': field '{field}' with sce:dma-burst-align=\"{burst_align}\" cannot be honored — {reason}"
    )]
    CodecDmaAlignmentUnsatisfiable {
        codec: String,
        field: String,
        burst_align: u32,
        reason: String,
    },

    /// RFC §5.B B5-γ parent-flags dependency: a body codec declared
    /// `<sce:requires-parent-flags carrier="X">` but the parent codec
    /// (resolved through a variant arm wire-up) doesn't satisfy the
    /// declared layout. Three orthogonal causes:
    ///   (a) parent codec lacks a field named `<carrier>`;
    ///   (b) the named carrier exists but is not a `<sce:flags>`
    ///       container or is not a uint8 (v1 fixes parent flag
    ///       carrier type at uint8 per Zenoh transport pattern);
    ///   (c) a flag declared in the body's block has a name or
    ///       `bit=` that doesn't match the parent's actual layout.
    /// Repair is structural: fix the body's declared parent-flag
    /// layout to match the parent's carrier shape, or wire the body
    /// codec to a different parent.
    #[error(
        "codec '{body_codec}' (body): requires-parent-flags layout mismatch against parent codec '{parent_codec}' — {reason}"
    )]
    CodecParentFlagMismatch {
        body_codec: String,
        parent_codec: String,
        reason: String,
    },

    /// RFC §5.C B6-α byte-stream link endpoint: `<sce:framer ref="..."/>`
    /// is required on `sce:kind="link"` declarations. Without a framer
    /// reference, the codegen cannot wire the §5.B codec into the RX/TX
    /// path, so the parser rejects the document at authoring time. The
    /// repair is structural — add a `<sce:framer ref="<codec_name>"/>`
    /// child whose `ref` matches a `sce:kind="codec"` document imported
    /// or declared inline.
    #[error(
        "link '{name}': missing required <sce:framer ref=\"...\"/> child — `sce:kind=\"link\"` requires a framer codec reference so RX bytes can be decoded and TX events can be encoded; add a <sce:framer ref=\"<codec_name>\"/> child"
    )]
    LinkFramerMissing {
        /// Link document name (root `name=` attribute).
        name: String,
    },

    /// RFC §5.C B6-γ negative coverage: `<sce:link-class>` body text is
    /// not in the closed enumeration (RFC §5.C lines 765-771 — `udp` /
    /// `tcp` / `serial` / `websocket` / `raw_eth`). Promotes the
    /// generic `validation/invalid-attribute` raised by B6-α to a
    /// dedicated link-kind code so downstream agents can pattern-match
    /// on link-class violations without inspecting the message prose.
    /// Repair: replace `value` with one of the listed candidates.
    #[error(
        "link '{name}': <sce:link-class> body text {value:?} is not in the closed enum {{`udp`, `tcp`, `serial`, `websocket`, `raw_eth`}} per RFC §5.C lines 765-771; replace with one of the listed candidates (OS-specific classes such as `unix_socket` or `qnx_msg` land additively in later phases)"
    )]
    LinkLinkClassUnknown {
        /// Link document name (root `name=` attribute).
        name: String,
        /// The body text the author wrote that did not match the enum.
        value: String,
    },

    /// RFC §5.C B6-γ negative coverage: `<sce:backpressure>` element
    /// is required on `sce:kind="link"` declarations — the policy is
    /// load-bearing for the runtime crate's RX queue behavior under
    /// load. B6-α tolerated the missing element by parser-side
    /// defaulting to `drop`; γ promotes the absence to a hard error
    /// so authors must declare `drop` / `block` / `signal-event`
    /// intentionally rather than inheriting an implicit default.
    #[error(
        "link '{name}': missing required <sce:backpressure> child — `sce:kind=\"link\"` requires an explicit backpressure policy declaration per RFC §5.C; add a <sce:backpressure>drop|block|signal-event</sce:backpressure> child"
    )]
    LinkBackpressureUndeclared {
        /// Link document name (root `name=` attribute).
        name: String,
    },

    /// RFC §5.C B6-η OS-axis negative coverage: the declared
    /// `<sce:link-class>` cannot run on the deploy-resolved
    /// `platform.os`. RFC §5.C lines 838 names this code; the
    /// admissibility matrix lives in [`LinkClass::admits_os`] mirroring
    /// the table at RFC §5.C lines 765-771 strict-literal:
    /// `udp` / `tcp` admit `bare_metal | linux | qnx`; `serial` /
    /// `websocket` / `raw_eth` admit `bare_metal` only. Anything off
    /// the table fires this diagnostic. The `candidates` axis is the
    /// list of OS names the class admits — drives `Fix::ReplaceOneOf`
    /// repair surface so the author can either change the class
    /// (`<sce:link-class>` body) or the deployment target
    /// (deploy.yaml `machines.<id>.platform.os`).
    #[error(
        "link '{name}': link-class `{class}` cannot run on target OS `{target_os}` per RFC §5.C lines 765-771; the matrix admits `{class}` on {candidates:?} only — change either the <sce:link-class> body or the deploy.yaml `machines.<id>.platform.os` for the target machine"
    )]
    LinkClassUnsupportedOnTarget {
        /// Link document name (root `name=` attribute).
        name: String,
        /// The declared `<sce:link-class>` body (e.g. `serial`).
        class: String,
        /// The deploy-resolved `platform.os` for the target machine
        /// (e.g. `linux`).
        target_os: String,
        /// The list of OS names this class DOES admit, for `Fix::ReplaceOneOf`.
        candidates: Vec<String>,
    },

    /// RFC §5.C B6-α' cross-resolution: the `<sce:rx-pool>` or
    /// `<sce:tx-pool>` reference resolves to a buffer-pool whose
    /// `<sce:slot-size>` is smaller than the framer codec's
    /// recursive worst-case encoded byte count. The TX path
    /// `event extract -> framer.encode() -> pool slot -> driver.send()`
    /// (RFC §5.C lines 786-789) cannot honor zero-copy when the slot
    /// cannot hold a full encoded frame; the link would silently
    /// stage-copy on every TX, defeating the whole point of pinning
    /// pool to framer in the schema. Fires only via
    /// [`compile_forge_with_imports`] post-enrichment, when both the
    /// `<sce:rx-pool>`/`<sce:tx-pool>` ref and the `<sce:framer>` ref
    /// resolve to imported documents whose [`ImportContext`] carries
    /// `buffer_pool_slot_size` and `codec_max_bytes` respectively.
    /// Skipped silently when either side fails to enrich (matches the
    /// other cross-file diagnostics' tolerance for partial topologies).
    /// No `candidates` axis — the repair is to raise `<sce:slot-size>`
    /// on the bound pool or shrink the codec's worst-case body, both
    /// author choices; emitted as `Fix::None`.
    /// RFC §5.C lines 793-794 spec anchor (rx-pool/tx-pool inherit the
    /// §5.E pool model on both sides of the byte-stream).
    ///
    /// [`ImportContext`]: crate::forge::generator::ImportContext
    /// [`compile_forge_with_imports`]: crate::compile_forge_with_imports
    #[error(
        "link '{link_name}': {pool_side}-pool '{pool_alias}' slot-size {pool_slot_size} bytes is smaller than framer '{framer_alias}' worst-case encoded size {framer_max_bytes} bytes — raise <sce:slot-size> on the bound pool or shrink the codec's worst-case body"
    )]
    LinkPoolSlotSmallerThanFramerMax {
        /// Link document name (root `name=` attribute).
        link_name: String,
        /// `"rx"` or `"tx"` — which `<sce:*-pool>` reference triggered.
        pool_side: &'static str,
        /// Imported buffer-pool's alias / document name (matches the
        /// `<sce:rx-pool ref>` / `<sce:tx-pool ref>` value).
        pool_alias: String,
        /// `<sce:slot-size>` body of the resolved buffer-pool.
        pool_slot_size: u32,
        /// Imported codec's alias / document name (matches the
        /// `<sce:framer ref>` value).
        framer_alias: String,
        /// Recursive worst-case encoded byte count of the resolved
        /// codec (matches `ImportContext::codec_max_bytes`, which
        /// already folds variant arm bodies / repeat / TLV chain
        /// per RFC §5.B).
        framer_max_bytes: u32,
    },

    /// RFC §5.E B7-α buffer-pool placement validation: the declared
    /// `<sce:section>` is not in the resolved machine's
    /// `memory.sram_regions` map. Fires only via
    /// [`compile_forge_with_deploy`] when both `deploy` and
    /// `target_machine` are present (Q-η5 (a) precedent — skip silently
    /// when deploy.yaml is unavailable). The `candidates` axis is the
    /// list of region names the resolved machine declares — drives
    /// `Fix::ReplaceOneOf` so the author can either rename the pool's
    /// `<sce:section>` body or extend deploy.yaml `memory.sram_regions`.
    /// RFC §5.E lines 1000-1023 + 1537 spec anchor.
    #[error(
        "buffer-pool '{name}': section `{section}` is not declared in deploy.yaml `machines.{machine}.memory.sram_regions` — extend the memory map or rename the pool's <sce:section> body to one of {candidates:?}"
    )]
    BufferPoolSectionConflict {
        /// Buffer-pool document name (root `name=` attribute).
        name: String,
        /// Target machine name (deploy.yaml top-level key).
        machine: String,
        /// The declared `<sce:section>` body (e.g. `sram1`).
        section: String,
        /// The list of section names the resolved machine declares, for `Fix::ReplaceOneOf`.
        candidates: Vec<String>,
    },

    /// RFC §5.E B7-β buffer-pool size validation: the pool's storage
    /// footprint (`slot_count × slot_size`) does not fit inside the
    /// resolved SRAM region's `size` field. Fires only via
    /// [`compile_forge_with_deploy`] when section validation already
    /// passed (Q-η5 (a) precedent: skip silently when deploy.yaml is
    /// unavailable; `mem/pool-section-conflict` is the prerequisite
    /// gate). No `candidates` axis — the repair is to raise the
    /// region size in deploy.yaml or shrink `slot_count`/`slot_size`;
    /// emitted as `Fix::None` because both axes are author choices.
    /// RFC §5.E lines 1031-1086 spec anchor (linker-fragment-side
    /// SECTIONS{} entry constrains the same byte budget).
    #[error(
        "buffer-pool '{name}': storage footprint {bytes_required} bytes ({slot_count} × {slot_size}) does not fit in deploy.yaml `machines.{machine}.memory.sram_regions.{section}` of size {region_size} bytes — raise the region size or shrink slot-count/slot-size"
    )]
    BufferPoolTooLarge {
        /// Buffer-pool document name (root `name=` attribute).
        name: String,
        /// Target machine name (deploy.yaml top-level key).
        machine: String,
        /// The declared `<sce:section>` body (e.g. `sram1`).
        section: String,
        /// `<sce:slot-count>` body.
        slot_count: u32,
        /// `<sce:slot-size>` body.
        slot_size: u32,
        /// Computed `slot_count × slot_size` in bytes.
        bytes_required: u64,
        /// Declared region size in bytes (deploy.yaml `size` field).
        region_size: u64,
    },

    /// RFC §5.E B7-β codegen self-check: the rendered linker fragment
    /// is missing the explicit `. = ALIGN(<n>);` inter-pool sentinel
    /// (§5.E lines 1059-1064). This is a codegen invariant violation,
    /// not an authoring mistake — fires only when the template itself
    /// drops the sentinel. The artifact is what makes the inter-pool
    /// boundary diff-visible (any PR that drops it shows up in the
    /// linker fragment) and what protects the post-pool boundary from
    /// master-script INCLUDE re-ordering. RFC §5.E lines 1059-1064 +
    /// 1537 spec anchor.
    #[error(
        "buffer-pool '{name}': linker fragment is missing the inter-pool `. = ALIGN(N);` sentinel — codegen invariant violation per RFC §5.E lines 1059-1064; report at https://github.com/newmassrael/scxml-core-engine/issues"
    )]
    BufferPoolInterPoolPaddingNotEmitted {
        /// Buffer-pool document name (root `name=` attribute).
        name: String,
    },

    /// watching-zenoh RFC §5.E C5 cache-maintenance validation
    /// (spec line 1544): pool `alignment` is smaller than the resolved
    /// target's `platform.dcache_line_size` while `cache-policy:
    /// maintain` is in effect. The cache-line alignment violation
    /// matters because partial-line `cache_invalidate_by_addr` calls
    /// corrupt adjacent data on the start side — the unaligned head
    /// crosses into the previous slot's last cache line, which the
    /// invalidate then evicts together with the slot's own bytes.
    /// Fires only via [`compile_forge_with_deploy`] after section
    /// validation passes (Q-η5 (a) silent-skip when deploy.yaml is
    /// unavailable). Author resolution: raise `<sce:alignment>` to at
    /// least the platform's `dcache_line_size`. RFC §5.E line 1544 +
    /// §5.I line 1742-1744 spec anchor.
    #[error(
        "buffer-pool '{name}': alignment {pool_alignment} is smaller than target platform's `dcache_line_size` {dcache_line_size} on machine '{machine}' under `cache-policy: maintain`. Partial-line cache_invalidate_by_addr corrupts adjacent slot data on the start side. Raise <sce:alignment> to at least {dcache_line_size}."
    )]
    BufferPoolCacheLineAlignment {
        /// Buffer-pool document name (root `name=` attribute).
        name: String,
        /// Target machine name (deploy.yaml top-level key).
        machine: String,
        /// `<sce:alignment>` body as authored.
        pool_alignment: u32,
        /// `platform.dcache_line_size` from deploy.yaml.
        dcache_line_size: u32,
    },

    /// watching-zenoh RFC §5.E C5 cache-maintenance validation
    /// (spec line 1545): `<sce:slot-size>` is not a whole-number
    /// multiple of `platform.dcache_line_size` while `cache-policy:
    /// maintain` is in effect. Each slot must occupy a whole number
    /// of cache lines so that `cache_invalidate_by_addr(slot, len)`
    /// after RX cannot touch the bytes of the adjacent slot that
    /// share the boundary cache line. Fires only via
    /// [`compile_forge_with_deploy`] after section validation passes
    /// (Q-η5 (a) silent-skip when deploy.yaml is unavailable). Author
    /// resolution: round `slot_size` up to the next cache-line
    /// multiple and continue using the original logical size from
    /// within each slot. RFC §5.E line 1545 + §5.I line 1742-1744
    /// spec anchor.
    #[error(
        "buffer-pool '{name}': slot-size {slot_size} is not a whole-number multiple of target platform's `dcache_line_size` {dcache_line_size} on machine '{machine}' (remainder {remainder}) under `cache-policy: maintain`. The boundary cache line is shared with the adjacent slot — cache_invalidate_by_addr after RX would corrupt it. Round slot-size up to {next_multiple} (next cache-line multiple)."
    )]
    BufferPoolSlotSizeNotCacheLineMultiple {
        /// Buffer-pool document name (root `name=` attribute).
        name: String,
        /// Target machine name (deploy.yaml top-level key).
        machine: String,
        /// `<sce:slot-size>` body as authored.
        slot_size: u32,
        /// `platform.dcache_line_size` from deploy.yaml.
        dcache_line_size: u32,
        /// `slot_size % dcache_line_size` — the over-the-line excess.
        remainder: u32,
        /// `slot_size + (dcache_line_size - remainder)` — repair target.
        next_multiple: u32,
    },

    /// watching-zenoh RFC §5.E C5 cache-maintenance validation
    /// (spec line 1543): pool declares `cache-policy: maintain` or
    /// `cache-policy: non-cacheable` while the resolved target
    /// platform has `has_dcache: false`. The maintenance call sites
    /// would be no-ops at best, MPU configuration request at worst —
    /// neither is meaningful on a core without a data cache. Fires
    /// only via [`compile_forge_with_deploy`] after section
    /// validation passes (Q-η5 (a) silent-skip when deploy.yaml is
    /// unavailable). Author resolution: switch the pool to
    /// `cache-policy: none`. RFC §5.E line 1543 spec anchor.
    #[error(
        "buffer-pool '{name}': `cache-policy: {declared_policy}` declared on machine '{machine}' which has `platform.has_dcache: false`. Cache maintenance is meaningless on a core without a data cache. Switch to `cache-policy: none`."
    )]
    BufferPoolCachePolicyUnsupportedOnNoDcacheCore {
        /// Buffer-pool document name (root `name=` attribute).
        name: String,
        /// Target machine name (deploy.yaml top-level key).
        machine: String,
        /// The declared `<sce:cache-policy>` body — `maintain` or
        /// `non-cacheable`. (`none` does not trigger.)
        declared_policy: String,
    },

    /// watching-zenoh RFC §5.E C5 cache-maintenance + §5.I author-
    /// guard (spec line 1548): an `<sce:extern>` declaration in the
    /// build attempts to author one of the cache-maintenance trio
    /// (`sce_dcache_clean_by_addr`, `sce_dcache_invalidate_by_addr`,
    /// `sce_dcache_clean_invalidate_by_addr`). Per spec lines
    /// 1222-1227, cache maintenance is **FSM-driven**: codegen
    /// auto-injects the externs and emits the calls on the buffer-
    /// pool lifecycle edges. Author authoring would silently allow
    /// duplicate declarations and the class of bugs ("the maintenance
    /// call sits in the wrong place") that the FSM-driven design
    /// prevents. Fires at parse time, before atomic A's whitelist
    /// validator. Author resolution: remove the offending
    /// `<sce:extern>`; the buffer-pool kind handles cache calls
    /// automatically when `cache-policy: maintain`. RFC §5.E line
    /// 1548 + lines 1222-1227 spec anchor.
    #[error(
        "<sce:extern name=\"{attempted_symbol}\">: cache-maintenance intrinsics are FSM-driven and authored automatically by the buffer-pool kind under `cache-policy: maintain` (RFC §5.E lines 1222-1227). Author <sce:extern> for the cache trio is forbidden — remove the declaration; codegen emits the calls on lifecycle edges."
    )]
    PoolCacheMaintenanceMisplaced {
        /// The cache trio symbol the author tried to declare.
        attempted_symbol: String,
    },

    /// watching-zenoh RFC §5.E C5 cache-maintenance config-
    /// completeness diagnostic (spec line 1553): a target machine
    /// declares `platform.has_dcache: true` without setting
    /// `platform.has_speculative_prefetch`. Codegen cannot decide
    /// whether to emit the `free → dma-armed-rx` pre-arm cache-
    /// invalidate edge — silently emitting it on M0/M3/M4 wastes
    /// cycles, silently omitting it on M7+/A-class cores leads to
    /// documented packet corruption (RFC §5.E lines 1199-1212).
    /// Fires only via [`compile_forge_with_deploy`] when at least
    /// one buffer-pool with `cache-policy: maintain` exists in the
    /// build (silent skip when no maintain-policy pool is reachable
    /// — the field has no consumer to require it). Author
    /// resolution: declare `has_speculative_prefetch` per the SoC
    /// datasheet (M7+/A-class = true, M3/M4 = false). RFC §5.E
    /// line 1553 spec anchor.
    #[error(
        "machine '{machine}': `platform.has_dcache: true` is set but `platform.has_speculative_prefetch` is not. Buffer-pool '{pool_name}' uses `cache-policy: maintain` and codegen cannot decide whether to emit the pre-DMA-RX invalidate edge. Declare `has_speculative_prefetch` per the SoC datasheet (M7+/A-class = true, M3/M4 = false)."
    )]
    PoolSpeculativePrefetchFlagMissing {
        /// Target machine name (deploy.yaml top-level key) whose
        /// platform block lacks the field.
        machine: String,
        /// Name of one buffer-pool that triggered the requirement
        /// — surfaces a concrete consumer in the message so the
        /// author can localize "why does my deploy.yaml suddenly
        /// require this field?".
        pool_name: String,
    },

    /// watching-zenoh RFC §5.E C5 cache-maintenance codegen self-
    /// check (spec line 1552): `cache-policy: maintain` +
    /// `platform.has_speculative_prefetch: true` resolved, but the
    /// rendered buffer-pool template did not emit a
    /// `sce_dcache_invalidate_by_addr` call inside the
    /// `link_arm_rx` body. Codegen-invariant violation — fires only
    /// when the `tools/codegen/templates/forge/{rust,c}/buffer_pool`
    /// template itself drops the pre-arm invalidate edge. The
    /// diagnostic guards against template regression that would
    /// silently corrupt RX data on M7+ cores. Authors cannot fix
    /// this from the SCXML side; the prose links to the issue
    /// tracker so a regression report finds the right team. RFC
    /// §5.E line 1552 spec anchor.
    #[error(
        "buffer-pool '{name}': generated source for backend `{backend}` is missing the `sce_dcache_invalidate_by_addr` call on the `free → dma-armed-rx` edge despite `cache-policy: maintain` + `platform.has_speculative_prefetch: true` — codegen invariant violation per RFC §5.E lines 1186-1198 + 1552; report at https://github.com/newmassrael/scxml-core-engine/issues"
    )]
    PoolCachePreArmInvalidateMissingOnSpeculativeCore {
        /// Buffer-pool document name (root `name=` attribute).
        name: String,
        /// Backend label whose template skipped the call (e.g.
        /// "rust", "c11"). Surfaces which template needs repair
        /// in the regression report.
        backend: String,
    },

    /// RFC §5.E B7-ε codegen self-check: the rendered C11 buffer-pool
    /// header is missing the `#include <sce/sample.h>` directive. The
    /// generated pool header surfaces the runtime Sample API
    /// (typestate-tracked `sce_sample_t` + Layer 1 attribute family) by
    /// pulling in `sce-c-runtime/include/sce/sample.h`; without the
    /// include, downstream consumers building against the pool header
    /// silently lose Layer 1 typestate coverage even on Clang ≥ 9
    /// because the macro family is unreachable. The diagnostic fires
    /// only when the template itself drops the include — it is a
    /// codegen invariant, not an authoring mistake. RFC §5.E lines
    /// 1276-1346 + 1520-1525 spec anchors.
    #[error(
        "buffer-pool '{name}': generated C11 header is missing the `#include <sce/sample.h>` directive — Layer 1 typestate attributes will be unavailable on consumer builds, codegen invariant violation per RFC §5.E lines 1276-1346; report at https://github.com/newmassrael/scxml-core-engine/issues"
    )]
    BufferPoolSampleTypestateAttributesDisabled {
        /// Buffer-pool document name (root `name=` attribute).
        name: String,
    },

    /// watching-zenoh RFC §5.E B7-η' Q-OnSample-2 (a): a `<sce:on-sample>`
    /// element appears outside a `<state>` or `<parallel>` parent.
    /// Q-OnSample-1 (Y) parser-AST extension means the validator can
    /// see the actual parent at parse time and quote the offending
    /// XML path so authors do not have to guess. Reachable via
    /// document tree walk (the well-formed-placement parser collects
    /// children of `<state>` / `<parallel>` only — strays remain
    /// unparented in the AST and are surfaced here).
    #[error("<sce:on-sample> at {path}: must appear directly inside a <state> or <parallel>; found inside <{actual_parent}>. Move the element under a state or parallel ancestor.")]
    OnSampleInvalidParent {
        /// XML path describing where the stray element was found,
        /// e.g. "scxml > onentry" or "scxml > final > onentry". The
        /// path is descriptive prose, not a machine-parseable
        /// XPath, because authoring-time tooling reads the message.
        path: String,
        /// Tag name of the immediate parent (`scxml`, `onentry`,
        /// `final`, etc.) so the diagnostic surfaces the boundary
        /// without forcing the author to expand the path.
        actual_parent: String,
    },

    /// watching-zenoh RFC §5.E B7-η' Q-OnSample-5 (a): two or more
    /// `<sce:on-sample>` blocks in the same state declare the same
    /// `link=`. Multiple blocks per state are explicitly allowed
    /// (fan-in across links) but each link must appear at most once
    /// per state — duplicate registrations would compete for the
    /// same RX callback slot at runtime, producing undefined
    /// dispatch order.
    #[error("state '{state_id}': duplicate <sce:on-sample link=\"{link}\"> declarations. Each link is allowed at most one on-sample block per state; merge the duplicates or rename one of the link references.")]
    OnSampleLinkDuplicateInState {
        /// State id whose body contains the duplicates.
        state_id: String,
        /// Link name that appears more than once.
        link: String,
    },

    /// watching-zenoh RFC §5.E B7-η' Q-OnSample-7: a `<sce:on-sample>`
    /// declares an `event=` whose name collides with a built-in W3C
    /// SCXML event prefix (`error.*`, `done.*`). The W3C SCXML §5.10
    /// internal event family carries fixed semantics — letting an
    /// author overload `done.state.foo` (raised when state foo
    /// reaches `<final>`) by an on-sample dispatch would silently
    /// crosstalk completions with sample arrivals. Author safety net.
    #[error("<sce:on-sample event=\"{event}\"> collides with the reserved W3C SCXML internal event prefix '{reserved_prefix}'. Pick an event name outside that family (e.g. 'sample.{event}') so dispatched samples stay distinct from built-in lifecycle events.")]
    OnSampleEventNameConflict {
        /// Event name as authored.
        event: String,
        /// Reserved prefix that the event name collides with
        /// (`error.` or `done.`).
        reserved_prefix: String,
    },

    /// watching-zenoh RFC §5.E B7-η' Atomic B Q-OnSample-3 cross-ref:
    /// a `<sce:on-sample link="X">` reference points at a name that
    /// no `.forge` file in the build declares as a link kind. The
    /// `Fix::ReplaceOneOf` candidate list is sourced from the
    /// build's `SceCrossDocRegistry` (sorted) so authors see legal
    /// alternatives without scraping the message body. An empty
    /// `candidates` list means no link kind is declared anywhere in
    /// the build — likely a missing `.forge` file rather than a
    /// typo. State id surfaces in the message for source navigation.
    #[error("state '{state_id}': <sce:on-sample link=\"{link}\"> references a name that no `.forge` file in the build declares as a link kind. Add a forge `<scxml sce:kind=\"link\" name=\"{link}\">` document or fix the reference. See watching-zenoh RFC §5.E.")]
    OnSampleLinkNotDeclared {
        /// State id whose `<sce:on-sample>` carries the unresolved
        /// reference.
        state_id: String,
        /// Link name as authored.
        link: String,
        /// Sorted list of every link kind name registered in the
        /// build. Drives `Fix::ReplaceOneOf` so authors can repoint
        /// the reference to one of the legal alternatives. Empty
        /// when no link kind has been registered.
        candidates: Vec<String>,
    },

    /// watching-zenoh RFC §5.E B7-η' Atomic B Q-OnSample-3 cross-ref:
    /// a `<sce:on-sample link="X">` reference resolves to a forge
    /// artifact that exists but is not a link kind. Today only link
    /// kind documents satisfy the on-sample subscriber contract;
    /// algorithm / codec / buffer-pool / etc. kinds cannot back a
    /// callback registration because they have no RX path. The
    /// repair is to point the reference at one of the build's
    /// actual link kind names.
    ///
    /// Production reachability: forward-compat. The single-variant
    /// `ScxmlDocKind` registry today only stores Link kinds, so
    /// the validator's match never reaches the `Some(non-Link)`
    /// arm. Wired through the full 11-place sync (enum,
    /// `ALL_DIAGNOSTIC_CODES`, schema, acceptance, golden, payload)
    /// so a future cross-registry generalization (or new
    /// `ScxmlDocKind` variant) can fire it without re-plumbing.
    #[error("state '{state_id}': <sce:on-sample link=\"{link}\"> resolves to a forge '{actual_kind}' kind, not 'link'. Only link kind documents back the on-sample subscriber contract. Repoint the reference at one of the build's link kind names. See watching-zenoh RFC §5.E.")]
    OnSampleLinkWrongKind {
        /// State id whose `<sce:on-sample>` carries the
        /// wrongly-kinded reference.
        state_id: String,
        /// Link name as authored.
        link: String,
        /// Forge kind label of the resolved artifact (e.g.
        /// "buffer-pool", "codec"). Slash-path-free wire form to
        /// match the spec-line-1515 family of `Fix::ReplaceOneOf`
        /// diagnostics.
        actual_kind: String,
        /// Sorted list of every link kind name registered in the
        /// build. Drives `Fix::ReplaceOneOf` so authors can repoint
        /// at a legal link kind.
        candidates: Vec<String>,
    },

    /// watching-zenoh RFC §5.E B7-η' Atomic A1 application-layer
    /// ownership diagnostic (spec lines 1513-1515): a state declares
    /// `<sce:on-sample link="X">` and link `X` is registered, but the
    /// link's forge document does not declare a `<sce:stage-pool>`
    /// element. Without a stage pool the generated `Sample::take()`
    /// has no destination to copy into and the link's
    /// `LinkConfig::stage_copy_hook` falls back to `PanicOnTakeHook`
    /// (sce-link-runtime default) — silently pushing the failure to
    /// runtime callbacks. The diagnostic surfaces the gap at codegen
    /// time so authors decide consciously: either add
    /// `<sce:stage-pool ref="...">` to the link kind, or accept that
    /// callbacks on this link must be borrow-only (no `.take()`
    /// across the callback boundary).
    ///
    /// Schema locality choice (Atomic A1 vs prior interpretation): the
    /// stage pool is a *link* property, co-located with rx_pool /
    /// tx_pool on the `<scxml sce:kind="link">` document, not a
    /// deploy-yaml binding property. The B7-η' Q-StagePool field on
    /// `BindingConfig.stage_pool` (already landed) becomes a
    /// deploy-time override mechanism — orthogonal to this diagnostic.
    #[error("state '{state_id}': <sce:on-sample link=\"{link}\"> targets a link kind whose forge document does not declare a `<sce:stage-pool>` element. Subscriber callbacks on this link cannot escape the borrow lifetime via `Sample::take()` because there is no stage-copy destination. Add `<sce:stage-pool ref=\"...\">` to the link's `.forge` document or restrict callbacks to borrow-only access. See watching-zenoh RFC §5.E.")]
    PoolSampleTakeWithoutStagePool {
        /// State id whose `<sce:on-sample>` triggers the gap.
        state_id: String,
        /// Link name as authored. Cross-references the link kind
        /// document that lacks a `<sce:stage-pool>`.
        link: String,
        /// Sorted list of every buffer-pool kind name registered in
        /// the build. Drives `Fix::ReplaceOneOf` so authors picking
        /// a `<sce:stage-pool ref="...">` value see legal pool names
        /// at hand. Empty when no buffer-pool kind has been declared
        /// anywhere in the build (then the fix is to add a
        /// `<scxml sce:kind="buffer-pool">` document first).
        candidates: Vec<String>,
    },

    /// watching-zenoh RFC §5.E B7-η' Atomic A2 application-layer
    /// ownership diagnostic (spec lines 1516-1519): an
    /// `<sce:on-sample callback="rust:crate::path::fn">` attribute
    /// carries an authoring path that fails the Q-Callback-3 Rust
    /// path subset. Today's reachable arms are path-syntax failures
    /// (unknown language prefix, leading/trailing/double `::`,
    /// non-NCName segment, empty path); future signature inspection
    /// extends the same diagnostic code with shape-mismatch arms
    /// (owned-mode first parameter rejected at SCE-side parser when
    /// β-extension lands).
    ///
    /// Diagnostic name preserves spec wording verbatim
    /// (`feedback_spec_mirror_parity.md`); the `reason` field
    /// disambiguates the per-instance message so authors see the
    /// exact path-syntax mistake rather than generic
    /// "callback-signature-non-borrow" wording.
    #[error(
        "state '{state_id}': <sce:on-sample link=\"{link}\" callback=\"{callback}\"> {reason}. \
         The `callback` value must match `rust:crate::module::fn` (Q-Callback-3 Rust path \
         subset). The borrow-mode contract is enforced at the dispatch site; rustc rejects \
         owned-mode signatures at user-crate compile time. See watching-zenoh RFC §5.E."
    )]
    PoolSampleCallbackSignatureNonBorrow {
        /// State id whose `<sce:on-sample callback>` triggers the
        /// path-syntax violation.
        state_id: String,
        /// Link name as authored — surfaces in the message so a
        /// reader can locate the offending element without grepping
        /// for the callback string alone (which may not be unique).
        link: String,
        /// `callback` attribute value verbatim. Carried through to
        /// the wire format's `actual` field so consumers see exactly
        /// what was authored.
        callback: String,
        /// Path-classification result. Drives the message body's
        /// "reason" clause so authors see the specific mistake (vs
        /// generic "malformed callback").
        reason: CallbackPathReason,
    },

    /// watching-zenoh RFC §5.I `<sce:extern>` whitelist rejection
    /// (spec line 1847): `<sce:extern name="...">` references a
    /// symbol absent from the §5.I baseline registry. `candidates`
    /// rides `Fix::ReplaceOneOf` so authors see closest-match
    /// suggestions without paging through 101 baseline entries.
    /// Q-Call-4 (a) lock: parse-time rejection; closed-set membership
    /// follows the `LinkLinkClassUnknown` (B6-γ) precedent.
    #[error(
        "<sce:extern name=\"{name}\"> references a symbol that is not on the §5.I baseline whitelist. \
         Choose a registry-listed name (closest matches: {candidates_list}) or extend the whitelist via a target plugin (deploy.yaml `extern_symbols.target_plugin`)."
    )]
    ExternSymbolNotInWhitelist {
        /// Symbol name as authored — guaranteed absent from the
        /// registry.
        name: String,
        /// Closest baseline-name candidates, sorted by shared-prefix
        /// length. Bounded at 8 for wire-payload bound.
        candidates: Vec<String>,
        /// Joined `candidates` for the message body. Filled at
        /// raise-site so the user-visible string lists names without
        /// the consumer needing to format them itself.
        candidates_list: String,
    },

    /// watching-zenoh RFC §5.I `<sce:extern abi="...">` mismatch
    /// (spec line 1848): the authored ABI does not match the
    /// registry entry's canonical ABI. Closed two-element repair set
    /// `[c, rust]` rides `Fix::ReplaceOneOf`.
    #[error(
        "<sce:extern name=\"{name}\" abi=\"{actual}\"> uses a non-canonical ABI; the registry entry requires `abi=\"{expected}\"`. The accepted set is [\"c\", \"rust\"]."
    )]
    ExternAbiMismatch {
        /// Symbol name (registry-resolved).
        name: String,
        /// Registry's canonical ABI (`c` or `rust`).
        expected: String,
        /// What the author wrote.
        actual: String,
    },

    /// watching-zenoh RFC §5.I `<sce:extern sig="...">` mismatch
    /// (spec line 1849): the authored signature does not byte-match
    /// the registry entry's canonical signature. `Fix::Replace`
    /// carries the canonical sig.
    #[error(
        "<sce:extern name=\"{name}\" sig=\"{actual}\"> declares a signature that does not match the registry entry. Replace with `sig=\"{expected}\"`."
    )]
    ExternSignatureMismatch {
        /// Symbol name (registry-resolved).
        name: String,
        /// Registry's canonical signature.
        expected: String,
        /// What the author wrote.
        actual: String,
    },

    /// watching-zenoh RFC §5.I atomic-family ordering-suffix omission
    /// (spec line 1850): the authored `name` is an atomic-family base
    /// (`sce_atomic_load`, `sce_atomic_cas_weak`, …) without the
    /// required `_<ordering>_<width>` suffix. `Fix::ReplaceOneOf`
    /// carries the legal completions.
    #[error(
        "<sce:extern name=\"{base}\"> is an atomic-family base without an explicit ordering + width suffix. Pick one of: {candidates_list}."
    )]
    ExternOrderingUnspecified {
        /// Atomic-family base as authored
        /// (e.g. `sce_atomic_load`, `sce_atomic_fence`).
        base: String,
        /// Suffix-bearing legal completions
        /// (e.g. `sce_atomic_load_acquire_u32`, …). 10 entries for
        /// load/store/fetch_*; 15 for cas_*; 4 for fences.
        candidates: Vec<String>,
        /// Joined `candidates` for the message body.
        candidates_list: String,
    },

    /// watching-zenoh RFC §5.I target-plugin baseline-shadowing
    /// (spec line 1852 verbatim): a target plugin YAML
    /// (`extern_symbols.target_plugin: <path>`) declares a `name` that
    /// already appears in the §5.I baseline registry. Q-Call-6 (a)
    /// additive-composition lock: plugins extend, never override; a
    /// platform-specific impl plugs in via the registry entry's
    /// `crate` field on a differently-named symbol. Repair is
    /// non-algorithmic — the plugin author renames the conflicting
    /// entry to a non-baseline name; SCE cannot synthesize a
    /// candidate. `fix: None` per the wire contract.
    #[error(
        "target plugin {plugin_path} redefines core whitelist symbol `{name}`. Plugin entries extend the §5.I baseline registry but cannot override it (Q-Call-6 additive-composition lock). Rename the plugin entry to a name not already in the §5.I baseline; for a platform-specific impl, declare the entry under a vendor-prefixed name (e.g. `sce_hw_<symbol>`) and route through the registry entry's `crate` field."
    )]
    ExternTargetPluginSymbolConflict {
        /// Symbol name declared by both the plugin and the baseline.
        name: String,
        /// Plugin file path (deploy-relative or absolute) for source
        /// location surfacing in diagnostic.
        plugin_path: String,
    },

    /// watching-zenoh RFC §5.D line 911 — worker kind cannot reach
    /// other workers' state through any path other than its own inbox.
    /// C2-α implements the static recognition layers: layer 1 rejects
    /// `<sce:import kind="worker">` siblings inside a worker document
    /// (workers must not import other workers' kinds — encapsulation
    /// boundary); layer 2 rejects SCXML body data-refs whose namespace
    /// prefix names a foreign owner (not the worker's own name, not
    /// `_event` / `_data` / `_name` / `_iolocation`, not the declared
    /// `<sce:outbox ref="...">` target). Layer 3 — `<sce:extern>`
    /// non-inbox symbol use in the body — couples to C4 intrinsic
    /// registry composition and lands in a tracked follow-up atomic;
    /// spec line 911 phrasing "any non-inbox access" covers all three
    /// layers together.
    ///
    /// Per Q-C2-7 (a) lock 2026-05-10. Fires at parse time; the
    /// per-instance payload carries which layer detected the
    /// violation so the diagnostic message can name the exact path-
    /// syntax mistake. RFC §5.D line 911 spec anchor.
    #[error(
        "worker '{worker_name}': {reason}. \
         Workers must communicate with other workers only through their \
         own inbox (consume) and the recipient's inbox via <sce:outbox \
         ref=\"...\"> (produce); all other paths to another worker's \
         state are forbidden per RFC §5.D line 911 (\"any non-inbox \
         access to another worker's state\")."
    )]
    WorkerSharedMutableState {
        /// The worker document whose body / sibling imports triggered
        /// the diagnostic. Anchored at the offending DOM node by
        /// `located()` at the call site.
        worker_name: String,
        /// Layered classification — see [`WorkerSharedStateReason`].
        reason: WorkerSharedStateReason,
    },

    /// watching-zenoh RFC §5.D (C2-β cross-resolution). The worker's
    /// `<sce:link-rx ref="X">` names `X` that does not resolve to a
    /// `<sce:import as="X" kind="link">` declaration on this worker
    /// document. `validate_link_pool_framer_resolution` precedent: a
    /// worker driven by a link kind must declare the link via
    /// `<sce:import>` so cross-resolution within
    /// `compile_forge_with_imports` can confirm shape compatibility
    /// before codegen. Closed candidate list rides `Fix::ReplaceOneOf`
    /// with the sorted set of link-kind import aliases (η-precedent for
    /// closest-match suggestions). Non-spec diagnostic per Q-C2-2 (a):
    /// the spec example elides imports but SCE's per-doc compile path
    /// requires explicit cross-resolution.
    #[error(
        "worker '{worker_name}': <sce:link-rx ref=\"{ref_name}\"> references a name that is not imported as a link kind. \
         Declare the link via <sce:import as=\"{ref_name}\" src=\"...\" kind=\"link\"/> on this worker document, or replace the ref with one of the imported link-kind aliases (closest matches: {candidates_list})."
    )]
    WorkerLinkRxRefUnknown {
        /// The worker document whose `<sce:link-rx>` carries the
        /// unresolvable ref. Anchored at the `<sce:link-rx>` node by
        /// `located()` at the call site.
        worker_name: String,
        /// Offending `<sce:link-rx ref>` value as authored.
        ref_name: String,
        /// Sorted closed candidate set — every kind=link alias known
        /// to `parsed.imports` for this document. Wire payload's
        /// `Fix::ReplaceOneOf` consumes this verbatim.
        candidates: Vec<String>,
        /// Joined comma-space form of `candidates` for the message
        /// body (matches `ExternSymbolNotInWhitelist`'s shape so
        /// per-instance message rendering stays parity).
        candidates_list: String,
    },

    /// watching-zenoh RFC §5.I line 1757-1758 — `<sce:inbox>` declared
    /// without an `ordering` attribute. Spec phrasing labels this a
    /// "warning, codegen defaults to acquire/release"; SCE's error-only
    /// wire surface (no severity dimension yet) realizes the warning as
    /// a required-when-worker-exists error: the author must explicitly
    /// pick `ordering="acq_rel"` or `ordering="relaxed"`. The choice
    /// changes the emitted atomic operations on head/tail indices in
    /// both Rust + C11 codegen, so silent default is risk-prone on a
    /// cross-core multi-MCU target. Diagnostic name preserves spec
    /// wording verbatim per `feedback_spec_mirror_parity.md`.
    #[error(
        "worker '{worker_name}': <sce:inbox> declared without an `ordering` attribute. \
         Pick `ordering=\"acq_rel\"` (safe default; producer and consumer pair head/tail with acquire+release on every push/pop) or `ordering=\"relaxed\"` (single-core fast-path; cross-core placement raises `worker/inbox-ordering-relaxed-across-cores`). Spec §5.I line 1752-1758 mandates one of these two for every SPSC inbox."
    )]
    WorkerInboxOrderingUnspecified {
        /// The worker document whose `<sce:inbox>` lacks ordering.
        /// Anchored at the `<sce:inbox>` node by `located()`.
        worker_name: String,
    },

    /// watching-zenoh RFC §5.I line 1755-1756 — `<sce:inbox
    /// ordering="relaxed">` declared on a worker whose producer and
    /// consumer halves resolve to different cores via deploy.placement.
    /// Per spec, `relaxed` on cross-core shared state is "insufficient";
    /// head/tail indices need acquire/release pairing to guarantee
    /// happens-before ordering across the cache-coherency boundary.
    /// Codegen-invariant guard: silent-skip when deploy is absent
    /// (`ForgeCompileOptions.worker_placement` is `None`), fires only
    /// when explicit cross-core placement coexists with `relaxed`
    /// ordering. Diagnostic name preserves spec wording verbatim.
    #[error(
        "worker '{worker_name}': <sce:inbox ordering=\"relaxed\"> declared but deploy.placement pins producer on core {producer_core} and consumer on core {consumer_core}. \
         Cross-core SPSC inboxes require acquire/release pairing on head/tail (per spec §5.I lines 1752-1758). Replace with `ordering=\"acq_rel\"` or co-locate producer + consumer on the same core via deploy.placement."
    )]
    WorkerInboxOrderingRelaxedAcrossCores {
        /// The worker document whose inbox declared relaxed ordering
        /// against cross-core placement. Anchored at the `<sce:inbox>`
        /// node by `located()`.
        worker_name: String,
        /// Core index hosting the inbox producer (link-rx-driven path).
        producer_core: u32,
        /// Core index hosting the inbox consumer (the worker's own
        /// SCXML processing thread).
        consumer_core: u32,
    },

    /// watching-zenoh RFC §5.D line 912
    /// (`worker/scheduler-unsupported`) — a Worker doc reached
    /// [`crate::compile_forge_with_deploy`] but the resolved target
    /// machine does not list it under `machines.<m>.workers`. The
    /// cooperative scheduler tracks one tick slot per declared worker;
    /// an undeclared worker has no slot, so codegen would emit a
    /// worker the scheduler cannot account for. The deploy-side anchor
    /// for the slot-count sum check is
    /// [`crate::mesh::error::DeployError::SchedulerIncompatibleWithWorkerCount`]
    /// (spec §5.K line 2423); the forge-side anchor here fires on the
    /// per-doc miss.
    #[error(
        "worker '{worker_name}': not declared in deploy.yaml under \
         `machines.{machine}.workers`. watching-zenoh RFC §5.D line 912 \
         (`worker/scheduler-unsupported`) — the cooperative scheduler \
         tracks one tick slot per declared worker; an undeclared \
         worker has no slot. Repair: add `{worker_name}:` under \
         `machines.{machine}.workers:` in deploy.yaml, or remove the \
         Worker doc from the build."
    )]
    WorkerSchedulerUnsupported {
        /// Worker name from `<scxml sce:kind="worker" name="...">`.
        worker_name: String,
        /// Target machine that did not list the worker.
        machine: String,
    },

    /// watching-zenoh RFC §5.D C2 follow-up — `<sce:outbox ref="X">`
    /// names an owner segment (`X.split('.').next()`) that does not
    /// resolve to a recorded statechart or worker doc in the build's
    /// [`crate::forge::cross_doc_registry::SceCrossDocRegistry`]. Q-Outbox-3
    /// (b) admits both statechart and worker recipients per spec line
    /// 911 ("any non-inbox access" admits inbox access regardless of
    /// owner kind). Q-Outbox-8 (c) splits the failure axis: this code
    /// fires on owner-not-in-registry; [`Self::WorkerOutboxTargetWrongKind`]
    /// fires when the owner resolves but to an incompatible kind (e.g.
    /// link kind); [`Self::WorkerOutboxTargetSuffixInvalid`] fires on
    /// suffix !=  `inbox` per Q-Outbox-6 (a) strict-suffix lock.
    ///
    /// Closed candidate list rides `Fix::ReplaceOneOf` with the sorted
    /// union of statechart + worker doc names (each suffixed with
    /// `.inbox` so the candidate strings are drop-in replacements for
    /// the entire `ref` attribute). η-precedent:
    /// [`Self::WorkerLinkRxRefUnknown`] uses the same sorted-closed-set
    /// shape.
    #[error(
        "worker '{worker_name}': <sce:outbox ref=\"{outbox_value}\"> names owner '{owner}' which is not a registered statechart or worker. \
         Declare the recipient as a separate `.scxml` document in this build (statechart: `<scxml name=\"{owner}\">`; worker: `<scxml sce:kind=\"worker\" name=\"{owner}\">`), or replace the ref with one of the registered recipients: {candidates_list}."
    )]
    WorkerOutboxRefUnknown {
        /// The worker document whose `<sce:outbox>` carries the
        /// unresolvable ref. Anchored at the `<sce:outbox>` node by
        /// `located()` at the call site.
        worker_name: String,
        /// `<sce:outbox ref>` value as authored (full
        /// `<owner>.<suffix>` string).
        outbox_value: String,
        /// Owner segment extracted from `outbox_value` (the substring
        /// before the first `.`). Surfaced so the diagnostic message
        /// names the failing segment rather than burying it in the
        /// full ref string.
        owner: String,
        /// Sorted closed candidate set — every registered statechart +
        /// worker doc name, each suffixed with `.inbox` so each entry
        /// is a complete drop-in replacement for the offending `ref`
        /// attribute value. Wire payload's `Fix::ReplaceOneOf` consumes
        /// this verbatim.
        candidates: Vec<String>,
        /// Joined comma-space form of `candidates` for the message
        /// body (matches `WorkerLinkRxRefUnknown` shape).
        candidates_list: String,
    },

    /// watching-zenoh RFC §5.D C2 follow-up — `<sce:outbox ref="X">`
    /// names an owner that *does* resolve in the cross-doc registry but
    /// to a kind incompatible with the outbox contract (today: link
    /// kind, since `<sce:outbox>` can only target the heapless::spsc::Queue
    /// inbox primitive that statechart + worker docs both lower into).
    /// Distinct from [`Self::WorkerOutboxRefUnknown`] (owner not in
    /// registry at all) so authors get distinct repair guidance: a
    /// wrong-kind hit usually means the author confused a link import
    /// alias with a statechart name; an unknown hit usually means a
    /// typo or a missing file.
    ///
    /// Closed candidate list rides `Fix::ReplaceOneOf` with the same
    /// sorted union as [`Self::WorkerOutboxRefUnknown`] — valid
    /// statechart + worker `.inbox` targets.
    #[error(
        "worker '{worker_name}': <sce:outbox ref=\"{outbox_value}\"> names '{owner}' which is registered as a {actual_kind} kind, not a statechart or worker. \
         Outbox refs may only target statechart or worker inboxes (RFC §5.D line 911 \"any non-inbox access\" by negation admits inbox access on statechart + worker kinds). Replace with one of: {candidates_list}."
    )]
    WorkerOutboxTargetWrongKind {
        /// The worker document whose `<sce:outbox>` carries the
        /// wrong-kind ref. Anchored at the `<sce:outbox>` node by
        /// `located()` at the call site.
        worker_name: String,
        /// `<sce:outbox ref>` value as authored.
        outbox_value: String,
        /// Owner segment extracted from `outbox_value`.
        owner: String,
        /// Actual kind registered for `owner` in the cross-doc registry
        /// (e.g. `"link"` for a link import alias mistaken for a
        /// statechart). Slash-path label from
        /// [`crate::forge::cross_doc_registry::ScxmlDocKind::as_str`].
        actual_kind: String,
        /// Sorted closed candidate set — same union shape as
        /// [`Self::WorkerOutboxRefUnknown::candidates`].
        candidates: Vec<String>,
        /// Joined comma-space form of `candidates` for the message
        /// body.
        candidates_list: String,
    },

    /// watching-zenoh RFC §5.D C2 follow-up — `<sce:outbox ref="X">`
    /// declares a suffix !=  `inbox`, violating the Q-Outbox-6 (a)
    /// strict-suffix lock. Spec line 895 example writes
    /// `session_fsm.inbox` exactly; codegen contract for both
    /// statechart and worker recipients is "deliver to the
    /// heapless::spsc::Queue named `inbox`" (spec line 1998 codegen
    /// table), so other suffixes (`.event_loop`, `.inbx` typos, bare
    /// `<owner>` without dot, …) have no codegen contract today.
    ///
    /// Repair is deterministic: keep the authored owner, replace the
    /// suffix with the literal `inbox`. `Fix::ReplaceWith` carries
    /// `"{owner}.inbox"`. Single-value repair places this in the
    /// `NeutralOrDeterministic` non-overlap class.
    ///
    /// Suffix is checked before owner — even if the owner is also
    /// unresolvable, the syntactic repair surfaces first. One-error-
    /// at-a-time wire policy then surfaces the owner failure on the
    /// next build cycle after the suffix is fixed.
    #[error(
        "worker '{worker_name}': <sce:outbox ref=\"{outbox_value}\"> declares suffix '{suffix}' but the only legal suffix is 'inbox' (RFC §5.D line 895 example: `<owner>.inbox`; spec line 1998 codegen table fixes the recipient queue name to `inbox`). \
         Replace with `{owner}.inbox`."
    )]
    WorkerOutboxTargetSuffixInvalid {
        /// The worker document whose `<sce:outbox>` carries the
        /// suffix-invalid ref. Anchored at the `<sce:outbox>` node by
        /// `located()`.
        worker_name: String,
        /// `<sce:outbox ref>` value as authored.
        outbox_value: String,
        /// Owner segment as authored (substring before the first `.`,
        /// or the entire `outbox_value` if there is no `.`). Used to
        /// compose the deterministic `Fix::ReplaceWith` target.
        owner: String,
        /// Suffix as authored (substring after the first `.`, or empty
        /// when no `.` is present — both forms violate the strict
        /// `<owner>.inbox` requirement).
        suffix: String,
    },

    /// watching-zenoh RFC §5.D line 909
    /// (`timer/period-below-tick-rate`) — `<sce:period>` declared
    /// shorter than `scheduler.tick_period_us`. The cooperative
    /// scheduler cannot dispatch a timer faster than its tick rate.
    #[error(
        "timer '{timer_name}': <sce:period> = {period_us} us is shorter \
         than scheduler.tick_period_us = {tick_period_us} us on machine \
         '{machine}'. watching-zenoh RFC §5.D line 909 \
         (`timer/period-below-tick-rate`) — the cooperative scheduler \
         dispatches at most one timer per tick, so a period below the \
         tick rate would miss every other deadline. Repair: raise \
         `<sce:period>` to >= {tick_period_us}us, or lower \
         `scheduler.tick_period_us` (warning: lowering tick rate \
         increases scheduler overhead), or switch the target machine \
         to `scheduler.kind: tokio` / `rt` (preemptive)."
    )]
    TimerPeriodBelowTickRate {
        /// Timer name from `<scxml sce:kind="timer" name="...">`.
        timer_name: String,
        /// Target machine whose scheduler tick rate this period falls below.
        machine: String,
        /// Period declared in the source SCXML (microseconds).
        period_us: u64,
        /// Cooperative scheduler tick period (microseconds).
        tick_period_us: u32,
    },

    /// watching-zenoh RFC §5.L line 2559
    /// (`collection/ordering-sorted-requires-index-by`) — a
    /// `<sce:ordering>sorted-by(index-by)</sce:ordering>` declaration
    /// without an accompanying `<sce:index-by field="..."/>` element.
    /// Spec line 2559 fixes the SortedByIndex iteration order to the
    /// `index-by` field; without that field there is no comparator the
    /// codegen can lower. C6-α parse-time structure check.
    #[error(
        "bounded-collection '{collection_name}': <sce:ordering>sorted-by(index-by)</sce:ordering> declared without <sce:index-by field=\"...\"/>. \
         watching-zenoh RFC §5.L line 2559 fixes sorted iteration to the `index-by` field; without it the codegen has no comparator to lower. \
         Repair: add an `<sce:index-by field=\"FIELD\"/>` element naming a field of the element-type struct, or change `<sce:ordering>` to `insertion`."
    )]
    CollectionOrderingSortedRequiresIndexBy {
        /// Bounded-collection name from `<scxml sce:kind="bounded-collection" name="...">`.
        collection_name: String,
    },

    /// watching-zenoh RFC §5.L line 2655
    /// (`collection/overflow-policy-oldest-wins-requires-ordering-insertion`)
    /// — `<sce:on-overflow>oldest-wins</sce:on-overflow>` declared
    /// together with `<sce:ordering>sorted-by(index-by)</sce:ordering>`.
    /// Spec line 2655 lists this combination as the explicit anti-
    /// pattern: the `oldest-wins` policy presumes a temporal ordering
    /// (insertion timestamp) that `sorted-by` mode replaces with the
    /// `index-by` field comparator, so "oldest" has no defined meaning.
    /// C6-α parse-time structure check.
    #[error(
        "bounded-collection '{collection_name}': <sce:on-overflow>oldest-wins</sce:on-overflow> requires <sce:ordering>insertion</sce:ordering>, but ordering is `sorted-by(index-by)`. \
         watching-zenoh RFC §5.L line 2655 lists this combination as the explicit anti-pattern: `oldest-wins` presumes a temporal ordering that `sorted-by` replaces with the `index-by` field comparator. \
         Repair: change `<sce:ordering>` to `insertion` (keeps the oldest-wins policy), or change `<sce:on-overflow>` to `reject` / `diagnostic-event`."
    )]
    CollectionOverflowPolicyOldestWinsRequiresOrderingInsertion {
        /// Bounded-collection name from `<scxml sce:kind="bounded-collection" name="...">`.
        collection_name: String,
    },

    /// watching-zenoh RFC §5.M line 2944
    /// (`mem/reassembly-pool-variant-missing-max-fragments`) —
    /// `<sce:variant>reassembly</sce:variant>` declared on a buffer-pool
    /// without an accompanying `<sce:max-fragments-per-message>` sibling.
    /// Spec line 2688 fixes the per-slot fragment-index bitmap width to
    /// this value; without it codegen has no upper bound on the per-slot
    /// fragment-ID tracking. C9-α parse-time structure check.
    #[error(
        "buffer-pool '{pool_name}': <sce:variant>reassembly</sce:variant> declared without <sce:max-fragments-per-message>N</sce:max-fragments-per-message>. \
         watching-zenoh RFC §5.M line 2688 fixes the per-slot fragment-index bitmap width to this value; without it codegen has no upper bound on the per-slot fragment-ID tracking. \
         Repair: add an `<sce:max-fragments-per-message>N</sce:max-fragments-per-message>` element with a positive integer N derived from the wire framer's per-message maximum."
    )]
    MemReassemblyPoolVariantMissingMaxFragments {
        /// Buffer-pool name from `<scxml sce:kind="buffer-pool" name="...">`.
        pool_name: String,
    },

    /// watching-zenoh RFC §5.M line 2945
    /// (`mem/reassembly-pool-variant-missing-timeout`) —
    /// `<sce:variant>reassembly</sce:variant>` declared on a buffer-pool
    /// without an accompanying `<sce:reassembly-timeout-ms>` sibling.
    /// Spec line 2689 + line 2696 fix the per-slot deadline field to
    /// this value; without it the reassembly FSM has no
    /// `Receiving → TimedOut` edge timer (`docs/reassembly-fsm.md`
    /// §2.4.5). C9-α parse-time structure check.
    #[error(
        "buffer-pool '{pool_name}': <sce:variant>reassembly</sce:variant> declared without <sce:reassembly-timeout-ms>N</sce:reassembly-timeout-ms>. \
         watching-zenoh RFC §5.M line 2689 fixes the per-slot deadline field to this value; without it the reassembly FSM has no `Receiving → TimedOut` edge timer (`docs/reassembly-fsm.md` §2.4.5). \
         Repair: add an `<sce:reassembly-timeout-ms>N</sce:reassembly-timeout-ms>` element with a positive integer N (milliseconds) derived from link latency budget and acceptable hold time."
    )]
    MemReassemblyPoolVariantMissingTimeout {
        /// Buffer-pool name from `<scxml sce:kind="buffer-pool" name="...">`.
        pool_name: String,
    },

    /// watching-zenoh RFC §5.M line 2946
    /// (`mem/reassembly-slot-size-below-declared-mtu`) — an `<sce:rx-pool
    /// ref>` binding resolved to a buffer-pool whose `<sce:slot-size>`
    /// is smaller than the bound link's `mtu_bytes`. The slot cannot
    /// hold a single full-MTU datagram; even the non-fragmented happy
    /// path fails to admit one wire frame. C13-α-2 cross-doc consumer
    /// of `resolve_link_rx_pool_slot_count` (silent-skip on join
    /// failure).
    #[error(
        "buffer-pool '{pool_name}' is bound as RX pool for link '{link_name}' on machine '{machine}', but `<sce:slot-size>{slot_size}</sce:slot-size>` is smaller than the link's `mtu_bytes: {mtu_bytes}`. \
         watching-zenoh RFC §5.M line 2946 — the slot cannot admit a single full-MTU datagram, so even the non-fragmented happy path fails. \
         Repair: raise `<sce:slot-size>` on pool '{pool_name}' to >= {mtu_bytes}, lower `mtu_bytes` on link '{link_name}', or bind a different (larger) pool."
    )]
    MemReassemblySlotSizeBelowDeclaredMtu {
        /// Buffer-pool name from `<scxml sce:kind="buffer-pool" name="...">`.
        pool_name: String,
        /// Pool's declared `<sce:slot-size>` value.
        slot_size: u32,
        /// Link's declared `mtu_bytes` from deploy.yaml.
        mtu_bytes: u32,
        /// Deploy machine hosting the link binding.
        machine: String,
        /// Link name (joins deploy.yaml + forge `<sce:link>`).
        link_name: String,
    },

    /// watching-zenoh RFC §5.M line 2947-2949
    /// (`reassembly/max-fragments-insufficient-for-mtu`) — reassembly-
    /// variant pool's `<sce:slot-size>` cannot hold the worst-case
    /// reassembled message implied by `<sce:max-fragments-per-message>`
    /// and the bound link's `mtu_bytes`. Spec invariant verbatim:
    /// `slot_size >= max-fragments-per-message × mtu_bytes`. Hard
    /// error. C13-α-2 cross-doc consumer.
    #[error(
        "reassembly-variant buffer-pool '{pool_name}' is bound to link '{link_name}' on machine '{machine}', but `<sce:slot-size>{slot_size}</sce:slot-size>` cannot hold the worst-case reassembled message: `<sce:max-fragments-per-message>{max_fragments_per_message}</sce:max-fragments-per-message> × link.mtu_bytes ({mtu_bytes}) = {required}` bytes required. \
         watching-zenoh RFC §5.M line 2947-2949 verbatim: `slot_size >= max-fragments-per-message × mtu_bytes` — worst-case message must complete reassembly within declared bounds. \
         Repair: raise `<sce:slot-size>` on pool '{pool_name}' to >= {required}, lower `<sce:max-fragments-per-message>`, or lower link `mtu_bytes`."
    )]
    ReassemblyMaxFragmentsInsufficientForMtu {
        pool_name: String,
        slot_size: u32,
        max_fragments_per_message: u32,
        mtu_bytes: u32,
        required: u32,
        machine: String,
        link_name: String,
    },

    /// watching-zenoh RFC §5.M line 2950-2952
    /// (`reassembly/expected-fragmentation-rate-high`) — the bound
    /// link's `expected_p99_bytes` exceeds the regular RX pool's
    /// `<sce:slot-size>` such that more than 25% of inbound traffic
    /// would run the ARCHITECTURE §9.3 stage-copy path. Default
    /// warning per spec (suppressible via
    /// `<sce:accept-stage-copy-rate>` on the link source, gated by
    /// C13-γ). Silent-skip when no regular `BufferPoolVariant::Default`
    /// pool is bound (Q-C13-α2-4 (a) — the formula references "the
    /// regular RX pool's slot_size" which does not exist for the link).
    #[error(
        "link '{link_name}' on machine '{machine}': `expected_p99_bytes: {expected_p99_bytes}` exceeds RX pool '{pool_name}' `<sce:slot-size>{slot_size}</sce:slot-size>` by more than the 25% default stage-copy threshold (rate = {rate_percent}%). \
         watching-zenoh RFC §5.M line 2950-2952 — `(expected_p99_bytes - rx_pool.slot_size) / expected_p99_bytes > 0.25` triggers the warning. \
         Repair: raise `<sce:slot-size>` on pool '{pool_name}', lower `expected_p99_bytes` (with justification), or add `<sce:accept-stage-copy-rate>` on the link source (C13-γ scope)."
    )]
    ReassemblyExpectedFragmentationRateHigh {
        pool_name: String,
        slot_size: u32,
        expected_p99_bytes: u32,
        rate_percent: u32,
        machine: String,
        link_name: String,
    },

    /// watching-zenoh RFC §5.M line 2964-2969
    /// (`reassembly/untrusted-link-binding`) — reassembly-variant
    /// pool bound to a link with `trust_class: untrusted` or
    /// `session_arming`. Hard error: fragmentation on these links
    /// is forbidden; only `established_session` links may carry
    /// fragmented traffic. Defends against UDP source-IP spoofing
    /// exhausting per-peer quota space.
    #[error(
        "reassembly-variant buffer-pool '{pool_name}' is bound to link '{link_name}' on machine '{machine}', but the link declares `trust_class: {trust_class}`. \
         watching-zenoh RFC §5.M line 2964-2969 — only `trust_class: established_session` links may carry fragmented traffic; reassembly on `untrusted` / `session_arming` links exposes the per-peer quota space to source-IP spoofing. \
         Repair: change link '{link_name}' to `trust_class: established_session` (only if the link is in fact post-handshake), or remove the reassembly-pool binding."
    )]
    ReassemblyUntrustedLinkBinding {
        pool_name: String,
        trust_class: String,
        machine: String,
        link_name: String,
    },

    /// watching-zenoh RFC §5.M line 2970-2975
    /// (`reassembly/trust-class-missing-on-fragmenting-link`) —
    /// reassembly-variant pool bound to a link whose `domain_attrs`
    /// block is absent entirely (Q-C13-α2-8 (a) lock). Build cannot
    /// decide whether the binding is safe.
    #[error(
        "reassembly-variant buffer-pool '{pool_name}' is bound to link '{link_name}' on machine '{machine}', but the link does not declare `domain_attrs.trust_class`. \
         watching-zenoh RFC §5.M line 2970-2975 — build cannot decide whether the binding is safe without a declared trust class. \
         Repair: declare `domain_attrs: {{ trust_class: established_session }}` on link '{link_name}' (data-plane links), or remove the reassembly-pool binding (control-plane links)."
    )]
    ReassemblyTrustClassMissingOnFragmentingLink {
        pool_name: String,
        machine: String,
        link_name: String,
    },

    /// watching-zenoh RFC §5.M line 2995-2999
    /// (`reassembly/stage-copy-wcet-exceeds-slot-budget`) — the
    /// implicit memcpy in the stage-copy path alone blows the
    /// cooperative slot. Formula verbatim: `expected_p99_bytes ×
    /// memcpy_cycles_per_byte / clock_freq_mhz > worker_slot_budget_us`.
    /// Silent-skip when any of the four platform/scheduler inputs
    /// absent (per Q-η5 (a) precedent).
    #[error(
        "link '{link_name}' on machine '{machine}': stage-copy WCET ({stage_copy_wcet_us} µs) exceeds `scheduler.worker_slot_budget_us: {worker_slot_budget_us}`. \
         watching-zenoh RFC §5.M line 2995-2999 — `expected_p99_bytes ({expected_p99_bytes}) × memcpy_cycles_per_byte ({memcpy_cycles_per_byte}) / clock_freq_mhz ({clock_freq_mhz}) > worker_slot_budget_us`. \
         The stage copy alone starves Keepalive and parallel-region timers (ARCHITECTURE §9.3 + §3.4). \
         Repair: raise `worker_slot_budget_us` (and re-validate every algorithm), lower `expected_p99_bytes` so stage copy is never invoked at that size, or raise the bound pool's `<sce:slot-size>` to absorb p99 without invoking stage copy."
    )]
    ReassemblyStageCopyWcetExceedsSlotBudget {
        machine: String,
        link_name: String,
        expected_p99_bytes: u32,
        memcpy_cycles_per_byte: f32,
        clock_freq_mhz: u32,
        worker_slot_budget_us: u32,
        stage_copy_wcet_us: u32,
    },

    /// watching-zenoh RFC §5.L lines 2566-2567 +  2650
    /// (`collection/element-type-not-a-kind`) — `<sce:element-type>NAME`
    /// body text does not resolve in the build's forge-doc registry to
    /// a codec-kind struct (§5.B) or procedure-kind state record. C6-α
    /// stores the body text as an opaque `String`; C6-β consumes the
    /// orchestrator-assembled element-type candidate map
    /// (`HashMap<String, ForgeDocument>` populated only for codec +
    /// procedure docs during pass-1 of
    /// [`crate::compile_scxml_with_imports`]) and either fires this
    /// code (name absent from the map OR present but with an
    /// incompatible kind — both surface as the same code per Q-Outbox-3
    /// (b) `*RefUnknown` + `*WrongKind` precedent that split axes only
    /// when repair surfaces differ; here both axes share the closed
    /// candidate set so the single code suffices) or returns Ok.
    ///
    /// Closed candidate list rides `Fix::ReplaceOneOf` with the sorted
    /// union of registered codec + procedure doc names. η-precedent:
    /// [`Self::WorkerOutboxRefUnknown`] uses the same sorted-closed-set
    /// shape.
    #[error(
        "bounded-collection '{collection_name}': <sce:element-type>{element_type}</sce:element-type> does not name a codec-kind struct or procedure-kind state record in this build. \
         watching-zenoh RFC §5.L line 2566-2567 — element types must reference another forge kind by name (codec for byte-encoded structs, procedure for stateful records). \
         Declare the element type as a separate `.scxml` document (codec: `<scxml sce:kind=\"codec\" name=\"{element_type}\">`; procedure: `<scxml sce:kind=\"procedure\" name=\"{element_type}\">`), or replace the body text with one of the registered candidates: {candidates_list}."
    )]
    CollectionElementTypeNotAKind {
        /// Bounded-collection name from `<scxml sce:kind="bounded-collection" name="...">`.
        collection_name: String,
        /// `<sce:element-type>` body text as authored. Surfaced in the
        /// message + `actual` so the diagnostic names the failing
        /// reference verbatim.
        element_type: String,
        /// Sorted closed candidate set — every registered codec +
        /// procedure doc name. Wire payload's `Fix::ReplaceOneOf`
        /// consumes this verbatim.
        candidates: Vec<String>,
        /// Joined comma-space form of `candidates` for the message
        /// body (matches `WorkerOutboxRefUnknown` shape).
        candidates_list: String,
    },

    /// watching-zenoh RFC §5.L line 2615 + 2651
    /// (`collection/index-by-field-missing`) — `<sce:index-by field="X"/>`
    /// names a field that does not exist on the resolved element-type
    /// struct. Fires only when [`Self::CollectionElementTypeNotAKind`]
    /// would not also fire (suffix-then-owner pattern: element-type
    /// resolution succeeds first, then field enumeration runs against
    /// the resolved kind's field set).
    ///
    /// Field enumeration mirrors the codec + procedure arms of
    /// [`crate::discover_stateful_member_fields`] — codec exposes
    /// `CodecModel.fields[].id`; procedure exposes
    /// `ProcedureModel.inputs[].id + internals[].id`.
    ///
    /// Closed candidate list rides `Fix::ReplaceOneOf` with the sorted
    /// list of field names from the resolved element-type kind.
    #[error(
        "bounded-collection '{collection_name}': <sce:index-by field=\"{field}\"/> names a field that does not exist on element-type '{element_type}' ({element_kind} kind). \
         watching-zenoh RFC §5.L line 2615 — the `index-by` field enables `find_by_index(IndexKey)` and must name an actual struct field of the element type. \
         Replace `field=\"{field}\"` with one of the {element_type}'s declared fields: {candidates_list}."
    )]
    CollectionIndexByFieldMissing {
        /// Bounded-collection name from `<scxml sce:kind="bounded-collection" name="...">`.
        collection_name: String,
        /// `<sce:index-by field>` value as authored.
        field: String,
        /// Element-type name (already verified to resolve in the
        /// forge-doc registry to a codec or procedure kind — this
        /// code runs after element-type-not-a-kind passes).
        element_type: String,
        /// Slash-path label for the resolved element-type kind
        /// (`codec` or `procedure`). Surfaced in the message so the
        /// author sees which field surface applies.
        element_kind: String,
        /// Sorted closed candidate set — every declared field of the
        /// resolved element-type kind.
        candidates: Vec<String>,
        /// Joined comma-space form of `candidates` for the message
        /// body.
        candidates_list: String,
    },

    /// watching-zenoh RFC §5.L lines 2560-2562 + 2652
    /// (`collection/multi-writer-without-atomics`) —
    /// `<sce:concurrency>multi-writer</sce:concurrency>` declared on a
    /// bounded-collection without any §5.I atomic intrinsic having been
    /// imported via `<sce:extern>` anywhere in the build. The spec
    /// fixes multi-writer to "acquire/release atomics on head/tail",
    /// so the build's `<sce:extern>` trust-surface must acknowledge
    /// atomic intrinsics for codegen to legitimately emit them.
    ///
    /// Check is build-wide cross-doc per user direction (Gate B Q2
    /// `C6-β 에 포함, 빌드 단위 cross-doc 검사`): pass-1 of
    /// [`crate::compile_scxml_with_imports`] aggregates every parsed
    /// forge doc's `extern_declarations` into a single slice; the
    /// validator scans for any entry whose registry-resolved purpose
    /// starts with `"atomic-"` (the C4 atomic A baseline registry
    /// tags atomic-load / atomic-store / atomic-cas-* / atomic-fetch-*
    /// uniformly via the [`crate::forge::intrinsic_registry::Symbol::purpose`]
    /// field). At least one such declaration anywhere in the build
    /// allows multi-writer; zero declarations fires this code.
    ///
    /// No closed candidate set — the C4 baseline registry's atomic
    /// family is too large (≥101 spans load/store/cas/fetch ×
    /// 5 widths × multiple orderings) for a useful
    /// `Fix::ReplaceOneOf`; author judgment chooses the right ordering
    /// + width. NeutralOrDeterministic non_overlap_class with
    /// `fix: None`.
    #[error(
        "bounded-collection '{collection_name}': <sce:concurrency>multi-writer</sce:concurrency> requires at least one §5.I atomic intrinsic to be declared via <sce:extern> somewhere in this build. \
         watching-zenoh RFC §5.L lines 2560-2562 — multi-writer codegen lowers to acquire/release atomics on head/tail; the build's <sce:extern> trust-surface must acknowledge atomic intrinsics for codegen to emit them. \
         Repair: either declare an atomic intrinsic via <sce:extern> (e.g. `<sce:extern name=\"sce_atomic_load_acquire_u32\" sig=\"(*const u32) -> u32\" abi=\"c\"/>` in any forge doc in this build), or change `<sce:concurrency>` to `single-writer`."
    )]
    CollectionMultiWriterWithoutAtomics {
        /// Bounded-collection name from `<scxml sce:kind="bounded-collection" name="...">`.
        collection_name: String,
    },

    /// watching-zenoh RFC §5.L lines 2583-2585 + 2649
    /// (`collection/capacity-unresolved`) — `<sce:capacity source="deploy"
    /// key="machines.<machine>.limits.<limit>"/>` names a deploy-key
    /// whose `<limit>` segment is not declared under
    /// `machines.<machine>.limits:` in deploy.yaml. The codegen must
    /// lower the capacity into a per-language compile-time constant
    /// (spec line 2570-2585), so an unresolved key blocks emit.
    ///
    /// Fires only on the [`crate::compile_forge_with_deploy`] path
    /// (deploy + target_machine both Some) per the Q-η5 (a) silent-
    /// skip precedent: single-file compile paths cannot resolve
    /// deploy-key capacities because they don't know which machine
    /// will host the BC doc. Silent-skips also when the key's
    /// machine segment does not equal `target_machine` — the BC doc
    /// was designed for a different machine; deploy.yaml resolution
    /// runs only on the host machine's compile.
    ///
    /// Closed candidate list rides `Fix::ReplaceOneOf` with the
    /// sorted set of limit names declared under
    /// `machines.<machine>.limits:` (matches the
    /// `BufferPoolSectionConflict` precedent for sorted-declared-name
    /// candidate sets). FixCarriesCandidates non_overlap_class.
    #[error(
        "bounded-collection '{collection_name}': <sce:capacity source=\"deploy\" key=\"{key}\"/> references limit '{limit}' on machine '{machine}', but deploy.yaml does not declare `machines.{machine}.limits.{limit}`. \
         watching-zenoh RFC §5.L lines 2583-2585 — `<sce:capacity source=\"deploy\">` resolves at codegen time to a per-language compile-time constant from `machines.<machine>.limits.<limit>:`; an unresolved limit blocks emission. \
         Repair: declare `{limit}: <count>` under `machines.{machine}.limits:` in deploy.yaml (declared limits today: {candidates_list}), or switch the BC's `<sce:capacity>` to `const=\"N\"`."
    )]
    CollectionCapacityUnresolved {
        /// Bounded-collection name from `<scxml sce:kind="bounded-collection" name="...">`.
        collection_name: String,
        /// `<sce:capacity key>` value as authored (full dotted path,
        /// e.g. `machines.mcu_node.limits.local_subscriptions`).
        key: String,
        /// Target machine name (extracted from the key's middle
        /// segment; matches `compile_forge_with_deploy`'s
        /// `target_machine` param when the validator fires).
        machine: String,
        /// Limit name (final dotted segment) the author asked for.
        limit: String,
        /// Sorted closed candidate set — every declared limit name
        /// under `machines.<machine>.limits:`. Wire payload's
        /// `Fix::ReplaceOneOf` consumes this verbatim.
        candidates: Vec<String>,
        /// Joined comma-space form of `candidates` for the message
        /// body (matches the sibling cross-doc diagnostics' shape).
        candidates_list: String,
    },
}

/// watching-zenoh RFC §5.E B7-η' Atomic A2 callback-path failure
/// classification. Attached to
/// [`ValidationError::PoolSampleCallbackSignatureNonBorrow`] so the
/// per-instance message names the exact path-syntax mistake; the
/// outer code stays spec-verbatim
/// (`pool/sample-callback-signature-non-borrow`) per
/// `feedback_spec_mirror_parity.md`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CallbackPathReason {
    /// Empty `callback=""` attribute or empty body after the language
    /// prefix (`rust:` with nothing after). Authors typically arrive
    /// here by removing a path mid-edit and forgetting to delete the
    /// attribute itself.
    #[error("declares an empty callback path")]
    EmptyPath,
    /// Unknown or missing language prefix. Today the only legal
    /// prefix is `rust:` (Q-Callback-2 future axes are forward-compat
    /// schema slots). The empty `prefix` carries the missing-colon
    /// case; non-empty prefixes carry the unknown-prefix case.
    #[error("uses an unsupported language prefix `{prefix}` (only `rust:` is accepted today)")]
    UnknownLanguagePrefix {
        /// Prefix as authored, or `""` when no `:` separator was
        /// present in the original value.
        prefix: String,
    },
    /// Leading `::`, trailing `::`, or `::::` between two segments.
    /// Captured separately from `MalformedSegment` so the message
    /// can name the structural mistake rather than echoing a
    /// suspicious-looking empty token.
    #[error("contains a malformed `::` separator")]
    MalformedPath,
    /// A path segment failed the
    /// `(crate|self|super|<NCName-identifier>)` subset. The
    /// `segment` field carries the offending substring so the
    /// message can quote it for the author.
    #[error("contains a non-identifier segment `{segment}`")]
    MalformedSegment {
        /// Offending segment verbatim.
        segment: String,
    },
}

/// watching-zenoh RFC §5.D line 911 — worker shared-mutable-state
/// failure classification. Attached to
/// [`ValidationError::WorkerSharedMutableState`] so the outer code
/// stays spec-verbatim (`worker/shared-mutable-state`) per
/// `feedback_spec_mirror_parity.md` while each per-instance message
/// names the exact path that crossed the encapsulation boundary.
///
/// C2-α implements layers 1 + 2; layer 3 (C4-composition hardening
/// against `<sce:extern>` non-inbox symbol use in worker bodies)
/// lands in a tracked follow-up atomic per Q-C2-7 (a)+(b) lock.
/// Each future layer extends this enum with additional variants
/// without changing the outer code.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WorkerSharedStateReason {
    /// Layer 1: `<sce:import kind="worker">` sibling declared inside
    /// (or alongside) the worker document. Cross-worker direct imports
    /// would expose the imported worker's data model as a named alias
    /// (`<alias>.field`), which is exactly the non-inbox access path
    /// spec line 911 forbids. The empty `imported_src` value carries
    /// the case where the offending `<sce:import>` has no `src=` (a
    /// shape that fails earlier validation but reaches this enum on
    /// malformed authoring).
    #[error("declares <sce:import as=\"{imported_alias}\" src=\"{imported_src}\" kind=\"worker\"/>; workers cannot import other worker kinds")]
    WorkerImportForbidden {
        /// `<sce:import as="X">` value as authored — surfaces the
        /// alias name in the message so authors can locate the
        /// offending declaration without grepping the kind attribute.
        imported_alias: String,
        /// `<sce:import src="...">` value as authored.
        imported_src: String,
    },
    /// Layer 2: a body SCXML element carries a `location`, `target`,
    /// `id`, `expr`, or similar attribute whose value names a foreign
    /// owner — a namespace prefix not in the allowlist `[<self-name>,
    /// "_event", "_data", "_name", "_iolocation", <outbox-target>]`.
    /// The `foreign_prefix` captures the offending namespace owner so
    /// the message can quote what was crossed.
    #[error("body element <{element} {attr}=\"{value}\"/> reaches into namespace `{foreign_prefix}`, which is outside the inbox-only access surface")]
    BodyForeignNamespace {
        /// XML element local name where the foreign attribute appeared
        /// (`assign` / `send` / `data` / `param` etc.).
        element: String,
        /// Attribute name that carried the foreign reference
        /// (`location` / `target` / `id` / `expr` etc.).
        attr: String,
        /// Attribute value verbatim — surfaces the offending dotted
        /// expression so authors locate the exact ref.
        value: String,
        /// Foreign owner prefix extracted from the value (i.e. the
        /// text before the first `.`).
        foreign_prefix: String,
    },
}

// ── Stage 4: Expression transpilation ──────────────────────────

/// Errors from the `tokenize → parse → infer → emit` expression pipeline.
#[derive(Debug, thiserror::Error)]
pub enum ExprError {
    /// Empty input to `transpile_typed` or `transpile_lvalue`.
    #[error("empty {what}")]
    Empty { what: &'static str },

    /// Lexer failure: unterminated string, unexpected character, etc.
    #[error("at position {position}: {detail}")]
    Lex { position: usize, detail: String },

    /// Unsupported ECMAScript construct (arrow, nullish, spread, …).
    #[error("unsupported ECMAScript construct: {construct}. Extended SCXML expressions must use the stateless subset.")]
    UnsupportedConstruct { construct: String },

    /// Loose equality (`==` / `!=`) instead of strict.
    #[error("loose {operator} is not permitted in Extended SCXML. Use {strict} instead.")]
    StrictEquality {
        operator: &'static str,
        strict: &'static str,
    },

    /// Parser failure: unexpected or mismatched token.
    #[error("expected {expected}, got '{got}'")]
    ParseMismatch { expected: String, got: String },

    /// Parser failure: trailing or unexpected token.
    #[error("unexpected token: '{token}'")]
    UnexpectedToken { token: String },

    /// Assignment target is not a legal lvalue.
    #[error("assign location {location:?} is not an lvalue: {detail}")]
    InvalidLvalue { location: String, detail: String },

    /// Type coercion failure in a language emitter (Rust, Go).
    #[error("cannot coerce {lang} expression: {detail}")]
    TypeCoercion { lang: &'static str, detail: String },

    /// Target language cannot represent the expression construct.
    /// Currently: Go has no ternary expression.
    #[error("cannot transpile ternary expression to Go: Go has no conditional expression")]
    GoTernary,
}

// ── Stage 5: Cross-file import resolution ──────────────────────

/// Errors from `<sce:import>` validation and enrichment.
#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    /// The imported file does not exist at the resolved path.
    #[error("<sce:import src=\"{src}\">: file not found (searched: {searched})")]
    FileNotFound { src: String, searched: String },

    /// The declared kind on `<sce:import>` doesn't match the file's actual kind.
    #[error("<sce:import src=\"{src}\" kind=\"{declared}\">: actual kind is '{actual}' (mismatch)")]
    KindMismatch {
        src: String,
        declared: String,
        actual: String,
    },

    /// The imported file is not a forge document.
    #[error("<sce:import src=\"{src}\">: not a forge document (no sce:kind)")]
    NotForge { src: String },

    /// Cannot read the imported file.
    #[error("<sce:import src=\"{src}\">: cannot read: {source}")]
    ReadError {
        src: String,
        source: std::io::Error,
    },
}

// ── Stage 6: Dependency manifest ───────────────────────────────

/// Errors from manifest building and topological sort.
#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    /// A cycle in the import dependency graph.
    #[error("circular dependency detected among: {}", .0.join(", "))]
    CircularDependency(Vec<String>),

    /// Filesystem error during directory scan.
    #[error("{context}: {source}")]
    Io {
        context: String,
        source: std::io::Error,
    },
}

// ── Stage 7: Template rendering ────────────────────────────────

/// Errors from Jinja2 template loading and rendering.
#[derive(Debug, thiserror::Error)]
pub enum GenerateError {
    /// Language-specific configuration is missing or invalid.
    #[error("{0}")]
    InvalidConfig(String),

    /// Template directory not found or template load failure.
    #[error("template load error: {0}")]
    TemplateLoad(String),

    /// Template rendering failure.
    #[error("template render error: {0}")]
    TemplateRender(String),

    /// SCXML construct exists in the model but the requested target
    /// language does not (yet) implement it. Distinct from
    /// `InvalidConfig` because the model itself is well-formed — the
    /// gap is in the codegen backend, and the message names both the
    /// feature and the language so the operator can pick a different
    /// `--lang` or wait on backend support.
    #[error("feature unsupported in this language: {0}")]
    UnsupportedFeature(String),

    /// Watching-zenoh RFC §5.J.4: an MCU-class kind (link / worker /
    /// buffer-pool / reassembly, or an MCU-only codec sub-feature)
    /// was authored against a language target outside `(rust, c11)`.
    /// MCU-class kinds bottom out on the rust/c11 substrate only;
    /// binding them to cpp/kotlin/go/python has no defined emitter
    /// shape. Producer + matrix walker land with the algorithm kind
    /// in Phase A3.
    #[error(
        "MCU-class kind '{kind}' cannot be lowered to language '{language}': \
         only rust and c11 have MCU substrate (watching-zenoh RFC §5.J.4)"
    )]
    CodegenMcuClassKindOnNonMcuLanguage { kind: String, language: String },

    /// Watching-zenoh RFC §5.J.5: a generic-class kind expected to
    /// emit on every backend per the parity matrix is missing its
    /// per-kind Jinja2 template for the requested language. Template
    /// absence is an SCE bug, not a downstream concern. Producer +
    /// matrix walker land with the algorithm kind in Phase A3.
    #[error(
        "generic-class kind '{kind}': template missing for language '{language}' \
         (watching-zenoh RFC §5.J.4 expects all six backends to emit)"
    )]
    CodegenGenericKindBackendEmitMissing { kind: String, language: String },

    /// Watching-zenoh RFC §5.J.2 (C3 Atomic B-β): the SCXML document
    /// is generated with `sce-codegen generate -l rust --no-std` but
    /// contains a `<script>` element. The `sce-rust-runtime`
    /// `no_std` Cargo feature is mutually exclusive with the
    /// `script-engine-lua` / `script-engine-quickjs` features per
    /// spec line 1989 ("zero `alloc` dependency"): a Lua or QuickJS
    /// interpreter cannot be linked into a target without a global
    /// allocator. Author repair is to remove every `<script>` from
    /// the document, or drop `--no-std` from this codegen call.
    /// `document` is the SCXML basename; `locations` is a single
    /// human-readable summary of where the offending `<script>` was
    /// found (e.g. `"<script> in state 'init'"`).
    #[error(
        "Rust no_std variant rejects `<script>`: document '{document}' uses ECMAScript \
         at {locations} (watching-zenoh RFC §5.J.2; sce-rust-runtime no_std feature \
         is incompatible with `script-engine-lua` and `script-engine-quickjs`)"
    )]
    CodegenNoStdScriptNotSupported { document: String, locations: String },

    /// Watching-zenoh RFC §5.J.2 (C3 Atomic B-β): the SCXML document
    /// is generated with `sce-codegen generate -l rust --no-std` but
    /// contains a W3C SCXML C.2 `<send>` that targets
    /// `BasicHTTPEventProcessor` (either by explicit `type=` or by
    /// `target` URL beginning with `http://` / `https://`). The
    /// runtime crate's `http-send` feature pulls in `tokio` +
    /// `reqwest`, both of which require std; the `no_std` feature
    /// asserts incompatibility at the cfg layer in
    /// `sce-rust-runtime/src/lib.rs`. Author repair is to remove the
    /// HTTP send, or drop `--no-std`. `locations` is a single
    /// human-readable summary of the offending `<send>` site.
    #[error(
        "Rust no_std variant rejects HTTP send: document '{document}' uses \
         BasicHTTPEventProcessor at {locations} (watching-zenoh RFC §5.J.2; \
         sce-rust-runtime no_std feature is incompatible with `http-send`)"
    )]
    CodegenNoStdHttpNotSupported { document: String, locations: String },

    /// Watching-zenoh RFC §5.J.2 (C3 Atomic B-γ2c): the SCXML document is
    /// generated with `sce-codegen generate -l rust --no-std` but contains a
    /// `<data src="...">` element. External-file loading requires
    /// `std::fs::read_to_string` plus `PathBuf` / `std::env::current_exe`,
    /// all of which are alloc- or OS-coupled and forbidden under the no_std
    /// variant per spec line 1989-1994 ("no path from generated no_std code
    /// into `alloc::*`"). Author repair is to inline the data via `expr` /
    /// element content or drop `--no-std`. `locations` is a single
    /// human-readable summary of the offending `<data>` sites.
    #[error(
        "Rust no_std variant rejects external `<data src>`: document '{document}' \
         loads file content at {locations} (watching-zenoh RFC §5.J.2; \
         filesystem helpers are gated to !no_std and unreachable from emitted code)"
    )]
    CodegenNoStdFsLoadNotSupported { document: String, locations: String },

    /// Watching-zenoh RFC §5.J.2 (C3 Atomic B-γ2c): the SCXML document is
    /// generated with `sce-codegen generate -l rust --no-std` but contains a
    /// `<invoke>` element. SCXML invoke binds child-session lifecycle to the
    /// parent statechart through `Arc<Mutex<Vec<…>>>` queues plus a
    /// `HashMap` of active sessions, all of which are alloc-coupled and
    /// forbidden under the no_std variant per spec line 1989-1994. Author
    /// repair is to remove the `<invoke>` (sub-statecharts are out of scope
    /// for the firmware profile) or drop `--no-std`. `locations` is a single
    /// human-readable summary of the offending `<invoke>` sites.
    #[error(
        "Rust no_std variant rejects `<invoke>`: document '{document}' invokes \
         child sessions at {locations} (watching-zenoh RFC §5.J.2; \
         invoke processing is gated to !no_std and unreachable from emitted code)"
    )]
    CodegenNoStdInvokeNotSupported { document: String, locations: String },

    /// RFC §5.F: a `<sce:fold>` body or a `<sce:const init=...>` scalar
    /// expression cannot be reduced to a build-time value. The host
    /// interpreter rejects every construct outside the §5.F substrate
    /// (member access, function calls, runtime-only identifiers, string
    /// or null literals, malformed numeric literals, …) under this
    /// single wire code; `detail` quotes the specific clause that
    /// triggered. `algorithm` + `const_name` locate the offending
    /// declaration so the consumer can route the repair without parsing
    /// the message.
    #[error(
        "algorithm '{algorithm}': <sce:const name=\"{const_name}\">: \
         const-not-foldable: {detail}"
    )]
    ConstNotFoldable {
        algorithm: String,
        const_name: String,
        detail: String,
    },

    /// RFC §5.F bound 1: total iteration count across the body of a
    /// single `<sce:fold>` (or its nested while/foreach loops) exceeded
    /// the configured budget. Default is
    /// [`crate::forge::const_fold::Budget::DEFAULT_MAX_ITERS`]
    /// (1_000_000); the CLI knob is `--const-fold-budget=N`.
    /// `const_name` is `Some` when the budget tripped inside a specific
    /// const declaration (the common case); a future caller that drives
    /// the interpreter outside `lower_algorithm_consts` can pass `None`.
    #[error(
        "algorithm '{algorithm}': {}const-fold-budget-exceeded: \
         total iteration count exceeded the configured budget of {budget} \
         (RFC §5.F bound 1; override with --const-fold-budget=N)",
        match const_name { Some(n) => format!("<sce:const name=\"{n}\">: "), None => String::new() }
    )]
    ConstFoldBudgetExceeded {
        algorithm: String,
        const_name: Option<String>,
        budget: u64,
    },

    /// RFC §5.F: the value yielded by a `<sce:fold>` body (or the init
    /// expression of a scalar `<sce:const>`) cannot be coerced to the
    /// declared element / scalar type. `expected` is the declared slot
    /// type; `actual` is a short tag describing the produced value's
    /// domain (e.g. `"bool"`, `"float"`) — substring `"bool→Uint16"` /
    /// `"float→Int32"` patterns the prior β slug emitted, preserved for
    /// consumers dispatching on the message text.
    #[error(
        "algorithm '{algorithm}': <sce:const name=\"{const_name}\">: \
         const-yield-type-mismatch: cannot coerce {actual} to {expected:?}"
    )]
    ConstYieldTypeMismatch {
        algorithm: String,
        const_name: String,
        expected: SceType,
        actual: String,
    },
}

/// CLI exit code by error category.
impl ForgeError {
    pub fn exit_code(&self) -> i32 {
        match self {
            ForgeError::Xml(_) => 2,
            ForgeError::Validation(_) => 3,
            ForgeError::Expression(_) => 4,
            ForgeError::Import(_) => 5,
            ForgeError::Manifest(_) => 6,
            ForgeError::Generate(_) => 7,
            // RFC §W5 D2: SCXML semantic-validation shares the
            // forge-validation exit code (3) — both are post-parse
            // semantic-stage rejections; the wire `code` distinguishes
            // forge vs SCXML failures, the exit code does not.
            ForgeError::Scxml(_) => 3,
            ForgeError::Io { .. } => 8,
        }
    }
}
