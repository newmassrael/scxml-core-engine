// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A statechart declared as a transition function answers as its machine does
// (docs/SCE_TRANSITION_FUNCTION.md) — Rust AOT, the nested lifecycle.
//
// The document declares `sce:transition-function="emit"` and reaches every
// rule the function is computed by: transitions held by an ancestor, a
// compound state entered by its `initial` attribute, by an `<initial>` element
// and by its first child, an internal and an external transition, a prefix and
// a wildcard descriptor, a targetless transition and a top-level final. This
// drives the machine to every state by the function's shortest path, sends
// every event there, and holds what the engine does to what the function says,
// pair by pair.
//
// Fixture: integration_resources/nested_lifecycle/nested_lifecycle.scxml
// (canonical, shared with the C++ / Go / Kotlin / Python / C11 channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_nested_lifecycle.sh

use sce_rust_tests::integration::nested_lifecycle::{
    nested_lifecycle_transition_function as tf, NestedLifecyclePolicy,
};
use sce_rust_tests::transition_function_equivalence::{
    expect_every_pair_agrees, TransitionFunction,
};

#[test]
fn the_machine_and_its_transition_function_agree_on_every_pair() {
    let function = TransitionFunction {
        states: &tf::ALL_STATES,
        events: &tf::ALL_EVENTS,
        initial: tf::INITIAL,
        transition: tf::transition,
        is_final: tf::is_final,
        configuration: tf::configuration,
        state_name: tf::State::as_str,
        event_name: tf::Event::as_str,
    };
    let pairs = expect_every_pair_agrees(&function, || {
        let mut engine = sce_rust_runtime::Engine::new(NestedLifecyclePolicy::new());
        engine.initialize();
        engine
    });
    assert_eq!(
        pairs,
        tf::ALL_STATES.len() * tf::ALL_EVENTS.len(),
        "the drive did not check every pair"
    );
}
