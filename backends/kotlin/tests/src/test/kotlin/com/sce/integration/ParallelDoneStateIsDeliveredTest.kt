// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.4 + 3.7: `done.state.<parallel>` is delivered, not merely
// declared — Kotlin AOT path.
//
// The sibling fixture `parallel_completion_raises_done_state` carries no
// listener, deliberately: a transition's `event` attribute is itself a
// registration site, so a listener there would register the completion event
// no matter what the `<final>` walk does and leave that fixture unable to fail
// for the defect it exists to catch. What it proves is that the event is
// DECLARED.
//
// Declared is not delivered. A backend that names the event and never raises
// it — or raises it where nothing selects from — passes there.
//
// This channel exposes the current leaf rather than the whole configuration,
// which is precisely why the fixture's verdict is a TOP-LEVEL `<final>`:
// `Settled` is observable here exactly as it is everywhere else, and it is
// reachable by no route other than the completion event. A run that stalls
// inside the parallel reports `A2` or `B2` instead, which names the failure —
// the regions finished and the event went nowhere.
//
// Fixture: integration_resources/parallel_done_state_is_delivered/parallel_done_state_is_delivered.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_parallel_done_state_is_delivered_kotlin.sh

package com.sce.integration

import com.sce.integration.parallel_done_state_is_delivered.ParallelDoneStateIsDeliveredEvent
import com.sce.integration.parallel_done_state_is_delivered.ParallelDoneStateIsDeliveredState
import com.sce.integration.parallel_done_state_is_delivered.ParallelDoneStateIsDeliveredStateMachine
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML 3.4 + 3.7 — the parallel's completion event reaches a listener.
@DisplayName("ParallelDoneStateIsDelivered — W3C SCXML 3.4 + 3.7")
class ParallelDoneStateIsDeliveredTest {

    @Test
    fun completionCarriesTheMachineToATopLevelFinal() {
        // `datamodel="null"` — this machine needs no script engine, so the
        // generated constructor takes none.
        val sm = ParallelDoneStateIsDeliveredStateMachine()
        sm.initialize()

        sm.send(ParallelDoneStateIsDeliveredEvent.Go)
        sm.tick()

        assertEquals(
            ParallelDoneStateIsDeliveredState.Settled,
            sm.currentState.value,
            "every region reaching its `<final>` completes the parallel, so " +
                "done.state.run had to be raised AND selected — `settled` is " +
                "reachable by nothing else",
        )
    }
}
