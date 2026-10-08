// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-C-2: the `<param>`s of a BasicHTTP `<send>` of a `datamodel="sce-static"`
// machine are read from its own fields when the send runs, and cross as the text
// a form carries (docs/adr/0005, decision 4) — Rust path. The request is observed
// where the engine hands it to its transport, so no listener is involved.
//
// Fixture: sce-build/tests/fixtures/static_datamodel/static_send_http.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_static_datamodel_rust.sh

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use sce_rust_runtime::{Engine, HttpSendRequest};
use sce_rust_tests::integration::static_datamodel::static_send_http_sm::{
    StaticSendHttpPersist, StaticSendHttpPolicy,
};
use serde_json::Value;

/// The requests the transport was handed, in the order it was handed them.
type Posted = Arc<Mutex<Vec<HttpSendRequest>>>;

/// A machine whose transport keeps what it was handed.
fn machine() -> (Engine<StaticSendHttpPolicy>, Posted) {
    let mut engine = Engine::new(StaticSendHttpPolicy::new());
    let posted: Posted = Arc::new(Mutex::new(Vec::new()));
    let recorder = Arc::clone(&posted);
    engine.set_http_send_callback(move |request| {
        recorder.lock().unwrap().push(request);
        None
    });
    engine.initialize();
    (engine, posted)
}

fn raise(engine: &mut Engine<StaticSendHttpPolicy>, name: &str) {
    engine.raise_external_by_name(name, "");
    engine.step();
}

/// How many `error.execution` the machine has counted, read back as its saved
/// state holds it.
fn errors(engine: &Engine<StaticSendHttpPolicy>) -> Value {
    let saved: Value =
        serde_json::from_str(&engine.save().expect("saves").to_json()).expect("a saved state");
    saved["variables"]["errors"].clone()
}

fn pairs(list: &[(&str, &str)]) -> HashMap<String, Vec<String>> {
    list.iter()
        .map(|(name, text)| (name.to_string(), vec![text.to_string()]))
        .collect()
}

#[test]
fn a_send_over_http_carries_the_text_the_fields_hold_when_it_runs() {
    let (mut engine, posted) = machine();
    raise(&mut engine, "bump");
    raise(&mut engine, "go");

    let posted = posted.lock().unwrap();
    assert_eq!(posted.len(), 1, "one request is handed to the transport");
    let request = &posted[0];
    assert_eq!(request.target, "http://example.invalid/hook");
    assert_eq!(request.event_name, "note");
    assert_eq!(
        request.content, "",
        "no `<content>`, so the body is the pairs"
    );
    assert_eq!(
        request.params,
        pairs(&[
            ("count", "4"),
            ("ready", "true"),
            ("label", "busy"),
            ("twice", "8"),
            ("delta", "-5"),
            ("ratio", "1.5"),
        ]),
        "each value is the text it spells: an integer's digits, `true`, the string, \
         a negative number, a real's `String()`"
    );
}

#[test]
fn the_pairs_are_the_fields_as_they_stand_and_not_a_copy_from_start_up() {
    let (mut engine, posted) = machine();
    raise(&mut engine, "go");

    let posted = posted.lock().unwrap();
    assert_eq!(posted.len(), 1);
    assert_eq!(
        posted[0].params,
        pairs(&[
            ("count", "3"),
            ("ready", "false"),
            ("label", "idle"),
            ("twice", "6"),
            ("delta", "-5"),
            ("ratio", "1.5"),
        ]),
        "without `bump` the fields hold their initial values, and the request \
         carries those: it is read when the send runs"
    );
}

#[test]
fn a_param_that_cannot_be_read_is_left_out_and_the_request_still_goes() {
    let (mut engine, posted) = machine();
    raise(&mut engine, "bump");
    raise(&mut engine, "boom");

    {
        let posted = posted.lock().unwrap();
        assert_eq!(
            posted.len(),
            1,
            "the request goes with the pair that could be read"
        );
        assert_eq!(
            posted[0].params,
            pairs(&[("count", "4")]),
            "`big` is `count * 2000000000`, which a 32-bit field cannot hold: its pair \
             is left out, not carried as a zero"
        );
    }
    assert_eq!(
        errors(&engine),
        Value::from(1),
        "§scxml-5.7.1: the failed pair is reported as error.execution, once"
    );
}
