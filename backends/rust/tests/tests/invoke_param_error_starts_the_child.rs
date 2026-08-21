// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.7.1 under 6.4 — Rust AOT.
//
// A `<param>` of an `<invoke>` whose expression will not evaluate is the one
// place two clauses meet: §scxml-6.4.2 terminates the element when "the
// evaluation of its arguments produces an error", and §scxml-5.7.1 says a
// failing `<param>` costs `error.execution` and "MUST ignore the name and
// value" — then delegates only the SUCCESSFUL name and value to the context,
// naming `<donedata>`, `<send>` and `<invoke>` in that sentence.
//
// 5.7.1 governs: it has already said what a failed `<param>` costs, in this
// context by name, and reading 6.4.2 over it would leave "ignore the name and
// value" with no session for the name to be absent from. W3C test343 settles
// the same clause from the `<donedata>` side; no IRP document asks it of
// `<invoke>`, which is why this fixture exists.
//
// Fixture: integration_resources/invoke_param_error_starts_the_child/invoke_param_error_starts_the_child.scxml
// (canonical, shared with the other channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_invoke_param_error_starts_the_child.sh

use std::time::Duration;

use sce_rust_tests::integration::invoke_param_error_starts_the_child::{
    InvokeParamErrorStartsTheChildPolicy, InvokeParamErrorStartsTheChildState,
};

#[test]
fn an_invoke_param_that_will_not_evaluate_costs_its_pair_and_nothing_else() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = InvokeParamErrorStartsTheChildPolicy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();

    let completed = engine.run_until_completion(Duration::from_secs(10), Duration::from_millis(10));
    assert!(
        completed,
        "invoke_param_error_starts_the_child timed out before reaching a final \
         state — even the `timeout` that judges a never-started child never \
         fired, so the machine is not being ticked"
    );

    match engine.get_current_state() {
        InvokeParamErrorStartsTheChildState::Pass => {}
        InvokeParamErrorStartsTheChildState::FailNoParamError => panic!(
            "`childUp` arrived with no `error.execution` before it — W3C SCXML \
             5.7.1 puts that error on the internal queue while the `<invoke>` is \
             being evaluated, so it is dequeued before the child's first word"
        ),
        InvokeParamErrorStartsTheChildState::FailInvokeNotStarted => panic!(
            "the child never started — this channel read W3C SCXML 6.4.2's \
             \"terminate the processing of the element\" over 5.7.1's per-item \
             rule. 5.7.1 handles the failure itself and delegates only the \
             successful name and value to the context, so one `<param>` that \
             will not evaluate costs its own pair, not the session"
        ),
        InvokeParamErrorStartsTheChildState::FailGoodParamLost => panic!(
            "the child's `kept` did not arrive as 'here' — W3C SCXML 6.4.3 seeds \
             the child's matching `<data>` from the param's value, and one \
             sibling that failed does not cost the others"
        ),
        InvokeParamErrorStartsTheChildState::FailBrokenParamSeeded => panic!(
            "the child found the empty string under `broken` — 5.7.1 says ignore \
             the name AND the value, so the child must find its own declaration \
             untouched rather than a placeholder the author never wrote"
        ),
        other => panic!(
            "invoke_param_error_starts_the_child settled in {other:?}, which is \
             not a verdict state"
        ),
    }
}
