// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2.4 + C.2: a delay is a property of the send, not of the
// processor it names — a delayed BasicHTTP send is POSTed when due, and
// <cancel> reaches it while it waits — Kotlin AOT path.
//
// Fixture: integration_resources/a_delayed_http_send_is_posted_when_due/a_delayed_http_send_is_posted_when_due.scxml
// (canonical, shared with the C++ AOT / Rust / Go / Python channels).
//
// Regeneration: scripts/regen_a_delayed_http_send_is_posted_when_due_kotlin.sh

package com.sce.integration

import com.sce.integration.a_delayed_http_send_is_posted_when_due.ADelayedHttpSendIsPostedWhenDueState
import com.sce.integration.a_delayed_http_send_is_posted_when_due.ADelayedHttpSendIsPostedWhenDueStateMachine
import com.sce.runtime.ManualClock
import com.sce.runtime.StateMachineEngine
import com.sce.w3c.W3CTestBase
import java.util.concurrent.CopyOnWriteArrayList
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("ADelayedHttpSendIsPostedWhenDue — W3C SCXML 6.2.4 + C.2")
class ADelayedHttpSendIsPostedWhenDueTest {

    @Test
    fun aDelayedHttpSendIsPostedWhenDue() {
        val sm = ADelayedHttpSendIsPostedWhenDueStateMachine(W3CTestBase.createEngine())
        // The host stands in for the HTTP transport: it keeps what it was handed.
        val posted = mutableListOf<StateMachineEngine.HttpSendRequest>()
        sm.onHttpSend = { request -> posted.add(request) }
        val events = { posted.map { it.eventName } }
        sm.clock = ManualClock(0L)
        sm.initialize()
        sm.tick()

        // A zero wait, written or evaluated, is no deferral: the POST is made
        // before initialize() returns, with no tick to bring it out.
        assertEquals(
            listOf("now", "zero", "zeroexpr"), events(),
            "the undelayed send and the two zero-delay sends are POSTed at once"
        )

        sm.advanceTimeMs(99)
        sm.tick()
        assertEquals(listOf("now", "zero", "zeroexpr"), events(), "a send delayed 100ms is not POSTed at 99ms")

        sm.advanceTimeMs(1)
        sm.tick()
        assertEquals(
            listOf("now", "zero", "zeroexpr", "later"), events(),
            "it is POSTed when the delay has elapsed, and the cancelled one never is"
        )
        assertEquals("http://127.0.0.1:18081/later", posted[3].target)
        assertEquals("later", posted[3].sendId)

        sm.advanceTimeMs(100)
        sm.tick()
        assertEquals(listOf("now", "zero", "zeroexpr", "later", "dynamic"), events())
        assertEquals("http://127.0.0.1:18081/dynamic", posted[4].target, "a targetexpr is read when the send is made")
        assertEquals(listOf("v"), posted[4].params["k"])

        sm.advanceTimeMs(100)
        sm.tick()
        assertEquals(ADelayedHttpSendIsPostedWhenDueState.Done, sm.terminalState, "the run must end in `done`")
        assertEquals(listOf("now", "zero", "zeroexpr", "later", "dynamic"), events())
    }

    /**
     * The coroutine mode POSTs a delayed BasicHTTP send when it is due too. Its
     * deadline sits in the same queue in both modes, and this mode's loop
     * performs what is due on its own coroutine; a loop that waited on the
     * event channel alone would never POST it, with nothing to say so. Real
     * time, because the engine's coroutine runs on `Dispatchers.Default` and
     * reads the system clock: only what cannot vary with load is asserted —
     * the order, that the cancelled send never goes out, that a send is not
     * POSTed before its delay, and that the run reaches `done` by its own timer.
     */
    @Test
    fun aDelayedHttpSendIsPostedWhenDueInCoroutineMode() = runBlocking {
        val sm = ADelayedHttpSendIsPostedWhenDueStateMachine(W3CTestBase.createEngine())
        val posted = CopyOnWriteArrayList<Pair<StateMachineEngine.HttpSendRequest, Long>>()
        val startedAt = System.nanoTime()
        sm.onHttpSend = { request -> posted.add(request to (System.nanoTime() - startedAt) / 1_000_000L) }
        val endedIn = try {
            // The engine's coroutine is a child of this scope and ends when the
            // machine reaches a final state, so the scope returns when the run
            // is over; the timeout is only what turns a machine that never gets
            // there into a failure rather than a hang.
            withTimeout(10_000) { coroutineScope { sm.start(this) } }
            sm.terminalState
        } finally {
            sm.stop()
        }
        assertEquals(
            listOf("now", "zero", "zeroexpr", "later", "dynamic"), posted.map { it.first.eventName },
            "each is POSTed once and in order, and the cancelled one never is"
        )
        // 5 ms under the fixture's 100 ms: the engine's clock counts whole
        // milliseconds, so a send armed a fraction into one is due a fraction early
        // on this side's finer reading.
        assertTrue(posted[3].second >= 95L, "`later` was POSTed after ${posted[3].second} ms, before its delay")
        assertEquals(ADelayedHttpSendIsPostedWhenDueState.Done, endedIn, "the run must end in `done`")
    }
}
