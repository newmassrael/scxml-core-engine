// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The router core's receive half: an envelope off the wire, through duplicate suppression
// (SCE_MESH.md §mesh-10.5) and receiver ordering (§mesh-10.6), to the envelopes the engine may now
// see, in order. The Swift twin of backends/kotlin/mesh Inbound.kt and
// backends/rust/mesh/src/inbound.rs, with the semantics of the C++ `OrderingBuffer` and
// `DedupRouter`.
//
// Every decision is a call into the generated standard rules (`sce:std/mesh`); what this adds is the
// state they are applied to, kept per sender, and the envelopes themselves while they are held. The
// ordering rules keep at most 256 held envelopes on the bounded backends and grow on the heap
// backends (stdlib/mesh/ordering_hold.scxml), so this one, a heap backend, has no full hold to
// report.

import SceForgeRuntime

/// What one binding asks of the envelopes it receives: dedup only where the transport does not
/// already suppress duplicates (§mesh-10.5), ordering only where the binding declares it and the
/// transport does not supply it (§mesh-10.6).
public struct Delivery: Equatable, Sendable {
    public var dedup: Bool
    public var ordered: Bool

    public init(dedup: Bool, ordered: Bool) {
        self.dedup = dedup
        self.ordered = ordered
    }
}

/// What one call released, in the order the engine must see it, and what it signalled.
public struct Outcome {
    public var released: [Envelope] = []
    public var signals: [Signal] = []
}

/// Why an envelope was not admitted.
public enum AdmitError: Equatable, Sendable {
    /// The bytes are not an envelope the standard document accepts.
    case malformed

    /// The binding is ordered and the envelope carries no `sequence_no` (§mesh-10.6.3). Refused
    /// before the dedup window sees its id, so a refused envelope does not take a window slot.
    case unstamped

    /// A rule refused its input: a sequence at the `UInt64` maximum, or a clock that cannot be
    /// subtracted.
    case rule(AlgorithmError)
}

/// An admission's answer: what it released, or why it was refused.
public enum Admission {
    case admitted(Outcome)
    case refused(AdmitError)
}

/// A rule of `sce:std/mesh` throws only the contract's own failures (`AlgorithmError`), which is
/// what the receive half answers with.
func ruleFailure(_ error: any Error) -> AlgorithmError {
    guard let failure = error as? AlgorithmError else {
        preconditionFailure("a standard Mesh rule threw \(error), which is not an AlgorithmError")
    }
    return failure
}

/// One sender's receive state.
private final class Sender {
    var window: [EnvelopeIdPayload] = []

    /// The sequence this sender is expected to send next; nil until its first ordered envelope,
    /// which anchors it (a receiver that joins mid-stream did not miss what was sent before it
    /// listened).
    var next: UInt64?
    var pending: [OrderingSlotPayload] = []
    var held: [UInt64: Envelope] = [:]

    /// Release everything held that follows on from `start` without a gap, and expect what comes
    /// after it.
    func releaseFrom(_ start: UInt64, into out: inout Outcome) throws {
        let drained = try OrderingDrain.orderingDrain(pending, start)
        for sequence in held.keys.filter({ $0 < drained }).sorted() {
            if let envelope = held.removeValue(forKey: sequence) {
                out.released.append(envelope)
            }
        }
        pending = OrderingPrune.orderingPrune(pending, drained)
        next = drained
    }
}

/// The receive half of a router: every sender's window and held envelopes, with deploy.yaml's
/// `window_size` (§mesh-10.5) and `gap_timeout_ms` (§mesh-10.6.1).
public final class Inbound {
    private let dedupWindow: UInt32
    private let gapTimeoutMs: Int64
    private var senders: [String: Sender] = [:]

    public init(dedupWindow: UInt32, gapTimeoutMs: Int64) {
        precondition(dedupWindow > 0, "deploy.yaml's window_size is 0; §mesh-10.5 needs at least one id")
        precondition(gapTimeoutMs >= 0, "deploy.yaml's gap_timeout_ms is negative")
        self.dedupWindow = dedupWindow
        self.gapTimeoutMs = gapTimeoutMs
    }

    /// Admit the envelope `bytes` from a binding asking `delivery`, at `nowMs` on the host's monotonic
    /// clock. §mesh-10.6.4: an ordered envelope below its sender's expected sequence is dropped, one
    /// above it is held, and the one it expects is released together with every held envelope that
    /// now follows on.
    public func admit(_ bytes: [UInt8], delivery: Delivery, nowMs: Int64) -> Admission {
        var cursor = SceCursor(bytes)
        guard let envelope = try? Envelope.decode(&cursor) else {
            return .refused(.malformed)
        }
        let sequence = envelope.sequence_no
        if delivery.ordered && sequence == nil {
            return .refused(.unstamped)
        }
        do {
            return .admitted(try admitDecoded(envelope, sequence: sequence, delivery: delivery, nowMs: nowMs))
        } catch {
            return .refused(.rule(ruleFailure(error)))
        }
    }

    private func admitDecoded(
        _ envelope: Envelope, sequence: UInt64?, delivery: Delivery, nowMs: Int64
    ) throws -> Outcome {
        var out = Outcome()
        let sender = senderFor(envelope.source)
        if delivery.dedup {
            let id = envelopeID(envelope.id)
            if DedupHolds.dedupHolds(sender.window, id) {
                return out
            }
            let full = sender.window.count >= Int(dedupWindow)
            sender.window = try DedupAdmit.dedupAdmit(sender.window, id, dedupWindow)
            if full {
                out.signals.append(.dedupWindowOverflow(source: envelope.source, windowSize: dedupWindow))
            }
        }
        guard delivery.ordered, let sequence else {
            out.released.append(envelope)
            return out
        }
        let next: UInt64
        if let anchored = sender.next {
            next = anchored
        } else {
            next = sequence
            sender.next = sequence
        }
        if sequence < next {
            return out
        }
        if sequence > next {
            sender.pending = OrderingHold.orderingHold(sender.pending, sequence, nowMs)
            // A re-delivery of a held sequence keeps its first copy, as ordering_hold keeps its
            // first arrival time.
            if sender.held[sequence] == nil {
                sender.held[sequence] = envelope
            }
            return out
        }
        out.released.append(envelope)
        if sequence == UInt64.max {
            throw AlgorithmError.overflow
        }
        try sender.releaseFrom(sequence + 1, into: &out)
        return out
    }

    private func senderFor(_ source: String) -> Sender {
        if let sender = senders[source] {
            return sender
        }
        let sender = Sender()
        senders[source] = sender
        return sender
    }

    /// End every gap that has waited `gap_timeout_ms` by `nowMs`: signal the sequences lost and
    /// release what was held behind them. The host calls this on its own schedule, so a gap ends
    /// even when nothing else arrives from that sender. Senders are visited in name order, as the
    /// Rust core visits them.
    public func tick(nowMs: Int64) -> Admission {
        do {
            var out = Outcome()
            for source in senders.keys.sorted() {
                guard let sender = senders[source], let next = sender.next else {
                    continue
                }
                let end = try OrderingGapEnd.orderingGapEnd(sender.pending, next, nowMs, gapTimeoutMs)
                if end == next {
                    continue
                }
                out.signals.append(.orderingGap(source: source, lostLo: next, lostHi: end - 1))
                try sender.releaseFrom(end, into: &out)
            }
            return .admitted(out)
        } catch {
            return .refused(.rule(ruleFailure(error)))
        }
    }
}

/// An envelope id as the rules hold it: its first eight bytes and its last eight, big-endian
/// (`sce:std/mesh/envelope_id`). The envelope document fixes `id` at exactly 16 bytes, so a decoded
/// id always splits.
private func envelopeID(_ id: [UInt8]) -> EnvelopeIdPayload {
    func half(_ from: Int) -> UInt64 {
        var value: UInt64 = 0
        for index in from..<(from + 8) {
            value = (value << 8) | UInt64(id[index])
        }
        return value
    }
    return EnvelopeIdPayload(hi: half(0), lo: half(8))
}
