// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A C++ union's unit and a C++ algorithm over it (docs/adr/0092, section 7,
//! the C++ row, and Landing order step 5), held to the documents under
//! `sce-build/tests/fixtures/union/` that the Rust, Kotlin and Go lanes are
//! held to (`a_go_algorithm_narrows_a_union_by_its_tag.rs`):
//!
//! * `union_shape_minimal.scxml` generates a struct of one `std::variant`, an
//!   alternative struct per case in the tag enum's order, the record of a case
//!   its `value`; `kind()` answers the tag from the alternative's index, and a
//!   proven read per case that holds a record ends the process in another case.
//! * `algorithm_shape_extent.scxml` takes that struct and reads each case's
//!   fields where a test of the tag proves the case: the tag by `kind()`, a
//!   case's field through the unit's `proven<Case>()`. Its three tests each
//!   return, so no case is left after them (section 6.1 item 4): C++ closes
//!   the body with `std::abort`, at the body's own indentation.
//! * `algorithm_shape_grown.scxml` returns a union: the one it was given, by
//!   name, or a union local it builds as the case its tag names, the
//!   alternative of the case around the record's payload struct.
//!
//! What the generated units do when they run is not run here, as on the other
//! lanes: no committed tree and no CMake target holds these documents.

use std::path::{Path, PathBuf};

use sce_build::generator::Language;
use sce_build::ForgeCompileOptions;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/union")
}

/// The one C++ header the fixture `name` generates, as `(file, text)`.
fn unit(name: &str) -> (String, String) {
    let path = fixtures().join(name);
    let output =
        sce_build::compile_forge_file(&path, Language::Cpp, &[], &ForgeCompileOptions::default())
            .unwrap_or_else(|e| panic!("{name} does not generate on C++: {}", e.error));
    let mut files: Vec<(String, String)> = output
        .files
        .into_iter()
        .filter(|(file, _)| file.ends_with(".h"))
        .collect();
    assert_eq!(files.len(), 1, "{name}: one C++ header, got {files:?}");
    files.remove(0)
}

/// Whether `text` holds `needle`, with `text` in the failure.
fn holds(text: &str, needle: &str) {
    assert!(text.contains(needle), "the unit holds `{needle}`:\n{text}");
}

const SHAPE: &str = "SCE::Generated::UnionShapeMinimal::UnionShapeMinimal";

/// Pinned by `render_union_cpp` (`sce-build/src/forge/generator.rs`) and
/// `tools/codegen/templates/forge/cpp/union.h.jinja2`, over the spellings a C++
/// machine's union is named by (`composed_lowering/cpp.rs`).
#[test]
fn a_cpp_unit_holds_a_union_as_a_struct_of_one_variant() {
    let (file, text) = unit("union_shape_minimal.scxml");
    assert_eq!(file, "union_shape_minimal.h");
    for needle in [
        "#include <cstdlib>\n#include <variant>\n",
        "#include \"shape_kind.h\"\n",
        "#include \"shape_radius_minimal.h\"\n",
        "#include \"shape_rect_minimal.h\"\n",
        "namespace SCE::Generated::UnionShapeMinimal {",
        "inline constexpr std::uint32_t REVISION = 1;",
        "inline constexpr const char *SCHEMA_ID = \"urn:sce:schema:union_shape_minimal:1\";",
        "struct UnionShapeMinimal {\n",
        "    /// The case `dot`, its tag alone.\n    struct Dot {};\n",
        "    struct Circle {\n        \
         ::SCE::Generated::ShapeRadiusMinimal::ShapeRadiusMinimalPayload value;\n    };\n",
        "    ::std::variant<Dot, Circle, Rect> cases;\n",
        "    ::SCE::Generated::ShapeKind::ShapeKind kind() const {\n        switch \
         (cases.index()) {\n        case 0:\n            return \
         ::SCE::Generated::ShapeKind::ShapeKind::Dot;\n",
        "    const ::SCE::Generated::ShapeRadiusMinimal::ShapeRadiusMinimalPayload \
         &provenCircle() const {\n        const Circle *held = ::std::get_if<Circle>(&cases);\n        \
         if (held == nullptr) {\n            ::std::abort();\n        }\n        return \
         held->value;\n    }\n",
    ] {
        holds(&text, needle);
    }
    // An empty case has nothing to read.
    assert!(!text.contains("provenDot"), "{text}");
}

/// Pinned by `cpp_unit_union_read` (`sce-build/src/forge/composed_lowering/cpp.rs`),
/// its arm in `lower_union_reads` (`sce-build/src/forge/expr.rs`) and the C++
/// arm of `unreached_end` (`sce-build/src/forge/generator.rs`).
#[test]
fn a_cpp_algorithm_reads_each_case_of_its_union_where_its_tag_is_tested() {
    let (file, text) = unit("algorithm_shape_extent.scxml");
    assert_eq!(file, "algorithm_shape_extent.h");
    holds(&text, "#include \"union_shape_minimal.h\"\n");
    holds(&text, &format!("({SHAPE} shape)"));
    // The tag is read by the struct's method.
    holds(&text, "shape.kind() == ");
    holds(&text, "shape.provenCircle().radius");
    holds(&text, "shape.provenRect().width");
    holds(&text, "shape.provenRect().height");
    // Every case returned before the end of the body, which no path reaches.
    holds(
        &text,
        "\n    ::std::abort(); // algorithm_shape_extent: every case of its union returned \
         before the end of its body\n}",
    );
}

/// Pinned by the C++ arms of `algorithm_union_case` and of the union local's
/// lowering (`lower_algorithm_stmt`, `sce-build/src/forge/generator.rs`).
#[test]
fn a_cpp_algorithm_returns_a_union_it_was_given_or_built() {
    let (_, text) = unit("algorithm_shape_grown.scxml");
    holds(&text, "#include \"shape_radius_minimal.h\"\n");
    holds(&text, &format!("{SHAPE} "));
    holds(&text, "return shape;");
    holds(
        &text,
        &format!(
            "const {SHAPE} grown = {SHAPE}{{{SHAPE}::Circle{{\
             SCE::Generated::ShapeRadiusMinimal::ShapeRadiusMinimalPayload{{/*radius=*/"
        ),
    );
    holds(&text, "return grown;");
    // Its last statement returns, so the body needs nothing after it.
    assert!(!text.contains("::std::abort()"), "{text}");
}
