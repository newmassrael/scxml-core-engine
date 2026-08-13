// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.4: every region of a `<parallel>` takes its own enabled
// transition in the same microstep — Kotlin AOT path.
//
// The fixture is asymmetric on purpose. One region's transition on the event
// is an external self-transition, whose domain Appendix D resolves through
// `findLCCA` over the proper ancestors — candidates that never include the
// state itself. Answering with the state left the exit-set walk without a
// stopping point, so it ran to the document root, the exit set named the
// enclosing `<parallel>`, and conflict resolution preempted the deeper
// region's transition on that same event.
//
// The observable is `settled`, which the document reaches only when both
// regions' assignments have run — a configuration check alone would still
// pass for a region that moved without executing its transition content.
//
// Fixture: integration_resources/parallel_regions_take_own_transitions/parallel_regions_take_own_transitions.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_parallel_regions_take_own_transitions_kotlin.sh

package com.sce.integration

import com.sce.integration.parallel_regions_take_own_transitions.ParallelRegionsTakeOwnTransitionsEvent
import com.sce.integration.parallel_regions_take_own_transitions.ParallelRegionsTakeOwnTransitionsState
import com.sce.integration.parallel_regions_take_own_transitions.ParallelRegionsTakeOwnTransitionsStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML 3.4 — a self-transition must not preempt a sibling region.
@DisplayName("ParallelRegionsTakeOwnTransitions — W3C SCXML 3.4")
class ParallelRegionsTakeOwnTransitionsTest {

    @Test
    fun everyRegionTakesItsOwnTransition() {
        // The fixture's `<assign>`s make this an ECMAScript-datamodel machine.
        val sm = ParallelRegionsTakeOwnTransitionsStateMachine(W3CTestBase.createEngine())
        sm.initialize()

        sm.send(ParallelRegionsTakeOwnTransitionsEvent.E)
        sm.tick()
        sm.send(ParallelRegionsTakeOwnTransitionsEvent.Check)
        sm.tick()

        // `settled` is a top-level `<final>` guarded on `n == 1 && m == 1`, so
        // reaching it says both regions took their transition on `e` AND ran
        // its content. A deeper region that was preempted never enters
        // `judging`, so `check` finds no enabled transition and the machine
        // stays inside the parallel.
        assertEquals(
            ParallelRegionsTakeOwnTransitionsState.Settled,
            sm.currentState.value,
            "`check` did not carry the machine to `settled`, which the document guards " +
                "on both regions' assignments having run",
        )
    }
}
