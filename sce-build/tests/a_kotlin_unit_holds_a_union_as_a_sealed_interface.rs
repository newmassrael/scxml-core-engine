// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Kotlin lowers a union (docs/adr/0092, section 7, the Kotlin row), held to
//! the documents under `sce-build/tests/fixtures/union/`:
//!
//! * `union_shape.scxml` generates a sealed interface named for the document,
//!   `val kind` on it, one case nested in it per variant of the tag's enum in
//!   the enum's order (not the document's), a `data class` holding the case's
//!   record or a `data object` for an empty case, and one `proven<Case>()` per
//!   case that holds a record; beside it `REVISION` (the tag enum's),
//!   `DATA_MAX_LEN`, `SCHEMA_ID` and `JSON_SCHEMA` (section 3.1, item 4).
//! * `event_shapes.scxml`, whose fields hold the union alone, in a list and
//!   optionally, generates a payload class that names the sealed interface at
//!   each depth and imports the union's unit.
//! * The units, with the enum and the records they name, compile together under
//!   `kotlinc -Werror` with a file that builds every case, reads one through its
//!   `proven` read and matches a value exhaustively.
//!
//! A case whose Kotlin class would be another's, and a case that holds a
//! record no unit declares a class for, are refused by name.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::forge::error::{ForgeError, GenerateError, Located};
use sce_build::generator::{GeneratedOutput, Language};
use sce_build::ForgeCompileOptions;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/union")
}

/// What Kotlin generates for the document at `path`.
fn kotlin(path: &Path) -> Result<GeneratedOutput, Located<ForgeError>> {
    sce_build::compile_forge_file(path, Language::Kotlin, &[], &ForgeCompileOptions::default())
}

/// The one Kotlin file the fixture `name` generates, as `(file, text)`.
fn unit(name: &str) -> (String, String) {
    let output = kotlin(&fixtures().join(name))
        .unwrap_or_else(|e| panic!("{name} does not generate on Kotlin: {}", e.error));
    let mut kt: Vec<(String, String)> = output
        .files
        .into_iter()
        .filter(|(file, _)| file.ends_with(".kt"))
        .collect();
    assert_eq!(kt.len(), 1, "{name}: one Kotlin unit, got {kt:?}");
    kt.remove(0)
}

/// Where `needle` first stands in `text`, which must hold it.
fn position(text: &str, needle: &str) -> usize {
    text.find(needle)
        .unwrap_or_else(|| panic!("the unit holds `{needle}`:\n{text}"))
}

const KIND: &str = "com.sce.generated.shape_kind.ShapeKind";
const SHAPE: &str = "com.sce.generated.union_shape.UnionShape";
const RADIUS: &str = "com.sce.generated.shape_radius.ShapeRadiusPayload";
const RECT: &str = "com.sce.generated.shape_rect.ShapeRectPayload";

#[test]
fn a_union_is_a_sealed_interface_with_one_case_per_variant_in_the_enums_order() {
    let (file, text) = unit("union_shape.scxml");
    assert_eq!(file, "UnionShape.kt");
    for fragment in [
        "package com.sce.generated.union_shape\n".to_string(),
        "sealed interface UnionShape {".to_string(),
        format!("    val kind: {KIND}\n"),
        format!("    data object Dot : {SHAPE} {{"),
        format!("    data class Circle(val value: {RADIUS}) : {SHAPE} {{"),
        format!("    data class Rect(val value: {RECT}) : {SHAPE} {{"),
        format!("get() = {KIND}.DOT\n"),
        format!("get() = {KIND}.CIRCLE\n"),
        format!("get() = {KIND}.RECT\n"),
        format!("    fun provenCircle(): {RADIUS} =\n"),
        "(this as? Circle)?.value".to_string(),
        format!("    fun provenRect(): {RECT} =\n"),
        "(this as? Rect)?.value".to_string(),
    ] {
        position(&text, &fragment);
    }
    // An empty case has nothing to read.
    assert!(!text.contains("provenDot"), "{text}");
    // The document writes rect, dot, circle; the enum declares dot, circle,
    // rect, and the cases follow the enum.
    let dot = position(&text, "data object Dot");
    let circle = position(&text, "data class Circle");
    let rect = position(&text, "data class Rect");
    assert!(dot < circle && circle < rect, "{text}");
    // Every case is declared before the reads on the interface.
    assert!(rect < position(&text, "fun provenCircle"), "{text}");
}

/// The constants of a union's unit (docs/adr/0092, section 3.1, item 4): its
/// revision is its tag enum's (section 8.1), and its largest text is its
/// largest case, `{"kind":"rect","width":65535,"height":65535}`.
#[test]
fn a_union_unit_names_its_revision_its_largest_text_and_its_schema() {
    let (_, text) = unit("union_shape.scxml");
    for fragment in [
        "const val REVISION: UInt = 1u\n",
        "const val DATA_MAX_LEN: ULong = 44uL\n",
        "const val SCHEMA_ID: String = \"urn:sce:schema:union_shape:1\"\n",
        "const val JSON_SCHEMA: String = \"",
    ] {
        position(&text, fragment);
    }
    assert!(text.contains("oneOf"), "{text}");
    assert!(!text.contains("No JSON_SCHEMA"), "{text}");
}

#[test]
fn a_payload_names_the_sealed_interface_at_every_depth() {
    let (file, text) = unit("event_shapes.scxml");
    assert_eq!(file, "EventShapes.kt");
    for fragment in [
        "import com.sce.generated.union_shape.*\n",
        "data class EventShapesPayload(",
        "val one: UnionShape,",
        "val many: List<UnionShape>,",
        "val maybe: UnionShape?)",
        "const val MANY_MAX_COUNT: UInt = 3u\n",
        "const val JSON_SCHEMA: String = \"",
    ] {
        position(&text, fragment);
    }
}

/// A file that uses every part of the union's unit: a case of each kind built,
/// a payload that holds the union at each depth, a `proven` read, the tag, and
/// a `when` with no `else` that the compiler holds to every case.
const PROBE: &str = r#"package probe

import com.sce.generated.event_shapes.EventShapesPayload
import com.sce.generated.shape_kind.ShapeKind
import com.sce.generated.shape_radius.ShapeRadiusPayload
import com.sce.generated.shape_rect.ShapeRectPayload
import com.sce.generated.union_shape.UnionShape

fun named(shape: UnionShape): String =
    when (shape) {
        UnionShape.Dot -> "dot"
        is UnionShape.Circle -> "circle of ${shape.value.radius}"
        is UnionShape.Rect -> "rect of ${shape.value.width} by ${shape.value.height}"
    }

fun radiusOf(shape: UnionShape): UShort = shape.provenCircle().radius

fun tagOf(shape: UnionShape): ShapeKind = shape.kind

fun sent(): EventShapesPayload =
    EventShapesPayload(
        one = UnionShape.Circle(ShapeRadiusPayload(2u)),
        many = listOf(UnionShape.Rect(ShapeRectPayload(3u, 4u)), UnionShape.Dot),
        maybe = null,
    )
"#;

#[test]
fn the_kotlin_of_a_union_and_the_payload_that_holds_it_compiles_under_werror() {
    let Some(kotlinc) =
        sce_build::toolchain::require_any_or_skip(&["kotlinc"], "compile a generated union")
    else {
        return;
    };
    let dir = tempfile::tempdir().expect("a scratch directory");
    let mut sources = Vec::new();
    for name in [
        "shape_kind.scxml",
        "shape_radius.scxml",
        "shape_rect.scxml",
        "union_shape.scxml",
        "event_shapes.scxml",
    ] {
        let (file, text) = unit(name);
        let path = dir.path().join(file);
        std::fs::write(&path, text).expect("write the unit");
        sources.push(path);
    }
    let probe = dir.path().join("Probe.kt");
    std::fs::write(&probe, PROBE).expect("write the probe");
    sources.push(probe);
    let out = Command::new(&kotlinc)
        .arg("-Werror")
        .arg("-d")
        .arg(dir.path().join("classes"))
        .args(&sources)
        .output()
        .expect("kotlinc runs");
    assert!(
        out.status.success(),
        "the generated Kotlin does not compile:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// The refusal Kotlin gives the union `union` written beside `documents`, each
/// `(file, text)`, and the record of `shape_radius.scxml`; its detail.
fn refused(union: &str, documents: &[(&str, &str)]) -> String {
    let dir = tempfile::tempdir().expect("a scratch directory");
    std::fs::copy(
        fixtures().join("shape_radius.scxml"),
        dir.path().join("shape_radius.scxml"),
    )
    .expect("copy the record");
    for (file, text) in documents {
        std::fs::write(dir.path().join(file), text).expect("write a document");
    }
    let path = dir.path().join("probe_union.scxml");
    std::fs::write(&path, union).expect("write the union");
    let err = match kotlin(&path) {
        Ok(_) => panic!("Kotlin generated the union"),
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

/// A union document `probe_union` whose tag is a value of the enum in
/// `enum_file`, which imports `shape_radius.scxml` as `Radius` and the further
/// imports `imports`, and whose cases are `cases`.
fn union_document(enum_file: &str, imports: &str, cases: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="union" name="probe_union">
  <sce:import kind="enum" src="{enum_file}" as="Kind"/>
  <sce:import kind="event-schema" src="shape_radius.scxml" as="Radius"/>
{imports}  <datamodel>
    <data id="kind" sce:type="enum:Kind">
{cases}    </data>
  </datamodel>
</scxml>
"#
    )
}

/// Two variants an enum tells apart, `dot` and `Dot`, are one Kotlin class
/// name, which two cases cannot both be.
#[test]
fn two_cases_of_one_kotlin_class_name_are_refused_by_name() {
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
            "pair_kind.scxml",
            "",
            "      <sce:case variant=\"dot\"/>\n      <sce:case variant=\"Dot\" record=\"Radius\"/>\n",
        ),
        &[("pair_kind.scxml", pair)],
    );
    assert!(
        detail.contains("the cases `dot` and `Dot` are both the Kotlin class `Dot`"),
        "{detail}"
    );
}

/// A record of no field has no payload class (`carries_payload`), so a case
/// that holds one would name a class no unit declares.
#[test]
fn a_case_that_holds_a_record_of_no_field_is_refused_by_name() {
    let empty = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="event-schema" name="shape_empty" sce:event-name="shape.empty">
  <datamodel sce:payload="none"/>
</scxml>
"#;
    let kinds = std::fs::read_to_string(fixtures().join("shape_kind.scxml")).expect("the enum");
    let detail = refused(
        &union_document(
            "shape_kind.scxml",
            "  <sce:import kind=\"event-schema\" src=\"shape_empty.scxml\" as=\"Empty\"/>\n",
            "      <sce:case variant=\"dot\"/>\n      <sce:case variant=\"circle\" record=\"Radius\"/>\n      <sce:case variant=\"rect\" record=\"Empty\"/>\n",
        ),
        &[("shape_kind.scxml", kinds.as_str()), ("shape_empty.scxml", empty)],
    );
    assert!(
        detail.contains("the case `rect` holds `Empty`, which declares no field"),
        "{detail}"
    );
}
