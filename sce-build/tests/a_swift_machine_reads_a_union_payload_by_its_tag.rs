// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A Swift `sce-static` machine reads a payload that holds a union
//! (docs/adr/0092, sections 4, 6.1 and 7), held to
//! `sce-build/tests/fixtures/static_datamodel/static_union.scxml`:
//!
//! * The payload struct holds the union as the machine's own enum, in a list and
//!   optionally. No member holds a value nobody wrote (docs/adr/0098, section 4),
//!   so the payload is built whole by the initializer of every member and the
//!   policy's view of it holds nothing until a delivery sets it.
//! * Each read is spelled by the cases the tests leave: the tag by its property,
//!   a field of the one case left through that case's proven read, and a field
//!   two cases share through its accessor.
//! * The machine's saved form writes a union as its tag beside its case.
//! * An index read of a list of unions is read as on every lane: the runtime's
//!   checked read throws an index outside the list and answers no union in its
//!   place (docs/adr/0098, section 5).
//!
//! What the generated machine does at run time is held by the scenario
//! `static_union.json`, replayed over the committed machine by
//! `backends/swift/tests/Tests/StaticDatamodelTests/ScalarScenarioTests.swift`.

use std::path::{Path, PathBuf};

use sce_build::generator::Language;
use sce_build::{compile_scxml_lang_typed, find_template_dir_for};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/static_datamodel")
}

/// The Swift machine of the fixture.
fn machine() -> String {
    let language = Language::Swift;
    let output = compile_scxml_lang_typed(
        fixtures()
            .join("static_union.scxml")
            .to_str()
            .expect("a utf-8 path"),
        &find_template_dir_for(language),
        language,
    )
    .unwrap_or_else(|e| panic!("Swift reads a payload that holds a union: {e:?}"));
    output
        .files
        .into_iter()
        .find(|(name, _)| name == "static_union_sm.swift")
        .map(|(_, code)| code)
        .expect("the machine's Swift is generated")
}

/// `code` holds each of `lines`.
fn holds(code: &str, lines: &[&str]) {
    for line in lines {
        assert!(code.contains(line), "expected `{line}` in:\n{code}");
    }
}

const UNION: &str = "StaticUnionUnionShapeUnion";

/// Pinned by `SwiftComposed::union_def`
/// (`sce-build/src/forge/composed_lowering/swift.rs`) and by the payload channel
/// (`swift_composed_payload`, `sce-build/src/forge/generator.rs`).
#[test]
fn the_payload_holds_the_union_in_the_machines_own_enum_built_whole() {
    let code = machine();
    holds(
        &code,
        &[
            &format!("public enum {UNION}: Equatable, Sendable {{"),
            "struct StaticUnionShapesSentPayload {",
            &format!("    var one: {UNION}\n"),
            &format!("    var many: [{UNION}]\n"),
            &format!("    var maybe: {UNION}?\n"),
            // The policy's view holds nothing until a delivery sets it.
            "    var pendingShapesSentPayload: StaticUnionShapesSentPayload!",
            "let payload = StaticUnionShapesSentPayload(one: sceField0, many: sceField1, maybe: sceField2)",
        ],
    );
    assert!(
        !code.contains(&format!("{UNION}.allCases")),
        "a union has no first case to start at:\n{code}"
    );
}

/// Pinned by `SwiftComposed::union_member` and the read the expression lowering
/// spells by the cases the facts leave (`lower_union_reads`,
/// `sce-build/src/forge/expr.rs`).
#[test]
fn each_read_is_spelled_by_the_cases_the_tests_leave() {
    let code = machine();
    holds(
        &code,
        &[
            // The tag by its property, qualified by the enum once.
            "    public var kind: StaticUnionKindEnum {",
            "            return StaticUnionKindEnum.circle\n",
            "self.pendingShapesSentPayload.one.kind == StaticUnionKindEnum.dot",
            // A field of the one case left, through that case's proven read.
            "self.vWidth = self.pendingShapesSentPayload.one.provenRect.width",
            // A field two cases share, through its accessor.
            "    public var provenFieldRadius: UInt16 {",
            // And the place a write goes through.
            "self.vBuilt.provenRectPlace.width = sceValue",
        ],
    );
    assert!(
        !code.contains("StaticUnionKindEnum.StaticUnionKindEnum"),
        "the enum's variant is already qualified by its type:\n{code}"
    );
}

/// Pinned by `SwiftComposed::saved_union_ext` and `SwiftComposed::saved_record_ext`.
#[test]
fn a_saved_union_is_its_tag_beside_its_case_and_is_read_back_by_its_tag() {
    let code = machine();
    holds(
        &code,
        &[
            "extension StaticUnionUnionShapeUnion {",
            "case .circle(let sceRecord): return SavedValues.union(tag: \"kind\", variant: \"circle\", sceRecord.toSaved())",
            "let sceTag = try SavedValues.unionTag(value, what, tag: \"kind\")",
            "case \"circle\": return .circle(try StaticUnionRadiusRecord.fromSaved(value, what))",
            // A case record that holds a list writes and reads it by its type, held to its bound.
            "extension StaticUnionPathRecord {",
            "SavedValues.list(self.points) { sceSaved0 in SavedValues.of(sceSaved0) }",
            "capacity: 3) { sceSaved0, sceWhat0 in try SavedValues.uint8(sceSaved0, sceWhat0) }",
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

/// The indexed reader, written beside the documents it imports, as the text of
/// every file it generates.
fn compile_indexed(language: Language) -> Result<String, String> {
    let dir = tempfile::tempdir().expect("tempdir");
    for entry in std::fs::read_dir(fixtures()).expect("the fixtures are listed") {
        let entry = entry.expect("an entry");
        if entry.path().extension().is_some_and(|ext| ext == "scxml") {
            std::fs::copy(entry.path(), dir.path().join(entry.file_name())).expect("a sibling");
        }
    }
    let path = dir.path().join("static_union.scxml");
    std::fs::write(&path, indexed_reader()).expect("the reader");
    compile_scxml_lang_typed(
        path.to_str().expect("a utf-8 path"),
        &find_template_dir_for(language),
        language,
    )
    .map(|output| {
        output
            .files
            .into_iter()
            .map(|(_, code)| code)
            .collect::<Vec<_>>()
            .join("\n")
    })
    .map_err(|e| format!("{e:?}"))
}

/// Pinned by the checked index read of the Swift expression lowering
/// (`sce-build/src/forge/expr/swift.rs`): an element at an index is read by the
/// runtime's `Checked.at`, which throws an index outside the list and answers no
/// value in its place, so a list of unions is read as a list of numbers is
/// (docs/adr/0098, section 5), on Swift as on Rust.
#[test]
fn an_index_read_of_a_list_of_unions_is_read_on_swift_as_on_rust() {
    let code =
        compile_indexed(Language::Swift).expect("Swift reads the tag of an element at an index");
    holds(
        &code,
        &["(try Checked.at(self.pendingShapesSentPayload.many, 0)).kind"],
    );
    compile_indexed(Language::Rust).expect("Rust reads the tag of an element at an index");
}
