// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The send and receive halves on their own: what the router tests reach only through a whole
// router. Every expectation is a decision of a generated rule (`sce:std/mesh`), seen through the
// state this core keeps for it.

import SceMesh
import Testing

@Suite struct OutboundTests {
    private let buffer = OutboundBuffer(maxPending: 2, maxAgeMs: 100)

    @Test func withoutABufferEverythingIsSentAtOnce() {
        let outbound = Outbound(buffer: nil)
        #expect(outbound.admit([1], nowMs: 0) == .send([1]))
        #expect(outbound.markNotReady() == nil, "a target that was never ready had no connection to lose")
    }

    @Test func aTargetThatIsNotReadyQueuesUntilItsQueueIsFull() {
        let outbound = Outbound(buffer: buffer)
        #expect(outbound.admit([1], nowMs: 0) == .queued)
        #expect(outbound.admit([2], nowMs: 0) == .queued)
        #expect(outbound.admit([3], nowMs: 0) == .dropped(.backpressureDrop(depth: 2)))
        #expect(outbound.depth == 2)
    }

    @Test func readinessReleasesTheQueueInOrderLessWhatWaitedTooLong() throws {
        let outbound = Outbound(buffer: buffer)
        _ = outbound.admit([1], nowMs: 0)
        _ = outbound.admit([2], nowMs: 60)
        let drained = try outbound.markReady(nowMs: 150)
        #expect(drained.send == [[2]])
        #expect(drained.signals == [.outboundStaleDrop(ageMs: 150, maxAgeMs: 100)])
        #expect(outbound.depth == 0)
    }

    @Test func aReadyTargetSendsAtOnceOnlyWhenNothingWaitsAheadOfIt() throws {
        let outbound = Outbound(buffer: buffer)
        _ = outbound.admit([1], nowMs: 0)
        _ = try outbound.markReady(nowMs: 1)
        #expect(outbound.admit([2], nowMs: 2) == .send([2]))
        #expect(outbound.markNotReady() == .transportUnavailable)
        #expect(outbound.markNotReady() == nil, "only the ready-to-not-ready edge signals")
    }
}

@Suite struct AttemptsTests {
    private let policy = RetryPolicy(
        maxRetries: 2, initialBackoffMs: 100, backoffMultiplier: 2.0, maxBackoffMs: 150, jitterPct: 0)

    @Test func withoutAPolicyTheFirstFailureIsFinal() throws {
        let attempts = Attempts()
        #expect(
            try attempts.afterFailure(policy: nil, retryable: true, transportError: "reset", draw: 0)
                == .giveUp(.sendFailed(transportError: "reset")))
    }

    @Test func theBackoffGrowsToItsCapAndThenTheRetriesRunOut() throws {
        let attempts = Attempts()
        #expect(
            try attempts.afterFailure(policy: policy, retryable: true, transportError: nil, draw: 0)
                == .retryIn(waitMs: 100))
        #expect(
            try attempts.afterFailure(policy: policy, retryable: true, transportError: nil, draw: 0)
                == .retryIn(waitMs: 150))
        #expect(
            try attempts.afterFailure(policy: policy, retryable: true, transportError: "gone", draw: 0)
                == .giveUp(.deliveryExhausted(attempts: 3, transportError: "gone")))
        #expect(attempts.made == 3)
    }

    @Test func aTerminalFailureIsNotRetried() throws {
        let attempts = Attempts()
        #expect(
            try attempts.afterFailure(policy: policy, retryable: false, transportError: nil, draw: 0)
                == .giveUp(.deliveryExhausted(attempts: 1, transportError: nil)))
    }
}

@Suite struct InboundTests {
    private func envelopeBytes(
        source: String = "ecu", id: Int, sequence: UInt64?, pattern: PatternKind = .fireForget
    ) throws -> [UInt8] {
        var envelope = Envelope()
        envelope.id = envelopeID(id)
        envelope.source = source
        envelope.event_type = "a"
        envelope.pattern = pattern
        envelope.sequence_no = sequence
        return try encodeEnvelope(envelope)
    }

    private func admitted(_ admission: Admission) throws -> Outcome {
        guard case let .admitted(outcome) = admission else {
            throw Unexpected(description: "refused: \(admission)")
        }
        return outcome
    }

    @Test func aFullWindowSignalsOnceWhenAnotherIdIsAdmitted() throws {
        let inbound = Inbound(dedupWindow: 1, gapTimeoutMs: 50)
        let delivery = Delivery(dedup: true, ordered: false)
        let first = try admitted(
            inbound.admit(link: "ecu", envelopeBytes(id: 1, sequence: nil), delivery: delivery, nowMs: 0))
        #expect(first.signals.isEmpty)
        let second = try admitted(
            inbound.admit(link: "ecu", envelopeBytes(id: 2, sequence: nil), delivery: delivery, nowMs: 0))
        #expect(second.signals == [.dedupWindowOverflow(source: "ecu", windowSize: 1)])
        #expect(second.released.count == 1)
    }

    @Test func aGapEndsWhenItsTimeoutHasPassedAndReleasesWhatWasHeld() throws {
        let inbound = Inbound(dedupWindow: 8, gapTimeoutMs: 50)
        let delivery = Delivery(dedup: true, ordered: true)
        _ = try admitted(inbound.admit(link: "ecu", envelopeBytes(id: 1, sequence: 1), delivery: delivery, nowMs: 0))
        let held = try admitted(
            inbound.admit(link: "ecu", envelopeBytes(id: 4, sequence: 4), delivery: delivery, nowMs: 10))
        #expect(held.released.isEmpty)

        #expect(try admitted(inbound.tick(nowMs: 59)).signals.isEmpty, "the gap has not waited its timeout")
        let ended = try admitted(inbound.tick(nowMs: 60))
        #expect(ended.signals == [.orderingGap(source: "ecu", lostLo: 2, lostHi: 3)])
        #expect(ended.released.map(\.sequence_no) == [4])
    }

    @Test func aSequenceBelowTheExpectedOneIsDropped() throws {
        let inbound = Inbound(dedupWindow: 8, gapTimeoutMs: 50)
        let delivery = Delivery(dedup: false, ordered: true)
        _ = try admitted(inbound.admit(link: "ecu", envelopeBytes(id: 1, sequence: 5), delivery: delivery, nowMs: 0))
        _ = try admitted(inbound.admit(link: "ecu", envelopeBytes(id: 2, sequence: 6), delivery: delivery, nowMs: 0))
        let late = try admitted(
            inbound.admit(link: "ecu", envelopeBytes(id: 3, sequence: 5), delivery: delivery, nowMs: 0))
        #expect(late.released.isEmpty)
        #expect(late.signals.isEmpty)
    }

    /// SCE_MESH.md section 10.6.4: a reply and a field notification are not in their sender's
    /// sequence. Unnumbered on an ordered binding, each is released at once, past a gap the stream is
    /// waiting on, and the duplicate window still drops a second copy. Red if the receiver asks them
    /// for a number: each is then refused as `unstamped`.
    @Test func anUnnumberedReplyOrFieldNotificationPassesAnOrderedBinding() throws {
        let inbound = Inbound(dedupWindow: 8, gapTimeoutMs: 50)
        let delivery = Delivery(dedup: true, ordered: true)
        let anchored = try admitted(
            inbound.admit(link: "ecu", envelopeBytes(id: 1, sequence: 1), delivery: delivery, nowMs: 0))
        #expect(anchored.released.map(\.sequence_no) == [1])
        let held = try admitted(
            inbound.admit(link: "ecu", envelopeBytes(id: 3, sequence: 3), delivery: delivery, nowMs: 0))
        #expect(held.released.isEmpty)
        for (id, pattern) in [(10, PatternKind.rpcReply), (11, PatternKind.fieldNotify)] {
            let bytes = try envelopeBytes(id: id, sequence: nil, pattern: pattern)
            let passed = try admitted(inbound.admit(link: "ecu", bytes, delivery: delivery, nowMs: 0))
            #expect(passed.released.map(\.pattern) == [pattern], "\(pattern) is released at once")
            let again = try admitted(inbound.admit(link: "ecu", bytes, delivery: delivery, nowMs: 0))
            #expect(again.released.isEmpty, "a second copy of \(pattern) is a duplicate")
        }
        // The stream still waits for 2, and 3 follows it.
        let filled = try admitted(
            inbound.admit(link: "ecu", envelopeBytes(id: 2, sequence: 2), delivery: delivery, nowMs: 0))
        #expect(filled.released.map(\.sequence_no) == [2, 3])
    }

    /// SCE_MESH.md section 10.6.4: a reply its sender numbered anyway is passed as one that carries no
    /// number. It neither anchors the stream nor is held behind a gap, and its number is never waited
    /// for, so no gap times out. Red if a numbered reply is ordered: the first one would anchor the
    /// stream at 9 and the envelope numbered 1 would be dropped as a straggler.
    @Test func aNumberedReplyTakesNoPlaceInTheStream() throws {
        let inbound = Inbound(dedupWindow: 8, gapTimeoutMs: 50)
        let delivery = Delivery(dedup: false, ordered: true)
        let first = try admitted(
            inbound.admit(
                link: "ecu", envelopeBytes(id: 9, sequence: 9, pattern: .rpcReply), delivery: delivery, nowMs: 0))
        #expect(first.released.map(\.pattern) == [.rpcReply])
        for sequence: UInt64 in [1, 2] {
            let step = try admitted(
                inbound.admit(
                    link: "ecu", envelopeBytes(id: Int(sequence), sequence: sequence), delivery: delivery,
                    nowMs: 10))
            #expect(step.released.map(\.sequence_no) == [Optional(sequence)])
        }
        let between = try admitted(
            inbound.admit(
                link: "ecu", envelopeBytes(id: 20, sequence: 3, pattern: .rpcReply), delivery: delivery, nowMs: 20))
        #expect(between.released.count == 1)
        let third = try admitted(
            inbound.admit(link: "ecu", envelopeBytes(id: 3, sequence: 3), delivery: delivery, nowMs: 30))
        #expect(third.released.map(\.sequence_no) == [3])
        let later = try admitted(inbound.tick(nowMs: 10_000))
        #expect(later.released.isEmpty && later.signals.isEmpty, "no gap times out: \(later.signals)")
    }

    /// Every other pattern keeps its place: an unnumbered one on an ordered binding is refused.
    @Test func anUnnumberedSendOnAnOrderedBindingIsStillRefused() throws {
        let inbound = Inbound(dedupWindow: 8, gapTimeoutMs: 50)
        let refusal = inbound.admit(
            link: "ecu", try envelopeBytes(id: 1, sequence: nil), delivery: Delivery(dedup: false, ordered: true),
            nowMs: 0)
        guard case .refused(.unstamped) = refusal else {
            throw Unexpected(description: "expected an unstamped refusal, got \(refusal)")
        }
    }

    @Test func aSequenceAtTheMaximumIsRefusedByTheRule() throws {
        let inbound = Inbound(dedupWindow: 8, gapTimeoutMs: 50)
        let delivery = Delivery(dedup: false, ordered: true)
        let refusal = inbound.admit(
            link: "ecu", try envelopeBytes(id: 1, sequence: UInt64.max), delivery: delivery, nowMs: 0)
        guard case .refused(.rule) = refusal else {
            throw Unexpected(description: "expected a rule refusal, got \(refusal)")
        }
    }

    /// SCE_MESH.md section 10.5: the windows are as many as the links. Ten thousand envelopes on one
    /// link, each claiming a source of its own, make one sender; a second link is a second. Red if the
    /// window is keyed by the envelope's `source`: every claimed name then makes a window of its own.
    @Test func envelopesClaimingManySourcesOnOneLinkMakeOneWindow() throws {
        let inbound = Inbound(dedupWindow: 4, gapTimeoutMs: 50)
        let delivery = Delivery(dedup: true, ordered: false)
        for n in 0..<10_000 {
            var envelope = Envelope()
            envelope.id = [UInt8](repeating: 0x5A, count: 12) + [
                UInt8(truncatingIfNeeded: n >> 24), UInt8(truncatingIfNeeded: n >> 16),
                UInt8(truncatingIfNeeded: n >> 8), UInt8(truncatingIfNeeded: n),
            ]
            envelope.source = "s\(n)"
            envelope.event_type = "a"
            envelope.pattern = .fireForget
            let bytes = try encodeEnvelope(envelope)
            _ = try admitted(inbound.admit(link: "phone", bytes, delivery: delivery, nowMs: 0))
        }
        #expect(inbound.senderCount == 1)
        let other = try envelopeBytes(source: "x", id: 1, sequence: nil)
        _ = try admitted(inbound.admit(link: "tablet", other, delivery: delivery, nowMs: 0))
        #expect(inbound.senderCount == 2)
    }
}
