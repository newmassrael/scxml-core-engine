// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.7: only a top-level `<final>` ends the session — Kotlin AOT path.
//
// Appendix D `enterStates` sets `running = false` for a `<final>` only when
// `isSCXMLElement(s.parent)`; otherwise it queues `done.state.<parent>` and the
// machine carries on. The structural question — "is this state a `<final>`
// element" — is not the completion criterion, and only the latter may gate
// completion, the completion callback, and the `done.invoke.<id>` a parent
// emits for this machine.
//
// The fixture rests in the nested final rather than passing through it: a
// machine that continues within the same macrostep is only ever sampled at the
// end, where a right and a wrong predicate agree.
//
// Fixture: integration_resources/nested_final_not_terminal/nested_final_not_terminal.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_nested_final_not_terminal_kotlin.sh

package com.sce.integration

import com.sce.integration.nested_final_not_terminal.NestedFinalNotTerminalEvent
import com.sce.integration.nested_final_not_terminal.NestedFinalNotTerminalState
import com.sce.integration.nested_final_not_terminal.NestedFinalNotTerminalStateMachine
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML 3.7 — a nested `<final>` completes its compound state, not the session (Kotlin AOT).
@DisplayName("NestedFinalNotTerminal — W3C SCXML 3.7")
class NestedFinalNotTerminalTest {

    @Test
    fun aNestedFinalDoesNotEndTheSession() {
        val sm = NestedFinalNotTerminalStateMachine()
        sm.initialize()

        assertEquals(
            NestedFinalNotTerminalState.PhaseDone,
            sm.currentState.value,
            "the fixture is supposed to come to rest in the nested <final>; it did " +
                "not, so nothing below is testing what it claims"
        )
        assertFalse(
            sm.isInFinalState,
            "the engine reported completion while resting in `phaseDone`, a <final> " +
                "nested inside `phase`. W3C SCXML Appendix D enterStates ends the " +
                "session only when the final's parent is the <scxml> element — a " +
                "nested one finishes its compound state and queues done.state.phase, " +
                "leaving the machine live."
        )

        sm.send(NestedFinalNotTerminalEvent.Resume)

        val deadline = System.currentTimeMillis() + 2000L
        while (!sm.isInFinalState && System.currentTimeMillis() < deadline) {
            sm.tick()
            Thread.sleep(10)
        }

        assertEquals(
            NestedFinalNotTerminalState.Pass,
            sm.currentState.value,
            "`resume` did not carry the machine out of the nested final to the " +
                "top-level one"
        )
    }
}
