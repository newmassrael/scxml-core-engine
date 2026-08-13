// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.4 + 3.7: `done.state.<parallel>` is delivered, not merely
// declared — Rust AOT path.
//
// The sibling fixture `parallel_completion_raises_done_state` carries no
// listener, deliberately: a transition's `event` attribute is itself a
// registration site, so a listener there would register the completion event
// no matter what the `<final>` walk does and leave that fixture unable to fail
// for the defect it exists to catch. What it proves is therefore that the
// event is DECLARED.
//
// Declared is not delivered. A backend that names the event and never raises
// it — or raises it where nothing selects from — passes there. This document
// listens, and `settled` is a top-level `<final>` no other route reaches.
//
// Fixture: integration_resources/parallel_done_state_is_delivered/parallel_done_state_is_delivered.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_parallel_done_state_is_delivered.sh

use sce_rust_tests::integration::parallel_done_state_is_delivered::{
    ParallelDoneStateIsDeliveredEvent as Event, ParallelDoneStateIsDeliveredPolicy as Policy,
    ParallelDoneStateIsDeliveredState as State,
};

#[test]
fn completion_carries_the_machine_to_a_top_level_final() {
    let policy = Policy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();

    let entry = engine.get_active_states();
    assert!(
        entry.contains(&State::A1) && entry.contains(&State::B1),
        "the fixture is supposed to start with both regions inside the `<parallel>`; \
         it came up as {entry:?}, so nothing below is testing what it claims"
    );

    engine.raise_external(Event::Go, "", "");
    engine.step();

    // One assertion, with the configuration in its message, because the two
    // ways this can fail are not separately observable: completion is selected
    // within the SAME macrostep as the regions' finals, so once `step` returns
    // the parallel has been exited and `A2`/`B2` are gone. Measured — checking
    // them as a precondition failed against engines that had already done the
    // right thing.
    //
    // The remaining states tell the two apart: `A1`/`B1` means `go` moved
    // nothing; `A2`/`B2` means the parallel completed and the event went
    // nowhere.
    let after = engine.get_active_states();
    assert!(
        after.contains(&State::Settled),
        "every region reaching its `<final>` completes the parallel, so `done.state.run` \
         had to be raised AND selected — `settled` is reachable by nothing else \
         (active: {after:?})"
    );
}
