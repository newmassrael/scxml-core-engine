// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A codec field refused after parsing is placed at the element that
//! declares it, on the attribute the record reports.
//!
//! # What was wrong
//!
//! The codec's post-parse validators — a repeat's count, a present-if
//! predicate, a tail that is not last, a bytes field with a bit count, a
//! length or embed source, DMA alignment — judge a field from its model and
//! raised every refusal at `<datamodel>`. Its attributes hold none of the
//! values those records report, so each record stood on the `<datamodel>`
//! row, above the element that wrote the text (measured 2026-09-24). A
//! repeat's count refusal also named the attribute `sce:count`, which the
//! grammar does not declare — documents write `count`.
//!
//! # What is held
//!
//! Each case writes the offending field rows below `<datamodel>`, and each
//! record must stand where its `actual` is written — read off the fixture
//! text, not restated.

use std::path::Path;
use std::process::Command;

/// The row and column `needle` starts at in `text`, counted from 1, a
/// column per character.
fn written_at(text: &str, needle: &str) -> (u64, u64) {
    let offset = text.find(needle).expect("the fixture spells the needle");
    let before = &text[..offset];
    let row = before.matches('\n').count() as u64 + 1;
    let col = before[before.rfind('\n').map_or(0, |i| i + 1)..]
        .chars()
        .count() as u64
        + 1;
    (row, col)
}

/// The one record of `code` that `check` refuses `document` with, the
/// imported sibling the repeat cases name written beside it.
fn refusal(document: &str, code: &str) -> serde_json::Value {
    let dir = tempfile::tempdir().expect("tempdir");
    let resources = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/resources");
    std::fs::copy(
        resources.join("codec_repeat_elem.scxml"),
        dir.path().join("codec_repeat_elem.scxml"),
    )
    .expect("copy sibling");
    let path = dir.path().join("doc.scxml");
    std::fs::write(&path, document).expect("write document");
    let run = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .args([
            "--error-format=json",
            "check",
            path.to_str().unwrap(),
            "-l",
            "rust",
        ])
        .output()
        .expect("spawn sce-codegen");
    let records: Vec<serde_json::Value> = String::from_utf8_lossy(&run.stderr)
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('{'))
        .map(|line| serde_json::from_str(line).expect("one JSON record per line"))
        .collect();
    assert_ne!(
        run.status.code(),
        Some(0),
        "the document built:\n{document}"
    );
    let mut of_code = records.iter().filter(|r| r["code"] == code);
    match (of_code.next(), of_code.next()) {
        (Some(one), None) => one.clone(),
        _ => panic!("expected one {code} record, got {records:?}"),
    }
}

/// `record` reports `actual` and stands where `needle` starts in `document`.
fn assert_placed(record: &serde_json::Value, document: &str, actual: &str, needle: &str) {
    assert_eq!(record["actual"], actual, "{record}");
    let (row, col) = written_at(document, needle);
    assert_eq!(
        (
            record["location"]["line"].as_u64(),
            record["location"]["col"].as_u64()
        ),
        (Some(row), Some(col)),
        "the record does not stand where its actual is written: {record}"
    );
}

fn codec(datamodel: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="doc">
  <sce:import src="codec_repeat_elem.scxml" kind="codec" as="codec_repeat_elem"/>
  <datamodel>
{datamodel}
  </datamodel>
</scxml>
"#
    )
}

/// A count naming a field declared after the repeat stands at the repeat,
/// on the field it refuses — not on the `<datamodel>` row.
#[test]
fn a_repeat_counting_a_later_field_stands_at_the_repeat() {
    let document = codec(
        r#"    <sce:repeat id="msgs" type="codec_repeat_elem" sce:byte="0"
                count="n" max-count="8"/>
    <sce:field id="n" sce:type="uint8" sce:byte="1" sce:bit-size="8"/>"#,
    );
    let record = refusal(&document, "codec/repeat-count-refs-later-field");
    assert_placed(&record, &document, "msgs", "id=\"msgs\"");
    assert!(
        record["message"]
            .as_str()
            .is_some_and(|m| m.contains("count=\"n\"") && !m.contains("sce:count")),
        "the message names the attribute as the document writes it: {record}"
    );
}

/// A count naming a field that is no integer stands at the `count` it
/// refuses, and names that attribute as written.
#[test]
fn a_repeat_counting_a_non_integer_stands_at_its_count() {
    let document = codec(
        r#"    <sce:field id="n" sce:type="bool" sce:byte="0" sce:bit-size="8"/>
    <sce:repeat id="msgs" type="codec_repeat_elem" sce:byte="1"
                count="n" max-count="8"/>"#,
    );
    let record = refusal(&document, "validation/attribute-rule-violated");
    assert_placed(&record, &document, "n", "count=\"n\"");
    assert!(
        record["message"]
            .as_str()
            .is_some_and(|m| m.contains("invalid count value")),
        "the attribute is named as the document writes it: {record}"
    );
}

/// A tail field followed by another stands at its own `sce:bit-size`.
#[test]
fn a_tail_that_is_not_last_stands_at_its_bit_size() {
    let document = codec(
        r#"    <sce:field id="head" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:field id="rest" sce:type="bytes" sce:byte="1"
               sce:bit-size="tail"/>
    <sce:field id="after" sce:type="uint8" sce:byte="2" sce:bit-size="8"/>"#,
    );
    let record = refusal(&document, "validation/attribute-rule-violated");
    assert_placed(&record, &document, "tail", "sce:bit-size=\"tail\"");
}
