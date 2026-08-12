// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10: `_sessionid` is the id of a session - Kotlin AOT.
//
// The clause binds `_sessionid` to the system-generated id for the current
// session, and Appendix C.1.1 derives the address a session publishes from
// that id. Two live sessions holding one id publish one address, so a
// `<send>` addressed to either reaches both. Every corpus test that reaches
// `_sessionid` runs a single session and cannot ask.
//
// Fixture: integration_resources/session_ids_are_distinct/session_ids_are_distinct.scxml
// (canonical, shared with every other channel).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_session_ids_are_distinct_kotlin.sh

package com.sce.integration

import com.sce.integration.session_ids_are_distinct.SessionIdsAreDistinctState
import com.sce.integration.session_ids_are_distinct.SessionIdsAreDistinctStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML 5.10 - two live sessions are issued different ids (Kotlin AOT).
@DisplayName("SessionIdsAreDistinct - W3C SCXML 5.10")
class SessionIdsAreDistinctTest {

    @Test
    fun twoLiveSessionsAreIssuedDifferentIds() {
        val sm = SessionIdsAreDistinctStateMachine(W3CTestBase.createEngine())
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
            SessionIdsAreDistinctState.Fail ->
                "two live sessions reported the same `_sessionid`. The clause binds it " +
                    "to the id of the current session, and C.1.1 publishes an address " +
                    "derived from it, so one id for two sessions is one address for two."
            SessionIdsAreDistinctState.Pass -> ""
            else ->
                "parked in $reached rather than a verdict state: only one child reported " +
                    "its `_sessionid`, so the two ids were never compared."
        }

        assertEquals(SessionIdsAreDistinctState.Pass, reached, why)
    }
}
