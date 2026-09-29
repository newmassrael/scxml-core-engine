// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2 + 6.4 + C.1: a <send> reaches what its target names, carries
// its payload there, and a target that names nothing reachable is reported —
// Kotlin AOT path.
//
// Fixture: integration_resources/a_send_reaches_only_what_its_target_names/a_send_reaches_only_what_its_target_names.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python channels).
//
// Regeneration: scripts/regen_a_send_reaches_only_what_its_target_names_kotlin.sh

package com.sce.integration

import com.sce.integration.a_send_reaches_only_what_its_target_names.ASendReachesOnlyWhatItsTargetNamesState
import com.sce.integration.a_send_reaches_only_what_its_target_names.ASendReachesOnlyWhatItsTargetNamesStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("ASendReachesOnlyWhatItsTargetNames — W3C SCXML C.1")
class ASendReachesOnlyWhatItsTargetNamesTest {

    /**
     * W3C SCXML 6.2.4: this document sends to `#_parent`, so a host that runs
     * machines as roots and asks for the refusal gets it — and nothing starts.
     * The plain `initialize` below runs the same machine; the refusal is opt-in.
     */
    @Test
    fun aRootStartOfAMachineThatNeedsAParentIsRefused() {
        val sm = ASendReachesOnlyWhatItsTargetNamesStateMachine(W3CTestBase.createEngine())
        assertEquals(com.sce.runtime.RootStartRefusal.NEEDS_PARENT, sm.rootStartRefusal())
        assertEquals(com.sce.runtime.RootStartRefusal.NEEDS_PARENT, sm.initializeAsRoot())
        assertTrue(sm.activeConfiguration.isEmpty(), "a refused root start must enter nothing")
        assertEquals(
            "the machine sends to #_parent and was started with no parent session",
            com.sce.runtime.RootStartRefusal.NEEDS_PARENT.reason,
        )
    }

    @Test
    fun aSendReachesOnlyWhatItsTargetNames() {
        val sm = ASendReachesOnlyWhatItsTargetNamesStateMachine(W3CTestBase.createEngine())
        sm.initialize()

        val deadline = System.currentTimeMillis() + 2000L
        while (!sm.isInFinalState && System.currentTimeMillis() < deadline) {
            sm.tick()
            Thread.sleep(10)
        }

        assertTrue(sm.isInFinalState, "the machine never completed (parked in ${sm.currentState.value})")
        assertEquals(ASendReachesOnlyWhatItsTargetNamesState.Done, sm.terminalState, "the run must end in `done`")
        val observed = mapOf(
            "execErrors" to (sm.execErrors() to 1L),
            "commErrors" to (sm.commErrors() to 3L),
            "afterRefused" to (sm.afterRefused() to 0L),
            "afterNobody" to (sm.afterNobody() to 0L),
            "afterStranger" to (sm.afterStranger() to 0L),
            "afterOrphan" to (sm.afterOrphan() to 0L),
            "bareArrived" to (sm.bareArrived() to 1L),
            "pongOk" to (sm.pongOk() to 1L),
        )
        val wrong = observed.filterValues { (got, want) -> got != want }
        assertEquals(emptyMap<String, Pair<Long?, Long>>(), wrong, "observed (got to want)")
    }
}
