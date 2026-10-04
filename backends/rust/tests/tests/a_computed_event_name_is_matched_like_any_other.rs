// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A computed event name is matched like any other (§scxml-3.12.1, §scxml-5.10) —
// Rust AOT.
//
// A `<send eventexpr>` names its event at run time, so the document cannot have
// written the name: it is delivered as the event the document's names resolve it
// to (its own, the longest token prefix of it the document writes, or its
// wildcard), and `_event.name` is the whole name. The document sends six, over
// the external and the internal queue, now and after a delay, and takes each only
// when it is told the whole name.
//
// Fixture: integration_resources/a_computed_event_name_is_matched_like_any_other/a_computed_event_name_is_matched_like_any_other.scxml
// (canonical, shared with the C++ / Go / Kotlin / Python / C11 channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_a_computed_event_name_is_matched_like_any_other.sh

use std::time::Duration;

use sce_rust_tests::integration::a_computed_event_name_is_matched_like_any_other::{
    AComputedEventNameIsMatchedLikeAnyOtherPolicy, AComputedEventNameIsMatchedLikeAnyOtherState,
};

#[test]
fn a_computed_name_is_matched_by_token_and_told_whole() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = AComputedEventNameIsMatchedLikeAnyOtherPolicy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();

    let completed = engine.run_until_completion(Duration::from_secs(2), Duration::from_millis(10));
    assert!(
        completed,
        "a_computed_event_name_is_matched_like_any_other stopped before its final state: a \
         computed name was dropped, or matched but told shorter than the name it was sent \
         under (`request.new`, `other.thing`, `request.again`, `last.one`)"
    );

    assert_eq!(
        engine.terminal_state(),
        Some(AComputedEventNameIsMatchedLikeAnyOtherState::Pass)
    );
}
