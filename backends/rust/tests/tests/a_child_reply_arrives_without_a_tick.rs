// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4: a reply an invoked child has already sent is on the
// parent's external queue, so a host that only hands the machine events
// still sees it ahead of its own later events — Rust AOT path.
//
// Fixture: integration_resources/a_child_reply_arrives_without_a_tick/a_child_reply_arrives_without_a_tick.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_a_child_reply_arrives_without_a_tick.sh

use sce_rust_tests::integration::a_child_reply_arrives_without_a_tick::{
    AChildReplyArrivesWithoutATickEvent as Event, AChildReplyArrivesWithoutATickPolicy as Policy,
    AChildReplyArrivesWithoutATickState as State,
};

#[test]
fn the_childs_reply_arrives_before_the_hosts_next_event() {
    // The handler records with `<assign>`, so the policy takes an engine.
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let mut e = sce_rust_runtime::Engine::new(Policy::new(script_engine));
    e.initialize();
    e.raise_external(Event::Finish, "", "");
    e.step();

    let p = e.policy();
    let seen = format!("hellos={:?} (wanted 1)", p.hellos());
    assert_eq!(
        e.terminal_state(),
        Some(State::Done),
        "`finish` must carry the run to `done`. {seen}"
    );
    assert_eq!(
        p.hellos(),
        Some(1),
        "the child's start-time reply arrives before `finish`. {seen}"
    );
}
