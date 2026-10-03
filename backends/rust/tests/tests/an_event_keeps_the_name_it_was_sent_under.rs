// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The name an event arrives under (§scxml-5.10, §scxml-3.12.1) — Rust AOT
// local-invoke path.
//
// A transition on `request` takes `request.new` by whole-token matching, and
// `_event.name` is then the name the event was sent under, not the descriptor it
// was matched through. The public IRP suite never reads a name the document does
// not write, so a machine that is told the shorter one passes all of it.
//
// Fixture: integration_resources/an_event_keeps_the_name_it_was_sent_under/an_event_keeps_the_name_it_was_sent_under.scxml
// (canonical, shared with the C++ / Go / Kotlin / Python / C11 channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_an_event_keeps_the_name_it_was_sent_under.sh

use std::time::Duration;

use sce_rust_tests::integration::an_event_keeps_the_name_it_was_sent_under::{
    AnEventKeepsTheNameItWasSentUnderPolicy, AnEventKeepsTheNameItWasSentUnderState,
};

#[test]
fn a_name_the_document_does_not_write_is_told_whole() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = AnEventKeepsTheNameItWasSentUnderPolicy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();

    let completed = engine.run_until_completion(Duration::from_secs(2), Duration::from_millis(10));
    assert!(
        completed,
        "an_event_keeps_the_name_it_was_sent_under timed out before reaching a final \
         state — the child never heard `request.new`, so it never answered"
    );

    assert_eq!(
        engine.terminal_state(),
        Some(AnEventKeepsTheNameItWasSentUnderState::Pass),
        "the child reported `arrivedShortened`: `request.new` took the transition on \
         `request` but `_event.name` told the child `request`. W3C §5.10 makes the name \
         the one the event was sent under, and §3.12.1 only decides which transition \
         takes it"
    );
}
