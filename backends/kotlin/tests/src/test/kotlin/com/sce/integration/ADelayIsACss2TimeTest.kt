// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2: a <send> delay is read as one CSS2 time — Kotlin AOT path.
//
// Fixture: integration_resources/a_delay_is_a_css2_time/a_delay_is_a_css2_time.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python channels).
//
// Regeneration: scripts/regen_a_delay_is_a_css2_time_kotlin.sh

package com.sce.integration

import com.sce.integration.a_delay_is_a_css2_time.ADelayIsACss2TimeState
import com.sce.integration.a_delay_is_a_css2_time.ADelayIsACss2TimeStateMachine
import com.sce.runtime.ManualClock
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("ADelayIsACss2Time — W3C SCXML 6.2")
class ADelayIsACss2TimeTest {

    /// On a manual clock advanced a full minute: both valid delays fire in the
    /// order their milliseconds give, and a refused message, had it been
    /// scheduled under some default wait, would have arrived and moved `bad`.
    @Test
    fun eachDelayIsReadAsOneCss2Time() {
        val sm = ADelayIsACss2TimeStateMachine(W3CTestBase.createEngine())
        sm.clock = ManualClock(0L)
        sm.initialize()
        sm.advanceTimeMs(60000)
        sm.tick()

        assertEquals(ADelayIsACss2TimeState.Done, sm.terminalState, "`a` must carry the run to `done`")
        val observed = mapOf(
            "errors" to (sm.errors() to 2L),
            "after" to (sm.after() to 0L),
            "bad" to (sm.bad() to 0L),
            "aAfterB" to (sm.aAfterB() to 1L),
        )
        val wrong = observed.filterValues { (got, want) -> got != want }
        assertEquals(emptyMap<String, Pair<Long?, Long>>(), wrong, "observed (got to want)")
    }
}
