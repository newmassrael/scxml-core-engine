// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The unit an event-schema document generates names the event it types and the
//! revision it is as typed constants, on every backend, so a host that stamps what it
//! sends (a command log, a wire record) reads them from the generated code rather than
//! copying them from the document.
//!
//! # What was wrong
//!
//! Measured 2026-10-21 by a project outside this repository: the generated unit
//! carried the event name in a comment and the revision nowhere, so a host copied
//! `"record_list.edit"` and `2` by hand, and a revision the schema gained would have
//! left the copy behind without a word.

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

/// The standard schema the cases read: `record_list.edit` at revision 3, whose `text`
/// is bounded by `sce:max-size="32"` and named `shared_typed_text`.
const SCHEMA: &str = "stdlib/shared/record_edit.scxml";

/// What `generate -l <language>` writes for `document` under the limits `given`,
/// every file of it in one text.
fn generated(document: &str, language: &str, given: &[&str]) -> String {
    let out_dir = tempdir().expect("tempdir");
    let mut command = Command::new(sce_codegen_bin());
    command.arg("--workspace-root").arg(repo_root());
    for limit in given {
        command.args(["--limit", limit]);
    }
    let out = command
        .arg("generate")
        .arg(repo_root().join(document))
        .args([
            "-l",
            language,
            "--go-module-prefix",
            "example.test/generated",
        ])
        .arg("-o")
        .arg(out_dir.path())
        .output()
        .expect("invoke sce-codegen");
    assert!(
        out.status.success(),
        "generate {document} -l {language}:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let mut text = String::new();
    let mut pending = vec![out_dir.path().to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).expect("the output directory") {
            let path = entry.expect("an entry").path();
            if path.is_dir() {
                pending.push(path);
            } else {
                text.push_str(&std::fs::read_to_string(&path).unwrap_or_default());
            }
        }
    }
    text
}

/// Each backend's spelling of the two constants, as the unit declares them: typed,
/// and named by the unit (a module, a package, a namespace) or, in C, by the schema.
#[test]
fn every_backend_names_the_event_and_the_revision_of_a_schema() {
    for (language, fragments) in [
        (
            "rust",
            &[
                "pub const EVENT_NAME: &str = \"record_list.edit\";",
                "pub const REVISION: u32 = 3;",
            ][..],
        ),
        (
            "kotlin",
            &[
                "const val EVENT_NAME: String = \"record_list.edit\"",
                "const val REVISION: UInt = 3u",
            ][..],
        ),
        (
            "cpp",
            &[
                "inline constexpr const char *EVENT_NAME = \"record_list.edit\";",
                "inline constexpr std::uint32_t REVISION = 3;",
            ][..],
        ),
        (
            "go",
            &[
                "const EventName string = \"record_list.edit\"",
                "const Revision uint32 = 3",
            ][..],
        ),
        (
            "python",
            &[
                "EVENT_NAME: Final[str] = \"record_list.edit\"",
                "REVISION: Final[int] = 3",
            ][..],
        ),
        (
            "c",
            &[
                "#define RECORD_EDIT_EVENT_NAME \"record_list.edit\"",
                "#define RECORD_EDIT_REVISION UINT32_C(3)",
            ][..],
        ),
    ] {
        let out = generated(SCHEMA, language, &[]);
        for fragment in fragments {
            assert!(
                out.contains(fragment),
                "{language}: the schema's unit declares `{fragment}`:\n{out}"
            );
        }
    }
}

/// The bound of each text the schema declares one for is a typed constant of the
/// unit, as the build resolved it: `record_edit`'s `text` is 32 bytes, named
/// `shared_typed_text`, so `--limit shared_typed_text=48` moves the constant with the
/// buffer (docs/adr/0050). Measured 2026-10-21, before it: no generated code named
/// the bound, and a host learned it from a refusal.
#[test]
fn every_backend_names_the_bound_of_each_text_a_schema_bounds() {
    for (language, written, moved) in [
        (
            "rust",
            "pub const TEXT_MAX_SIZE: u32 = 32;",
            "pub const TEXT_MAX_SIZE: u32 = 48;",
        ),
        (
            "kotlin",
            "const val TEXT_MAX_SIZE: UInt = 32u",
            "const val TEXT_MAX_SIZE: UInt = 48u",
        ),
        (
            "cpp",
            "inline constexpr std::uint32_t TEXT_MAX_SIZE = 32;",
            "inline constexpr std::uint32_t TEXT_MAX_SIZE = 48;",
        ),
        (
            "go",
            "const TextMaxSize uint32 = 32",
            "const TextMaxSize uint32 = 48",
        ),
        (
            "python",
            "TEXT_MAX_SIZE: Final[int] = 32",
            "TEXT_MAX_SIZE: Final[int] = 48",
        ),
        (
            "c",
            "#define RECORD_EDIT_TEXT_MAX_SIZE UINT32_C(32)",
            "#define RECORD_EDIT_TEXT_MAX_SIZE UINT32_C(48)",
        ),
    ] {
        let as_written = generated(SCHEMA, language, &[]);
        assert!(
            as_written.contains(written) && as_written.contains("shared_typed_text"),
            "{language}: the unit declares `{written}` and names the bound:\n{as_written}"
        );
        let built = generated(SCHEMA, language, &["shared_typed_text=48"]);
        assert!(
            built.contains(moved),
            "{language}: the build's number moves the constant to `{moved}`:\n{built}"
        );
    }
    // A field with no bound has no constant.
    let rust = generated(SCHEMA, "rust", &[]);
    assert!(!rust.contains("ID_MAX_SIZE"), "{rust}");
}

/// The largest `_event.data` of `record_edit` with its text bounded at `text_bound`,
/// written out: every number at its longest spelling (a 64-bit integer as the quoted
/// text of its digits), the text at its bound of control characters at their
/// longest escape, the keys in their wire order. A field a later revision added
/// (`command`, revision 3) is one a delivery may carry, so it is counted.
fn largest_record_edit(text_bound: usize) -> String {
    let wide = "\"-9223372036854775808\"";
    let wide_unsigned = "\"18446744073709551615\"";
    format!(
        "{{\"command\":{wide_unsigned},\"field\":65535,\"id\":4294967295,\"more\":false,\
         \"now\":{wide},\"text\":\"{}\",\"value\":{wide}}}",
        "\\u0001".repeat(text_bound)
    )
}

/// Every unit declares the most bytes its event's data takes (docs/adr/0086, section
/// 2.2), as the build resolved each bound in it: `record_edit`'s largest data is 339
/// bytes with its text at 32, and 435 with `--limit shared_typed_text=48`.
#[test]
fn every_backend_names_the_longest_data_of_a_schema() {
    let written = largest_record_edit(32).len();
    let moved = largest_record_edit(48).len();
    assert_eq!((written, moved), (339, 435));
    for (language, spelling) in [
        ("rust", "pub const DATA_MAX_LEN: u64 = {};"),
        ("kotlin", "const val DATA_MAX_LEN: ULong = {}uL"),
        ("cpp", "inline constexpr std::uint64_t DATA_MAX_LEN = {}u;"),
        ("go", "const DataMaxLen uint64 = {}"),
        ("python", "DATA_MAX_LEN: Final[int] = {}"),
        ("c", "#define RECORD_EDIT_DATA_MAX_LEN UINT64_C({})"),
        ("swift", "public static let dataMaxLen: UInt64 = {}"),
    ] {
        for (given, size) in [(&[][..], written), (&["shared_typed_text=48"][..], moved)] {
            let out = generated(SCHEMA, language, given);
            let fragment = spelling.replace("{}", &size.to_string());
            assert!(
                out.contains(&fragment),
                "{language} under {given:?}: the unit declares `{fragment}`:\n{out}"
            );
        }
    }
}

/// A schema with no field carries `{}`, two bytes, and one of numbers only the
/// longest spelling of each: `record_clear` holds no text, so no limit moves it.
#[test]
fn the_longest_data_of_a_schema_is_its_fields_at_their_longest() {
    let fieldless = generated(
        "sce-build/tests/fixtures/event_schema_payload_free/job_requested.scxml",
        "rust",
        &[],
    );
    assert!(
        fieldless.contains("pub const DATA_MAX_LEN: u64 = 2;"),
        "{fieldless}"
    );
    let largest = "{\"command\":\"18446744073709551615\",\"field\":65535,\"id\":4294967295,\
                   \"more\":false,\"now\":\"-9223372036854775808\"}";
    assert_eq!(largest.len(), 106);
    let out = generated("stdlib/shared/record_clear.scxml", "rust", &[]);
    assert!(
        out.contains(&format!("pub const DATA_MAX_LEN: u64 = {};", largest.len())),
        "{out}"
    );
}

/// A schema that declares no revision is the first, and says so.
#[test]
fn a_schema_with_no_revision_declared_is_the_first() {
    let out = generated("stdlib/shared/record_room.scxml", "rust", &[]);
    assert!(
        out.contains("pub const REVISION: u32 = 1;"),
        "record_room declares no revision:\n{out}"
    );
    assert!(
        out.contains("pub const EVENT_NAME: &str = \"record_list.room\";"),
        "{out}"
    );
}
