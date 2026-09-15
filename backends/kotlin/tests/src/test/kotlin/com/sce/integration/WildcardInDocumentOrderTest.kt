// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A wildcard keeps its own guard and its own type — Kotlin AOT path.
//
// W3C SCXML 3.12.1 lets a `*` descriptor match every event, and that is all
// it changes: W3C SCXML 3.13 still asks the transition's `cond` whether it is
// enabled, and a `type="internal"` transition whose target is a proper
// descendant of its compound source still does not exit that source.
//
// This is the channel the fixture was written for. Until 2026-09-13 the Kotlin
// generator hoisted a bare wildcard out of its state's transitions into the
// `when` block's `else` branch, written by hand: it carried no condition, and
// it spelled `TransitionResult.External` instead of calling `render_result`,
// which is what reads `is_true_internal`. No W3C document writes a wildcard
// with a `cond` or with `type="internal"`, so both went unseen.
//
// Fixture: integration_resources/wildcard_in_document_order/wildcard_in_document_order.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_wildcard_in_document_order_kotlin.sh

package com.sce.integration

import com.sce.integration.wildcard_in_document_order.WildcardInDocumentOrderState
import com.sce.integration.wildcard_in_document_order.WildcardInDocumentOrderStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// A wildcard transition keeps its own guard and its own type (Kotlin AOT).
@DisplayName("WildcardInDocumentOrder — W3C SCXML 3.12.1 + 3.13")
class WildcardInDocumentOrderTest {

    @Test
    fun aWildcardKeepsItsGuardAndItsType() {
        val sm = WildcardInDocumentOrderStateMachine(W3CTestBase.createEngine())
        sm.initialize()

        val deadline = System.currentTimeMillis() + 2000L
        while (!sm.isInFinalState && System.currentTimeMillis() < deadline) {
            sm.tick()
            Thread.sleep(10)
        }

        // Each failure final names the case, so this one assertion says which
        // property of the wildcard was lost:
        //   FailGuardIgnored              a wildcard fired with its guard false
        //   FailGuardNeverFired           a wildcard did not fire with its guard true
        //   FailGuardedInternalReentered  a guarded internal wildcard exited its source
        //   FailSealedInternalReentered   an unguarded internal wildcard exited its source
        // A machine resting in `GuardedFrom` or `SealedFrom` took no internal
        // wildcard at all.
        assertEquals(
            WildcardInDocumentOrderState.Pass,
            sm.currentState.value,
            "the machine did not reach pass: a wildcard is enabled only when its guard " +
                "is true, and an internal wildcard targeting a descendant of its compound " +
                "source must not exit and re-enter that source"
        )
    }
}
