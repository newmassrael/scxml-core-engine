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
import org.junit.jupiter.api.Assertions.assertEquals
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

        assertEquals(listOf("now"), events(), "only the undelayed send is POSTed at once")

        sm.advanceTimeMs(99)
        sm.tick()
        assertEquals(listOf("now"), events(), "a send delayed 100ms is not POSTed at 99ms")

        sm.advanceTimeMs(1)
        sm.tick()
        assertEquals(
            listOf("now", "later"), events(),
            "it is POSTed when the delay has elapsed, and the cancelled one never is"
        )
        assertEquals("http://127.0.0.1:18081/later", posted[1].target)
        assertEquals("later", posted[1].sendId)

        sm.advanceTimeMs(100)
        sm.tick()
        assertEquals(listOf("now", "later", "dynamic"), events())
        assertEquals("http://127.0.0.1:18081/dynamic", posted[2].target, "a targetexpr is read when the send is made")
        assertEquals(listOf("v"), posted[2].params["k"])

        sm.advanceTimeMs(100)
        sm.tick()
        assertEquals(ADelayedHttpSendIsPostedWhenDueState.Done, sm.terminalState, "the run must end in `done`")
        assertEquals(listOf("now", "later", "dynamic"), events())
    }
}
