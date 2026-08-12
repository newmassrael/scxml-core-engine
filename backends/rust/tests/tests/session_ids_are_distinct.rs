// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10: `_sessionid` is the id of a session — Rust AOT.
//
// The clause binds `_sessionid` to "the system-generated id for the current
// SCXML session", and Appendix C.1.1 derives the address a session publishes
// from that id. Two live sessions holding one id therefore publish one
// address, and a `<send>` addressed to either reaches both.
//
// No test in the public IRP corpus can ask: every one of them that reaches
// `_sessionid` runs a single session, so a processor that hands the same
// value to every session it starts passes them all. The C11 backend did
// exactly that until this fixture was added.
//
// The fixture runs two children at once, each reporting the id it was
// issued, and the parent compares them. Reused id lands in `fail`; a second
// report that never arrives leaves the parent parked and this driver times
// out, which is the honest signal for a child that was never started.
//
// Fixture: integration_resources/session_ids_are_distinct/session_ids_are_distinct.scxml
// (canonical, shared with every other channel).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_session_ids_are_distinct.sh

use std::time::Duration;

use sce_rust_tests::integration::session_ids_are_distinct::{
    SessionIdsAreDistinctPolicy, SessionIdsAreDistinctState,
};

#[test]
fn two_live_sessions_are_issued_different_ids() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = SessionIdsAreDistinctPolicy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();

    let completed = engine.run_until_completion(Duration::from_secs(2), Duration::from_millis(10));
    assert!(
        completed,
        "session_ids_are_distinct timed out parked in {:?}: only one child reported its `_sessionid`, so the ids were never compared",
        engine.get_current_state()
    );

    match engine.get_current_state() {
        SessionIdsAreDistinctState::Pass => {}
        SessionIdsAreDistinctState::Fail => panic!("two live sessions reported the same `_sessionid`. W3C SCXML 5.10 binds it to the id of the current session, and C.1.1 publishes an address derived from it, so one id for two sessions is one address for two sessions"),
        other => panic!("session_ids_are_distinct settled in {other:?}, which is not a verdict state"),
    }
}
