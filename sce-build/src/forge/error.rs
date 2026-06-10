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
///
/// Watching-zenoh RFC §synth-5-O: also reused as the per-IR-node
/// provenance record. Every emission-eligible node carries an
/// `Option<SourceLocation>` so codegen templates can emit per-backend
/// SCE-MAP markers (`#line` / `//line` / `// SCE-MAP:` / `#[doc]`)
/// above the function header that node lowers to. Serialised so
/// minijinja templates can `{% if state.source_location %}{{ ... }}{% endif %}`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, Default)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct SourceLocation {
    /// Producer-defined identifier for the source document — typically
    /// the SCXML file's basename, but consumers MUST treat it as an
    /// opaque label scoped to the current emit (see
    /// `docs/SCE_FORGE_AST.md` §9 for the consumer contract). Not
    /// guaranteed to be an absolute or workspace-relative path, and
    /// not guaranteed unique across emits when two inputs share a
    /// basename.
    pub file: String,
    /// 1-based source line. Optional — XSD-level diagnostics carry
    /// no line; synthesised IR nodes have no source position.
    pub line: Option<u32>,
    /// 1-based source column. Optional — only node-precise raises
    /// carry column info.
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
    pub fn new(error: E, file: impl Into<String>, line: Option<u32>, col: Option<u32>) -> Self {
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
// The four largest child enums (`Validation` 160 B, `Mesh` 168 B,
// `Scxml` 80 B, `Generate` 80 B) are boxed so `Located<ForgeError>`
// (the wrapped shape callers actually return) stays under clippy's
// `result_large_err` threshold (128 B). `Located` adds 40 B for its
// `SourceLocation` payload, so any inline child enum >= 88 B pushes
// the wrapper over the lint. The smaller variants (`Xml`,
// `Expression`, `Import`, `Manifest`) stay inline because they fit
// comfortably under the budget without an extra allocation.
#[derive(Debug, thiserror::Error)]
pub enum ForgeError {
    #[error(transparent)]
    Xml(#[from] XmlError),

    #[error(transparent)]
    Validation(Box<ValidationError>),

    #[error(transparent)]
    Expression(#[from] ExprError),

    #[error(transparent)]
    Import(#[from] ImportError),

    #[error(transparent)]
    Manifest(#[from] ManifestError),

    #[error(transparent)]
    Generate(Box<GenerateError>),

    /// SCXML semantic-validation failures — distinct from forge
    /// `ValidationError` because the rules come from §scxml-3
    /// reference resolution, not forge-document structure rules.
    /// §wire-W5 D2 keeps `ScxmlSemanticError` as a parallel enum
    /// outside `forge::*` but routes it through `ForgeError` so the
    /// `Located<ForgeError>` plumbing and JSON wire layer apply
    /// uniformly. Wire codes mostly REUSE existing `validation/*`
    /// per the §wire-W4 D4 fold (concept identity); only `TopLevelScriptUnloaded`
    /// is W3C-SCXML-specific and gets its own `scxml/*` code.
    #[error(transparent)]
    Scxml(Box<crate::scxml_semantic::ScxmlSemanticError>),

    /// Mesh-deploy / topology / external-config / codegen failures
    /// routed through the forge compile pipeline. Mirrors the
    /// [`Self::Scxml`] precedent — `MeshError` is a parallel enum
    /// outside `forge::*` (its own `mesh::error` module owns deploy
    /// validation, transport routing, etc.), but the orchestrator
    /// [`crate::compile_scxml_with_imports`] needs a deploy-aware
    /// path that surfaces `mesh/deploy/*` codes through the same
    /// `Located<ForgeError>` plumbing the forge cross-doc validators
    /// already use. The `MeshError` `SingleDiagnostic` impl handles
    /// wire payload conversion for every variant (Deploy / External /
    /// Topology / Codegen / Io) — `forge_error_fields` delegates to
    /// it via `e.diagnostic_payload()`.
    #[error(transparent)]
    Mesh(Box<crate::mesh::error::MeshError>),

    #[error("I/O error on {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
}

// Hand-written `From` impls so `?` still propagates the unboxed leaf
// types into `ForgeError` for the boxed variants — the `#[from]`
// derive can't be used on a boxed variant because it would generate
// `From<Box<E>>` and leave the unboxed form unsupported. Both arms
// are provided per boxed variant so callers may pass either shape.
// The inline variants (Xml/Expression/Import/Manifest) get their
// `From` from thiserror's `#[from]` derive.

impl From<ValidationError> for ForgeError {
    fn from(err: ValidationError) -> Self {
        Self::Validation(Box::new(err))
    }
}

impl From<Box<ValidationError>> for ForgeError {
    fn from(err: Box<ValidationError>) -> Self {
        Self::Validation(err)
    }
}

impl From<GenerateError> for ForgeError {
    fn from(err: GenerateError) -> Self {
        Self::Generate(Box::new(err))
    }
}

impl From<Box<GenerateError>> for ForgeError {
    fn from(err: Box<GenerateError>) -> Self {
        Self::Generate(err)
    }
}

impl From<crate::scxml_semantic::ScxmlSemanticError> for ForgeError {
    fn from(err: crate::scxml_semantic::ScxmlSemanticError) -> Self {
        Self::Scxml(Box::new(err))
    }
}

impl From<Box<crate::scxml_semantic::ScxmlSemanticError>> for ForgeError {
    fn from(err: Box<crate::scxml_semantic::ScxmlSemanticError>) -> Self {
        Self::Scxml(err)
    }
}

impl From<crate::mesh::error::MeshError> for ForgeError {
    fn from(err: crate::mesh::error::MeshError) -> Self {
        Self::Mesh(Box::new(err))
    }
}

impl From<Box<crate::mesh::error::MeshError>> for ForgeError {
    fn from(err: Box<crate::mesh::error::MeshError>) -> Self {
        Self::Mesh(err)
    }
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
    /// Mirrors C++ `SCE::parsing::ParseFileNotFound` (§wire-W4 D2).
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
    /// Mirrors C++ `SCE::parsing::ParseWrongRootElement` (§wire-W4 D2).
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
    /// expansion per RFC §6.5 — the C++ runtime does not
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

    /// §scxml-G-7 — a `<sce:action>` Custom Action Element appears
    /// somewhere v1 does not support: anywhere other than a direct
    /// `<transition>` child (e.g. `<onentry>`, `<onexit>`, an initial
    /// transition, or nested inside `<if>` / `<foreach>`).
    #[error("<sce:action name=\"{name}\">: {detail}")]
    NativeActionPlacement { name: String, detail: String },

    /// §scxml-G-7 — a `<sce:action>` `<sce:arg>` cannot be lowered to
    /// a typed native value: it is not a bare `_event.data.<field>`
    /// reference, the triggering event imports no EventSchema, or the
    /// referenced payload field is enum-typed (not natively representable).
    #[error("<sce:action name=\"{name}\">: {detail}")]
    NativeActionArgument { name: String, detail: String },

    /// §scxml-G-7 — a `<sce:action name>` appears on more than one
    /// transition with incompatible argument signatures, so a single
    /// generated `Actions` trait method cannot serve every call site.
    #[error("<sce:action name=\"{name}\">: {detail}")]
    NativeActionSignatureConflict { name: String, detail: String },

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
    /// on a `<send>`. The migration window (parse-tolerant
    /// warning) is closed; presence of the attribute is
    /// a hard build error.
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
    /// each other before any runtime data flows.
    #[error("{procedure}: {detail}")]
    BytesMaxSizeViolation { procedure: String, detail: String },

    /// RFC §synth-5-A: a local `<sce:var>` or `<sce:foreach item>` reuses
    /// the name of a parameter (or another local) inside the same
    /// algorithm body. Read/write access becomes ambiguous in v1, so
    /// the parser rejects the reuse before lowering.
    #[error("algorithm: identifier '{name}' shadows {what}")]
    AlgorithmLocalShadowsParam { name: String, what: String },

    /// RFC §synth-5-A: `<sce:assign target=...>` writes to an l-value the
    /// algorithm body cannot mutate. v1 forbids assigning to a
    /// parameter (parameters are read-only) and to the foreach loop
    /// variable. `target` is the offending l-value text;
    /// `restriction` names which rule was hit.
    #[error("<sce:assign target=\"{target}\">: {restriction}")]
    AlgorithmLvalueUnsupported { target: String, restriction: String },

    /// RFC §synth-5-A: an algorithm declares a non-void `<sce:return type>`
    /// in the signature but the body contains no terminal
    /// `<sce:return expr>` along every code path. v1 detects only the
    /// trivial case (last statement is not a return); flow-sensitive
    /// path tracking is not implemented until a consumer needs it.
    #[error(
        "algorithm: signature declares return type but body's last statement is not <sce:return>"
    )]
    AlgorithmReturnMissing,

    /// RFC §synth-5-A + §synth-5-L line 2642-2647 (item C7 lowering): `<sce:foreach
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

    /// RFC §synth-5-A line 311 + §synth-5-L line 2642-2647 (item C7 lowering): `<sce:call
    /// target="alias.method">` where `alias` does not match any
    /// `<sce:import as="...">` declared in the enclosing algorithm doc.
    /// `candidates` is the sorted list of declared import aliases.
    #[error("algorithm: <sce:call target=\"{target}\">: alias '{alias}' is not a declared import")]
    AlgorithmCallTargetUnknown {
        target: String,
        alias: String,
        candidates: Vec<String>,
    },

    /// RFC §synth-5-A line 311 + §synth-5-L line 2611-2618 (item C7 lowering): dotted
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

    /// RFC §synth-5-A line 333 (algorithms are pure: no heap allocation, no
    /// closures, no exceptions/panics) + §synth-5-L line 2611 (BC mutation
    /// API is `insert`/`remove`). Algorithm-body dispatch into a BC
    /// alias is read-only; `<sce:call target="bc_alias.insert">` or
    /// `bc_alias.remove` violates the purity contract.
    #[error("algorithm: <sce:call target=\"{target}\">: mutating bounded-collection method '{method}' is forbidden from algorithm body (algorithms are pure per RFC §5.A)")]
    AlgorithmBcMutationForbidden { target: String, method: String },

    /// RFC §synth-5-A v1 + §synth-5-L line 2642-2647 (item C7 lowering): `<sce:foreach
    /// in="<bc-alias>">` body declares a `<sce:var name="..."
    /// type="uint8">` — the legacy bytes-iteration pattern where the
    /// loop item is a `u8`. BC iteration carries the element-type, not
    /// `uint8`; the body cannot rely on `u8` semantics.
    #[error("algorithm: <sce:foreach in=\"{src}\"> over bounded-collection: body's <sce:var name=\"{var_name}\" type=\"uint8\"> uses the bytes-iteration pattern but '{src}' is a bounded-collection (item carries element-type)")]
    AlgorithmForeachSourceBcWithBytesItemType { src: String, var_name: String },

    /// RFC §synth-5-A line 311 (item C7 lowering): dotted `<sce:call
    /// target="alias.method">` argument count does not match the
    /// imported callable's signature arity. For algorithm imports the
    /// expected arity comes from the imported `<sce:signature>`'s
    /// `<sce:param>` count. BC methods have fixed arities (1-2 per
    /// §synth-5-L), validated by the same path.
    #[error("algorithm: <sce:call target=\"{target}\">: argument count {actual} does not match callable's arity {expected}")]
    AlgorithmCallArgCountMismatch {
        target: String,
        actual: usize,
        expected: usize,
    },

    /// RFC §synth-5-B variant primitive: the variant's enumerated
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

    /// Variant default-arm uniformity rule: more than one
    /// `<sce:arm default="true"/>` declared inside the same
    /// `<sce:variant>`. The `default` attribute steers the outer
    /// codec's `Default::default()` to a single deliberately-chosen
    /// arm — two declarations are ambiguous and the parser refuses
    /// to silently pick one. Author resolves by removing the
    /// `default="true"` from all but the intended arm. Distinct
    /// from `<sce:default>` (catch-all for unknown tag values),
    /// which the parser still permits at most once per RFC §synth-5-B.
    #[error(
        "codec '{codec}': <sce:variant> declares more than one <sce:arm default=\"true\"/> \
         (first arm value={first_arm_value:#x}, second arm value={second_arm_value:#x}) — \
         only one arm may be marked the Default-trait starting value; remove \
         default=\"true\" from all but the intended arm. (The catch-all \
         <sce:default> element is unrelated and still permitted once.)"
    )]
    CodecVariantDuplicateDefaultArm {
        codec: String,
        first_arm_value: u64,
        second_arm_value: u64,
    },

    /// Variant default-arm uniformity rule: an outer
    /// `<sce:arm value="X"/>` declares X as the wire-dispatch
    /// value, but the inner codec it points at declares a
    /// different `<sce:flag value="Y"/>` on its matching
    /// peek-byte flag. Round-trip would land the wrong arm at
    /// decode time (peek byte = Y ≠ X) for any default-constructed
    /// inner instance. Applies to every arm — the wire-MID baked
    /// on the inner codec is intrinsic to its identity, not to
    /// whether it is the variant's Default-trait starting arm.
    /// Author resolves by aligning either the outer arm value or
    /// the inner flag value.
    #[error(
        "codec '{codec}': <sce:arm value={arm_value:#x}/> selects inner codec \
         '{inner_codec}' but that codec declares <sce:flag name='{inner_flag}' \
         value={inner_flag_value:#x}/> on its dispatch field — outer arm value and \
         inner flag value must match for round-trip dispatch to resolve to the same \
         arm; align one to the other"
    )]
    CodecVariantArmMidMismatch {
        codec: String,
        arm_value: u64,
        inner_codec: String,
        inner_flag: String,
        inner_flag_value: u64,
    },

    /// Variant default-arm uniformity rule: an outer
    /// `<sce:arm value="X"/>` selects an inner codec, but that
    /// codec's matching peek-byte flag does NOT declare a
    /// `value="..."` constant. Without a baked wire-MID the
    /// inner's `Default::default()` zero-fills the dispatch byte
    /// and a standalone encode of the default-constructed inner
    /// produces wire bytes that decode into the catch-all (or a
    /// different) arm. Applies to every arm — the inner codec's
    /// wire-MID is intrinsic to its identity, not gated on
    /// whether the arm is the variant's Default-trait starting
    /// arm. Author resolves by adding `value="..."` to the inner
    /// codec's flag whose bit-range matches the variant's peek
    /// byte.
    #[error(
        "codec '{codec}': <sce:arm value={arm_value:#x}/> selects inner codec \
         '{inner_codec}', but '{inner_codec}' does not declare a <sce:flag value=\"...\"/> \
         constant on its dispatch field — the inner's Default would zero-fill the wire byte \
         and break round-trip; add <sce:flag name='{expected_flag}' value={arm_value:#x}/> \
         to '{inner_codec}'"
    )]
    CodecVariantArmInnerMidUndeclared {
        codec: String,
        arm_value: u64,
        inner_codec: String,
        expected_flag: String,
    },

    /// A variant arm
    /// body resolves to a codec whose `<sce:variant>` is itself in
    /// caller-tag shape (no `tag=` attribute). Caller-tag leaves require
    /// the caller to supply the dispatch tag as a positional decode
    /// argument; in a variant-arm context there is no natural source
    /// for that tag (the parent dispatcher uses its OWN tag field to
    /// select which arm to invoke, not to forward a tag onward). The
    /// codegen would emit `ArmBody::decode(cursor, [flag-binds…])`
    /// without the required `tag` arg, producing a downstream compile
    /// error (`rustc E0061: missing argument of type u8`). Reject
    /// upstream with a typed diagnostic so the author sees the
    /// constraint at codegen time instead of through a cross-language
    /// compiler error.
    ///
    /// `arm_value` is `Some(v)` for enumerated `<sce:arm value="v">`
    /// arms and `None` for the catch-all `<sce:default>` arm (which
    /// has no specific value but goes through the same codegen call
    /// site and has the same arity requirement). The two are
    /// indistinguishable downstream but must be distinguishable in the
    /// diagnostic so authors with BOTH an enumerated arm at 0x00 AND a
    /// default arm see two distinct messages.
    ///
    /// Repair: either give the arm body its own `tag=` attribute on
    /// `<sce:variant>` so it reads the tag from its own wire bytes,
    /// or redesign so the dispatcher consumes the caller-tag leaf through
    /// `<sce:embed>` with `<sce:variant-dispatch>` (the embed-site
    /// path which threads the tag from a parent flag — already
    /// supported).
    #[error(
        "codec '{parent_codec}': variant arm {} (alias '{embedded_alias}') resolves to codec \
         '{embedded_codec}' whose <sce:variant> is in caller-tag shape (no tag= attribute) \
         — there is no natural source for the inner tag in a variant-arm context. Either add \
         tag=\"<field>\" to '{embedded_codec}' so it reads its tag from its own wire bytes, \
         or expose '{embedded_codec}' via <sce:embed> + <sce:variant-dispatch> on a parent \
         flag instead of as a variant arm body.",
        arm_value.map(|v| format!("value={v:#x}")).unwrap_or_else(|| "<default>".to_string())
    )]
    CodecVariantArmBodyCallerTagUnsupported {
        parent_codec: String,
        arm_value: Option<u64>,
        embedded_alias: String,
        embedded_codec: String,
    },

    /// Variant default-arm uniformity rule: the
    /// `<sce:variant>` declares no `<sce:arm default="true"/>` —
    /// every variant must name a deliberate default arm so the
    /// outer codec's `Default::default()` does not fall back to
    /// the implicit "first declared arm" convention (which led to
    /// the watching-zenoh R87 defect). Author resolves by adding
    /// `default="true"` to the intended arm. Distinct from
    /// `<sce:default>` (catch-all for unknown tag values), which
    /// may or may not coexist on the same variant.
    #[error(
        "codec '{codec}': <sce:variant> declares no <sce:arm default=\"true\"/> — \
         every variant must mark one arm as the deliberate Default-trait starting \
         value so codegen does not implicitly pick the first declared arm; add \
         default=\"true\" to the intended arm. (The catch-all <sce:default> \
         element is a separate concept and does not satisfy this requirement.)"
    )]
    CodecVariantNoDefaultArm { codec: String },

    /// Variant-default deploy overlay rule: the `deploy.yaml`
    /// `variant_defaults:` overlay names a codec and an arm value,
    /// but the codec either has no `<sce:variant>` to dispatch over
    /// (the overlay can't apply) or has a variant whose declared
    /// `<sce:arm value="V"/>` set does not contain the overlay's
    /// chosen value. Author resolves by aligning the overlay entry
    /// with a declared arm value, or removing the overlay entry if
    /// the codec is not meant to carry a per-consumer default
    /// choice. The candidates axis lists every declared arm value
    /// on the codec — empty when the codec has no variant at all.
    #[error(
        "codec '{codec}': deploy.yaml variant_defaults names arm value \
         {overlay_arm_value:#x}, but the codec declares no matching <sce:arm value=...> — \
         {declared_summary}; align the overlay entry with one of the declared values \
         or remove it from variant_defaults",
        declared_summary = if declared_arms.is_empty() {
            "the codec has no <sce:variant> at all".to_string()
        } else {
            format!(
                "declared arms: [{}]",
                declared_arms
                    .iter()
                    .map(|v| format!("{v:#x}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    )]
    CodecVariantDefaultOverlayArmNotDeclared {
        codec: String,
        overlay_arm_value: u64,
        declared_arms: Vec<u64>,
    },

    /// Parent-side variant-dispatch rule: a parent codec's
    /// `<sce:variant-dispatch flag="X.Y">` at an import-site names a
    /// dotted `<carrier>.<flag>` that does not resolve against the
    /// parent's own fields — either the carrier field `X` is not
    /// declared, or the flag `Y` is not declared on that carrier.
    /// Author repairs the dotted reference; the candidates list
    /// surfaces the available carriers (or flags on the resolved
    /// carrier) to disambiguate typos.
    #[error(
        "parent codec '{parent_codec}': <sce:variant-dispatch flag=\"{flag_source}\"/> on \
         import '{embedded_alias}' does not resolve — {detail}. Correct the dotted reference \
         to one of: [{candidates_summary}].",
        candidates_summary = candidates.join(", ")
    )]
    CodecVariantDispatchFlagNotResolved {
        parent_codec: String,
        embedded_alias: String,
        flag_source: String,
        detail: String,
        candidates: Vec<String>,
    },

    /// Parent-side variant-dispatch rule: a parent codec's
    /// `<sce:variant-dispatch flag="X.Y">` names a flag whose `width`
    /// cannot represent the imported codec's full arm enumeration.
    /// The dispatch value is `(carrier >> bit) & ((1<<width)-1)` —
    /// `1 << width` distinct values; the imported variant has more
    /// arms than that. Author resolves by widening the flag (more bits)
    /// or shrinking the variant (fewer arms).
    #[error(
        "parent codec '{parent_codec}': <sce:variant-dispatch flag=\"{carrier}.{flag}\"/> on \
         import '{embedded_alias}' (codec '{embedded_codec}') — flag width {flag_width} can \
         encode at most {max_values} dispatch values, but the imported codec declares \
         {arm_count} arms. Widen the flag or reduce the arm count."
    )]
    CodecVariantDispatchBitWidthMismatch {
        parent_codec: String,
        embedded_alias: String,
        embedded_codec: String,
        carrier: String,
        flag: String,
        flag_width: u32,
        max_values: u64,
        arm_count: usize,
    },

    /// Parent-side variant-dispatch rule: a parent codec imports a variant
    /// codec WITHOUT declaring `<sce:variant-dispatch>`, AND the
    /// imported codec has no `<sce:arm default="true"/>` marker. With
    /// no wire-level dispatch and no default arm, the parent's decode
    /// cannot deterministically pick which arm to instantiate from
    /// the wire bytes — the leaf's arms would all be equally
    /// plausible. Author resolves by either (a) adding
    /// `<sce:variant-dispatch flag="X.Y"/>` on the import to provide a
    /// dispatch source, or (b) marking one arm in the imported codec
    /// as `default="true"` so the parent's decode falls back to it
    /// when no dispatch is wired.
    #[error(
        "parent codec '{parent_codec}': import '{embedded_alias}' (codec '{embedded_codec}') \
         is a variant codec but the import declares no <sce:variant-dispatch> and the \
         imported codec has no <sce:arm default=\"true\"/> marker. Add \
         <sce:variant-dispatch flag=\"...\"/> to the import, or mark one arm in '{embedded_codec}' \
         as default=\"true\"."
    )]
    CodecVariantDispatchArmsNotDistinguishableWithoutDefault {
        parent_codec: String,
        embedded_alias: String,
        embedded_codec: String,
    },

    /// Parent-side variant-dispatch rule: a parent codec's
    /// `<sce:variant-dispatch flag="X.Y">` targets a flag that ALSO
    /// carries a static `value=` constant on the same parent codec.
    /// Derived (from arm choice) and static cannot coexist on the
    /// same bit — they would have to agree, which is structurally
    /// ambiguous when the author later changes the arm. Author
    /// resolves by either removing the `value=` constant (derivation
    /// wins) or moving the dispatch to a different flag.
    #[error(
        "parent codec '{parent_codec}': flag '{carrier}.{flag}' has static <sce:flag \
         value={static_value:#x}/>, but <sce:variant-dispatch flag=\"{carrier}.{flag}\"/> on \
         import '{embedded_alias}' would derive the same bit from the variant's arm choice — \
         static and derived cannot coexist. Remove the value= constant or move the dispatch \
         to a different flag."
    )]
    CodecVariantDispatchFlagHasStaticValue {
        parent_codec: String,
        embedded_alias: String,
        carrier: String,
        flag: String,
        static_value: u64,
    },

    /// Parent-side variant-dispatch rule: a parent codec declares a field
    /// with `<sce:variant-dispatch>` BEFORE the carrier field that
    /// the dispatch flag belongs to. Encode-side derivation needs the
    /// carrier byte to be emitted AFTER (or with) the derived bit set;
    /// having the embed declared before the carrier breaks readable
    /// declaration order (the carrier-first convention also matches
    /// wire order). Author resolves by reordering fields so the
    /// carrier appears before the embed field.
    #[error(
        "parent codec '{parent_codec}': field '{embedded_field}' (import '{embedded_alias}') \
         has <sce:variant-dispatch flag=\"{carrier}.{flag}\"/>, but carrier '{carrier}' is \
         declared at field index {carrier_index} which is AFTER the embed field at index \
         {embedded_index}. Reorder fields so '{carrier}' precedes '{embedded_field}'."
    )]
    CodecVariantDispatchCarrierAfterEmbed {
        parent_codec: String,
        embedded_alias: String,
        embedded_field: String,
        carrier: String,
        flag: String,
        carrier_index: usize,
        embedded_index: usize,
    },

    /// Parent-side flag-bind rule: parent's `<sce:flag-bind input="X" ...>`
    /// references a leaf-side input name that the imported codec does
    /// not declare in its `<sce:flag-inputs>` block. Either the leaf
    /// renamed the input or the parent's bind has a typo. Repair: align
    /// the bind's `input=` attribute with the leaf's
    /// `<sce:flag-input name="...">`.
    #[error(
        "codec '{parent_codec}': <sce:flag-bind input=\"{input}\"/> on <sce:import as=\"{embedded_alias}\"> targets a leaf-side input that '{embedded_codec}' does not declare. Available inputs on the imported leaf: [{available_inputs}]. Align the bind's input= attribute with a declared <sce:flag-input name=\"…\">, or remove the bind if the leaf no longer needs that input."
    )]
    CodecFlagBindInputNotDeclared {
        parent_codec: String,
        embedded_alias: String,
        embedded_codec: String,
        input: String,
        available_inputs: String,
    },

    /// Parent-side flag-bind rule: parent's `<sce:flag-bind source="...">`
    /// references a source identifier that does not resolve to either
    /// (a) a local flags-carrier flag in `<carrier>.<flag>` dotted form,
    /// or (b) one of the parent's own `<sce:flag-input>` declarations
    /// for the chain-forwarder bare-name form. The carrier or flag name
    /// is typo'd, or the chain-forwarder source is missing from the
    /// parent's own `<sce:flag-inputs>`.
    #[error(
        "codec '{parent_codec}': <sce:flag-bind input=\"{input}\" source=\"{bind_source}\"/> on <sce:import as=\"{embedded_alias}\"> cannot be resolved against this codec's namespace. {detail}. Use <carrier>.<flag> form to reference a local flags-carrier flag, or the bare input name to forward one of this codec's own <sce:flag-input> declarations."
    )]
    CodecFlagBindSourceNotResolved {
        parent_codec: String,
        embedded_alias: String,
        input: String,
        bind_source: String,
        detail: String,
    },

    /// Parent-side flag-bind rule: parent's `<sce:flag-bind>` source width
    /// does not match the leaf-side input's declared width. v1 fixes
    /// flag-input width at 1 (single-bit), so this fires when the
    /// parent's source flag declares `width != 1`. Multi-bit input
    /// widening defers to a reachable consumer.
    #[error(
        "codec '{parent_codec}': <sce:flag-bind input=\"{input}\" source=\"{bind_source}\"/> on <sce:import as=\"{embedded_alias}\"> has source width {source_width} but leaf-side input '{input}' declares width {input_width}. v1 lock-in fixes flag-input width at 1; multi-bit inputs defer to a reachable consumer."
    )]
    CodecFlagBindWidthMismatch {
        parent_codec: String,
        embedded_alias: String,
        input: String,
        bind_source: String,
        source_width: u32,
        input_width: u32,
    },

    /// Parent-side flag-bind rule: the imported leaf codec declares a
    /// `<sce:flag-input name="X" .../>` but the parent's `<sce:import>`
    /// does not supply a matching `<sce:flag-bind input="X" .../>` —
    /// the leaf would receive an undefined value for that input. Repair:
    /// add the missing `<sce:flag-bind input="X" source="..."/>` child
    /// to the `<sce:import>`.
    #[error(
        "codec '{parent_codec}': <sce:import as=\"{embedded_alias}\"> imports '{embedded_codec}' which declares <sce:flag-input name=\"{input}\"/> but no matching <sce:flag-bind input=\"{input}\"/> is supplied. Bind the input to one of this codec's local flags-carrier flags (<sce:flag-bind input=\"{input}\" source=\"carrier.flag\"/>) or to one of this codec's own <sce:flag-input> declarations (<sce:flag-bind input=\"{input}\" source=\"local_input\"/>)."
    )]
    CodecFlagInputUnbound {
        parent_codec: String,
        embedded_alias: String,
        embedded_codec: String,
        input: String,
    },

    /// Parent-side flag-bind rule: a parent's `<sce:import>` declares two
    /// `<sce:flag-bind>` children with the same `input=` attribute.
    /// Each leaf-side input must be bound at most once.
    #[error(
        "codec '{parent_codec}': <sce:import as=\"{embedded_alias}\"> has duplicate <sce:flag-bind input=\"{input}\"/> declarations. Each leaf-side input may be bound at most once per import site."
    )]
    CodecFlagBindDuplicateInput {
        parent_codec: String,
        embedded_alias: String,
        input: String,
    },

    /// Parent-side flag-bind rule: parent's `<sce:flag-bind source="X.Y">`
    /// references a local carrier flag whose carrier field is declared
    /// AFTER the embed field that depends on it. The streaming codec
    /// cannot read the flag's bit before reaching the embed; the
    /// carrier must precede the embed in field declaration order.
    /// Mirrors the legacy
    /// `codec/requires-parent-flags-carrier-after-embed` ordering
    /// constraint translated into the inverted shape.
    #[error(
        "codec '{parent_codec}': <sce:flag-bind input=\"{input}\" source=\"{carrier}.{flag}\"/> on <sce:import as=\"{embedded_alias}\"> references a carrier '{carrier}' declared at field-index {carrier_index} but the embed '{embedded_field}' (which consumes the bound input) is at field-index {embedded_index}. Streaming decode requires carrier to precede consumer — reorder the fields so '{carrier}' is declared before '{embedded_field}'."
    )]
    CodecFlagBindCarrierAfterEmbed {
        parent_codec: String,
        embedded_alias: String,
        embedded_field: String,
        input: String,
        carrier: String,
        flag: String,
        carrier_index: usize,
        embedded_index: usize,
    },

    /// RFC §synth-5-B present-if primitive (item B1): the predicate on a
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

    /// RFC §synth-5-B repeat primitive (B2): the `sce:count` reference on a
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

    /// RFC §synth-5-B test-vector primitive: a `<sce:test-vector>` element
    /// appears under a `sce:kind` other than `algorithm` or
    /// `codec`. Other kinds (transform / lookup / validator
    /// / etc.) cannot host a hex-bytes round-trip oracle in v1 —
    /// their wire shape is not byte-stable enough to anchor a single
    /// reference vector. Author resolves by moving the test vector
    /// onto a supported kind or expressing the round-trip in the
    /// kind-specific harness oracle.
    #[error(
        "<sce:test-vector> is only supported on sce:kind=\"algorithm\" and sce:kind=\"codec\", but '{name}' declares sce:kind=\"{kind:?}\" — move the test vector to an algorithm/codec file or use the kind-specific harness oracle"
    )]
    TestVectorUnsupportedKind {
        /// Forge document name (root `name=` attribute).
        name: String,
        /// Actual kind that the test-vector was declared under.
        kind: ForgeKind,
    },

    /// RFC §synth-5-B B3 TLV chain primitive: `<sce:tlv-chain>` declared
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
    CodecTlvChainDepthUnspecified { codec: String, field: String },

    /// RFC §synth-5-B B3 DMA alignment primitive: a field with
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

    /// RFC §synth-5-B peek-byte cross-codec contract — a
    /// parent variant declares `<sce:peek-byte id="X"><sce:flag name=
    /// "F" bit="B" width="W"/></sce:peek-byte>` and an arm body codec
    /// declares its first `<sce:flags>` field with a `<sce:flag>` of
    /// the same name but a different bit / width. Since the peeked byte
    /// IS the arm body's first wire byte, both declarations must agree
    /// exactly. Repair by aligning one side to the other.
    #[error(
        "codec '{body_codec}' (arm body): peek-byte flag layout mismatch against parent codec '{parent_codec}' — {reason}"
    )]
    CodecPeekByteFlagLayoutMismatch {
        body_codec: String,
        parent_codec: String,
        reason: String,
    },

    /// RFC §synth-5-C byte-stream link endpoint: `<sce:framer ref="..."/>`
    /// is required on `sce:kind="link"` declarations. Without a framer
    /// reference, the codegen cannot wire the §synth-5-B codec into the RX/TX
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

    /// RFC §synth-5-C negative coverage: `<sce:link-class>` body text is
    /// not in the closed enumeration (RFC §synth-5-C lines 765-771 — `udp` /
    /// `tcp` / `serial` / `websocket` / `raw_eth`). Promotes the
    /// generic `validation/invalid-attribute` to a
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

    /// RFC §synth-5-C negative coverage: `<sce:backpressure>` element
    /// is required on `sce:kind="link"` declarations — the policy is
    /// load-bearing for the runtime crate's RX queue behavior under
    /// load. The parser used to tolerate the missing element by
    /// defaulting to `drop`; absence is now a hard error
    /// so authors must declare `drop` / `block` / `signal-event`
    /// intentionally rather than inheriting an implicit default.
    #[error(
        "link '{name}': missing required <sce:backpressure> child — `sce:kind=\"link\"` requires an explicit backpressure policy declaration per RFC §5.C; add a <sce:backpressure>drop|block|signal-event</sce:backpressure> child"
    )]
    LinkBackpressureUndeclared {
        /// Link document name (root `name=` attribute).
        name: String,
    },

    /// RFC §synth-5-C OS-axis negative coverage: the declared
    /// `<sce:link-class>` cannot run on the deploy-resolved
    /// `platform.os`. RFC §synth-5-C lines 838 names this code; the
    /// admissibility matrix lives in [`LinkClass::admits_os`] mirroring
    /// the table at RFC §synth-5-C lines 765-771 strict-literal:
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

    /// RFC §synth-5-C cross-resolution: the `<sce:rx-pool>` or
    /// `<sce:tx-pool>` reference resolves to a buffer-pool whose
    /// `<sce:slot-size>` is smaller than the framer codec's
    /// recursive worst-case encoded byte count. The TX path
    /// `event extract -> framer.encode() -> pool slot -> driver.send()`
    /// (RFC §synth-5-C lines 786-789) cannot honor zero-copy when the slot
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
    /// RFC §synth-5-C lines 793-794 spec anchor (rx-pool/tx-pool inherit the
    /// §synth-5-E pool model on both sides of the byte-stream).
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
        /// per RFC §synth-5-B).
        framer_max_bytes: u32,
    },

    /// RFC §synth-5-E buffer-pool placement validation: the declared
    /// `<sce:section>` is not in the resolved machine's
    /// `memory.sram_regions` map. Fires only via
    /// [`compile_forge_with_deploy`] when both `deploy` and
    /// `target_machine` are present (skip silently
    /// when deploy.yaml is unavailable). The `candidates` axis is the
    /// list of region names the resolved machine declares — drives
    /// `Fix::ReplaceOneOf` so the author can either rename the pool's
    /// `<sce:section>` body or extend deploy.yaml `memory.sram_regions`.
    /// RFC §synth-5-E lines 1000-1023 + 1537 spec anchor.
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

    /// RFC §synth-5-E buffer-pool size validation: the pool's storage
    /// footprint (`slot_count × slot_size`) does not fit inside the
    /// resolved SRAM region's `size` field. Fires only via
    /// [`compile_forge_with_deploy`] when section validation already
    /// passed (skip silently when deploy.yaml is
    /// unavailable; `mem/pool-section-conflict` is the prerequisite
    /// gate). No `candidates` axis — the repair is to raise the
    /// region size in deploy.yaml or shrink `slot_count`/`slot_size`;
    /// emitted as `Fix::None` because both axes are author choices.
    /// RFC §synth-5-E lines 1031-1086 spec anchor (linker-fragment-side
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

    /// RFC §synth-5-E codegen self-check: the rendered linker fragment
    /// is missing the explicit `. = ALIGN(<n>);` inter-pool sentinel
    /// (§synth-5-E lines 1059-1064). This is a codegen invariant violation,
    /// not an authoring mistake — fires only when the template itself
    /// drops the sentinel. The artifact is what makes the inter-pool
    /// boundary diff-visible (any PR that drops it shows up in the
    /// linker fragment) and what protects the post-pool boundary from
    /// master-script INCLUDE re-ordering. RFC §synth-5-E lines 1059-1064 +
    /// 1537 spec anchor.
    #[error(
        "buffer-pool '{name}': linker fragment is missing the inter-pool `. = ALIGN(N);` sentinel — codegen invariant violation per RFC §5.E lines 1059-1064; report at https://github.com/newmassrael/scxml-core-engine/issues"
    )]
    BufferPoolInterPoolPaddingNotEmitted {
        /// Buffer-pool document name (root `name=` attribute).
        name: String,
    },

    /// watching-zenoh RFC §synth-5-E C5 cache-maintenance validation
    /// (spec line 1544): pool `alignment` is smaller than the resolved
    /// target's `platform.dcache_line_size` while `cache-policy:
    /// maintain` is in effect. The cache-line alignment violation
    /// matters because partial-line `cache_invalidate_by_addr` calls
    /// corrupt adjacent data on the start side — the unaligned head
    /// crosses into the previous slot's last cache line, which the
    /// invalidate then evicts together with the slot's own bytes.
    /// Fires only via [`compile_forge_with_deploy`] after section
    /// validation passes (silent-skip when deploy.yaml is
    /// unavailable). Author resolution: raise `<sce:alignment>` to at
    /// least the platform's `dcache_line_size`. RFC §synth-5-E line 1544 +
    /// §synth-5-I line 1742-1744 spec anchor.
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

    /// watching-zenoh RFC §synth-5-E C5 cache-maintenance validation
    /// (spec line 1545): `<sce:slot-size>` is not a whole-number
    /// multiple of `platform.dcache_line_size` while `cache-policy:
    /// maintain` is in effect. Each slot must occupy a whole number
    /// of cache lines so that `cache_invalidate_by_addr(slot, len)`
    /// after RX cannot touch the bytes of the adjacent slot that
    /// share the boundary cache line. Fires only via
    /// [`compile_forge_with_deploy`] after section validation passes
    /// (silent-skip when deploy.yaml is unavailable). Author
    /// resolution: round `slot_size` up to the next cache-line
    /// multiple and continue using the original logical size from
    /// within each slot. RFC §synth-5-E line 1545 + §synth-5-I line 1742-1744
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

    /// watching-zenoh RFC §synth-5-E C5 cache-maintenance validation
    /// (spec line 1543): pool declares `cache-policy: maintain` or
    /// `cache-policy: non-cacheable` while the resolved target
    /// platform has `has_dcache: false`. The maintenance call sites
    /// would be no-ops at best, MPU configuration request at worst —
    /// neither is meaningful on a core without a data cache. Fires
    /// only via [`compile_forge_with_deploy`] after section
    /// validation passes (silent-skip when deploy.yaml is
    /// unavailable). Author resolution: switch the pool to
    /// `cache-policy: none`. RFC §synth-5-E line 1543 spec anchor.
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

    /// watching-zenoh RFC §synth-5-E C5 cache-maintenance + §synth-5-I author-
    /// guard (spec line 1548): an `<sce:extern>` declaration in the
    /// build attempts to author one of the cache-maintenance trio
    /// (`sce_dcache_clean_by_addr`, `sce_dcache_invalidate_by_addr`,
    /// `sce_dcache_clean_invalidate_by_addr`). Per spec lines
    /// 1222-1227, cache maintenance is **FSM-driven**: codegen
    /// auto-injects the externs and emits the calls on the buffer-
    /// pool lifecycle edges. Author authoring would silently allow
    /// duplicate declarations and the class of bugs ("the maintenance
    /// call sits in the wrong place") that the FSM-driven design
    /// prevents. Fires at parse time, before the whitelist
    /// validator. Author resolution: remove the offending
    /// `<sce:extern>`; the buffer-pool kind handles cache calls
    /// automatically when `cache-policy: maintain`. RFC §synth-5-E line
    /// 1548 + lines 1222-1227 spec anchor.
    #[error(
        "<sce:extern name=\"{attempted_symbol}\">: cache-maintenance intrinsics are FSM-driven and authored automatically by the buffer-pool kind under `cache-policy: maintain` (RFC §5.E lines 1222-1227). Author <sce:extern> for the cache trio is forbidden — remove the declaration; codegen emits the calls on lifecycle edges."
    )]
    PoolCacheMaintenanceMisplaced {
        /// The cache trio symbol the author tried to declare.
        attempted_symbol: String,
    },

    /// watching-zenoh RFC §synth-5-E C5 cache-maintenance config-
    /// completeness diagnostic (spec line 1553): a target machine
    /// declares `platform.has_dcache: true` without setting
    /// `platform.has_speculative_prefetch`. Codegen cannot decide
    /// whether to emit the `free → dma-armed-rx` pre-arm cache-
    /// invalidate edge — silently emitting it on M0/M3/M4 wastes
    /// cycles, silently omitting it on M7+/A-class cores leads to
    /// documented packet corruption (RFC §synth-5-E lines 1199-1212).
    /// Fires only via [`compile_forge_with_deploy`] when at least
    /// one buffer-pool with `cache-policy: maintain` exists in the
    /// build (silent skip when no maintain-policy pool is reachable
    /// — the field has no consumer to require it). Author
    /// resolution: declare `has_speculative_prefetch` per the SoC
    /// datasheet (M7+/A-class = true, M3/M4 = false). RFC §synth-5-E
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

    /// watching-zenoh RFC §synth-5-E C5 cache-maintenance codegen self-
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
    /// §synth-5-E line 1552 spec anchor.
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

    /// RFC §synth-5-E codegen self-check: the rendered C11 buffer-pool
    /// header is missing the `#include <sce/sample.h>` directive. The
    /// generated pool header surfaces the runtime Sample API
    /// (typestate-tracked `sce_sample_t` + Layer 1 attribute family) by
    /// pulling in `sce-c-runtime/include/sce/sample.h`; without the
    /// include, downstream consumers building against the pool header
    /// silently lose Layer 1 typestate coverage even on Clang ≥ 9
    /// because the macro family is unreachable. The diagnostic fires
    /// only when the template itself drops the include — it is a
    /// codegen invariant, not an authoring mistake. RFC §synth-5-E lines
    /// 1276-1346 + 1520-1525 spec anchors.
    #[error(
        "buffer-pool '{name}': generated C11 header is missing the `#include <sce/sample.h>` directive — Layer 1 typestate attributes will be unavailable on consumer builds, codegen invariant violation per RFC §5.E lines 1276-1346; report at https://github.com/newmassrael/scxml-core-engine/issues"
    )]
    BufferPoolSampleTypestateAttributesDisabled {
        /// Buffer-pool document name (root `name=` attribute).
        name: String,
    },

    /// watching-zenoh RFC §synth-5-E: a `<sce:on-sample>`
    /// element appears outside a `<state>` or `<parallel>` parent.
    /// The parser-AST extension means the validator can
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

    /// watching-zenoh RFC §synth-5-E: two or more
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

    /// watching-zenoh RFC §synth-5-E: a `<sce:on-sample>`
    /// declares an `event=` whose name collides with a built-in W3C
    /// SCXML event prefix (`error.*`, `done.*`). The §scxml-5.10
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

    /// watching-zenoh RFC §synth-5-E cross-ref:
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

    /// watching-zenoh RFC §synth-5-E cross-ref:
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

    /// watching-zenoh RFC §synth-5-E application-layer
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
    /// Schema locality choice: the
    /// stage pool is a *link* property, co-located with rx_pool /
    /// tx_pool on the `<scxml sce:kind="link">` document, not a
    /// deploy-yaml binding property. The
    /// `BindingConfig.stage_pool` field is a
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

    /// watching-zenoh RFC §synth-5-E application-layer
    /// ownership diagnostic (spec lines 1516-1519): an
    /// `<sce:on-sample callback="rust:crate::path::fn">` attribute
    /// carries an authoring path that fails the Rust
    /// path subset. Today's reachable arms are path-syntax failures
    /// (unknown language prefix, leading/trailing/double `::`,
    /// non-NCName segment, empty path); future signature inspection
    /// extends the same diagnostic code with shape-mismatch arms
    /// (owned-mode first parameter rejected at the SCE-side parser
    /// when a consumer needs it).
    ///
    /// Diagnostic name preserves spec wording
    /// verbatim; the `reason` field
    /// disambiguates the per-instance message so authors see the
    /// exact path-syntax mistake rather than generic
    /// "callback-signature-non-borrow" wording.
    #[error(
        "state '{state_id}': <sce:on-sample link=\"{link}\" callback=\"{callback}\"> {reason}. \
         The `callback` value must match the `rust:crate::module::fn` path \
         subset. The borrow-mode contract is enforced at the dispatch site; rustc rejects \
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

    /// watching-zenoh RFC §synth-5-I `<sce:extern>` whitelist rejection
    /// (spec line 1847): `<sce:extern name="...">` references a
    /// symbol absent from the §synth-5-I baseline registry. `candidates`
    /// rides `Fix::ReplaceOneOf` so authors see closest-match
    /// suggestions without paging through 101 baseline entries.
    /// Parse-time rejection; closed-set membership
    /// follows the `LinkLinkClassUnknown` precedent.
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

    /// watching-zenoh RFC §synth-5-I `<sce:extern abi="...">` mismatch
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

    /// watching-zenoh RFC §synth-5-I `<sce:extern sig="...">` mismatch
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

    /// watching-zenoh RFC §synth-5-I atomic-family ordering-suffix omission
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

    /// watching-zenoh RFC §synth-5-I target-plugin baseline-shadowing
    /// (spec line 1852 verbatim): a target plugin YAML
    /// (`extern_symbols.target_plugin: <path>`) declares a `name` that
    /// already appears in the §synth-5-I baseline registry.
    /// Additive-composition rule: plugins extend, never override; a
    /// platform-specific impl plugs in via the registry entry's
    /// `crate` field on a differently-named symbol. Repair is
    /// non-algorithmic — the plugin author renames the conflicting
    /// entry to a non-baseline name; SCE cannot synthesize a
    /// candidate. `fix: None` per the wire contract.
    #[error(
        "target plugin {plugin_path} redefines core whitelist symbol `{name}`. Plugin entries extend the §5.I baseline registry but cannot override it (additive composition — extend, never override). Rename the plugin entry to a name not already in the §5.I baseline; for a platform-specific impl, declare the entry under a vendor-prefixed name (e.g. `sce_hw_<symbol>`) and route through the registry entry's `crate` field."
    )]
    ExternTargetPluginSymbolConflict {
        /// Symbol name declared by both the plugin and the baseline.
        name: String,
        /// Plugin file path (deploy-relative or absolute) for source
        /// location surfacing in diagnostic.
        plugin_path: String,
    },

    /// watching-zenoh RFC §synth-5-D line 911 — worker kind cannot reach
    /// other workers' state through any path other than its own inbox.
    /// Two static recognition layers are implemented: layer 1 rejects
    /// `<sce:import kind="worker">` siblings inside a worker document
    /// (workers must not import other workers' kinds — encapsulation
    /// boundary); layer 2 rejects SCXML body data-refs whose namespace
    /// prefix names a foreign owner (not the worker's own name, not
    /// `_event` / `_data` / `_name` / `_iolocation`, not the declared
    /// `<sce:outbox ref="...">` target). Layer 3 — `<sce:extern>`
    /// non-inbox symbol use in the body — couples to the §synth-5-I
    /// intrinsic-registry composition and is not implemented until a
    /// consumer needs it; spec line 911 phrasing "any non-inbox
    /// access" covers all three layers together.
    ///
    /// Fires at parse time; the
    /// per-instance payload carries which layer detected the
    /// violation so the diagnostic message can name the exact path-
    /// syntax mistake. RFC §synth-5-D line 911 spec anchor.
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

    /// watching-zenoh RFC §synth-5-D cross-resolution. The worker's
    /// `<sce:link-rx ref="X">` names `X` that does not resolve to a
    /// `<sce:import as="X" kind="link">` declaration on this worker
    /// document. `validate_link_pool_framer_resolution` precedent: a
    /// worker driven by a link kind must declare the link via
    /// `<sce:import>` so cross-resolution within
    /// `compile_forge_with_imports` can confirm shape compatibility
    /// before codegen. Closed candidate list rides `Fix::ReplaceOneOf`
    /// with the sorted set of link-kind import aliases (mirroring the
    /// link-class closest-match suggestions). Non-spec diagnostic:
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

    /// watching-zenoh RFC §synth-5-I line 1757-1758 — `<sce:inbox>` declared
    /// without an `ordering` attribute. Spec phrasing labels this a
    /// "warning, codegen defaults to acquire/release"; SCE's error-only
    /// wire surface (no severity dimension yet) realizes the warning as
    /// a required-when-worker-exists error: the author must explicitly
    /// pick `ordering="acq_rel"` or `ordering="relaxed"`. The choice
    /// changes the emitted atomic operations on head/tail indices in
    /// both Rust + C11 codegen, so silent default is risk-prone on a
    /// cross-core multi-MCU target. Diagnostic name preserves spec
    /// wording verbatim.
    #[error(
        "worker '{worker_name}': <sce:inbox> declared without an `ordering` attribute. \
         Pick `ordering=\"acq_rel\"` (safe default; producer and consumer pair head/tail with acquire+release on every push/pop) or `ordering=\"relaxed\"` (single-core fast-path; cross-core placement raises `worker/inbox-ordering-relaxed-across-cores`). Spec §5.I line 1752-1758 mandates one of these two for every SPSC inbox."
    )]
    WorkerInboxOrderingUnspecified {
        /// The worker document whose `<sce:inbox>` lacks ordering.
        /// Anchored at the `<sce:inbox>` node by `located()`.
        worker_name: String,
    },

    /// watching-zenoh RFC §synth-5-I line 1755-1756 — `<sce:inbox
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

    /// watching-zenoh RFC §synth-5-D line 912
    /// (`worker/scheduler-unsupported`) — a Worker doc reached
    /// [`crate::compile_forge_with_deploy`] but the resolved target
    /// machine does not list it under `machines.<m>.workers`. The
    /// cooperative scheduler tracks one tick slot per declared worker;
    /// an undeclared worker has no slot, so codegen would emit a
    /// worker the scheduler cannot account for. The deploy-side anchor
    /// for the slot-count sum check is
    /// [`crate::mesh::error::DeployError::SchedulerIncompatibleWithWorkerCount`]
    /// (spec §synth-5-K line 2423); the forge-side anchor here fires on the
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

    /// watching-zenoh RFC §synth-5-D worker-outbox cross-resolution —
    /// `<sce:outbox ref="X">`
    /// names an owner segment (`X.split('.').next()`) that does not
    /// resolve to a recorded statechart or worker doc in the build's
    /// [`crate::forge::cross_doc_registry::SceCrossDocRegistry`].
    /// Both statechart and worker recipients are admitted per spec line
    /// 911 ("any non-inbox access" admits inbox access regardless of
    /// owner kind). The failure axis splits: this code
    /// fires on owner-not-in-registry; [`Self::WorkerOutboxTargetWrongKind`]
    /// fires when the owner resolves but to an incompatible kind (e.g.
    /// link kind); [`Self::WorkerOutboxTargetSuffixInvalid`] fires on
    /// suffix !=  `inbox` per the strict-suffix rule.
    ///
    /// Closed candidate list rides `Fix::ReplaceOneOf` with the sorted
    /// union of statechart + worker doc names (each suffixed with
    /// `.inbox` so the candidate strings are drop-in replacements for
    /// the entire `ref` attribute). Precedent:
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

    /// watching-zenoh RFC §synth-5-D worker-outbox cross-resolution —
    /// `<sce:outbox ref="X">`
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

    /// watching-zenoh RFC §synth-5-D worker-outbox cross-resolution —
    /// `<sce:outbox ref="X">`
    /// declares a suffix !=  `inbox`, violating the
    /// strict-suffix rule. Spec line 895 example writes
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

    /// watching-zenoh RFC §synth-5-D line 909
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

    /// watching-zenoh RFC §synth-5-L line 2559
    /// (`collection/ordering-sorted-requires-index-by`) — a
    /// `<sce:ordering>sorted-by(index-by)</sce:ordering>` declaration
    /// without an accompanying `<sce:index-by field="..."/>` element.
    /// Spec line 2559 fixes the SortedByIndex iteration order to the
    /// `index-by` field; without that field there is no comparator the
    /// codegen can lower. Parse-time structure check.
    #[error(
        "bounded-collection '{collection_name}': <sce:ordering>sorted-by(index-by)</sce:ordering> declared without <sce:index-by field=\"...\"/>. \
         watching-zenoh RFC §5.L line 2559 fixes sorted iteration to the `index-by` field; without it the codegen has no comparator to lower. \
         Repair: add an `<sce:index-by field=\"FIELD\"/>` element naming a field of the element-type struct, or change `<sce:ordering>` to `insertion`."
    )]
    CollectionOrderingSortedRequiresIndexBy {
        /// Bounded-collection name from `<scxml sce:kind="bounded-collection" name="...">`.
        collection_name: String,
    },

    /// watching-zenoh RFC §synth-5-L line 2655
    /// (`collection/overflow-policy-oldest-wins-requires-ordering-insertion`)
    /// — `<sce:on-overflow>oldest-wins</sce:on-overflow>` declared
    /// together with `<sce:ordering>sorted-by(index-by)</sce:ordering>`.
    /// Spec line 2655 lists this combination as the explicit anti-
    /// pattern: the `oldest-wins` policy presumes a temporal ordering
    /// (insertion timestamp) that `sorted-by` mode replaces with the
    /// `index-by` field comparator, so "oldest" has no defined meaning.
    /// Parse-time structure check.
    #[error(
        "bounded-collection '{collection_name}': <sce:on-overflow>oldest-wins</sce:on-overflow> requires <sce:ordering>insertion</sce:ordering>, but ordering is `sorted-by(index-by)`. \
         watching-zenoh RFC §5.L line 2655 lists this combination as the explicit anti-pattern: `oldest-wins` presumes a temporal ordering that `sorted-by` replaces with the `index-by` field comparator. \
         Repair: change `<sce:ordering>` to `insertion` (keeps the oldest-wins policy), or change `<sce:on-overflow>` to `reject` / `diagnostic-event`."
    )]
    CollectionOverflowPolicyOldestWinsRequiresOrderingInsertion {
        /// Bounded-collection name from `<scxml sce:kind="bounded-collection" name="...">`.
        collection_name: String,
    },

    /// watching-zenoh RFC §synth-5-M line 2944
    /// (`mem/reassembly-pool-variant-missing-max-fragments`) —
    /// `<sce:variant>reassembly</sce:variant>` declared on a buffer-pool
    /// without an accompanying `<sce:max-fragments-per-message>` sibling.
    /// Spec line 2688 fixes the per-slot fragment-index bitmap width to
    /// this value; without it codegen has no upper bound on the per-slot
    /// fragment-ID tracking. Parse-time structure check.
    #[error(
        "buffer-pool '{pool_name}': <sce:variant>reassembly</sce:variant> declared without <sce:max-fragments-per-message>N</sce:max-fragments-per-message>. \
         watching-zenoh RFC §5.M line 2688 fixes the per-slot fragment-index bitmap width to this value; without it codegen has no upper bound on the per-slot fragment-ID tracking. \
         Repair: add an `<sce:max-fragments-per-message>N</sce:max-fragments-per-message>` element with a positive integer N derived from the wire framer's per-message maximum."
    )]
    MemReassemblyPoolVariantMissingMaxFragments {
        /// Buffer-pool name from `<scxml sce:kind="buffer-pool" name="...">`.
        pool_name: String,
    },

    /// watching-zenoh RFC §synth-5-M line 2945
    /// (`mem/reassembly-pool-variant-missing-timeout`) —
    /// `<sce:variant>reassembly</sce:variant>` declared on a buffer-pool
    /// without an accompanying `<sce:reassembly-timeout-ms>` sibling.
    /// Spec line 2689 + line 2696 fix the per-slot deadline field to
    /// this value; without it the reassembly FSM has no
    /// `Receiving → TimedOut` edge timer (`docs/reassembly-fsm.md`
    /// §2.4.5). Parse-time structure check.
    #[error(
        "buffer-pool '{pool_name}': <sce:variant>reassembly</sce:variant> declared without <sce:reassembly-timeout-ms>N</sce:reassembly-timeout-ms>. \
         watching-zenoh RFC §5.M line 2689 fixes the per-slot deadline field to this value; without it the reassembly FSM has no `Receiving → TimedOut` edge timer (`docs/reassembly-fsm.md` §2.4.5). \
         Repair: add an `<sce:reassembly-timeout-ms>N</sce:reassembly-timeout-ms>` element with a positive integer N (milliseconds) derived from link latency budget and acceptable hold time."
    )]
    MemReassemblyPoolVariantMissingTimeout {
        /// Buffer-pool name from `<scxml sce:kind="buffer-pool" name="...">`.
        pool_name: String,
    },

    /// watching-zenoh RFC §synth-5-M line 2946
    /// (`mem/reassembly-slot-size-below-declared-mtu`) — an `<sce:rx-pool
    /// ref>` binding resolved to a buffer-pool whose `<sce:slot-size>`
    /// is smaller than the bound link's `mtu_bytes`. The slot cannot
    /// hold a single full-MTU datagram; even the non-fragmented happy
    /// path fails to admit one wire frame. Cross-doc consumer
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

    /// watching-zenoh RFC §synth-5-M line 2947-2949
    /// (`reassembly/max-fragments-insufficient-for-mtu`) — reassembly-
    /// variant pool's `<sce:slot-size>` cannot hold the worst-case
    /// reassembled message implied by `<sce:max-fragments-per-message>`
    /// and the bound link's `mtu_bytes`. Spec invariant verbatim:
    /// `slot_size >= max-fragments-per-message × mtu_bytes`. Hard
    /// error. Cross-doc consumer.
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

    /// watching-zenoh RFC §synth-5-M line 2950-2952
    /// (`reassembly/expected-fragmentation-rate-high`) — the bound
    /// link's `expected_p99_bytes` exceeds the regular RX pool's
    /// `<sce:slot-size>` such that more than 25% of inbound traffic
    /// would run the ARCHITECTURE §9.3 stage-copy path. Default
    /// warning per spec (suppressible via
    /// `<sce:accept-stage-copy-rate>` on the link source, gated by
    /// the deploy `stage_copy_policy`). Silent-skip when no regular
    /// `BufferPoolVariant::Default`
    /// pool is bound (the formula references "the
    /// regular RX pool's slot_size" which does not exist for the link).
    #[error(
        "link '{link_name}' on machine '{machine}': `expected_p99_bytes: {expected_p99_bytes}` exceeds RX pool '{pool_name}' `<sce:slot-size>{slot_size}</sce:slot-size>` by more than the 25% default stage-copy threshold (rate = {rate_percent}%). \
         watching-zenoh RFC §5.M line 2950-2952 — `(expected_p99_bytes - rx_pool.slot_size) / expected_p99_bytes > 0.25` triggers the warning. \
         Repair: raise `<sce:slot-size>` on pool '{pool_name}', lower `expected_p99_bytes` (with justification), or add `<sce:accept-stage-copy-rate>` on the link source."
    )]
    ReassemblyExpectedFragmentationRateHigh {
        pool_name: String,
        slot_size: u32,
        expected_p99_bytes: u32,
        rate_percent: u32,
        machine: String,
        link_name: String,
    },

    /// watching-zenoh RFC §synth-5-M line 2964-2969
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

    /// watching-zenoh RFC §synth-5-M line 2970-2975
    /// (`reassembly/trust-class-missing-on-fragmenting-link`) —
    /// reassembly-variant pool bound to a link whose `domain_attrs`
    /// block is absent entirely. Build cannot
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

    /// watching-zenoh RFC §synth-5-M line 2995-2999
    /// (`reassembly/stage-copy-wcet-exceeds-slot-budget`) — the
    /// implicit memcpy in the stage-copy path alone blows the
    /// cooperative slot. Formula verbatim: `expected_p99_bytes ×
    /// memcpy_cycles_per_byte / clock_freq_mhz > worker_slot_budget_us`.
    /// Silent-skip when any of the four platform/scheduler inputs
    /// absent (deploy-aware silent-skip precedent).
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

    /// watching-zenoh RFC §synth-5-M line 2976-2981 verbatim
    /// (`reassembly/peer-id-not-zid-on-established-session`) — internal
    /// codegen invariant: per-peer quota check on an
    /// `established_session` link must use ZID (handshake-derived) as
    /// the peer key, not the wire source address. Codegen guard
    /// against template regression that would silently fall back to
    /// spoofable wire ID.
    ///
    /// Wired as a post-render substring check inside
    /// [`render_buffer_pool_rust`] / [`render_buffer_pool_c`] when the
    /// resolved variant is [`BufferPoolVariant::Reassembly`]: the
    /// emitted output must contain the 16-byte ZID peer-id signature.
    /// In normal use the template always emits the ZID shape (the
    /// reassembly variant only resolves for `established_session`
    /// bindings — the cross-doc validator
    /// `reassembly/untrusted-link-binding` rejects any other
    /// trust class). The diagnostic exists to catch a future
    /// template edit that drops the ZID type or substitutes a
    /// wire-source typedef — mirrors the
    /// `BufferPoolInterPoolPaddingNotEmitted` self-check shape per
    /// generator.rs:10225.
    #[error(
        "reassembly-variant buffer-pool '{pool_name}' ({language} backend): emitted per-slot peer-id is not the 16-byte ZID signature required for `trust_class: established_session` bindings. \
         watching-zenoh RFC §5.M line 2976-2981 — codegen invariant violation: per-peer quota check must use the handshake-derived ZID as the peer key, not the wire source address (defends against UDP source-IP spoofing on `established_session` links). \
         In well-formed templates the reassembly variant always emits the 16-byte ZID typedef (the cross-doc validator `reassembly/untrusted-link-binding` gates non-`established_session` bindings upstream), so this diagnostic fires only on template regression; report at https://github.com/newmassrael/scxml-core-engine/issues"
    )]
    ReassemblyPeerIdNotZidOnEstablishedSession { pool_name: String, language: String },

    /// watching-zenoh RFC §synth-5-C lines 849-856 verbatim
    /// (`link/listener-link-not-paired-with-established-sibling`) —
    /// codegen self-check that every `session_arming` listener
    /// instance has emitted its `established_session` sibling per the
    /// "Listener-link sibling emission" contract above (RFC §synth-5-C
    /// lines 799-833). Hard error. Template regression guard,
    /// unreachable in well-formed codegen; it exists to ensure the
    /// listener emission template cannot silently regress to single-
    /// instance shape (which would re-introduce the unstable
    /// per-peer dispatch identity contradiction).
    ///
    /// Wired as a post-render substring check inside
    /// [`super::generator::render_link_rust`] +
    /// [`super::generator::render_link_c`] when
    /// [`crate::ForgeCompileOptions::listener_links`] contains the
    /// rendered link's name: the emitted output must carry the
    /// durable Sibling type-name suffix (`EstablishedSession` for
    /// Rust, `_established_session_t` for C11). In normal use the
    /// template always emits both halves under the orchestrator
    /// flag; the diagnostic exists to catch a future template edit
    /// that drops the Sibling block — mirrors the
    /// `BufferPoolInterPoolPaddingNotEmitted` /
    /// `ReassemblyPeerIdNotZidOnEstablishedSession` self-check shape
    /// per generator.rs:10225.
    #[error(
        "link '{link_name}' ({language} backend): listener-link sibling emission missing the `established_session` half. \
         watching-zenoh RFC §5.C lines 849-856 — codegen invariant violation: every `session_arming` listener must emit its paired `established_session` sibling so per-peer dispatch retains a stable codegen-time identity (re-introduces OQ-W22 if dropped). \
         In well-formed templates the diagnostic never fires (the per-language link template emits both halves unconditionally when `listener_links` contains this name); report at https://github.com/newmassrael/scxml-core-engine/issues"
    )]
    LinkListenerLinkNotPairedWithEstablishedSibling { link_name: String, language: String },

    /// watching-zenoh RFC §synth-5-M lines 2982-2994 verbatim
    /// (`reassembly/binding-on-unpaired-listener`) — a reassembly-
    /// pool binding has resolved to a `session_arming` link instance
    /// whose paired `established_session` sibling does not exist.
    /// Hard error. In well-formed codegen this is unreachable (the
    /// listener-link sibling emission contract in §synth-5-C guarantees
    /// pairing); the diagnostic guards SCXML that explicitly targets
    /// the `session_arming` half (bypassing the auto-resolution) and
    /// any future schema evolution that introduces non-listener
    /// `session_arming` instances. Distinct from
    /// `reassembly/untrusted-link-binding` (which now fires
    /// only on Untrusted bindings) and from
    /// `link/listener-link-not-paired-with-established-sibling`
    /// (which is the §synth-5-C link-side codegen self-check).
    ///
    /// Wired inside
    /// [`crate::mesh::deploy::validate_reassembly_cross_doc`]:
    /// when the bound link's trust_class is
    /// `session_arming` AND the orchestrator-resolved listener-link
    /// set does NOT contain the link name (i.e. the deploy link has
    /// no `role: listener` AND/OR the machine's source SCXML has no
    /// `<sce:session-role kind="accept-side"/>` declaration — the
    /// implicit `Accepting.*` pattern walker has been deleted in
    /// favor of the explicit role pair), the validator fires this
    /// code in place of the historic
    /// `reassembly/untrusted-link-binding` for the session-arming
    /// subcase. NeutralOrDeterministic — two valid repair paths:
    /// declare the explicit listener-pair role on both sides
    /// (making the link a real listener so the sibling auto-
    /// synthesizes), or remove the reassembly-pool binding.
    #[error(
        "reassembly-variant buffer-pool '{pool_name}' is bound to link '{link_name}' on machine '{machine}'; the link declares `trust_class: session_arming` but its machine source SCXML did not pair with a listener-role declaration (deploy `role: listener` + SCXML `<sce:session-role kind=\"accept-side\"/>`), so codegen cannot synthesize the paired `established_session` sibling. \
         watching-zenoh RFC §5.M lines 2982-2994 — only listeners (the explicit deploy/SCXML role pair) auto-rebind a `session_arming` reassembly binding to the `established_session` sibling; without that pairing the binding has no valid landing site. \
         Repair: declare `role: listener` on the deploy link AND add `<sce:session-role kind=\"accept-side\"/>` to machine '{machine}'s source SCXML (making link '{link_name}' a real listener so the sibling auto-synthesizes), or remove the reassembly-pool binding from link '{link_name}'."
    )]
    ReassemblyBindingOnUnpairedListener {
        pool_name: String,
        machine: String,
        link_name: String,
    },

    /// watching-zenoh RFC §synth-5-N line 3062 verbatim
    /// (`link/inbound-event-queue-unsized`) — cross-doc
    /// orchestrator-level check. A `<sce:link>` declares
    /// `<sce:inbound event="X"/>` rows but no FSM event-queue
    /// capacity reaches the machine that imports the link. Two
    /// acceptable sources: SCXML per-instance
    /// `<scxml sce:capacity="N">` (preferred) or deploy
    /// `machines.<m>.scheduler.default_event_queue_capacity`
    /// (fallback). Both absent ⇒ no compile-time bound on the
    /// downstream queue depth.
    ///
    /// Wired inside
    /// [`crate::compile_scxml_with_imports`] pass-2 after the
    /// reassembly + listener-pair validators (matching the
    /// orchestrator-level cross-doc
    /// precedent). Silent-skip when the link has no inbound events
    /// declared or when no SCXML imports the link.
    /// NeutralOrDeterministic — two-axis repair (per-instance vs
    /// per-machine size source).
    #[error(
        "link '{link_name}' on machine '{machine}': declares {inbound_event_count} inbound event(s) but no downstream FSM event-queue capacity is bound. \
         watching-zenoh RFC §5.N line 3062 — link declared but downstream FSM inbox depth unset. \
         Repair: add `<scxml sce:capacity=\"N\">` to machine '{machine}'s source SCXML (per-instance), or add `scheduler.default_event_queue_capacity: N` under `machines.{machine}` (per-machine fallback)."
    )]
    LinkInboundEventQueueUnsized {
        machine: String,
        link_name: String,
        inbound_event_count: u32,
    },

    /// watching-zenoh RFC §synth-5-K line 2504-2511 verbatim
    /// (`pool/stage-copy-policy-error`). `pool_defaults.stage_copy_policy:
    /// error` (or `forbid`) AND the §synth-5-M / ARCHITECTURE §9.3
    /// stage-copy-rate gate fires; the warning is promoted to hard
    /// error. Deploy-aware consumer of
    /// `MachineConfig::resolved_stage_copy_policy`.
    #[error(
        "link '{link_name}' on machine '{machine}': `expected_p99_bytes: {expected_p99_bytes}` vs RX pool '{pool_name}' `<sce:slot-size>{slot_size}</sce:slot-size>` triggers stage-copy rate {rate_percent}% (> 25% threshold), promoted to hard error under `pool_defaults.stage_copy_policy: {policy}`. \
         watching-zenoh RFC §5.K line 2504-2511 — author resolution: raise `<sce:slot-size>` on pool '{pool_name}', lower `expected_p99_bytes`, or add `<sce:accept-stage-copy-rate>` on link '{link_name}' (last option unavailable under `forbid`)."
    )]
    PoolStageCopyPolicyError {
        pool_name: String,
        slot_size: u32,
        expected_p99_bytes: u32,
        rate_percent: u32,
        machine: String,
        link_name: String,
        policy: String,
    },

    /// watching-zenoh RFC §synth-5-K line 2512-2516 verbatim
    /// (`pool/stage-copy-accept-rejected-under-forbid`).
    /// `pool_defaults.stage_copy_policy: forbid` AND the link source
    /// carries `<sce:accept-stage-copy-rate>`. The opt-out itself is
    /// rejected outright regardless of whether the rate threshold is
    /// exceeded (spec contract is the element's mere presence under
    /// `forbid` is the violation).
    #[error(
        "link '{link_name}' on machine '{machine}': `<sce:accept-stage-copy-rate>` declared but `pool_defaults.stage_copy_policy: forbid` rejects the opt-out outright. \
         watching-zenoh RFC §5.K line 2512-2516 — only structural fixes (raise `<sce:slot-size>` or lower `expected_p99_bytes`) are accepted under `forbid`. \
         Repair: remove `<sce:accept-stage-copy-rate>` from link '{link_name}', or change `pool_defaults.stage_copy_policy` to `error` (which permits the opt-out)."
    )]
    PoolStageCopyAcceptRejectedUnderForbid { machine: String, link_name: String },

    /// watching-zenoh RFC §synth-5-L lines 2566-2567 +  2650
    /// (`collection/element-type-not-a-kind`) — `<sce:element-type>NAME`
    /// body text does not resolve in the build's forge-doc registry to
    /// a codec-kind struct (§synth-5-B) or procedure-kind state record. The
    /// parser stores the body text as an opaque `String`; the
    /// cross-doc pass consumes the
    /// orchestrator-assembled element-type candidate map
    /// (`HashMap<String, ForgeDocument>` populated only for codec +
    /// procedure docs during pass-1 of
    /// [`crate::compile_scxml_with_imports`]) and either fires this
    /// code (name absent from the map OR present but with an
    /// incompatible kind — both surface as the same code per the
    /// `*RefUnknown` + `*WrongKind` precedent that splits axes only
    /// when repair surfaces differ; here both axes share the closed
    /// candidate set so the single code suffices) or returns Ok.
    ///
    /// Closed candidate list rides `Fix::ReplaceOneOf` with the sorted
    /// union of registered codec + procedure doc names. Precedent:
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

    /// watching-zenoh RFC §synth-5-L line 2615 + 2651
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

    /// watching-zenoh RFC §synth-5-L lines 2560-2562 + 2652
    /// (`collection/multi-writer-without-atomics`) —
    /// `<sce:concurrency>multi-writer</sce:concurrency>` declared on a
    /// bounded-collection without any §synth-5-I atomic intrinsic having been
    /// imported via `<sce:extern>` anywhere in the build. The spec
    /// fixes multi-writer to "acquire/release atomics on head/tail",
    /// so the build's `<sce:extern>` trust-surface must acknowledge
    /// atomic intrinsics for codegen to legitimately emit them.
    ///
    /// The check is build-wide cross-doc: pass-1 of
    /// [`crate::compile_scxml_with_imports`] aggregates every parsed
    /// forge doc's `externs` into a single slice; the
    /// validator scans for any entry whose registry-resolved purpose
    /// starts with `"atomic-"` (the §synth-5-I baseline registry
    /// tags atomic-load / atomic-store / atomic-cas-* / atomic-fetch-*
    /// uniformly via the [`crate::forge::intrinsic_registry::Symbol::purpose`]
    /// field). At least one such declaration anywhere in the build
    /// allows multi-writer; zero declarations fires this code.
    ///
    /// No closed candidate set — the §synth-5-I baseline registry's atomic
    /// family is too large (≥101 spans load/store/cas/fetch ×
    /// 5 widths × multiple orderings) for a useful
    /// `Fix::ReplaceOneOf`; author judgment chooses the right ordering +
    /// width. NeutralOrDeterministic non_overlap_class with
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

    /// watching-zenoh RFC §synth-5-L lines 2583-2585 + 2649
    /// (`collection/capacity-unresolved`) — `<sce:capacity source="deploy"
    /// key="machines.<machine>.limits.<limit>"/>` names a deploy-key
    /// whose `<limit>` segment is not declared under
    /// `machines.<machine>.limits:` in deploy.yaml. The codegen must
    /// lower the capacity into a per-language compile-time constant
    /// (spec line 2570-2585), so an unresolved key blocks emit.
    ///
    /// Fires only on the [`crate::compile_forge_with_deploy`] path
    /// (deploy + target_machine both Some) per the deploy-aware silent-
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

    /// Watching-zenoh RFC §synth-5-O — IR provenance pre-emit
    /// guard. Fires when a node eligible for SCE-MAP marker emission
    /// reaches the codegen pre-emit walker with `source_location:
    /// None`. Codegen-internal invariant: authors never see this
    /// signal in practice; the fix lives in the parser site that
    /// produced the IR node. `node_kind` names the IR type
    /// (`<scxml>`, `<state>`, `<transition>`, `<action>`); `node_id`
    /// names the document-order identifier where available (state
    /// id, transition event+target, or auto-id) so the parser site
    /// is locatable from the wire payload alone.
    #[error(
        "{node_kind} '{node_id}': source_location not populated — \
         sourcemap pre-emit guard (parser site missed)"
    )]
    TraceabilityScxmlLineRangeMissing {
        node_kind: &'static str,
        node_id: String,
    },

    /// Watching-zenoh RFC §5.O — symbol-mangling collision.
    /// Fires when the cross-IR symbol-table walker (`forge::
    /// symbol_mangling::build_symbol_table`) finds two distinct IR
    /// nodes whose `(machine, state_path, artifact)` triple mangles
    /// to the same identifier. The common trigger is XInclude or
    /// `sce:template` composition that imports a state fragment whose
    /// id collides with a top-level state on the importer; the dual-
    /// location payload pins both sites so the author can resolve the
    /// clash by renaming one.
    ///
    /// `mangled` carries the colliding mangled id verbatim; `first_*` +
    /// `second_*` describe the two offending sites by file:line. The
    /// diagnostic carries no closed candidate set — the repair (rename
    /// one of the two states) is author-domain — so it rides
    /// `FixCarriesCandidates` with the empty-but-present second-site
    /// pinned (one alternative, naming the second-site path).
    #[error(
        "symbol collision: '{mangled}' maps to two IR nodes — \
         {first_file}:{first_line} and {second_file}:{second_line}. \
         Repair: rename one of the colliding ids so the mangled symbols \
         differ"
    )]
    TraceabilityStateIdCollision {
        mangled: String,
        first_file: String,
        first_line: u32,
        second_file: String,
        second_line: u32,
    },

    /// Watching-zenoh RFC §5.O — mangled symbol exceeds the
    /// C99 §5.2.4.1 external-identifier length limit (31 chars).
    /// Default rendering is warn; `platform.strict_c99_identifiers:
    /// true` in deploy.yaml escalates to hard-error. `mangled` is the
    /// offending identifier verbatim so the author can see exactly
    /// what was emitted; `over_by` carries the excess char count.
    #[error(
        "mangled symbol '{mangled}' exceeds C99 external identifier \
         limit by {over_by} char(s) (got {actual_len}, max 31). \
         Repair: shorten one of the contributing names (machine id, \
         state id, or artifact suffix) or enable \
         `platform.strict_c99_identifiers: false` in deploy.yaml to \
         suppress this warning"
    )]
    TraceabilitySymbolNameExceedsCIdentifierLimit {
        mangled: String,
        actual_len: u32,
        over_by: u32,
    },

    /// Watching-zenoh RFC §5.O — sourcemap `source_hash`
    /// drift against the §6.2.6 header `source-hash`. Codegen-
    /// invariant: every emitted `sce_sourcemap.json`'s
    /// `source_hash` field MUST be byte-equal to the
    /// `source-hash` value in the per-file §synth-6.2.6 drift header.
    /// Mismatch indicates the sourcemap was written from a stale
    /// snapshot or a manual edit; `sce-codegen addr2sce` rejects
    /// the sourcemap until the values match. No author repair —
    /// regenerate via `sce-codegen generate`.
    #[error(
        "sourcemap source_hash drift: sourcemap recorded '{sourcemap_hash}' \
         but §6.2.6 header recorded '{header_hash}' on {file}. \
         Repair: regenerate via `sce-codegen generate` to rebuild both \
         sides from the same inputs"
    )]
    TraceabilitySourcemapSourceHashMismatch {
        file: String,
        sourcemap_hash: String,
        header_hash: String,
    },

    /// Watching-zenoh RFC §synth-5-O — Rust SCE-MAP marker
    /// preservation guard. Fires from `sce-codegen
    /// addr2sce` when a rustdoc JSON dump for the generated crate
    /// contains no `#[doc = "SCE-MAP: ..."]` attribute on a function
    /// whose sourcemap entry says one should exist. The empirical
    /// preservation test catches both `--release` strip mishaps and
    /// future rustdoc behaviour changes. Author repair is to fall
    /// back to the `// SCE-MAP:` line-comment form via the dual-emit
    /// path (the default emission shape); this diagnostic
    /// signals the fallback is needed.
    #[error(
        "SCE-MAP `#[doc]` marker stripped from '{function}' in \
         {crate_name} ({profile}); falling back to `// SCE-MAP:` \
         line comments. Repair: re-emit with the dual-marker form \
         (the default dual-marker form) or upstream the rustdoc fix"
    )]
    TraceabilitySceMapAttributeStripped {
        crate_name: String,
        function: String,
        profile: String,
    },

    /// Watching-zenoh RFC §synth-5-O — codegen-internal
    /// traceability invariant: every SCE-emitted file (one carrying a
    /// `// SCE-GENERATED` drift header per §synth-6.2.6) MUST contain at
    /// least one `SCE-MAP:` marker line. The two artefacts ship
    /// together — the templates that populate the markers are the
    /// same ones the sourcemap JSON fingerprints —
    /// so a drift-headered file with no marker indicates a template
    /// edit dropped the marker macro call without anyone noticing.
    /// ARCHITECTURE.md "Traceability Ownership Boundary" pins the
    /// scope: only files SCE emits directly are subject to this
    /// invariant; external meta-generator output (protoc, bindgen,
    /// cbindgen, capnproto, hand-authored sources) carries no drift
    /// header and is silently out-of-scope.
    ///
    /// Author repair is empty: this is an SCE codegen-pipeline bug,
    /// not a document bug. The fix is upstream in
    /// `tools/codegen/templates/_macros/sce_map_marker.jinja2`'s
    /// callers — re-add the missing `SCE-MAP:` marker emission and
    /// the invariant repairs.
    #[error(
        "emitted file '{file}' carries a §6.2.6 drift header but no \
         `SCE-MAP:` marker line. Per ARCHITECTURE.md \"Traceability \
         Ownership Boundary\", every SCE-emitted file must carry at \
         least one marker. Repair: a template under \
         `tools/codegen/templates/` is missing its \
         `sce_map_marker` macro call — report upstream"
    )]
    TraceabilityMetaGeneratedSourceLineMarkerMissing { file: String },

    /// A `<sce:driver href="..."/>`
    /// reference cannot be resolved against `deploy.yaml`'s
    /// `platform.driver_root` (or the SCXML file's parent directory
    /// as fallback). The referenced header is the author's contract
    /// with the C11 backend: `*_sm.c` `#include`s the resolved path,
    /// so absence breaks cross-TU symbol resolution before any C
    /// compiler can speak up. The diagnostic fires at compile-model
    /// time, before codegen.
    ///
    /// Repair is author-domain — fix the `href` value, add the
    /// missing file, or set `platform.driver_root` so the relative
    /// path resolves. No closed candidate set; NeutralOrDeterministic.
    #[error(
        "driver header reference '{href}' could not be resolved \
         (searched under '{resolved_dir}'). Repair: correct the \
         `<sce:driver href=\"...\"/>` value, add the missing header, \
         or set `platform.driver_root` in deploy.yaml so the relative \
         path resolves."
    )]
    McuDriverHeaderNotFound {
        /// Author-written `href` value, verbatim from the SCXML.
        href: String,
        /// Directory the resolver searched (resolved root or SCXML
        /// file's parent).
        resolved_dir: String,
    },

    /// Listener session-role declarations — a `<sce:session-role kind="..."/>`
    /// element on the SCXML root nominates a kind value outside the
    /// v1 closed set ([`crate::model::SessionRoleKind::all_wire_names`]).
    /// Fires at parse time so the author surfaces the typo before any
    /// downstream cross-doc validator runs.
    ///
    /// Repair surface: pick a kind value from the embedded vocabulary
    /// list. The `Fix::ReplaceOneOf` payload carries the v1 set, so
    /// CLI / IDE consumers can surface a closed-set picker without
    /// scraping the message body. Future kind variants extend the
    /// vocabulary in lockstep with their codegen-side semantics.
    #[error(
        "<sce:session-role kind=\"{kind}\"/>: unknown session-role kind. \
         v1 vocabulary: {allowed:?}. Repair: change `kind` to one of the \
         listed values or remove the element if no session-FSM role applies."
    )]
    ScxmlUnknownSessionRoleKind {
        /// Author-written `kind` attribute value, verbatim from the SCXML.
        kind: String,
        /// Closed-set vocabulary list — copy of
        /// [`crate::model::SessionRoleKind::all_wire_names`] as
        /// `Vec<String>` so the variant lives in a non-`'static` arm.
        /// Powers the `Fix::ReplaceOneOf` candidate list.
        allowed: Vec<String>,
    },

    /// Listener session-role declarations — the same session-role kind appears
    /// in two distinct `<sce:session-role kind="..."/>` declarations on
    /// one SCXML document root. Set semantics: multiple
    /// distinct kinds are permitted, but duplicates of one kind are
    /// not — the author intent is undefined and silently keeping only
    /// one would obscure the authoring mistake.
    ///
    /// Repair: delete the duplicate declaration. The element has no
    /// payload beyond `kind`, so two declarations with the same kind
    /// carry no extra information.
    #[error(
        "<sce:session-role kind=\"{kind}\"/>: declared more than once on this SCXML \
         document. Each session-role kind may appear at most once per document. \
         Repair: delete the duplicate `<sce:session-role kind=\"{kind}\"/>` element."
    )]
    ScxmlDuplicateSessionRoleDeclaration {
        /// Kind that was declared twice. Stable wire-form string from
        /// [`crate::model::SessionRoleKind::as_str`] so the diagnostic
        /// echo matches the original SCXML attribute value verbatim.
        kind: String,
    },

    /// Listener-role partial-claim (typed per direction) —
    /// a deploy.yaml link declares `role: listener`
    /// but its machine's source SCXML carries no
    /// `<sce:session-role kind="accept-side"/>` declaration. The
    /// implicit-claim hazard that the historic `Accepting.*` substate
    /// pattern walker silently degraded on — now typed.
    ///
    /// Repair: add `<sce:session-role kind="accept-side"/>` to the
    /// machine's source SCXML if it implements the canonical session-
    /// FSM accept-side state machine, OR remove `role: listener` from
    /// the deploy link config if the link is not a listener half.
    /// `NeutralOrDeterministic` non_overlap class (2-axis repair).
    #[error(
        "deploy machine '{machine}' link '{link_name}': declares `role: listener` but \
         its source SCXML carries no `<sce:session-role kind=\"accept-side\"/>` top-level \
         declaration. Repair: add `<sce:session-role kind=\"accept-side\"/>` to the SCXML \
         root if it implements the session-FSM accept-side, OR remove `role: listener` \
         from the deploy link if the link is not a listener half."
    )]
    LinkDeployRoleListenerWithoutScxmlAcceptSideRole {
        /// Deploy `machines.<n>` that declared the listener role.
        machine: String,
        /// `<sce:link name="X">` body name. Same shape as the
        /// `reassembly/binding-on-unpaired-listener` `key_fragments`
        /// quoting (item C10 listener-pair precedent) so external consumers can join
        /// diagnostic streams on `(machine, link_name)`.
        link_name: String,
    },

    /// Listener-role partial-claim (typed per direction) —
    /// an SCXML doc declares
    /// `<sce:session-role kind="accept-side"/>` but no deploy.yaml
    /// link on the machine that sources this SCXML has
    /// `role: listener`. The mirror direction of the
    /// `LinkDeployRoleListenerWithoutScxmlAcceptSideRole` partial-
    /// claim.
    ///
    /// Repair: declare `role: listener` on the deploy link that hosts
    /// the accept-side handshake, OR remove the
    /// `<sce:session-role kind="accept-side"/>` element if the SCXML
    /// is not actually serving as the accept-side FSM.
    /// `NeutralOrDeterministic` (2-axis repair).
    #[error(
        "SCXML machine '{machine}' (source `{scxml_source}`): declares \
         `<sce:session-role kind=\"accept-side\"/>` but no deploy link on this machine \
         has `role: listener`. Repair: add `role: listener` to the deploy link that \
         hosts the accept-side handshake, OR remove the `<sce:session-role>` element \
         from the SCXML if it does not serve as the accept-side FSM."
    )]
    ScxmlAcceptSideRoleWithoutListenerLink {
        /// Deploy `machines.<n>` whose source SCXML carries the
        /// accept-side role declaration.
        machine: String,
        /// Source SCXML basename (deploy `machine.source` value).
        /// Surfaces in `key_fragments` so consumers can navigate to
        /// the offending SCXML file without parsing the message body.
        /// Named `scxml_source` (not `source`) to avoid thiserror's
        /// `#[source]` magic — a plain `source: String` is interpreted
        /// as a wrapped error source.
        scxml_source: String,
    },

    /// Listener-role × trust-class matrix — a deploy.yaml link
    /// declares `role: listener` but `trust_class != session_arming`.
    /// The combination is structurally invalid because the only trust
    /// tier where pre-handshake listener semantics apply is
    /// `session_arming` (see the `mesh/deploy.rs::TrustClass`
    /// doc). The new explicit `role`
    /// field decouples the listener-role declaration from the trust
    /// tier by design — so an explicit
    /// `role: listener` + `trust_class: untrusted` combination must
    /// be rejected eagerly rather than silently dropped.
    ///
    /// Repair: flip `trust_class` to `session_arming` if the link
    /// genuinely carries pre-handshake traffic, OR remove
    /// `role: listener` if the link is on a different trust tier.
    /// `NeutralOrDeterministic` (2-axis repair).
    #[error(
        "deploy machine '{machine}' link '{link_name}': declares `role: listener` but \
         `trust_class: {trust_class}` (not `session_arming`). The listener-role \
         declaration applies only to pre-handshake traffic, which lives on the \
         `session_arming` trust tier. Repair: change `trust_class` to `session_arming`, \
         OR remove `role: listener`."
    )]
    LinkRoleListenerWithNonSessionArmingTrustClass {
        /// Deploy `machines.<n>`.
        machine: String,
        /// `<sce:link name="X">` body name.
        link_name: String,
        /// Trust-class wire form from
        /// [`crate::mesh::deploy::TrustClass::as_str`] — stable across
        /// Rust edition / `Debug`-impl changes.
        trust_class: String,
    },

    /// Migration-helper diagnostic
    /// — an SCXML document carries any `Accepting` or `Accepting.*`
    /// state-id but no `<sce:session-role kind="accept-side"/>` top-
    /// level declaration. The canonical session-FSM state naming
    /// (`docs/session-fsm.md` §2.6) is reserved for the accept-side
    /// session FSM; using it without claiming the role is either a
    /// migration mistake (legacy SCXML not yet declaring
    /// the role) or a naming collision (an unrelated terms-acceptance
    /// flow happening to share the stem).
    ///
    /// Repair: add `<sce:session-role kind="accept-side"/>` to the
    /// SCXML root if the document genuinely implements the session-
    /// FSM accept-side, OR rename the offending state ids so they do
    /// not collide with the reserved `Accepting.*` prefix.
    /// `NeutralOrDeterministic` (2-axis repair).
    ///
    /// Invariant: the migrated test corpus is
    /// expected to be free of fixtures triggering this diagnostic.
    /// Production SCXML adopting SCE must follow the same discipline.
    #[error(
        "SCXML doc carries state ids matching the reserved `Accepting.*` prefix \
         ({offending_ids:?}) but no top-level \
         `<sce:session-role kind=\"accept-side\"/>` declaration. The canonical \
         session-FSM accept-side state names are reserved for documents that \
         claim the accept-side role. Repair: add \
         `<sce:session-role kind=\"accept-side\"/>` to the SCXML root if the doc \
         implements the session-FSM accept-side, OR rename the offending state \
         ids to avoid the `Accepting.*` reservation."
    )]
    ScxmlAcceptSideStatesWithoutRoleDeclaration {
        /// Sorted list of state ids that triggered the reserved-prefix
        /// check. Empty would not reach this variant, so the field is
        /// always non-empty at the diagnostic emit site. `Vec` not
        /// `BTreeSet` so insertion order is preserved for the
        /// diagnostic's `actual` payload — sorted-by-construction at
        /// the parser layer for determinism.
        offending_ids: Vec<String>,
    },

    /// Reassembly declared-consumption invariant (watching-zenoh RFC
    /// §synth-5-M lines 2841-2861):
    /// the build-time invariant
    /// `peer_table.capacity × per_peer_quota >= slot_count` is
    /// violated for a reassembly-variant buffer-pool bound to a
    /// session-arming link with a declared `peer_table`. Without the
    /// guarantee, a peer storm under attack can occupy more slots
    /// than the per-peer cap allows — the per-peer accounting silently
    /// degrades into shared-pool contention.
    ///
    /// Fields surface every input the invariant needs so authors
    /// repair on the appropriate axis: raise `peer_table.capacity`,
    /// raise `per_peer_quota`, or lower `slot_count`.
    /// `NeutralOrDeterministic` (3-axis repair).
    #[error(
        "reassembly-variant buffer-pool '{pool_name}' (slot_count={slot_count}) bound to \
         machine '{machine}' link '{link_name}' violates the per-peer-quota build invariant: \
         `peer_table.capacity ({peer_table_capacity}) × per_peer_quota ({per_peer_quota}) = \
         {product}` < `slot_count ({slot_count})`. RFC §5.M lines 2841-2861 — without this \
         bound a peer storm can occupy more slots than the per-peer cap permits, silently \
         degrading per-peer accounting into shared-pool contention. Repair: raise \
         `peer_table.capacity` on the link's `stateless_accept`, raise `per_peer_quota` on \
         the pool, or lower `slot_count` on the pool."
    )]
    ReassemblyPerPeerQuotaBuildInvariantViolated {
        pool_name: String,
        slot_count: u32,
        machine: String,
        link_name: String,
        peer_table_capacity: u32,
        per_peer_quota: u32,
        /// `peer_table.capacity × per_peer_quota` — surfaced verbatim
        /// so authors don't need to recompute from the other fields.
        /// `u64` to absorb the multiplication without overflow on the
        /// extreme corner (`u32::MAX × 1` is the realistic ceiling).
        product: u64,
    },

    /// `sce:req="ID1 ID2 ID2"` repeats the same requirement ID on a
    /// single node. The `sce:req` token is opaque, but
    /// duplicates would survive into req-coverage NDJSON as a phantom
    /// double-count and mask the actually-missing second annotation.
    /// Rejected so the author either drops the duplicate or splits
    /// the annotation across siblings deliberately.
    #[error("{element}: duplicate sce:req id '{id}'")]
    DuplicateRequirementId {
        /// Author-facing element label, e.g. `<state id="armed">`,
        /// `<transition>`, `<onentry>`.
        element: String,
        /// The repeated requirement id verbatim — opaque, no shape
        /// constraint enforced (SCE keeps `sce:req` tokens opaque by
        /// design).
        id: String,
    },

    /// `sce:unresolved` placeholder found while `--strict-unresolved`
    /// is in effect. The marker is a
    /// deliberate "revisit later" signal; strict mode lifts it from
    /// silent metadata to a build-failing rejection so CI gates
    /// cannot merge unresolved IR. Default (non-strict) builds pass
    /// — the marker survives only in the model and the
    /// `sce-codegen unresolved` NDJSON report.
    #[error(
        "{element}: unresolved placeholder id='{id}'{}",
        if let Some(r) = reason.as_ref() {
            format!(" reason='{r}'")
        } else {
            String::new()
        }
    )]
    UnresolvedPlaceholder {
        /// Author-facing element label, e.g. `<state id="armed">`.
        element: String,
        /// Opaque marker id from `sce:unresolved="..."` or the
        /// `<sce:unresolved id="..."/>` element.
        id: String,
        /// Optional human-readable reason carried verbatim from
        /// `sce:unresolved-reason="..."` or the element's
        /// `reason="..."` attribute.
        reason: Option<String>,
    },

    /// An expression in an importing kind references `<alias>.<field>`
    /// where `<alias>` resolves to a known `<sce:import>` declaration
    /// but `<field>` is not a member exposed by the imported kind. The
    /// `candidates` list is the imported kind's full member surface
    /// (sorted, deduplicated) so the diagnostic carries a closed
    /// `Fix::ReplaceOneOf` for `did_you_mean`-style repair. Sibling of
    /// `<sce:on-sample link>` cross-resolution but on the field-binding
    /// axis: that one resolves doc names, this one resolves
    /// post-resolution member fields.
    ///
    /// Cross-kind typed binding. Today
    /// emitted only from the Forge→Forge path (a Forge document's
    /// expressions reference another Forge document imported via
    /// `<sce:import>`); the diagnostic itself is kind-agnostic so a
    /// future Statechart→Forge binding wires through the same code
    /// without renaming.
    #[error(
        "{importing_kind} '{importing_name}': '{alias}.{field}' references an undeclared field on imported {imported_kind} '{imported_name}'{}",
        if candidates.is_empty() {
            String::new()
        } else {
            format!(" (declared fields: {})", candidates.join(", "))
        }
    )]
    CrossKindFieldNotFound {
        /// Importing kind, e.g. `algorithm`, `procedure`.
        importing_kind: ForgeKind,
        /// Importing document's `name` attribute.
        importing_name: String,
        /// The `<sce:import as="..."/>` alias used in the expression.
        alias: String,
        /// The field name that did not resolve, verbatim from the
        /// expression source.
        field: String,
        /// Imported kind, e.g. `codec`, `bounded-collection`.
        imported_kind: ForgeKind,
        /// Imported document's `name` attribute.
        imported_name: String,
        /// Sorted, deduplicated list of legal field names exposed by
        /// the imported kind. Drives `Fix::ReplaceOneOf` so consumers
        /// see the closed candidate set.
        candidates: Vec<String>,
    },

    /// An expression in an importing kind references `<alias>.<field>`
    /// where the field resolves but the inferred type at the use site
    /// is incompatible with the field's declared type. Emitted only
    /// when the surrounding context constrains the expected type (e.g.
    /// `<sce:return expr="alias.field"/>` whose return type is declared
    /// on the kind signature) — opportunistic checks where the expected
    /// type is `Unknown` stay silent, since there is no contract to
    /// violate.
    ///
    /// Cross-kind typed binding.
    #[error(
        "{importing_kind} '{importing_name}': '{alias}.{field}' has type '{actual}' but context expects '{expected}'"
    )]
    CrossKindTypeMismatch {
        importing_kind: ForgeKind,
        importing_name: String,
        alias: String,
        field: String,
        /// Declared type of the imported field, rendered through
        /// [`SceType::canonical`] for stable wire form.
        actual: String,
        /// Expected type imposed by the use site (signature return
        /// type, `<sce:param type=...>`, …), rendered the same way.
        expected: String,
    },

    /// Two operands carrying `sce:quantity=…` annotations on
    /// **different** units meet in an arithmetic or bitwise operator.
    /// Arithmetic between different units is rejected without
    /// introducing a new `DiagnosticCode` variant
    /// — semantically a "type incompatibility", it
    /// surfaces under `validation/cross-kind-type-
    /// mismatch` with a typed payload that names both units and the
    /// operator they collide under.
    #[error(
        "{kind} '{name}': operator '{op}' combines incompatible quantity units '{left_unit}' and '{right_unit}' (expression: `{expr}`)"
    )]
    QuantityUnitMismatch {
        /// Kind of the enclosing document (Transform, Condition, …) so
        /// the diagnostic key is stable across kinds.
        kind: ForgeKind,
        /// Document name (the kind's `name` attribute).
        name: String,
        /// The operator token (`+`, `-`, `*`, `/`, `%`, `&`, `|`, `^`,
        /// `<<`, `>>`, …) that meets the two units.
        op: String,
        /// Left operand's unit, rendered via `UnitTag::as_str`.
        left_unit: String,
        /// Right operand's unit, rendered via `UnitTag::as_str`.
        right_unit: String,
        /// Original expression source as authored, for the
        /// diagnostic's `actual` slot. Helps the author find the
        /// specific site without a separate location pointer.
        expr: String,
    },

    /// The `<sce:import>` graph contains a cycle:
    /// `A.scxml` imports `B.scxml`, `B.scxml` imports `A.scxml`. This
    /// is a defensive check; the import enrichment pass would otherwise
    /// recurse into infinite open-file work or surface as an opaque
    /// stack-overflow at codegen time.
    ///
    /// Cross-kind typed binding.
    #[error(
        "circular <sce:import> dependency: {}",
        cycle.join(" → ")
    )]
    CrossKindCircularDependency {
        /// Cycle path in traversal order, starting and ending with the
        /// same document name. Length >= 2 by construction.
        cycle: Vec<String>,
    },

    // ── Enum kind invariants ───────────────────────────────────
    /// Enum document declares no `<sce:variant>` children. An
    /// enum must enumerate at least one
    /// named variant — empty vocabularies have no codegen lowering.
    ///
    /// Wire code: `validation/enum-no-variants`.
    #[error("enum '{name}': declares no <sce:variant> — at least one variant required")]
    EnumNoVariants {
        /// Document name (the kind's `name` attribute).
        name: String,
    },

    /// Two variants in the same enum document share an identifier.
    /// Variant names must be unique because authors reference
    /// variants as `<EnumName>.<variant_name>` — a collision would
    /// produce an ambiguous reference at the import site.
    ///
    /// Wire code: `validation/enum-variant-duplicate-name`.
    #[error("enum '{enum_name}': duplicate variant name '{name}'")]
    EnumVariantDuplicateName {
        /// Owning enum document name.
        enum_name: String,
        /// The duplicate variant name (appears twice).
        name: String,
    },

    /// Two variants in the same enum document share an underlying
    /// integer value. Enum's bijectivity invariant: the inverse
    /// function `variant → wire_byte` must be defined and total.
    /// Enum's defining departure from Lookup is exactly this
    /// invariant.
    ///
    /// Wire code: `validation/enum-variant-duplicate-value`.
    #[error(
        "enum '{enum_name}': variants '{first_name}' and '{second_name}' both have value {value}"
    )]
    EnumVariantDuplicateValue {
        /// Owning enum document name.
        enum_name: String,
        /// The shared underlying integer value.
        value: u64,
        /// Name of the first variant declared with this value.
        first_name: String,
        /// Name of the second variant attempting to reuse the value.
        second_name: String,
    },

    /// A variant's declared `value` does not fit in the document's
    /// `sce:underlying-type`. The per-variant overflow check
    /// runs at parse time so authors see the diagnostic anchored at
    /// the specific `<sce:variant>` element, not at downstream
    /// import sites.
    ///
    /// Wire code: `validation/enum-variant-value-overflows-underlying`.
    #[error(
        "enum '{enum_name}' variant '{variant_name}': value {value} overflows underlying type '{underlying}'"
    )]
    EnumVariantValueOverflowsUnderlying {
        /// Owning enum document name.
        enum_name: String,
        /// Variant carrying the overflowing value.
        variant_name: String,
        /// The numeric value as authored (parsed as u64).
        value: u64,
        /// The underlying-type spelling (e.g. `uint8`).
        underlying: String,
    },

    /// `sce:underlying-type` declares a type that is not one of the
    /// supported unsigned integer carriers (`uint8`/`uint16`/`uint32`/
    /// `uint64`). Unsigned only —
    /// signed-integer underlying types defer until a consumer needs
    /// negative wire bytes. Non-integer types (string/bool/float) are
    /// rejected unconditionally — enum variants need a fixed-width
    /// integer carrier for wire round-tripping.
    ///
    /// Wire code: `validation/enum-unsupported-underlying-type`.
    #[error(
        "enum '{name}': sce:underlying-type='{declared}' is not supported \
         (supported: uint8 | uint16 | uint32 | uint64)"
    )]
    EnumUnsupportedUnderlyingType {
        /// Document name (the kind's `name` attribute).
        name: String,
        /// The author's literal attribute value.
        declared: String,
    },

    /// An EventSchema
    /// document declares `sce:event-name="<X>"` against an event name
    /// that is reserved for the W3C SCXML platform (`error.*`,
    /// `done.invoke.*`, `done.state.*`). The platform raises these
    /// events with implementation-defined payload shape; an authored
    /// schema cannot constrain them without contradicting the platform
    /// contract. Wire code: `validation/event-schema-on-builtin-event`.
    /// See [`crate::forge::model::EventSchemaModel::BUILTIN_EVENT_PREFIXES`]
    /// for the closed prefix set.
    #[error(
        "EventSchema cannot declare a schema for W3C built-in event '{event_name}' \
         (reserved namespace: {})",
        crate::forge::model::EventSchemaModel::BUILTIN_EVENT_PREFIXES.join(", ")
    )]
    EventSchemaOnBuiltinEvent {
        /// Authored `sce:event-name` value that collides with a
        /// reserved W3C namespace prefix.
        event_name: String,
    },

    /// Send-side payload validation: a
    /// `<send event="X">` or `<raise event="X">` carries a
    /// `<param name="F"/>` whose name `F` is not declared on the
    /// EventSchema imported for event `X`. Wire code:
    /// `validation/event-payload-field-unknown`. Carries the schema's
    /// declared field surface as a closed `Fix::ReplaceOneOf`
    /// candidate set so consumers see the legal alternatives for
    /// `did_you_mean`-style typo repair, mirroring
    /// [`ValidationError::CrossKindFieldNotFound`] from the receive-
    /// side validator.
    #[error(
        "{importing_kind} '{importing_name}': <send event=\"{event_name}\"> declares <param name=\"{field}\"> not in the EventSchema for '{event_name}' (imported {imported_kind} '{imported_name}'){}",
        if candidates.is_empty() {
            String::new()
        } else {
            format!(" (declared fields: {})", candidates.join(", "))
        }
    )]
    EventPayloadFieldUnknown {
        /// Importing kind — typically `ForgeKind::Statechart` since
        /// `<send>`/`<raise>` originate from SCXML executable content.
        importing_kind: ForgeKind,
        /// Statechart-document name carrying the offending send.
        importing_name: String,
        /// SCXML event name (the `event="..."` attribute on the
        /// offending `<send>` / `<raise>`).
        event_name: String,
        /// Authored `<param name="...">` value that did not resolve
        /// to a declared field on the schema.
        field: String,
        /// Imported kind — fixed at `ForgeKind::EventSchema` for this
        /// variant; carried for diagnostic-payload symmetry with
        /// [`ValidationError::CrossKindFieldNotFound`].
        imported_kind: ForgeKind,
        /// Schema-document `name` attribute (typically derived from
        /// the schema's `sce:event-name`).
        imported_name: String,
        /// Sorted, deduplicated list of the schema's declared field
        /// ids. Drives `Fix::ReplaceOneOf` so consumers see the
        /// closed candidate set for typo repair.
        candidates: Vec<String>,
    },

    /// A transition guard applies an ordering operator (`<`, `>`, `<=`,
    /// `>=`) to a `bytes`-typed `_event.data.<field>`. Lexicographic
    /// ordering of an opaque payload byte-blob is not a meaningful
    /// author intent; only equality-as-bytes (`===` / `!==`) lowers to
    /// a well-defined, byte-identical comparison on every backend.
    /// Rejecting is more textbook than silently defining an order that
    /// would diverge per language. Wire code:
    /// `validation/bytes-comparison-not-equality`.
    #[error(
        "{importing_kind} '{importing_name}': operator '{op}' is not defined on the bytes payload '_event.data.{field}' \u{2014} only equality ('===' / '!==') is supported on bytes"
    )]
    BytesComparisonNotEquality {
        /// Importing kind — `ForgeKind::Statechart`, since the guard
        /// originates from an SCXML transition `cond`.
        importing_kind: ForgeKind,
        /// Statechart-document name carrying the offending guard.
        importing_name: String,
        /// The `bytes` payload field id (the `_event.data.<field>`
        /// operand the operator was applied to).
        field: String,
        /// The rejected ordering operator token (`<`, `>`, `<=`, `>=`).
        op: String,
    },
}

/// watching-zenoh RFC §synth-5-E callback-path failure
/// classification. Attached to
/// [`ValidationError::PoolSampleCallbackSignatureNonBorrow`] so the
/// per-instance message names the exact path-syntax mistake; the
/// outer code stays spec-verbatim
/// (`pool/sample-callback-signature-non-borrow`).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CallbackPathReason {
    /// Empty `callback=""` attribute or empty body after the language
    /// prefix (`rust:` with nothing after). Authors typically arrive
    /// here by removing a path mid-edit and forgetting to delete the
    /// attribute itself.
    #[error("declares an empty callback path")]
    EmptyPath,
    /// Unknown or missing language prefix. Today the only legal
    /// prefix is `rust:` (future language axes are forward-compat
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

/// watching-zenoh RFC §synth-5-D line 911 — worker shared-mutable-state
/// failure classification. Attached to
/// [`ValidationError::WorkerSharedMutableState`] so the outer code
/// stays spec-verbatim (`worker/shared-mutable-state`)
/// while each per-instance message
/// names the exact path that crossed the encapsulation boundary.
///
/// Layers 1 + 2 are implemented; layer 3 (hardening
/// against `<sce:extern>` non-inbox symbol use in worker bodies)
/// couples to the §synth-5-I intrinsic-registry composition and is not
/// implemented until a consumer needs it.
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
    #[error(
        "<sce:import src=\"{src}\" kind=\"{declared}\">: actual kind is '{actual}' (mismatch)"
    )]
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
    ReadError { src: String, source: std::io::Error },
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

    /// Watching-zenoh RFC §synth-5-J-4: an MCU-class kind (link / worker /
    /// buffer-pool / reassembly, or an MCU-only codec sub-feature)
    /// was authored against a language target outside `(rust, c11)`.
    /// MCU-class kinds bottom out on the rust/c11 substrate only;
    /// binding them to cpp/kotlin/go/python has no defined emitter
    /// shape. Producer: the codegen matrix walker in
    /// `forge::codegen_matrix`.
    #[error(
        "MCU-class kind '{kind}' cannot be lowered to language '{language}': \
         only rust and c11 have MCU substrate (watching-zenoh RFC §5.J.4)"
    )]
    CodegenMcuClassKindOnNonMcuLanguage { kind: String, language: String },

    /// Watching-zenoh RFC §synth-5-J-5: a generic-class kind expected to
    /// emit on every backend per the parity matrix is missing its
    /// per-kind Jinja2 template for the requested language. Template
    /// absence is an SCE bug, not a downstream concern. Producer:
    /// the codegen matrix walker in `forge::codegen_matrix`.
    #[error(
        "generic-class kind '{kind}': template missing for language '{language}' \
         (watching-zenoh RFC §5.J.4 expects all six backends to emit)"
    )]
    CodegenGenericKindBackendEmitMissing { kind: String, language: String },

    /// `deploy.yaml`'s
    /// `platform.c11_section_attribute` is present but the codegen
    /// target backend is not C11. The section attribute injects
    /// `__attribute__((section("...")))` which only the C11 emitter
    /// understands; non-MCU backends (cpp / rust / kotlin / go /
    /// python) have no equivalent contract and reject the field by
    /// design, mirroring the `<sce:extern>` non-MCU reject pattern so the
    /// section directive does not silently disappear on a non-C11
    /// compile. Producer: `forge::codegen_matrix::check_c11_section_attribute`.
    #[error(
        "platform.c11_section_attribute is set in deploy.yaml but the \
         target backend is '{backend}', not 'c11'. The section attribute \
         injects `__attribute__((section(...)))` which only the C11 \
         backend emits. Repair: remove the section attribute, switch \
         the backend to 'c11', or split deploy configurations per target."
    )]
    McuSectionAttributeOnNonMcuTarget { backend: String },

    /// Watching-zenoh RFC §synth-5-J-2 (item C3): the SCXML document
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

    /// Watching-zenoh RFC §synth-5-J-2 (item C3): the SCXML document
    /// is generated with `sce-codegen generate -l rust --no-std` but
    /// contains a §scxml-C-2 `<send>` that targets
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

    /// Watching-zenoh RFC §synth-5-J-2 (item C3): the SCXML document is
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

    /// Watching-zenoh RFC §synth-5-J-2 (item C3): the SCXML document is
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

    /// RFC §synth-5-F: a `<sce:fold>` body or a `<sce:const init=...>` scalar
    /// expression cannot be reduced to a build-time value. The host
    /// interpreter rejects every construct outside the §synth-5-F substrate
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

    /// RFC §synth-5-F bound 1: total iteration count across the body of a
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

    /// RFC §synth-5-F: the value yielded by a `<sce:fold>` body (or the init
    /// expression of a scalar `<sce:const>`) cannot be coerced to the
    /// declared element / scalar type. `expected` is the declared slot
    /// type; `actual` is a short tag describing the produced value's
    /// domain (e.g. `"bool"`, `"float"`) — substring `"bool→Uint16"` /
    /// `"float→Int32"` patterns the prior slug shape emitted, preserved for
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
            // §wire-W5 D2: SCXML semantic-validation shares the
            // forge-validation exit code (3) — both are post-parse
            // semantic-stage rejections; the wire `code` distinguishes
            // forge vs SCXML failures, the exit code does not.
            ForgeError::Scxml(_) => 3,
            // Mesh-deploy / topology / external / codegen failures
            // delegate to MeshError::exit_code() so the deploy-aware
            // path through `compile_scxml_with_imports` surfaces the
            // same CLI exit code an mesh-only entry point would
            // (`sce-codegen mesh ...`). MeshError owns the categorical
            // mapping per its own taxonomy.
            ForgeError::Mesh(e) => e.exit_code(),
            ForgeError::Io { .. } => 8,
        }
    }
}
