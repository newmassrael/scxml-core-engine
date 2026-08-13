// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.4 + 3.7: a `<parallel>` completing raises `done.state.<id>` —
// Rust AOT path.
//
// A `<parallel>` owns no `<final>` of its own; its finals sit one level down,
// inside the regions. A rule that registers the completion event by walking
// from a `<final>` to its direct parent therefore never reaches the parallel,
// while an emitter that raises it from the grandparent does — which is how the
// C++ and C11 channels ended up naming an enumerator nothing had declared.
//
// This channel is asked the behavioural half of the same question: both
// regions reaching their `<final>` on one event, in one microstep.
//
// Fixture: integration_resources/parallel_completion_raises_done_state/parallel_completion_raises_done_state.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_parallel_completion_raises_done_state.sh

use sce_rust_tests::integration::parallel_completion_raises_done_state::{
    ParallelCompletionRaisesDoneStateEvent as Event,
    ParallelCompletionRaisesDoneStatePolicy as Policy,
    ParallelCompletionRaisesDoneStateState as State,
};

#[test]
fn every_region_final_completes_the_parallel() {
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

    let after = engine.get_active_states();
    assert!(
        after.contains(&State::A2),
        "region `a` did not reach its `<final>` on `go` (active: {after:?})"
    );
    assert!(
        after.contains(&State::B2),
        "region `b` did not reach its `<final>` on `go` (active: {after:?})"
    );
}
