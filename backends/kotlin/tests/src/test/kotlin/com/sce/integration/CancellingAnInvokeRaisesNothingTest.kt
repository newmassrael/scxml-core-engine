// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4: cancelling an invocation raises no event in the invoking
// session — Kotlin AOT path.
//
// Measured 2026-09-26, this channel already raised nothing; the fixture pins
// it.
//
// Fixture: integration_resources/cancelling_an_invoke_raises_nothing/cancelling_an_invoke_raises_nothing.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python channels).
//
// Regeneration: scripts/regen_cancelling_an_invoke_raises_nothing_kotlin.sh

package com.sce.integration

import com.sce.integration.cancelling_an_invoke_raises_nothing.CancellingAnInvokeRaisesNothingEvent
import com.sce.integration.cancelling_an_invoke_raises_nothing.CancellingAnInvokeRaisesNothingState
import com.sce.integration.cancelling_an_invoke_raises_nothing.CancellingAnInvokeRaisesNothingStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("CancellingAnInvokeRaisesNothing — W3C SCXML 6.4")
class CancellingAnInvokeRaisesNothingTest {

    @Test
    fun leavingTheInvokingStateRaisesNoCancelEvent() {
        val sm = CancellingAnInvokeRaisesNothingStateMachine(W3CTestBase.createEngine())
        sm.initialize()
        assertEquals(
            CancellingAnInvokeRaisesNothingState.P,
            sm.currentState.value,
            "the run has to start in `p`, with its child invoked",
        )

        for (event in listOf(
            CancellingAnInvokeRaisesNothingEvent.Leave,
            CancellingAnInvokeRaisesNothingEvent.Finish,
        )) {
            sm.send(event)
            sm.tick()
        }

        assertEquals(CancellingAnInvokeRaisesNothingState.Done, sm.terminalState, "`finish` must carry the run to `done`")
        assertEquals(
            0L,
            sm.spurious(),
            "leaving `p` cancelled its child, and a `cancel.invoke` event reached the invoking session; " +
                "W3C SCXML 6.4 defines no such event",
        )
    }
}
