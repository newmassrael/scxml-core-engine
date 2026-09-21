// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4.3: the value a `srcexpr` computes is the child that runs —
// Kotlin AOT path, over the candidate set `sce:candidates` declares.
//
// Kotlin is the channel this clause cost the most. It used to resolve the
// value into a document through an interpreter that shipped only in this
// repository's test module, so the behaviour was right and unshippable at
// once; moving it onto the build-time stub made it shippable and took the
// selection away. Candidates give the selection back on the AOT path, which
// is what this asserts.
//
//   ran `chosen`   -> `from.chosen`     -> pass
//   ran `other`    -> `from.other`      -> wrongChild
//   loaded nothing -> `error.execution` -> noChild
//   ran a stub     -> no event at all   -> parked in `probe`
//
// Fixture: integration_resources/invoke_candidate_selects_the_child/invoke_candidate_selects_the_child.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_invoke_candidate_selects_the_child_kotlin.sh

package com.sce.integration

import com.sce.integration.invoke_candidate_selects_the_child.InvokeCandidateSelectsTheChildState
import com.sce.integration.invoke_candidate_selects_the_child.InvokeCandidateSelectsTheChildStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML 6.4.3 — the evaluated value selects which candidate runs.
@DisplayName("InvokeCandidateSelectsTheChild — W3C SCXML 6.4.3")
class InvokeCandidateSelectsTheChildTest {

    @Test
    fun theEvaluatedValueSelectsWhichCandidateRuns() {
        val sm = InvokeCandidateSelectsTheChildStateMachine(W3CTestBase.createEngine())
        sm.initialize()

        val deadline = System.currentTimeMillis() + 2000L
        while (!sm.isInFinalState && System.currentTimeMillis() < deadline) {
            sm.tick()
            Thread.sleep(10)
        }

        assertTrue(
            sm.isInFinalState,
            "the machine never completed (parked in ${sm.currentState.value}). Parking " +
                "means no child spoke: a stub ran, or nothing did"
        )
        assertEquals(
            InvokeCandidateSelectsTheChildState.Pass,
            sm.currentState.value,
            "WrongChild means the value selected the other candidate; NoChild means " +
                "nothing was loaded at all"
        )
    }
}
