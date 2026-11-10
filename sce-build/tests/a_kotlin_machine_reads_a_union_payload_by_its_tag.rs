// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A Kotlin `sce-static` machine holds a union (docs/adr/0092, sections 4, 6.1
//! and 7, the Kotlin row), held to
//! `sce-build/tests/fixtures/static_datamodel/static_union.scxml`:
//!
//! * The payload class holds the union alone, in a list and where it may be
//!   absent, as the machine's own sealed interface, one nested class per case
//!   holding the machine's record of the case.
//! * The lift reads each union's tag first (`EventPayload.Value.union`), then
//!   the record of the case the tag names; a `<sce:lift>` reads a text into a
//!   union the same way (`EventPayload.liftValue`).
//! * Each read is spelled by the cases the tests leave: the tag by its
//!   property, a field of the one case left through that case's proven read,
//!   and a field two cases share through its accessor.
//! * A union is an immutable value, so a field of a case is written by
//!   assigning the variable the next value whole.
//! * The machine's saved form writes a union as its tag beside its case and
//!   reads it back by its tag, through extensions of the interface.
//! * An index read of a list of unions is read, since a Kotlin machine refuses
//!   an index outside a list rather than answer a zero a union does not have.
//!
//! What the generated machine does at run time is held by the scenario
//! `static_union.json`, replayed over the committed machine by
//! `backends/kotlin/tests/src/test/kotlin/com/sce/integration/StaticScenarioTest.kt`.

use std::path::{Path, PathBuf};

use sce_build::generator::Language;
use sce_build::{compile_scxml_lang_typed, find_template_dir_for};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/static_datamodel")
}

/// The Kotlin machine of the statechart at `path`.
fn kotlin_machine(path: &Path) -> Result<String, String> {
    let language = Language::Kotlin;
    let output = compile_scxml_lang_typed(
        path.to_str().expect("a utf-8 path"),
        &find_template_dir_for(language),
        language,
    )
    .map_err(|e| format!("{e:?}"))?;
    let names: Vec<String> = output.files.iter().map(|(name, _)| name.clone()).collect();
    output
        .files
        .into_iter()
        .find(|(name, _)| name.ends_with("Sm.kt"))
        .map(|(_, code)| code)
        .ok_or_else(|| format!("no machine among {names:?}"))
}

/// The Kotlin machine of the fixture.
fn machine() -> String {
    kotlin_machine(&fixtures().join("static_union.scxml"))
        .unwrap_or_else(|e| panic!("Kotlin holds a union: {e}"))
}

/// `code` holds each of `lines`.
fn holds(code: &str, lines: &[&str]) {
    for line in lines {
        assert!(code.contains(line), "expected `{line}` in:\n{code}");
    }
}

const UNION: &str = "StaticUnionUnionShapeUnion";

/// Pinned by `KotlinComposed::union_type` and `KotlinComposed::union_def`
/// (`sce-build/src/forge/composed_lowering/kotlin.rs`) and by the payload
/// channel (`kotlin_composed_payload`, `sce-build/src/forge/generator.rs`).
#[test]
fn the_payload_holds_the_union_in_the_machines_own_sealed_interface() {
    let code = machine();
    holds(
        &code,
        &[
            &format!("sealed interface {UNION} {{"),
            "    val kind: StaticUnionKindEnum\n",
            &format!("    data object DOT : {UNION} {{"),
            &format!("    data class CIRCLE(val value: StaticUnionRadiusRecord) : {UNION} {{"),
            &format!("    data class PATH(val value: StaticUnionPathRecord) : {UNION} {{"),
            "            get() = StaticUnionKindEnum.CIRCLE\n",
            &format!(
                "data class StaticUnionShapesSentPayload(val one: {UNION}, val many: \
                 List<{UNION}>, val maybe: {UNION}?)"
            ),
            // The lift reads the tag first, then the case it names.
            "fields.field(\"one\").let { when (it.union(\"kind\", { when (val sceName = \
             it.name()) {",
        ],
    );
}

/// Pinned by `KotlinComposed::union_member` and the read the expression
/// lowering spells by the cases the facts leave (`lower_union_reads`,
/// `sce-build/src/forge/expr.rs`), and by `KotlinComposed::union_member_write`
/// and the `<assign>` that writes through it (`union_field_place`,
/// `sce-build/src/forge/static_lowering.rs`).
#[test]
fn each_read_and_write_is_spelled_by_the_cases_the_tests_leave() {
    let code = machine();
    holds(
        &code,
        &[
            // The tag by its property.
            "pendingShapesSentPayload!!.one.kind == StaticUnionKindEnum.DOT",
            // A field of the one case left, through that case's proven read.
            "pendingShapesSentPayload!!.one.provenRect().width",
            // A field two cases share, through its accessor.
            "pendingShapesSentPayload!!.one.provenFieldRadius()",
            "    fun provenFieldRadius(): UShort = when (this) {",
            // A field of the built variable's proven case, written by assigning the
            // variable the next value whole.
            &format!("vBuilt = {UNION}.RECT(vBuilt.provenRect().copy(width = "),
            // A union put into an optional one whole.
            "vKept = pendingShapesSentPayload!!.one",
        ],
    );
}

/// Pinned by `union_variable_field` (`sce-build/src/forge/static_lowering.rs`)
/// and `KotlinComposed::union_value`: a variable of the union starts at the case
/// its `<sce:set>`s build, and one that may be absent starts absent.
#[test]
fn a_union_variable_starts_built_or_absent() {
    let code = machine();
    holds(
        &code,
        &[
            &format!("private var vBuilt: {UNION} = {UNION}.RECT(StaticUnionRectRecord(width = "),
            &format!("private var vKept: {UNION}? = null"),
            &format!("private var vLifted: {UNION}? = null"),
        ],
    );
}

/// Pinned by `KotlinComposed::saved_union_ext`, `KotlinComposed::saved_record_ext`
/// and the saved form the Kotlin template writes a variable by
/// (`saved_write`, `saved_read`, `tools/codegen/templates/kotlin/state_machine.kt.jinja2`).
#[test]
fn a_saved_union_is_its_tag_beside_its_case_and_is_read_back_by_its_tag() {
    let code = machine();
    holds(
        &code,
        &[
            &format!("fun {UNION}.toSaved(): Any = when (this) {{"),
            &format!(
                "    is {UNION}.CIRCLE -> SavedValues.union(\"kind\", \"circle\", \
                 this.value.toSaved())"
            ),
            &format!("    {UNION}.DOT -> SavedValues.union(\"kind\", \"dot\", null)"),
            &format!("fun {UNION}.Companion.fromSaved(value: Any?, what: String): {UNION} ="),
            "when (val sceTag = SavedValues.unionTag(value, what, \"kind\")) {",
            &format!(
                "        \"circle\" -> {UNION}.CIRCLE(StaticUnionRadiusRecord.fromSaved(value, \
                 what))"
            ),
            // A case record that holds a list writes and reads it by its type, held to its
            // bound.
            "fun StaticUnionPathRecord.toSaved(): Any = linkedMapOf<String, Any?>(\"points\" to \
             SavedValues.list(this.points) { sceSaved0 -> SavedValues.of(sceSaved0) })",
            "SavedValues.list(SavedValues.field(value, what, \"points\"), \"$what.points\", 3) { \
             sceSaved0, sceWhat0 -> SavedValues.uint8(sceSaved0, sceWhat0) }",
            // Each variable writes and reads its union by the union's own form.
            "variables += \"built\" to vBuilt.toSaved()",
            "variables += \"kept\" to SavedValues.optional(vKept) { it.toSaved() }",
            &format!("{UNION}.fromSaved(savedGrown.variable(\"built\"), \"built\")"),
            &format!(
                "SavedValues.optional(savedGrown.variable(\"kept\"), \"kept\") {{ e, w -> \
                 {UNION}.fromSaved(e, w) }}"
            ),
        ],
    );
}

/// Pinned by the Kotlin lift (`KotlinTarget::lift`,
/// `sce-build/src/forge/static_lowering.rs`): a text is read into the union
/// whole by the reader a delivery's field of it is read by.
#[test]
fn a_text_is_lifted_into_the_union_by_its_reader() {
    let code = machine();
    holds(
        &code,
        &[
            "com.sce.runtime.EventPayload.liftValue(",
            "{ when (it.union(\"kind\", { when (val sceName = it.name()) {",
            "vLifted = sceLift.value",
        ],
    );
}

/// The fixture, with one transition that reads the tag of an element of `many`
/// at an index.
fn indexed_reader() -> String {
    let fixture = std::fs::read_to_string(fixtures().join("static_union.scxml"))
        .expect("the fixture is readable");
    let anchor = "<assign location=\"last_kind\" expr=\"_event.data.one.kind\"/>";
    assert!(fixture.contains(anchor), "the anchor moved in the fixture");
    fixture.replacen(
        anchor,
        &format!(
            "{anchor}\n      <if cond=\"_event.data.many[0].kind === Kind.dot\">\
             <assign location=\"dots\" expr=\"dots + 1\"/></if>"
        ),
        1,
    )
}

/// Pinned by the Swift-only refusal of an index read of a list of unions
/// (`reject_zeroless_element_reads`, `sce-build/src/forge/expr.rs`): a Kotlin
/// machine reads an element at an index through the runtime's checked read,
/// which fails the statement for an index outside the list and answers no
/// zero, so it reads the tag of an element of a list of unions.
#[test]
fn an_index_read_of_a_list_of_unions_is_read_on_kotlin() {
    let dir = tempfile::tempdir().expect("tempdir");
    for entry in std::fs::read_dir(fixtures()).expect("the fixtures are listed") {
        let entry = entry.expect("an entry");
        if entry.path().extension().is_some_and(|ext| ext == "scxml") {
            std::fs::copy(entry.path(), dir.path().join(entry.file_name())).expect("a sibling");
        }
    }
    let path = dir.path().join("static_union.scxml");
    std::fs::write(&path, indexed_reader()).expect("the reader");
    let code = kotlin_machine(&path)
        .unwrap_or_else(|e| panic!("Kotlin reads the tag of an element at an index: {e}"));
    holds(
        &code,
        &["com.sce.forge.runtime.SceChecked.at(pendingShapesSentPayload!!.many, (0).toLong()).kind"],
    );
}
