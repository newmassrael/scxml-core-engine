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
