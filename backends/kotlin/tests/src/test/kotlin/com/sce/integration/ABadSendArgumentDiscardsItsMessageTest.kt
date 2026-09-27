// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2 + 4.9: a <send> whose own argument cannot be evaluated raises
// error.execution, discards the message, and ends its block — Kotlin AOT path.
//
// Fixture: integration_resources/a_bad_send_argument_discards_its_message/a_bad_send_argument_discards_its_message.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python channels).
//
// Regeneration: scripts/regen_a_bad_send_argument_discards_its_message_kotlin.sh

package com.sce.integration

import com.sce.integration.a_bad_send_argument_discards_its_message.ABadSendArgumentDiscardsItsMessageEvent
import com.sce.integration.a_bad_send_argument_discards_its_message.ABadSendArgumentDiscardsItsMessageState
import com.sce.integration.a_bad_send_argument_discards_its_message.ABadSendArgumentDiscardsItsMessageStateMachine
import com.sce.runtime.ManualClock
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("ABadSendArgumentDiscardsItsMessage — W3C SCXML 6.2 + 4.9")
class ABadSendArgumentDiscardsItsMessageTest {

    /// On a manual clock advanced a full minute before `finish`: a channel that
    /// scheduled the message whose `delayexpr` failed, under some default delay,
    /// delivers it within that minute and moves `sent`.
    @Test
    fun eachBadArgumentDiscardsItsMessage() {
        val sm = ABadSendArgumentDiscardsItsMessageStateMachine(W3CTestBase.createEngine())
        sm.clock = ManualClock(0L)
        sm.initialize()
        sm.advanceTimeMs(60000)
        sm.send(ABadSendArgumentDiscardsItsMessageEvent.Finish)
        sm.tick()

        assertEquals(ABadSendArgumentDiscardsItsMessageState.Done, sm.terminalState, "`finish` must carry the run to `done`")
        val observed = mapOf(
            "errors" to (sm.errors() to 6L),
            "sent" to (sm.sent() to 0L),
            "after" to (sm.after() to 0L),
        )
        val wrong = observed.filterValues { (got, want) -> got != want }
        assertEquals(emptyMap<String, Pair<Long?, Long>>(), wrong, "observed (got to want)")
    }
}
