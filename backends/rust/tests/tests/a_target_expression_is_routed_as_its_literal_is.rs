// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2.4 + C.1: a `targetexpr` is routed as the same value written in
// `target` is, at once or after a delay — Rust AOT path.
//
// Fixture: integration_resources/a_target_expression_is_routed_as_its_literal_is/a_target_expression_is_routed_as_its_literal_is.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_a_target_expression_is_routed_as_its_literal_is.sh

use std::time::Duration;

use sce_rust_tests::integration::a_target_expression_is_routed_as_its_literal_is::{
    ATargetExpressionIsRoutedAsItsLiteralIsPolicy as Policy,
    ATargetExpressionIsRoutedAsItsLiteralIsState as State,
};

#[test]
fn a_target_expression_is_routed_as_its_literal_is() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let mut e = sce_rust_runtime::Engine::new(Policy::new(script_engine));
    e.initialize();
    let completed = e.run_until_completion(Duration::from_secs(3), Duration::from_millis(5));

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
        ("internalNow", p.internal_now(), 1),
        ("internalLater", p.internal_later(), 1),
        ("kidNow", p.kid_now(), 1),
        ("kidLater", p.kid_later(), 1),
        ("sessNow", p.sess_now(), 1),
        ("sessLater", p.sess_later(), 1),
        ("commErrors", p.comm_errors(), 4),
        ("execErrors", p.exec_errors(), 2),
        ("afterStranger", p.after_stranger(), 0),
        ("afterStrangerLater", p.after_stranger_later(), 0),
        ("afterOrphan", p.after_orphan(), 0),
        ("afterOrphanLater", p.after_orphan_later(), 0),
        ("afterBogus", p.after_bogus(), 0),
        ("afterBogusLater", p.after_bogus_later(), 0),
    ];
    let wrong: Vec<_> = observed
        .iter()
        .filter(|(_, got, want)| *got != Some(*want))
        .collect();
    assert!(wrong.is_empty(), "observed (name, got, want): {wrong:?}");
}
