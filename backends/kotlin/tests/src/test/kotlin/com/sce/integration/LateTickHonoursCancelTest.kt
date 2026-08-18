// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2 + 6.3: a `<cancel>` still lands when the host ticked late —
// Kotlin AOT path (sync mode).
//
// In sync mode the scheduled sends live in a time-ordered queue that `tick()`
// drains. Draining it to exhaustion before running a macrostep is the defect:
// a host that wakes after two fire times have passed holds both entries, and
// queueing both makes the second undroppable before the first one's
// transitions have run. The `<cancel>` then executes against a queue the event
// has already left.
//
// The host below sleeps past BOTH fire times before its first tick, because
// that is the only condition under which the two dispatch orders differ. A
// host that wakes between them passes either way, which is why every existing
// suite was blind to this.
//
// Fixture: integration_resources/late_tick_honours_cancel/late_tick_honours_cancel.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_late_tick_honours_cancel_kotlin.sh

package com.sce.integration

import com.sce.integration.late_tick_honours_cancel.LateTickHonoursCancelState
import com.sce.integration.late_tick_honours_cancel.LateTickHonoursCancelStateMachine
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNotEquals
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML 6.2/6.3 — a late tick still honours `<cancel>` (Kotlin AOT).
@DisplayName("LateTickHonoursCancel — W3C SCXML 6.2/6.3")
class LateTickHonoursCancelTest {

    /** Past both `<send delay>`s in `waiting` (100 ms and 200 ms), with margin. */
    private val pastBothDeadlines = 400L

    private fun started(): LateTickHonoursCancelStateMachine {
        val sm = LateTickHonoursCancelStateMachine()
        sm.initialize()
        return sm
    }

    @Test
    fun theFixtureIsSchedulerDriven() {
        assertTrue(
            started().needsEventScheduler,
            "the fixture arms two delayed <send>s; a machine that does not declare " +
                "needsEventScheduler means the document lost them, and every assertion " +
                "below would then be measuring the wrong machine"
        )
    }

    @Test
    fun aCancelSurvivesATickThatArrivesAfterBothDeadlines() {
        val sm = started()
        assertEquals(
            LateTickHonoursCancelState.Waiting,
            sm.currentState.value,
            "the machine should be waiting on its two delayed sends"
        )

        Thread.sleep(pastBothDeadlines)
        sm.tick()

        assertNotEquals(
            LateTickHonoursCancelState.CancelLost,
            sm.currentState.value,
            "`settle` was delivered even though `active`'s <cancel sendid=\"s1\"> ran " +
                "first. Both entries were past due when this tick started, so the " +
                "scheduler drain queued them together and the cancel found nothing " +
                "left to drop. W3C SCXML 6.3 cancels a send that has not been " +
                "dispatched — dispatch is one entry per macrostep, not one " +
                "queue-flush per tick."
        )

        // The verdict is itself scheduler-driven, so a channel whose tick loop
        // stopped working fails here rather than passing by never moving.
        val deadline = System.currentTimeMillis() + 2000L
        while (!sm.isInFinalState && System.currentTimeMillis() < deadline) {
            sm.tick()
            Thread.sleep(20L)
        }
        assertEquals(
            LateTickHonoursCancelState.Pass,
            sm.currentState.value,
            "the machine did not reach `pass` after the cancel"
        )
        sm.cleanup()
    }

    @Test
    fun aPunctualHostReachesTheSameVerdict() {
        val sm = started()
        val deadline = System.currentTimeMillis() + 2000L
        while (!sm.isInFinalState && System.currentTimeMillis() < deadline) {
            sm.tick()
            Thread.sleep(10L)
        }
        assertEquals(
            LateTickHonoursCancelState.Pass,
            sm.currentState.value,
            "a 10 ms tick loop, which wakes between the 100 ms and 200 ms deadlines, " +
                "must reach `pass`"
        )
        sm.cleanup()
    }

    @Test
    fun theEngineSaysWhenItIsNextDue() {
        val sm = started()

        val due = sm.timeUntilNextScheduledMs()
        assertTrue(
            due != null && due <= 100L,
            "the nearer of the two armed sends is 100 ms out; the engine answered " +
                "$due, which would send a host past the earlier deadline"
        )
        // The lower bound is the half that catches an answer of "due now", which
        // reads as a working query and costs the caller a spin that never sleeps.
        assertTrue(
            due != null && due > 0L,
            "the nearer send is 100 ms out and nothing is due yet, but the engine " +
                "answered $due. A host sleeping on that answer does not sleep at all"
        )

        // Drive by the engine's own answer: every wait lands on a fire time, so
        // no wait can straddle two of them.
        val deadline = System.currentTimeMillis() + 3000L
        while (!sm.isInFinalState && System.currentTimeMillis() < deadline) {
            val wait = sm.timeUntilNextScheduledMs() ?: 5L
            Thread.sleep(maxOf(wait, 1L))
            sm.tick()
        }
        assertEquals(
            LateTickHonoursCancelState.Pass,
            sm.currentState.value,
            "deadline-driven ticking did not reach `pass`"
        )
        assertNull(
            sm.timeUntilNextScheduledMs(),
            "nothing is scheduled once the machine is finished, so no wake-up is owed"
        )
        sm.cleanup()
    }
}
