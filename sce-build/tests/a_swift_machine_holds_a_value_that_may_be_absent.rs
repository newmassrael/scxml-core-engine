// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The Swift backend holds a value that may be absent, `optional<T>`
//! (docs/adr/0055), in a `sce-static` statechart: a field of a record the machine
//! holds and a field of an event's payload are `T?` over the type of the present
//! value and start at `nil`, a payload field is read as absent from `null` and
//! refused where the key is lacking, a payload is written back with the key always
//! present, and a `<param>` that carries one is `null` where it is absent.
//!
//! `scenarios/static_optional.json`, `static_optional_widened.json`,
//! `static_optional_record.json` and `static_state_algorithm_optional.json` replay
//! the machines the build generates (`StaticDatamodelTests`); this file holds the
//! text the generator writes, which those read.
//!
//! # What was wrong
//!
//! Measured in the code on 2026-10-29: a record and a payload struct held the
//! present type of a field that may be absent, `Int64 = 0` for an `optional<int64>`,
//! so an absent day read as a day of zero and a delivery's `null` was lost. Nothing
//! refused it: the machine compiled.

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::tempdir;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

fn fixtures() -> PathBuf {
    repo_root().join("sce-build/tests/fixtures/static_datamodel")
}

/// The Swift `sce-codegen generate -l swift` writes for `machine`, a document of the
/// fixture directory, whose imports are copied beside it: the text of every file
/// written.
fn swift_of(machine: &str) -> String {
    let dir = tempdir().expect("tempdir");
    for entry in std::fs::read_dir(fixtures()).expect("the fixture directory") {
        let path = entry.expect("an entry").path();
        if path.is_file() {
            let name = path.file_name().expect("a file name").to_owned();
            std::fs::copy(&path, dir.path().join(name)).expect("copy a fixture");
        }
    }
    let out_dir = tempdir().expect("tempdir");
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(["generate", "-l", "swift"])
        .arg(dir.path().join(machine))
        .arg("-o")
        .arg(out_dir.path())
        .output()
        .expect("invoke sce-codegen");
    assert!(
        out.status.success(),
        "{machine} is written for Swift: {}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    let mut written = String::new();
    for entry in std::fs::read_dir(out_dir.path()).expect("the output directory") {
        let path = entry.expect("an entry").path();
        if path.is_file() {
            written.push_str(&std::fs::read_to_string(&path).unwrap_or_default());
            written.push('\n');
        }
    }
    written
}

/// `written` holds each of `needles`, or the test names the one it lacks and shows
/// what was written.
fn holds_each(written: &str, needles: &[&str]) {
    for needle in needles {
        assert!(
            written.contains(needle),
            "the generated Swift holds `{needle}`:\n{written}"
        );
    }
}

/// A payload's field that may be absent is `T?` and starts at `nil`, where it held
/// the present type at its zero: the delivery's `null` was a zero.
#[test]
fn a_payload_field_that_may_be_absent_is_the_optional_type_and_starts_at_nil() {
    let code = swift_of("static_optional.scxml");
    holds_each(
        &code,
        &[
            "    var due: Int64? = nil\n    var title: String? = nil\n    var status: StaticOptionalStatusEnum? = nil\n",
            "public var due: Int64? { vDue }",
        ],
    );
    assert!(
        !code.contains("var due: Int64 = 0\n    var title: String = \"\""),
        "a field that may be absent is not held at the zero of its present type:\n{code}"
    );
}

/// A payload field is read absent from `null`, as the present value otherwise, and a
/// delivery that lacks the key is refused (`PayloadFields.optional`); written back,
/// the key is always there.
#[test]
fn a_payload_field_is_read_absent_from_null_and_written_with_its_key() {
    let code = swift_of("static_optional.scxml");
    holds_each(
        &code,
        &[
            "try fields.optional(\"due\") { fields in try fields.signed64(\"due\") }",
            "try fields.optional(\"title\") { fields in try fields.text(\"title\") }",
            ".raw(\"due\", json: optionalJSON(due) { WireValue.signed64($0).json })",
            ".raw(\"status\", json: optionalJSON(status) { WireValue.text($0.declaredName).json })",
        ],
    );
}

/// A `<param>` that carries a value that may be absent is `WireValue.null` where it
/// is absent and the present value's wire value where it is present.
#[test]
fn a_param_that_carries_a_value_that_may_be_absent_is_null_where_it_is_absent() {
    let code = swift_of("static_optional.scxml");
    assert!(
        code.contains("?? WireValue.null"),
        "an absent value is the runtime's `null`:\n{code}"
    );
}

/// A record the schema declares with a field that may be absent holds it as `T?`:
/// the machine's own struct, and the initializer that builds it whole.
#[test]
fn a_record_field_that_may_be_absent_is_the_optional_type() {
    let code = swift_of("static_state_algorithm_optional.scxml");
    holds_each(
        &code,
        &[
            "    public var day: Int64? = nil\n",
            "public init(id: UInt32, day: Int64?, weight: Int32)",
        ],
    );
}

/// A field a later revision widened to one that may be absent is `T?` as the
/// payload carries it, and the key a first-revision entry lacks reads as its default.
#[test]
fn a_widened_payload_field_is_the_optional_type() {
    let code = swift_of("static_optional_widened.scxml");
    holds_each(
        &code,
        &[
            "    var due: Int64? = nil\n    var remind: Int64? = nil\n",
            "var vDue: Int64? = nil",
            "var vRemind: Int64? = nil",
        ],
    );
}
