// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What the router core tells the engine besides the envelopes themselves: the `error.communication`
// rows of SCE_MESH.md §mesh-16.7 that the core, rather than a transport, is in a position to
// observe. The Swift twin of backends/kotlin/mesh Signal.kt and backends/rust/mesh/src/signal.rs,
// row for row and byte for byte.

import SceRuntime

/// One §mesh-16.7 row, with the extras the row carries.
public enum Signal: Equatable, Sendable {
    /// `orderingGap` (row 12): `source`'s sequences `lostLo`...`lostHi` waited out the gap timeout.
    case orderingGap(source: String, lostLo: UInt64, lostHi: UInt64)

    /// `dedupWindowOverflow` (row 7): a novel id was admitted to a full window of `windowSize` ids.
    case dedupWindowOverflow(source: String, windowSize: UInt32)

    /// `envelopeCorrupt` (row 4): what arrived could not be read. `codec` is `"cbor"` when the bytes
    /// are not an envelope the standard document accepts, or the payload's own codec when the
    /// envelope decoded but its payload is not one the engine can be handed; then the envelope also
    /// named its `source`.
    case envelopeCorrupt(source: String?, codec: String)

    /// `missingSequence` (row 11): an envelope reached an ordered binding without a `sequence_no`.
    case missingSequence(source: String)

    /// `backpressureDrop` (row 9): the target's queue already held `depth` envelopes.
    case backpressureDrop(depth: UInt32)

    /// `outboundStaleDrop` (row 15): a queued envelope had waited `ageMs`, past `maxAgeMs`.
    case outboundStaleDrop(ageMs: Int64, maxAgeMs: Int64)

    /// `transportUnavailable` (row 1).
    case transportUnavailable

    /// `sendFailed` (row 2): declined with no retry configured; `transportError` is the transport's
    /// words.
    case sendFailed(transportError: String?)

    /// `deliveryExhausted` (row 3): given up after `attempts` sends.
    case deliveryExhausted(attempts: UInt32, transportError: String?)

    /// `rpcReplyFromUndeclaredPeer` (row 14): a reply from `source` named the live request
    /// `invokeID` (the wire id, RFC 4122 text) but arrived on a binding outside that request's
    /// responder set (§mesh-14.6). The request stays answerable.
    case rpcReplyFromUndeclaredPeer(source: String, invokeID: String)

    /// `invokeChildLost` (row 5): the link to `target` was lost while the request `invokeID` (the
    /// wire id, RFC 4122 text) waited on it. The request is forgotten, as a cancel forgets it
    /// (§mesh-9.5), since no reply can arrive on a link that is gone.
    case invokeChildLost(invokeID: String, target: String)

    /// `causalViewMismatch` (row 18): an envelope from `source` on a causal route was tagged with
    /// `epoch`, another than the receiver's view, or by the replica `replica` (RFC 4122 text) the view
    /// does not hold (docs/adr/0085, section 3.2). It was not delivered.
    case causalViewMismatch(source: String, epoch: UInt64, replica: String)

    /// The row's code, a member of the engines' one vocabulary (sce:std/engine/error_reason.scxml,
    /// docs/adr/0106 #111), whose communication half is §mesh-16.7 row by row.
    public var code: ErrorReason {
        switch self {
        case .orderingGap: ErrorReason.orderingGap
        case .dedupWindowOverflow: ErrorReason.dedupWindowOverflow
        case .envelopeCorrupt: ErrorReason.envelopeCorrupt
        case .missingSequence: ErrorReason.missingSequence
        case .backpressureDrop: ErrorReason.backpressureDrop
        case .outboundStaleDrop: ErrorReason.outboundStaleDrop
        case .transportUnavailable: ErrorReason.transportUnavailable
        case .sendFailed: ErrorReason.sendFailed
        case .deliveryExhausted: ErrorReason.deliveryExhausted
        case .rpcReplyFromUndeclaredPeer: ErrorReason.rpcReplyFromUndeclaredPeer
        case .invokeChildLost: ErrorReason.invokeChildLost
        case .causalViewMismatch: ErrorReason.causalViewMismatch
        }
    }

    /// The row's `reason`, the code's declared name.
    public var reason: String {
        code.declaredName
    }

    /// The rows a send observes: their columns name the `target` the envelope was bound for.
    private var isSending: Bool {
        switch self {
        case .backpressureDrop, .outboundStaleDrop, .transportUnavailable, .sendFailed, .deliveryExhausted:
            true
        default:
            false
        }
    }

    /// The machine a receiving row names as `source`, when it names one.
    private var source: String? {
        switch self {
        case let .orderingGap(source, _, _): source
        case let .dedupWindowOverflow(source, _): source
        case let .missingSequence(source): source
        case let .envelopeCorrupt(source, _): source
        case let .rpcReplyFromUndeclaredPeer(source, _): source
        case let .causalViewMismatch(source, _, _): source
        default: nil
        }
    }

    /// The `_event.data` of the `error.communication` this row raises: byte for byte what the C++
    /// core's `CommunicationError::toJsonBytes` writes for the same row: `errorName`, `reason`, then
    /// each present field in that header's declaration order, absent ones omitted. `binding`
    /// supplies `target` and `transport` to the rows whose columns name them.
    public func eventData(binding: Binding?) -> String {
        var json = ErrorData(errorName: "communication", reason: reason)
        var transportRow = isSending
        if case .envelopeCorrupt = self {
            transportRow = true
        }
        if let source {
            json.text("source", source)
        }
        if let binding {
            if isSending {
                json.text("target", binding.peer)
            }
            if transportRow, let transport = binding.transport {
                json.text("transport", transport)
            }
        }
        switch self {
        case let .sendFailed(transportError):
            if let transportError {
                json.text("transport_error", transportError)
            }
        case let .deliveryExhausted(attempts, transportError):
            if let transportError {
                json.text("transport_error", transportError)
            }
            json.number("attempts", attempts)
        case let .envelopeCorrupt(_, codec):
            json.text("codec", codec)
        case let .backpressureDrop(depth):
            json.number("queue_depth", depth)
        case let .dedupWindowOverflow(_, windowSize):
            json.number("window_size", windowSize)
        case let .outboundStaleDrop(ageMs, maxAgeMs):
            json.number("age_ms", ageMs)
            json.number("max_age_ms", maxAgeMs)
        case let .orderingGap(_, lostLo, lostHi):
            json.number("lost_seq_lo", lostLo)
            json.number("lost_seq_hi", lostHi)
        case let .rpcReplyFromUndeclaredPeer(_, invokeID):
            // §mesh-16.7 row 14's columns: `source`, then the request it tried to retire.
            json.text("invoke_id", invokeID)
        case let .invokeChildLost(invokeID, target):
            // Row 5 names its own `target` column, after `invoke_id`, where the C++ core's field
            // order puts it.
            json.text("invoke_id", invokeID)
            json.text("target", target)
        case let .causalViewMismatch(_, epoch, replica):
            // Row 18's columns: `source`, then the epoch the envelope was tagged with and the replica
            // that tagged it.
            json.number("epoch", epoch)
            json.text("replica", replica)
        case .missingSequence, .transportUnavailable:
            break
        }
        return json.finish()
    }
}

/// The deployment binding a row is observed on: the peer it names and the transport kind that
/// carries it. A send to a peer the deployment never bound names the peer and no transport.
public struct Binding: Equatable, Sendable {
    public var peer: String
    public var transport: String?

    public init(peer: String, transport: String?) {
        self.peer = peer
        self.transport = transport
    }
}
