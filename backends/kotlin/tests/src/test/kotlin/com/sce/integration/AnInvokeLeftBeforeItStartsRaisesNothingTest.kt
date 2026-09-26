// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4: an invoke whose state leaves within its macrostep is never
// attempted, so the §6.4.1 error.execution for a type no processor runs is
// never raised — Kotlin AOT path.
//
// Measured 2026-09-26, this channel already raised nothing; the fixture pins
// it.
//
// Fixture: integration_resources/an_invoke_left_before_it_starts_raises_nothing/an_invoke_left_before_it_starts_raises_nothing.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python channels).
//
// Regeneration: scripts/regen_an_invoke_left_before_it_starts_raises_nothing_kotlin.sh

package com.sce.integration

import com.sce.integration.an_invoke_left_before_it_starts_raises_nothing.AnInvokeLeftBeforeItStartsRaisesNothingEvent
import com.sce.integration.an_invoke_left_before_it_starts_raises_nothing.AnInvokeLeftBeforeItStartsRaisesNothingState
import com.sce.integration.an_invoke_left_before_it_starts_raises_nothing.AnInvokeLeftBeforeItStartsRaisesNothingStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("AnInvokeLeftBeforeItStartsRaisesNothing — W3C SCXML 6.4")
class AnInvokeLeftBeforeItStartsRaisesNothingTest {

    @Test
    fun theLeftStatesInvokeIsNeverAttempted() {
        val sm = AnInvokeLeftBeforeItStartsRaisesNothingStateMachine(W3CTestBase.createEngine())
        sm.initialize()
        assertEquals(
            AnInvokeLeftBeforeItStartsRaisesNothingState.S1,
            sm.currentState.value,
            "`s0` leaves on an eventless transition within the first macrostep",
        )

        sm.send(AnInvokeLeftBeforeItStartsRaisesNothingEvent.Finish)
        sm.tick()

        assertEquals(AnInvokeLeftBeforeItStartsRaisesNothingState.Done, sm.terminalState, "`finish` must carry the run to `done`")
        assertEquals(
            0L,
            sm.errors(),
            "`s0` left before its macrostep ended, yet its invoke was attempted and raised error.execution; " +
                "W3C SCXML 6.4 cancels an invoke whose state has left",
        )
    }
}
