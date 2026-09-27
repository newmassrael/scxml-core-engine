// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.7.1 + 4.9: a <send> whose <param> cannot be read still sends
// its message without that pair, and the error ends its block; a valid
// location param is sent — Kotlin AOT path.
//
// Measured 2026-09-27, this channel let the rest of the block run and never
// read a `location`: a valid one vanished from the event.
//
// Fixture: integration_resources/a_bad_send_param_ends_its_block/a_bad_send_param_ends_its_block.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python channels).
//
// Regeneration: scripts/regen_a_bad_send_param_ends_its_block_kotlin.sh

package com.sce.integration

import com.sce.integration.a_bad_send_param_ends_its_block.ABadSendParamEndsItsBlockEvent
import com.sce.integration.a_bad_send_param_ends_its_block.ABadSendParamEndsItsBlockState
import com.sce.integration.a_bad_send_param_ends_its_block.ABadSendParamEndsItsBlockStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("ABadSendParamEndsItsBlock — W3C SCXML 5.7.1 + 4.9")
class ABadSendParamEndsItsBlockTest {

    @Test
    fun theMessageGoesAndTheBlockStops() {
        val sm = ABadSendParamEndsItsBlockStateMachine(W3CTestBase.createEngine())
        sm.initialize()
        sm.send(ABadSendParamEndsItsBlockEvent.Finish)
        sm.tick()

        val seen = "errors=${sm.errors()} partials=${sm.partials()} bares=${sm.bares()} " +
            "after=${sm.after()} carried=${sm.carried()} (wanted 3 / 2 / 1 / 0 / 1)"
        assertEquals(ABadSendParamEndsItsBlockState.Done, sm.terminalState, "`finish` must carry the run to `done`. $seen")
        assertEquals(3L, sm.errors(), "each unreadable <param> raises one error.execution. $seen")
        assertEquals(2L, sm.partials(), "both internal sends go, carrying the good pair without the bad one. $seen")
        assertEquals(1L, sm.bares(), "the external send goes with its empty pair left out. $seen")
        assertEquals(0L, sm.after(), "the <param> error ends the block, so nothing after the <send> runs. $seen")
        assertEquals(1L, sm.carried(), "a valid location param is sent with its value. $seen")
    }
}
