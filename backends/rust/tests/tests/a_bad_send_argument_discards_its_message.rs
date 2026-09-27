// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2 + 4.9: a <send> whose own argument cannot be evaluated raises
// error.execution, discards the message, and ends its block — Rust AOT path.
//
// Fixture: integration_resources/a_bad_send_argument_discards_its_message/a_bad_send_argument_discards_its_message.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_a_bad_send_argument_discards_its_message.sh

use sce_rust_runtime::SceClock;
use sce_rust_tests::integration::a_bad_send_argument_discards_its_message::{
    ABadSendArgumentDiscardsItsMessageEvent as Event,
    ABadSendArgumentDiscardsItsMessagePolicy as Policy,
    ABadSendArgumentDiscardsItsMessageState as State,
};

/// On a manual clock advanced a full minute before `finish`: a channel that
/// scheduled the message whose `delayexpr` failed, under some default delay,
/// delivers it within that minute and moves `sent`.
#[test]
fn each_bad_argument_discards_its_message() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let mut e = sce_rust_runtime::Engine::new(Policy::new(script_engine));
    e.set_clock(SceClock::Manual(0));
    e.initialize();
    e.advance_time_ms(60_000);
    e.raise_external(Event::Finish, "", "");
    e.step();

    assert_eq!(
        e.terminal_state(),
        Some(State::Done),
        "`finish` must carry the run to `done`"
    );
    let p = e.policy();
    let observed = [
        ("errors", p.errors(), 6),
        ("sent", p.sent(), 0),
        ("after", p.after(), 0),
    ];
    for (name, got, want) in observed {
        assert_eq!(got, Some(want), "{name} = {got:?}, want {want}");
    }
}
