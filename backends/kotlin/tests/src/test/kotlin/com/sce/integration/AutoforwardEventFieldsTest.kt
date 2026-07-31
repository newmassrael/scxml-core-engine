// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4 autoforward field preservation — Kotlin AOT local-invoke path.
//
// W3C §6.4 requires the parent to forward an exact copy of every external
// event to an `<invoke autoforward="true">` child. The public IRP suite never
// checks the copy's contents: test229 only asserts the event name crosses, and
// test230 is a manual test whose field comparison is done by a human reading
// two log dumps. A forward stripped down to the bare event name passes both.
//
// Fixture: integration_resources/autoforward_event_fields/autoforward_event_fields.scxml
// (canonical, shared with the C++ / Rust / Go / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_autoforward_event_fields_kotlin.sh

package com.sce.integration

import com.sce.integration.autoforward_event_fields.AutoforwardEventFieldsState
import com.sce.integration.autoforward_event_fields.AutoforwardEventFieldsStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML 6.4 — autoforwarded copy keeps the source event's fields (Kotlin AOT).
@DisplayName("AutoforwardEventFields — W3C SCXML 6.4")
class AutoforwardEventFieldsTest {

    @Test
    fun forwardedCopyKeepsDataOriginAndInvokeid() {
        val sm = AutoforwardEventFieldsStateMachine(W3CTestBase.createEngine())
        sm.initialize()

        if (!sm.isInFinalState) {
            val deadline = System.currentTimeMillis() + 2000L
            while (!sm.isInFinalState && System.currentTimeMillis() < deadline) {
                Thread.sleep(10)
                sm.tick()
            }
        }

        assertEquals(
            AutoforwardEventFieldsState.Pass,
            sm.currentState.value,
            "the child reported `stripped`: the autoforwarded copy of `childToParent` lost " +
                "`_event.data.value`, `_event.origin` or `_event.invokeid`. W3C §6.4 requires " +
                "an exact copy — StateMachineEngine.autoForwardEvent must carry the source " +
                "event's EventMetadata, not just its name."
        )
    }
}
