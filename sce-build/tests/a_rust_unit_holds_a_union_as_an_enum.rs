// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Rust lowers a union (docs/adr/0092, section 7, the Rust row), held to the
//! documents under `sce-build/tests/fixtures/union/`:
//!
//! * `union_shape_minimal.scxml` generates an enum named for the document, deriving
//!   `Debug`, `Clone` and `PartialEq`, one variant per variant of the tag's
//!   enum in the enum's order (not the document's), holding the case's record
//!   or nothing for an empty case; `kind(&self)`, the tag's value; and one
//!   `proven_<case>(&self)` per case that holds a record. Beside it `REVISION`
//!   (the tag enum's), `DATA_MAX_LEN`, `SCHEMA_ID` and `JSON_SCHEMA` (section
//!   3.1, item 4).
//! * `event_shapes_minimal.scxml`, whose fields hold the union alone, in a list and
//!   optionally, generates a payload struct that names the enum at each depth
//!   and brings the union's module in.
//! * The union's unit, with the enum and the records it names, compiles under
//!   `rustc -D warnings` with a module that builds every case, reads one through
//!   its `proven` read and its tag, and matches a value exhaustively.
//!
//! A case whose Rust variant would be another's, a case that holds a record no
//! unit declares a type for, and a tag whose key would hide a derived method
//! are refused by name.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::forge::error::{ForgeError, GenerateError, Located};
use sce_build::generator::{GeneratedOutput, Language};
use sce_build::ForgeCompileOptions;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/union")
}

/// What Rust generates for the document at `path`.
fn rust(path: &Path) -> Result<GeneratedOutput, Located<ForgeError>> {
    sce_build::compile_forge_file(path, Language::Rust, &[], &ForgeCompileOptions::default())
}

/// The one Rust file the fixture `name` generates, as `(file, text)`.
fn unit(name: &str) -> (String, String) {
    let output = rust(&fixtures().join(name))
        .unwrap_or_else(|e| panic!("{name} does not generate on Rust: {}", e.error));
    let mut rs: Vec<(String, String)> = output
        .files
        .into_iter()
        .filter(|(file, _)| file.ends_with(".rs"))
        .collect();
    assert_eq!(rs.len(), 1, "{name}: one Rust unit, got {rs:?}");
    rs.remove(0)
}

/// Where `needle` first stands in `text`, which must hold it.
fn position(text: &str, needle: &str) -> usize {
    text.find(needle)
        .unwrap_or_else(|| panic!("the unit holds `{needle}`:\n{text}"))
}

const KIND: &str = "shape_kind::ShapeKind";
const RADIUS: &str = "shape_radius_minimal::ShapeRadiusMinimalPayload";
const RECT: &str = "shape_rect_minimal::ShapeRectMinimalPayload";

#[test]
fn a_union_is_an_enum_with_one_variant_per_case_in_the_enums_order() {
    let (file, text) = unit("union_shape_minimal.scxml");
    assert_eq!(file, "union_shape_minimal.rs");
    for fragment in [
        "use super::shape_kind;\n".to_string(),
        "use super::shape_radius_minimal;\n".to_string(),
        "use super::shape_rect_minimal;\n".to_string(),
        "#[derive(Debug, Clone, PartialEq)]\npub enum UnionShapeMinimal {\n".to_string(),
        "    Dot,\n".to_string(),
        format!("    Circle({RADIUS}),\n"),
        format!("    Rect({RECT}),\n"),
        format!("    pub fn kind(&self) -> {KIND} {{\n"),
        format!("Self::Dot => {KIND}::Dot,\n"),
        format!("Self::Circle(_) => {KIND}::Circle,\n"),
        format!("Self::Rect(_) => {KIND}::Rect,\n"),
        format!("    pub fn proven_circle(&self) -> &{RADIUS} {{\n"),
        "Self::Circle(record) => record,\n".to_string(),
        format!("    pub fn proven_rect(&self) -> &{RECT} {{\n"),
        "Self::Rect(record) => record,\n".to_string(),
    ] {
        position(&text, &fragment);
    }
    // An empty case has nothing to read.
    assert!(!text.contains("proven_dot"), "{text}");
    // The document writes rect, dot, circle; the enum declares dot, circle,
    // rect, and the variants follow the enum.
    let dot = position(&text, "    Dot,\n");
    let circle = position(&text, "    Circle(");
    let rect = position(&text, "    Rect(");
    assert!(dot < circle && circle < rect, "{text}");
    // Each proven read refuses the other cases in the one place it is written.
    assert_eq!(text.matches("unreachable!(").count(), 2, "{text}");
}

/// The constants of a union's unit (docs/adr/0092, section 3.1, item 4): its
/// revision is its tag enum's (section 8.1), and its largest text is its
/// largest case, `{"kind":"rect","width":65535,"height":65535}`.
#[test]
fn a_union_unit_names_its_revision_its_largest_text_and_its_schema() {
    let (_, text) = unit("union_shape_minimal.scxml");
    for fragment in [
        "pub const REVISION: u32 = 1;\n",
        "pub const DATA_MAX_LEN: u64 = 44;\n",
        "pub const SCHEMA_ID: &str = \"urn:sce:schema:union_shape_minimal:1\";\n",
        "pub const JSON_SCHEMA: &str = \"",
    ] {
        position(&text, fragment);
    }
    assert!(text.contains("oneOf"), "{text}");
    assert!(!text.contains("No JSON_SCHEMA"), "{text}");
}

#[test]
fn a_payload_names_the_enum_at_every_depth() {
    let (file, text) = unit("event_shapes_minimal.scxml");
    assert_eq!(file, "event_shapes_minimal.rs");
    for fragment in [
        "use super::union_shape_minimal;\n",
        "#[derive(Debug, Clone, PartialEq)]\npub struct EventShapesMinimalPayload {\n",
        "    pub one: union_shape_minimal::UnionShapeMinimal,\n",
        "    pub many: ::sce_portable_bytes::BoundedList<union_shape_minimal::UnionShapeMinimal, 3>,\n",
        "    pub maybe: Option<union_shape_minimal::UnionShapeMinimal>,\n",
        "pub const MANY_MAX_COUNT: u32 = 3;\n",
        "pub const JSON_SCHEMA: &str = \"",
    ] {
        position(&text, fragment);
    }
}

/// A module that uses every part of the union's unit: a case of each kind
/// built, a `proven` read, the tag, and a `match` with no wildcard that the
/// compiler holds to every case.
const PROBE: &str = r#"use super::shape_kind::ShapeKind;
use super::shape_radius_minimal::ShapeRadiusMinimalPayload;
use super::shape_rect_minimal::ShapeRectMinimalPayload;
use super::union_shape_minimal::UnionShapeMinimal;

pub fn named(shape: &UnionShapeMinimal) -> u32 {
    match shape {
        UnionShapeMinimal::Dot => 0,
        UnionShapeMinimal::Circle(circle) => u32::from(circle.radius),
        UnionShapeMinimal::Rect(rect) => u32::from(rect.width) * u32::from(rect.height),
    }
}

pub fn radius_of(shape: &UnionShapeMinimal) -> u16 {
    shape.proven_circle().radius
}

pub fn tag_of(shape: &UnionShapeMinimal) -> ShapeKind {
    shape.kind()
}

pub fn built() -> [UnionShapeMinimal; 3] {
    [
        UnionShapeMinimal::Dot,
        UnionShapeMinimal::Circle(ShapeRadiusMinimalPayload { radius: 2 }),
        UnionShapeMinimal::Rect(ShapeRectMinimalPayload { width: 3, height: 4 }),
    ]
}

pub fn same(a: &UnionShapeMinimal, b: &UnionShapeMinimal) -> bool {
    a.clone() == *b
}
"#;

#[test]
fn the_rust_of_a_union_compiles_under_deny_warnings() {
    let Some(rustc) =
        sce_build::toolchain::require_any_or_skip(&["rustc"], "compile a generated union")
    else {
        return;
    };
    let dir = tempfile::tempdir().expect("a scratch directory");
    let mut modules = String::new();
    for name in [
        "shape_kind.scxml",
        "shape_radius_minimal.scxml",
        "shape_rect_minimal.scxml",
        "union_shape_minimal.scxml",
    ] {
        let (file, text) = unit(name);
        std::fs::write(dir.path().join(&file), text).expect("write the unit");
        modules.push_str(&format!("pub mod {};\n", file.trim_end_matches(".rs")));
    }
    std::fs::write(dir.path().join("probe.rs"), PROBE).expect("write the probe");
    modules.push_str("pub mod probe;\n");
    let root = dir.path().join("lib.rs");
    std::fs::write(&root, modules).expect("write the crate root");
    let out = Command::new(&rustc)
        .args([
            "--edition=2021",
            "--crate-type=lib",
            "--emit=metadata",
            "-D",
            "warnings",
            "--out-dir",
        ])
        .arg(dir.path().join("out"))
        .arg(&root)
        .output()
        .expect("rustc runs");
    assert!(
        out.status.success(),
        "the generated Rust does not compile:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// The refusal Rust gives the union `union` written beside `documents`, each
/// `(file, text)`, the enum of `shape_kind.scxml` and the record of
/// `shape_radius_minimal.scxml`; its detail.
fn refused(union: &str, documents: &[(&str, &str)]) -> String {
    let dir = tempfile::tempdir().expect("a scratch directory");
    for fixture in ["shape_kind.scxml", "shape_radius_minimal.scxml"] {
        std::fs::copy(fixtures().join(fixture), dir.path().join(fixture)).expect("copy a fixture");
    }
    for (file, text) in documents {
        std::fs::write(dir.path().join(file), text).expect("write a document");
    }
    let path = dir.path().join("probe_union.scxml");
    std::fs::write(&path, union).expect("write the union");
    let err = match rust(&path) {
        Ok(_) => panic!("Rust generated the union"),
        Err(refusal) => refusal,
    };
    assert!(
        matches!(
            err.error.to_diagnostics()[0].code,
            DiagnosticCode::GenerateUnsupportedFeature
        ),
        "{err:?}"
    );
    match err.error {
        ForgeError::Generate(boxed) => match *boxed {
            GenerateError::UnsupportedFeature { detail, .. } => detail,
            other => panic!("{other:?}"),
        },
        other => panic!("{other:?}"),
    }
}

/// A union document `probe_union` whose tag, under the key `tag`, is a value
/// of the enum in `enum_file`, which imports `shape_radius_minimal.scxml` as `Radius`
/// and the further imports `imports`, and whose cases are `cases`.
fn union_document(tag: &str, enum_file: &str, imports: &str, cases: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="union" name="probe_union">
  <sce:import kind="enum" src="{enum_file}" as="Kind"/>
  <sce:import kind="event-schema" src="shape_radius_minimal.scxml" as="Radius"/>
{imports}  <datamodel>
    <data id="{tag}" sce:type="enum:Kind">
{cases}    </data>
  </datamodel>
</scxml>
"#
    )
}

/// The three cases of `shape_kind.scxml`, `rect` holding `rect_record`.
fn shape_cases(rect_record: &str) -> String {
    format!(
        "      <sce:case variant=\"dot\"/>\n      <sce:case variant=\"circle\" \
         record=\"Radius\"/>\n      <sce:case variant=\"rect\" record=\"{rect_record}\"/>\n"
    )
}

/// Two variants an enum tells apart, `dot` and `Dot`, are one Rust variant
/// name, which two cases cannot both be.
#[test]
fn two_cases_of_one_rust_variant_are_refused_by_name() {
    let pair = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="enum" name="pair_kind" sce:underlying-type="uint8">
  <datamodel>
    <data id="variants">
      <sce:variant name="dot" value="0"/>
      <sce:variant name="Dot" value="1"/>
    </data>
  </datamodel>
</scxml>
"#;
    let detail = refused(
        &union_document(
            "kind",
            "pair_kind.scxml",
            "",
            "      <sce:case variant=\"dot\"/>\n      <sce:case variant=\"Dot\" record=\"Radius\"/>\n",
        ),
        &[("pair_kind.scxml", pair)],
    );
    assert!(
        detail.contains("the cases `dot` and `Dot` are both the Rust variant `Dot`"),
        "{detail}"
    );
}

/// A record of no field has no payload struct (`carries_payload`), so a case
/// that holds one would name a type no unit declares.
#[test]
fn a_case_that_holds_a_record_of_no_field_is_refused_by_name() {
    let empty = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="event-schema" name="shape_empty" sce:event-name="shape.empty">
  <datamodel sce:payload="none"/>
</scxml>
"#;
    let detail = refused(
        &union_document(
            "kind",
            "shape_kind.scxml",
            "  <sce:import kind=\"event-schema\" src=\"shape_empty.scxml\" as=\"Empty\"/>\n",
            &shape_cases("Empty"),
        ),
        &[("shape_empty.scxml", empty)],
    );
    assert!(
        detail.contains("the case `rect` holds `Empty`, which declares no field"),
        "{detail}"
    );
}

/// A tag read by `clone(&self)` would hide `Clone::clone` at every call
/// written with a dot, so its key names no method of the union.
#[test]
fn a_tag_whose_key_would_hide_a_derived_method_is_refused_by_name() {
    let rect =
        std::fs::read_to_string(fixtures().join("shape_rect_minimal.scxml")).expect("the record");
    let detail = refused(
        &union_document(
            "clone",
            "shape_kind.scxml",
            "  <sce:import kind=\"event-schema\" src=\"shape_rect_minimal.scxml\" as=\"Rect\"/>\n",
            &shape_cases("Rect"),
        ),
        &[("shape_rect_minimal.scxml", rect.as_str())],
    );
    assert!(
        detail.contains("the tag's key `clone` names no Rust method its value can be read by"),
        "{detail}"
    );
}
