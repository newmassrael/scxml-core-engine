// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML C.1: an event a session sends to itself names its origin, and a
// target expression that evaluates to nothing reaches no one — Rust AOT path.
//
// Fixture: integration_resources/a_self_sent_event_names_its_origin/a_self_sent_event_names_its_origin.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_a_self_sent_event_names_its_origin.sh

use sce_rust_runtime::SceClock;
use sce_rust_tests::integration::a_self_sent_event_names_its_origin::{
    ASelfSentEventNamesItsOriginPolicy as Policy, ASelfSentEventNamesItsOriginState as State,
};

/// On a manual clock advanced past the delayed send: each self-sent event
/// carried this session's location, the reply to it arrived, and the blank
/// target raised error.communication and delivered nothing.
#[test]
fn a_self_sent_event_names_its_origin() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let mut e = sce_rust_runtime::Engine::new(Policy::new(script_engine));
    e.set_clock(SceClock::Manual(0));
    e.initialize();
    e.advance_time_ms(1_000);
    e.step();

    assert_eq!(
        e.terminal_state(),
        Some(State::Done),
        "the run must end in `done`"
    );
    let p = e.policy();
    let observed = [
        ("immediateOk", p.immediate_ok(), 1),
        ("replied", p.replied(), 1),
        ("delayedOk", p.delayed_ok(), 1),
        ("unreachable", p.unreachable(), 1),
        ("strayed", p.strayed(), 0),
    ];
    for (name, got, want) in observed {
        assert_eq!(got, Some(want), "{name} = {got:?}, want {want}");
    }
}
