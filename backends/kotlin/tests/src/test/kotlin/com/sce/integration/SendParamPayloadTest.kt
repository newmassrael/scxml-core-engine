// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2 `<send>` `<param>` payload delivery — Kotlin AOT.
//
// Two paths that were fixed at the template layer with no runtime witness,
// because no committed fixture had a machine of the required shape. The 215
// Kotlin cases could only show that nothing regressed; that same absence is
// why the defects survived as long as they did.
//
//   engine-less child -> parent   Kotlin gated param emission on the
//     *machine* needing a script engine rather than on the send needing one,
//     so a `datamodel="null"` child emitted only its HTTP shape and shipped
//     `<send target="#_parent">` with the params dropped.
//
//   #_internal                    the internal raise took no event data, so
//     params were built and then discarded — on every backend.
//
// The two reach distinct final states, so a failure names the path.
//
// Fixture: integration_resources/send_param_payload/send_param_payload.scxml
// (canonical, shared with the Rust / Go / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_send_param_payload_kotlin.sh

package com.sce.integration

import com.sce.integration.send_param_payload.SendParamPayloadState
import com.sce.integration.send_param_payload.SendParamPayloadStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML 6.2 — `<param>` payload survives both send paths (Kotlin AOT).
@DisplayName("SendParamPayload — W3C SCXML 6.2")
class SendParamPayloadTest {

    @Test
    fun sendParamsReachEventDataFromChildAndInternalQueue() {
        val sm = SendParamPayloadStateMachine(W3CTestBase.createEngine())
        sm.initialize()

        if (!sm.isInFinalState) {
            val deadline = System.currentTimeMillis() + 2000L
            while (!sm.isInFinalState && System.currentTimeMillis() < deadline) {
                Thread.sleep(10)
                sm.tick()
            }
        }

        val reached = sm.currentState.value
        val why = when (reached) {
            SendParamPayloadState.FailChildPayload ->
                "`fromChild` arrived without `_event.data.value`: a `datamodel=\"null\"` " +
                    "child needs no script engine, but its `<send>` still has to carry " +
                    "the params it declares. The gate is whether this send folds to " +
                    "literals, not whether the machine needs an engine."
            SendParamPayloadState.FailInternalPayload ->
                "`loopback` arrived without `_event.data.carried`: a " +
                    "`<send target=\"#_internal\">` must raise its params as event data, " +
                    "not build them and drop them at the internal-raise boundary."
            SendParamPayloadState.Pass -> ""
            else ->
                "settled in $reached, which is not a verdict state — neither send was " +
                    "judged, so the machine never got as far as the payload checks."
        }

        assertEquals(SendParamPayloadState.Pass, reached, why)
    }
}
