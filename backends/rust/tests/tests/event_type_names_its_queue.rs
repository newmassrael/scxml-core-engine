// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10.1: `_event.type` names the queue an event was taken from —
// Rust AOT path.
//
// The document queues an external event first and two internal ones after
// it. Measured 2026-09-26, this channel set an "external" flag when an event
// was ENQUEUED and consumed it on whichever event was bound next, so `int`
// was typed "external" and `ext` "internal".
//
// Fixture: integration_resources/event_type_names_its_queue/event_type_names_its_queue.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_event_type_names_its_queue.sh

use sce_rust_tests::integration::event_type_names_its_queue::{
    EventTypeNamesItsQueuePolicy as Policy, EventTypeNamesItsQueueState as State,
};

#[test]
fn each_event_is_typed_by_the_queue_it_was_taken_from() {
    // The handlers record with `<assign>`, so the policy takes an engine.
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let mut e = sce_rust_runtime::Engine::new(Policy::new(script_engine));
    // The document queues its own events; the run needs nothing from the host.
    e.initialize();

    let p = e.policy();
    let seen = format!(
        "intCode={:?} sendCode={:?} extCode={:?} (1 internal, 2 external, 3 other; wanted 1 / 1 / 2)",
        p.int_code(),
        p.send_code(),
        p.ext_code()
    );
    assert_eq!(
        e.terminal_state(),
        Some(State::Done),
        "`ext` must carry the run to `done`. {seen}"
    );
    assert_eq!(
        p.int_code(),
        Some(1),
        "`int` came off the internal queue while `ext` waited on the external one. {seen}"
    );
    assert_eq!(
        p.send_code(),
        Some(1),
        "a `<send target=\"#_internal\">` with a payload rides the internal queue too. {seen}"
    );
    assert_eq!(
        p.ext_code(),
        Some(2),
        "`ext` came off the external queue. {seen}"
    );
}
