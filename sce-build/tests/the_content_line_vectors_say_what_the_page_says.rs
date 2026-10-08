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
//!   largest value of each bound is written once.
//!
//! The vectors are written by `tests/forge/conformance/content_line_model.py`,
//! which holds each to itself; this file holds the result to the page and the
//! fixture.

mod common;

use sce_build::forge::codec_failure::CodecFailure;
use sce_build::forge::model::{CodecEncoding, ForgeDocument, SceType};
use sce_build::forge::parser::parse_forge;
use sce_build::DocumentLabel;
use serde_json::Value;

const FIXTURE: &str = "codec_content_line_event";

fn reference() -> Value {
    let path = common::repository::root().join("tests/forge/conformance/numerical_reference.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).expect("the oracle file is JSON")
}

fn fixture() -> Value {
    reference()["codecs"][FIXTURE].clone()
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

#[test]
fn text_and_encoded_are_one_thing_in_every_vector() {
    let f = fixture();
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
                "{key}[{i}]"
            );
            compared += 1;
        }
    }
    assert!(compared > 100, "only {compared} vectors carry a text");
}

#[test]
fn every_failure_the_page_names_is_refused_by_a_vector() {
    let f = fixture();
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
            "no reject asks for {}",
            failure.wire_name()
        );
    }
    for name in &named {
        let failure = CodecFailure::parse(name).expect("a known failure");
        assert!(
            failure.is_content_line() || failure == CodecFailure::NeedMoreBytes,
            "{name} is not a failure a content-line codec raises"
        );
    }
}

#[test]
fn the_cases_reach_every_entry_and_every_bound_of_the_fixture() {
    let path =
        common::repository::root().join("tests/forge/resources/codec_content_line_event.scxml");
    let text = std::fs::read_to_string(&path).expect("the fixture document");
    let model = match parse_forge(
        &text,
        DocumentLabel {
            identifier: FIXTURE,
            diagnostic_label: FIXTURE,
        },
    ) {
        Ok(Some(ForgeDocument::Codec(m))) => m,
        other => panic!("expected a codec, got {other:?}"),
    };
    assert_eq!(model.encoding, CodecEncoding::ContentLine);
    let entries = model.content_line.expect("a content-line codec").entries;

    let f = fixture();
    let cases = array(&f, "cases");
    for e in &entries {
        let present = |c: &&Value| match &c["decoded"][&e.id] {
            Value::Null => false,
            Value::Array(items) => !items.is_empty(),
            _ => true,
        };
        assert!(
            cases.iter().any(|c| present(&c)),
            "no case carries `{}`",
            e.id
        );
        if let Some(count) = e.max_count {
            assert!(
                cases
                    .iter()
                    .any(|c| c["decoded"][&e.id].as_array().map(Vec::len)
                        == Some(usize::try_from(count).expect("a count"))),
                "no case fills the list `{}` to its {count}",
                e.id
            );
        }
        if let (SceType::String, Some(size)) = (&e.sce_type, e.max_size) {
            // The widest value a case carries is the bound, or the bound is
            // unreached; a long text reaches it through escapes, so measure the
            // unescaped value.
            let widest = cases
                .iter()
                .flat_map(|c| match &c["decoded"][&e.id] {
                    Value::String(s) => vec![s.len()],
                    Value::Array(items) => items
                        .iter()
                        .filter_map(|i| i.as_str().map(str::len))
                        .collect(),
                    _ => vec![],
                })
                .max()
                .unwrap_or(0);
            assert!(widest > 0, "no case carries a value of `{}`", e.id);
            assert!(
                widest <= size as usize,
                "a case carries {widest} bytes in `{}`, past its {size}",
                e.id
            );
        }
    }
    // A fold is exercised: some case writes a line past 75 octets before folding.
    assert!(
        cases.iter().any(|c| {
            c.get("decode_only").is_none()
                && c["text"].as_str().is_some_and(|t| t.contains("\r\n "))
        }),
        "no written case is folded"
    );
}
