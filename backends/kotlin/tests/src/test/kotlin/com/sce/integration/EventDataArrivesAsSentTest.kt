// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10 + B.2: a payload a HOST injects reaches the datamodel as a
// value — Kotlin AOT.
//
// The edge nothing measured. Every other integration fixture submits its
// events with no metadata at all — measured 2026-08-16, on every channel — so
// the host-to-datamodel boundary was covered by no test. The W3C suite does
// not reach it either: its payloads originate inside the document
// (`<send><content>`, `<param>`, `<donedata>`), a separate path in every
// backend.
//
// Fixture: integration_resources/event_data_arrives_as_sent/event_data_arrives_as_sent.scxml
// (canonical, shared with the C++ / C11 / Rust / Go / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_event_data_arrives_as_sent_kotlin.sh

package com.sce.integration

import com.sce.integration.event_data_arrives_as_sent.EventDataArrivesAsSentEvent
import com.sce.integration.event_data_arrives_as_sent.EventDataArrivesAsSentState
import com.sce.integration.event_data_arrives_as_sent.EventDataArrivesAsSentStateMachine
import com.sce.runtime.EventMetadata
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNotEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML 5.10 — a host's payload arrives as the value it was sent as.
@DisplayName("EventDataArrivesAsSent — W3C SCXML 5.10 + B.2")
class EventDataArrivesAsSentTest {

    @Test
    fun aHostsJsonPayloadIsAddressableAndItsTextStaysText() {
        // The fixture reads `_event.data` in its guards, so this is an
        // ECMAScript-datamodel machine.
        val sm = EventDataArrivesAsSentStateMachine(W3CTestBase.createEngine())
        sm.initialize()

        // A JSON object, the shape an embedder has when it holds structured
        // data and a state machine to give it to.
        sm.send(
            EventDataArrivesAsSentEvent.Payload,
            EventMetadata(data = """{"milestone":"refined","turns":2}"""),
        )
        sm.tick()

        assertNotEquals(
            EventDataArrivesAsSentState.Mangled,
            sm.currentState.value,
            "the host sent a JSON object and the guard `_event.data.milestone === 'refined' " +
                "&& _event.data.turns === 2` did not hold, so the payload did not arrive as " +
                "an object with those properties",
        )
        assertEquals(
            EventDataArrivesAsSentState.Heard,
            sm.currentState.value,
            "the payload guard neither matched nor mismatched — the machine is not in `heard`",
        )

        // Text that is not JSON. The same call, and it must NOT be parsed into
        // something else: `hold the line` is the value the document compares
        // against, character for character.
        sm.send(EventDataArrivesAsSentEvent.Note, EventMetadata(data = "hold the line"))
        sm.tick()

        assertNotEquals(
            EventDataArrivesAsSentState.Garbled,
            sm.currentState.value,
            "the host sent the text `hold the line` and `_event.data === 'hold the line'` did " +
                "not hold, so a payload that is not JSON did not arrive as the string it was " +
                "sent as",
        )

        // Text that happens to be a valid expression. §scxml-B-2-8-1 gives the
        // payload three readings and none of them is "evaluate it": a payload
        // is what a host, a peer session or an HTTP sender put there, and
        // running it makes `_event.data` mean whatever the receiver's engine
        // is written in — this backend has two, and they disagreed.
        sm.send(EventDataArrivesAsSentEvent.Arith, EventMetadata(data = "2 + 3"))
        sm.tick()

        assertNotEquals(
            EventDataArrivesAsSentState.Evaluated,
            sm.currentState.value,
            "the host sent the text `2 + 3` and it arrived as 5 — the payload was run rather " +
                "than read",
        )
        assertEquals(
            EventDataArrivesAsSentState.Settled,
            sm.currentState.value,
            "the arithmetic-shaped payload neither matched nor mismatched",
        )
    }
}
