// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceRuntime
import Testing

@testable import StaticDatamodelMachines

// SCE Accepted Subset §2.13, "Hybrid `<invoke>`": an `<invoke srcexpr>` that declares
// `sce:candidates` starts the document its value names (W3C SCXML 6.4), by the
// document's stem, and hands it the invoke's arguments, each to the variable of the
// same name that candidate declares (W3C SCXML 6.4.3).
//
// `static_invoke_hybrid.scxml` runs four phases, each in a state of its own:
//
//   first    `file:static_hybrid_first.scxml`: handed start = 7 and enabled = true, and
//            ends. `extra` is evaluated and left out, the candidate declaring none.
//   second   an absolute path to `static_hybrid_second.scxml`: handed start = 7 and
//            extra = 3, and ends. `enabled` is left out.
//   lossy    `./static_hybrid_first.scxml` with an `extra` no 32-bit field can hold:
//            reported as `error.execution` and left out, and the child still starts
//            and ends.
//   missing  a document the invoke did not declare: `error.execution`, and no child
//            starts, so no `done.invoke` follows.
//
// A candidate handed what the OTHER one declares would never end, and the run would
// stop short of `over`. There is no scenario file: what the machine does is the sum of
// what its children do, which the Go and Rust halves state the same way.

/// How many rounds a host gives the machine for its children to run and report: a
/// child that ends while it initializes has reported by the time the parent's start
/// returns, so the whole run may be over before the first.
private let hybridRounds = 40

private func settledHybrid() -> (StaticInvokeHybridPolicy, Engine<StaticInvokeHybridPolicy>) {
    let policy = StaticInvokeHybridPolicy()
    let engine = Engine(policy: policy)
    engine.setClock(ManualClock())
    engine.initialize()
    for _ in 0..<hybridRounds {
        engine.tick()
    }
    return (policy, engine)
}

@Test func eachPhaseStartsTheCandidateItsValueNamesAndEnds() {
    let (policy, engine) = settledHybrid()
    // Each phase's `done.invoke` adds a power of ten of its own, so the sum says WHICH
    // candidates ended: `first` (1), `second` (10) and the retry of `first` that carried
    // an argument it could not hold (100).
    #expect(policy.completed == 111)
    // The last phase named no declared candidate, so the run is over.
    #expect(engine.isInFinalState)
}

@Test func anArgumentIsEvaluatedWhateverTheCandidateKeeps() {
    let (policy, _) = settledHybrid()
    // `lossy` hands an `extra` that overflows to a candidate that declares none (one
    // error), and `missing` names no declared candidate (the other).
    #expect(policy.errors == 2)
}

@Test func aValueNamingNoDeclaredCandidateStartsNothing() {
    let (policy, _) = settledHybrid()
    // `done.invoke.missing_run` would add 1000: nothing started to send it.
    #expect(policy.completed < 1000)
}
