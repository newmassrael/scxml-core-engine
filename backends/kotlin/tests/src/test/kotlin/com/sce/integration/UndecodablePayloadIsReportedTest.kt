// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML B.2.8.1: a payload the datamodel could not read arrives as a
// space-normalized string, and the host that built it can find out — Kotlin AOT.
//
// The clause gives a payload three readings and names the third "otherwise".
// That word is where a belief leaves the system quietly. A host serializes
// `{"done":true}`, something truncates it to `{"done":`, and the clause is
// satisfied: the content becomes a string. The document then evaluates
// `_event.data.done`, finds nothing, and takes the transition it would have
// taken had the host sent a payload with no `done` field at all. Nothing is
// raised — the fallback is CORRECT behaviour, not an error — so before this
// fixture nothing anywhere said it had happened.
//
// These two deliveries are what no pre-existing accessor separates:
//
//   answer  {"done":              the payload never parsed
//   answer  {"ready":true}        it parsed; `done` is genuinely absent
//
// This channel has four script engines behind one interface (Rhino twice,
// QuickJS, Lua), and each decides for itself what "could not read" means — so
// the rung is measured here rather than inherited from a sibling.
//
// Fixture: integration_resources/undecodable_payload_is_reported/undecodable_payload_is_reported.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_undecodable_payload_is_reported_kotlin.sh

package com.sce.integration

import com.sce.integration.undecodable_payload_is_reported.UndecodablePayloadIsReportedEvent
import com.sce.integration.undecodable_payload_is_reported.UndecodablePayloadIsReportedState
import com.sce.integration.undecodable_payload_is_reported.UndecodablePayloadIsReportedStateMachine
import com.sce.runtime.EventMetadata
import com.sce.w3c.W3CTestBase
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/// W3C SCXML B.2.8.1 — a payload the datamodel refused is something the host can see.
@DisplayName("UndecodablePayloadIsReported — W3C SCXML B.2.8.1")
class UndecodablePayloadIsReportedTest {

    private companion object {
        /// Content that announces an object and stops. The shape a truncated
        /// write, a half-flushed buffer or a serializer that died mid-record
        /// actually produces.
        const val TRUNCATED_OBJECT = """{"done":"""

        /// The same failure announced with `[`, under the other event name, so
        /// a channel that reports "the last event" rather than "the last event
        /// that lost a payload" cannot pass by accident.
        const val TRUNCATED_ARRAY = "[1,2"

        /// W3C test 562 sends exactly this shape and requires it to arrive as a
        /// string. Counting it would make the statistic fire on documents that
        /// are working.
        const val PROSE = "just a sentence"

        /// What the host meant to send.
        const val INTACT_OBJECT = """{"done":true}"""
    }

    private fun started(): UndecodablePayloadIsReportedStateMachine {
        // The fixture guards on `_event.data.done` and counts deliveries with
        // `<assign>`, so this is an ECMAScript-datamodel machine.
        val sm = UndecodablePayloadIsReportedStateMachine(W3CTestBase.createEngine())
        sm.initialize()
        return sm
    }

    private fun deliver(
        sm: UndecodablePayloadIsReportedStateMachine,
        event: UndecodablePayloadIsReportedEvent,
        payload: String
    ) {
        sm.send(event, EventMetadata(data = payload))
        sm.tick()
    }

    /// The axis: content that asked for the structured reading and did not get
    /// it is counted.
    @Test
    fun aPayloadThatAnnouncedStructureAndDidNotParseIsCounted() {
        val sm = started()
        assertEquals(0, sm.undecodablePayloads(), "nothing has been delivered before the first event")

        deliver(sm, UndecodablePayloadIsReportedEvent.Answer, TRUNCATED_OBJECT)

        assertEquals(
            1L,
            sm.answers(),
            "the `answer` transition did not run, so nothing below is measuring a delivery " +
                "that reached the document"
        )
        assertEquals(
            1,
            sm.undecodablePayloads(),
            "the host sent `$TRUNCATED_OBJECT`, which announces an object and does not parse " +
                "as one. W3C SCXML B.2.8.1 correctly delivers it as a string; the host that " +
                "built it has no other way to learn its payload stopped being structure"
        )
        assertEquals(
            UndecodablePayloadIsReportedState.Waiting,
            sm.currentState.value,
            "the reading a payload got must not change which transition fired"
        )
    }

    /// The other half. A count that also counts success cannot be used to
    /// detect failure, and the reading the clause calls "otherwise" is the
    /// NORMAL outcome for a document whose author wrote prose.
    @Test
    fun proseAndAPayloadThatParsedAreNotCounted() {
        val sm = started()

        deliver(sm, UndecodablePayloadIsReportedEvent.Note, PROSE)
        assertEquals(1L, sm.notes(), "the `note` transition did not run")
        assertEquals(
            0,
            sm.undecodablePayloads(),
            "`$PROSE` is the third reading working as W3C SCXML B.2.8.1 specifies and as W3C " +
                "test 562 requires. A diagnostic that fires when nothing is wrong is one " +
                "nobody reads"
        )

        deliver(sm, UndecodablePayloadIsReportedEvent.Answer, INTACT_OBJECT)
        assertEquals(
            UndecodablePayloadIsReportedState.Accepted,
            sm.currentState.value,
            "the guard `_event.data.done` did not hold for `$INTACT_OBJECT`, so the structured " +
                "reading did not happen and the zero below would be proving nothing"
        )
        assertEquals(
            0,
            sm.undecodablePayloads(),
            "a payload that parsed was counted as one that did not"
        )
    }

    /// Why the query has to exist at all: the two deliveries the fixture's
    /// comment names are identical through every accessor a host had.
    @Test
    fun theLossIsNotDerivableFromAnyOtherAccessor() {
        val broken = started()
        deliver(broken, UndecodablePayloadIsReportedEvent.Answer, TRUNCATED_OBJECT)

        val intact = started()
        // Valid JSON, and `done` is genuinely absent — the innocent explanation
        // an operator has to rule out.
        deliver(intact, UndecodablePayloadIsReportedEvent.Answer, """{"ready":true}""")

        assertEquals(
            broken.currentState.value,
            intact.currentState.value,
            "this fixture exists because a lost payload and an absent field are " +
                "indistinguishable through the accessors a host had; if they ever differ, " +
                "the fixture stopped measuring what it claims"
        )
        assertEquals(broken.isInFinalState, intact.isInFinalState)
        assertEquals(
            broken.answers(),
            intact.answers(),
            "the document's own count is the same for both, which is the whole problem"
        )
        assertEquals(
            1,
            broken.undecodablePayloads(),
            "the two runs agree on everything else, so this count is the only thing that " +
                "separates a broken sender from a working one"
        )
        assertEquals(0, intact.undecodablePayloads())
    }

    /// A count says a payload was lost; a host debugging a stalled supervisor
    /// needs to know which delivery lost it.
    @Test
    fun theEngineNamesTheDeliveryThatLostItsPayload() {
        val sm = started()
        assertNull(sm.lastUndecodablePayload(), "nothing has been delivered yet")

        deliver(sm, UndecodablePayloadIsReportedEvent.Answer, TRUNCATED_OBJECT)
        assertEquals(
            UndecodablePayloadIsReportedEvent.Answer,
            sm.lastUndecodablePayload(),
            "the engine counted a lost payload but cannot say which delivery lost it"
        )

        // A second loss, under the other event name: the accessor has to track
        // the last event THAT LOST A PAYLOAD, not the last event.
        deliver(sm, UndecodablePayloadIsReportedEvent.Note, TRUNCATED_ARRAY)
        assertEquals(2, sm.undecodablePayloads(), "the count is a count, not a flag")
        assertEquals(UndecodablePayloadIsReportedEvent.Note, sm.lastUndecodablePayload())

        // And a delivery that succeeds must leave both alone — otherwise the
        // last name would drift to whatever arrived most recently.
        deliver(sm, UndecodablePayloadIsReportedEvent.Answer, INTACT_OBJECT)
        assertEquals(
            UndecodablePayloadIsReportedState.Accepted,
            sm.currentState.value,
            "the intact payload did not take the guarded transition, so the two assertions " +
                "below are not measuring a successful delivery"
        )
        assertEquals(
            2,
            sm.undecodablePayloads(),
            "a delivery that parsed moved a count that belongs to ones that did not"
        )
        assertEquals(
            UndecodablePayloadIsReportedEvent.Note,
            sm.lastUndecodablePayload(),
            "a delivery that parsed moved a name that belongs to one that did not"
        )
    }
}
