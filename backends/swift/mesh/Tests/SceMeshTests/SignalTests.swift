// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Each expectation is what the C++ core's CommunicationError renders for the same row, and the literal
// the Rust and Kotlin cores' signal tests assert, so the cores are held to the same bytes.

import SceMesh
import Testing

@Suite struct SignalTests {
    private let wssToHmi = Binding(peer: "hmi", transport: "wss")

    @Test func aSendingRowNamesItsTargetThenItsTransport() {
        #expect(
            Signal.transportUnavailable.eventData(binding: wssToHmi)
                == #"{"errorName":"communication","reason":"TRANSPORT_UNAVAILABLE","target":"hmi","transport":"wss"}"#)
        #expect(
            Signal.outboundStaleDrop(ageMs: 900, maxAgeMs: 500).eventData(binding: wssToHmi)
                == #"{"errorName":"communication","reason":"OUTBOUND_STALE_DROP","target":"hmi","transport":"wss","age_ms":900,"max_age_ms":500}"#
        )
        #expect(
            Signal.backpressureDrop(depth: 4).eventData(binding: wssToHmi)
                == #"{"errorName":"communication","reason":"BACKPRESSURE_DROP","target":"hmi","transport":"wss","queue_depth":4}"#
        )
    }

    @Test func aTransportErrorPrecedesTheAttemptCount() {
        #expect(
            Signal.deliveryExhausted(attempts: 3, transportError: "closed \"early\"").eventData(binding: wssToHmi)
                == #"{"errorName":"communication","reason":"DELIVERY_EXHAUSTED","target":"hmi","transport":"wss","transport_error":"closed \"early\"","attempts":3}"#
        )
        #expect(
            Signal.sendFailed(transportError: nil).eventData(binding: wssToHmi)
                == #"{"errorName":"communication","reason":"SEND_FAILED","target":"hmi","transport":"wss"}"#)
    }

    @Test func aReceivingRowNamesItsSourceAndNoTarget() {
        #expect(
            Signal.envelopeCorrupt(source: nil, codec: "cbor").eventData(binding: wssToHmi)
                == #"{"errorName":"communication","reason":"ENVELOPE_CORRUPT","transport":"wss","codec":"cbor"}"#)
        #expect(
            Signal.envelopeCorrupt(source: "ecu", codec: "typed").eventData(binding: wssToHmi)
                == #"{"errorName":"communication","reason":"ENVELOPE_CORRUPT","source":"ecu","transport":"wss","codec":"typed"}"#
        )
        #expect(
            Signal.missingSequence(source: "ecu").eventData(binding: wssToHmi)
                == #"{"errorName":"communication","reason":"MISSING_SEQUENCE","source":"ecu"}"#)
        #expect(
            Signal.dedupWindowOverflow(source: "ecu", windowSize: 256).eventData(binding: wssToHmi)
                == #"{"errorName":"communication","reason":"DEDUP_WINDOW_OVERFLOW","source":"ecu","window_size":256}"#)
    }

    /// Row 18: its `source`, then the epoch the envelope was tagged with and the replica that tagged it,
    /// and no transport.
    @Test func aViewMismatchNamesItsSourceEpochAndReplica() {
        #expect(
            Signal.causalViewMismatch(source: "list", epoch: 1, replica: "01010101-0101-0101-0101-010101010101")
                .eventData(binding: wssToHmi)
                == #"{"errorName":"communication","reason":"CAUSAL_VIEW_MISMATCH","source":"list","epoch":1,"replica":"01010101-0101-0101-0101-010101010101"}"#
        )
    }

    @Test func aGapEndedByATickHasNoBindingToName() {
        #expect(
            Signal.orderingGap(source: "ecu", lostLo: 4, lostHi: 6).eventData(binding: nil)
                == #"{"errorName":"communication","reason":"ORDERING_GAP","source":"ecu","lost_seq_lo":4,"lost_seq_hi":6}"#
        )
    }
}
