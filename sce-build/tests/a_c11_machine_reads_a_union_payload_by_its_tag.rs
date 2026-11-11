// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A C11 `sce-static` machine holds a union (docs/adr/0092, sections 4, 6.1
//! and 7, the C11 row), held to
//! `sce-build/tests/fixtures/static_datamodel/static_union.scxml`:
//!
//! * The union is the machine's own struct of its tag and a C `union` of its
//!   cases' records, built whole by the constructor of its case, which zeroes
//!   the rest of the value first.
//! * The payload's reader reads each union's tag first
//!   (`sce_payload_union_open`), then the record of the case the tag names, by
//!   a function of the machine guarded by its name; a `<sce:lift>` reads a text
//!   into a union by the same function (`sce_payload_lift_value`), its refused
//!   path written as the refusal's pointer.
//! * Each read is spelled by the cases the tests leave: the tag as `.tag`, a
//!   field of the one case left as its case's member of `as`, and a field two
//!   cases share by a test of the tag over them.
//! * A field of a case is written in place, as its case's member.
//! * The machine's saved form writes a union as its tag beside its case's
//!   members and reads it back by its tag first; a case record that holds a
//!   list writes and reads it by its type, held to its bound.
//!
//! What the generated machine does at run time is held by the scenario
//! `static_union.json`, replayed over the machine CMake generates by
//! `backends/c/tests/integration/test_static_scalars.c`
//! (`c11_integration_static_scalars`).

use std::path::{Path, PathBuf};

use sce_build::generator::Language;
use sce_build::{compile_scxml_lang_typed, find_template_dir_for};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/static_datamodel")
}

/// The C11 machine of the fixture: every file it is generated as, one after
/// the other.
fn machine() -> String {
    let language = Language::C11;
    let path = fixtures().join("static_union.scxml");
    let output = compile_scxml_lang_typed(
        path.to_str().expect("a utf-8 path"),
        &find_template_dir_for(language),
        language,
    )
    .unwrap_or_else(|e| panic!("C11 holds a union: {e:?}"));
    let names: Vec<String> = output.files.iter().map(|(name, _)| name.clone()).collect();
    assert!(
        names.iter().any(|name| name.ends_with("_sm.h")),
        "the machine's header is generated among {names:?}"
    );
    output
        .files
        .into_iter()
        .map(|(_, code)| code)
        .collect::<Vec<_>>()
        .join("\n")
}

/// `code` holds each of `lines`.
fn holds(code: &str, lines: &[&str]) {
    for line in lines {
        assert!(code.contains(line), "expected `{line}` in:\n{code}");
    }
}

const UNION: &str = "static_union_union_union_shape_t";

/// Pinned by `C11Composed::union_type` and `C11Composed::union_def`
/// (`sce-build/src/forge/composed_lowering/c11.rs`). The tag is held in the
/// machine's own type of the tag's enum, the one every variable of the enum is
/// held in: named by the enum's document, `enum_shape_kind.scxml`
/// (`CTarget::enum_type`, `sce-build/src/forge/static_lowering.rs`).
#[test]
fn the_payload_holds_the_union_in_the_machines_own_struct() {
    let code = machine();
    holds(
        &code,
        &[
            &format!(
                "typedef struct {{\n    enum_shape_kind_enum_t tag;\n    union {{\n        \
                 static_union_record_radius_t circle;\n        static_union_record_radius_t \
                 disc;\n        static_union_record_rect_t rect;\n        \
                 static_union_record_label_t label;\n        static_union_record_path_t \
                 path;\n    }} as;\n}} {UNION};\n"
            ),
            &format!("static inline {UNION} static_union_union_union_shape_case_dot(void) {{\n"),
            &format!(
                "static inline {UNION} static_union_union_union_shape_case_rect(\
                 static_union_record_rect_t record) {{\n"
            ),
            &format!(
                "static inline uint16_t *static_union_union_union_shape_proven_field_radius_place(\
                 {UNION} *value) {{\n"
            ),
            &format!("static inline const char *static_union_union_union_shape_variant(const {UNION} *value) {{\n"),
        ],
    );
}

/// Pinned by `C11Composed::read_union` and `C11Composed::reader_of`
/// (`sce-build/src/forge/composed_lowering/c11.rs`): the reader of a union
/// reads its tag first, then the case it names, and every reader stands under
/// a guard of its name, so the payload's channel and the lift, each of which
/// reads a union, define one function between them.
#[test]
fn the_reader_reads_the_tag_first_under_a_guard_of_its_name() {
    let code = machine();
    holds(
        &code,
        &[
            "#ifndef SCE_COMPOSED_READ_STATIC_UNION_UNION_UNION_SHAPE\n\
             #define SCE_COMPOSED_READ_STATIC_UNION_UNION_UNION_SHAPE\n",
            "static const char *const sce_names[] = { \"dot\", \"circle\", \"disc\", \"rect\", \
             \"label\", \"path\", };",
            "const char *sce_refusal = sce_payload_union_open(sce_value, \"kind\", sce_names, \
             sizeof(sce_names) / sizeof(sce_names[0]), &sce_index, &sce_at_tag);",
            "*sce_out = static_union_union_union_shape_case_rect(sce_case);",
        ],
    );
    let guards = code
        .matches("#ifndef SCE_COMPOSED_READ_STATIC_UNION_UNION_UNION_SHAPE\n")
        .count();
    let definitions = code
        .matches("static const char *sce_composed_read_static_union_union_union_shape(")
        .count();
    assert!(
        guards >= 1 && guards == definitions,
        "each definition of the union's reader stands under its guard: {guards} guards, \
         {definitions} definitions"
    );
}

/// Pinned by `c11_union_member`, `c11_shared_union_read` and the read the
/// expression lowering spells by the cases the facts leave
/// (`lower_union_reads`, `sce-build/src/forge/expr.rs`), and by
/// `C11Composed::union_member_place` and the `<assign>` that writes through it
/// (`union_field_place`, `sce-build/src/forge/static_lowering.rs`).
#[test]
fn each_read_and_write_is_spelled_by_the_cases_the_tests_leave() {
    let code = machine();
    holds(
        &code,
        &[
            // The tag as its member, against the constants of the machine's enum.
            ".one.tag == ENUM_SHAPE_KIND_DOT",
            // A field of the one case left, as that case's member.
            ".one.as.rect.width",
            // A field two cases share, by a test of the tag over them.
            ".one.tag == ENUM_SHAPE_KIND_CIRCLE ? ",
            ".one.as.circle.radius : ",
            ".one.as.disc.radius",
            // A list a case holds, walked where its case is proven.
            ".one.as.path.points",
            // A field of the built variable's proven case, read and written in place.
            "sm->policy.v_built.as.rect.width",
        ],
    );
}

/// Pinned by `union_variable_field` (`sce-build/src/forge/static_lowering.rs`),
/// `C11Composed::union_value` and `C11Composed::copy_present_into`: a variable
/// of the union starts at the case its `<sce:set>`s build, one that may be
/// absent starts absent, and a union put into one whole is made present.
#[test]
fn a_union_variable_starts_built_or_absent_and_is_put_in_whole() {
    let code = machine();
    holds(
        &code,
        &[
            "static_union_union_union_shape_case_rect((static_union_record_rect_t){ .width = ",
            "sm->policy.v_kept = sce_optional_record_static_union_union_union_shape_none();",
            "sm->policy.v_lifted = sce_optional_record_static_union_union_union_shape_none();",
            "sce_optional_record_static_union_union_union_shape_some((",
            "SCE_OPTIONAL_DECLARE_RECORD(record_static_union_union_union_shape, \
             static_union_union_union_shape_t)",
        ],
    );
}

/// Pinned by `C11Composed::saved_union_ext`, `C11Composed::saved_record_ext`,
/// the members a record of entries writes (`CTarget::record_def`) and the
/// saved form the C11 template writes a variable by
/// (`tools/codegen/templates/c/saved.c.jinja2`).
#[test]
fn a_saved_union_is_its_tag_beside_its_case_and_is_read_back_by_its_tag() {
    let code = machine();
    holds(
        &code,
        &[
            &format!(
                "static inline void {UNION}_saved_write(sce_jw_t *w, const {UNION} *value) {{\n"
            ),
            "sce_saved_write_union_tag(w, \"kind\", declared != NULL ? declared : \"\");",
            "static_union_record_radius_t_saved_write_members(w, &value->as.circle);",
            "if (!sce_saved_read_union_tag(v, what, \"kind\", \"union_shape\", names, \
             sizeof(names) / sizeof(names[0]), &index, err)) {",
            // A record of entries writes its members by a function of their own.
            "static inline void static_union_record_radius_t_saved_write_members(sce_jw_t *w, \
             const static_union_record_radius_t *value) {\n",
            // A case record that holds a list writes and reads it by its type,
            // held to its bound.
            "static inline void static_union_record_path_t_saved_write_members(sce_jw_t *w, \
             const static_union_record_path_t *value) {\n",
            "if (!sce_saved_list_within(member, member_what, 0u, 3u, &count_1, err)) {",
            // Each variable writes and reads its union by the union's own form.
            &format!("if (!{UNION}_saved_read(v, what, at, &value, err)) {{"),
            &format!("if (!{UNION}_saved_read(v, what, at, &value.value, err)) {{"),
        ],
    );
}

/// Pinned by the C11 lift (`CTarget::lift`, `sce-build/src/forge/static_lowering.rs`):
/// a text is read into the union whole by the reader a delivery's field of it
/// is read by, the path that reader leaves written as the refusal's pointer and
/// the bound it broke kept for the refusal record.
#[test]
fn a_text_is_lifted_into_the_union_by_its_reader() {
    let code = machine();
    holds(
        &code,
        &[
            "char sce_pointer[257u]; char sce_path[257u]; ",
            "uint32_t sce_bound_at = 2u; uint32_t *const sce_bound = &sce_bound_at;",
            "sce_payload_lift_value(",
            "sce_refusal = sce_composed_read_static_union_union_union_shape(sce_at, sce_into, \
             sce_path, sce_path_cap, sce_bound);",
            "(void)sce_payload_pointer_of_path(sce_pointer, sizeof(sce_pointer), sce_path);",
            "<sce:lift>: the text is no value of %s",
            "sce_optional_record_static_union_union_union_shape_some(sce_lifted)",
        ],
    );
}
