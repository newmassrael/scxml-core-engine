// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.7.1 + 6.4: what each argument of an <invoke> costs when it
// cannot be read, and what a readable one delivers — Rust AOT path.
//
// Fixture: integration_resources/a_bad_invoke_argument_is_reported_once/a_bad_invoke_argument_is_reported_once.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_a_bad_invoke_argument_is_reported_once.sh

use sce_rust_tests::integration::a_bad_invoke_argument_is_reported_once::{
    ABadInvokeArgumentIsReportedOnceEvent as Event,
    ABadInvokeArgumentIsReportedOncePolicy as Policy,
    ABadInvokeArgumentIsReportedOnceState as State,
};

#[test]
fn each_argument_costs_what_its_clause_says() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let mut e = sce_rust_runtime::Engine::new(Policy::new(script_engine));
    e.initialize();
    for event in [Event::Go, Event::Finish] {
        e.raise_external(event, "", "");
        e.step();
    }

    assert_eq!(
        e.terminal_state(),
        Some(State::Done),
        "`finish` must carry the run to `done`"
    );
    let p = e.policy();
    let observed = [
        ("errors", p.errors(), 5),
        ("started", p.started(), 1),
        ("fromLocOk", p.from_loc_ok(), 1),
        ("emptyLocLeftOut", p.empty_loc_left_out(), 1),
        ("brokenLeftOut", p.broken_left_out(), 1),
    ];
    for (name, got, want) in observed {
        assert_eq!(got, Some(want), "{name} = {got:?}, want {want}");
    }
}
