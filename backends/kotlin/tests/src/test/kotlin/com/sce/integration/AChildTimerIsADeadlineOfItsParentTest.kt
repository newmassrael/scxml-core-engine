// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A child session's `<send delay>` is a deadline of the machine that invoked
// it — Kotlin AOT path.
//
// A host that drives a scheduler-owning machine asks the engine when it next
// needs a tick and sleeps that long. The parent's tick advances every running
// child by the same step, so a moment a child needs is a moment the host must
// not step over. An answer that counts only the parent's own scheduler tells a
// host with nothing of the parent's armed that there is nothing to wait for,
// and the child's timer is never fired.
//
// This backend already counted its children (`timeUntilNextScheduledMs`), and
// this driver is what says so: the other channels that expose the query did
// not, and nothing here could have noticed either way.
//
// Driven entirely on ManualClock: no case sleeps, and each move lands exactly
// on the deadline the engine reported, which is the use the answer exists for.
//
// Fixture: tests/integration/a_child_timer_is_a_deadline_of_its_parent.scxml.
// It is outside integration_resources/ for the reason
// scripts/regen_a_child_timer_is_a_deadline_of_its_parent.sh states.
//
// Regeneration: scripts/regen_a_child_timer_is_a_deadline_of_its_parent_kotlin.sh

package com.sce.integration

import com.sce.integration.a_child_timer_is_a_deadline_of_its_parent.AChildTimerIsADeadlineOfItsParentState
import com.sce.integration.a_child_timer_is_a_deadline_of_its_parent.AChildTimerIsADeadlineOfItsParentStateMachine
import com.sce.runtime.ManualClock
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("AChildTimerIsADeadlineOfItsParent — W3C SCXML 6.4")
class AChildTimerIsADeadlineOfItsParentTest {

    /// The machine on host-owned time. The clock is installed BEFORE
    /// `initialize()`: the child arms its first timer as it starts, and a clock
    /// swapped afterwards would be compared against deadlines armed on the
    /// previous one.
    private fun started(): AChildTimerIsADeadlineOfItsParentStateMachine {
        // No engine argument: the fixture reads nothing from a data model, so
        // codegen emits a machine with no script engine at all.
        val sm = AChildTimerIsADeadlineOfItsParentStateMachine()
        sm.clock = ManualClock(0L)
        sm.initialize()
        return sm
    }

    // The parent arms nothing, so the only deadline in the run is the child's
    // first timer, 200 ms after the child started.
    @Test
    fun theEngineNamesItsChildsFirstTimerAsItsOwnNextDeadline() {
        val sm = started()
        assertEquals(
            AChildTimerIsADeadlineOfItsParentState.Waiting,
            sm.currentState.value,
            "the parent should be waiting on its child",
        )
        assertEquals(
            200L,
            sm.timeUntilNextScheduledMs(),
            "the child armed `<send delay=\"200ms\">` when it started and the parent ticks the child, so that " +
                "deadline is the parent's. An answer of null tells a host there is nothing to wait for while a " +
                "running child's timer is pending",
        )
    }

    // The child arms its second timer when the first fires, so the whole run is
    // two moves of 200 ms and the engine has to name the second one only after
    // the first has been taken.
    @Test
    fun aHostWalkingTimeByTheAnswerReachesTheEndOfTheChild() {
        val sm = started()
        val walked = mutableListOf<Long>()
        while (true) {
            val due = sm.timeUntilNextScheduledMs() ?: break
            assertEquals(true, walked.size < 8, "the engine keeps naming deadlines: $walked")
            walked.add(due)
            sm.advanceTimeMs(due)
        }
        assertEquals(
            listOf(200L, 200L),
            walked,
            "each move should land on the child's next timer, the second of which is armed by the first firing",
        )
        assertEquals(
            AChildTimerIsADeadlineOfItsParentState.Finished,
            sm.terminalState,
            "the child's last timer ended it, so the parent should have taken `done.invoke.kid` and finished",
        )
        assertNull(sm.timeUntilNextScheduledMs(), "nothing is armed once the child has ended")
    }
}
