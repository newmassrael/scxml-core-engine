// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The specs folder reads a ledger the way the authoring tools do (ADR 0040).
//!
//! `tools/authoring/tests/fixtures/ledger_wire_cases.json` is a list of ledgers, each good or bad, that the tools'
//! tests read as well (`test_the_scope_and_ledger_schemas_say_what_the_code_enforces.py`). A ledger one side
//! refuses and the other keeps is one the owner's screen can save and no tool can count, so each side is held to
//! the same cases, and a rule added to one and not the other fails here or there.

use sce_specs::ledger::{Destination, Ledger};
use sce_specs::{Revision, StoreError};
use serde_json::Value;

const CASES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../tools/authoring/tests/fixtures/ledger_wire_cases.json"
);

fn cases() -> Value {
    let text = std::fs::read_to_string(CASES).unwrap_or_else(|e| {
        panic!("{CASES} cannot be read ({e}): the specs folder and the tools read one file")
    });
    serde_json::from_str(&text).expect("the cases are JSON")
}

/// A case with the words it stands for put in, as the tools' test puts them.
fn filled(value: &Value, revision: &str) -> Value {
    match value {
        Value::String(text) => Value::String(match text.as_str() {
            "$EXTRACTION" => revision.to_string(),
            "$SPEC80" => "a".repeat(80),
            "$SPEC81" => "a".repeat(81),
            "$TITLE200" => "t".repeat(200),
            "$TITLE201" => "t".repeat(201),
            "$REASON1000" => "r".repeat(1000),
            "$REASON1001" => "r".repeat(1001),
            other => other.to_string(),
        }),
        Value::Array(items) => Value::Array(items.iter().map(|v| filled(v, revision)).collect()),
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(k, v)| (k.clone(), filled(v, revision)))
                .collect(),
        ),
        other => other.clone(),
    }
}

struct Held {
    revision: Revision,
    order: Vec<String>,
    cases: Value,
}

fn held() -> Held {
    let cases = cases();
    let order: Vec<String> = cases["blocks"]
        .as_array()
        .expect("the cases name their blocks")
        .iter()
        .map(|b| b.as_str().unwrap().to_string())
        .collect();
    Held {
        revision: Revision::of(b"the extraction the cases are of"),
        order,
        cases,
    }
}

fn read(held: &Held, wire: &Value) -> Result<Ledger, StoreError> {
    Ledger::read(&wire.to_string(), &held.revision, &held.order)
}

#[test]
fn every_ledger_the_tools_accept_is_read() {
    let held = held();
    let good = held.cases["good"].as_array().unwrap();
    assert!(good.len() >= 5);

    for case in good {
        let wire = filled(&case["wire"], held.revision.as_str());

        let read = read(&held, &wire);

        assert!(read.is_ok(), "{}: {read:?}", case["name"]);
    }
}

#[test]
fn every_ledger_the_tools_refuse_is_refused_as_a_bad_ledger() {
    let held = held();
    let bad = held.cases["bad"].as_array().unwrap();
    let layers: std::collections::BTreeSet<&str> =
        bad.iter().map(|c| c["layer"].as_str().unwrap()).collect();
    assert_eq!(
        layers.into_iter().collect::<Vec<_>>(),
        ["extraction", "form", "list"]
    );

    for case in bad {
        let wire = filled(&case["wire"], held.revision.as_str());

        let refused = read(&held, &wire);

        match refused {
            Err(StoreError::Refused { kind, .. }) => {
                assert_eq!(kind, "bad-ledger", "{}", case["name"])
            }
            other => panic!("{}: expected a refusal, got {other:?}", case["name"]),
        }
    }
}

#[test]
fn what_is_read_is_what_the_ledger_says() {
    let held = held();
    let wire = filled(&held.cases["good"][1]["wire"], held.revision.as_str());

    let ledger = read(&held, &wire).unwrap();

    assert_eq!(ledger.allotments.len(), 3);
    assert_eq!(
        ledger.allotments[0].to,
        Destination::Spec("lamp-1".to_string())
    );
    assert_eq!(ledger.allotments[1].to, Destination::New("Fan".to_string()));
    assert_eq!(ledger.allotments[2].to, Destination::LeftOut);
    assert_eq!(ledger.allotments[2].reason, "the cover");
    assert_eq!(ledger.allotments[2].blocks, ["b5"]);
    // A spec is found by its id, and a spec to be made by the id it was made as.
    let bound = vec![("Fan".to_string(), "fan-1".to_string())];
    assert_eq!(
        ledger.allotment_of("lamp-1", &bound).unwrap().blocks,
        ["b1", "b2"]
    );
    assert_eq!(
        ledger.allotment_of("fan-1", &bound).unwrap().blocks,
        ["b3", "b4"]
    );
    assert!(ledger.allotment_of("fan-1", &[]).is_none());
    assert!(ledger.allotment_of("other-1", &bound).is_none());
}
