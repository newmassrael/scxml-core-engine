// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A computed event name is matched like any other (§scxml-3.12.1, §scxml-5.10) —
// Kotlin AOT.
//
// A `<send eventexpr>` names its event at run time, so the document cannot have
// written the name: it is delivered as the event the document's names resolve it
// to (its own, the longest token prefix of it the document writes, or its
// wildcard), and `_event.name` is the whole name. The document sends six, over
// the external and the internal queue, now and after a delay, and takes each only
// when it is told the whole name.
//
// Fixture: integration_resources/a_computed_event_name_is_matched_like_any_other/a_computed_event_name_is_matched_like_any_other.scxml
// (canonical, shared with the C++ / Rust / Go / Python / C11 channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_a_computed_event_name_is_matched_like_any_other_kotlin.sh

package com.sce.integration

import com.sce.integration.a_computed_event_name_is_matched_like_any_other.AComputedEventNameIsMatchedLikeAnyOtherState
import com.sce.integration.a_computed_event_name_is_matched_like_any_other.AComputedEventNameIsMatchedLikeAnyOtherStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML 3.12.1 + 5.10 — a computed event name is matched like any other (Kotlin AOT).
@DisplayName("AComputedEventNameIsMatchedLikeAnyOther — W3C SCXML 3.12.1 + 5.10")
class AComputedEventNameIsMatchedLikeAnyOtherTest {

    @Test
    fun aComputedNameIsMatchedByTokenAndToldWhole() {
        val sm = AComputedEventNameIsMatchedLikeAnyOtherStateMachine(W3CTestBase.createEngine())
        sm.initialize()

        if (!sm.isInFinalState) {
            val deadline = System.currentTimeMillis() + 2000L
            while (!sm.isInFinalState && System.currentTimeMillis() < deadline) {
                Thread.sleep(10)
                sm.tick()
            }
        }

        assertEquals(
            AComputedEventNameIsMatchedLikeAnyOtherState.Pass,
            sm.terminalState,
            "a computed name was dropped, or matched but told shorter than the name it was " +
                "sent under (`request.new`, `other.thing`, `request.again`, `last.one`)"
        )
    }
}
