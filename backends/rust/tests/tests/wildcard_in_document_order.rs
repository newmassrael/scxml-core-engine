// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A wildcard keeps its own guard and its own type — Rust AOT path.
//
// W3C SCXML 3.12.1 lets a `*` descriptor match every event, and that is all
// it changes: W3C SCXML 3.13 still asks the transition's `cond` whether it is
// enabled, and a `type="internal"` transition whose target is a proper
// descendant of its compound source still does not exit that source.
//
// No W3C document writes a wildcard with a `cond` or with `type="internal"`, so
// a generator that moved the wildcard into a hand-written fallback dropped both
// without a suite noticing — the Kotlin one did until 2026-09-13.
//
// Fixture: integration_resources/wildcard_in_document_order/wildcard_in_document_order.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_wildcard_in_document_order.sh

use std::sync::Arc;
use std::time::Duration;

use sce_rust_runtime::{Engine, IScriptEngine};
use sce_rust_tests::integration::wildcard_in_document_order::{
    WildcardInDocumentOrderPolicy as Policy, WildcardInDocumentOrderState as State,
};

#[test]
fn a_wildcard_keeps_its_guard_and_its_type() {
    let script_engine: Arc<dyn IScriptEngine> = Arc::new(sce_rust_lua::LuaEngine::new());
    let mut engine = Engine::new(Policy::new(script_engine));
    engine.initialize();

    let completed = engine.run_until_completion(Duration::from_secs(2), Duration::from_millis(10));
    assert!(
        completed,
        "the machine never reached a final state (parked in {:?}); a machine resting \
         in `GuardedFrom` or `SealedFrom` means an internal wildcard was not taken at all",
        engine.get_current_state()
    );

    // Each failure final names the case, so this one assertion says which
    // property of the wildcard was lost:
    //   FailGuardIgnored              a wildcard fired with its guard false
    //   FailGuardNeverFired           a wildcard did not fire with its guard true
    //   FailGuardedInternalReentered  a guarded internal wildcard exited its source
    //   FailSealedInternalReentered   an unguarded internal wildcard exited its source
    assert_eq!(
        engine.get_current_state(),
        State::Pass,
        "the machine rested in a failure final: a wildcard is enabled only when its \
         guard is true, and an internal wildcard targeting a descendant of its \
         compound source must not exit and re-enter that source"
    );
}
