// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.13, "Hybrid `<invoke>`" — Kotlin half of the Rust suite's
// `a_static_hybrid_invoke_starts_the_candidate_its_value_names.rs`.
//
// An `<invoke srcexpr>` that declares `sce:candidates` starts the document its
// value names (W3C SCXML 6.4), by the document's stem, and hands it the invoke's
// arguments, each to the variable of the same name that candidate declares (W3C
// SCXML 6.4.3).
//
// `static_invoke_hybrid.scxml` runs four phases, each in a state of its own:
//
//   first    `file:static_hybrid_first.scxml`: handed start = 7 and enabled =
//            true, and ends. `extra` is evaluated and left out, the candidate
//            declaring none.
//   second   an absolute path to `static_hybrid_second.scxml`: handed start = 7
//            and extra = 3, and ends. `enabled` is left out.
//   lossy    `./static_hybrid_first.scxml` with an `extra` no 32-bit field can
//            hold: reported as `error.execution` and left out, and the child
//            still starts and ends.
//   missing  a document the invoke did not declare: `error.execution`, and no
//            child starts, so no `done.invoke` follows.
//
// A candidate handed what the OTHER one declares would never end, and the run
// would stop short of `over`.

package com.sce.integration

import com.sce.integration.static_invoke_hybrid.StaticInvokeHybridStateMachine
import com.sce.runtime.ManualClock
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class AStaticHybridInvokeStartsTheCandidateItsValueNamesTest {

    /**
     * Start the machine and let its children run and report: a child that ends
     * during its own `initialize` has reported by the time the parent's start
     * returns, so the whole run may be over before the first tick.
     */
    private fun settled(): StaticInvokeHybridStateMachine {
        val sm = StaticInvokeHybridStateMachine()
        sm.clock = ManualClock(0)
        sm.initialize()
        repeat(40) { sm.tick() }
        return sm
    }

    @Test
    fun eachPhaseStartsTheCandidateItsValueNamesAndEnds() {
        val sm = settled()
        try {
            // Each phase's `done.invoke` adds a power of ten of its own, so the sum
            // says WHICH candidates ended: `first` (1), `second` (10) and the retry
            // of `first` that carried an argument it could not hold (100).
            assertEquals(111u, sm.completed)
            assertTrue(
                sm.isInFinalState,
                "the last phase named no declared candidate, so the run is over",
            )
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun anArgumentIsEvaluatedWhateverTheCandidateKeeps() {
        val sm = settled()
        try {
            // `lossy` hands an `extra` that overflows to a candidate that declares
            // none (one error), and `missing` names no declared candidate (the
            // other).
            assertEquals(2u, sm.errors)
        } finally {
            sm.cleanup()
        }
    }

    @Test
    fun aValueNamingNoDeclaredCandidateStartsNothing() {
        val sm = settled()
        try {
            // `done.invoke.missing_run` would add 1000: nothing started to send it.
            assertTrue(sm.completed < 1000u, "completed is ${sm.completed}")
        } finally {
            sm.cleanup()
        }
    }
}
