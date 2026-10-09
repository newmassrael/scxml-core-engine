// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The content-line codec's vectors in `numerical_reference.json` are what six
//! backends are held to, so the file itself is held to three things a backend's
//! run cannot say.
//!
//! * **`text` and `encoded` are one thing.** A reader checks a vector against
//!   `SCE_FORGE.md` §4.6.4 by its `text`; a backend is compared by its `encoded`
//!   bytes. A pair that drifted would be a vector one of them cannot see.
//! * **Every failure the page names is refused by a vector.** A reject that no
//!   vector asks for is a rule no backend is held to.
//! * **Every entry of the fixture is reached.** A case that never carries an
//!   entry proves nothing about it; the largest list is filled once, and the
//!   largest value of each bound is written once. A line record is reached
//!   through its list: its parameters are read out of the records (docs/adr/0014).
//!
//! There are two fixtures, the event one (a value, or a list of values, per
//! property) and the records one (a line is a record, a line holds a list). The
//! vectors are written by `tests/forge/conformance/content_line_model.py`, which
//! holds each to itself; this file holds the result to the page and the fixture.

mod common;

use sce_build::forge::codec_failure::CodecFailure;
use sce_build::forge::model::{CodecEncoding, ContentLineEntry, ForgeDocument, SceType};
use sce_build::forge::parser::parse_forge;
use sce_build::DocumentLabel;
use serde_json::Value;

const FIXTURES: [&str; 3] = [
    "codec_content_line_event",
    "codec_content_line_records",
    "codec_content_line_required_lists",
];

fn reference() -> Value {
    let path = common::repository::root().join("tests/forge/conformance/numerical_reference.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).expect("the oracle file is JSON")
}

fn fixture(name: &str) -> Value {
    reference()["codecs"][name].clone()
}

fn bytes_of(v: &Value) -> Vec<u8> {
    v.as_array()
        .expect("encoded is an array")
        .iter()
        .map(|b| u8::try_from(b.as_u64().expect("a byte")).expect("a byte"))
        .collect()
}

fn array<'a>(f: &'a Value, key: &str) -> &'a Vec<Value> {
    f[key]
        .as_array()
        .unwrap_or_else(|| panic!("{key} is an array"))
}

fn entries_of(name: &str) -> Vec<ContentLineEntry> {
    let path = common::repository::root().join(format!("tests/forge/resources/{name}.scxml"));
    let text = std::fs::read_to_string(&path).expect("the fixture document");
    let model = match parse_forge(
        &text,
        DocumentLabel {
            identifier: name,
            diagnostic_label: name,
        },
    ) {
        Ok(Some(ForgeDocument::Codec(m))) => m,
        other => panic!("expected a codec, got {other:?}"),
    };
    assert_eq!(model.encoding, CodecEncoding::ContentLine);
    model.content_line.expect("a content-line codec").entries
}

/// The property entry a parameter entry belongs to.
fn owner_of<'a>(entries: &'a [ContentLineEntry], e: &ContentLineEntry) -> &'a ContentLineEntry {
    entries
        .iter()
        .find(|o| o.param.is_none() && o.property.eq_ignore_ascii_case(&e.property))
        .expect("a parameter has its property's entry")
}

/// Whether the lines of `owner` are read as records: a repeated property that
/// declares a parameter, or whose line holds a list of values.
fn reads_records(entries: &[ContentLineEntry], owner: &ContentLineEntry) -> bool {
    owner.max_count.is_some()
        && (owner.separator.is_some()
            || entries
                .iter()
                .any(|p| p.param.is_some() && p.property.eq_ignore_ascii_case(&owner.property)))
}

/// Every value `entry` takes in one case's `decoded` object, whatever the list or
/// record it sits in: the strings, numbers and booleans a bound is measured on.
fn occurrences<'a>(
    entries: &[ContentLineEntry],
    entry: &ContentLineEntry,
    decoded: &'a Value,
) -> Vec<&'a Value> {
    let flatten = |v: &'a Value, out: &mut Vec<&'a Value>| match v {
        Value::Null => {}
        Value::Array(items) => out.extend(items.iter()),
        other => out.push(other),
    };
    let mut out = Vec::new();
    let owner = owner_of_or_self(entries, entry);
    if entry.param.is_some() && reads_records(entries, owner) {
        for record in decoded[&owner.id].as_array().into_iter().flatten() {
            flatten(&record[&entry.id], &mut out);
        }
    } else if entry.param.is_none() && reads_records(entries, entry) {
        let member = if entry.separator.is_some() {
            "values"
        } else {
            "value"
        };
        for record in decoded[&entry.id].as_array().into_iter().flatten() {
            flatten(&record[member], &mut out);
        }
    } else {
        flatten(&decoded[&entry.id], &mut out);
    }
    out
}

fn owner_of_or_self<'a>(
    entries: &'a [ContentLineEntry],
    e: &'a ContentLineEntry,
) -> &'a ContentLineEntry {
    if e.param.is_some() {
        owner_of(entries, e)
    } else {
        e
    }
}

#[test]
fn text_and_encoded_are_one_thing_in_every_vector() {
    for name in FIXTURES {
        let f = fixture(name);
        let mut compared = 0;
        for key in ["cases", "rejects"] {
            for (i, v) in array(&f, key).iter().enumerate() {
                let Some(text) = v.get("text").and_then(Value::as_str) else {
                    // A vector whose bytes are not UTF-8 has no readable text.
                    continue;
                };
                assert_eq!(
                    text.as_bytes(),
                    bytes_of(&v["encoded"]).as_slice(),
                    "{name} {key}[{i}]"
                );
                compared += 1;
            }
        }
        assert!(
            compared > 50,
            "{name}: only {compared} vectors carry a text"
        );
    }
}

#[test]
fn every_failure_the_page_names_is_refused_by_a_vector() {
    for name in FIXTURES {
        let f = fixture(name);
        let named: Vec<&str> = array(&f, "rejects")
            .iter()
            .map(|r| r["error"].as_str().expect("a reject names its failure"))
            .collect();
        let wanted = CodecFailure::ALL
            .into_iter()
            .filter(|f| f.is_content_line() || *f == CodecFailure::NeedMoreBytes);
        for failure in wanted {
            assert!(
                named.contains(&failure.wire_name()),
                "{name}: no reject asks for {}",
                failure.wire_name()
            );
        }
        for failure_name in &named {
            let failure = CodecFailure::parse(failure_name).expect("a known failure");
            assert!(
                failure.is_content_line() || failure == CodecFailure::NeedMoreBytes,
                "{name}: {failure_name} is not a failure a content-line codec raises"
            );
        }
    }
}

#[test]
fn the_cases_reach_every_entry_and_every_bound_of_the_fixture() {
    for name in FIXTURES {
        let entries = entries_of(name);
        let f = fixture(name);
        let cases = array(&f, "cases");
        for e in &entries {
            assert!(
                cases
                    .iter()
                    .any(|c| !occurrences(&entries, e, &c["decoded"]).is_empty()),
                "{name}: no case carries `{}`",
                e.id
            );
            if let Some(count) = e.max_count {
                let count = usize::try_from(count).expect("a count");
                assert!(
                    cases
                        .iter()
                        .any(|c| c["decoded"][&e.id].as_array().map(Vec::len) == Some(count)),
                    "{name}: no case fills the list `{}` to its {count}",
                    e.id
                );
            }
            if let Some(values) = e.max_values {
                // Some line holds as many values as it may: the list of a
                // single-line property, or the values of one record.
                let values = usize::try_from(values).expect("a count");
                let full = |item: &Value| match item {
                    Value::Array(items) => items.len() == values,
                    Value::Object(record) => record
                        .get("values")
                        .and_then(Value::as_array)
                        .is_some_and(|v| v.len() == values),
                    _ => false,
                };
                let single_line_list = e.max_count.is_none();
                assert!(
                    cases.iter().any(|c| {
                        let v = &c["decoded"][&e.id];
                        if single_line_list {
                            full(v)
                        } else {
                            v.as_array().is_some_and(|items| items.iter().any(full))
                        }
                    }),
                    "{name}: no case fills a line of `{}` to its {values} values",
                    e.id
                );
            }
            if let (SceType::String, Some(size)) = (&e.sce_type, e.max_size) {
                // The widest value a case carries is the bound, or the bound is
                // unreached; a long text reaches it through escapes, so measure
                // the unescaped value.
                let widest = cases
                    .iter()
                    .flat_map(|c| occurrences(&entries, e, &c["decoded"]))
                    .filter_map(|v| v.as_str().map(str::len))
                    .max()
                    .unwrap_or(0);
                assert!(widest > 0, "{name}: no case carries a value of `{}`", e.id);
                assert!(
                    widest <= size as usize,
                    "{name}: a case carries {widest} bytes in `{}`, past its {size}",
                    e.id
                );
            }
        }
        // A fold is exercised: some case writes a line past 75 octets before
        // folding. The required-lists fixture keeps every bound under eight
        // octets so that its rejects are a few bytes long, and no line of it
        // reaches 75; the other two fixtures hold the fold for all three.
        if name != "codec_content_line_required_lists" {
            assert!(
                cases.iter().any(|c| {
                    c.get("decode_only").is_none()
                        && c["text"].as_str().is_some_and(|t| t.contains("\r\n "))
                }),
                "{name}: no written case is folded"
            );
        }
    }
}

#[test]
fn a_line_record_is_told_from_a_list_of_values_by_what_its_entry_declares() {
    // The records fixture is the one that reads lines as records; the event
    // fixture's repeated property declares no parameter and no separator, so its
    // lines stay a list of values (docs/adr/0014, decision 1).
    let event = entries_of(FIXTURES[0]);
    assert!(event.iter().all(|e| !reads_records(&event, e)));
    let records = entries_of(FIXTURES[1]);
    let named: Vec<&str> = records
        .iter()
        .filter(|e| e.param.is_none() && reads_records(&records, e))
        .map(|e| e.id.as_str())
        .collect();
    assert_eq!(named, ["attendee", "exdate"]);
}
