// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.13, "Hybrid `<invoke>`": an `<invoke srcexpr>` that
// declares `sce:candidates` starts the document its value names (W3C SCXML
// 6.4), by the document's stem, and hands it the invoke's arguments, each to
// the variable of the same name that candidate declares (W3C SCXML 6.4.3).
//
// `static_invoke_hybrid.scxml` runs four phases, each in a state of its own:
//
//   first    `file:static_hybrid_first.scxml`: handed start = 7 and
//            enabled = true, and ends. `extra` is evaluated and left out, the
//            candidate declaring none.
//   second   an absolute path to `static_hybrid_second.scxml`: handed
//            start = 7 and extra = 3, and ends. `enabled` is left out.
//   lossy    `./static_hybrid_first.scxml` with an `extra` no 32-bit field can
//            hold: reported as `error.execution` and left out, and the child
//            still starts and ends.
//   missing  a document the invoke did not declare: `error.execution`, and no
//            child starts, so no `done.invoke` follows.
//
// A candidate handed what the OTHER one declares would never end, and the run
// would stop short of `over`.

use sce_rust_runtime::{Engine, SceClock};
use sce_rust_tests::integration::static_datamodel::static_invoke_hybrid_sm::StaticInvokeHybridPolicy;

fn started() -> Engine<StaticInvokeHybridPolicy> {
    let mut engine = Engine::new(StaticInvokeHybridPolicy::new());
    engine.set_clock(SceClock::Manual(0));
    engine.initialize();
    engine
}

/// Let the children run and report to their parent. A child that ends during
/// its own `initialize` has reported by the time the parent's start returns,
/// so the whole run may already be over before the first tick.
fn settled() -> Engine<StaticInvokeHybridPolicy> {
    let mut engine = started();
    for _ in 0..40 {
        engine.tick();
    }
    engine
}

#[test]
fn each_phase_starts_the_candidate_its_value_names_and_ends() {
    let engine = settled();
    // Each phase's `done.invoke` adds a power of ten of its own, so the sum
    // says WHICH candidates ended: `first` (1), `second` (10) and the retry of
    // `first` that carried an argument it could not hold (100).
    assert_eq!(engine.policy().completed(), 111);
    assert!(
        engine.is_in_final_state(),
        "the last phase named no declared candidate, so the run is over"
    );
}

#[test]
fn an_argument_is_evaluated_whatever_the_candidate_keeps() {
    let engine = settled();
    // `lossy` hands an `extra` that overflows to a candidate that declares none
    // (one error), and `missing` names no declared candidate (the other).
    assert_eq!(engine.policy().errors(), 2);
}

#[test]
fn a_value_naming_no_declared_candidate_starts_nothing() {
    let engine = settled();
    // `done.invoke.missing_run` would add 1000: nothing started to send it.
    assert!(engine.policy().completed() < 1000);
}
