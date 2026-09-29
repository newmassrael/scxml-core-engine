// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 4.9: an error ends the block it was raised in — whichever element
// raised it — and no other block — Kotlin AOT path.
//
// Fixture: integration_resources/an_error_ends_the_block_it_was_raised_in/an_error_ends_the_block_it_was_raised_in.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python channels).
//
// Regeneration: scripts/regen_an_error_ends_the_block_it_was_raised_in_kotlin.sh

package com.sce.integration

import com.sce.integration.an_error_ends_the_block_it_was_raised_in.AnErrorEndsTheBlockItWasRaisedInEvent
import com.sce.integration.an_error_ends_the_block_it_was_raised_in.AnErrorEndsTheBlockItWasRaisedInState
import com.sce.integration.an_error_ends_the_block_it_was_raised_in.AnErrorEndsTheBlockItWasRaisedInStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("AnErrorEndsTheBlockItWasRaisedIn — W3C SCXML 4.9")
class AnErrorEndsTheBlockItWasRaisedInTest {

    /**
     * The control for the refused root start: this document never sends to
     * `#_parent`, so a root-start policy lets it start, and the checked start
     * runs it as `initialize` would.
     */
    @Test
    fun aRootStartOfAMachineThatNeedsNoParentRuns() {
        val sm = AnErrorEndsTheBlockItWasRaisedInStateMachine(W3CTestBase.createEngine())
        assertEquals(null, sm.rootStartRefusal())
        assertEquals(null, sm.initializeAsRoot())
        org.junit.jupiter.api.Assertions.assertTrue(
            sm.activeConfiguration.isNotEmpty(),
            "a root start that is not refused must enter the machine",
        )
    }

    @Test
    fun eachErrorEndsOnlyItsOwnBlock() {
        val sm = AnErrorEndsTheBlockItWasRaisedInStateMachine(W3CTestBase.createEngine())
        sm.initialize()
        for (event in listOf(
            AnErrorEndsTheBlockItWasRaisedInEvent.T,
            AnErrorEndsTheBlockItWasRaisedInEvent.Finish,
        )) {
            sm.send(event)
            sm.tick()
        }

        assertEquals(AnErrorEndsTheBlockItWasRaisedInState.Done, sm.terminalState, "`finish` must carry the run to `done`")
        val observed = mapOf(
            "errors" to (sm.errors() to 10L),
            "afterAssign" to (sm.afterAssign() to 0L),
            "afterScript" to (sm.afterScript() to 0L),
            "afterLog" to (sm.afterLog() to 0L),
            "afterCancel" to (sm.afterCancel() to 0L),
            "afterIfInner" to (sm.afterIfInner() to 0L),
            "afterIf" to (sm.afterIf() to 0L),
            "afterSingle" to (sm.afterSingle() to 0L),
            "afterTrans" to (sm.afterTrans() to 0L),
            "initRan" to (sm.initRan() to 1L),
            "pairs" to (sm.pairs() to 4L),
            "sum" to (sm.sum() to 90L),
            "ifThen" to (sm.ifThen() to 0L),
            "ifElse" to (sm.ifElse() to 1L),
            "afterIfCond" to (sm.afterIfCond() to 0L),
            "elseifThen" to (sm.elseifThen() to 0L),
            "elseifElse" to (sm.elseifElse() to 1L),
            "afterElseifCond" to (sm.afterElseifCond() to 0L),
            "afterNestedIf" to (sm.afterNestedIf() to 0L),
            "afterOuterIf" to (sm.afterOuterIf() to 0L),
        )
        val wrong = observed.filterValues { (got, want) -> got != want }
        assertEquals(emptyMap<String, Pair<Long?, Long>>(), wrong, "observed (got to want)")
    }
}
