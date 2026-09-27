// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4: a reply an invoked child has already sent is on the
// parent's external queue, so a host that only hands the machine events
// still sees it ahead of its own later events — Kotlin AOT path.
//
// Fixture: integration_resources/a_child_reply_arrives_without_a_tick/a_child_reply_arrives_without_a_tick.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python channels).
//
// Regeneration: scripts/regen_a_child_reply_arrives_without_a_tick_kotlin.sh

package com.sce.integration

import com.sce.integration.a_child_reply_arrives_without_a_tick.AChildReplyArrivesWithoutATickEvent
import com.sce.integration.a_child_reply_arrives_without_a_tick.AChildReplyArrivesWithoutATickState
import com.sce.integration.a_child_reply_arrives_without_a_tick.AChildReplyArrivesWithoutATickStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("AChildReplyArrivesWithoutATick — W3C SCXML 6.4")
class AChildReplyArrivesWithoutATickTest {

    @Test
    fun theChildsReplyArrivesBeforeTheHostsNextEvent() {
        val sm = AChildReplyArrivesWithoutATickStateMachine(W3CTestBase.createEngine())
        sm.initialize()
        sm.send(AChildReplyArrivesWithoutATickEvent.Finish)
        sm.tick()

        val seen = "hellos=${sm.hellos()} (wanted 1)"
        assertEquals(AChildReplyArrivesWithoutATickState.Done, sm.terminalState, "`finish` must carry the run to `done`. $seen")
        assertEquals(1L, sm.hellos(), "the child's start-time reply arrives before `finish`. $seen")
    }
}
