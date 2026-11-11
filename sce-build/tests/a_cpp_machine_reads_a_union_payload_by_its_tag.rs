// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A C++ `sce-static` machine holds a union (docs/adr/0092, sections 4, 6.1
//! and 7, the C++ row, and its item 2), held to
//! `sce-build/tests/fixtures/static_datamodel/static_union.scxml`:
//!
//! * The union is the machine's own struct of one `std::variant`, an
//!   alternative struct per case holding the case's record, built whole from
//!   the alternative of its case, its tag read by `kind()`.
//! * The lift reads each union's tag first (`readUnion`), then the record of
//!   the case the tag names; a `<sce:lift>` reads a text into a union the same
//!   way (`EventPayloadFields::liftValue`).
//! * Each read is spelled by the cases the tests leave: the tag by `kind()`, a
//!   field of the one case left through that case's proven read, and a field
//!   two cases share through its accessor.
//! * A field of a case is written in place, through the reference the method
//!   beside the proven read answers.
//! * The machine's saved form writes a union as its tag beside its case and
//!   reads it back by its tag first; a case record that holds a list writes
//!   and reads it by its type.
//! * The raise seam holds a list of unions to the lower count its type states.
//!
//! What the generated machine does at run time is held by the scenario
//! `static_union.json`, replayed over the machine CMake generates by
//! `static_datamodel_generated_cpp_test` (`StaticScenario/StaticUnion`).

use std::path::{Path, PathBuf};

use sce_build::generator::Language;
use sce_build::{compile_scxml_lang_typed, find_template_dir_for};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/static_datamodel")
}

/// The C++ machine of the fixture: every file it is generated as, one after the
/// other.
fn machine() -> String {
    let language = Language::Cpp;
    let path = fixtures().join("static_union.scxml");
    let output = compile_scxml_lang_typed(
        path.to_str().expect("a utf-8 path"),
        &find_template_dir_for(language),
        language,
    )
    .unwrap_or_else(|e| panic!("C++ holds a union: {e:?}"));
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

const UNION: &str = "StaticUnionUnionShapeUnion";

/// Pinned by `CppComposed::union_type` and `CppComposed::union_def`
/// (`sce-build/src/forge/composed_lowering/cpp.rs`) and by the includes the
/// machine's header takes for it (`tools/codegen/templates/state_machine.jinja2`).
#[test]
fn the_payload_holds_the_union_in_the_machines_own_struct() {
    let code = machine();
    holds(
        &code,
        &[
            // The header's includes are sorted, so the two are apart (`<string>` falls between).
            "#include <cstdlib>\n",
            "#include <variant>\n",
            &format!(
                "struct {UNION} {{\n    /// The case `dot`, its tag alone.\n    struct Dot {{}};\n"
            ),
            "    struct Circle {\n        StaticUnionRadiusRecord value;\n    };\n",
            "    struct Path {\n        StaticUnionPathRecord value;\n    };\n",
            "    ::std::variant<Dot, Circle, Disc, Rect, Label, Path> cases;\n",
            "    StaticUnionKindEnum kind() const {\n",
            "            return StaticUnionKindEnum::Dot;\n",
            "    const StaticUnionRectRecord &provenRect() const {\n",
            "    StaticUnionRectRecord &provenRectPlace() {\n",
            "    uint16_t provenFieldRadius() const {\n",
            // The lift reads the tag first, then the case it names.
            "sceValue.readUnion<StaticUnionKindEnum>(\"kind\", ",
        ],
    );
}

/// Pinned by `cpp_union_member` and the read the expression lowering spells by
/// the cases the facts leave (`lower_union_reads`, `sce-build/src/forge/expr.rs`),
/// and by `CppComposed::union_member_place` and the `<assign>` that writes
/// through it (`union_field_place`, `sce-build/src/forge/static_lowering.rs`).
#[test]
fn each_read_and_write_is_spelled_by_the_cases_the_tests_leave() {
    let code = machine();
    holds(
        &code,
        &[
            // The tag by its method.
            "pendingShapesSentPayload_.one.kind() == StaticUnionKindEnum::Dot",
            // A field of the one case left, through that case's proven read.
            "pendingShapesSentPayload_.one.provenRect().width",
            // A field two cases share, through its accessor.
            "pendingShapesSentPayload_.one.provenFieldRadius()",
            // A field of the built variable's proven case, written in place.
            "v_built.provenRectPlace().width = ",
            "v_built.provenRect().width",
        ],
    );
}

/// Pinned by `union_variable_field` (`sce-build/src/forge/static_lowering.rs`)
/// and `CppComposed::union_value`: a variable of the union starts at the case
/// its `<sce:set>`s build, and one that may be absent starts absent.
#[test]
fn a_union_variable_starts_built_or_absent() {
    let code = machine();
    holds(
        &code,
        &[
            &format!("{UNION} v_built = {UNION}{{{UNION}::Rect{{StaticUnionRectRecord{{/*width=*/"),
            &format!("std::optional<{UNION}> v_kept = std::optional<{UNION}>();"),
            &format!("std::optional<{UNION}> v_lifted = std::optional<{UNION}>();"),
        ],
    );
}

/// Pinned by `CppComposed::saved_union_ext`, `CppComposed::saved_record_ext` and
/// the saved form the C++ template writes a variable by
/// (`tools/codegen/templates/_macros/cpp_saved_value.jinja2`).
#[test]
fn a_saved_union_is_its_tag_beside_its_case_and_is_read_back_by_its_tag() {
    let code = machine();
    holds(
        &code,
        &[
            &format!("inline ::SCE::Saved::JsonValue {UNION}ToSaved(const {UNION} &value) {{\n"),
            "        return ::SCE::Saved::savedWriteUnion(\"kind\", \"circle\", \
             StaticUnionRadiusRecordToSaved(held->value));\n",
            "        return ::SCE::Saved::savedWriteUnion(\"kind\", \"dot\", ::std::nullopt);\n",
            &format!(
                "inline {UNION} {UNION}FromSaved(const ::SCE::Saved::JsonValue &value, const \
                 std::string &what) {{\n"
            ),
            "::SCE::Saved::savedReadUnionTag(value, what, \"kind\")",
            // A case record that holds a list writes and reads it by its type,
            // held to its bound.
            "inline ::SCE::Saved::JsonValue StaticUnionPathRecordToSaved(const \
             StaticUnionPathRecord &value) {\n",
            "    members.set(\"points\", ([](const auto &sceSaved0) { return \
             ::SCE::Saved::savedWriteList(sceSaved0, ::SCE::Saved::savedWriteUint8); \
             })(value.points));\n",
            "::SCE::Saved::savedReadOwnedListWithin<3>(sceSaved0, sceWhat0, 0u, \
             ::SCE::Saved::savedReadUint8)",
            // Each variable writes and reads its union by the union's own form.
            &format!(".v_kept, {UNION}ToSaved)"),
            &format!("\"kept\", {UNION}FromSaved)"),
            &format!("{UNION}FromSaved("),
        ],
    );
}

/// Pinned by the C++ lift (`CppTarget::lift`, `sce-build/src/forge/static_lowering.rs`):
/// a text is read into the union whole by the reader a delivery's field of it
/// is read by.
#[test]
fn a_text_is_lifted_into_the_union_by_its_reader() {
    let code = machine();
    holds(
        &code,
        &[
            &format!(
                "{{ {UNION} sceLifted{{}}; const ::SCE::Common::EventPayloadRefusal sceRefusal = "
            ),
            "::SCE::Common::EventPayloadFields::liftValue(",
            "\"Shape\", 2u, 256u, sceLifted, [&](const ::SCE::Common::EventPayloadValue \
             &sceValue, auto &sceInto) -> ::SCE::Common::EventPayloadRefusal { return \
             sceValue.readUnion<StaticUnionKindEnum>(\"kind\", ",
            &format!("v_lifted = std::optional<{UNION}>(sceLifted);"),
        ],
    );
}

/// Pinned by `cpp_bound_check` (`sce-build/src/forge/composed_lowering/cpp.rs`):
/// the raise seam of `shapes.sent` holds `many` to the lower count its type
/// states before anything is sent, each union of it checked by the case it is
/// in, whose records hold no lower count.
#[test]
fn the_raise_seam_holds_a_list_of_unions_to_its_lower_count() {
    let code = machine();
    holds(
        &code,
        &[
            "::SCE::Common::EventPayloadBounds::listWithin(::std::string(\"many\"), many, 1u, 3u, \
           [](const ::std::string &, const auto &) -> ::std::string { return {}; })",
        ],
    );
}
