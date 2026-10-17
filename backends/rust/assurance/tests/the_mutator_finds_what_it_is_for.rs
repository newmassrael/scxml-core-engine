// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
//
// A fuzz that finds nothing says nothing unless it could have found something. Here parsers that have
// a defect planted in them are given to the harness, each a defect of the kind the mutator is meant to
// reach (a number that does not fit, a cut input, a value of another type, a key that is gone, a
// string of a length or a character nobody wrote), and the harness has to find every one.

mod common;

use std::panic::{catch_unwind, AssertUnwindSafe};

use common::fuzz;
use serde_json::{json, Value};

fn corpus() -> Vec<Vec<u8>> {
    vec![
        json!({"seq": "12", "name": "alice", "tags": ["a", "b"], "n": 3})
            .to_string()
            .into_bytes(),
        json!({"seq": "7", "name": "bob", "tags": [], "n": 1})
            .to_string()
            .into_bytes(),
    ]
}

fn finds(name: &str, target: impl FnMut(&[u8])) -> bool {
    catch_unwind(AssertUnwindSafe(|| fuzz(name, &corpus(), 4000, target))).is_err()
}

#[test]
fn the_harness_finds_a_defect_of_each_kind_it_is_made_for() {
    // A number that does not fit what it is read into.
    assert!(finds("overflow", |input| {
        if let Ok(value) = serde_json::from_slice::<Value>(input) {
            if let Some(text) = value["seq"].as_str() {
                let _ = text.parse::<u64>().unwrap_or(0).checked_add(1).unwrap();
            }
        }
    }));
    // An input that was cut off.
    assert!(finds("cut", |input| {
        let text = String::from_utf8_lossy(input);
        let _ = &text[text.len() - 1..];
        if serde_json::from_str::<Value>(&text).is_err() && !text.ends_with('}') && !text.is_empty()
        {
            panic!("a cut input");
        }
    }));
    // A value of another type than the one the parser expects.
    assert!(finds("type", |input| {
        if let Ok(value) = serde_json::from_slice::<Value>(input) {
            let _ = value["tags"].as_array().unwrap().len();
        }
    }));
    // A key that is not there.
    assert!(finds("missing", |input| {
        if let Ok(value) = serde_json::from_slice::<Value>(input) {
            let _ = value.get("name").unwrap();
        }
    }));
    // A string of a length nobody wrote.
    assert!(finds("long", |input| {
        if let Ok(value) = serde_json::from_slice::<Value>(input) {
            if let Some(name) = value["name"].as_str() {
                assert!(name.len() < 1000);
            }
        }
    }));
    // A character that is not ASCII, indexed as if it were.
    assert!(finds("unicode", |input| {
        if let Ok(value) = serde_json::from_slice::<Value>(input) {
            if let Some(name) = value["name"].as_str() {
                let _ = &name[..1];
                let _ = name.as_bytes()[0];
                assert!(name.is_ascii());
            }
        }
    }));
}

#[test]
fn a_parser_without_a_defect_passes_the_same_harness() {
    fuzz("sound", &corpus(), 4000, |input| {
        if let Ok(value) = serde_json::from_slice::<Value>(input) {
            let _ = value["seq"]
                .as_str()
                .and_then(|text| text.parse::<u64>().ok());
            let _ = value["tags"].as_array().map(Vec::len);
        }
    });
}
