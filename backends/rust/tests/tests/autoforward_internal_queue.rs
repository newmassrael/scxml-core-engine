// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4 autoforward skips internal-queue events — Rust AOT path.
//
// Appendix D's `mainEventLoop` forwards only what it dequeues from the
// external queue; the internal drain above it has no forwarding step at
// all. §6.2 raises `error.execution` onto the internal queue when `<send>`
// names an unsupported type, so it must never reach an `autoforward`
// child — and it must be excluded by where it was raised, not by a filter
// that recognises its name.
//
// Sibling of `autoforward_done_invoke`, which pins the positive half of
// the same loop. Together they leave no room for a name-based filter:
// one fails if `done.invoke` is withheld, the other if `done.state` leaks.
//
// Fixture: integration_resources/autoforward_internal_queue/autoforward_internal_queue.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_autoforward_internal_queue.sh

use std::time::Duration;

use sce_rust_tests::integration::autoforward_internal_queue::{
    AutoforwardInternalQueuePolicy, AutoforwardInternalQueueState,
};

#[test]
fn an_internal_queue_event_is_never_autoforwarded() {
    let policy = AutoforwardInternalQueuePolicy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();

    let completed = engine.run_until_completion(Duration::from_secs(2), Duration::from_millis(10));
    assert!(
        completed,
        "autoforward_internal_queue timed out before reaching a final state (parked in \
         {:?}) — the watcher child reported neither verdict, so neither `error.execution` \
         nor `probe` reached it",
        engine.get_current_state()
    );

    assert_eq!(
        engine.get_current_state(),
        AutoforwardInternalQueueState::Pass,
        "the watcher saw `error.execution`: an internal-queue event was autoforwarded. \
         W3C Appendix D `mainEventLoop` forwards only what it dequeues from the external \
         queue, and §6.2 raises `error.execution` onto the internal one — check that the \
         event was not routed onto the external queue for some unrelated reason (keeping \
         it from inline delivery, say), which would leak it past any name-blind forward"
    );
}
