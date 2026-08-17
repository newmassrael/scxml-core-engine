// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10 + B.2: a payload a HOST injects reaches the datamodel as a
// value — Rust AOT path.
//
// The edge nothing measured. Every other integration fixture in this
// repository drives its machine with `raise_external(event, "", "")`; the
// payload argument was the empty string in every call on every channel until
// this one, so the host-to-datamodel boundary was covered by no test at all.
// The W3C suite does not reach it either — its payloads originate inside the
// document (`<send><content>`, `<param>`, `<donedata>`), which every backend
// implements on a separate path from the one an embedder calls.
//
// It is the edge a supervising host actually uses: `examples/ai_loop` answers
// its machine with `{"done":true}` and the document selects on
// `_event.data.done`.
//
// Fixture: integration_resources/event_data_arrives_as_sent/event_data_arrives_as_sent.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_event_data_arrives_as_sent.sh

use sce_rust_tests::integration::event_data_arrives_as_sent::{
    EventDataArrivesAsSentEvent as Event, EventDataArrivesAsSentPolicy as Policy,
    EventDataArrivesAsSentState as State,
};

#[test]
fn a_hosts_json_payload_is_addressable_and_its_text_stays_text() {
    // The fixture reads `_event.data` in its guards, so this is an
    // ECMAScript-datamodel machine.
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let mut engine = sce_rust_runtime::Engine::new(Policy::new(script_engine));
    engine.initialize();

    assert_eq!(
        engine.get_active_states(),
        vec![State::Waiting],
        "the fixture is supposed to start in `waiting`, so nothing below is \
         testing what it claims"
    );

    // A JSON object, the shape an embedder has when it holds structured data
    // and a state machine to give it to.
    engine.raise_external(Event::Payload, r#"{"milestone":"refined","turns":2}"#, "");
    engine.step();

    let after_payload = engine.get_active_states();
    assert!(
        !after_payload.contains(&State::Mangled),
        "the host sent `{{\"milestone\":\"refined\",\"turns\":2}}` and the guard \
         `_event.data.milestone === 'refined' && _event.data.turns === 2` did not \
         hold, so the payload did not arrive as an object with those properties \
         (active: {after_payload:?})"
    );
    assert!(
        after_payload.contains(&State::Heard),
        "the payload guard neither matched nor mismatched — the machine is not in \
         `heard` (active: {after_payload:?})"
    );

    // Text that is not JSON. The same call, and it must NOT be parsed into
    // something else: `hold the line` is the value the document compares
    // against, character for character.
    engine.raise_external(Event::Note, "hold the line", "");
    engine.step();

    let after_note = engine.get_active_states();
    assert!(
        !after_note.contains(&State::Garbled),
        "the host sent the text `hold the line` and `_event.data === 'hold the line'` \
         did not hold, so a payload that is not JSON did not arrive as the string it \
         was sent as (active: {after_note:?})"
    );

    // Text that happens to be a valid expression. §scxml-B-2-8-1 gives the
    // payload three readings and none of them is "evaluate it": a payload is
    // what a host, a peer session or an HTTP sender put there, and running it
    // makes `_event.data` mean whatever the receiver's engine is written in.
    engine.raise_external(Event::Arith, "2 + 3", "");
    engine.step();

    let after_arith = engine.get_active_states();
    assert!(
        !after_arith.contains(&State::Evaluated),
        "the host sent the text `2 + 3` and it arrived as 5 — the payload was run \
         rather than read (active: {after_arith:?})"
    );
    assert!(
        after_arith.contains(&State::Settled),
        "the arithmetic-shaped payload neither matched nor mismatched \
         (active: {after_arith:?})"
    );
}
