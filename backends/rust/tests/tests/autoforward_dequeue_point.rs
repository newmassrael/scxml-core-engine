// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4 autoforward happens at the external dequeue — Rust AOT path.
//
// Appendix D's `mainEventLoop` forwards one statement after
// `externalQueue.dequeue()` and before `selectTransitions`, and §6.4.2 says
// the same in prose: the parent forwards "at the point at which it removes it
// from the external event queue". Forwarding where the event is *queued*
// instead breaks run-to-completion — the child sees event N before the parent
// has processed 1..N-1.
//
// Siblings `autoforward_done_invoke` and `autoforward_internal_queue` pin
// *which* events are forwarded and are deliberately blind to *when*; this one
// pins the position and nothing else.
//
// Fixture: integration_resources/autoforward_dequeue_point/autoforward_dequeue_point.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_autoforward_dequeue_point.sh

use std::time::Duration;

use sce_rust_tests::integration::autoforward_dequeue_point::{
    AutoforwardDequeuePointPolicy, AutoforwardDequeuePointState,
};

#[test]
fn an_external_event_is_forwarded_at_the_dequeue_not_the_enqueue() {
    let policy = AutoforwardDequeuePointPolicy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();

    let completed = engine.run_until_completion(Duration::from_secs(2), Duration::from_millis(10));
    assert!(
        completed,
        "autoforward_dequeue_point timed out before reaching a final state (parked in \
         {:?}) — the probe child reported neither verdict, so `second` never reached it",
        engine.get_current_state()
    );

    assert_eq!(
        engine.get_current_state(),
        AutoforwardDequeuePointState::Pass,
        "the probe child saw `second` before `mark`, so both events were handed over \
         while the parent was still executing the transition that queued them. W3C \
         Appendix D `mainEventLoop` forwards one statement after \
         `externalQueue.dequeue()`, and §6.4.2 puts it \"at the point at which it \
         removes it from the external event queue\" — forwarding at the enqueue lets \
         the child run ahead of the parent by a whole event"
    );
}
