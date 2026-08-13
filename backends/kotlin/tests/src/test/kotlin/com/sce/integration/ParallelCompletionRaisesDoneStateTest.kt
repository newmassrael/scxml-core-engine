// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.4 + 3.7: a `<parallel>` completing raises `done.state.<id>` —
// Kotlin AOT path.
//
// A `<parallel>` owns no `<final>` of its own; its finals sit one level down,
// inside the regions. A rule that registers the completion event by walking
// from a `<final>` to its direct parent therefore never reaches the parallel,
// while an emitter that raises it from the grandparent does — which is how the
// C++ and C11 channels ended up naming an enumerator nothing had declared.
//
// Fixture: integration_resources/parallel_completion_raises_done_state/parallel_completion_raises_done_state.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_parallel_completion_raises_done_state_kotlin.sh

package com.sce.integration

import com.sce.integration.parallel_completion_raises_done_state.ParallelCompletionRaisesDoneStateEvent
import com.sce.integration.parallel_completion_raises_done_state.ParallelCompletionRaisesDoneStateState
import com.sce.integration.parallel_completion_raises_done_state.ParallelCompletionRaisesDoneStateStateMachine
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML 3.4 + 3.7 — every region's `<final>` completes the parallel.
@DisplayName("ParallelCompletionRaisesDoneState — W3C SCXML 3.4 + 3.7")
class ParallelCompletionRaisesDoneStateTest {

    @Test
    fun everyRegionFinalCompletesTheParallel() {
        // `datamodel="null"` — this machine needs no script engine, so the
        // generated constructor takes none.
        val sm = ParallelCompletionRaisesDoneStateStateMachine()
        sm.initialize()

        sm.send(ParallelCompletionRaisesDoneStateEvent.Go)
        sm.tick()

        // This channel exposes the current leaf rather than the whole
        // configuration, so the assertion names the region that settles last
        // in document order. A region that lost its leaf cannot be the one
        // reported here, which is what makes this observable rather than
        // merely present.
        assertEquals(
            ParallelCompletionRaisesDoneStateState.B2,
            sm.currentState.value,
            "a region did not reach its `<final>` on `go`",
        )
    }
}
