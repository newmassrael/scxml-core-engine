// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML C.1: an event a session sends to itself names its origin, and a
// target expression that evaluates to nothing reaches no one — Kotlin AOT path.
//
// Fixture: integration_resources/a_self_sent_event_names_its_origin/a_self_sent_event_names_its_origin.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python channels).
//
// Regeneration: scripts/regen_a_self_sent_event_names_its_origin_kotlin.sh

package com.sce.integration

import com.sce.integration.a_self_sent_event_names_its_origin.ASelfSentEventNamesItsOriginState
import com.sce.integration.a_self_sent_event_names_its_origin.ASelfSentEventNamesItsOriginStateMachine
import com.sce.runtime.ManualClock
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("ASelfSentEventNamesItsOrigin — W3C SCXML C.1")
class ASelfSentEventNamesItsOriginTest {

    @Test
    fun aSelfSentEventNamesItsOrigin() {
        val sm = ASelfSentEventNamesItsOriginStateMachine(W3CTestBase.createEngine())
        sm.clock = ManualClock(0L)
        sm.initialize()
        sm.advanceTimeMs(1000)
        sm.tick()

        assertEquals(ASelfSentEventNamesItsOriginState.Done, sm.terminalState, "the run must end in `done`")
        val observed = mapOf(
            "immediateOk" to (sm.immediateOk() to 1L),
            "replied" to (sm.replied() to 1L),
            "delayedOk" to (sm.delayedOk() to 1L),
            "unreachable" to (sm.unreachable() to 1L),
            "strayed" to (sm.strayed() to 0L),
        )
        val wrong = observed.filterValues { (got, want) -> got != want }
        assertEquals(emptyMap<String, Pair<Long?, Long>>(), wrong, "observed (got to want)")
    }
}
