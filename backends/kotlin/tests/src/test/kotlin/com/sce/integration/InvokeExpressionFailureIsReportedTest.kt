// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4.3: an <invoke> naming its target through an expression
// evaluates that expression at invoke-fire time, and a failure to evaluate
// raises error.execution — Kotlin AOT path.
//
// Kotlin is the one channel whose generated parent resolves the evaluated
// string into a document rather than spawning the build-time stub the other
// five use (docs/SCE_ACCEPTED_SUBSET.md §2.13). This fixture is deliberately
// blind to that difference: it asserts only the obligation both shapes share,
// so it stays a measurement of the clause rather than of one backend's route
// to it.
//
// Fixture: integration_resources/invoke_expression_failure_is_reported/invoke_expression_failure_is_reported.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_invoke_expression_failure_is_reported_kotlin.sh

package com.sce.integration

import com.sce.integration.invoke_expression_failure_is_reported.InvokeExpressionFailureIsReportedState
import com.sce.integration.invoke_expression_failure_is_reported.InvokeExpressionFailureIsReportedStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML 6.4.3 — an invoke expression that cannot be evaluated is reported.
@DisplayName("InvokeExpressionFailureIsReported — W3C SCXML 6.4.3")
class InvokeExpressionFailureIsReportedTest {

    @Test
    fun anInvokeExpressionThatCannotBeEvaluatedRaisesErrorExecution() {
        // The fixture declares `<data>` and its `<invoke>` names an
        // expression, so the machine needs an engine to evaluate both.
        val sm = InvokeExpressionFailureIsReportedStateMachine(W3CTestBase.createEngine())
        sm.initialize()

        val deadline = System.currentTimeMillis() + 2000L
        while (!sm.isInFinalState && System.currentTimeMillis() < deadline) {
            sm.tick()
            Thread.sleep(10)
        }

        assertTrue(
            sm.isInFinalState,
            "the machine never completed (parked in ${sm.currentState.value}). W3C SCXML " +
                "6.4.3 requires the expression to be evaluated when the <invoke> fires; " +
                "parking means neither the raise nor the child arrived"
        )
        assertEquals(
            InvokeExpressionFailureIsReportedState.Pass,
            sm.currentState.value,
            "reaching `fail` means the child started on an expression that cannot be " +
                "evaluated, so nothing evaluated it"
        )
    }
}
