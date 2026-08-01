// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix D: pending invokes start before the external dequeue — Rust AOT path.
//
// `mainEventLoop` completes the macrostep on eventless and internal
// transitions alone, then runs `invoke(inv)` for every state entered on the
// last iteration, and only then reaches `externalQueue.dequeue()`. The
// external queue is named exactly once in that loop and it is after the
// invokes.
//
// An engine that folds the external drain into its macrostep completion loop
// consumes whatever `<onentry>` queued for the parent itself while the invoked
// children do not yet exist, so an `autoforward` child misses every event the
// parent queued on the way in. That is a lost event, not a reordered one.
//
// The sibling `autoforward_dequeue_point` pins *where in the loop* the forward
// sits and is deliberately blind to this axis: there the child opens the
// exchange, so it is alive before anything is queued. Here the parent queues
// first and the child starts second.
//
// Fixture: integration_resources/invoke_precedes_external_dequeue/invoke_precedes_external_dequeue.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_invoke_precedes_external_dequeue.sh

use std::time::Duration;

use sce_rust_tests::integration::invoke_precedes_external_dequeue::{
    InvokePrecedesExternalDequeuePolicy, InvokePrecedesExternalDequeueState,
};

#[test]
fn pending_invokes_start_before_the_external_dequeue() {
    let policy = InvokePrecedesExternalDequeuePolicy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();

    let completed = engine.run_until_completion(Duration::from_secs(2), Duration::from_millis(10));
    assert!(
        completed,
        "invoke_precedes_external_dequeue timed out before reaching a final state \
         (parked in {:?}) — the watching child answered neither verdict, so `probe` \
         never reached it",
        engine.get_current_state()
    );

    assert_eq!(
        engine.get_current_state(),
        InvokePrecedesExternalDequeueState::Pass,
        "the watching child answered `probe` from `waiting`, so it never saw `kick`: \
         the parent drained its external queue before starting the invoke, and the \
         event `<onentry>` had queued for itself was consumed while no child existed. \
         W3C Appendix D `mainEventLoop` runs `invoke(inv)` for every state entered on \
         the last iteration before it reaches `externalQueue.dequeue()`, so an \
         autoforward child is live for the whole external queue"
    );
}
