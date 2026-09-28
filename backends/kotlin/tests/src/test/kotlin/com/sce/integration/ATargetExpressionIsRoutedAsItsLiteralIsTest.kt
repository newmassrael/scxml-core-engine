// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2.4 + C.1: a `targetexpr` is routed as the same value written in
// `target` is, at once or after a delay — Kotlin AOT path.
//
// Fixture: integration_resources/a_target_expression_is_routed_as_its_literal_is/a_target_expression_is_routed_as_its_literal_is.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python channels).
//
// Regeneration: scripts/regen_a_target_expression_is_routed_as_its_literal_is_kotlin.sh

package com.sce.integration

import com.sce.integration.a_target_expression_is_routed_as_its_literal_is.ATargetExpressionIsRoutedAsItsLiteralIsState
import com.sce.integration.a_target_expression_is_routed_as_its_literal_is.ATargetExpressionIsRoutedAsItsLiteralIsStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

@DisplayName("ATargetExpressionIsRoutedAsItsLiteralIs — W3C SCXML 6.2.4")
class ATargetExpressionIsRoutedAsItsLiteralIsTest {

    @Test
    fun aTargetExpressionIsRoutedAsItsLiteralIs() {
        val sm = ATargetExpressionIsRoutedAsItsLiteralIsStateMachine(W3CTestBase.createEngine())
        sm.initialize()

        val deadline = System.currentTimeMillis() + 3000L
        while (!sm.isInFinalState && System.currentTimeMillis() < deadline) {
            sm.tick()
            Thread.sleep(5)
        }

        assertTrue(sm.isInFinalState, "the machine never completed (parked in ${sm.currentState.value})")
        assertEquals(ATargetExpressionIsRoutedAsItsLiteralIsState.Done, sm.terminalState, "the run must end in `done`")
        val observed = mapOf(
            "internalNow" to (sm.internalNow() to 1L),
            "internalLater" to (sm.internalLater() to 1L),
            "kidNow" to (sm.kidNow() to 1L),
            "kidLater" to (sm.kidLater() to 1L),
            "sessNow" to (sm.sessNow() to 1L),
            "sessLater" to (sm.sessLater() to 1L),
            "commErrors" to (sm.commErrors() to 4L),
            "execErrors" to (sm.execErrors() to 2L),
            "afterStranger" to (sm.afterStranger() to 0L),
            "afterStrangerLater" to (sm.afterStrangerLater() to 0L),
            "afterOrphan" to (sm.afterOrphan() to 0L),
            "afterOrphanLater" to (sm.afterOrphanLater() to 0L),
            "afterBogus" to (sm.afterBogus() to 0L),
            "afterBogusLater" to (sm.afterBogusLater() to 0L),
        )
        val wrong = observed.filterValues { (got, want) -> got != want }
        assertEquals(emptyMap<String, Pair<Long?, Long>>(), wrong, "observed (got to want)")
    }
}
