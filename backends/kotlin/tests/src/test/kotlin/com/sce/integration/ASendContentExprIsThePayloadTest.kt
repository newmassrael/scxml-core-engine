// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.6.2 + 6.2: a <send>'s <content expr> is evaluated when the send
// is, and its value is the event's data — Kotlin AOT path.
//
// Fixture: integration_resources/a_send_content_expr_is_the_payload/a_send_content_expr_is_the_payload.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python channels).
//
// Regeneration: scripts/regen_a_send_content_expr_is_the_payload_kotlin.sh

package com.sce.integration

import com.sce.integration.a_send_content_expr_is_the_payload.ASendContentExprIsThePayloadState
import com.sce.integration.a_send_content_expr_is_the_payload.ASendContentExprIsThePayloadStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("ASendContentExprIsThePayload — W3C SCXML 5.6.2")
class ASendContentExprIsThePayloadTest {

    @Test
    fun aSendContentExprIsThePayload() {
        val sm = ASendContentExprIsThePayloadStateMachine(W3CTestBase.createEngine())
        sm.initialize()

        val deadline = System.currentTimeMillis() + 2000L
        while (!sm.isInFinalState && System.currentTimeMillis() < deadline) {
            sm.tick()
            Thread.sleep(10)
        }

        assertTrue(sm.isInFinalState, "the machine never completed (parked in ${sm.currentState.value})")
        assertEquals(ASendContentExprIsThePayloadState.Done, sm.terminalState, "the run must end in `done`")
        val observed = mapOf(
            "numberOk" to (sm.numberOk() to 1L),
            "objectOk" to (sm.objectOk() to 1L),
            "textOk" to (sm.textOk() to 1L),
            "errors" to (sm.errors() to 1L),
            "badArrived" to (sm.badArrived() to 1L),
            "badEmpty" to (sm.badEmpty() to 1L),
            "afterBad" to (sm.afterBad() to 0L),
        )
        val wrong = observed.filterValues { (got, want) -> got != want }
        assertEquals(emptyMap<String, Pair<Long?, Long>>(), wrong, "observed (got to want)")
    }
}
