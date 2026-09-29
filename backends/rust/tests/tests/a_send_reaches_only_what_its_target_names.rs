// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2 + 6.4 + C.1: a <send> reaches what its target names, carries
// its payload there, and a target that names nothing reachable is reported —
// Rust AOT path.
//
// Fixture: integration_resources/a_send_reaches_only_what_its_target_names/a_send_reaches_only_what_its_target_names.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_a_send_reaches_only_what_its_target_names.sh

use std::time::Duration;

use sce_rust_tests::integration::a_send_reaches_only_what_its_target_names::{
    ASendReachesOnlyWhatItsTargetNamesPolicy as Policy,
    ASendReachesOnlyWhatItsTargetNamesState as State,
};

#[test]
fn a_send_reaches_only_what_its_target_names() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let mut e = sce_rust_runtime::Engine::new(Policy::new(script_engine));
    e.initialize();
    let completed = e.run_until_completion(Duration::from_secs(2), Duration::from_millis(10));

    assert!(
        completed,
        "the machine never completed (parked in {:?})",
        e.get_current_state()
    );
    assert_eq!(
        e.terminal_state(),
        Some(State::Done),
        "the run must end in `done`"
    );
    let p = e.policy();
    let observed = [
        ("execErrors", p.exec_errors(), 1),
        ("commErrors", p.comm_errors(), 3),
        ("afterRefused", p.after_refused(), 0),
        ("afterNobody", p.after_nobody(), 0),
        ("afterStranger", p.after_stranger(), 0),
        ("afterOrphan", p.after_orphan(), 0),
        ("bareArrived", p.bare_arrived(), 1),
        ("pongOk", p.pong_ok(), 1),
    ];
    for (name, got, want) in observed {
        assert_eq!(got, Some(want), "{name} = {got:?}, want {want}");
    }
}
