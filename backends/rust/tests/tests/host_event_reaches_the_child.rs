// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4: autoforward is owed to the external event, not to the door it
// came through — Rust AOT path.
//
// The four sibling `autoforward_*` stems all let the machine forward events it
// queued for itself. This one hands it one from outside, through the engine's
// own "here is an event" entry point, and asks whether the `autoforward` child
// sees it. Appendix D's `mainEventLoop` binds the preliminary step
// (`applyFinalize` + the autoforward `send`) to the external event it is about
// to select transitions for, so an engine with a second door has to run the
// step at both or the child goes blind to everything the host delivers.
//
// Measured 2026-08-21: the C++ AOT engine had the step written inline in its
// queue drain, so `processEvent()` skipped it. This engine's `process_event`
// raises onto the external queue and steps, so the drain is its only door and
// the fixture pins that — a later `process_event` that hands the event
// straight to the transition selector would go red here.
//
// Fixture: integration_resources/host_event_reaches_the_child/host_event_reaches_the_child.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_host_event_reaches_the_child.sh

use std::time::Duration;

use sce_rust_tests::integration::host_event_reaches_the_child::{
    HostEventReachesTheChildEvent, HostEventReachesTheChildPolicy, HostEventReachesTheChildState,
};

/// Drive the machine until the child's handshake has moved it to `armed`, the
/// one state that can be handed an event from outside. Bounded rather than
/// timed: every step here is the machine's own work, so a machine that has not
/// arrived after this many is not slow, it is not going to.
fn drive_to_armed(engine: &mut sce_rust_runtime::Engine<HostEventReachesTheChildPolicy>) {
    for _ in 0..50 {
        if engine.get_current_state() == HostEventReachesTheChildState::Armed {
            return;
        }
        engine.tick();
    }
}

#[test]
fn an_event_the_host_hands_over_reaches_the_autoforward_child() {
    let policy = HostEventReachesTheChildPolicy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();

    drive_to_armed(&mut engine);
    assert_eq!(
        engine.get_current_state(),
        HostEventReachesTheChildState::Armed,
        "the probe child never sent `ready`, so the fixture never reached the state where a \
         host event can be handed over — this is a broken handshake, not a forwarding verdict"
    );

    // The axis: the host's own entry point, not `raise_external` + `tick`.
    engine.process_event(HostEventReachesTheChildEvent::HostPing);

    let completed = engine.run_until_completion(Duration::from_secs(2), Duration::from_millis(10));
    assert!(
        completed,
        "host_event_reaches_the_child timed out before reaching a final state (parked in {:?}) \
         — the probe child answered neither verdict, so neither `hostPing` nor `marker` \
         reached it",
        engine.get_current_state()
    );

    assert_eq!(
        engine.get_current_state(),
        HostEventReachesTheChildState::Pass,
        "the probe child answered `sawMarkerOnly`, so the event the host handed to \
         `process_event` was never forwarded to it: the child only ever saw the `marker` the \
         parent's own transition body sent. W3C Appendix D `mainEventLoop` runs the \
         autoforward `send` against the external event before it selects transitions for it, \
         whichever door the event arrived through — an engine that runs that step only in its \
         queue drain leaves an `autoforward` child blind to everything its host delivers"
    );
}
