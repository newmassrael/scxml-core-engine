// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A CBOR or content-line codec document declares the format it is written in
//! (docs/adr/0055, section 8).
//!
//! # What was wrong
//!
//! An entry that may be absent was spelt, in the codecs, by leaving
//! `sce:required="true"` off it: the reverse of the rule everywhere else, where a
//! bare type is present and `optional<T>` may be absent. A codec of bare types
//! only is a codec of optional entries in the old spelling and of required entries
//! in the new one, so no document could be told from one in the other spelling by
//! its entries alone, and a rule that read the old documents with the new meaning
//! would have turned every optional entry required in silence.
//!
//! # What is held
//!
//! * Format 2 is the only one read: no `sce:codec-format`, or `="1"`, is
//!   `validation/codec-format-superseded` on the root, and its fix is the
//!   document's own rewrite, the type of every entry; any other value is
//!   `validation/invalid-attribute` naming `2`.
//! * The rewrite is computed from what format 1 says: `sce:required="true"` is
//!   the bare type, and no `sce:required`, or `="false"`, is `optional<T>`.
//! * `sce-codegen codec-format` prints the document so rewritten, byte for byte
//!   what the repository's own documents became, and `check` accepts it.
//! * `sce:required` is no part of format 2, and a positional codec declares no
//!   format at all.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use sce_build::forge::model::{CodecEncoding, ForgeDocument};
use sce_build::forge::parser::parse_forge;
use sce_build::DocumentLabel;

fn codegen() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// The row and column `needle` starts at in `text`, counted from 1, a column per
/// character.
fn written_at(text: &str, needle: &str) -> (u64, u64) {
    let offset = text.find(needle).expect("the document spells the needle");
    let before = &text[..offset];
    let row = before.matches('\n').count() as u64 + 1;
    let col = before[before.rfind('\n').map_or(0, |i| i + 1)..]
        .chars()
        .count() as u64
        + 1;
    (row, col)
}

/// `document` written to a file of its own, and the directory that holds it.
fn written(document: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("doc.scxml");
    std::fs::write(&path, document).expect("write the document");
    (dir, path)
}

/// `sce-codegen check -l rust` of `path`, with the records it wrote.
fn check(path: &Path) -> (Output, Vec<serde_json::Value>) {
    let run = codegen()
        .args(["--error-format=json", "check"])
        .arg(path)
        .args(["-l", "rust"])
        .output()
        .expect("spawn sce-codegen");
    let records = String::from_utf8_lossy(&run.stderr)
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('{'))
        .map(|line| serde_json::from_str(line).expect("one JSON record per line"))
        .collect();
    (run, records)
}

/// The one record of `code` that `check` refuses `document` with.
fn refusal(document: &str, code: &str) -> serde_json::Value {
    let (_dir, path) = written(document);
    let (run, records) = check(&path);
    assert_ne!(
        run.status.code(),
        Some(0),
        "the document was accepted:\n{document}"
    );
    let mut of_code = records.iter().filter(|r| r["code"] == code);
    match (of_code.next(), of_code.next()) {
        (Some(one), None) => one.clone(),
        _ => panic!("expected one {code} record, got {records:?}"),
    }
}

/// A CBOR codec whose root carries `format` (an attribute, or nothing) beside
/// `sce:encoding`, and whose `<datamodel>` holds `entries` verbatim.
fn cbor(format: &str, entries: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:encoding="cbor"{format} name="doc" version="1.0">
  <datamodel>
    <data id="raw" sce:type="bytes" sce:direction="in"/>
{entries}
  </datamodel>
</scxml>
"#
    )
}

/// A content-line codec of the same shape.
fn content_line(format: &str, entries: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:encoding="content-line"{format} sce:component="VEVENT" name="doc" version="1.0">
  <datamodel>
    <data id="raw" sce:type="bytes" sce:direction="in"/>
{entries}
  </datamodel>
</scxml>
"#
    )
}

/// Entries that say nothing of `sce:required`: every one of them was optional in
/// format 1.
const BARE_ENTRIES: &str = r#"    <data id="n" sce:type="uint8" sce:key="0" sce:direction="out"/>
    <data id="s" sce:type="string" sce:key="1" sce:max-size="4" sce:direction="out"/>
    <data id="b" sce:type="bytes" sce:key="2" sce:length="4" sce:direction="out"/>"#;

/// Entries that say it each way: `true`, `false` and nothing.
const MIXED_ENTRIES: &str = r#"    <data id="a" sce:type="uint8" sce:key="0" sce:required="true" sce:direction="out"/>
    <data id="b" sce:type="uint8" sce:key="1" sce:required="false" sce:direction="out"/>
    <data id="c" sce:type="uint8" sce:key="2" sce:direction="out"/>
    <data id="d" sce:type="bytes" sce:key="3" sce:required="true" sce:length="4" sce:direction="out"/>"#;

/// The entries of the fix `record` carries, as `(id, type)` pairs.
fn rewrite_of(record: &serde_json::Value) -> Vec<(String, String)> {
    assert_eq!(record["fix"]["kind"], "rewrite_codec_format", "{record}");
    assert_eq!(record["fix"]["format"], "2", "{record}");
    record["fix"]["entries"]
        .as_array()
        .unwrap_or_else(|| panic!("the fix lists entries: {record}"))
        .iter()
        .map(|e| {
            (
                e["id"].as_str().expect("an id").to_string(),
                e["type"].as_str().expect("a type").to_string(),
            )
        })
        .collect()
}

fn pairs(rows: &[(&str, &str)]) -> Vec<(String, String)> {
    rows.iter()
        .map(|(id, ty)| ((*id).to_string(), (*ty).to_string()))
        .collect()
}

#[test]
fn a_document_that_declares_no_format_is_refused_at_its_root_with_every_entry_optional() {
    let document = cbor("", BARE_ENTRIES);
    let record = refusal(&document, "validation/codec-format-superseded");
    // Entries that said nothing of `sce:required` were optional, and are written
    // as the optional type of what they are.
    assert_eq!(
        rewrite_of(&record),
        pairs(&[
            ("n", "optional<uint8>"),
            ("s", "optional<string>"),
            ("b", "optional<bytes>"),
        ])
    );
    // Placed on the root, and no `actual`: nothing on its row is the value refused.
    assert_eq!(record["location"]["line"], 2, "{record}");
    assert!(record.get("actual").is_none(), "{record}");
    assert!(record.get("expected").is_none(), "{record}");
}

#[test]
fn a_document_in_format_1_is_refused_at_the_declaration_it_wrote() {
    let document = cbor(r#" sce:codec-format="1""#, BARE_ENTRIES);
    let record = refusal(&document, "validation/codec-format-superseded");
    assert_eq!(record["actual"], "sce:codec-format", "{record}");
    let (row, col) = written_at(&document, "sce:codec-format");
    assert_eq!(
        (
            record["location"]["line"].as_u64(),
            record["location"]["col"].as_u64()
        ),
        (Some(row), Some(col)),
        "the record stands where the declaration is written: {record}"
    );
    assert_eq!(rewrite_of(&record).len(), 3, "{record}");
}

#[test]
fn any_other_format_is_refused_naming_the_one_there_is() {
    for written_format in ["3", "0", "two"] {
        let document = cbor(
            &format!(r#" sce:codec-format="{written_format}""#),
            r#"    <data id="n" sce:type="uint8" sce:key="0" sce:direction="out"/>"#,
        );
        let record = refusal(&document, "validation/invalid-attribute");
        assert_eq!(record["actual"], written_format, "{record}");
        assert_eq!(record["fix"]["kind"], "replace_one_of", "{record}");
        assert_eq!(record["fix"]["candidates"], serde_json::json!(["2"]));
    }
}

#[test]
fn a_document_that_says_it_each_way_is_rewritten_entry_by_entry() {
    let record = refusal(
        &cbor("", MIXED_ENTRIES),
        "validation/codec-format-superseded",
    );
    // `true` is the bare type; `false`, and nothing, the optional one.
    assert_eq!(
        rewrite_of(&record),
        pairs(&[
            ("a", "uint8"),
            ("b", "optional<uint8>"),
            ("c", "optional<uint8>"),
            ("d", "bytes"),
        ])
    );
}

#[test]
fn a_content_line_codec_is_refused_and_rewritten_the_same_way() {
    let entries = r#"    <data id="uid" sce:type="string" sce:property="UID" sce:required="true" sce:max-size="8"/>
    <data id="uidKind" sce:type="string" sce:property="UID" sce:param="KIND" sce:required="true" sce:max-size="8"/>
    <data id="note" sce:type="string" sce:property="NOTE" sce:max-size="8"/>
    <data id="rank" sce:type="uint8" sce:property="RANK" sce:required="false"/>"#;
    for format in ["", r#" sce:codec-format="1""#] {
        let record = refusal(
            &content_line(format, entries),
            "validation/codec-format-superseded",
        );
        assert_eq!(
            rewrite_of(&record),
            pairs(&[
                ("uid", "string"),
                ("uidKind", "string"),
                ("note", "optional<string>"),
                ("rank", "optional<uint8>"),
            ]),
            "{format}"
        );
    }
    let record = refusal(
        &content_line(r#" sce:codec-format="3""#, entries),
        "validation/invalid-attribute",
    );
    assert_eq!(record["fix"]["candidates"], serde_json::json!(["2"]));
}

#[test]
fn a_type_that_format_1_has_no_word_for_is_not_rewritten_but_told_why() {
    // A document that writes `optional<T>` is written in format 2 and does not
    // say so: no rewrite is the repair, the declaration is.
    let document = cbor(
        "",
        r#"    <data id="n" sce:type="optional&lt;uint8&gt;" sce:key="0" sce:direction="out"/>"#,
    );
    let record = refusal(&document, "validation/attribute-rule-violated");
    assert_eq!(record["actual"], "optional&lt;uint8&gt;", "{record}");
    assert!(
        record["expected"][0]
            .as_str()
            .is_some_and(|rule| rule.contains("sce:codec-format")),
        "{record}"
    );
}

#[test]
fn sce_required_is_refused_in_format_2_naming_both_spellings_of_what_it_meant() {
    for (written_required, bare_in_message) in [("true", "`uint8`"), ("false", "`uint8`")] {
        let document = cbor(
            r#" sce:codec-format="2""#,
            &format!(
                r#"    <data id="n" sce:type="uint8" sce:key="0" sce:required="{written_required}" sce:direction="out"/>"#
            ),
        );
        let record = refusal(&document, "validation/attribute-rule-violated");
        assert_eq!(record["actual"], written_required, "{record}");
        let rule = record["expected"][0].as_str().expect("the rule");
        assert!(
            rule.contains(bare_in_message) && rule.contains("`optional<uint8>`"),
            "the rule names the bare type and optional<T>: {rule}"
        );
        // Placed on the attribute, which is on the entry's row.
        let (row, _) = written_at(&document, "sce:required");
        assert_eq!(record["location"]["line"].as_u64(), Some(row), "{record}");
    }
    // A content-line entry the same.
    let record = refusal(
        &content_line(
            r#" sce:codec-format="2""#,
            r#"    <data id="uid" sce:type="string" sce:property="UID" sce:required="true" sce:max-size="8"/>"#,
        ),
        "validation/attribute-rule-violated",
    );
    assert!(
        record["expected"][0]
            .as_str()
            .is_some_and(|rule| rule.contains("`string`") && rule.contains("`optional<string>`")),
        "{record}"
    );
}

#[test]
fn a_positional_codec_declares_no_format() {
    let document = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" sce:codec-format="2" name="doc">
  <datamodel>
    <data id="raw" sce:type="bytes" sce:direction="in"/>
    <sce:field id="n" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
  </datamodel>
</scxml>
"#;
    let record = refusal(document, "validation/sce-attribute-unread");
    assert_eq!(record["actual"], "sce:codec-format", "{record}");
}

#[test]
fn a_bare_type_is_required_and_optional_is_optional_in_the_model() {
    let label = DocumentLabel {
        identifier: "doc",
        diagnostic_label: "doc",
    };
    let text = cbor(r#" sce:codec-format="2""#, MIXED_ENTRIES)
        .replace(r#" sce:required="true""#, "")
        .replace(r#" sce:required="false""#, "")
        .replace(
            r#"id="b" sce:type="uint8""#,
            r#"id="b" sce:type="optional&lt;uint8&gt;""#,
        )
        .replace(
            r#"id="c" sce:type="uint8""#,
            r#"id="c" sce:type="optional&lt;uint8&gt;""#,
        );
    let Ok(Some(ForgeDocument::Codec(m))) = parse_forge(&text, label) else {
        panic!("expected a codec in format 2:\n{text}");
    };
    assert_eq!(m.encoding, CodecEncoding::Cbor);
    let required: Vec<(&str, bool)> = m
        .cbor_entries
        .iter()
        .map(|e| (e.id.as_str(), e.is_required()))
        .collect();
    assert_eq!(
        required,
        [("a", true), ("b", false), ("c", false), ("d", true)]
    );
}

/// `sce-codegen codec-format` of `document`, written to a file of its own.
fn codec_format(document: &str) -> Output {
    let (_dir, path) = written(document);
    codegen()
        .arg("codec-format")
        .arg(path)
        .output()
        .expect("spawn sce-codegen")
}

/// What `sce-codegen codec-format` printed for `document`, which it must have
/// printed without refusing.
fn printed(document: &str) -> String {
    let run = codec_format(document);
    assert!(
        run.status.success(),
        "codec-format refused:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    String::from_utf8(run.stdout).expect("the document is UTF-8")
}

#[test]
fn codec_format_prints_a_document_that_check_accepts() {
    for document in [
        cbor("", MIXED_ENTRIES),
        content_line("", MIXED_CONTENT_LINE),
    ] {
        let rewritten = printed(&document);
        assert!(
            rewritten.contains(r#"sce:codec-format="2""#) && !rewritten.contains("sce:required"),
            "{rewritten}"
        );
        let (_dir, path) = written(&rewritten);
        let (run, records) = check(&path);
        assert_eq!(
            run.status.code(),
            Some(0),
            "check refused the rewrite:\n{rewritten}\n{records:?}"
        );
    }
}

/// Entries of a content-line codec that a rewrite must keep accepted: a property
/// that must be present with a parameter of it that must be, and optional ones.
const MIXED_CONTENT_LINE: &str = r#"    <data id="uid" sce:type="string" sce:property="UID" sce:required="true" sce:max-size="8"/>
    <data id="uidKind" sce:type="string" sce:property="UID" sce:param="KIND" sce:required="true" sce:max-size="8"/>
    <data id="note" sce:type="string" sce:property="NOTE" sce:max-size="8"/>
    <data id="noteKind" sce:type="string" sce:property="NOTE" sce:param="KIND" sce:max-size="8"/>
    <data id="rank" sce:type="uint8" sce:property="RANK" sce:required="false"/>"#;

#[test]
fn the_document_codec_format_prints_reads_with_the_optionality_format_1_said() {
    let label = DocumentLabel {
        identifier: "doc",
        diagnostic_label: "doc",
    };
    let rewritten = printed(&cbor("", MIXED_ENTRIES));
    let Ok(Some(ForgeDocument::Codec(m))) = parse_forge(&rewritten, label) else {
        panic!("expected a codec in format 2:\n{rewritten}");
    };
    let required: Vec<(&str, bool)> = m
        .cbor_entries
        .iter()
        .map(|e| (e.id.as_str(), e.is_required()))
        .collect();
    assert_eq!(
        required,
        [("a", true), ("b", false), ("c", false), ("d", true)]
    );

    let rewritten = printed(&content_line("", MIXED_CONTENT_LINE));
    let Ok(Some(ForgeDocument::Codec(m))) = parse_forge(&rewritten, label) else {
        panic!("expected a codec in format 2:\n{rewritten}");
    };
    let entries = m.content_line.expect("a content-line codec").entries;
    let required: Vec<(&str, bool)> = entries
        .iter()
        .map(|e| (e.id.as_str(), e.is_required()))
        .collect();
    assert_eq!(
        required,
        [
            ("uid", true),
            ("uidKind", true),
            ("note", false),
            ("noteKind", false),
            ("rank", false),
        ]
    );
}

/// The root of `codec_cbor_map.scxml` and everything below it, as it was written
/// in format 1.
const CBOR_MAP_FORMAT_1: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:encoding="cbor" name="codec_cbor_map" version="1.0">
  <datamodel>
    <data id="raw" sce:type="bytes" sce:direction="in"/>
    <data id="source" sce:type="string" sce:key="1" sce:required="true" sce:max-size="64" sce:direction="out"/>
    <data id="id" sce:type="bytes" sce:key="0" sce:required="true" sce:length="16" sce:direction="out"/>
    <data id="pattern" sce:type="uint8" sce:key="3" sce:required="true" sce:direction="out"/>
    <data id="payload" sce:type="bytes" sce:key="5" sce:required="true" sce:max-size="256" sce:direction="out"/>
    <data id="deadline" sce:type="uint64" sce:key="12" sce:direction="out"/>
    <data id="urgent" sce:type="bool" sce:key="7" sce:direction="out"/>
  </datamodel>
</scxml>
"#;

/// The root of `codec_content_line_records.scxml` and everything below it, as it
/// was written in format 1.
const CONTENT_LINE_RECORDS_FORMAT_1: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0"
       sce:kind="codec" sce:encoding="content-line" sce:component="VEVENT"
       name="codec_content_line_records">
  <datamodel>
    <data id="raw" sce:type="bytes" sce:direction="in"/>
    <data id="uid" sce:type="string" sce:property="UID" sce:required="true" sce:max-size="24"/>
    <data id="attendee" sce:type="string" sce:property="ATTENDEE" sce:max-count="3" sce:max-size="40"/>
    <data id="attendeeCn" sce:type="string" sce:property="ATTENDEE" sce:param="CN" sce:max-size="24"/>
    <data id="attendeeRole" sce:type="string" sce:property="ATTENDEE" sce:param="ROLE" sce:required="true" sce:max-size="16"/>
    <data id="exdate" sce:type="string" sce:property="EXDATE" sce:max-count="3" sce:max-size="20" sce:separator="," sce:max-values="3"/>
    <data id="exdateTzid" sce:type="string" sce:property="EXDATE" sce:param="TZID" sce:max-size="24"/>
    <data id="categories" sce:type="string" sce:property="CATEGORIES" sce:value="text" sce:max-size="12" sce:separator="," sce:max-values="4"/>
    <data id="geo" sce:type="string" sce:property="GEO" sce:max-size="16" sce:separator=";" sce:max-values="2"/>
  </datamodel>
</scxml>
"#;

/// The text of a repository document from its root on: what a rewrite changes,
/// everything above it being a comment.
fn from_the_root(relative: &str) -> String {
    let text = std::fs::read_to_string(repository().join(relative)).expect("the document");
    let at = text.find("<scxml").expect("a root element");
    text[at..].to_string()
}

/// The repository's own codec documents were moved to format 2 by hand, the rule
/// applied entry by entry; the tool's rewrite of what they were is what they are.
/// A fixture whose conformance cases hold the bytes it generates is therefore the
/// document the rewrite of its format 1 says, and those bytes are not changed.
#[test]
fn the_rewrite_of_a_documents_format_1_is_the_document_the_repository_holds() {
    for (format_1, relative) in [
        (
            CBOR_MAP_FORMAT_1,
            "tests/forge/resources/codec_cbor_map.scxml",
        ),
        (
            CONTENT_LINE_RECORDS_FORMAT_1,
            "tests/forge/resources/codec_content_line_records.scxml",
        ),
    ] {
        assert_eq!(printed(format_1), from_the_root(relative), "{relative}");
    }
}

#[test]
fn a_document_in_format_2_is_printed_as_it_is_and_one_that_is_no_codec_is_refused() {
    let in_format_2 = cbor(
        r#" sce:codec-format="2""#,
        r#"    <data id="n" sce:type="optional&lt;uint8&gt;" sce:key="0" sce:direction="out"/>"#,
    );
    assert_eq!(printed(&in_format_2), in_format_2);

    // A positional codec has no format to rewrite.
    let positional = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="doc">
  <datamodel>
    <data id="raw" sce:type="bytes" sce:direction="in"/>
    <sce:field id="n" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
  </datamodel>
</scxml>
"#;
    let run = codec_format(positional);
    assert_ne!(run.status.code(), Some(0));
    assert!(
        String::from_utf8_lossy(&run.stderr).contains("not one"),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
}

#[test]
fn a_document_that_expands_entries_is_not_rewritten_as_if_they_were_its_text() {
    let document = cbor(
        "",
        r#"    <xi:include href="entries.xml" xmlns:xi="http://www.w3.org/2001/XInclude"/>"#,
    );
    let run = codec_format(&document);
    assert_ne!(run.status.code(), Some(0));
    assert!(
        String::from_utf8_lossy(&run.stderr).contains("xi:include"),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
}
