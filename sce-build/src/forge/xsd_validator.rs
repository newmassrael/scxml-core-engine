// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
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

use libxml::parser::Parser;
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

impl crate::forge::diagnostic::ToDiagnostics for XsdErrors {
    /// XSD violations have a fixed routing (stage=xml, exit=2) — the
    /// container's only job is to emit one `Diagnostic` per violation,
    /// each with libxml2's line data attached. Carries the XSD
    /// spec anchor on every record for upstream agent grounding.
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
pub fn validate(xml_text: &str, source_label: &str, schema_path: &Path) -> Result<(), XsdErrors> {
    let xml_parser = Parser::default();
    let doc = xml_parser
        .parse_string(xml_text)
        .map_err(|e| XsdErrors {
            source_label: source_label.to_string(),
            diagnostics: vec![XsdDiag {
                line: None,
                col: None,
                message: format!("XML parse error: {e}"),
            }],
        })?;

    let schema_path_str = schema_path
        .to_str()
        .ok_or_else(|| XsdErrors {
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

/// Convenience: validate using the cached schema path. Returns `Ok(())`
/// when the schema cannot be located so downstream crates that vendor
/// `sce-build` without the `schemas/` directory still build — the
/// guarantee is "if a schema is available, validation runs", not "every
/// invocation must validate". The CI matrix runs from a checkout that
/// always has `schemas/`, so production fixtures always get validated.
pub fn validate_or_skip(xml_text: &str, source_label: &str) -> Result<(), XsdErrors> {
    match cached_schema_path() {
        Some(p) => validate(xml_text, source_label, p),
        None => Ok(()),
    }
}

fn format_error(err: &libxml::error::StructuredError) -> XsdDiag {
    let msg = err
        .message
        .as_deref()
        .unwrap_or("(libxml2 produced no message)")
        .trim_end_matches('\n')
        .to_string();
    XsdDiag {
        line: err.line.and_then(|l| if l > 0 { u32::try_from(l).ok() } else { None }),
        // libxml2's StructuredError exposes column only on parse
        // errors, not schema violations; fall through as None rather
        // than probe an unreliable field.
        col: None,
        message: msg,
    }
}

#[cfg(test)]
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
        assert!(combined.contains("bad_kind.scxml"), "filename in error: {combined}");
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
        assert!(combined.contains("direction"), "attribute cited: {combined}");
        assert!(combined.contains("sideways"), "value cited: {combined}");
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
}
