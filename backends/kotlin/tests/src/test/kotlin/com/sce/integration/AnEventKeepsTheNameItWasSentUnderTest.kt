// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The name an event arrives under (§scxml-5.10, §scxml-3.12.1) — Kotlin AOT
// local-invoke path.
//
// A transition on `request` takes `request.new` by whole-token matching, and
// `_event.name` is then the name the event was sent under, not the descriptor it
// was matched through. The public IRP suite never reads a name the document does
// not write, so a machine that is told the shorter one passes all of it.
//
// Fixture: integration_resources/an_event_keeps_the_name_it_was_sent_under/an_event_keeps_the_name_it_was_sent_under.scxml
// (canonical, shared with the C++ / Rust / Go / Python / C11 channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_an_event_keeps_the_name_it_was_sent_under_kotlin.sh

package com.sce.integration

import com.sce.integration.an_event_keeps_the_name_it_was_sent_under.AnEventKeepsTheNameItWasSentUnderState
import com.sce.integration.an_event_keeps_the_name_it_was_sent_under.AnEventKeepsTheNameItWasSentUnderStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML 5.10 — `_event.name` is the name the event was sent under (Kotlin AOT).
@DisplayName("AnEventKeepsTheNameItWasSentUnder — W3C SCXML 5.10")
class AnEventKeepsTheNameItWasSentUnderTest {

    @Test
    fun aNameTheDocumentDoesNotWriteIsToldWhole() {
        val sm = AnEventKeepsTheNameItWasSentUnderStateMachine(W3CTestBase.createEngine())
        sm.initialize()

        if (!sm.isInFinalState) {
            val deadline = System.currentTimeMillis() + 2000L
            while (!sm.isInFinalState && System.currentTimeMillis() < deadline) {
                Thread.sleep(10)
                sm.tick()
            }
        }

        assertEquals(
            AnEventKeepsTheNameItWasSentUnderState.Pass,
            sm.terminalState,
            "the child reported `arrivedShortened`: `request.new` took the transition on " +
                "`request` but `_event.name` told the child `request`. W3C §5.10 makes the " +
                "name the one the event was sent under, and §3.12.1 only decides which " +
                "transition takes it."
        )
    }
}
