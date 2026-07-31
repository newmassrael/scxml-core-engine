// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4 autoforward field preservation — Rust AOT local-invoke path.
//
// W3C §6.4 requires the parent to forward an exact copy of every external
// event to an `<invoke autoforward="true">` child. The public IRP suite never
// checks the copy's contents: test229 only asserts the event name crosses, and
// test230 is a manual test whose field comparison is done by a human reading
// two log dumps. A forward stripped down to the bare event name passes both.
//
// Fixture: integration_resources/autoforward_event_fields/autoforward_event_fields.scxml
// (canonical, shared with the C++ / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_autoforward_event_fields.sh

use std::time::Duration;

use sce_rust_tests::integration::autoforward_event_fields::{
    AutoforwardEventFieldsPolicy, AutoforwardEventFieldsState,
};

#[test]
fn forwarded_copy_keeps_data_origin_and_invokeid() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = AutoforwardEventFieldsPolicy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();

    let completed = engine.run_until_completion(Duration::from_secs(2), Duration::from_millis(10));
    assert!(
        completed,
        "autoforward_event_fields timed out before reaching a final state — the \
         child never received the forwarded `childToParent`, so no \
         done.invoke.inv_echo was emitted"
    );

    assert_eq!(
        engine.get_current_state(),
        AutoforwardEventFieldsState::Pass,
        "the child reported `stripped`: the autoforwarded copy of `childToParent` lost \
         `_event.data.value`, `_event.origin` or `_event.invokeid`. W3C §6.4 requires an \
         exact copy — forward the source event's metadata, not just its name"
    );
}
