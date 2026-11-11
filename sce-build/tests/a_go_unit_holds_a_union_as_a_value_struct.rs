// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Go lowers a union (docs/adr/0092, section 7, the Go row, and its item 4),
//! held to the documents under `sce-build/tests/fixtures/union/`:
//!
//! * `union_shape_minimal.scxml` generates a value struct named for the
//!   document, the package of its document: an unexported `tag` of the tag's
//!   enum and one unexported member per case that holds a record, in the enum's
//!   order (not the document's); one constructor per case, `<Union><Case>`,
//!   the only way a value is built; `Kind()`, the tag's value; and one
//!   `Proven<Case>()` per case that holds a record, whose other cases panic.
//!   Beside it `Revision` (the tag enum's), `DataMaxLen`, `SchemaID` and
//!   `JSONSchema` (section 3.1, item 4). The struct is the shape a Go machine
//!   holds its own union in (`a_go_machine_reads_a_union_payload_by_its_tag.rs`).
//! * `event_shapes_minimal.scxml`, whose fields hold the union alone, in a list
//!   and optionally, generates a payload struct that names the value struct at
//!   each depth through the union's package, which it imports.
//! * The units, with the enum and the records they name, build with `go` and
//!   a package that builds every case, reads one through its proven read and
//!   tells every case apart by `Kind()`; its test holds two values of one case
//!   and one record equal, and a read of another case to its panic.
//!
//! Two cases of one Go constructor, a constructor spelled as a constant of the
//! unit, and a case that holds a record no unit declares a type for, are
//! refused by name.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::forge::error::{ForgeError, GenerateError, Located};
use sce_build::generator::{GeneratedOutput, Language};
use sce_build::ForgeCompileOptions;

/// The module path the generated packages are imported under.
const PREFIX: &str = "example.com/generated";

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/union")
}

fn go_runtime() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../backends/go/forge-runtime")
}

fn options() -> ForgeCompileOptions {
    ForgeCompileOptions {
        go_module_prefix: Some(PREFIX.to_string()),
        ..ForgeCompileOptions::default()
    }
}

/// What Go generates for the document at `path`.
fn go(path: &Path) -> Result<GeneratedOutput, Located<ForgeError>> {
    sce_build::compile_forge_file(path, Language::Go, &[], &options())
}

/// The one Go file the fixture `name` generates, as `(file, text)`.
fn unit(name: &str) -> (String, String) {
    let output = go(&fixtures().join(name))
        .unwrap_or_else(|e| panic!("{name} does not generate on Go: {}", e.error));
    let mut files: Vec<(String, String)> = output
        .files
        .into_iter()
        .filter(|(file, _)| file.ends_with(".go"))
        .collect();
    assert_eq!(files.len(), 1, "{name}: one Go unit, got {files:?}");
    files.remove(0)
}

/// Where `needle` first stands in `text`, which must hold it.
fn position(text: &str, needle: &str) -> usize {
    text.find(needle)
        .unwrap_or_else(|| panic!("the unit holds `{needle}`:\n{text}"))
}

const SHAPE: &str = "UnionShapeMinimal";
const KIND: &str = "sce_shape_kind.ShapeKind";
const RADIUS: &str = "sce_shape_radius_minimal.ShapeRadiusMinimalPayload";
const RECT: &str = "sce_shape_rect_minimal.ShapeRectMinimalPayload";

/// Pinned by `render_union_go` (`sce-build/src/forge/generator.rs`) and
/// `tools/codegen/templates/forge/go/union.go.jinja2`, over the spellings a Go
/// machine's union is written by (`composed_lowering::go`).
#[test]
fn a_union_is_a_value_struct_with_one_member_per_case_that_holds_a_record() {
    let (file, text) = unit("union_shape_minimal.scxml");
    assert_eq!(file, "union_shape_minimal.go");
    for fragment in [
        "package union_shape_minimal\n".to_string(),
        format!("\tsce_shape_kind \"{PREFIX}/shape_kind\"\n"),
        format!("\tsce_shape_radius_minimal \"{PREFIX}/shape_radius_minimal\"\n"),
        format!("\tsce_shape_rect_minimal \"{PREFIX}/shape_rect_minimal\"\n"),
        format!(
            "type {SHAPE} struct {{\n\ttag {KIND}\n\tcaseCircle {RADIUS}\n\tcaseRect {RECT}\n}}"
        ),
        format!("func (u {SHAPE}) Kind() {KIND} {{\n\treturn u.tag\n}}"),
        format!(
            "func {SHAPE}Dot() {SHAPE} {{\n\treturn {SHAPE}{{tag: sce_shape_kind.ShapeKindDot}}\n}}"
        ),
        format!(
            "func {SHAPE}Circle(r {RADIUS}) {SHAPE} {{\n\treturn {SHAPE}{{tag: \
             sce_shape_kind.ShapeKindCircle, caseCircle: r}}\n}}"
        ),
        format!(
            "func {SHAPE}Rect(r {RECT}) {SHAPE} {{\n\treturn {SHAPE}{{tag: \
             sce_shape_kind.ShapeKindRect, caseRect: r}}\n}}"
        ),
        format!(
            "func (u {SHAPE}) ProvenCircle() {RADIUS} {{\n\tif u.tag != \
             sce_shape_kind.ShapeKindCircle {{\n\t\tpanic(\"a value of union_shape_minimal read \
             as circle is in another case\")\n\t}}\n\treturn u.caseCircle\n}}"
        ),
        format!("func (u {SHAPE}) ProvenRect() {RECT} {{"),
    ] {
        position(&text, &fragment);
    }
    // An empty case holds nothing and has nothing to read.
    assert!(!text.contains("caseDot"), "{text}");
    assert!(!text.contains("ProvenDot"), "{text}");
    // The document writes rect, dot, circle; the enum declares dot, circle,
    // rect, and the constructors follow the enum.
    let dot = position(&text, &format!("func {SHAPE}Dot()"));
    let circle = position(&text, &format!("func {SHAPE}Circle("));
    let rect = position(&text, &format!("func {SHAPE}Rect("));
    assert!(dot < circle && circle < rect, "{text}");
    // Every constructor is declared before the reads.
    assert!(rect < position(&text, "ProvenCircle()"), "{text}");
}

/// The constants of a union's unit (docs/adr/0092, section 3.1, item 4): its
/// revision is its tag enum's (section 8.1), and its largest text is its
/// largest case, `{"kind":"rect","width":65535,"height":65535}`.
#[test]
fn a_union_unit_names_its_revision_its_largest_text_and_its_schema() {
    let (_, text) = unit("union_shape_minimal.scxml");
    for fragment in [
        "const Revision uint32 = 1\n",
        "const DataMaxLen uint64 = 44\n",
        "const SchemaID string = \"urn:sce:schema:union_shape_minimal:1\"\n",
        "const JSONSchema string = \"",
    ] {
        position(&text, fragment);
    }
    assert!(text.contains("oneOf"), "{text}");
    assert!(!text.contains("No JSONSchema"), "{text}");
}

/// Pinned by `SchemaUnitNames::union` (`sce-build/src/forge/generator.rs`)
/// and the Go arm of `schema_unit_type::unit_type`.
#[test]
fn a_payload_names_the_value_struct_at_every_depth() {
    let (file, text) = unit("event_shapes_minimal.scxml");
    assert_eq!(file, "event_shapes_minimal.go");
    for fragment in [
        format!("\tsce_union_shape_minimal \"{PREFIX}/union_shape_minimal\"\n"),
        "type EventShapesMinimalPayload struct {".to_string(),
        "\tOne sce_union_shape_minimal.UnionShapeMinimal\n".to_string(),
        "\tMany []sce_union_shape_minimal.UnionShapeMinimal\n".to_string(),
        "\tMaybe optional.Of[sce_union_shape_minimal.UnionShapeMinimal]\n".to_string(),
        "const ManyMaxCount uint32 = 3\n".to_string(),
    ] {
        position(&text, &fragment);
    }
}

/// A package that uses every part of the union's unit: a case of each kind
/// built, a payload that holds the union at each depth, a proven read, and the
/// tag, which tells every case apart.
const PROBE: &str = r#"package probe

import (
	"example.com/generated/event_shapes_minimal"
	"example.com/generated/shape_kind"
	"example.com/generated/shape_radius_minimal"
	"example.com/generated/shape_rect_minimal"
	"example.com/generated/union_shape_minimal"

	"github.com/newmassrael/sce-forge-runtime/optional"
)

// Named tells every case apart by its tag and reads a rect's record through
// its proven read.
func Named(shape union_shape_minimal.UnionShapeMinimal) string {
	switch shape.Kind() {
	case shape_kind.ShapeKindDot:
		return "dot"
	case shape_kind.ShapeKindCircle:
		return "circle"
	}
	rect := shape.ProvenRect()
	if rect.Width > rect.Height {
		return "wide rect"
	}
	return "rect"
}

// RadiusOf reads a circle's radius through its proven read.
func RadiusOf(shape union_shape_minimal.UnionShapeMinimal) uint16 {
	return shape.ProvenCircle().Radius
}

// Circle is the circle of radius.
func Circle(radius uint16) union_shape_minimal.UnionShapeMinimal {
	return union_shape_minimal.UnionShapeMinimalCircle(shape_radius_minimal.ShapeRadiusMinimalPayload{Radius: radius})
}

// Sent is a payload that holds the union alone, in a list and optionally.
func Sent() event_shapes_minimal.EventShapesMinimalPayload {
	return event_shapes_minimal.EventShapesMinimalPayload{
		One: Circle(2),
		Many: []union_shape_minimal.UnionShapeMinimal{
			union_shape_minimal.UnionShapeMinimalRect(shape_rect_minimal.ShapeRectMinimalPayload{Width: 3, Height: 4}),
			union_shape_minimal.UnionShapeMinimalDot(),
		},
		Maybe: optional.None[union_shape_minimal.UnionShapeMinimal](),
	}
}
"#;

/// The probe's test: a value reads the case it was built in, two values of one
/// case and one record are equal and of another record are not, and a read of
/// another case panics.
const PROBE_TEST: &str = r#"package probe

import (
	"testing"

	"example.com/generated/union_shape_minimal"
)

func TestAUnionReadsTheCaseItWasBuiltIn(t *testing.T) {
	if RadiusOf(Circle(2)) != 2 {
		t.Fatal("a circle of radius 2 reads another radius")
	}
	if Named(Sent().Many[0]) != "rect" || Named(Sent().Many[1]) != "dot" || Named(Circle(1)) != "circle" {
		t.Fatal("a case is told apart from the others by its tag")
	}
	if Circle(2) != Circle(2) {
		t.Error("two circles of one radius are not equal")
	}
	if Circle(2) == Circle(3) {
		t.Error("two circles of two radii are equal")
	}
}

func TestAReadOfAnotherCasePanics(t *testing.T) {
	defer func() {
		if recover() == nil {
			t.Error("a dot read as a circle did not panic")
		}
	}()
	RadiusOf(union_shape_minimal.UnionShapeMinimalDot())
}
"#;

/// The units, written each into the package directory of its document below
/// the module [`PREFIX`], beside the probe, build and the probe's test passes:
/// the struct, the constructors, the reads and the payload are Go the
/// compiler accepts, and they do what the ADR's Go row says.
#[test]
fn the_go_of_a_union_and_the_payload_that_holds_it_builds_and_reads_its_cases() {
    let Some(go_bin) = sce_build::toolchain::require_or_skip("go", "build a generated Go union")
    else {
        return;
    };
    let dir = tempfile::tempdir().expect("a scratch directory");
    std::fs::write(
        dir.path().join("go.mod"),
        format!(
            "module {PREFIX}\n\ngo 1.22\n\n\
             require github.com/newmassrael/sce-forge-runtime v0.0.0\n\n\
             replace github.com/newmassrael/sce-forge-runtime => {}\n",
            go_runtime()
                .canonicalize()
                .expect("the Go forge runtime")
                .display()
        ),
    )
    .expect("write go.mod");
    for name in [
        "shape_kind.scxml",
        "shape_radius_minimal.scxml",
        "shape_rect_minimal.scxml",
        "union_shape_minimal.scxml",
        "event_shapes_minimal.scxml",
    ] {
        let (file, text) = unit(name);
        let package = dir.path().join(file.trim_end_matches(".go"));
        std::fs::create_dir_all(&package).expect("the package directory");
        std::fs::write(package.join(&file), text).expect("write the unit");
    }
    let probe = dir.path().join("probe");
    std::fs::create_dir_all(&probe).expect("the probe's directory");
    std::fs::write(probe.join("probe.go"), PROBE).expect("write the probe");
    std::fs::write(probe.join("probe_test.go"), PROBE_TEST).expect("write the probe's test");
    let out = Command::new(&go_bin)
        .args(["test", "./..."])
        .current_dir(dir.path())
        .output()
        .expect("go runs");
    assert!(
        out.status.success(),
        "the generated Go does not build or its probe fails:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// The refusal Go gives the union `union`, written as `<name>.scxml` beside
/// `documents`, each `(file, text)`, and the record of
/// `shape_radius_minimal.scxml`; its detail.
fn refused_as(name: &str, union: &str, documents: &[(&str, &str)]) -> String {
    let dir = tempfile::tempdir().expect("a scratch directory");
    std::fs::copy(
        fixtures().join("shape_radius_minimal.scxml"),
        dir.path().join("shape_radius_minimal.scxml"),
    )
    .expect("copy the record");
    for (file, text) in documents {
        std::fs::write(dir.path().join(file), text).expect("write a document");
    }
    let path = dir.path().join(format!("{name}.scxml"));
    std::fs::write(&path, union).expect("write the union");
    let err = match go(&path) {
        Ok(_) => panic!("Go generated the union"),
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

/// The refusal Go gives the union `probe_union`, as [`refused_as`] does.
fn refused(union: &str, documents: &[(&str, &str)]) -> String {
    refused_as("probe_union", union, documents)
}

/// A union document `probe_union` whose tag is a value of the enum in
/// `enum_file`, which imports `shape_radius_minimal.scxml` as `Radius`, and
/// whose cases are `cases`.
fn union_document(enum_file: &str, cases: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="union" name="probe_union">
  <sce:import kind="enum" src="{enum_file}" as="Kind"/>
  <sce:import kind="event-schema" src="shape_radius_minimal.scxml" as="Radius"/>
  <datamodel>
    <data id="kind" sce:type="enum:Kind">
{cases}    </data>
  </datamodel>
</scxml>
"#
    )
}

/// An enum `pair_kind` of the two variants `first` and `second`.
fn pair_enum(first: &str, second: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="enum" name="pair_kind" sce:underlying-type="uint8">
  <datamodel>
    <data id="variants">
      <sce:variant name="{first}" value="0"/>
      <sce:variant name="{second}" value="1"/>
    </data>
  </datamodel>
</scxml>
"#
    )
}

/// Two variants an enum tells apart, `dot` and `Dot`, are one Go constructor,
/// which two cases cannot both be.
#[test]
fn two_cases_of_one_go_constructor_are_refused_by_name() {
    let pair = pair_enum("dot", "Dot");
    let detail = refused(
        &union_document(
            "pair_kind.scxml",
            "      <sce:case variant=\"dot\"/>\n      <sce:case variant=\"Dot\" record=\"Radius\"/>\n",
        ),
        &[("pair_kind.scxml", pair.as_str())],
    );
    assert!(
        detail.contains(
            "the cases `dot` and `Dot` are both built by the Go function `ProbeUnionDot`"
        ),
        "{detail}"
    );
}

/// The case `iD` of the union `probe_union` is built by `ProbeUnionID`, which
/// is not a constant of the unit; a union named `schema` would build its case
/// `iD` by `SchemaID`, the unit's constant, which the unit cannot declare
/// twice. The union's name is its document's, so the refusal is asked of a
/// union named `schema`.
#[test]
fn a_case_built_by_the_name_of_a_constant_is_refused_by_name() {
    let pair = pair_enum("iD", "other");
    let named_schema = union_document(
        "pair_kind.scxml",
        "      <sce:case variant=\"iD\"/>\n      <sce:case variant=\"other\" record=\"Radius\"/>\n",
    )
    .replace("name=\"probe_union\"", "name=\"schema\"");
    let detail = refused_as(
        "schema",
        &named_schema,
        &[("pair_kind.scxml", pair.as_str())],
    );
    assert!(
        detail.contains(
            "the case `iD` is built by `SchemaID`, the name of a constant the unit declares"
        ),
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
    let kinds = std::fs::read_to_string(fixtures().join("shape_kind.scxml")).expect("the enum");
    let union = union_document(
        "shape_kind.scxml",
        "      <sce:case variant=\"dot\"/>\n      <sce:case variant=\"circle\" record=\"Radius\"/>\n      <sce:case variant=\"rect\" record=\"Empty\"/>\n",
    )
    .replace(
        "  <datamodel>",
        "  <sce:import kind=\"event-schema\" src=\"shape_empty.scxml\" as=\"Empty\"/>\n  <datamodel>",
    );
    let detail = refused(
        &union,
        &[
            ("shape_kind.scxml", kinds.as_str()),
            ("shape_empty.scxml", empty),
        ],
    );
    assert!(
        detail.contains("the case `rect` holds `Empty`, which declares no field"),
        "{detail}"
    );
}
