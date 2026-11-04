// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.12 and docs/adr/0055, step 10 — a host-run `<invoke>`
// whose `sce:result` record holds values that may be absent. Rust compile+run
// gate.
//
// The committed machine under `src/integration/host_processor/` is generated
// from `sce-build/tests/fixtures/host_processor/statechart_host_invoker_optional.scxml`
// (regen: `scripts/regen_host_processor.sh`) and is constructed with NO script
// engine: a `datamodel="sce-static"` machine holds its variables as fields.
//
// The host answers through the generated typed interface, so the record it hands
// in holds each field that may be absent as an `Option`; the engine delivers the
// record as JSON, an absent value as `null` with its key written all the same;
// and the machine reads each field where a test proves it present. What these
// hold apart is the three states a field has — absent, present and zero or
// empty, and present — which a sentinel could not: `Some("")` is a label,
// `Some(false)` is a flag that was answered, and `Some(0)` is a day.

use std::sync::{Arc, Mutex};

use sce_rust_runtime::Engine;
use sce_rust_tests::integration::host_processor::{
    StatechartHostInvokerOptionalHostInvokers as HostInvokers,
    StatechartHostInvokerOptionalLookupResult as LookupResult,
    StatechartHostInvokerOptionalPolicy as Policy,
    StatechartHostInvokerOptionalXSceHostInvoker as XSceHostInvoker,
};

/// The type the fixture was compiled for; `scripts/regen_host_processor.sh`
/// passes the same string to the invoker declaration.
const DECLARED_TYPE: &str = "x-sce-host";

/// A host implementing the generated interface; its work outlives the call, so
/// each test completes the start it was handed. Records each start's token.
struct LookupHost(Arc<Mutex<Vec<u64>>>);

impl XSceHostInvoker for LookupHost {
    fn start_lookup(&mut self, token: u64) -> Option<LookupResult> {
        self.0.lock().expect("lookup starts").push(token);
        None
    }

    fn cancel_lookup(&mut self, _token: u64) {}
}

/// A machine standing at `asking`, with the token the host was started with.
fn asking() -> (Engine<Policy>, u64) {
    let starts: Arc<Mutex<Vec<u64>>> = Arc::default();
    let mut engine = Engine::new(Policy::new());
    engine.register_x_sce_host_invoker(LookupHost(Arc::clone(&starts)));
    engine.initialize();
    engine.raise_external_by_name("ask", "");
    engine.step();
    let token = {
        let tokens = starts.lock().expect("lookup starts");
        assert_eq!(tokens.len(), 1, "the lookup was started once");
        tokens[0]
    };
    (engine, token)
}

/// What the machine published after a completion: `answered` counts the
/// completions it read, and the rest is what each field of the record held.
#[derive(Debug, Default, PartialEq)]
struct Answer {
    answered: u8,
    rows_seen: u8,
    total_seen: u64,
    at_present: bool,
    at_value: i64,
    label_present: bool,
    label_value: String,
    hit_known: bool,
    hit_true: bool,
}

fn read(engine: &Engine<Policy>) -> Answer {
    let policy = engine.policy();
    Answer {
        answered: policy.answered(),
        rows_seen: policy.rows_seen(),
        total_seen: policy.total_seen(),
        at_present: policy.at_present(),
        at_value: policy.at_value(),
        label_present: policy.label_present(),
        label_value: policy.label_value().to_string(),
        hit_known: policy.hit_known(),
        hit_true: policy.hit_true(),
    }
}

/// Ask, complete the lookup with `result` through the typed interface, and settle
/// the machine.
fn complete(result: LookupResult) -> Answer {
    let (mut engine, token) = asking();
    assert!(
        engine.complete_lookup(token, result),
        "a running invocation's typed completion was refused"
    );
    engine.step();
    read(&engine)
}

/// Every field that may be absent is: the machine reads three absent values and
/// the count every answer carries.
#[test]
fn an_absent_field_is_read_as_absent() {
    let got = complete(LookupResult {
        rows: 3,
        total: 5,
        at: None,
        label: None,
        hit: None,
    });
    assert_eq!(
        got,
        Answer {
            answered: 1,
            rows_seen: 3,
            total_seen: 5,
            ..Answer::default()
        },
        "an all-absent answer"
    );
}

/// Every field is present: the machine reads each value.
#[test]
fn a_present_field_is_read_as_its_value() {
    let got = complete(LookupResult {
        rows: 4,
        total: 40,
        at: Some(41),
        label: Some("ab".to_string()),
        hit: Some(true),
    });
    assert_eq!(
        got,
        Answer {
            answered: 1,
            rows_seen: 4,
            total_seen: 40,
            at_present: true,
            at_value: 41,
            label_present: true,
            label_value: "ab".to_string(),
            hit_known: true,
            hit_true: true,
        },
        "an all-present answer"
    );
}

/// A zero, an empty text and a false flag are values, not absences: each is
/// present, and is read as itself.
#[test]
fn a_zero_an_empty_text_and_a_false_flag_are_values_and_not_absences() {
    let got = complete(LookupResult {
        rows: 1,
        total: 0,
        at: Some(0),
        label: Some(String::new()),
        hit: Some(false),
    });
    assert_eq!(
        got,
        Answer {
            answered: 1,
            rows_seen: 1,
            at_present: true,
            label_present: true,
            hit_known: true,
            ..Answer::default()
        },
        "zero, empty and false are present"
    );
}

/// Each field is independent: a day with no label and a flag.
#[test]
fn each_field_is_absent_or_present_on_its_own() {
    let got = complete(LookupResult {
        rows: 2,
        total: 9,
        at: Some(7),
        label: None,
        hit: Some(true),
    });
    assert_eq!(
        got,
        Answer {
            answered: 1,
            rows_seen: 2,
            total_seen: 9,
            at_present: true,
            at_value: 7,
            hit_known: true,
            hit_true: true,
            ..Answer::default()
        },
        "a mixed answer"
    );
}

/// The record's JSON writes the key of every field, `null` where the value is
/// absent, in the order the schema declares them. A 64-bit integer is written as
/// the text of its digits (`total` is one whether or not a value is absent), so
/// the day being absent in both leaves what is held here the key, the `null` and
/// the other values' own spelling.
#[test]
fn the_record_is_written_with_null_for_an_absent_value() {
    let absent = LookupResult {
        rows: 3,
        total: 5,
        at: None,
        label: None,
        hit: None,
    };
    assert_eq!(
        absent.wire().as_str(),
        r#"{"rows":3, "total":"5", "at":null, "label":null, "hit":null}"#
    );
    let mixed = LookupResult {
        rows: 3,
        total: 5,
        at: None,
        label: Some("ab".to_string()),
        hit: Some(false),
    };
    assert_eq!(
        mixed.wire().as_str(),
        r#"{"rows":3, "total":"5", "at":null, "label":"ab", "hit":false}"#
    );
}

/// A 64-bit integer crosses as the text of its digits, so the record the host
/// completes with is read exactly past 2^53 - 1, where a JSON number held in a
/// double is not: the first integer a double cannot hold, and the greatest value
/// of each width.
#[test]
fn a_64_bit_field_crosses_as_the_text_of_its_digits() {
    let wide = LookupResult {
        rows: 1,
        total: u64::MAX,
        at: Some(9_007_199_254_740_993),
        label: None,
        hit: None,
    };
    assert_eq!(
        wide.wire().as_str(),
        r#"{"rows":1, "total":"18446744073709551615", "at":"9007199254740993", "label":null, "hit":null}"#
    );
    for (name, total, at) in [
        (
            "the first integer a double cannot hold",
            9_007_199_254_740_993_u64,
            9_007_199_254_740_993_i64,
        ),
        ("the greatest of each width", u64::MAX, i64::MAX),
        ("the least signed value", 0, i64::MIN),
        (
            "the greatest integer a double holds exactly",
            9_007_199_254_740_991,
            -9_007_199_254_740_991,
        ),
    ] {
        let got = complete(LookupResult {
            rows: 1,
            total,
            at: Some(at),
            label: None,
            hit: None,
        });
        assert_eq!(
            got,
            Answer {
                answered: 1,
                rows_seen: 1,
                total_seen: total,
                at_present: true,
                at_value: at,
                ..Answer::default()
            },
            "{name}"
        );
    }
}

/// A key left out is not an absent value: every field of the first revision of a
/// schema is carried by every delivery, and `null` is the one spelling of absence,
/// so a completion that lacks `at` is refused as any typed payload the data does
/// not fit is, and the machine reads no answer.
#[test]
fn a_completion_that_leaves_a_key_out_is_refused() {
    let (mut engine, token) = asking();
    assert!(
        engine.complete_host_invoke(
            DECLARED_TYPE,
            "lookup",
            token,
            r#"{"rows":1,"total":"0","label":null,"hit":null}"#
        ),
        "a running invocation's completion was refused"
    );
    engine.step();
    assert_eq!(
        read(&engine).answered,
        0,
        "a record without `at` was read as an answer"
    );
}
