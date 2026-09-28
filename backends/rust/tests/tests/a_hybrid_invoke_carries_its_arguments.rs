// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4.1 + 6.4.3: an <invoke> whose child is named by an expression
// carries its arguments as one whose child is fixed does — Rust AOT path.
//
// The value always names `keeper`, and `bare` declares the one name `keeper`
// does not, so a pair seeded by the wrong candidate's declarations is a leak
// `keeper` reports rather than an absence the test has to infer.
//
// Fixture: integration_resources/a_hybrid_invoke_carries_its_arguments/a_hybrid_invoke_carries_its_arguments.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_a_hybrid_invoke_carries_its_arguments.sh

use std::time::Duration;

use sce_rust_tests::integration::a_hybrid_invoke_carries_its_arguments::{
    AHybridInvokeCarriesItsArgumentsPolicy as Policy,
    AHybridInvokeCarriesItsArgumentsState as State,
};

#[test]
fn each_argument_reaches_only_the_child_that_declares_it() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let mut e = sce_rust_runtime::Engine::new(Policy::new(script_engine));
    e.initialize();

    let completed = e.run_until_completion(Duration::from_secs(2), Duration::from_millis(10));

    assert!(
        completed,
        "the machine never completed (parked in {:?}); `refusedPhase` parks when \
         its invoke raised nothing",
        e.get_current_state()
    );
    assert_eq!(
        e.terminal_state(),
        Some(State::Done),
        "`FailWrongChild` means `bare` ran; `FailRefusedChildStarted` means a \
         namelist that cannot be read still started a child"
    );
    let p = e.policy();
    let observed = [
        ("errors", p.errors(), 2),
        ("started", p.started(), 2),
        ("paramsOk", p.params_ok(), 1),
        ("namelistOk", p.namelist_ok(), 1),
    ];
    for (name, got, want) in observed {
        assert_eq!(got, Some(want), "{name} = {got:?}, want {want}");
    }
}
