// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4.3: the value a `srcexpr` computes is the child that runs —
// Rust AOT path, over the candidate set `sce:candidates` declares.
//
// This is the clause's other half. `invoke_expression_failure_is_reported`
// holds the part every channel already owed: that the expression is
// evaluated, and that a failure is reported. What it could not ask is
// whether the VALUE means anything, because on this path the child used to
// be fixed at build time and the same stub answered whatever was computed
// (docs/SCE_ACCEPTED_SUBSET.md §2.13).
//
// The two candidates announce themselves differently, so the fixture can
// tell the right child from the wrong one and from no child at all:
//
//   ran `chosen`   -> `from.chosen`     -> pass
//   ran `other`    -> `from.other`      -> wrongChild
//   loaded nothing -> `error.execution` -> noChild
//   ran a stub     -> no event at all   -> parked in `probe`
//
// Four outcomes, four resting places. The first draft of the fixture sent
// the middle two to one `fail` state, and telling them apart then needed a
// log — which is the defect this whole family is about, one level up.
//
// Fixture: integration_resources/invoke_candidate_selects_the_child/invoke_candidate_selects_the_child.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_invoke_candidate_selects_the_child.sh

use std::time::Duration;

use sce_rust_tests::integration::invoke_candidate_selects_the_child::{
    InvokeCandidateSelectsTheChildPolicy, InvokeCandidateSelectsTheChildState,
};

#[test]
fn the_evaluated_value_selects_which_candidate_runs() {
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let policy = InvokeCandidateSelectsTheChildPolicy::new(script_engine);
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();

    let completed = engine.run_until_completion(Duration::from_secs(2), Duration::from_millis(10));

    assert!(
        completed,
        "the machine never completed (parked in {:?}). Parking means no child \
         spoke: a stub ran, or nothing did",
        engine.get_current_state()
    );
    assert_eq!(
        engine.get_current_state(),
        InvokeCandidateSelectsTheChildState::Pass,
        "`WrongChild` means the value selected the other candidate; `NoChild` \
         means nothing was loaded at all"
    );
}
