// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-6.5.2 — what an EMPTY `<finalize>` does, and what an absent one does
// not. Kotlin AOT channel.
//
// With no executable content the Processor "MUST update the data model each
// time an event is received from the child process ... for each item in the
// 'namelist' attribute and each such `<param>` element ... as if by
// `<assign>` with any return value that has a name that matches", and then:
// "Note that the automatic update does not take place if the `<finalize>`
// element is absent as opposed to empty."
//
// The corpus holds two `<finalize>` documents (W3C 233/234) and zero empty
// ones. Measured 2026-08-22, no channel implemented the automatic update.
//
// This channel needs no lowering for the body it runs — Rhino is a JavaScript
// engine, so the parser's JavaScript is already the engine's own language.
// The Lua-backed channels apply `to_lua_script` for the same reason.
//
// Fixture: integration_resources/empty_finalize_updates_the_location/empty_finalize_updates_the_location.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_empty_finalize_updates_the_location_kotlin.sh

package com.sce.integration

import com.sce.integration.empty_finalize_updates_the_location.EmptyFinalizeUpdatesTheLocationState
import com.sce.integration.empty_finalize_updates_the_location.EmptyFinalizeUpdatesTheLocationStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// §scxml-6.5.2 — an empty `<finalize>` updates; an absent one must not.
@DisplayName("EmptyFinalizeUpdatesTheLocation — §scxml-6.5.2")
class EmptyFinalizeUpdatesTheLocationTest {

    @Test
    fun anEmptyFinalizeUpdatesTheLocationAndAnAbsentOneDoesNot() {
        val sm = EmptyFinalizeUpdatesTheLocationStateMachine(W3CTestBase.createEngine())
        sm.initialize()

        // Each phase is settled by a 3 s delayed `<send>`: a child that never
        // answers must reach a verdict rather than hang.
        if (!sm.isInFinalState) {
            val deadline = System.currentTimeMillis() + 15000L
            while (!sm.isInFinalState && System.currentTimeMillis() < deadline) {
                Thread.sleep(10)
                sm.tick()
            }
        }

        try {
            assertEquals(
                EmptyFinalizeUpdatesTheLocationState.Pass,
                sm.currentState.value,
                "FailNotUpdated: the empty <finalize/> left `tally` at its old " +
                    "value — §scxml-6.5.2 makes an empty element mean the automatic " +
                    "update, writing each namelist item's location as if by " +
                    "<assign> with the matching return value. " +
                    "FailUpdatedWithoutFinalize: `guard` moved with no <finalize> " +
                    "element at all — the clause's note is a prohibition, not an " +
                    "omission. " +
                    "FailUnmatchedNameWrote: an event carrying no matching name " +
                    "still wrote `keeper` — the clause says \"with ANY return value " +
                    "that has a name that matches\", so an unconditional write " +
                    "blanks the parent's data model on every unrelated answer. " +
                    "FailEmptyChildSilent / FailAbsentChildSilent / " +
                    "FailUnmatchedChildSilent: a child never answered, so that half " +
                    "was never exercised."
            )
        } finally {
            sm.cleanup()
        }
    }
}
