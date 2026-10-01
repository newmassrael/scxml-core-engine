// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15 — a `<param expr>` of a `<send>` or an `<invoke>`
// the HOST serves, in a `datamodel="sce-static"` machine, carries the value of
// a typed expression read from the machine's own fields when the send or the
// invoke happens. Rust compile+run gate; the Kotlin twin is
// `StaticHostParamsTest`.
//
// The committed machine under `src/integration/host_processor/` is generated
// from `sce-build/tests/fixtures/host_processor/statechart_static_host_params.scxml`
// (regen: `scripts/regen_host_processor.sh`) and constructed with NO script
// engine: its variables are fields, and a `<param>` that needed an engine to be
// read would not have one to ask.
//
// What this holds is the value on the wire. The run `bump`, `go` changes every
// variable before the send and the invoke read it, so a copy taken at start-up
// (3, false, "idle") is told from what the fields hold now (4, true, "busy").
// Before the `<param>` was lowered, the Rust machine called a script engine it
// had not been given and did not compile, and a host invoke read a copy of the
// variable inside an engine that `<assign>` never wrote.

use std::sync::{Arc, Mutex};

use sce_rust_runtime::json::{self, Value};
use sce_rust_runtime::{Engine, HostInvokeEvent, HostInvokeRequest, HostSendRequest};
use sce_rust_tests::integration::host_processor::{
    StatechartStaticHostParamsPolicy as Policy, StatechartStaticHostParamsState as State,
};

/// The type the fixture was compiled for; `scripts/regen_host_processor.sh`
/// passes the same string to both declarations.
const DECLARED_TYPE: &str = "x-sce-host";

type Sends = Arc<Mutex<Vec<HostSendRequest>>>;
type Starts = Arc<Mutex<Vec<HostInvokeRequest>>>;

/// A machine with the host's side registered, standing at `idle`.
fn started() -> (Engine<Policy>, Sends, Starts) {
    let sends: Sends = Arc::new(Mutex::new(Vec::new()));
    let starts: Starts = Arc::new(Mutex::new(Vec::new()));

    let mut engine = Engine::new(Policy::new());
    let recorder = Arc::clone(&sends);
    engine.register_event_processor(DECLARED_TYPE, move |request: HostSendRequest| {
        recorder.lock().expect("send log").push(request);
        Vec::new()
    });
    let recorder = Arc::clone(&starts);
    engine.register_invoker(DECLARED_TYPE, move |event: HostInvokeEvent| {
        if let HostInvokeEvent::Start(request) = event {
            recorder.lock().expect("start log").push(request);
        }
        None
    });
    engine.initialize();
    (engine, sends, starts)
}

fn drive(engine: &mut Engine<Policy>, events: &[&str]) {
    for event in events {
        engine.raise_external_by_name(event, "");
        engine.step();
    }
}

fn text_params(
    params: &std::collections::HashMap<String, Vec<String>>,
) -> Vec<(String, Vec<String>)> {
    let mut read: Vec<_> = params
        .iter()
        .map(|(name, values)| (name.clone(), values.clone()))
        .collect();
    read.sort();
    read
}

/// The text each param crosses as, given what `count`, `ready`, `label` and
/// `twice` hold. `delta` and `ratio` never change; `boom` is left out, because
/// the multiplication that makes it overflows a 32-bit field.
fn wanted_text_params(
    count: &str,
    ready: &str,
    label: &str,
    twice: &str,
) -> Vec<(String, Vec<String>)> {
    let mut wanted: Vec<_> = [
        ("count", count),
        ("ready", ready),
        ("label", label),
        ("twice", twice),
        ("delta", "-5"),
        ("ratio", "1.5"),
    ]
    .iter()
    .map(|(name, value)| ((*name).to_string(), vec![(*value).to_string()]))
    .collect();
    wanted.sort();
    wanted
}

/// `event_data` is the pairs as JSON, typed as the data model holds them: a
/// number stays a number, a bool a bool, a string a string. Compared as
/// values, not text, because the order of an object's members is the backend's.
fn assert_typed_event_data(event_data: &str, what: &str) {
    let value = json::parse(event_data)
        .unwrap_or_else(|e| panic!("{what}: event_data is not JSON ({e}): {event_data}"));
    let wanted = [
        ("count", Value::Number("4".to_string())),
        ("ready", Value::Bool(true)),
        ("label", Value::Text("busy".to_string())),
        ("twice", Value::Number("8".to_string())),
        ("delta", Value::Number("-5".to_string())),
        ("ratio", Value::Number("1.5".to_string())),
    ];
    for (name, expected) in wanted {
        assert_eq!(
            value.member(name),
            Some(&expected),
            "{what}: `{name}` in {event_data}"
        );
    }
    assert_eq!(
        value.member("boom"),
        None,
        "{what}: a pair whose value failed is left out: {event_data}"
    );
}

#[test]
fn a_send_param_carries_the_value_the_fields_hold_when_it_is_sent() {
    let (mut engine, sends, _) = started();
    drive(&mut engine, &["bump", "go"]);
    assert_eq!(engine.get_current_state(), State::Working);

    let sends = sends.lock().expect("send log");
    assert_eq!(sends.len(), 1, "one <send>, one request: {sends:?}");
    assert_eq!(
        text_params(&sends[0].params),
        wanted_text_params("4", "true", "busy", "8"),
        "the text each <param> crosses as"
    );
    assert_typed_event_data(&sends[0].event_data, "send");
}

#[test]
fn an_invoke_param_carries_the_value_the_fields_hold_when_it_starts() {
    let (mut engine, _, starts) = started();
    drive(&mut engine, &["bump", "go"]);

    let starts = starts.lock().expect("start log");
    assert_eq!(starts.len(), 1, "one <invoke>, one start: {starts:?}");
    assert_eq!(
        text_params(&starts[0].params),
        wanted_text_params("4", "true", "busy", "8"),
        "the text each <param> crosses as: a copy taken at start-up would say \
         count 3, ready false, label idle"
    );
    assert_typed_event_data(&starts[0].event_data, "invoke");
}

#[test]
fn a_param_read_before_any_bump_carries_the_declared_values() {
    // The same machine on the shorter run: nothing has written a variable, so
    // the fields still hold what `<data expr>` gave them. The control that
    // keeps the two cases above from passing on a value that is simply always
    // the new one.
    let (mut engine, sends, starts) = started();
    drive(&mut engine, &["go"]);

    assert_eq!(
        text_params(&sends.lock().expect("send log")[0].params),
        wanted_text_params("3", "false", "idle", "6")
    );
    assert_eq!(
        text_params(&starts.lock().expect("start log")[0].params),
        wanted_text_params("3", "false", "idle", "6")
    );
}

/// W3C SCXML 5.7.1: a `<param>` whose value cannot be computed — here a
/// multiplication a 32-bit field cannot hold — is reported with
/// `error.execution` and its pair left out, while the message still goes and
/// the invocation still starts. `errors` counts the reports the document took,
/// one for the send and one for the invoke, so a pair dropped in silence is
/// told from one reported.
#[test]
fn a_param_whose_value_cannot_be_computed_is_reported_and_left_out() {
    let (mut engine, sends, starts) = started();
    drive(&mut engine, &["bump", "go"]);

    assert_eq!(
        sends.lock().expect("send log").len(),
        1,
        "the send still went"
    );
    assert_eq!(
        starts.lock().expect("start log").len(),
        1,
        "the invoke still started"
    );
    assert_eq!(
        engine.policy().errors(),
        2,
        "one error.execution for the send's `boom` and one for the invoke's"
    );
    for (what, params) in [
        (
            "send",
            text_params(&sends.lock().expect("send log")[0].params),
        ),
        (
            "invoke",
            text_params(&starts.lock().expect("start log")[0].params),
        ),
    ] {
        assert!(
            params.iter().all(|(name, _)| name != "boom"),
            "{what}: the failed pair is left out: {params:?}"
        );
    }
}
