// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.7.1 + 6.4: what each argument of an <invoke> costs when it
// cannot be read, and what a readable one delivers — Kotlin AOT path.
//
// Fixture: integration_resources/a_bad_invoke_argument_is_reported_once/a_bad_invoke_argument_is_reported_once.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python channels).
//
// Regeneration: scripts/regen_a_bad_invoke_argument_is_reported_once_kotlin.sh

package com.sce.integration

import com.sce.integration.a_bad_invoke_argument_is_reported_once.ABadInvokeArgumentIsReportedOnceEvent
import com.sce.integration.a_bad_invoke_argument_is_reported_once.ABadInvokeArgumentIsReportedOnceState
import com.sce.integration.a_bad_invoke_argument_is_reported_once.ABadInvokeArgumentIsReportedOnceStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("ABadInvokeArgumentIsReportedOnce — W3C SCXML 5.7.1 + 6.4")
class ABadInvokeArgumentIsReportedOnceTest {

    @Test
    fun eachArgumentCostsWhatItsClauseSays() {
        val sm = ABadInvokeArgumentIsReportedOnceStateMachine(W3CTestBase.createEngine())
        sm.initialize()
        for (event in listOf(
            ABadInvokeArgumentIsReportedOnceEvent.Go,
            ABadInvokeArgumentIsReportedOnceEvent.Finish,
        )) {
            sm.send(event)
            sm.tick()
        }

        assertEquals(ABadInvokeArgumentIsReportedOnceState.Done, sm.terminalState, "`finish` must carry the run to `done`")
        val observed = mapOf(
            "errors" to (sm.errors() to 5L),
            "started" to (sm.started() to 1L),
            "fromLocOk" to (sm.fromLocOk() to 1L),
            "emptyLocLeftOut" to (sm.emptyLocLeftOut() to 1L),
            "brokenLeftOut" to (sm.brokenLeftOut() to 1L),
        )
        val wrong = observed.filterValues { (got, want) -> got != want }
        assertEquals(emptyMap<String, Pair<Long?, Long>>(), wrong, "observed (got to want)")
    }
}
