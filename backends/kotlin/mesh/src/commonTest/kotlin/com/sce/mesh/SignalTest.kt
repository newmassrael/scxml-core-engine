// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
            """{"errorName":"communication","reason":"TRANSPORT_UNAVAILABLE","target":"hmi","transport":"wss"}""",
            Signal.TransportUnavailable.eventData(wssToHmi),
        )
        assertEquals(
            """{"errorName":"communication","reason":"OUTBOUND_STALE_DROP","target":"hmi","transport":"wss","age_ms":900,"max_age_ms":500}""",
            Signal.OutboundStaleDrop(900, 500).eventData(wssToHmi),
        )
        assertEquals(
            """{"errorName":"communication","reason":"BACKPRESSURE_DROP","target":"hmi","transport":"wss","queue_depth":4}""",
            Signal.BackpressureDrop(4u).eventData(wssToHmi),
        )
    }

    @Test
    fun aTransportErrorPrecedesTheAttemptCount() {
        assertEquals(
            """{"errorName":"communication","reason":"DELIVERY_EXHAUSTED","target":"hmi","transport":"wss","transport_error":"closed \"early\"","attempts":3}""",
            Signal.DeliveryExhausted(3u, "closed \"early\"").eventData(wssToHmi),
        )
        assertEquals(
            """{"errorName":"communication","reason":"SEND_FAILED","target":"hmi","transport":"wss"}""",
            Signal.SendFailed(null).eventData(wssToHmi),
        )
    }

    @Test
    fun aReceivingRowNamesItsSourceAndNoTarget() {
        assertEquals(
            """{"errorName":"communication","reason":"ENVELOPE_CORRUPT","transport":"wss","codec":"cbor"}""",
            Signal.EnvelopeCorrupt(null, "cbor").eventData(wssToHmi),
        )
        assertEquals(
            """{"errorName":"communication","reason":"ENVELOPE_CORRUPT","source":"ecu","transport":"wss","codec":"typed"}""",
            Signal.EnvelopeCorrupt("ecu", "typed").eventData(wssToHmi),
        )
        assertEquals(
            """{"errorName":"communication","reason":"MISSING_SEQUENCE","source":"ecu"}""",
            Signal.MissingSequence("ecu").eventData(wssToHmi),
        )
        assertEquals(
            """{"errorName":"communication","reason":"DEDUP_WINDOW_OVERFLOW","source":"ecu","window_size":256}""",
            Signal.DedupWindowOverflow("ecu", 256u).eventData(wssToHmi),
        )
    }

    @Test
    fun aGapEndedByATickHasNoBindingToName() {
        assertEquals(
            """{"errorName":"communication","reason":"ORDERING_GAP","source":"ecu","lost_seq_lo":4,"lost_seq_hi":6}""",
            Signal.OrderingGap("ecu", 4u, 6u).eventData(null),
        )
    }
}
