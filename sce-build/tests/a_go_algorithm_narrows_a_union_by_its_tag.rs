// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A Go algorithm holds a union (docs/adr/0092, Landing order step 5), held
//! to the documents under `sce-build/tests/fixtures/union/` that the Rust and
//! Kotlin lanes are held to (`a_rust_algorithm_narrows_a_union_by_its_tag.rs`,
//! `a_kotlin_algorithm_narrows_a_union_by_its_tag.rs`):
//!
//! * `algorithm_shape_extent.scxml` takes a `union:Shape`, the value struct the
//!   union's own unit declares, and reads each case's fields where a test of
//!   the tag proves the case: the tag by the struct's `Kind()`, a case's field
//!   through the unit's `Proven<Case>()` and its Pascal name. Its three tests
//!   each return, so no case is left after them (section 6.1 item 4): Go
//!   closes the body with a `panic`, at the body's own indentation.
//! * `algorithm_shape_grown.scxml` returns a union: the one it was given, by
//!   name, or a union local it builds as the case its tag names, by the
//!   constructor of the case around the record's payload struct.
//! * Both build with `go` beside the union's unit and the units it names, with
//!   a package that hands the one's union to the other, whose test holds what
//!   each case's extent is.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::forge::error::{ForgeError, Located};
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

/// The one Go file the document at `path` generates, as `(file, text)`.
fn unit_at(path: &Path) -> (String, String) {
    let output = go(path)
        .unwrap_or_else(|e| panic!("{} does not generate on Go: {}", path.display(), e.error));
    let mut files: Vec<(String, String)> = output
        .files
        .into_iter()
        .filter(|(file, _)| file.ends_with(".go"))
        .collect();
    assert_eq!(
        files.len(),
        1,
        "{}: one Go unit, got {files:?}",
        path.display()
    );
    files.remove(0)
}

/// The one Go file the fixture `name` generates, as `(file, text)`.
fn unit(name: &str) -> (String, String) {
    unit_at(&fixtures().join(name))
}

/// Whether `text` holds `needle`, with `text` in the failure.
fn holds(text: &str, needle: &str) {
    assert!(text.contains(needle), "the unit holds `{needle}`:\n{text}");
}

const SHAPE: &str = "sce_union_shape_minimal.UnionShapeMinimal";

/// Pinned by `go_unit_union_read` (`sce-build/src/forge/composed_lowering/go.rs`),
/// its arm in `lower_union_reads` (`sce-build/src/forge/expr.rs`) and the Go
/// arm of `unreached_end` (`sce-build/src/forge/generator.rs`).
#[test]
fn a_go_algorithm_reads_each_case_of_its_union_where_its_tag_is_tested() {
    let (file, text) = unit("algorithm_shape_extent.scxml");
    assert_eq!(file, "algorithm_shape_extent.go");
    holds(
        &text,
        &format!("\tsce_union_shape_minimal \"{PREFIX}/union_shape_minimal\"\n"),
    );
    holds(
        &text,
        &format!("func AlgorithmShapeExtent(shape {SHAPE}) uint16 {{"),
    );
    // The tag is read by the struct's method and compared with the variant of
    // the algorithm's own import of the enum.
    holds(&text, "shape.Kind() == sce_shape_kind.ShapeKindDot");
    holds(&text, "shape.Kind() == sce_shape_kind.ShapeKindRect");
    holds(&text, "shape.ProvenCircle().Radius");
    holds(&text, "shape.ProvenRect().Width");
    holds(&text, "shape.ProvenRect().Height");
    // Every case returned before the end of the body, which no path reaches.
    holds(
        &text,
        "\n    panic(\"algorithm_shape_extent: every case of its union returned before the end \
         of its body\")\n}",
    );
}

/// Pinned by the Go arms of `algorithm_union_case` and of the union local's
/// lowering (`lower_algorithm_stmt`, `sce-build/src/forge/generator.rs`).
#[test]
fn a_go_algorithm_returns_a_union_it_was_given_or_built() {
    let (_, text) = unit("algorithm_shape_grown.scxml");
    // The case's record is named by the build, through its package.
    holds(
        &text,
        &format!("\tsce_shape_radius_minimal \"{PREFIX}/shape_radius_minimal\"\n"),
    );
    holds(&text, &format!("radius uint16) {SHAPE} {{"));
    holds(&text, "shape.Kind() != sce_shape_kind.ShapeKindDot");
    holds(&text, "return shape\n");
    holds(
        &text,
        "grown := sce_union_shape_minimal.UnionShapeMinimalCircle(\
         sce_shape_radius_minimal.ShapeRadiusMinimalPayload{Radius: radius})\n",
    );
    holds(&text, "return grown\n");
    // Its last statement returns, so the body needs nothing after it.
    assert!(!text.contains("panic("), "{text}");
}

/// A package that hands the one algorithm's union to the other.
const PROBE: &str = r#"package probe

import (
	"example.com/generated/algorithm_shape_extent"
	"example.com/generated/algorithm_shape_grown"
	"example.com/generated/union_shape_minimal"
)

// GrownExtent is the extent of the shape grown into a circle of radius 3 where
// it is a dot.
func GrownExtent(shape union_shape_minimal.UnionShapeMinimal) uint16 {
	return algorithm_shape_extent.AlgorithmShapeExtent(algorithm_shape_grown.AlgorithmShapeGrown(shape, 3))
}
"#;

/// The probe's test: each case's extent, a dot grown first.
const PROBE_TEST: &str = r#"package probe

import (
	"testing"

	"example.com/generated/shape_radius_minimal"
	"example.com/generated/shape_rect_minimal"
	"example.com/generated/union_shape_minimal"
)

func TestEachCaseHasTheExtentItsRecordGives(t *testing.T) {
	if got := GrownExtent(union_shape_minimal.UnionShapeMinimalDot()); got != 3 {
		t.Errorf("a dot grown into a circle of radius 3 has the extent %d", got)
	}
	circle := union_shape_minimal.UnionShapeMinimalCircle(shape_radius_minimal.ShapeRadiusMinimalPayload{Radius: 5})
	if got := GrownExtent(circle); got != 5 {
		t.Errorf("a circle of radius 5 has the extent %d", got)
	}
	for _, c := range []struct{ width, height, want uint16 }{{1, 2, 2}, {7, 4, 7}} {
		rect := union_shape_minimal.UnionShapeMinimalRect(shape_rect_minimal.ShapeRectMinimalPayload{Width: c.width, Height: c.height})
		if got := GrownExtent(rect); got != c.want {
			t.Errorf("a rect of %d by %d has the extent %d", c.width, c.height, got)
		}
	}
}
"#;

/// The algorithms, beside the union's unit and the units it names, written
/// each into the package directory of its document below the module
/// [`PREFIX`], with the probe: the reads, the build, the returns and the closed
/// end are Go the compiler accepts, and each case's extent is the one its
/// record gives.
#[test]
fn the_go_of_an_algorithm_over_a_union_builds_and_answers_each_case() {
    let Some(go_bin) =
        sce_build::toolchain::require_or_skip("go", "build a generated algorithm over a union")
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
        "algorithm_shape_extent.scxml",
        "algorithm_shape_grown.scxml",
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

/// A field two cases share, read where a test leaves both, is read by a
/// function literal of the field's type whose `switch` takes each case apart.
/// The documents are this test's alone, written into a directory of their own.
#[test]
fn a_field_two_cases_share_is_read_by_a_switch_over_the_cases_left() {
    let dir = tempfile::tempdir().expect("a scratch directory");
    for fixture in ["shape_radius_minimal.scxml", "shape_rect_minimal.scxml"] {
        std::fs::copy(fixtures().join(fixture), dir.path().join(fixture)).expect("copy a fixture");
    }
    let kinds = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="enum" name="round_kind" sce:underlying-type="uint8">
  <datamodel>
    <data id="variants">
      <sce:variant name="circle" value="0"/>
      <sce:variant name="disc" value="1"/>
      <sce:variant name="rect" value="2"/>
    </data>
  </datamodel>
</scxml>
"#;
    let union = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="union" name="round_shape">
  <sce:import kind="enum" src="round_kind.scxml" as="RoundKind"/>
  <sce:import kind="event-schema" src="shape_radius_minimal.scxml" as="Radius"/>
  <sce:import kind="event-schema" src="shape_rect_minimal.scxml" as="Rect"/>
  <datamodel>
    <data id="kind" sce:type="enum:RoundKind">
      <sce:case variant="circle" record="Radius"/>
      <sce:case variant="disc" record="Radius"/>
      <sce:case variant="rect" record="Rect"/>
    </data>
  </datamodel>
</scxml>
"#;
    let algorithm = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="round_radius" version="1.0">
  <sce:import kind="enum" src="round_kind.scxml" as="RoundKind"/>
  <sce:import kind="union" src="round_shape.scxml" as="Round"/>
  <sce:signature>
    <sce:param name="shape" type="union:Round"/>
    <sce:return type="uint16"/>
  </sce:signature>
  <sce:body>
    <sce:if cond="shape.kind === RoundKind.rect">
      <sce:return expr="shape.width"/>
    </sce:if>
    <sce:return expr="shape.radius"/>
  </sce:body>
</scxml>
"#;
    std::fs::write(dir.path().join("round_kind.scxml"), kinds).expect("write the enum");
    std::fs::write(dir.path().join("round_shape.scxml"), union).expect("write the union");
    let path = dir.path().join("round_radius.scxml");
    std::fs::write(&path, algorithm).expect("write the algorithm");
    let (_, text) = unit_at(&path);
    holds(
        &text,
        "func() uint16 { switch shape.Kind().DeclaredName().Proven() { case \"circle\": return \
         shape.ProvenCircle().Radius; case \"disc\": return shape.ProvenDisc().Radius; }; \
         panic(\"a value of round_shape read as one of its cases circle, disc is in another \
         case\") }()",
    );
}
