// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
            "errors" to (sm.errors() to 7L),
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
        )
        val wrong = observed.filterValues { (got, want) -> got != want }
        assertEquals(emptyMap<String, Pair<Long?, Long>>(), wrong, "observed (got to want)")
    }
}
