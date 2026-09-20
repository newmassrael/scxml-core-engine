// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4.3: an `<invoke>` naming its target through an expression
// evaluates that expression at invoke-fire time, and a failure to evaluate
// raises `error.execution` — Rust AOT path.
//
// The clause puts two obligations on the Processor and this fixture is about
// the second only. The evaluated string does not select the child on this
// path: codegen fixes the child at build time and writes an
// immediate-`<final>` stub for it (docs/SCE_ACCEPTED_SUBSET.md §2.13). A
// fixture built on the value would therefore measure nothing here. One built
// on the FAILURE measures what every channel still owes: evaluate, and say so
// when you cannot.
//
// `target` holds null and the expression indexes it, which is a runtime error
// in every expression runtime SCE lowers to. An undeclared bare identifier
// would be refused at build time instead and never reach the clause.
//
// Fixture: integration_resources/invoke_expression_failure_is_reported/invoke_expression_failure_is_reported.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_invoke_expression_failure_is_reported.sh

use std::time::Duration;

use sce_rust_tests::integration::invoke_expression_failure_is_reported::{
    InvokeExpressionFailureIsReportedPolicy, InvokeExpressionFailureIsReportedState,
};

#[test]
fn an_invoke_expression_that_cannot_be_evaluated_raises_error_execution() {
    // Engine DI Parity RFC (Path B+): the fixture declares `<data>` and its
    // `<invoke>` names an expression, so the policy needs an engine to
    // evaluate both. Constructed per test rather than taken from a global.
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = InvokeExpressionFailureIsReportedPolicy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();

    let completed = engine.run_until_completion(Duration::from_secs(2), Duration::from_millis(10));

    assert!(
        completed,
        "the machine never completed (parked in {:?}). W3C SCXML 6.4.3 requires \
         the expression to be evaluated when the `<invoke>` fires; parking means \
         neither the raise nor the child arrived",
        engine.get_current_state()
    );
    assert_eq!(
        engine.get_current_state(),
        InvokeExpressionFailureIsReportedState::Pass,
        "reaching `fail` means the child started on an expression that cannot be \
         evaluated, so nothing evaluated it"
    );
}
