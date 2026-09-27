// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10 + 6.2: a <send>'s payload is the data of the event it sends,
// and a data-less event dequeued before it carries none — Rust AOT path.
//
// Fixture: integration_resources/a_payload_rides_on_its_own_event/a_payload_rides_on_its_own_event.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_a_payload_rides_on_its_own_event.sh

use sce_rust_tests::integration::a_payload_rides_on_its_own_event::{
    APayloadRidesOnItsOwnEventEvent as Event, APayloadRidesOnItsOwnEventPolicy as Policy,
    APayloadRidesOnItsOwnEventState as State,
};

#[test]
fn each_payload_arrives_on_its_own_event() {
    // The handlers record with `<assign>`, so the policy takes an engine.
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let mut e = sce_rust_runtime::Engine::new(Policy::new(script_engine));
    e.initialize();
    e.raise_external(Event::Finish, "", "");
    e.step();

    let p = e.policy();
    let seen = format!(
        "got={:?} stolen={:?} plains={:?} (wanted 4 / 0 / 4)",
        p.got(),
        p.stolen(),
        p.plains()
    );
    assert_eq!(
        e.terminal_state(),
        Some(State::Done),
        "`finish` must carry the run to `done`. {seen}"
    );
    assert_eq!(
        p.got(),
        Some(4),
        "each payload event arrives carrying its own payload. {seen}"
    );
    assert_eq!(
        p.stolen(),
        Some(0),
        "no data-less event arrives carrying a payload. {seen}"
    );
    assert_eq!(
        p.plains(),
        Some(4),
        "every data-less event arrives, and arrives empty. {seen}"
    );
}
