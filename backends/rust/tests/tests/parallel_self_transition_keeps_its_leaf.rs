// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.4: a region that took an external self-transition still holds an
// atomic state, so it answers the next event too — Rust AOT path.
//
// Its sibling `parallel_regions_take_own_transitions` owns the microstep axis.
// This one owns what that axis cannot reach: a region can take its transition
// and run its content exactly as required and still be left holding no leaf,
// and the only thing that tells you so is a later event it fails to answer.
//
// Measured 2026-08-14 on the C++ AOT channel, where the defect lived: with
// `sce-build/tests/mutations/parallel_microstep_owns_exit_and_entry.cases`
// restoring it, the sibling fixture's driver stayed green (SURVIVED, 0/2 red)
// and this fixture's went red (CAUGHT, 1/2). The two differ in which region
// the engine's current-state pointer rests in — re-exiting a leaf the
// microstep has already left is harmless, re-exiting one it just re-entered is
// the loss — which is why the self-transitioning region is first in document
// order here.
//
// Fixture: integration_resources/parallel_self_transition_keeps_its_leaf/parallel_self_transition_keeps_its_leaf.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_parallel_self_transition_keeps_its_leaf.sh

use sce_rust_tests::integration::parallel_self_transition_keeps_its_leaf::{
    ParallelSelfTransitionKeepsItsLeafEvent as Event,
    ParallelSelfTransitionKeepsItsLeafPolicy as Policy,
    ParallelSelfTransitionKeepsItsLeafState as State,
};

#[test]
fn the_self_transitioned_region_answers_the_next_event() {
    // The fixture's `<assign>`s make this an ECMAScript-datamodel machine, so
    // the policy takes an engine — the same injection the other scripted
    // integration fixtures use on this channel.
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = Policy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();

    let entry = engine.get_active_states();
    assert!(
        entry.contains(&State::Within) && entry.contains(&State::Working),
        "the fixture is supposed to start with the self-transitioning region in `within` and \
         the deeper one in `working`; it came up as {entry:?}, so nothing below is testing \
         what it claims"
    );

    engine.raise_external(Event::E, "", "");
    engine.step();

    // The symptom, named where it happens. A region holding no atomic state is
    // still "in" the parallel by every ancestor test, so this is the only
    // place the loss is visible before it becomes a missing answer.
    let after = engine.get_active_states();
    assert!(
        after.contains(&State::Within),
        "the self-transitioning region lost its leaf on the first event (active: {after:?}). \
         `within` is both the source and the target of its own external self-transition, so \
         the microstep exits and re-enters it; anything that exits it a second time takes it \
         back out and does not put it back"
    );
    assert!(
        after.contains(&State::Judging),
        "the deeper region did not take its own transition on `e` (active: {after:?})"
    );

    // The second event is the one this fixture exists for: `judging` has no
    // `e` transition, so nothing but the self-transitioning region can answer,
    // and it can only answer from a leaf.
    engine.raise_external(Event::E, "", "");
    engine.step();

    let twice = engine.get_active_states();
    assert!(
        twice.contains(&State::Within),
        "the self-transitioning region is not in `within` after the second `e` (active: {twice:?})"
    );

    engine.raise_external(Event::Check, "", "");
    engine.step();

    let settled = engine.get_active_states();
    assert!(
        settled.contains(&State::Settled),
        "`check` did not carry the machine to `settled` (active: {settled:?}), which the \
         document guards on `n == 1 && m == 2`. `m` reaches 2 only if the self-transitioning \
         region still had a leaf to transition from when the second `e` arrived — a region \
         that kept its root and lost its leaf answers no event and raises nothing while it \
         fails to"
    );
}
