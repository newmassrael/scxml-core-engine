// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2: a <send> delay is read as one CSS2 time — Rust AOT path.
//
// Fixture: integration_resources/a_delay_is_a_css2_time/a_delay_is_a_css2_time.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_a_delay_is_a_css2_time.sh

use sce_rust_runtime::SceClock;
use sce_rust_tests::integration::a_delay_is_a_css2_time::{
    ADelayIsACss2TimePolicy as Policy, ADelayIsACss2TimeState as State,
};

/// On a manual clock advanced a full minute: both valid delays fire in the
/// order their milliseconds give, and a refused message, had it been scheduled
/// under some default wait, would have arrived and moved `bad`.
#[test]
fn each_delay_is_read_as_one_css2_time() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let mut e = sce_rust_runtime::Engine::new(Policy::new(script_engine));
    e.set_clock(SceClock::Manual(0));
    e.initialize();
    e.advance_time_ms(60_000);
    e.step();

    assert_eq!(
        e.terminal_state(),
        Some(State::Done),
        "`a` must carry the run to `done`"
    );
    let p = e.policy();
    let observed = [
        ("errors", p.errors(), 2),
        ("after", p.after(), 0),
        ("bad", p.bad(), 0),
        ("aAfterB", p.a_after_b(), 1),
    ];
    for (name, got, want) in observed {
        assert_eq!(got, Some(want), "{name} = {got:?}, want {want}");
    }
}
