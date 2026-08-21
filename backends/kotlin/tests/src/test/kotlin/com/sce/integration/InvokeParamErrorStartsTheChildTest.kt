// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-5.7.1 under §scxml-6.4: a `<param>` of an `<invoke>` whose expression
// will not evaluate — Kotlin AOT channel.
//
// Two clauses meet here and only one governs. §scxml-6.4.2 terminates the
// element when "the evaluation of its arguments produces an error", and the
// sentence after it — "Otherwise the Processor MUST start a new logical
// instance" — makes the alternative explicit. §scxml-5.7.1 says a failing
// `<param>` costs `error.execution` and "MUST ignore the name and value", then
// delegates only the SUCCESSFUL name and value to the context: "See 5.5
// <donedata>, 6.2 <send> and 6.4 <invoke> for details."
//
// 5.7.1 governs. This channel took the other reading and dropped the reporting
// half with it: the param arm caught the failure, `return@run`, and raised
// nothing — so a document lost the child AND the event that would have
// explained why. The comment beside it called that "the C++ pattern"; C++ does
// not cancel.
//
// Fixture: integration_resources/invoke_param_error_starts_the_child/invoke_param_error_starts_the_child.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_invoke_param_error_starts_the_child_kotlin.sh

package com.sce.integration

import com.sce.integration.invoke_param_error_starts_the_child.InvokeParamErrorStartsTheChildState
import com.sce.integration.invoke_param_error_starts_the_child.InvokeParamErrorStartsTheChildStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// §scxml-5.7.1 under 6.4 — a failed invoke param costs its pair, not the session.
@DisplayName("InvokeParamErrorStartsTheChild — §scxml-5.7.1 under §scxml-6.4")
class InvokeParamErrorStartsTheChildTest {

    @Test
    fun anInvokeParamThatWillNotEvaluateCostsItsPairAndNothingElse() {
        val sm = InvokeParamErrorStartsTheChildStateMachine(W3CTestBase.createEngine())
        sm.initialize()

        // The fixture's own `timeout` is a 3 s delayed `<send>`: a channel that
        // terminated the element leaves the machine waiting on a session that
        // was never created, and only the clock turns that into a verdict.
        if (!sm.isInFinalState) {
            val deadline = System.currentTimeMillis() + 10000L
            while (!sm.isInFinalState && System.currentTimeMillis() < deadline) {
                Thread.sleep(10)
                sm.tick()
            }
        }

        try {
            assertEquals(
                InvokeParamErrorStartsTheChildState.Pass,
                sm.currentState.value,
                "FailNoParamError: `childUp` arrived with no `error.execution` " +
                    "before it — §scxml-5.7.1 puts that error on the internal queue " +
                    "while the `<invoke>` is being evaluated, so it is dequeued " +
                    "before the child's first word. " +
                    "FailInvokeNotStarted: the child never started — this channel " +
                    "read §scxml-6.4.2's \"terminate the processing of the element\" " +
                    "over 5.7.1's per-item rule, so one `<param>` that would not " +
                    "evaluate cost the whole session. " +
                    "FailGoodParamLost: the child's `kept` did not arrive as 'here' " +
                    "— one sibling that failed does not cost the others. " +
                    "FailBrokenParamSeeded: the child found the empty string under " +
                    "`broken` — 5.7.1 says ignore the name AND the value."
            )
        } finally {
            sm.cleanup()
        }
    }
}
