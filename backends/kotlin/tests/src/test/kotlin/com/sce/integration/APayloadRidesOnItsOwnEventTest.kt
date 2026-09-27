// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10 + 6.2: a <send>'s payload is the data of the event it sends,
// and a data-less event dequeued before it carries none — Kotlin AOT path.
//
// Fixture: integration_resources/a_payload_rides_on_its_own_event/a_payload_rides_on_its_own_event.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python channels).
//
// Regeneration: scripts/regen_a_payload_rides_on_its_own_event_kotlin.sh

package com.sce.integration

import com.sce.integration.a_payload_rides_on_its_own_event.APayloadRidesOnItsOwnEventEvent
import com.sce.integration.a_payload_rides_on_its_own_event.APayloadRidesOnItsOwnEventState
import com.sce.integration.a_payload_rides_on_its_own_event.APayloadRidesOnItsOwnEventStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("APayloadRidesOnItsOwnEvent — W3C SCXML 5.10 + 6.2")
class APayloadRidesOnItsOwnEventTest {

    @Test
    fun eachPayloadArrivesOnItsOwnEvent() {
        val sm = APayloadRidesOnItsOwnEventStateMachine(W3CTestBase.createEngine())
        sm.initialize()
        sm.send(APayloadRidesOnItsOwnEventEvent.Finish)
        sm.tick()

        val seen = "got=${sm.got()} stolen=${sm.stolen()} plains=${sm.plains()} (wanted 4 / 0 / 4)"
        assertEquals(APayloadRidesOnItsOwnEventState.Done, sm.terminalState, "`finish` must carry the run to `done`. $seen")
        assertEquals(4L, sm.got(), "each payload event arrives carrying its own payload. $seen")
        assertEquals(0L, sm.stolen(), "no data-less event arrives carrying a payload. $seen")
        assertEquals(4L, sm.plains(), "every data-less event arrives, and arrives empty. $seen")
    }
}
