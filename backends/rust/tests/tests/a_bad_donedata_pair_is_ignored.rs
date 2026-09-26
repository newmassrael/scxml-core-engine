// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.7: a <donedata> pair that cannot be evaluated raises
// error.execution and is ignored; the done events are raised all the same —
// Rust AOT path.
//
// Measured 2026-09-27, this channel treated an empty `location` as a
// structural error that withheld done.state.<parent> — while still raising
// the <parallel>'s done event, so the two answers disagreed with each other.
//
// Fixture: integration_resources/a_bad_donedata_pair_is_ignored/a_bad_donedata_pair_is_ignored.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_a_bad_donedata_pair_is_ignored.sh

use sce_rust_tests::integration::a_bad_donedata_pair_is_ignored::{
    ABadDonedataPairIsIgnoredPolicy as Policy, ABadDonedataPairIsIgnoredState as State,
};

#[test]
fn the_bad_pairs_are_dropped_and_the_done_events_still_arrive() {
    // The handlers record with `<assign>`, so the policy takes an engine.
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let mut e = sce_rust_runtime::Engine::new(Policy::new(script_engine));
    // The run needs nothing from the host.
    e.initialize();

    let p = e.policy();
    let seen = format!(
        "errors={:?} shape={:?} (wanted 2 / 1)",
        p.errors(),
        p.shape()
    );
    assert_eq!(
        e.terminal_state(),
        Some(State::Done),
        "done.state.p must still arrive and carry the run to `done`. {seen}"
    );
    assert_eq!(
        p.errors(),
        Some(2),
        "each ignored pair raises its own error.execution. {seen}"
    );
    assert_eq!(
        p.shape(),
        Some(1),
        "done.state.r1 must carry the surviving pair and neither ignored one. {seen}"
    );
}
