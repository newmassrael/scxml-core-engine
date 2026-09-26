// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix D exitInterpreter: a run ends by exiting every state it
// is still in, the way exitStates exits one — Rust AOT path.
//
// Reached two ways, and both are driven here: the run enters a top-level
// `<final>` after a step, or the host stops it. Measured 2026-09-26, this
// channel ran the final's `<onexit>` only when the run ended during
// `initialize()` with a completion callback set, kept the final in the
// configuration afterwards, and ran no `<onexit>` at all on `stop()`.
//
// Fixture: integration_resources/the_run_ends_by_exiting_every_state/the_run_ends_by_exiting_every_state.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_the_run_ends_by_exiting_every_state.sh

use sce_rust_tests::integration::the_run_ends_by_exiting_every_state::{
    TheRunEndsByExitingEveryStateEvent as Event, TheRunEndsByExitingEveryStatePolicy as Policy,
    TheRunEndsByExitingEveryStateState as State,
};

type Engine = sce_rust_runtime::Engine<Policy>;

fn started() -> Engine {
    // The handlers record with `<assign>`, so the policy takes an engine.
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let mut e = sce_rust_runtime::Engine::new(Policy::new(script_engine));
    e.initialize();
    let entry = e.get_active_states();
    assert!(
        entry.contains(&State::Inner),
        "the run has to start inside `inner`; it came up as {entry:?}"
    );
    e
}

/// What the run left behind (W3C SCXML 5.3 readers), printed together so a
/// failure says what happened and not only which clause broke.
fn describe(e: &Engine) -> String {
    let p = e.policy();
    format!(
        "active: {:?}, ended in {:?}, running={}, order={:?} finalExits={:?} selfInFinal={:?}",
        e.get_active_states(),
        e.terminal_state(),
        e.is_running(),
        p.order(),
        p.final_exits(),
        p.self_in_final()
    )
}

/// The run ends in a top-level `<final>` after a step: the final is exited too.
#[test]
fn a_run_that_reaches_its_final_exits_the_final() {
    let mut e = started();
    e.raise_external(Event::Finish, "", "");
    e.step();

    let seen = describe(&e);
    assert_eq!(e.terminal_state(), Some(State::Done), "{seen}");
    assert!(!e.is_running(), "{seen}");
    assert!(
        e.get_active_states().is_empty(),
        "exitInterpreter deletes every state it exits, the final included. {seen}"
    );
    assert_eq!(
        e.policy().final_exits(),
        Some(1),
        "the final's own <onexit> must run exactly once as the run ends. {seen}"
    );
    assert_eq!(
        e.policy().self_in_final(),
        Some(1),
        "`done` must still be in the configuration during its own <onexit>. {seen}"
    );
    assert_eq!(
        e.policy().order(),
        Some(12),
        "`finish` exits `inner` then `outer`. {seen}"
    );
}

/// The host stops a run that has not ended: every state is exited, innermost first.
#[test]
fn a_stopped_run_exits_every_state_innermost_first() {
    let mut e = started();
    e.stop();

    let seen = describe(&e);
    assert_eq!(
        e.terminal_state(),
        None,
        "a stopped run did not end in a final. {seen}"
    );
    assert!(!e.is_running(), "{seen}");
    assert!(e.get_active_states().is_empty(), "{seen}");
    assert_eq!(
        e.policy().order(),
        Some(12),
        "stop() must run `inner`'s <onexit> and then `outer`'s: 0 is a stop that exited nothing, \
         21 one that exited in document order instead of exit order. {seen}"
    );
    assert_eq!(
        e.policy().final_exits(),
        Some(0),
        "the run never entered `done`. {seen}"
    );
}
