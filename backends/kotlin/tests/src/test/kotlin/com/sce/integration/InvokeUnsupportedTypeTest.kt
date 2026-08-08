// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4.1: an `<invoke>` naming an unsupported `type` raises
// `error.execution` — Kotlin AOT path.
//
// The spec defines the case ("the processor MUST place error.execution in the
// internal event queue"), so the document is valid SCXML with one observable:
// that raise. No child session starts and `done.invoke.<id>` never fires.
//
// Both engines were silent here in different ways before this landed — the
// Interpreter substituted an SCXML handler for the unknown type, and AOT
// dropped the `<invoke>` from the model entirely. A backend that renders this
// fixture without the raise reproduces the AOT form, and the machine then
// rests in `probe` instead of reaching `pass`.
//
// Fixture: integration_resources/invoke_unsupported_type/invoke_unsupported_type.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_invoke_unsupported_type_kotlin.sh

package com.sce.integration

import com.sce.integration.invoke_unsupported_type.InvokeUnsupportedTypeState
import com.sce.integration.invoke_unsupported_type.InvokeUnsupportedTypeStateMachine
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML 6.4.1 — an unsupported `<invoke type>` raises error.execution (Kotlin AOT).
@DisplayName("InvokeUnsupportedType — W3C SCXML 6.4.1")
class InvokeUnsupportedTypeTest {

    @Test
    fun anUnsupportedInvokeTypeRaisesErrorExecution() {
        val sm = InvokeUnsupportedTypeStateMachine()
        sm.initialize()

        val deadline = System.currentTimeMillis() + 2000L
        while (!sm.isInFinalState && System.currentTimeMillis() < deadline) {
            sm.tick()
            Thread.sleep(10)
        }

        assertTrue(
            sm.isInFinalState,
            "the machine never completed (parked in ${sm.currentState.value}). W3C SCXML " +
                "6.4.1 requires an <invoke> whose `type` names no supported processor to " +
                "place error.execution on the internal queue; parking in `probe` means " +
                "the <invoke> was dropped rather than lowered"
        )
        assertEquals(
            InvokeUnsupportedTypeState.Pass,
            sm.currentState.value,
            "the machine completed somewhere other than the error.execution target"
        )
    }
}
