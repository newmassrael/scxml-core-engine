// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix D exitStates: a state leaves the configuration AFTER its
// own `<onexit>` has run — Rust AOT path.
//
// The procedure is onexit, then cancelInvoke, then configuration.delete(s),
// for each state in exitOrder. Measured 2026-09-26, this channel did the
// reverse: its generated exit deactivated the state first, cancelled its
// invocations, and ran the `<onexit>` last, so `In(s)` inside s's own handler
// answered false. The configuration after the microstep cannot show that, so
// the handlers record what they saw and those records are the verdict.
//
// Fixture: integration_resources/onexit_runs_before_the_state_leaves/onexit_runs_before_the_state_leaves.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_onexit_runs_before_the_state_leaves.sh

use sce_rust_tests::integration::onexit_runs_before_the_state_leaves::{
    OnexitRunsBeforeTheStateLeavesEvent as Event, OnexitRunsBeforeTheStateLeavesPolicy as Policy,
    OnexitRunsBeforeTheStateLeavesState as State,
};

#[test]
fn a_state_is_still_active_while_its_own_onexit_runs() {
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

    e.raise_external(Event::Leave, "", "");
    e.step();

    let settled = e.get_active_states();
    // What the handlers recorded (W3C SCXML 5.3 readers): the final says which
    // clause broke, these say what the handler actually saw.
    let p = e.policy();
    let records = format!(
        "selfInInner={:?} parentInInner={:?} selfInOuter={:?} childInOuter={:?} exits={:?}; \
         wanted Some(1) / Some(1) / Some(1) / Some(0) / Some(2)",
        p.self_in_inner(),
        p.parent_in_inner(),
        p.self_in_outer(),
        p.child_in_outer(),
        p.exits()
    );
    assert_eq!(
        e.terminal_state(),
        Some(State::Settled),
        "`leave` did not carry the machine to `settled` (active: {settled:?}; {records}). The document \
         checks its clauses in document order and lands each in a `<final>` of its own: \
         `failExits` (a handler did not run), `failSelfInInner` / `failSelfInOuter` (a state \
         was already out of the configuration during its own `<onexit>`), `failParentInInner` \
         (the parent left before its child's `<onexit>`), `failChildInOuter` (the child was \
         still active during its parent's `<onexit>`)"
    );
}
