// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix C.1 `_event.origin` is an address — Kotlin AOT.
//
// The clause has two halves. The origin of a delivered event must match the
// `location` field the sending session published for the SCXML Event I/O
// Processor in its `_ioprocessors`, and that location is what a peer sends
// back to. A machine that puts a bare session id — or an invoke-instance
// path — there satisfies neither: the value matches nothing the sender
// published, and it names no target.
//
// The public IRP suite cannot separate the two spellings. Test 336 and test
// 350 both check `_event.origin` by sending to it with the sender and the
// receiver being the same session, so any value at all round-trips. Nothing
// in the corpus sends across sessions, which is the only arrangement where a
// bare id and a location differ.
//
// The fixture puts a second session on the other end, so the two halves
// separate and each has its own signal:
//
//   mismatch  the parent lands in `fail` — `_event.origin` did not equal the
//             location the child published for itself
//   routing   the parent parks in `await_reply` and the run times out — a
//             target that resolves nowhere delivers no event to fail on
//
// Fixture: integration_resources/event_origin_is_a_location/event_origin_is_a_location.scxml
// (canonical, shared with the C++ / Rust / Go / Python / C11 channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_event_origin_is_a_location_kotlin.sh

package com.sce.integration

import com.sce.integration.event_origin_is_a_location.EventOriginIsALocationState
import com.sce.integration.event_origin_is_a_location.EventOriginIsALocationStateMachine
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML Appendix C.1 — `_event.origin` is the sender's published location (Kotlin AOT).
@DisplayName("EventOriginIsALocation — W3C SCXML C.1")
class EventOriginIsALocationTest {

    @Test
    fun originIsTheSendersPublishedLocationAndRoutesBack() {
        val sm = EventOriginIsALocationStateMachine(W3CTestBase.createEngine())
        sm.initialize()

        if (!sm.isInFinalState) {
            val deadline = System.currentTimeMillis() + 2000L
            while (!sm.isInFinalState && System.currentTimeMillis() < deadline) {
                Thread.sleep(10)
                sm.tick()
            }
        }

        val reached = sm.currentState.value
        val why = when (reached) {
            EventOriginIsALocationState.Fail ->
                "`_event.origin` did not carry the sender's published `_ioprocessors` " +
                    "location. Appendix C.1 requires the origin to match that location, " +
                    "which is what makes it an address a peer can answer; a bare session " +
                    "id or an invoke-instance path matches nothing the sender published."
            EventOriginIsALocationState.Pass -> ""
            else ->
                "parked in $reached rather than a verdict state. The parent accepted " +
                    "`_event.origin` as an address and sent `reply` to it, and nothing " +
                    "came back: C.1 requires the published location to be a usable " +
                    "<send> target, so an origin that routes nowhere fails the half a " +
                    "self-addressed test cannot exercise."
        }

        assertEquals(EventOriginIsALocationState.Pass, reached, why)
    }
}
