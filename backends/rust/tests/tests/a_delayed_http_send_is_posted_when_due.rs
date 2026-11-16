// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2.4 + C.2: a delay is a property of the send, not of the
// processor it names — a delayed BasicHTTP send is POSTed when due, and
// `<cancel>` reaches it while it waits — Rust AOT path.
//
// Fixture: integration_resources/a_delayed_http_send_is_posted_when_due/a_delayed_http_send_is_posted_when_due.scxml
// (canonical, shared with the C++ AOT / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/materialize.sh rust-integration

use std::sync::{Arc, Mutex};

use sce_rust_runtime::{HttpSendFailure, HttpSendRequest, SceClock, StatePolicy};
use sce_rust_tests::integration::a_delayed_http_send_is_posted_when_due::{
    ADelayedHttpSendIsPostedWhenDuePolicy as Policy, ADelayedHttpSendIsPostedWhenDueState as State,
};

/// The requests the transport was handed, in the order it was handed them.
type Posted = Arc<Mutex<Vec<HttpSendRequest>>>;

fn events(posted: &Posted) -> Vec<String> {
    posted
        .lock()
        .unwrap()
        .iter()
        .map(|request| request.event_name.clone())
        .collect()
}

#[test]
fn a_delayed_http_send_is_posted_when_due() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let mut e = sce_rust_runtime::Engine::new(Policy::new(script_engine));
    let posted: Posted = Arc::new(Mutex::new(Vec::new()));
    let recorder = Arc::clone(&posted);
    e.set_http_send_callback(move |request| {
        recorder.lock().unwrap().push(request);
        Ok(None)
    });
    e.set_clock(SceClock::Manual(0));
    e.initialize()
        .expect("the machine's script session is established");

    // A zero wait, written or evaluated, is no deferral: the POST is made
    // before initialize() returns, with no tick to bring it out.
    assert_eq!(
        events(&posted),
        ["now", "zero", "zeroexpr"],
        "the undelayed send and the two zero-delay sends are POSTed at once"
    );

    e.advance_time_ms(99);
    assert_eq!(
        events(&posted),
        ["now", "zero", "zeroexpr"],
        "a send delayed 100ms is not POSTed at 99ms"
    );

    e.advance_time_ms(1);
    assert_eq!(
        events(&posted),
        ["now", "zero", "zeroexpr", "later"],
        "it is POSTed when the delay has elapsed, and the cancelled one never is"
    );
    let later = posted.lock().unwrap()[3].clone();
    assert_eq!(later.target, "http://127.0.0.1:18081/later");
    assert_eq!(later.send_id, "later");

    e.advance_time_ms(100);
    assert_eq!(
        events(&posted),
        ["now", "zero", "zeroexpr", "later", "dynamic"]
    );
    let dynamic = posted.lock().unwrap()[4].clone();
    assert_eq!(
        dynamic.target, "http://127.0.0.1:18081/dynamic",
        "a targetexpr is read when the send is made"
    );
    assert_eq!(dynamic.params.get("k"), Some(&vec!["v".to_string()]));

    e.advance_time_ms(100);
    assert_eq!(
        e.terminal_state(),
        Some(State::Done),
        "the run must end in `done`"
    );
    assert_eq!(
        events(&posted),
        ["now", "zero", "zeroexpr", "later", "dynamic"]
    );
}

/// The fixture's machine, started with a manual clock, and its
/// `error.communication` event. `transport` runs before `initialize`.
fn started(
    transport: impl FnOnce(&mut sce_rust_runtime::Engine<Policy>),
) -> (
    sce_rust_runtime::Engine<Policy>,
    <Policy as StatePolicy>::Event,
) {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let mut e = sce_rust_runtime::Engine::new(Policy::new(script_engine));
    transport(&mut e);
    e.set_clock(SceClock::Manual(0));
    e.initialize()
        .expect("the machine's script session is established");
    let communication = Policy::get_event_from_name("error.communication")
        .expect("the fixture's machine names an error.communication event");
    (e, communication)
}

/// W3C SCXML 6.2.4: a send the processor is unable to dispatch puts
/// error.communication on the internal queue. The transport says the POST of
/// `now` did not reach its server; the document declares no transition for the
/// error, so it is the one unhandled error the engine counts.
#[test]
fn a_basic_http_send_its_transport_could_not_deliver_is_error_communication() {
    let (e, communication) = started(|e| {
        e.set_http_send_callback(|request: HttpSendRequest| {
            if request.event_name == "now" {
                Err(HttpSendFailure::new("connection refused"))
            } else {
                Ok(None)
            }
        });
    });
    assert_eq!(
        e.unhandled_error_events(),
        1,
        "the undelivered POST of `now` must raise exactly one error"
    );
    assert_eq!(e.last_unhandled_error(), Some(communication));
}

/// A machine given no BasicHTTP transport has no way to dispatch the send: each
/// of the three sends made at once is reported, none is dropped unsaid.
#[test]
fn a_basic_http_send_with_no_transport_is_error_communication() {
    let (e, communication) = started(|_| {});
    assert_eq!(
        e.unhandled_error_events(),
        3,
        "the three sends made at once with no transport must each raise an error"
    );
    assert_eq!(e.last_unhandled_error(), Some(communication));
}
