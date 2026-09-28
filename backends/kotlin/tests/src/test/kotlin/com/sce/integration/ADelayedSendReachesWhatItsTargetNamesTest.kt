// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2 + 6.4 + C.1: a delay postpones a <send>, it does not change
// where the send goes — Kotlin AOT path.
//
// Fixture: integration_resources/a_delayed_send_reaches_what_its_target_names/a_delayed_send_reaches_what_its_target_names.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python channels).
//
// Regeneration: scripts/regen_a_delayed_send_reaches_what_its_target_names_kotlin.sh

package com.sce.integration

import com.sce.integration.a_delayed_send_reaches_what_its_target_names.ADelayedSendReachesWhatItsTargetNamesState
import com.sce.integration.a_delayed_send_reaches_what_its_target_names.ADelayedSendReachesWhatItsTargetNamesStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("ADelayedSendReachesWhatItsTargetNames — W3C SCXML 6.2")
class ADelayedSendReachesWhatItsTargetNamesTest {

    @Test
    fun aDelayedSendReachesWhatItsTargetNames() {
        val sm = ADelayedSendReachesWhatItsTargetNamesStateMachine(W3CTestBase.createEngine())
        sm.initialize()

        val deadline = System.currentTimeMillis() + 3000L
        while (!sm.isInFinalState && System.currentTimeMillis() < deadline) {
            sm.tick()
            Thread.sleep(5)
        }

        assertTrue(sm.isInFinalState, "the machine never completed (parked in ${sm.currentState.value})")
        assertEquals(ADelayedSendReachesWhatItsTargetNamesState.Done, sm.terminalState, "the run must end in `done`")
        val observed = mapOf(
            "order" to (sm.order() to 31L),
            "innerInternal" to (sm.innerInternal() to 1L),
            "lateOk" to (sm.lateOk() to 1L),
            "lateCount" to (sm.lateCount() to 1L),
            "pongOk" to (sm.pongOk() to 1L),
            "commErrors" to (sm.commErrors() to 2L),
            "lostArrived" to (sm.lostArrived() to 0L),
            "afterStranger" to (sm.afterStranger() to 0L),
        )
        val wrong = observed.filterValues { (got, want) -> got != want }
        assertEquals(emptyMap<String, Pair<Long?, Long>>(), wrong, "observed (got to want)")
    }
}
