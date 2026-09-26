// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4: an invoke whose state leaves within its macrostep is never
// attempted, so the §6.4.1 error.execution for a type no processor runs is
// never raised — Rust AOT path.
//
// Measured 2026-09-26, a document whose only `<invoke>` was refused emitted
// no exit chain in this channel, so the pending invoke survived its state and
// raised error.execution at the end of the macrostep.
//
// Fixture: integration_resources/an_invoke_left_before_it_starts_raises_nothing/an_invoke_left_before_it_starts_raises_nothing.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_an_invoke_left_before_it_starts_raises_nothing.sh

use sce_rust_tests::integration::an_invoke_left_before_it_starts_raises_nothing::{
    AnInvokeLeftBeforeItStartsRaisesNothingEvent as Event,
    AnInvokeLeftBeforeItStartsRaisesNothingPolicy as Policy,
    AnInvokeLeftBeforeItStartsRaisesNothingState as State,
};

#[test]
fn the_left_states_invoke_is_never_attempted() {
    // The handlers record with `<assign>`, so the policy takes an engine.
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let mut e = sce_rust_runtime::Engine::new(Policy::new(script_engine));
    e.initialize();
    assert!(
        e.get_active_states().contains(&State::S1),
        "`s0` leaves on an eventless transition within the first macrostep"
    );

    e.raise_external(Event::Finish, "", "");
    e.step();

    assert_eq!(
        e.terminal_state(),
        Some(State::Done),
        "`finish` must carry the run to `done`"
    );
    assert_eq!(
        e.policy().errors(),
        Some(0),
        "`s0` left before its macrostep ended, yet its invoke was attempted and raised \
         error.execution; W3C SCXML 6.4 cancels an invoke whose state has left"
    );
}
