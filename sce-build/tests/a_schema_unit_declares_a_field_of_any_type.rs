// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The unit an event schema generates declares a field of any type of the one
//! type grammar on every backend (docs/adr/0086, section 7.1, the schema-unit
//! column), held to the fixtures of section 14.1:
//!
//! * `event_batch.scxml`: a list of records that hold lists, a list of lists, a
//!   list of an enum, a list of values that may be absent, a list that may be
//!   absent, a text, and a list a later revision added. Each field is spelled
//!   by its constructors, the record by the payload type its own unit declares,
//!   with the outer bounds and the largest payload named as constants.
//! * `event_batch_item.scxml`: a record that holds a list of texts, its enum
//!   named by its own document's alias.
//! * `legacy_bound_spelling.scxml`: `event_batch_item.scxml` with its outermost
//!   bounds written as the legacy attributes; under the same name it generates
//!   what `event_batch_item.scxml` generates, file for file, and its check
//!   reports each attribute with its rewrite.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::generator::Language;
use tempfile::tempdir;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

fn fixtures() -> PathBuf {
    repo_root().join("sce-build/tests/fixtures/static_datamodel")
}

/// The documents the batch fixtures read, copied beside each other.
const BATCH: [&str; 4] = [
    "enum_signal_level.scxml",
    "event_batch_item.scxml",
    "event_batch.scxml",
    "legacy_bound_spelling.scxml",
];

/// What `sce-codegen <args> <document>` writes, each file by its name, with the
/// directories of the run and every source hash taken out; and the records it
/// wrote on its error stream. `document` is written into the run's directory,
/// beside the batch fixtures, as `name`.
fn run(
    args: &[&str],
    name: &str,
    document: &str,
) -> (bool, Vec<(String, String)>, Vec<serde_json::Value>) {
    let dir = tempdir().expect("tempdir");
    for fixture in BATCH {
        std::fs::copy(fixtures().join(fixture), dir.path().join(fixture)).expect("a fixture");
    }
    std::fs::write(dir.path().join(name), document).expect("write the document");
    let out = tempdir().expect("tempdir");
    let mut command = Command::new(env!("CARGO_BIN_EXE_sce-codegen"));
    command
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(args)
        .arg(dir.path().join(name));
    // `generate` writes a tree; `check` writes nothing.
    if args.first() == Some(&"generate") {
        command
            .args(["--go-module-prefix", "example.test/generated", "-o"])
            .arg(out.path());
    }
    let output = command.output().expect("invoke sce-codegen");
    let places = [
        dir.path().to_string_lossy().into_owned(),
        out.path().to_string_lossy().into_owned(),
    ];
    let mut files = Vec::new();
    let mut pending = vec![out.path().to_path_buf()];
    while let Some(at) = pending.pop() {
        for entry in std::fs::read_dir(&at).expect("the output directory") {
            let path = entry.expect("an entry").path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            let mut text = std::fs::read_to_string(&path).unwrap_or_default();
            for place in &places {
                text = text.replace(place.as_str(), "<dir>");
            }
            let relative = path
                .strip_prefix(out.path())
                .expect("under the output")
                .to_string_lossy()
                .into_owned();
            files.push((relative, without_hashes(&with_char_arrays_read(&text))));
        }
    }
    files.sort();
    let records: Vec<serde_json::Value> = String::from_utf8_lossy(&output.stderr)
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('{'))
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    assert!(
        output.status.success() || !files.is_empty() || !records.is_empty(),
        "sce-codegen said nothing:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    (output.status.success(), files, records)
}

/// The character constant at `at` (`'a'`, `'\''`, `'\x7b'`, `'\0'`): the
/// character it is, and where the text after it begins.
fn char_constant(chars: &[char], at: usize) -> Option<(char, usize)> {
    if chars.get(at) != Some(&'\'') {
        return None;
    }
    match chars.get(at + 1..)? {
        ['\\', 'x', high, low, '\'', ..] => {
            let byte = u8::from_str_radix(&format!("{high}{low}"), 16).ok()?;
            Some((char::from(byte), at + 6))
        }
        ['\\', '0', '\'', ..] => Some(('\0', at + 4)),
        ['\\', escaped, '\'', ..] => Some((*escaped, at + 4)),
        [c, '\'', ..] if *c != '\\' => Some((*c, at + 3)),
        _ => None,
    }
}

/// `text` with each run of two or more character constants, which a C or C++
/// unit spells a text constant in (`'a', 'b', ...`, docs/adr/0087 section
/// 1.1), written as the one string they spell: a source hash inside the
/// payload schema is then a run of hexadecimal characters as anywhere else.
fn with_char_arrays_read(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut at = 0;
    while at < chars.len() {
        let mut spelled = String::new();
        let mut end = at;
        while let Some((c, after)) = char_constant(&chars, end) {
            spelled.push(c);
            end = after;
            let mut next = after;
            while chars
                .get(next)
                .is_some_and(|c| *c == ',' || c.is_whitespace())
            {
                next += 1;
            }
            if char_constant(&chars, next).is_none() {
                break;
            }
            end = next;
        }
        if spelled.chars().count() >= 2 {
            out.push('"');
            out.push_str(&spelled);
            out.push('"');
            at = end;
        } else {
            out.push(chars[at]);
            at += 1;
        }
    }
    out
}

/// `text` with every run of 64 lowercase hexadecimal characters, a source hash,
/// replaced: two documents of one meaning hash apart.
fn without_hashes(text: &str) -> String {
    let mut out = String::new();
    let mut run = String::new();
    for c in text.chars() {
        if c.is_ascii_hexdigit() && !c.is_ascii_uppercase() {
            run.push(c);
            continue;
        }
        out.push_str(if run.len() == 64 { "<hash>" } else { &run });
        run.clear();
        out.push(c);
    }
    out.push_str(if run.len() == 64 { "<hash>" } else { &run });
    out
}

fn fixture(name: &str) -> String {
    std::fs::read_to_string(fixtures().join(name)).expect("the fixture")
}

/// Every file `generate -l language` writes for the fixture `name`, as one text.
fn generated(language: &str, name: &str) -> String {
    let (ok, files, records) = run(&["generate", "-l", language], name, &fixture(name));
    assert!(ok, "generate -l {language} {name}: {records:#?}");
    files
        .into_iter()
        .map(|(_, text)| text)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Each backend's spelling of the composed fields of `event_batch.scxml`, as its
/// unit's struct declares them.
#[test]
fn every_backend_declares_each_composed_field_of_the_batch() {
    for (language, fragments) in [
        (
            "rust",
            &[
                "use super::event_batch_item;",
                "pub items: ::sce_portable_bytes::BoundedList<event_batch_item::EventBatchItemPayload, 3>,",
                "pub grid: ::sce_portable_bytes::BoundedList<::sce_portable_bytes::BoundedList<u8, 2>, 2>,",
                "pub maybe: ::sce_portable_bytes::BoundedList<Option<u16>, 2>,",
                "pub spare: Option<::sce_portable_bytes::BoundedList<u8, 2>>,",
                "pub note: ::sce_portable_bytes::SceString<4>,",
                "pub late: ::sce_portable_bytes::BoundedList<u8, 2>,",
                "pub const ITEMS_MAX_COUNT: u32 = 3;",
            ][..],
        ),
        (
            "kotlin",
            &[
                "val items: List<EventBatchItemPayload>",
                "val grid: List<List<UByte>>",
                "val maybe: List<UShort?>",
                "val spare: List<UByte>?",
            ][..],
        ),
        (
            "cpp",
            &[
                "#include \"event_batch_item.h\"",
                "#include <optional>",
                "#include <sce/forge/bounded.h>",
                "::SCE::Forge::OwnedList<SCE::Generated::EventBatchItem::EventBatchItemPayload, 3> \
                 items{};",
                "::SCE::Forge::OwnedList<::SCE::Forge::OwnedList<uint8_t, 2>, 2> grid{};",
                "::SCE::Forge::OwnedList<std::optional<uint16_t>, 2> maybe{};",
                "std::optional<::SCE::Forge::OwnedList<uint8_t, 2>> spare{};",
                "::SCE::Forge::OwnedText<4> note{};",
                "::SCE::Forge::OwnedList<uint8_t, 2> late{};",
            ][..],
        ),
        (
            "go",
            &[
                "Items []sce_event_batch_item.EventBatchItemPayload",
                "Grid [][]uint8",
                "Maybe []optional.Of[uint16]",
                "Spare optional.Of[[]uint8]",
                "\"github.com/newmassrael/sce-forge-runtime/optional\"",
            ][..],
        ),
        (
            "python",
            &[
                "from typing import Final, Optional",
                "grid: list[list[int]]",
                "maybe: list[Optional[int]]",
                "spare: Optional[list[int]]",
                ".EventBatchItemPayload]",
            ][..],
        ),
        (
            "swift",
            &[
                "public var items: [EventBatchItemPayload] = []",
                "public var grid: [[UInt8]] = []",
                "public var maybe: [UInt16?] = []",
                "public var spare: [UInt8]? = nil",
            ][..],
        ),
        (
            "c",
            &[
                "#include \"event_batch_item.h\"",
                "sce_list_3_record_EventBatchItemPayload_t items;",
                "sce_list_2_list_2_uint8_t grid;",
                "sce_list_2_opt_uint16_t maybe;",
                "sce_opt_list_2_uint8_t spare;",
                "} sce_list_2_uint8_t;",
                "    sce_list_2_uint8_t items[2];",
                "#define EVENT_BATCH_ITEMS_MAX_COUNT UINT32_C(3)",
            ][..],
        ),
    ] {
        let out = generated(language, "event_batch.scxml");
        for fragment in fragments {
            assert!(
                out.contains(fragment),
                "{language}: the batch's unit declares `{fragment}`:\n{out}"
            );
        }
    }
}

/// A record that holds a list is spelled with every leaf the unit has always
/// spelled, its list of texts by its constructors.
#[test]
fn every_backend_declares_a_record_that_holds_a_list_of_texts() {
    for (language, fragment) in [
        (
            "rust",
            "pub tags: ::sce_portable_bytes::BoundedList<::sce_portable_bytes::SceString<8>, 4>,",
        ),
        ("kotlin", "val tags: List<String>"),
        (
            "cpp",
            "::SCE::Forge::OwnedList<::SCE::Forge::OwnedText<8>, 4> tags{};",
        ),
        ("go", "Tags []string"),
        ("python", "tags: list[str]"),
        ("swift", "public var tags: [String] = []"),
        ("c", "sce_list_4_string_8_t tags;"),
    ] {
        let out = generated(language, "event_batch_item.scxml");
        assert!(
            out.contains(fragment),
            "{language}: the item's unit declares `{fragment}`:\n{out}"
        );
    }
}

/// The batch's largest payload is sized with the record's fields read in the
/// record's own document (docs/adr/0086, section 2.1). An item is at most 403
/// bytes: braces and six commas (8), its seven quoted keys and colons (50), and
/// its values at their longest, `id` 10, `amount` 22, `label` 50, `level` 10
/// (`"critical"`), `due` 22, `blob` 26 and `tags` 205. The batch is braces and
/// six commas (8), its keys (54), and `items` 1213 (three items and two
/// commas), `grid` 21, `levels` 23, `maybe` 13, `spare` 9, `note` 26, `late` 9.
#[test]
fn the_batch_names_its_largest_payload() {
    let out = generated("rust", "event_batch.scxml");
    assert!(
        out.contains("pub const DATA_MAX_LEN: u64 = 1376;"),
        "the unit names its largest payload:\n{out}"
    );
    let item = generated("rust", "event_batch_item.scxml");
    assert!(
        item.contains("pub const DATA_MAX_LEN: u64 = 403;"),
        "the item names its largest payload:\n{item}"
    );
}

/// The legacy spelling of the item, under the item's name, generates what the
/// item generates on every backend, file for file.
#[test]
fn the_legacy_spelling_generates_what_the_bracket_generates() {
    let legacy = fixture("legacy_bound_spelling.scxml").replace(
        "name=\"legacy_bound_spelling\"",
        "name=\"event_batch_item\"",
    );
    for language in Language::ALL
        .iter()
        .map(|language| language.canonical_name())
    {
        let args = ["generate", "-l", language];
        let (ok, from_legacy, records) = run(&args, "event_batch_item.scxml", &legacy);
        assert!(
            ok,
            "{language}: the legacy spelling generates: {records:#?}"
        );
        let (ok, from_bracket, records) = run(
            &args,
            "event_batch_item.scxml",
            &fixture("event_batch_item.scxml"),
        );
        assert!(
            ok,
            "{language}: the bracket spelling generates: {records:#?}"
        );
        assert!(!from_bracket.is_empty(), "{language}: a unit is written");
        assert_eq!(from_legacy, from_bracket, "{language}");
    }
}

/// `check` reads each legacy attribute as its bracket and says so, one record
/// for each, with the type that carries the bound in its place.
#[test]
fn the_legacy_spelling_is_reported_once_for_each_attribute() {
    let (ok, _, records) = run(
        &["check", "-l", "rust"],
        "legacy_bound_spelling.scxml",
        &fixture("legacy_bound_spelling.scxml"),
    );
    assert!(ok, "a legacy bound is read, not refused: {records:#?}");
    let superseded: Vec<&serde_json::Value> = records
        .iter()
        .filter(|r| r["code"] == "validation/bound-attribute-superseded")
        .collect();
    assert_eq!(superseded.len(), 3, "{records:#?}");
    let types: Vec<&str> = superseded
        .iter()
        .filter_map(|r| r["fix"]["type"].as_str())
        .collect();
    for ty in ["string[8]", "bytes[4]", "list<string[8]>[4]"] {
        assert!(types.contains(&ty), "{ty} in {types:?}");
    }
}
