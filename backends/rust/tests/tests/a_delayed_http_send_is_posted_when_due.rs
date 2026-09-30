// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
//   scripts/regen_a_delayed_http_send_is_posted_when_due.sh

use std::sync::{Arc, Mutex};

use sce_rust_runtime::{HttpSendRequest, SceClock};
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
        None
    });
    e.set_clock(SceClock::Manual(0));
    e.initialize();

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
