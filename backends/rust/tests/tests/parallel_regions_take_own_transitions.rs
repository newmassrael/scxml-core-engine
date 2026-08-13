// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.4: every region of a `<parallel>` takes its own enabled
// transition in the same microstep — Rust AOT path.
//
// The fixture is asymmetric on purpose. One region's transition on the event
// is an external self-transition, whose domain Appendix D resolves through
// `findLCCA` over the proper ancestors — candidates that never include the
// state itself. Answering with the state left the exit-set walk without a
// stopping point, so it ran to the document root, the exit set named the
// enclosing `<parallel>`, and conflict resolution preempted the deeper
// region's transition on that same event.
//
// The observable is `settled`, which the document reaches only when both
// regions' assignments have run — a configuration check alone would still
// pass for a region that moved without executing its transition content.
//
// Fixture: integration_resources/parallel_regions_take_own_transitions/parallel_regions_take_own_transitions.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_parallel_regions_take_own_transitions.sh

use sce_rust_tests::integration::parallel_regions_take_own_transitions::{
    ParallelRegionsTakeOwnTransitionsEvent as Event,
    ParallelRegionsTakeOwnTransitionsPolicy as Policy,
    ParallelRegionsTakeOwnTransitionsState as State,
};

#[test]
fn every_region_takes_its_own_transition() {
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
        entry.contains(&State::Working) && entry.contains(&State::Within),
        "the fixture is supposed to start with the deeper region in `working` and the \
         shallower one in `within`; it came up as {entry:?}, so nothing below is \
         testing what it claims"
    );

    engine.raise_external(Event::E, "", "");
    engine.step();

    let after = engine.get_active_states();
    assert!(
        after.contains(&State::Judging),
        "the deeper region lost its leaf (active: {after:?}). W3C SCXML 3.4 has every \
         region take its own enabled transition on `e`; the sibling region's external \
         self-transition must not preempt this one"
    );
    assert!(
        after.contains(&State::Within),
        "the shallower region left `within`, which is both the source and the target \
         of its own external self-transition (active: {after:?})"
    );

    engine.raise_external(Event::Check, "", "");
    engine.step();

    let settled = engine.get_active_states();
    assert!(
        settled.contains(&State::Settled),
        "`check` did not carry the machine to `settled` (active: {settled:?}), which the \
         document guards on both regions' assignments having run. Reaching `judging` \
         without `n == 1 && m == 1` means a region changed state while its transition \
         content was skipped"
    );
}
