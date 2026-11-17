// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Each expectation is what the C++ core's CommunicationError renders for the
// same row, and the literal the Rust core's signal.rs tests assert — so the
// three cores are held to the same bytes.

package com.sce.mesh

import kotlin.test.Test
import kotlin.test.assertEquals

class SignalTest {
    private val wssToHmi = Binding("hmi", "wss")

    @Test
    fun aSendingRowNamesItsTargetThenItsTransport() {
        assertEquals(
            """{"errorName":"communication","reason":"transportUnavailable","target":"hmi","transport":"wss"}""",
            Signal.TransportUnavailable.eventData(wssToHmi),
        )
        assertEquals(
            """{"errorName":"communication","reason":"outboundStaleDrop","target":"hmi","transport":"wss","age_ms":900,"max_age_ms":500}""",
            Signal.OutboundStaleDrop(900, 500).eventData(wssToHmi),
        )
        assertEquals(
            """{"errorName":"communication","reason":"backpressureDrop","target":"hmi","transport":"wss","queue_depth":4}""",
            Signal.BackpressureDrop(4u).eventData(wssToHmi),
        )
    }

    @Test
    fun aTransportErrorPrecedesTheAttemptCount() {
        assertEquals(
            """{"errorName":"communication","reason":"deliveryExhausted","target":"hmi","transport":"wss","transport_error":"closed \"early\"","attempts":3}""",
            Signal.DeliveryExhausted(3u, "closed \"early\"").eventData(wssToHmi),
        )
        assertEquals(
            """{"errorName":"communication","reason":"sendFailed","target":"hmi","transport":"wss"}""",
            Signal.SendFailed(null).eventData(wssToHmi),
        )
    }

    @Test
    fun aReceivingRowNamesItsSourceAndNoTarget() {
        assertEquals(
            """{"errorName":"communication","reason":"envelopeCorrupt","transport":"wss","codec":"cbor"}""",
            Signal.EnvelopeCorrupt(null, "cbor").eventData(wssToHmi),
        )
        assertEquals(
            """{"errorName":"communication","reason":"envelopeCorrupt","source":"ecu","transport":"wss","codec":"typed"}""",
            Signal.EnvelopeCorrupt("ecu", "typed").eventData(wssToHmi),
        )
        assertEquals(
            """{"errorName":"communication","reason":"missingSequence","source":"ecu"}""",
            Signal.MissingSequence("ecu").eventData(wssToHmi),
        )
        assertEquals(
            """{"errorName":"communication","reason":"dedupWindowOverflow","source":"ecu","window_size":256}""",
            Signal.DedupWindowOverflow("ecu", 256u).eventData(wssToHmi),
        )
    }

    /**
     * Row 18 names its source, then the epoch and the replica, and neither a target nor a
     * transport: the columns and order the Rust core's signal.rs writes for it.
     */
    @Test
    fun aViewMismatchNamesItsSourceEpochAndReplica() {
        assertEquals(
            """{"errorName":"communication","reason":"causalViewMismatch","source":"list","epoch":1,"replica":"01010101-0101-0101-0101-010101010101"}""",
            Signal.CausalViewMismatch("list", 1uL, "01010101-0101-0101-0101-010101010101").eventData(wssToHmi),
        )
    }

    @Test
    fun aGapEndedByATickHasNoBindingToName() {
        assertEquals(
            """{"errorName":"communication","reason":"orderingGap","source":"ecu","lost_seq_lo":4,"lost_seq_hi":6}""",
            Signal.OrderingGap("ecu", 4u, 6u).eventData(null),
        )
    }
}
