// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix D exitInterpreter: a run ends by exiting every state it
// is still in, the way exitStates exits one — Kotlin AOT path.
//
// Reached two ways, and both are driven here: the run enters a top-level
// `<final>` after a step, or the host stops it. Measured 2026-09-26, this
// channel already exited the final when the run ended there, but ran no
// `<onexit>` on stop(), destroyed the script session with it (so nothing the
// handlers wrote could be read back), and forgot where an ended run had
// ended as soon as it was stopped.
//
// Fixture: integration_resources/the_run_ends_by_exiting_every_state/the_run_ends_by_exiting_every_state.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python channels).
//
// Regeneration: scripts/regen_the_run_ends_by_exiting_every_state_kotlin.sh

package com.sce.integration

import com.sce.integration.the_run_ends_by_exiting_every_state.TheRunEndsByExitingEveryStateEvent
import com.sce.integration.the_run_ends_by_exiting_every_state.TheRunEndsByExitingEveryStateState
import com.sce.integration.the_run_ends_by_exiting_every_state.TheRunEndsByExitingEveryStateStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("TheRunEndsByExitingEveryState — W3C SCXML Appendix D exitInterpreter")
class TheRunEndsByExitingEveryStateTest {

    private fun started(): TheRunEndsByExitingEveryStateStateMachine {
        // The handlers record with `<assign>`, so this is an ECMAScript-datamodel
        // machine.
        val sm = TheRunEndsByExitingEveryStateStateMachine(W3CTestBase.createEngine())
        sm.initialize()
        assertEquals(
            TheRunEndsByExitingEveryStateState.Inner,
            sm.currentState.value,
            "the run has to start inside `inner`",
        )
        return sm
    }

    /** What the run left behind (W3C SCXML 5.3 readers), so a failure says what happened. */
    private fun describe(sm: TheRunEndsByExitingEveryStateStateMachine): String =
        "active: ${sm.activeConfiguration}, ended in ${sm.terminalState}, order=${sm.order()} " +
            "finalExits=${sm.finalExits()} selfInFinal=${sm.selfInFinal()}"

    @Test
    fun aRunThatReachesItsFinalExitsTheFinal() {
        val sm = started()
        sm.send(TheRunEndsByExitingEveryStateEvent.Finish)
        sm.tick()

        val seen = describe(sm)
        assertEquals(TheRunEndsByExitingEveryStateState.Done, sm.terminalState, seen)
        assertTrue(
            sm.activeConfiguration.isEmpty(),
            "exitInterpreter deletes every state it exits, the final included. $seen",
        )
        assertEquals(1L, sm.finalExits(), "the final's own <onexit> must run exactly once as the run ends. $seen")
        assertEquals(1L, sm.selfInFinal(), "`done` must still be in the configuration during its own <onexit>. $seen")
        assertEquals(12L, sm.order(), "`finish` exits `inner` then `outer`. $seen")
    }

    @Test
    fun aStoppedRunExitsEveryStateInnermostFirst() {
        val sm = started()
        sm.stop()

        val seen = describe(sm)
        assertNull(sm.terminalState, "a stopped run did not end in a final. $seen")
        assertTrue(sm.activeConfiguration.isEmpty(), seen)
        assertEquals(
            12L,
            sm.order(),
            "stop() must run `inner`'s <onexit> and then `outer`'s: 0 is a stop that exited nothing, " +
                "21 one that exited in document order instead of exit order. $seen",
        )
        assertEquals(0L, sm.finalExits(), "the run never entered `done`. $seen")
    }
}
