// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A Kotlin algorithm holds a union (docs/adr/0092, Landing order step 5), held
//! to the documents under `sce-build/tests/fixtures/union/` that the Rust lane
//! is held to (`a_rust_algorithm_narrows_a_union_by_its_tag.rs`):
//!
//! * `algorithm_shape_extent.scxml` takes a `union:Shape`, the sealed interface
//!   the union's own unit declares, and reads each case's fields where a test of
//!   the tag proves the case: the tag by the interface's `kind`, a case's field
//!   through the unit's `proven<Case>()`. Its three tests each return, so no
//!   case is left after them (section 6.1 item 4): Kotlin closes the body with
//!   a `throw` of `IllegalStateException`, at the body's own indentation, inside
//!   the `try` of a `may-fail` body too.
//! * `algorithm_shape_grown.scxml` returns a union: the one it was given, by
//!   name, or a union local it builds as the case its tag names, a `val` of the
//!   interface's type holding the case's class around the record's payload
//!   class.
//! * Both compile under `kotlinc -Werror` beside the union's unit and the units
//!   it names, with a file that calls each.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::forge::error::{ForgeError, Located};
use sce_build::generator::{GeneratedOutput, Language};
use sce_build::ForgeCompileOptions;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/union")
}

/// What Kotlin generates for the document at `path`.
fn kotlin(path: &Path) -> Result<GeneratedOutput, Located<ForgeError>> {
    sce_build::compile_forge_file(path, Language::Kotlin, &[], &ForgeCompileOptions::default())
}

/// The one Kotlin file the document at `path` generates, as `(file, text)`.
fn unit_at(path: &Path) -> (String, String) {
    let output = kotlin(path).unwrap_or_else(|e| {
        panic!(
            "{} does not generate on Kotlin: {}",
            path.display(),
            e.error
        )
    });
    let mut kt: Vec<(String, String)> = output
        .files
        .into_iter()
        .filter(|(file, _)| file.ends_with(".kt"))
        .collect();
    assert_eq!(
        kt.len(),
        1,
        "{}: one Kotlin unit, got {kt:?}",
        path.display()
    );
    kt.remove(0)
}

/// The one Kotlin file the fixture `name` generates, as `(file, text)`.
fn unit(name: &str) -> (String, String) {
    unit_at(&fixtures().join(name))
}

/// Whether `text` holds `needle`, with `text` in the failure.
fn holds(text: &str, needle: &str) {
    assert!(text.contains(needle), "the unit holds `{needle}`:\n{text}");
}

const SHAPE: &str = "UnionShapeMinimal";

#[test]
fn a_kotlin_algorithm_reads_each_case_of_its_union_where_its_tag_is_tested() {
    let (file, text) = unit("algorithm_shape_extent.scxml");
    assert_eq!(file, "AlgorithmShapeExtent.kt");
    holds(&text, "import com.sce.generated.union_shape_minimal.*\n");
    holds(
        &text,
        &format!("fun algorithmShapeExtent(shape: {SHAPE}): UShort"),
    );
    // The tag is read by the interface's property and compared with the
    // algorithm's own enum.
    holds(&text, "shape.kind");
    holds(&text, "ShapeKind.DOT");
    holds(&text, "ShapeKind.RECT");
    holds(&text, "shape.provenCircle().radius");
    holds(&text, "shape.provenRect().width");
    holds(&text, "shape.provenRect().height");
    // Every case returned before the end of the body, which no path reaches.
    holds(
        &text,
        "\n    throw IllegalStateException(\"algorithm_shape_extent: every case of its union \
         returned before the end of its body\")",
    );
}

#[test]
fn a_kotlin_algorithm_returns_a_union_it_was_given_or_built() {
    let (_, text) = unit("algorithm_shape_grown.scxml");
    // The case's record is named by no expression, and is imported all the same.
    holds(&text, "import com.sce.generated.shape_radius_minimal.*\n");
    holds(&text, &format!("radius: UShort): {SHAPE}"));
    holds(&text, "return shape\n");
    holds(
        &text,
        &format!(
            "val grown: {SHAPE} = {SHAPE}.Circle(ShapeRadiusMinimalPayload(radius = radius))\n"
        ),
    );
    holds(&text, "return grown\n");
    // Its last statement returns, so the body needs nothing after it.
    assert!(!text.contains("IllegalStateException("), "{text}");
}

/// A file that calls both algorithms, the one's union handed to the other.
const PROBE: &str = r#"package probe

import com.sce.generated.algorithm_shape_extent.algorithmShapeExtent
import com.sce.generated.algorithm_shape_grown.algorithmShapeGrown
import com.sce.generated.union_shape_minimal.UnionShapeMinimal

fun grownExtent(shape: UnionShapeMinimal): UShort =
    algorithmShapeExtent(algorithmShapeGrown(shape, 3u))

fun dot(): UnionShapeMinimal = UnionShapeMinimal.Dot
"#;

/// The algorithms beside a file that calls each, compiled with the union's unit
/// and the units it names: the reads, the build, the returns and the closed end
/// are Kotlin the compiler accepts with every warning an error.
#[test]
fn the_kotlin_of_an_algorithm_over_a_union_compiles_under_werror() {
    let Some(kotlinc) = sce_build::toolchain::require_any_or_skip(
        &["kotlinc"],
        "compile a generated algorithm over a union",
    ) else {
        return;
    };
    let dir = tempfile::tempdir().expect("a scratch directory");
    let mut sources = Vec::new();
    for name in [
        "shape_kind.scxml",
        "shape_radius_minimal.scxml",
        "shape_rect_minimal.scxml",
        "union_shape_minimal.scxml",
        "algorithm_shape_extent.scxml",
        "algorithm_shape_grown.scxml",
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

/// A `may-fail` body sits inside the `try` that turns a failure into a value,
/// and the statement that closes it where every case returned stands at that
/// body's indentation, inside the `try`: an `IllegalStateException` is no
/// `AlgorithmFailure`, so the `catch` does not turn it into one.
#[test]
fn a_may_fail_body_is_closed_inside_its_try() {
    let dir = tempfile::tempdir().expect("a scratch directory");
    for fixture in [
        "shape_kind.scxml",
        "shape_radius_minimal.scxml",
        "shape_rect_minimal.scxml",
        "union_shape_minimal.scxml",
    ] {
        std::fs::copy(fixtures().join(fixture), dir.path().join(fixture)).expect("copy a fixture");
    }
    let text = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_shape" version="1.0">
  <sce:import kind="enum" src="shape_kind.scxml" as="ShapeKind"/>
  <sce:import kind="union" src="union_shape_minimal.scxml" as="Shape"/>
  <sce:signature>
    <sce:param name="shape" type="union:Shape"/>
    <sce:return type="uint16" may-fail="true"/>
  </sce:signature>
  <sce:body>
    <sce:if cond="shape.kind === ShapeKind.dot">
      <sce:return expr="0"/>
    </sce:if>
    <sce:if cond="shape.kind === ShapeKind.circle">
      <sce:return expr="shape.radius"/>
    </sce:if>
    <sce:if cond="shape.kind === ShapeKind.rect">
      <sce:return expr="shape.width"/>
    </sce:if>
  </sce:body>
</scxml>
"#;
    let path = dir.path().join("probe_shape.scxml");
    std::fs::write(&path, text).expect("write the probe");
    let (_, text) = unit_at(&path);
    let closed = "\n        throw IllegalStateException(\"probe_shape: every case of its union \
                  returned before the end of its body\")";
    holds(&text, closed);
    let at = text.find(closed).expect("held above");
    let opened = text.find("    try {").expect("a may-fail body opens a try");
    let caught = text
        .find("} catch (failure: com.sce.forge.runtime.AlgorithmFailure)")
        .expect("a may-fail body closes its try");
    assert!(opened < at && at < caught, "{text}");
}
