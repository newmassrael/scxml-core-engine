// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! An element that takes exactly one of several attributes is refused for
//! what the document writes, not for a sentence about it.
//!
//! # What was wrong
//!
//! Three rules of this shape — `<sce:repeat>` takes `count` or
//! `until-eof`, `<sce:decoded>` takes `value`, `hex` or `string`,
//! `<sce:capacity>` takes `source="deploy" key="…"` or `const` — were
//! refused as `validation/attribute-rule-violated`, which needs a value,
//! and each invented one: `<both or neither>`, `2 of value/hex/string
//! attributes set`, the empty string. None is text a row of the document
//! holds, so SCE_ERROR_CONTRACT §3.1.1's consumer searched for it in vain.
//!
//! # What is held
//!
//! Each rule is now `validation/exactly-one-attribute`. The attributes the
//! element takes are `expected`. With one too many, the attribute written
//! beyond the first is `actual`, and the record stands at it — here always
//! on a row of its own, so the row proves the placement. With none, nothing
//! is reported and the record stands at the element. The expected rows and
//! columns are read off the fixture text, not restated.

use std::path::Path;
use std::process::Command;

const CODE: &str = "validation/exactly-one-attribute";

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

/// Every record `check` writes for `document`, with the imported siblings
/// the fixtures name beside it, and the run's exit status.
fn check(document: &str) -> (Option<i32>, Vec<serde_json::Value>) {
    let dir = tempfile::tempdir().expect("tempdir");
    let resources = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/resources");
    for sibling in ["codec_repeat_elem.scxml", "subscription_entry.scxml"] {
        std::fs::copy(resources.join(sibling), dir.path().join(sibling)).expect("copy sibling");
    }
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
    let records = String::from_utf8_lossy(&run.stderr)
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('{'))
        .map(|line| serde_json::from_str(line).expect("one JSON record per line"))
        .collect();
    (run.status.code(), records)
}

/// The one record of `code` `check` refuses `document` with.
fn refusal(document: &str, code: &str) -> serde_json::Value {
    let (status, records) = check(document);
    assert_ne!(status, Some(0), "the document built:\n{document}");
    let mut of_code = records.iter().filter(|r| r["code"] == code);
    match (of_code.next(), of_code.next()) {
        (Some(one), None) => one.clone(),
        _ => panic!("expected one {code} record, got {records:?}"),
    }
}

/// `record` names `alternatives`, reports `extra` as written, and stands
/// where `needle` starts in `document`.
fn assert_refused(
    record: &serde_json::Value,
    document: &str,
    alternatives: &[&str],
    extra: Option<&str>,
    needle: &str,
) {
    assert_eq!(
        record["expected"],
        serde_json::json!(alternatives),
        "{record}"
    );
    assert_eq!(
        record["actual"].as_str(),
        extra,
        "the record reports text the row does not hold: {record}"
    );
    let (row, col) = written_at(document, needle);
    assert_eq!(
        (
            record["location"]["line"].as_u64(),
            record["location"]["col"].as_u64()
        ),
        (Some(row), Some(col)),
        "{record}"
    );
    assert!(
        record.get("fix").is_none(),
        "no edit is the producer's: {record}"
    );
}

fn repeat(attributes: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="doc">
  <sce:import src="codec_repeat_elem.scxml" kind="codec" as="codec_repeat_elem"/>
  <datamodel>
    <sce:field id="n" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:repeat id="msgs" type="codec_repeat_elem" sce:byte="1"{attributes}
                max-count="64"/>
  </datamodel>
</scxml>
"#
    )
}

#[test]
fn a_repeat_with_both_counts_names_the_second() {
    let document = repeat("\n                count=\"n\"\n                until-eof=\"true\"");
    let record = refusal(&document, CODE);
    assert_refused(
        &record,
        &document,
        &["count", "until-eof"],
        Some("until-eof"),
        "until-eof=",
    );
}

#[test]
fn a_repeat_with_neither_count_reports_nothing_at_the_element() {
    let document = repeat("");
    let record = refusal(&document, CODE);
    assert_refused(
        &record,
        &document,
        &["count", "until-eof"],
        None,
        "<sce:repeat",
    );
}

/// `until-eof="false"` is written and asserts nothing, so it stands beside
/// `count` as it always has.
#[test]
fn until_eof_false_beside_count_is_no_second_count() {
    let (_, records) = check(&repeat(
        "\n                count=\"n\"\n                until-eof=\"false\"",
    ));
    assert!(
        records.iter().all(|r| r["code"] != CODE),
        "until-eof=\"false\" was read as a second count: {records:?}"
    );
}

fn decoded(attributes: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="doc">
  <datamodel>
    <sce:field id="reason" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
  </datamodel>
  <sce:test-vector hex="00">
    <sce:decoded field="reason"{attributes}/>
  </sce:test-vector>
</scxml>
"#
    )
}

#[test]
fn a_decoded_row_with_two_values_names_the_second() {
    let document = decoded("\n                 value=\"0\"\n                 hex=\"00\"");
    let record = refusal(&document, CODE);
    assert_refused(
        &record,
        &document,
        &["value", "hex", "string"],
        Some("hex"),
        "hex=\"00\"/>",
    );
}

#[test]
fn a_decoded_row_with_no_value_reports_nothing_at_the_element() {
    let document = decoded("");
    let record = refusal(&document, CODE);
    assert_refused(
        &record,
        &document,
        &["value", "hex", "string"],
        None,
        "<sce:decoded",
    );
}

fn capacity(attributes: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="bounded-collection"
       name="doc"
       version="1.0">
  <sce:element-type>subscription_entry</sce:element-type>
  <sce:capacity{attributes}/>
</scxml>
"#
    )
}

/// The deploy form is two attributes; the form written second is the one
/// too many, named by its first attribute.
#[test]
fn a_capacity_in_both_forms_names_the_second_form() {
    let document =
        capacity("\n      source=\"deploy\" key=\"machines.m.limits.subs\"\n      const=\"8\"");
    let record = refusal(&document, CODE);
    assert_refused(
        &record,
        &document,
        &["source", "const"],
        Some("const"),
        "const=\"8\"",
    );
}

#[test]
fn a_capacity_in_neither_form_reports_nothing_at_the_element() {
    let document = capacity("");
    let record = refusal(&document, CODE);
    assert_refused(
        &record,
        &document,
        &["source", "const"],
        None,
        "<sce:capacity",
    );
}

/// An incomplete deploy form, or one naming another source, is refused for
/// what it lacks or holds — its own codes, not a choice of form.
#[test]
fn an_incomplete_deploy_form_is_refused_for_what_it_lacks() {
    let lacks_key = refusal(
        &capacity(" source=\"deploy\""),
        "validation/missing-attribute",
    );
    assert_eq!(lacks_key["fix"]["attr"], "key", "{lacks_key}");
    let lacks_source = refusal(
        &capacity(" key=\"machines.m.limits.subs\""),
        "validation/missing-attribute",
    );
    assert_eq!(lacks_source["fix"]["attr"], "source", "{lacks_source}");
    let other_source = refusal(
        &capacity(" source=\"env\" key=\"machines.m.limits.subs\""),
        "validation/invalid-attribute",
    );
    assert_eq!(other_source["actual"], "env", "{other_source}");
}
