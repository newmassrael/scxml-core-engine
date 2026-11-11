// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A C11 union's unit and a C11 algorithm over it (docs/adr/0092, section 7,
//! the C11 row, and Landing order step 5), held to the documents under
//! `sce-build/tests/fixtures/union/` that the Rust, Kotlin, Go and C++ lanes
//! are held to (`a_cpp_algorithm_narrows_a_union_by_its_tag.rs`):
//!
//! * `union_shape_minimal.scxml` generates a struct of its tag and a C `union`
//!   of one member per case that holds a record, in the tag enum's order, each
//!   the payload struct of its case's event schema; the declared name of the
//!   case a value is in; and a constructor per case, which zeroes the rest of
//!   the value.
//! * `algorithm_shape_extent.scxml` takes that struct and reads each case's
//!   fields where a test of the tag proves the case: the tag as its member, a
//!   case's field as that case's member of `as`. Its three tests each return,
//!   so no case is left after them (section 6.1 item 4): C11 closes the body
//!   with `abort`.
//! * `algorithm_shape_grown.scxml` returns a union: the one it was given, by
//!   name, or a union local it builds as the case its tag names, by the
//!   constructor of the case around a compound literal of the record's payload
//!   struct.
//!
//! What the generated units do when they run is not run here, as on the other
//! lanes: no committed tree and no CMake target holds these documents.

use std::path::{Path, PathBuf};

use sce_build::generator::Language;
use sce_build::ForgeCompileOptions;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/union")
}

/// The one C11 header the fixture `name` generates, as `(file, text)`.
fn unit(name: &str) -> (String, String) {
    let path = fixtures().join(name);
    let output =
        sce_build::compile_forge_file(&path, Language::C11, &[], &ForgeCompileOptions::default())
            .unwrap_or_else(|e| panic!("{name} does not generate on C11: {}", e.error));
    let mut files: Vec<(String, String)> = output
        .files
        .into_iter()
        .filter(|(file, _)| file.ends_with(".h"))
        .collect();
    assert_eq!(files.len(), 1, "{name}: one C11 header, got {files:?}");
    files.remove(0)
}

/// Whether `text` holds `needle`, with `text` in the failure.
fn holds(text: &str, needle: &str) {
    assert!(text.contains(needle), "the unit holds `{needle}`:\n{text}");
}

const SHAPE: &str = "union_shape_minimal_t";

/// Pinned by `render_union_c11` (`sce-build/src/forge/generator.rs`) and
/// `tools/codegen/templates/forge/c/union.h.jinja2`, over the spellings a C11
/// machine's union is named by (`composed_lowering/c11.rs`).
#[test]
fn a_c11_unit_holds_a_union_as_a_struct_of_its_tag_and_its_cases() {
    let (file, text) = unit("union_shape_minimal.scxml");
    assert_eq!(file, "union_shape_minimal.h");
    for needle in [
        "#include <stdlib.h>\n#include <string.h>\n",
        "#include \"shape_kind.h\"\n",
        "#include \"shape_radius_minimal.h\"\n",
        "#include \"shape_rect_minimal.h\"\n",
        "#define UNION_SHAPE_MINIMAL_REVISION UINT32_C(1)\n",
        "#define UNION_SHAPE_MINIMAL_SCHEMA_ID \"urn:sce:schema:union_shape_minimal:1\"\n",
        "typedef struct {\n    ShapeKind_t tag;\n    union {\n        \
         ShapeRadiusMinimalPayload_t circle;\n        ShapeRectMinimalPayload_t rect;\n    } \
         as;\n} union_shape_minimal_t;\n",
        "static inline const char *union_shape_minimal_variant(const union_shape_minimal_t \
         *value) {\n    if (value->tag == SHAPE_KIND_DOT) {\n        return \"dot\";\n    }\n",
        "static inline union_shape_minimal_t union_shape_minimal_case_dot(void) {\n    \
         union_shape_minimal_t value;\n    memset(&value, 0, sizeof(value));\n    value.tag = \
         SHAPE_KIND_DOT;\n    return value;\n}\n",
        "static inline union_shape_minimal_t \
         union_shape_minimal_case_circle(ShapeRadiusMinimalPayload_t record) {\n",
        "    value.as.circle = record;\n",
    ] {
        holds(&text, needle);
    }
    // An empty case holds no member.
    assert!(!text.contains(" dot;"), "{text}");
}

/// Pinned by `c11_unit_union_read` (`sce-build/src/forge/composed_lowering/c11.rs`),
/// its arm in `lower_union_reads` (`sce-build/src/forge/expr.rs`) and the C11
/// arm of `unreached_end` (`sce-build/src/forge/generator.rs`).
#[test]
fn a_c11_algorithm_reads_each_case_of_its_union_where_its_tag_is_tested() {
    let (file, text) = unit("algorithm_shape_extent.scxml");
    assert_eq!(file, "algorithm_shape_extent.h");
    holds(&text, "#include \"union_shape_minimal.h\"\n");
    holds(&text, &format!("{SHAPE} shape"));
    // The tag is read as its member.
    holds(&text, "shape.tag == SHAPE_KIND_DOT");
    holds(&text, "shape.as.circle.radius");
    holds(&text, "shape.as.rect.width");
    holds(&text, "shape.as.rect.height");
    // Every case returned before the end of the body, which no path reaches.
    holds(
        &text,
        "abort(); /* algorithm_shape_extent: every case of its union returned before the end \
         of its body */\n}",
    );
}

/// Pinned by the C11 arms of `algorithm_union_case` and of the union local's
/// lowering (`lower_algorithm_stmt`, `sce-build/src/forge/generator.rs`).
#[test]
fn a_c11_algorithm_returns_a_union_it_was_given_or_built() {
    let (_, text) = unit("algorithm_shape_grown.scxml");
    holds(&text, "#include \"shape_radius_minimal.h\"\n");
    holds(&text, &format!("{SHAPE} shape"));
    holds(&text, "shape.tag != SHAPE_KIND_DOT");
    holds(&text, "return shape;");
    holds(
        &text,
        &format!(
            "const {SHAPE} grown = union_shape_minimal_case_circle((ShapeRadiusMinimalPayload_t){{ \
             .radius = "
        ),
    );
    holds(&text, "return grown;");
    // Its last statement returns, so the body needs nothing after it.
    assert!(!text.contains("abort();"), "{text}");
}
