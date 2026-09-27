// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4: every event an invoked child sends to `#_parent` reaches
// the parent, however many it sends in one tick — Rust AOT path.
//
// Fixture: integration_resources/a_child_may_send_many_events_in_one_tick/a_child_may_send_many_events_in_one_tick.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_a_child_may_send_many_events_in_one_tick.sh

use sce_rust_tests::integration::a_child_may_send_many_events_in_one_tick::{
    AChildMaySendManyEventsInOneTickEvent as Event,
    AChildMaySendManyEventsInOneTickPolicy as Policy,
    AChildMaySendManyEventsInOneTickState as State,
};

#[test]
fn every_event_the_child_sent_arrives() {
    // The handler records with `<assign>`, so the policy takes an engine.
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let mut e = sce_rust_runtime::Engine::new(Policy::new(script_engine));
    e.initialize();
    e.raise_external(Event::Finish, "", "");
    e.step();

    let p = e.policy();
    let seen = format!("ticks={:?} (wanted 110)", p.ticks());
    assert_eq!(
        e.terminal_state(),
        Some(State::Done),
        "`finish` must carry the run to `done`. {seen}"
    );
    assert_eq!(
        p.ticks(),
        Some(110),
        "every event the child sent arrives before `finish`. {seen}"
    );
}
