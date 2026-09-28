// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.6.2 + 6.2: a <send>'s <content expr> is evaluated when the send
// is, and its value is the event's data — Rust AOT path.
//
// Fixture: integration_resources/a_send_content_expr_is_the_payload/a_send_content_expr_is_the_payload.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_a_send_content_expr_is_the_payload.sh

use std::time::Duration;

use sce_rust_tests::integration::a_send_content_expr_is_the_payload::{
    ASendContentExprIsThePayloadPolicy as Policy, ASendContentExprIsThePayloadState as State,
};

#[test]
fn a_send_content_expr_is_the_payload() {
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
        ("numberOk", p.number_ok(), 1),
        ("objectOk", p.object_ok(), 1),
        ("textOk", p.text_ok(), 1),
        ("errors", p.errors(), 1),
        ("badArrived", p.bad_arrived(), 1),
        ("badEmpty", p.bad_empty(), 1),
        ("afterBad", p.after_bad(), 0),
    ];
    for (name, got, want) in observed {
        assert_eq!(got, Some(want), "{name} = {got:?}, want {want}");
    }
}
