// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 4.9 + 4.6: an error raised inside a <foreach> body ends the
// block that contains the <foreach>, and it is the only error raised —
// Kotlin AOT path.
//
// Fixture: integration_resources/an_error_inside_a_foreach_ends_its_block/an_error_inside_a_foreach_ends_its_block.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python channels).
//
// Regeneration: scripts/regen_an_error_inside_a_foreach_ends_its_block_kotlin.sh

package com.sce.integration

import com.sce.integration.an_error_inside_a_foreach_ends_its_block.AnErrorInsideAForeachEndsItsBlockEvent
import com.sce.integration.an_error_inside_a_foreach_ends_its_block.AnErrorInsideAForeachEndsItsBlockState
import com.sce.integration.an_error_inside_a_foreach_ends_its_block.AnErrorInsideAForeachEndsItsBlockStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("AnErrorInsideAForeachEndsItsBlock — W3C SCXML 4.9 + 4.6")
class AnErrorInsideAForeachEndsItsBlockTest {

    @Test
    fun theErrorEndsTheBlockAndIsTheOnlyOne() {
        val sm = AnErrorInsideAForeachEndsItsBlockStateMachine(W3CTestBase.createEngine())
        sm.initialize()
        for (event in listOf(
            AnErrorInsideAForeachEndsItsBlockEvent.Go,
            AnErrorInsideAForeachEndsItsBlockEvent.T,
            AnErrorInsideAForeachEndsItsBlockEvent.Finish,
        )) {
            sm.send(event)
            sm.tick()
        }

        assertEquals(AnErrorInsideAForeachEndsItsBlockState.Done, sm.terminalState, "`finish` must carry the run to `done`")
        val observed = mapOf(
            "iters1" to (sm.iters1() to 1L),
            "after1" to (sm.after1() to 0L),
            "iters2" to (sm.iters2() to 1L),
            "after2" to (sm.after2() to 0L),
            "iters3" to (sm.iters3() to 1L),
            "after3" to (sm.after3() to 0L),
            "errors" to (sm.errors() to 3L),
            "sent" to (sm.sent() to 2L),
        )
        val wrong = observed.filterValues { (got, want) -> got != want }
        assertEquals(emptyMap<String, Pair<Long?, Long>>(), wrong, "observed (got to want)")
    }
}
