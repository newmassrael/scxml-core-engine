// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML C.2 + 6.2.3 `<send namelist>` over BasicHTTP — Rust AOT.
//
// Two claims the IRP corpus states and cannot measure:
//
//   the namelist reaches the form   test518 is titled "namelist values get
//     encoded as POST parameters" and passes as soon as the event comes
//     back, whatever it carried. This channel filled `http_params` from
//     `<param>` alone, so a `namelist` was evaluated into the payload and
//     then posted with an empty form — green the whole time.
//
//   an unreadable item reports and discards   §scxml-5.9.2 puts
//     `error.execution` on the internal queue; §scxml-6.2.3 discards the
//     message. `<param>`'s per-item exception (§scxml-5.7.1, "ignore the
//     name and value") has no counterpart for namelist anywhere in the
//     specification.
//
// The two reach distinct final states, so a failure names which one broke.
//
// Fixture: integration_resources/send_namelist_over_http/send_namelist_over_http.scxml
// (canonical, shared with the other channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_send_namelist_over_http.sh
//
// Needs the W3C harness server, like every BasicHTTP fixture here:
//   node tests/w3c/standalone_http_server.js 8080 /test

use std::time::Duration;

use sce_rust_tests::integration::send_namelist_over_http::{
    SendNamelistOverHttpPolicy, SendNamelistOverHttpState,
};

#[test]
fn send_namelist_reaches_the_form_and_a_broken_item_discards_the_message() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let mut policy = SendNamelistOverHttpPolicy::new(script_engine);
    policy.set_basic_http_access_uri(sce_rust_tests::harness::HTTP_TEST_SERVER_URL);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    sce_rust_tests::harness::setup_http_test(&mut engine);
    engine.initialize();

    let completed = engine.run_until_completion(Duration::from_secs(15), Duration::from_millis(10));
    assert!(
        completed,
        "send_namelist_over_http timed out before reaching a final state — the \
         delayed `timeoutMap` / `timeoutDiscard` sends that give each phase its \
         verdict never fired, so the machine is not being ticked"
    );

    match engine.get_current_state() {
        SendNamelistOverHttpState::Pass => {}
        SendNamelistOverHttpState::FailNamelistNeverArrived => panic!(
            "the BasicHTTP send never came back at all — the harness server did not \
             answer, which is a different failure from posting the wrong form"
        ),
        SendNamelistOverHttpState::FailNamelistNotPosted => panic!(
            "`mapped` arrived without `Var1` in its data — W3C SCXML C.2 requires a \
             namelist's variable names and values to be mapped to HTTP POST \
             parameters, and this channel posted a form built from `<param>` alone"
        ),
        SendNamelistOverHttpState::FailMessageNotDiscarded => panic!(
            "`shouldNotArrive` was delivered — W3C SCXML 6.2.3 discards the message \
             when the evaluation of a `<send>`'s arguments produces an error. \
             `<param>`'s per-item rule (5.7.1) does not reach a namelist item"
        ),
        SendNamelistOverHttpState::FailNoNamelistError => panic!(
            "no `error.execution` preceded the timeout — W3C SCXML 5.9.2 requires it \
             when a location expression yields no valid location, and the wire the \
             send would have crossed does not change the answer"
        ),
        other => {
            panic!("send_namelist_over_http settled in {other:?}, which is not a verdict state")
        }
    }
}
