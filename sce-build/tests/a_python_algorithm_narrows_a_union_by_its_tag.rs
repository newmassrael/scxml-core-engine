// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A Python union's unit and a Python algorithm over it (docs/adr/0092,
//! section 7, the Python row, and Landing order step 5), held to the documents
//! under `sce-build/tests/fixtures/union/` that the Rust, Kotlin, Go and C++
//! lanes are held to (`a_cpp_algorithm_narrows_a_union_by_its_tag.rs`):
//!
//! * `union_shape_minimal.scxml` generates a class and one frozen dataclass per
//!   case derived from it, in the tag enum's order, the record of a case its
//!   `value`, checked where it is made; each case answers its tag by a `kind`
//!   its class holds, and a proven read per case that holds a record raises in
//!   another case.
//! * `algorithm_shape_extent.scxml` takes that class and reads each case's
//!   fields where a test of the tag proves the case: the tag by `kind`, a
//!   case's field through the unit's `proven_<case>()`. Its three tests each
//!   return, so no case is left after them (section 6.1 item 4): Python closes
//!   the body with a `raise` of `AssertionError`, at the body's own
//!   indentation.
//! * `algorithm_shape_grown.scxml` returns a union: the one it was given, by
//!   name, or a union local it builds as the case its tag names, the class of
//!   the case around the record's payload class.
//!
//! What the generated modules do when they run is not run here, as on the
//! other lanes: no generated tree holds these documents.

use std::path::{Path, PathBuf};

use sce_build::generator::Language;
use sce_build::ForgeCompileOptions;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/union")
}

/// The one Python module the fixture `name` generates, as `(file, text)`.
fn unit(name: &str) -> (String, String) {
    let path = fixtures().join(name);
    let output = sce_build::compile_forge_file(
        &path,
        Language::Python,
        &[],
        &ForgeCompileOptions::default(),
    )
    .unwrap_or_else(|e| panic!("{name} does not generate on Python: {}", e.error));
    let mut files: Vec<(String, String)> = output
        .files
        .into_iter()
        .filter(|(file, _)| file.ends_with(".py"))
        .collect();
    assert_eq!(files.len(), 1, "{name}: one Python module, got {files:?}");
    files.remove(0)
}

/// Whether `text` holds `needle`, with `text` in the failure.
fn holds(text: &str, needle: &str) {
    assert!(text.contains(needle), "the unit holds `{needle}`:\n{text}");
}

const SHAPE: &str = "sce_union_shape_minimal.UnionShapeMinimal";

/// Pinned by `render_union_python` (`sce-build/src/forge/generator.rs`) and
/// `tools/codegen/templates/forge/python/union.py.jinja2`, over the spellings a
/// Python machine's union is named by (`composed_lowering/python.rs`).
#[test]
fn a_python_unit_holds_a_union_as_a_class_and_a_dataclass_per_case() {
    let (file, text) = unit("union_shape_minimal.scxml");
    assert_eq!(file, "union_shape_minimal.py");
    for needle in [
        "from . import shape_kind as sce_shape_kind\n",
        "from . import shape_radius_minimal as sce_shape_radius_minimal\n",
        "from . import shape_rect_minimal as sce_shape_rect_minimal\n",
        "REVISION: Final[int] = 1\n",
        "SCHEMA_ID: Final[str] = \"urn:sce:schema:union_shape_minimal:1\"\n",
        "class UnionShapeMinimal:\n",
        "    def proven_circle(self) -> \
         sce_shape_radius_minimal.ShapeRadiusMinimalPayload:\n",
        "        if isinstance(self, UnionShapeMinimalCircle):\n            return self.value\n        \
         raise AssertionError(\"a value of union_shape_minimal read as circle is in another \
         case\")\n",
        "@dataclass(frozen=True)\nclass UnionShapeMinimalDot(UnionShapeMinimal):\n    \"\"\"The \
         case `dot`, its tag alone.\"\"\"\n\n    kind = sce_shape_kind.ShapeKind.DOT\n",
        "@dataclass(frozen=True)\nclass UnionShapeMinimalCircle(UnionShapeMinimal):\n    \"\"\"The \
         case `circle`, holding its record.\"\"\"\n\n    value: \
         sce_shape_radius_minimal.ShapeRadiusMinimalPayload\n    kind = \
         sce_shape_kind.ShapeKind.CIRCLE\n",
        "        if not isinstance(self.value, \
         sce_shape_radius_minimal.ShapeRadiusMinimalPayload):\n",
    ] {
        holds(&text, needle);
    }
    // An empty case has nothing to read.
    assert!(!text.contains("proven_dot"), "{text}");
}

/// Pinned by `python_unit_union_read` (`sce-build/src/forge/composed_lowering/python.rs`),
/// its arm in `lower_union_reads` (`sce-build/src/forge/expr.rs`) and the
/// Python arm of `unreached_end` (`sce-build/src/forge/generator.rs`).
#[test]
fn a_python_algorithm_reads_each_case_of_its_union_where_its_tag_is_tested() {
    let (file, text) = unit("algorithm_shape_extent.scxml");
    assert_eq!(file, "algorithm_shape_extent.py");
    holds(
        &text,
        "from . import union_shape_minimal as sce_union_shape_minimal\n",
    );
    holds(&text, &format!("shape: {SHAPE}"));
    // The tag is read by the attribute every case holds.
    holds(&text, "shape.kind == ");
    holds(&text, "shape.proven_circle().radius");
    holds(&text, "shape.proven_rect().width");
    holds(&text, "shape.proven_rect().height");
    // Every case returned before the end of the body, which no path reaches.
    holds(
        &text,
        "\n    raise AssertionError(\"algorithm_shape_extent: every case of its union returned \
         before the end of its body\")",
    );
}

/// Pinned by the Python arms of `algorithm_union_case` and of the union local's
/// lowering (`lower_algorithm_stmt`, `sce-build/src/forge/generator.rs`).
#[test]
fn a_python_algorithm_returns_a_union_it_was_given_or_built() {
    let (_, text) = unit("algorithm_shape_grown.scxml");
    holds(
        &text,
        "from . import shape_radius_minimal as sce_shape_radius_minimal\n",
    );
    holds(&text, &format!("-> {SHAPE}:"));
    holds(&text, "return shape\n");
    holds(
        &text,
        &format!(
            "grown = {SHAPE}Circle(sce_shape_radius_minimal.ShapeRadiusMinimalPayload(radius="
        ),
    );
    // The body's last statement returns the local, and the module ends there.
    // A render carries no line break after its last line: the file gains one
    // where it is written (`generator::with_trailing_newline`), so the end is
    // read past any.
    assert!(
        text.trim_end().ends_with("\n    return grown"),
        "the unit ends in `return grown`:\n{text}"
    );
    // Its last statement returns, so the body needs nothing after it.
    assert!(!text.contains("raise AssertionError"), "{text}");
}
