// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.7.1 + 4.9: a <send> whose <param> cannot be read still sends
// its message without that pair, and the error ends its block; a valid
// location param is sent — Rust AOT path.
//
// Measured 2026-09-27, this channel let the rest of the block run and
// silently omitted a `location=""` pair without an error.
//
// Fixture: integration_resources/a_bad_send_param_ends_its_block/a_bad_send_param_ends_its_block.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_a_bad_send_param_ends_its_block.sh

use sce_rust_tests::integration::a_bad_send_param_ends_its_block::{
    ABadSendParamEndsItsBlockEvent as Event, ABadSendParamEndsItsBlockPolicy as Policy,
    ABadSendParamEndsItsBlockState as State,
};

#[test]
fn the_message_goes_and_the_block_stops() {
    // The handlers record with `<assign>`, so the policy takes an engine.
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let mut e = sce_rust_runtime::Engine::new(Policy::new(script_engine));
    e.initialize();
    e.raise_external(Event::Finish, "", "");
    e.step();

    let p = e.policy();
    let seen = format!(
        "errors={:?} partials={:?} bares={:?} after={:?} carried={:?} (wanted 3 / 2 / 1 / 0 / 1)",
        p.errors(),
        p.partials(),
        p.bares(),
        p.after(),
        p.carried()
    );
    assert_eq!(
        e.terminal_state(),
        Some(State::Done),
        "`finish` must carry the run to `done`. {seen}"
    );
    assert_eq!(
        p.errors(),
        Some(3),
        "each unreadable <param> raises one error.execution. {seen}"
    );
    assert_eq!(
        p.partials(),
        Some(2),
        "both internal sends go, carrying the good pair without the bad one. {seen}"
    );
    assert_eq!(
        p.bares(),
        Some(1),
        "the external send goes with its empty pair left out. {seen}"
    );
    assert_eq!(
        p.after(),
        Some(0),
        "the <param> error ends the block, so nothing after the <send> runs. {seen}"
    );
    assert_eq!(
        p.carried(),
        Some(1),
        "a valid location param is sent with its value. {seen}"
    );
}
