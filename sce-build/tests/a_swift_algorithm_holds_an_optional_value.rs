// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A value that may be absent (`optional<T>`, docs/adr/0055), written for Swift: the
// slot, the return and the field of a record are `T?` over the type of the present
// value, absent is `nil`, a test of presence is `== nil` and `!= nil`, a read with a
// default is `??`, and a read of a value proven present is `SceOptional.proven`, which
// fails loudly where the proof was wrong.

use std::path::{Path, PathBuf};

use sce_build::generator::Language;
use sce_build::{compile_forge_file, ForgeCompileOptions};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

const FIXTURES: &str = "sce-build/tests/fixtures/static_datamodel";

/// The Swift source `document` (a path under the repository) is written as, or the
/// refusal it is answered with.
fn swift_of(document: &str) -> Result<String, String> {
    let output = compile_forge_file(
        &repo_root().join(document),
        Language::Swift,
        &[],
        &ForgeCompileOptions::default(),
    )
    .map_err(|e| format!("{e:?}"))?;
    let (_, code) = output
        .files
        .into_iter()
        .next()
        .expect("a document that is written writes a file");
    Ok(code)
}

#[test]
fn a_parameter_and_a_return_that_may_be_absent_are_the_optional_of_the_type() {
    let code = swift_of(&format!("{FIXTURES}/algorithm_goal_until.scxml"))
        .expect("an optional slot is written");
    assert!(
        code.contains("_ day: Int64?) throws -> Int64?"),
        "the parameter and the return are `T?`: {code}"
    );
}

#[test]
fn a_test_of_presence_a_proven_read_and_absence_are_spelled_over_nil() {
    let code = swift_of(&format!("{FIXTURES}/algorithm_goal_until.scxml"))
        .expect("an optional slot is written");
    assert!(code.contains("let sceCond2 = day == nil"), "{code}");
    assert!(code.contains("let sceReturned: Int64? = nil"), "{code}");
    assert!(
        code.contains("SceOptional.proven(day)"),
        "a read proven present is by the runtime's function: {code}"
    );
}

#[test]
fn a_read_with_a_default_is_the_nil_coalescing_operator() {
    let code = swift_of(&format!("{FIXTURES}/algorithm_goal_day_sum.scxml"))
        .expect("a read with a default is written");
    assert!(code.contains("g.day ?? instead"), "{code}");
}

#[test]
fn a_local_that_may_be_absent_starts_at_nil_and_takes_a_whole_record() {
    let code = swift_of(&format!("{FIXTURES}/algorithm_item_pick_first.scxml"))
        .expect("an optional record is written");
    assert!(
        code.contains("var found: SchemaItemPickPayload? = nil"),
        "{code}"
    );
    // A struct is a value that a `T?` takes a copy of, and a Swift statement ends
    // with the line.
    assert!(code.contains("found = it\n"), "{code}");
    assert!(!code.contains("found = it;"), "{code}");
}

#[test]
fn a_field_that_may_be_absent_is_at_nil_until_written() {
    let code = swift_of(&format!("{FIXTURES}/schema_goal.scxml"))
        .expect("a schema with a field that may be absent is written");
    assert!(code.contains("public var day: Int64? = nil"), "{code}");
    assert!(
        code.contains("public init(id: UInt32, day: Int64?, weight: Int32)"),
        "{code}"
    );
}

#[test]
fn an_optional_byte_string_is_refused_by_name() {
    // A byte string is held in no Swift slot yet, so a value that may be one is none.
    let document = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
          version="1.0" sce:kind="event-schema" name="schema_blob" sce:event-name="blob.put">
  <datamodel>
    <data id="blob" sce:type="optional&lt;bytes&gt;" sce:max-size="8" sce:direction="in"/>
  </datamodel>
</scxml>"#;
    let dir = tempfile::tempdir().expect("a scratch directory");
    let path = dir.path().join("schema_blob.scxml");
    std::fs::write(&path, document).expect("the document is written");
    let refusal = compile_forge_file(&path, Language::Swift, &[], &ForgeCompileOptions::default())
        .map(|_| ())
        .expect_err("an optional byte string is not lowered for Swift");
    let text = format!("{refusal:?}");
    assert!(
        text.contains("Swift") && text.contains("optional field `blob`"),
        "{text}"
    );
}
