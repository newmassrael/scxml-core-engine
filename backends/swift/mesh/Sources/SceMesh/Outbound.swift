// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The router core's send half, for one target: readiness-gated buffering and retry (SCE_MESH.md
// §mesh-10.10), with the semantics of the C++ `OutboundBuffer` and `RetryingDispatcher`. The Swift
// twin of backends/kotlin/mesh Outbound.kt and backends/rust/mesh/src/outbound.rs. Sans-IO: every
// call returns what to send and what to signal, and the host does the sending. Every decision is a
// call into the generated rules (`sce:std/mesh`).

import SceForgeRuntime

/// What became of an envelope handed to `Outbound.admit`.
public enum Admitted: Equatable, Sendable {
    /// Send these bytes now.
    case send([UInt8])

    /// Held until the target is ready.
    case queued

    /// Dropped: the queue was full.
    case dropped(Signal)
}

/// What a target becoming ready releases: the queued envelopes still worth sending, in the order
/// they were admitted, and a signal for each one that waited too long.
public struct Drained: Equatable, Sendable {
    public var send: [[UInt8]] = []
    public var signals: [Signal] = []
}

/// deploy.yaml's `outbound_buffer:` section (§mesh-10.10): how many envelopes may wait for a target
/// to become ready (`maxPending`, `max_pending_per_target`), and for how long (`maxAgeMs`, 0 for no
/// bound).
public struct OutboundBuffer: Equatable, Sendable {
    public var maxPending: UInt32
    public var maxAgeMs: Int64

    public init(maxPending: UInt32, maxAgeMs: Int64) {
        self.maxPending = maxPending
        self.maxAgeMs = maxAgeMs
    }
}

/// One target's outbound side: its readiness, and the buffer that holds envelopes until it is ready
/// when the deployment declares one. Nil when the machine declares no `outbound_buffer:` section:
/// then §mesh-10.10 dispatches every envelope directly, ready or not, and the transport reports what
/// it could not send. A target starts not ready: nothing has said it is.
public final class Outbound {
    private struct Queued {
        var bytes: [UInt8]
        var enqueuedAtMs: Int64
    }

    private let buffer: OutboundBuffer?
    private var ready = false
    private var queue: [Queued] = []

    public init(buffer: OutboundBuffer?) {
        self.buffer = buffer
    }

    /// Envelopes waiting.
    public var depth: UInt32 {
        UInt32(queue.count)
    }

    /// Admit `bytes`, sent by the host at `nowMs`. Without a buffer it is sent at once (§mesh-10.10's
    /// opt-in gate). With one, it is sent at once only when the target is ready and nothing waits
    /// ahead of it: an envelope sent past a queue would overtake the ones waiting.
    public func admit(_ bytes: [UInt8], nowMs: Int64) -> Admitted {
        guard let buffer else {
            return .send(bytes)
        }
        let depth = depth
        if OutboundSendsNow.outboundSendsNow(ready, depth) {
            return .send(bytes)
        }
        if OutboundOverflows.outboundOverflows(depth, buffer.maxPending) {
            return .dropped(.backpressureDrop(depth: depth))
        }
        queue.append(Queued(bytes: bytes, enqueuedAtMs: nowMs))
        return .queued
    }

    /// The target became ready at `nowMs`: release the queue in order, less what waited longer than
    /// the bound.
    public func markReady(nowMs: Int64) throws -> Drained {
        ready = true
        var out = Drained()
        // Without a buffer nothing was queued, so there is nothing to release.
        let maxAgeMs = buffer?.maxAgeMs ?? 0
        let waiting = queue
        queue.removeAll()
        for (index, queued) in waiting.enumerated() {
            let stale: Bool
            do {
                stale = try OutboundStale.outboundStale(queued.enqueuedAtMs, nowMs, maxAgeMs)
            } catch {
                // As the Kotlin and Rust cores leave it: the envelope the rule refused is taken
                // off the queue, and what it has not looked at stays queued.
                queue = Array(waiting[(index + 1)...])
                throw error
            }
            if stale {
                out.signals.append(.outboundStaleDrop(ageMs: nowMs - queued.enqueuedAtMs, maxAgeMs: maxAgeMs))
            } else {
                out.send.append(queued.bytes)
            }
        }
        return out
    }

    /// The target stopped being ready. Signals only on the ready-to-not-ready edge: a target that
    /// was never ready had no connection to lose.
    public func markNotReady() -> Signal? {
        let wasReady = ready
        ready = false
        return wasReady ? .transportUnavailable : nil
    }
}

/// deploy.yaml's `retry` block (§mesh-10.10). A target without one gives up on the first failed send.
public struct RetryPolicy: Equatable, Sendable {
    public var maxRetries: UInt32
    public var initialBackoffMs: Int64
    public var backoffMultiplier: Double
    public var maxBackoffMs: Int64
    public var jitterPct: Int64

    public init(
        maxRetries: UInt32, initialBackoffMs: Int64, backoffMultiplier: Double, maxBackoffMs: Int64,
        jitterPct: Int64
    ) {
        self.maxRetries = maxRetries
        self.initialBackoffMs = initialBackoffMs
        self.backoffMultiplier = backoffMultiplier
        self.maxBackoffMs = maxBackoffMs
        self.jitterPct = jitterPct
    }
}

/// What to do after a send failed.
public enum AfterFailure: Equatable, Sendable {
    /// Send the same envelope again after `waitMs`.
    case retryIn(waitMs: Int64)

    /// Stop; the envelope is lost, and this is why.
    case giveUp(Signal)
}

/// One envelope's retry history: the sends made and the wait last chosen before jitter.
public final class Attempts {
    /// Sends made so far.
    public private(set) var made: UInt32 = 0
    private var lastBackoffMs: Int64?

    public init() {}

    /// The send just made failed. `retryable` is the transport's reading of its failure; `draw` is a
    /// non-negative number the host drew uniformly for jitter, so two backends given the same draw
    /// wait the same time.
    public func afterFailure(
        policy: RetryPolicy?, retryable: Bool, transportError: String?, draw: Int64
    ) throws -> AfterFailure {
        made += 1
        guard let policy else {
            return .giveUp(.sendFailed(transportError: transportError))
        }
        if try RetryExhausted.retryExhausted(made, policy.maxRetries, retryable) {
            return .giveUp(.deliveryExhausted(attempts: made, transportError: transportError))
        }
        let backoff: Int64
        if let previous = lastBackoffMs {
            backoff = try RetryNextBackoff.retryNextBackoff(previous, policy.backoffMultiplier, policy.maxBackoffMs)
        } else {
            backoff = policy.initialBackoffMs
        }
        lastBackoffMs = backoff
        return .retryIn(waitMs: try RetryJittered.retryJittered(backoff, policy.jitterPct, draw))
    }
}
