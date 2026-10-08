// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-C-2: the <param>s of a BasicHTTP <send> of a datamodel="sce-static"
// machine are read from its own fields when the send runs, and cross as the text
// a form carries (docs/adr/0005, decision 4) — Kotlin path. The request is
// observed where the engine hands it to its transport, so no listener is involved.
//
// Fixture: sce-build/tests/fixtures/static_datamodel/static_send_http.scxml
//
// Regeneration: scripts/regen_static_datamodel_kotlin.sh

package com.sce.integration

import com.sce.integration.static_send_http.StaticSendHttpStateMachine
import com.sce.runtime.StateMachineEngine
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("AStaticSendOverHttpCarriesItsParams — §scxml-C-2")
class AStaticSendOverHttpCarriesItsParamsTest {

    /** A machine whose transport keeps what it was handed, run to completion. */
    private fun withMachine(
        body: (StaticSendHttpStateMachine, List<StateMachineEngine.HttpSendRequest>) -> Unit
    ) {
        val sm = StaticSendHttpStateMachine()
        val posted = mutableListOf<StateMachineEngine.HttpSendRequest>()
        sm.onHttpSend = { request -> posted.add(request) }
        sm.initialize()
        try {
            body(sm, posted)
        } finally {
            sm.cleanup()
        }
    }

    private fun StaticSendHttpStateMachine.raise(name: String) {
        sendEventByName(name)
        tick()
    }

    private fun pairs(vararg list: Pair<String, String>): Map<String, List<String>> =
        list.associate { (name, text) -> name to listOf(text) }

    @Test
    fun aSendOverHttpCarriesTheTextTheFieldsHoldWhenItRuns() = withMachine { sm, posted ->
        sm.raise("bump")
        sm.raise("go")

        assertEquals(1, posted.size, "one request is handed to the transport")
        val request = posted[0]
        assertEquals("http://example.invalid/hook", request.target)
        assertEquals("note", request.eventName)
        assertEquals("", request.content, "no <content>, so the body is the pairs")
        assertEquals(
            pairs(
                "count" to "4",
                "ready" to "true",
                "label" to "busy",
                "twice" to "8",
                "delta" to "-5",
                "ratio" to "1.5",
            ),
            request.params,
            "each value is the text it spells: an integer's digits, `true`, the string, " +
                "a negative number, a real's String()"
        )
    }

    @Test
    fun thePairsAreTheFieldsAsTheyStandAndNotACopyFromStartUp() = withMachine { sm, posted ->
        sm.raise("go")

        assertEquals(1, posted.size)
        assertEquals(
            pairs(
                "count" to "3",
                "ready" to "false",
                "label" to "idle",
                "twice" to "6",
                "delta" to "-5",
                "ratio" to "1.5",
            ),
            posted[0].params,
            "without `bump` the fields hold their initial values, and the request carries " +
                "those: it is read when the send runs"
        )
    }

    @Test
    fun aParamThatCannotBeReadIsLeftOutAndTheRequestStillGoes() = withMachine { sm, posted ->
        sm.raise("bump")
        sm.raise("boom")

        assertEquals(1, posted.size, "the request goes with the pair that could be read")
        assertEquals(
            pairs("count" to "4"),
            posted[0].params,
            "`big` is `count * 2000000000`, which a 32-bit field cannot hold: its pair is " +
                "left out, not carried as a zero"
        )
        assertEquals(1u, sm.errors, "§scxml-5.7.1: the failed pair is reported as error.execution, once")
    }
}
