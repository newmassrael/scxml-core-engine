// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// XSD validation for Extended SCXML documents.
//
// Wires `schemas/sce-forge.xsd` (the W3C SCXML wrapper that imports the
// `sce:` namespace extension schema) into the parser entry point so the
// `sce:` attribute namespace is validated against a single declarative
// source of truth before any structural parsing runs. Errors include
// libxml2's line/column information so authors can locate problems
// directly.
//
// The validator is invoked once per forge document at the start of
// `parser::parse_forge_with_imports`. A document that fails XSD
// validation never reaches kind-specific parsing — fail fast at the
// system boundary, contracts at the entry point.
//
// libxml2 is a build-time C dependency (libxml2-dev on Debian/Ubuntu,
// libxml2 on macOS Homebrew, libxml2 on Windows vcpkg). It is required
// only by `sce-build`, the host-side codegen tool — generated code has
// no libxml2 dependency, so the embedded deployment constraints in
// SCE_FORGE.md §2.1 are unaffected.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// libxml2 is linked only under the `xsd` feature; see `validate_or_skip`.
#[cfg(feature = "xsd")]
use libxml::parser::Parser;
#[cfg(feature = "xsd")]
use libxml::schemas::{SchemaParserContext, SchemaValidationContext};

/// A single XSD validation violation.
///
/// libxml2 reports line (and sometimes column) for every violation,
/// plus a free-form message. Preserving these as structured fields
/// — instead of collapsing to a formatted string — lets each violation
/// emit its own NDJSON diagnostic with accurate `location` data.
#[derive(Debug, Clone)]
pub struct XsdDiag {
    pub line: Option<u32>,
    pub col: Option<u32>,
    pub message: String,
}

/// The collection of XSD violations produced by one `validate()` call.
///
/// `source_label` is the filename (or caller-meaningful identifier)
/// that authors see in error messages; it rides on the container, not
/// each `XsdDiag`, because it is invariant across a single validation.
/// `diagnostics` preserves libxml2's natural ordering (top of file to
/// bottom).
///
/// Implements `Display` by rendering one line per violation in the
/// same format the editor-style "go to error" convention expects, so
/// human-mode CLI output continues to look as it did when XsdErrors
/// was `Vec<String>`.
#[derive(Debug, thiserror::Error)]
pub struct XsdErrors {
    pub source_label: String,
    pub diagnostics: Vec<XsdDiag>,
}

/// `XsdErrors` is a *multi-record container*: one libxml2 validation
/// run surfaces N violations and each violation becomes its own
/// `Diagnostic` with its own `message` (from libxml2) and line number.
/// Collapsing them into a single record would hide the per-violation
/// line data upstream consumers rely on, so this type deliberately does
/// **not** implement
/// [`SingleDiagnostic`](crate::forge::diagnostic::SingleDiagnostic) —
/// the trait split at the diagnostic layer makes "no single payload"
/// expressible directly instead of via `unreachable!` escape.
impl crate::forge::diagnostic::ToDiagnostics for XsdErrors {
    /// XSD violations have a fixed routing (stage=xml, exit=2).
    fn exit_code(&self) -> i32 {
        2
    }

    fn to_diagnostics(&self) -> Vec<crate::forge::diagnostic::Diagnostic> {
        use crate::forge::diagnostic::{
            compute_id, Diagnostic, DiagnosticCode, Location, Stage, SCHEMA_VERSION,
        };
        let code = DiagnosticCode::XmlSchemaValidation;
        let stage = Stage::Xml;
        self.diagnostics
            .iter()
            .map(|d| {
                let key_fragments = vec![
                    self.source_label.clone(),
                    d.line.map(|l| l.to_string()).unwrap_or_default(),
                    d.message.clone(),
                ];
                let id = compute_id(
                    code,
                    stage,
                    Some(self.source_label.as_str()),
                    &key_fragments,
                );
                Diagnostic {
                    schema_version: SCHEMA_VERSION,
                    id,
                    generator: crate::GENERATOR_COMMIT,
                    code,
                    stage,
                    spec: code.spec_anchor(),
                    message: d.message.clone(),
                    location: Some(Location {
                        file: self.source_label.clone(),
                        line: d.line,
                        col: d.col,
                    }),
                    expected: None,
                    actual: None,
                    fix: None,
                    spec_provenance: Vec::new(),
                    question_kind: None,
                }
            })
            .collect()
    }
}

impl std::fmt::Display for XsdErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        for d in &self.diagnostics {
            if !first {
                writeln!(f)?;
            }
            first = false;
            write!(
                f,
                "{}:{}: {}",
                self.source_label,
                d.line.unwrap_or(0),
                d.message
            )?;
        }
        Ok(())
    }
}

/// Resolve the absolute path of `schemas/sce-forge.xsd`.
///
/// Resolution order:
/// 1. `SCE_SCHEMAS_DIR` environment variable (for tests / overrides)
/// 2. `CARGO_MANIFEST_DIR/../schemas/sce-forge.xsd` (development checkout)
/// 3. `schemas/sce-forge.xsd` relative to current working directory
///    (running from project root)
///
/// Returns `None` if the schema file cannot be located. Callers may
/// treat this as a non-fatal "validation skipped" condition — see
/// `validate_or_skip` below — so dropping `sce-build` into a downstream
/// crate that does not vendor the schemas does not break the build.
pub fn find_schema_path() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("SCE_SCHEMAS_DIR") {
        let p = Path::new(&dir).join("sce-forge.xsd");
        if p.exists() {
            return Some(p);
        }
    }
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let candidate = crate_dir.join("../schemas/sce-forge.xsd");
    if candidate.exists() {
        return Some(candidate);
    }
    let candidate = Path::new("schemas/sce-forge.xsd");
    if candidate.exists() {
        return Some(candidate.to_path_buf());
    }
    None
}

/// Cache the resolved schema path so we resolve filesystem paths once per
/// process instead of on every parse call. The validation context itself
/// is rebuilt per-call because libxml's `SchemaValidationContext` holds
/// raw FFI pointers and is not `Sync`.
#[cfg(feature = "xsd")]
fn cached_schema_path() -> Option<&'static Path> {
    static SCHEMA_PATH: OnceLock<Option<PathBuf>> = OnceLock::new();
    SCHEMA_PATH.get_or_init(find_schema_path).as_deref()
}

/// Validate `xml_text` against `schemas/sce-forge.xsd` using libxml2.
///
/// `source_label` is included in error messages so authors see which
/// SCXML file failed (libxml2 reports the in-memory document as
/// "noname.xml" otherwise). Pass the SCXML filename or any other
/// caller-meaningful identifier.
///
/// Returns `Ok(())` on a clean validation, `Err(XsdErrors)` on any
/// schema or validity failure. The error vector preserves libxml2's
/// natural ordering (top of file to bottom).
#[cfg(feature = "xsd")]
pub fn validate(xml_text: &str, source_label: &str, schema_path: &Path) -> Result<(), XsdErrors> {
    let xml_parser = Parser::default();
    let doc = xml_parser.parse_string(xml_text).map_err(|e| XsdErrors {
        source_label: source_label.to_string(),
        diagnostics: vec![XsdDiag {
            line: None,
            col: None,
            message: format!("XML parse error: {e}"),
        }],
    })?;

    let schema_path_str = schema_path.to_str().ok_or_else(|| XsdErrors {
        source_label: source_label.to_string(),
        diagnostics: vec![XsdDiag {
            line: None,
            col: None,
            message: format!("schema path is not valid UTF-8: {schema_path:?}"),
        }],
    })?;

    let mut schema_parser = SchemaParserContext::from_file(schema_path_str);
    let mut schema_ctx =
        SchemaValidationContext::from_parser(&mut schema_parser).map_err(|errs| XsdErrors {
            source_label: source_label.to_string(),
            diagnostics: errs.into_iter().map(|e| format_error(&e)).collect(),
        })?;

    schema_ctx
        .validate_document(&doc)
        .map_err(|errs| XsdErrors {
            source_label: source_label.to_string(),
            diagnostics: errs.into_iter().map(|e| format_error(&e)).collect(),
        })
}

/// Why a document was not validated. Never a failure — each variant is a
/// legitimate configuration — but never invisible either.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XsdSkipReason {
    /// `schemas/sce-forge.xsd` could not be located: a downstream crate
    /// vendoring `sce-build` without the `schemas/` directory.
    SchemaNotFound,
    /// Built without the `xsd` feature, so libxml2 is not linked. The
    /// `wasm32` target takes this path — libxml2 is a native C library and
    /// cannot cross-compile to WebAssembly.
    FeatureDisabled,
}

impl XsdSkipReason {
    /// Operator-facing explanation, used in the parser's warning.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SchemaNotFound => {
                "schemas/sce-forge.xsd not found (set SCE_SCHEMAS_DIR or vendor schemas/)"
            }
            Self::FeatureDisabled => {
                "built without the `xsd` feature, so libxml2 is not linked \
                 (the wasm32 target cannot link it)"
            }
        }
    }
}

/// Whether a document actually went through the schema.
///
/// This type exists because the previous signature returned `Ok(())` for
/// BOTH "validated clean" and "could not validate", making the two
/// indistinguishable to every caller. A build that silently stops
/// validating at the system boundary still reports success, which is the
/// failure mode this whole module is meant to prevent. Callers now have to
/// name which case they are in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub enum XsdOutcome {
    /// The document was checked against the schema and is valid.
    Validated,
    /// No check ran. The document may or may not conform.
    NotValidated(XsdSkipReason),
}

/// Report a non-validating parse once per process.
///
/// Shared by both parser entry points (`parser::parse_forge_with_imports` and
/// `forge::parser`) so the wording and the once-only behaviour have a single
/// implementation. Once, not per document: N identical lines on a batch run
/// train the reader to ignore them, which is silence with extra steps.
pub fn warn_if_not_validated(outcome: XsdOutcome) {
    if let XsdOutcome::NotValidated(reason) = outcome {
        static WARNED: OnceLock<()> = OnceLock::new();
        WARNED.get_or_init(|| {
            eprintln!(
                "warning: XSD schema validation did not run — {}. Documents are \
                 parsed structurally but their sce: attributes are unchecked.",
                reason.as_str()
            );
        });
    }
}

/// Validate using the cached schema path, reporting whether validation
/// actually ran.
///
/// The guarantee is "if a schema is available, validation runs", not "every
/// invocation validates" — downstream crates that vendor `sce-build` without
/// `schemas/` must still build. The CI matrix runs from a checkout that always
/// has `schemas/`, so production fixtures are always validated. What changed is
/// that the not-validated case is now returned rather than swallowed.
#[cfg(feature = "xsd")]
pub fn validate_or_skip(xml_text: &str, source_label: &str) -> Result<XsdOutcome, XsdErrors> {
    match cached_schema_path() {
        Some(p) => validate(xml_text, source_label, p).map(|()| XsdOutcome::Validated),
        None => Ok(XsdOutcome::NotValidated(XsdSkipReason::SchemaNotFound)),
    }
}

/// `xsd`-less build: libxml2 is not linked, so no validation is possible.
///
/// Returning the reason instead of `Ok(())` is what keeps a `wasm32` build from
/// reporting the same success as a fully validated one.
#[cfg(not(feature = "xsd"))]
pub fn validate_or_skip(_xml_text: &str, _source_label: &str) -> Result<XsdOutcome, XsdErrors> {
    Ok(XsdOutcome::NotValidated(XsdSkipReason::FeatureDisabled))
}

#[cfg(feature = "xsd")]
fn format_error(err: &libxml::error::StructuredError) -> XsdDiag {
    let msg = err
        .message
        .as_deref()
        .unwrap_or("(libxml2 produced no message)")
        .trim_end_matches('\n')
        .to_string();
    XsdDiag {
        line: err
            .line
            .and_then(|l| if l > 0 { u32::try_from(l).ok() } else { None }),
        // libxml2's StructuredError exposes column only on parse
        // errors, not schema violations; fall through as None rather
        // than probe an unreliable field.
        col: None,
        message: msg,
    }
}

#[cfg(test)]
mod outcome_tests {
    use super::*;

    /// A clean validation and a non-validating build must be distinguishable.
    ///
    /// Both returned `Ok(())` before, so a build that stopped validating at the
    /// system boundary reported exactly what a validating one did. These two
    /// tests are the pair that pins the distinction: whichever feature
    /// configuration is compiled, the outcome names itself.
    #[cfg(feature = "xsd")]
    #[test]
    fn xsd_build_reports_validated() {
        let doc = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" name="x" initial="a">
  <state id="a"/>
</scxml>"#;
        // Only assert the discriminant: whether the schema is reachable in this
        // checkout is a property of the environment, not of the contract. What
        // must hold is that a validated document never reports NotValidated.
        match validate_or_skip(doc, "t.scxml") {
            Ok(XsdOutcome::Validated) => {}
            Ok(XsdOutcome::NotValidated(r)) => {
                assert_eq!(
                    r,
                    XsdSkipReason::SchemaNotFound,
                    "an xsd-enabled build may only skip for a missing schema"
                );
            }
            Err(e) => panic!("valid document rejected: {e}"),
        }
    }

    #[cfg(not(feature = "xsd"))]
    #[test]
    fn non_xsd_build_reports_feature_disabled() {
        assert_eq!(
            validate_or_skip("<scxml/>", "t.scxml").unwrap(),
            XsdOutcome::NotValidated(XsdSkipReason::FeatureDisabled),
            "a build without libxml2 must say so rather than report success"
        );
    }

    #[test]
    fn skip_reasons_explain_themselves() {
        // The parser prints these verbatim; an empty or placeholder string
        // would reintroduce the silence in a different shape.
        for r in [
            XsdSkipReason::SchemaNotFound,
            XsdSkipReason::FeatureDisabled,
        ] {
            assert!(r.as_str().len() > 20, "{r:?} needs an actionable message");
        }
    }
}

// Exercises `validate()` directly, which only exists when libxml2 is linked.
#[cfg(all(test, feature = "xsd"))]
mod tests {
    use super::*;

    const VALID_CODEC: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="little" name="x">
  <datamodel>
    <data id="a" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
  </datamodel>
</scxml>"#;

    const BAD_KIND: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="not_a_kind" name="x">
  <datamodel/>
</scxml>"#;

    const BAD_BIT_SIZE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" name="x">
  <datamodel>
    <data id="a" sce:type="uint8" sce:byte="0" sce:bit-size="not-a-number"/>
  </datamodel>
</scxml>"#;

    const BAD_DIRECTION: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="transform" name="x">
  <datamodel>
    <data id="a" sce:type="uint8" sce:direction="sideways"/>
  </datamodel>
</scxml>"#;

    // ── sce:template / sce:use / sce:param grammar.
    //    Normal build flow expands <sce:use> before XSD runs, so these
    //    cases exercise the schema declarations directly — useful for
    //    editor integrations and for catching missing `template=` on
    //    documents that reach the validator unexpanded (e.g. via the
    //    in-memory parse_string path). ──
    const VALID_USE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       name="x">
  <state id="accepting">
    <sce:use template="guard.sce-template.xml" port="80" proto="TCP"/>
  </state>
</scxml>"#;

    const USE_MISSING_TEMPLATE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       name="x">
  <state id="accepting">
    <sce:use port="80"/>
  </state>
</scxml>"#;

    fn schema() -> PathBuf {
        find_schema_path().expect("schemas/sce-forge.xsd must be reachable from CARGO_MANIFEST_DIR")
    }

    #[test]
    fn valid_codec_passes() {
        validate(VALID_CODEC, "valid_codec.scxml", &schema()).expect("must pass");
    }

    #[test]
    fn bad_kind_is_rejected_with_enum_error() {
        let err = validate(BAD_KIND, "bad_kind.scxml", &schema()).unwrap_err();
        let combined = err.to_string();
        assert!(
            combined.contains("bad_kind.scxml"),
            "filename in error: {combined}"
        );
        assert!(combined.contains("not_a_kind"), "value cited: {combined}");
        assert!(combined.contains("kind"), "attribute cited: {combined}");
    }

    #[test]
    fn bad_bit_size_is_rejected() {
        let err = validate(BAD_BIT_SIZE, "bad_bit_size.scxml", &schema()).unwrap_err();
        let combined = err.to_string();
        assert!(combined.contains("bit-size"), "attribute cited: {combined}");
        assert!(combined.contains("not-a-number"), "value cited: {combined}");
    }

    #[test]
    fn bad_direction_is_rejected() {
        let err = validate(BAD_DIRECTION, "bad_direction.scxml", &schema()).unwrap_err();
        let combined = err.to_string();
        assert!(
            combined.contains("direction"),
            "attribute cited: {combined}"
        );
        assert!(combined.contains("sideways"), "value cited: {combined}");
    }

    #[test]
    fn valid_sce_use_passes() {
        validate(VALID_USE, "valid_use.scxml", &schema()).expect("must pass");
    }

    #[test]
    fn sce_use_without_template_attribute_is_rejected() {
        let err = validate(USE_MISSING_TEMPLATE, "no_template.scxml", &schema()).unwrap_err();
        let combined = err.to_string();
        assert!(
            combined.contains("template"),
            "schema error must cite the missing `template` attribute: {combined}"
        );
    }

    #[test]
    fn all_real_fixtures_validate() {
        let schema = schema();
        let resources = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/resources");
        let mut failures = Vec::new();
        for entry in std::fs::read_dir(&resources).expect("resources dir") {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("scxml") {
                continue;
            }
            let xml = std::fs::read_to_string(&path).unwrap();
            let label = path.file_name().unwrap().to_string_lossy().into_owned();
            if let Err(errs) = validate(&xml, &label, &schema) {
                failures.push(format!("{label}:\n{errs}"));
            }
        }
        assert!(
            failures.is_empty(),
            "the following fixtures failed XSD validation:\n{}",
            failures.join("\n---\n")
        );
    }

    /// Sweep every statechart fixture under `resources/` through the same
    /// XSD validator that `parser::SCXMLParser::parse_impl` now invokes at
    /// the system boundary. Catches drift between the schema's permissive
    /// W3C SCXML wrapper (xs:any lax) and any `sce:*` attribute a fixture
    /// introduces without declaring it in `sce-forge-ext.xsd`.
    ///
    /// Runs against the entire W3C conformance corpus (~200 fixtures), so
    /// it doubles as a smoke test: if a new sce: attribute lands in one
    /// statechart test and the ext schema hasn't been updated, this test
    /// fails with a concrete file + line pointer.
    #[test]
    fn all_statechart_fixtures_validate() {
        let schema = schema();
        let resources = Path::new(env!("CARGO_MANIFEST_DIR")).join("../resources");
        assert!(
            resources.exists(),
            "statechart resources dir missing: {}",
            resources.display()
        );
        let mut total = 0usize;
        let mut failures = Vec::new();
        for top in std::fs::read_dir(&resources).expect("resources dir") {
            let top = top.unwrap();
            if !top.file_type().unwrap().is_dir() {
                continue;
            }
            for entry in std::fs::read_dir(top.path()).expect("fixture subdir") {
                let entry = entry.unwrap();
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("scxml") {
                    continue;
                }
                total += 1;
                let xml = std::fs::read_to_string(&path).unwrap();
                let label = path
                    .strip_prefix(&resources)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                if let Err(errs) = validate(&xml, &label, &schema) {
                    failures.push(format!("{label}:\n{errs}"));
                }
            }
        }
        assert!(total > 0, "no statechart fixtures found to sweep");
        assert!(
            failures.is_empty(),
            "{} of {total} statechart fixtures failed XSD validation:\n{}",
            failures.len(),
            failures.join("\n---\n")
        );
    }

    /// Drift guard: every value the XSD
    /// (`schemas/sce-forge-ext.xsd`) enumerates for a closed-set grammar
    /// type must be a token the engine's `from_attr` parser accepts. The
    /// XSD is the structural gate SCE itself runs (see [`validate`]); if
    /// it enumerated a token the parser rejects, an author (or the
    /// authoring loop) who satisfied the XSD would still hit a
    /// downstream codegen rejection — the XSD would be lying about the
    /// accepted subset. This pins XSD ⊆ engine-accepted for the four
    /// closed-set enums (`kindType`, `sceType`, `directionType`,
    /// `endianType`). The reverse direction — a token the engine accepts
    /// but the XSD omits — self-surfaces immediately as an XSD rejection
    /// of an otherwise-valid document, so it needs no separate guard.
    /// See `SCE_ACCEPTED_SUBSET.md` §2.1 / §2.2.
    #[test]
    fn xsd_enums_are_subset_of_engine_accepted_tokens() {
        use crate::forge::model::{Direction, Endian, ForgeKind, SceType};
        const EXT_XSD: &str = include_str!("../../../schemas/sce-forge-ext.xsd");

        // Extract the `value="..."` of every <xs:enumeration> under a
        // named <xs:simpleType>. A tight string scan rather than a full
        // XML parse: the file is checked-in and the sweep test above
        // already validates its well-formedness, so a scan keeps this
        // guard dependency-free.
        fn enum_values(xsd: &str, type_name: &str) -> Vec<String> {
            let open = format!("<xs:simpleType name=\"{type_name}\">");
            let start = xsd
                .find(&open)
                .unwrap_or_else(|| panic!("XSD missing simpleType {type_name}"));
            let body = &xsd[start..];
            let end = body
                .find("</xs:simpleType>")
                .expect("unterminated simpleType");
            let mut out = Vec::new();
            for frag in body[..end].split("<xs:enumeration value=\"").skip(1) {
                if let Some(q) = frag.find('"') {
                    out.push(frag[..q].to_string());
                }
            }
            assert!(!out.is_empty(), "simpleType {type_name} enumerates nothing");
            out
        }

        for v in enum_values(EXT_XSD, "kindType") {
            assert!(
                ForgeKind::from_attr(&v).is_some(),
                "XSD kindType enumerates '{v}' but ForgeKind::from_attr rejects \
                 it (XSD ↔ model.rs drift — see SCE_ACCEPTED_SUBSET.md §2.1)"
            );
        }
        for v in enum_values(EXT_XSD, "sceType") {
            assert!(
                SceType::from_attr(&v).is_some(),
                "XSD sceType enumerates '{v}' but SceType::from_attr rejects it \
                 (see SCE_ACCEPTED_SUBSET.md §2.2)"
            );
        }
        for v in enum_values(EXT_XSD, "directionType") {
            assert!(
                Direction::from_attr(&v).is_some(),
                "XSD directionType enumerates '{v}' but Direction::from_attr \
                 rejects it"
            );
        }
        for v in enum_values(EXT_XSD, "endianType") {
            assert!(
                Endian::from_attr(&v).is_some(),
                "XSD endianType enumerates '{v}' but Endian::from_attr rejects it"
            );
        }
    }
}
