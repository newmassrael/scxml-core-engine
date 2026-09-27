// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4: every event an invoked child sends to `#_parent` reaches
// the parent, however many it sends in one tick — Kotlin AOT path.
//
// Fixture: integration_resources/a_child_may_send_many_events_in_one_tick/a_child_may_send_many_events_in_one_tick.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python channels).
//
// Regeneration: scripts/regen_a_child_may_send_many_events_in_one_tick_kotlin.sh

package com.sce.integration

import com.sce.integration.a_child_may_send_many_events_in_one_tick.AChildMaySendManyEventsInOneTickEvent
import com.sce.integration.a_child_may_send_many_events_in_one_tick.AChildMaySendManyEventsInOneTickState
import com.sce.integration.a_child_may_send_many_events_in_one_tick.AChildMaySendManyEventsInOneTickStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("AChildMaySendManyEventsInOneTick — W3C SCXML 6.4")
class AChildMaySendManyEventsInOneTickTest {

    @Test
    fun everyEventTheChildSentArrives() {
        val sm = AChildMaySendManyEventsInOneTickStateMachine(W3CTestBase.createEngine())
        sm.initialize()
        sm.send(AChildMaySendManyEventsInOneTickEvent.Finish)
        sm.tick()

        val seen = "ticks=${sm.ticks()} (wanted 110)"
        assertEquals(AChildMaySendManyEventsInOneTickState.Done, sm.terminalState, "`finish` must carry the run to `done`. $seen")
        assertEquals(110L, sm.ticks(), "every event the child sent arrives before `finish`. $seen")
    }
}
