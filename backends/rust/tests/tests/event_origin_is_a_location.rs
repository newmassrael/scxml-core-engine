// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix C.1 `_event.origin` is an address — Rust AOT.
//
// The clause has two halves. The origin of a delivered event must match the
// `location` field the sending session published for the SCXML Event I/O
// Processor in its `_ioprocessors`, and that location is what a peer sends
// back to. A machine that puts a bare session id there satisfies neither:
// the value matches nothing the sender published, and it names no target.
//
// The public IRP suite cannot separate the two spellings. Test 336 and
// test 350 both check `_event.origin` by sending to it with the sender and
// the receiver being the same session, so any value at all round-trips.
// Nothing in the corpus sends across sessions, which is the only
// arrangement where a bare id and a location differ.
//
// The fixture puts a second session on the other end, so the two halves
// separate and each has its own signal:
//
//   mismatch   the parent lands in `fail` — `_event.origin` did not equal
//              the location the child published for itself
//   routing    the parent parks in `await_reply` and the run times out —
//              a target that resolves nowhere delivers no event to fail on
//
// Fixture: integration_resources/event_origin_is_a_location/event_origin_is_a_location.scxml
// (canonical, shared with the C++ / Go / Kotlin / Python / C11 channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_event_origin_is_a_location.sh

use std::time::Duration;

use sce_rust_tests::integration::event_origin_is_a_location::{
    EventOriginIsALocationPolicy, EventOriginIsALocationState,
};

#[test]
fn origin_is_the_senders_published_location_and_routes_back() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = EventOriginIsALocationPolicy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();

    let completed = engine.run_until_completion(Duration::from_secs(2), Duration::from_millis(10));
    assert!(
        completed,
        "event_origin_is_a_location timed out parked in {:?}. The parent accepted \
         `_event.origin` as an address and sent `reply` to it, and nothing came \
         back: Appendix C.1 requires the published location to be a usable <send> \
         target, so an origin that routes nowhere fails the half a self-addressed \
         test cannot exercise",
        engine.get_current_state()
    );

    match engine.get_current_state() {
        EventOriginIsALocationState::Pass => {}
        EventOriginIsALocationState::Fail => panic!(
            "`_event.origin` did not carry the sender's published `_ioprocessors` \
             location. Appendix C.1 requires the origin to match that location, \
             which is what makes it an address a peer can answer; a bare session \
             id matches nothing the sender published"
        ),
        other => {
            panic!("event_origin_is_a_location settled in {other:?}, which is not a verdict state")
        }
    }
}
