// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A change to the shared record list is one event, `record_list.change`
//! (`stdlib/shared/record_change.scxml`), whose operations are a bounded list of
//! records (`stdlib/shared/record_op.scxml`), and the largest change is a number the
//! build knows (docs/adr/0085, sections 1.1 and 5): every backend's unit names it, a
//! build that moves `shared_parts` or `merge_text` moves it, and the type holds at
//! least one operation, since a change that names nothing is never sent.
//!
//! The number is what C11's slot is sized by once the slot is derived from the
//! machine's largest payload (docs/adr/0086, landing step 5), and what a transport
//! whose frame is smaller fragments below the delivery layer (docs/adr/0085, section 5).

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

/// The change, `record_list.change`.
const CHANGE: &str = "stdlib/shared/record_change.scxml";

/// One operation of a change.
const OP: &str = "stdlib/shared/record_op.scxml";

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
        "generate {document} -l {language} under {given:?}:\n{}",
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

/// The longest spelling of a signed 64-bit integer: the quoted text of its digits.
const WIDE: &str = "\"-9223372036854775808\"";

/// The longest spelling of an unsigned 64-bit integer, quoted as a signed one is.
const WIDE_UNSIGNED: &str = "\"18446744073709551615\"";

/// The largest operation with its text at `text_bound` bytes, written out: every
/// number at its longest spelling and the text at its bound of control characters at
/// their longest escape.
fn largest_op(text_bound: usize) -> String {
    format!(
        "{{\"kind\":255,\"id\":4294967295,\"field\":65535,\"value\":{WIDE},\"text\":\"{}\",\
         \"wallTime\":{WIDE},\"counter\":4294967295,\"nodeId\":{WIDE_UNSIGNED}}}",
        "\\u0001".repeat(text_bound)
    )
}

/// The largest change of `parts` operations, each the largest with its text at
/// `text_bound` bytes.
fn largest_change(parts: usize, text_bound: usize) -> String {
    let ops = vec![largest_op(text_bound); parts].join(",");
    format!("{{\"by\":{WIDE_UNSIGNED},\"wallTime\":{WIDE},\"counter\":4294967295,\"ops\":[{ops}]}}")
}

/// Each backend's spelling of a unit's largest data, `{}` standing for the number.
const DATA_MAX_LEN: [(&str, &str); 7] = [
    ("rust", "pub const DATA_MAX_LEN: u64 = {};"),
    ("kotlin", "const val DATA_MAX_LEN: ULong = {}uL"),
    ("cpp", "inline constexpr std::uint64_t DATA_MAX_LEN = {}u;"),
    ("go", "const DataMaxLen uint64 = {}"),
    ("python", "DATA_MAX_LEN: Final[int] = {}"),
    ("c", "#define RECORD_CHANGE_DATA_MAX_LEN UINT64_C({})"),
    ("swift", "public static let dataMaxLen: UInt64 = {}"),
];

/// The largest change of the list as written is 47,900 bytes: 64 operations, each
/// with 96 bytes of text. A build that gives `shared_parts` 8 holds one of 6,068, and
/// one that gives `merge_text` 32 one of 23,324, and every backend's unit says so.
#[test]
fn every_backend_names_the_largest_change_as_the_build_bounds_it() {
    let written = largest_change(64, 96).len();
    let fewer = largest_change(8, 96).len();
    let shorter = largest_change(64, 32).len();
    assert_eq!((written, fewer, shorter), (47_900, 6_068, 23_324));
    for (language, spelling) in DATA_MAX_LEN {
        for (given, size) in [
            (&[][..], written),
            (&["shared_parts=8"][..], fewer),
            (&["merge_text=32"][..], shorter),
        ] {
            let out = generated(CHANGE, language, given);
            let fragment = spelling.replace("{}", &size.to_string());
            assert!(
                out.contains(&fragment),
                "{language} under {given:?}: the unit declares `{fragment}`:\n{out}"
            );
        }
    }
}

/// `ops` holds at most `shared_parts` operations, a bound the build may move, and at
/// least one: the payload's grammar refuses an empty list, so no change that names
/// nothing can be written on any wire.
#[test]
fn a_change_holds_at_least_one_operation_and_at_most_shared_parts() {
    for (language, written, moved) in [
        (
            "rust",
            "pub const OPS_MAX_COUNT: u32 = 64;",
            "pub const OPS_MAX_COUNT: u32 = 8;",
        ),
        (
            "kotlin",
            "const val OPS_MAX_COUNT: UInt = 64u",
            "const val OPS_MAX_COUNT: UInt = 8u",
        ),
        (
            "go",
            "const OpsMaxCount uint32 = 64",
            "const OpsMaxCount uint32 = 8",
        ),
        (
            "c",
            "#define RECORD_CHANGE_OPS_MAX_COUNT UINT32_C(64)",
            "#define RECORD_CHANGE_OPS_MAX_COUNT UINT32_C(8)",
        ),
        (
            "swift",
            "public static let opsMaxCount: UInt32 = 64",
            "public static let opsMaxCount: UInt32 = 8",
        ),
    ] {
        let as_written = generated(CHANGE, language, &[]);
        assert!(
            as_written.contains(written) && as_written.contains("shared_parts"),
            "{language}: the unit declares `{written}` and names the bound:\n{as_written}"
        );
        let built = generated(CHANGE, language, &["shared_parts=8"]);
        assert!(
            built.contains(moved),
            "{language}: the build's number moves the bound to `{moved}`:\n{built}"
        );
    }
    let rust = generated(CHANGE, "rust", &[]);
    for fragment in [
        "pub const EVENT_NAME: &str = \"record_list.change\";",
        "pub const REVISION: u32 = 1;",
        "\\\"minItems\\\":1",
        "\\\"maxItems\\\":64",
    ] {
        assert!(
            rust.contains(fragment),
            "the unit holds `{fragment}`:\n{rust}"
        );
    }
}

/// An operation carries a value's text whole, `merge_text` bytes, so its largest is
/// 746 bytes as written and moves with the bound.
#[test]
fn an_operation_carries_the_text_of_a_value_whole() {
    let written = largest_op(96).len();
    let moved = largest_op(32).len();
    assert_eq!((written, moved), (746, 362));
    let out = generated(OP, "rust", &[]);
    for fragment in [
        "pub const TEXT_MAX_SIZE: u32 = 96;".to_string(),
        format!("pub const DATA_MAX_LEN: u64 = {written};"),
    ] {
        assert!(
            out.contains(&fragment),
            "the unit declares `{fragment}`:\n{out}"
        );
    }
    let built = generated(OP, "rust", &["merge_text=32"]);
    for fragment in [
        "pub const TEXT_MAX_SIZE: u32 = 32;".to_string(),
        format!("pub const DATA_MAX_LEN: u64 = {moved};"),
    ] {
        assert!(
            built.contains(&fragment),
            "the build's number moves `{fragment}`:\n{built}"
        );
    }
}

/// What a copy that keeps a journal offers its host of a change (`record_list.made`) is
/// the change and the host's command that ended it, one report per change
/// (docs/adr/0085, section 7.1); a host gives it back as `record_list.recall` with
/// whether it was provisional (docs/adr/0093, section 3.5); and a stable state is handed
/// over as `record_list.restore`, the change's fields read as a state (docs/adr/0085,
/// section 4.4). So each is a number the build knows: the largest change and a known
/// number of bytes more, and every one moves with `shared_parts`.
#[test]
fn a_change_offered_recalled_or_restored_is_the_change_and_a_known_number_of_bytes_more() {
    let with_command = |change: String| {
        let body = change.strip_suffix('}').expect("a change is an object");
        format!("{body},\"command\":{WIDE_UNSIGNED}}}")
    };
    let with_flag = |made: String| {
        let body = made.strip_suffix('}').expect("a report is an object");
        format!("{body},\"provisional\":false}}")
    };
    let made = with_command(largest_change(64, 96)).len();
    let recalled = with_flag(with_command(largest_change(64, 96))).len();
    let restored = largest_change(64, 96).len();
    assert_eq!((made, recalled, restored), (47_933, 47_953, 47_900));
    let fewer = with_command(largest_change(8, 96)).len();
    for (document, written, moved) in [
        ("stdlib/shared/record_made.scxml", made, fewer),
        (
            "stdlib/shared/record_recall.scxml",
            recalled,
            with_flag(with_command(largest_change(8, 96))).len(),
        ),
        (
            "stdlib/shared/record_restore.scxml",
            restored,
            largest_change(8, 96).len(),
        ),
    ] {
        for (given, size) in [(&[][..], written), (&["shared_parts=8"][..], moved)] {
            let out = generated(document, "rust", given);
            let fragment = format!("pub const DATA_MAX_LEN: u64 = {size};");
            assert!(
                out.contains(&fragment),
                "{document} under {given:?}: the unit declares `{fragment}`:\n{out}"
            );
        }
    }
}
