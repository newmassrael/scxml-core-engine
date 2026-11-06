// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A Rust `sce-static` machine reads a payload that holds a union
//! (docs/adr/0092, sections 4, 6.1 and 7), held to
//! `sce-build/tests/fixtures/static_datamodel/static_union.scxml`:
//!
//! * The payload struct holds the union alone, in a bounded list and
//!   optionally, as the machine's own sum, declared once with a variant per
//!   case holding the machine's record of the case.
//! * The lift reads each union tag first (`PayloadValue::union`), the record of
//!   the case the tag names after it; the inject seam writes each by its case
//!   (`union_json`).
//! * Each read is spelled by the cases the tests leave: the tag by its method,
//!   a field of the one case left through that case's proven read, a field two
//!   cases share through its accessor, a list a case holds walked through the
//!   proven read, and each item of a list of unions narrowed alike.
//!
//! What the generated machine does at run time is held by
//! `backends/rust/tests/tests/a_union_payload_is_read_by_its_tag.rs` over the
//! committed machine.

use std::path::{Path, PathBuf};

use sce_build::{compile_scxml_lang_typed, find_template_dir_for};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/static_datamodel/static_union.scxml")
}

/// The Rust machine of the fixture.
fn machine() -> String {
    let language = sce_build::generator::Language::Rust;
    let output = compile_scxml_lang_typed(
        fixture().to_str().expect("a utf-8 path"),
        &find_template_dir_for(language),
        language,
    )
    .unwrap_or_else(|e| panic!("Rust reads a payload that holds a union: {e:?}"));
    output
        .files
        .into_iter()
        .find(|(name, _)| name == "static_union_sm.rs")
        .map(|(_, code)| code)
        .expect("the machine's Rust is generated")
}

/// `code` holds each of `lines`.
fn holds(code: &str, lines: &[&str]) {
    for line in lines {
        assert!(code.contains(line), "expected `{line}` in:\n{code}");
    }
}

const UNION: &str = "StaticUnionUnionShapeUnion";

/// Pinned by `RustComposed::union_type` and `RustComposed::union_def`
/// (`sce-build/src/forge/composed_lowering.rs`) and by the declaration of each
/// union a payload names (`declare_composed_payload`,
/// `sce-build/src/forge/static_lowering.rs`).
#[test]
fn the_payload_holds_the_union_in_the_machines_own_sum() {
    let code = machine();
    holds(
        &code,
        &[
            "pub struct StaticUnionShapesSentPayload {",
            &format!("    pub one: {UNION},"),
            &format!("    pub many: ::sce_rust_runtime::portable::BoundedList<{UNION}, 3>,"),
            &format!("    pub maybe: Option<{UNION}>,"),
            &format!(
                "pub enum {UNION} {{\n    Dot,\n    Circle(StaticUnionRadiusRecord),\n    \
                 Disc(StaticUnionRadiusRecord),\n    Rect(StaticUnionRectRecord),\n    \
                 Label(StaticUnionLabelRecord),\n    Path(StaticUnionPathRecord),\n}}"
            ),
            "    pub fn kind(&self) -> StaticUnionKindEnum {",
            "    pub fn proven_rect(&self) -> &StaticUnionRectRecord {",
            "    pub fn proven_field_radius(&self) -> u16 {",
        ],
    );
    assert_eq!(
        code.matches(&format!("pub enum {UNION} {{")).count(),
        1,
        "a union is declared once"
    );
}

/// Pinned by `RustComposed::read_union` and `RustComposed::write_union`.
#[test]
fn the_lift_reads_the_tag_first_and_the_seam_writes_each_case() {
    let code = machine();
    let p = "::sce_rust_runtime::event_payload";
    holds(
        &code,
        &[
            r#"one: fields.field("one").and_then(|v| v.union("kind", |v| v.name()"#,
            r#"many: fields.field("many").and_then(|v| v.list_within::<_, 3>(1, |v| v.union("kind","#,
            r#"maybe: fields.field("maybe").and_then(|v| v.optional(|v| v.union("kind","#,
            &format!("StaticUnionKindEnum::Dot => Ok({UNION}::Dot), "),
            &format!(
                "(\"one\", match &payload.one {{ {UNION}::Dot => {p}::union_json(\"kind\", \"dot\", \
                 Vec::new()), "
            ),
        ],
    );
}

/// Pinned by `lower_union_reads` (`sce-build/src/forge/expr.rs`), by
/// `rust_union_member` (`sce-build/src/forge/composed_lowering.rs`) and by the
/// union-aware `place_text` of a loop's list
/// (`sce-build/src/forge/static_lowering.rs`).
#[test]
fn each_read_is_spelled_by_the_cases_its_tests_leave() {
    let code = machine();
    holds(
        &code,
        &[
            "ev.one.kind()",
            "ev.one.proven_field_radius()",
            "ev.one.proven_rect().width",
            "ev.one.proven_rect().height",
            "ev.one.proven_path().points.clone()",
            "for shape in ev.many.clone()",
            "shape.kind()",
            "shape.proven_rect().width",
            "shape.proven_field_radius()",
            // The present value of an optional union, read in place through
            // the runtime's `proven_ref`, then by the case its test proves.
            "sce_forge_runtime::optional::proven_ref(&ev.maybe).kind()",
            "sce_forge_runtime::optional::proven_ref(&ev.maybe).proven_rect().width",
        ],
    );
    // No read of a case's field is left as a field of the sum.
    assert!(!code.contains("ev.one.radius"), "{code}");
    assert!(!code.contains("shape.width"), "{code}");
}
