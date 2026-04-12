// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
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
}

/// Expression transpilation errors encountered during code generation are
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
