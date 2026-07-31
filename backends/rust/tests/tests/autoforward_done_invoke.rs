// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4 autoforward carries `done.invoke.<id>` — Rust AOT path.
//
// Appendix D's `mainEventLoop` forwards every event it dequeues from the
// external queue to each `autoforward` child, without testing the event's
// name; the sole exclusion is the cancel event, and it is expressed as
// control flow. §6.4.2 puts `done.invoke.<id>` on the external queue of
// the invoking session, so a sibling child that is still running must
// receive it.
//
// The IRP suite cannot see this: test229 checks only that a name crosses
// and test230 is a manual test, and neither runs two concurrent invokes.
//
// Fixture: integration_resources/autoforward_done_invoke/autoforward_done_invoke.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_autoforward_done_invoke.sh

use std::time::Duration;

use sce_rust_tests::integration::autoforward_done_invoke::{
    AutoforwardDoneInvokePolicy, AutoforwardDoneInvokeState,
};

#[test]
fn done_invoke_from_a_sibling_reaches_the_autoforward_child() {
    let policy = AutoforwardDoneInvokePolicy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();

    let completed = engine.run_until_completion(Duration::from_secs(2), Duration::from_millis(10));
    assert!(
        completed,
        "autoforward_done_invoke timed out before reaching a final state — the \
         watcher child reported neither verdict, so `done.invoke.inv_short` never \
         reached the parent's external queue at all"
    );

    assert_eq!(
        engine.get_current_state(),
        AutoforwardDoneInvokeState::Pass,
        "the watcher saw only `probe`: `done.invoke.inv_short` was withheld from a \
         live `autoforward` child. W3C Appendix D `mainEventLoop` forwards every \
         event dequeued from the external queue and excludes only the cancel event, \
         and §6.4.2 places `done.invoke.<id>` on that queue — so no name-based \
         platform-event filter belongs on the forwarding path"
    );
}
