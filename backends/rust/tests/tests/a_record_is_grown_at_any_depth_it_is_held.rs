// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0112, D5, and docs/adr/0044: a record a saved state holds is read at the
// defaults of the fields a later revision of its schema added, at every depth it is
// held at. `static_grow_1` holds records of the first revision of their schemas, in a
// record variable, an optional record and a record inside it, a list a record holds, a
// list variable, an optional variable and the case of a union; `static_grow_2` holds the same records at
// their second revision, which added a number with a default, a list, an optional
// record and an optional text. Its shape is the first version's, so it restores a state
// the first version saved with no migration, and each record reads its added fields at
// their defaults. The C++ half is tests/integration/ARecordIsGrownAtAnyDepthItIsHeldTest.cpp.
//
// Both machines are generated from sce-build/tests/fixtures/static_datamodel/ by the
// crate's build script (tree `rust-static-datamodel`).

use sce_rust_runtime::json::{self, Value};
use sce_rust_runtime::saved_state::{PersistentPolicy, SavedState};
use sce_rust_runtime::{Engine, SceClock};
use sce_rust_tests::integration::static_datamodel::static_grow_1_sm::StaticGrow1Policy;
use sce_rust_tests::integration::static_datamodel::static_grow_2_sm::StaticGrow2Policy;

/// The wall-clock moment both machines are saved and restored at.
const SAVED_AT_MS: u64 = 1_700_000_000_000;

/// A state of the first version holding a record at every depth, as a process boundary
/// carries it: written to JSON and read back.
fn saved_by_the_first_version() -> SavedState {
    let mut engine = Engine::new(StaticGrow1Policy::new());
    engine.set_clock(SceClock::Manual(0));
    let Ok(()) = engine.initialize();
    for (event, data) in [
        (
            "grow.holding",
            r#"{"outer":{"id":7,"inner":{"n":1},"inners":[{"n":2},{"n":3}]}}"#,
        ),
        ("grow.inner", r#"{"n":4}"#),
        ("grow.item", r#"{"n":8}"#),
        ("grow.maybe", r#"{"inner":{"n":5}}"#),
        ("grow.shaped", r#"{"shape":{"kind":"single","n":6}}"#),
    ] {
        engine.raise_external_by_name(event, data);
        engine.step();
    }
    let saved = StaticGrow1Policy::save_at(&engine, SAVED_AT_MS).expect("a running machine saves");
    SavedState::from_json(&saved.to_json()).expect("a saved state reads back")
}

/// The record `path` names in the state the next version saves after it restored
/// `saved`: a variable, then a member or an element at each step.
fn held<'s>(state: &'s SavedState, variable: &str, path: &[&str]) -> &'s Value {
    let mut value = state
        .variable(variable)
        .unwrap_or_else(|_| panic!("the state holds {variable}"));
    for step in path {
        value = match step.parse::<usize>() {
            Ok(index) => match value {
                Value::Array(items) => &items[index],
                other => panic!("{variable} at {step} is {other:?}"),
            },
            Err(_) => value
                .member(step)
                .unwrap_or_else(|| panic!("{variable} has no {step}: {value:?}")),
        };
    }
    value
}

/// Each place the first version held a record at, by the variable and the steps to it,
/// with the member it held and whether the second revision of its schema added a list
/// and a record (every one but the list variable's, whose element holds neither).
const HELD: [(&str, &[&str], &str, bool); 7] = [
    ("kept", &[], "n", true),
    ("outer", &[], "id", true),
    ("outer", &["inner"], "n", true),
    ("outer", &["inners", "1"], "n", true),
    ("many", &["0"], "n", false),
    ("maybe", &[], "n", true),
    ("shape", &[], "n", true),
];

#[test]
fn every_record_the_first_version_saved_is_read_at_the_defaults_its_second_revision_added() {
    let saved = saved_by_the_first_version();
    let restored = <StaticGrow2Policy as PersistentPolicy>::restore_with(
        StaticGrow2Policy::new(),
        &saved,
        SceClock::Manual(0),
        SAVED_AT_MS,
    )
    .unwrap_or_else(|refusal| panic!("the next version restores it: {}", refusal.reason()));
    let again = StaticGrow2Policy::save_at(&restored, SAVED_AT_MS).expect("saves");
    for (variable, path, kept, composed) in HELD {
        let record = held(&again, variable, path);
        assert!(
            record.member(kept).is_some(),
            "{variable} {path:?} keeps what it held: {record:?}"
        );
        assert_eq!(
            record.member("rank"),
            Some(&Value::Number("-2".to_string())),
            "{variable} {path:?}: a number takes its default: {record:?}"
        );
        if composed {
            assert_eq!(
                record.member("tags"),
                Some(&Value::Array(Vec::new())),
                "{variable} {path:?}: a list reads empty: {record:?}"
            );
            assert_eq!(
                record.member("leaf"),
                Some(&Value::Null),
                "{variable} {path:?}: a record a revision added reads absent: {record:?}"
            );
        }
        assert_eq!(
            record.member("memo"),
            Some(&Value::Null),
            "{variable} {path:?}: an optional text reads absent: {record:?}"
        );
    }
    assert_eq!(
        held(&again, "shape", &[]).member("kind"),
        Some(&Value::Text("single".to_string()))
    );
    // What the state now holds is the next version's: restored again, nothing grows twice.
    let text = again.to_json();
    <StaticGrow2Policy as PersistentPolicy>::restore_with(
        StaticGrow2Policy::new(),
        &SavedState::from_json(&text).expect("reads back"),
        SceClock::Manual(0),
        SAVED_AT_MS,
    )
    .unwrap_or_else(|refusal| {
        panic!(
            "a state the next version saved restores: {}",
            refusal.reason()
        )
    });
    let _ = json::parse(&text).expect("JSON");
}
