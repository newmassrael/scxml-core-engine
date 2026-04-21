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

use crate::forge::model::ForgeKind;
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

    /// Replace the `file` label while preserving `line`/`col` and the
    /// wrapped error. Used at the CLI boundary where the layer that
    /// owns the full on-disk path (e.g. the basename-with-extension)
    /// wants to override the identifier-oriented `name` that inner
    /// parser layers threaded into `location.file`. Separating these
    /// two concerns keeps parsers' `name` pure (a symbol identifier
    /// for model storage) without giving up the diagnostic precision
    /// downstream tooling expects when it opens `location.file`.
    pub fn with_file(mut self, file: impl Into<String>) -> Self {
        self.location.file = file.into();
        self
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
            ForgeError::Io { .. } => 8,
        }
    }
}
