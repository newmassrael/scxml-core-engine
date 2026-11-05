// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
    private func envelopeBytes(source: String = "ecu", id: Int, sequence: UInt64?) throws -> [UInt8] {
        var envelope = Envelope()
        envelope.id = envelopeID(id)
        envelope.source = source
        envelope.event_type = "a"
        envelope.pattern = .fireForget
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
        let first = try admitted(inbound.admit(envelopeBytes(id: 1, sequence: nil), delivery: delivery, nowMs: 0))
        #expect(first.signals.isEmpty)
        let second = try admitted(inbound.admit(envelopeBytes(id: 2, sequence: nil), delivery: delivery, nowMs: 0))
        #expect(second.signals == [.dedupWindowOverflow(source: "ecu", windowSize: 1)])
        #expect(second.released.count == 1)
    }

    @Test func aGapEndsWhenItsTimeoutHasPassedAndReleasesWhatWasHeld() throws {
        let inbound = Inbound(dedupWindow: 8, gapTimeoutMs: 50)
        let delivery = Delivery(dedup: true, ordered: true)
        _ = try admitted(inbound.admit(envelopeBytes(id: 1, sequence: 1), delivery: delivery, nowMs: 0))
        let held = try admitted(inbound.admit(envelopeBytes(id: 4, sequence: 4), delivery: delivery, nowMs: 10))
        #expect(held.released.isEmpty)

        #expect(try admitted(inbound.tick(nowMs: 59)).signals.isEmpty, "the gap has not waited its timeout")
        let ended = try admitted(inbound.tick(nowMs: 60))
        #expect(ended.signals == [.orderingGap(source: "ecu", lostLo: 2, lostHi: 3)])
        #expect(ended.released.map(\.sequence_no) == [4])
    }

    @Test func aSequenceBelowTheExpectedOneIsDropped() throws {
        let inbound = Inbound(dedupWindow: 8, gapTimeoutMs: 50)
        let delivery = Delivery(dedup: false, ordered: true)
        _ = try admitted(inbound.admit(envelopeBytes(id: 1, sequence: 5), delivery: delivery, nowMs: 0))
        _ = try admitted(inbound.admit(envelopeBytes(id: 2, sequence: 6), delivery: delivery, nowMs: 0))
        let late = try admitted(inbound.admit(envelopeBytes(id: 3, sequence: 5), delivery: delivery, nowMs: 0))
        #expect(late.released.isEmpty)
        #expect(late.signals.isEmpty)
    }

    @Test func aSequenceAtTheMaximumIsRefusedByTheRule() throws {
        let inbound = Inbound(dedupWindow: 8, gapTimeoutMs: 50)
        let delivery = Delivery(dedup: false, ordered: true)
        let refusal = inbound.admit(
            try envelopeBytes(id: 1, sequence: UInt64.max), delivery: delivery, nowMs: 0)
        guard case .refused(.rule) = refusal else {
            throw Unexpected(description: "expected a rule refusal, got \(refusal)")
        }
    }
}
