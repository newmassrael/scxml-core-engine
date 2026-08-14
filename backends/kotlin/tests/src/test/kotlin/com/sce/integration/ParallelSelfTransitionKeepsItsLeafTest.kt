// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.4: a region that took an external self-transition still holds an
// atomic state, so it answers the next event too — Kotlin AOT path.
//
// Its sibling `parallel_regions_take_own_transitions` owns the microstep axis.
// This one owns what that axis cannot reach: a region can take its transition
// and run its content exactly as required and still be left holding no leaf,
// and the only thing that tells you so is a later event it fails to answer.
//
// Measured 2026-08-14 on the C++ AOT channel, where the defect lived: with the
// mutation `parallel_microstep_owns_exit_and_entry.cases` restoring it, the
// sibling fixture's driver stayed green and this fixture's went red.
//
// Fixture: integration_resources/parallel_self_transition_keeps_its_leaf/parallel_self_transition_keeps_its_leaf.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_parallel_self_transition_keeps_its_leaf_kotlin.sh

package com.sce.integration

import com.sce.integration.parallel_self_transition_keeps_its_leaf.ParallelSelfTransitionKeepsItsLeafEvent
import com.sce.integration.parallel_self_transition_keeps_its_leaf.ParallelSelfTransitionKeepsItsLeafState
import com.sce.integration.parallel_self_transition_keeps_its_leaf.ParallelSelfTransitionKeepsItsLeafStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML 3.4 — a self-transitioned region must still be able to answer.
@DisplayName("ParallelSelfTransitionKeepsItsLeaf — W3C SCXML 3.4")
class ParallelSelfTransitionKeepsItsLeafTest {

    @Test
    fun theSelfTransitionedRegionAnswersTheNextEvent() {
        // The fixture's `<assign>`s make this an ECMAScript-datamodel machine.
        val sm = ParallelSelfTransitionKeepsItsLeafStateMachine(W3CTestBase.createEngine())
        sm.initialize()

        // Two `e`s, then the verdict. The second one is the point: `judging`
        // has no `e` transition, so nothing but the self-transitioning region
        // can answer it, and it can only answer from a leaf.
        sm.send(ParallelSelfTransitionKeepsItsLeafEvent.E)
        sm.tick()
        sm.send(ParallelSelfTransitionKeepsItsLeafEvent.E)
        sm.tick()
        sm.send(ParallelSelfTransitionKeepsItsLeafEvent.Check)
        sm.tick()

        // `settled` is a top-level `<final>` guarded on `n == 1 && m == 2`, so
        // reaching it says the region still held `within` when the second `e`
        // arrived AND ran its transition content both times. This channel
        // reports a single current leaf rather than a configuration, which is
        // why the document decides its own verdict.
        assertEquals(
            ParallelSelfTransitionKeepsItsLeafState.Settled,
            sm.currentState.value,
            "`check` did not carry the machine to `settled`, which the document guards on " +
                "`n == 1 && m == 2`; `m` reaches 2 only if the self-transitioning region " +
                "still had a leaf to transition from when the second `e` arrived",
        )
    }
}
