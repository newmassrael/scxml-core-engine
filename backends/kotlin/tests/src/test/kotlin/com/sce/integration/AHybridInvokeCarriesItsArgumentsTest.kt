// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4.1 + 6.4.3: an <invoke> whose child is named by an expression
// carries its arguments as one whose child is fixed does — Kotlin AOT path.
//
// The value always names `keeper`, and `bare` declares the one name `keeper`
// does not, so a pair seeded by the wrong candidate's declarations is a leak
// `keeper` reports rather than an absence the test has to infer.
//
// Fixture: integration_resources/a_hybrid_invoke_carries_its_arguments/a_hybrid_invoke_carries_its_arguments.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python channels).
//
// Regeneration: scripts/regen_a_hybrid_invoke_carries_its_arguments_kotlin.sh

package com.sce.integration

import com.sce.integration.a_hybrid_invoke_carries_its_arguments.AHybridInvokeCarriesItsArgumentsState
import com.sce.integration.a_hybrid_invoke_carries_its_arguments.AHybridInvokeCarriesItsArgumentsStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("AHybridInvokeCarriesItsArguments — W3C SCXML 6.4.1 + 6.4.3")
class AHybridInvokeCarriesItsArgumentsTest {

    @Test
    fun eachArgumentReachesOnlyTheChildThatDeclaresIt() {
        val sm = AHybridInvokeCarriesItsArgumentsStateMachine(W3CTestBase.createEngine())
        sm.initialize()

        val deadline = System.currentTimeMillis() + 2000L
        while (!sm.isInFinalState && System.currentTimeMillis() < deadline) {
            sm.tick()
            Thread.sleep(10)
        }

        assertTrue(
            sm.isInFinalState,
            "the machine never completed (parked in ${sm.currentState.value}); " +
                "refusedPhase parks when its invoke raised nothing"
        )
        assertEquals(
            AHybridInvokeCarriesItsArgumentsState.Done,
            sm.terminalState,
            "FailWrongChild means bare ran; FailRefusedChildStarted means an " +
                "unreadable namelist still started a child"
        )
        val observed = mapOf(
            "errors" to (sm.errors() to 2L),
            "started" to (sm.started() to 2L),
            "paramsOk" to (sm.paramsOk() to 1L),
            "namelistOk" to (sm.namelistOk() to 1L),
        )
        val wrong = observed.filterValues { (got, want) -> got != want }
        assertEquals(emptyMap<String, Pair<Long?, Long>>(), wrong, "observed (got to want)")
    }
}
