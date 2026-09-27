// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The router core's send half, for one target: readiness-gated buffering and
// retry (SCE_MESH.md §mesh-10.10), with the semantics of the C++
// `OutboundBuffer` and `RetryingDispatcher`. The Kotlin twin of
// backends/rust/mesh/src/outbound.rs. Sans-IO: every call returns what to send
// and what to signal, and the host does the sending. Every decision is a call
// into the generated rules (`sce:std/mesh`).

package com.sce.mesh

import com.sce.forge.runtime.AlgorithmResult
import com.sce.generated.outbound_overflows.outboundOverflows
import com.sce.generated.outbound_sends_now.outboundSendsNow
import com.sce.generated.outbound_stale.outboundStale
import com.sce.generated.retry_exhausted.retryExhausted
import com.sce.generated.retry_jittered.retryJittered
import com.sce.generated.retry_next_backoff.retryNextBackoff

/** What became of an envelope handed to [Outbound.admit]. */
sealed class Admitted {
    /** Send these bytes now. */
    class Send(val bytes: ByteArray) : Admitted()

    /** Held until the target is ready. */
    data object Queued : Admitted()

    /** Dropped: the queue was full. */
    data class Dropped(val signal: Signal) : Admitted()
}

/**
 * What a target becoming ready releases: the queued envelopes still worth
 * sending, in the order they were admitted, and a signal for each one that
 * waited too long.
 */
class Drained {
    val send: MutableList<ByteArray> = mutableListOf()
    val signals: MutableList<Signal> = mutableListOf()
}

/**
 * One target's outbound buffer, with deploy.yaml's `max_pending_per_target`
 * and `max_age_ms` (0 for no bound). A target starts not ready: nothing has
 * said it is.
 */
class Outbound(private val maxPending: UInt, private val maxAgeMs: Long) {
    private class Queued(val bytes: ByteArray, val enqueuedAtMs: Long)

    private var ready = false
    private val queue = ArrayDeque<Queued>()

    /** Envelopes waiting. */
    val depth: UInt get() = queue.size.toUInt()

    /**
     * Admit [bytes], sent by the host at [nowMs]: sent at once only when the
     * target is ready and nothing waits ahead of it — an envelope sent past a
     * queue would overtake the ones waiting.
     */
    fun admit(bytes: ByteArray, nowMs: Long): Admitted {
        val depth = depth
        if (outboundSendsNow(ready, depth)) return Admitted.Send(bytes)
        if (outboundOverflows(depth, maxPending)) return Admitted.Dropped(Signal.BackpressureDrop(depth))
        queue.addLast(Queued(bytes, nowMs))
        return Admitted.Queued
    }

    /**
     * The target became ready at [nowMs]: release the queue in order, less
     * what waited longer than the bound.
     */
    fun markReady(nowMs: Long): AlgorithmResult<Drained> {
        ready = true
        val out = Drained()
        while (queue.isNotEmpty()) {
            val queued = queue.removeFirst()
            when (val stale = outboundStale(queued.enqueuedAtMs, nowMs, maxAgeMs)) {
                is AlgorithmResult.Failed -> return stale
                is AlgorithmResult.Ok ->
                    if (stale.value) {
                        out.signals += Signal.OutboundStaleDrop(nowMs - queued.enqueuedAtMs, maxAgeMs)
                    } else {
                        out.send += queued.bytes
                    }
            }
        }
        return AlgorithmResult.Ok(out)
    }

    /**
     * The target stopped being ready. Signals only on the ready-to-not-ready
     * edge: a target that was never ready had no connection to lose.
     */
    fun markNotReady(): Signal? {
        val wasReady = ready
        ready = false
        return if (wasReady) Signal.TransportUnavailable else null
    }
}

/** deploy.yaml's `retry` block (§mesh-10.10). A target without one gives up on the first failed send. */
data class RetryPolicy(
    val maxRetries: UInt,
    val initialBackoffMs: Long,
    val backoffMultiplier: Double,
    val maxBackoffMs: Long,
    val jitterPct: Long,
)

/** What to do after a send failed. */
sealed class AfterFailure {
    /** Send the same envelope again after [waitMs]. */
    data class RetryIn(val waitMs: Long) : AfterFailure()

    /** Stop; the envelope is lost, and this is why. */
    data class GiveUp(val signal: Signal) : AfterFailure()
}

/** One envelope's retry history: the sends made and the wait last chosen before jitter. */
class Attempts {
    /** Sends made so far. */
    var made: UInt = 0u
        private set
    private var lastBackoffMs: Long? = null

    /**
     * The send just made failed. [retryable] is the transport's reading of
     * its failure; [draw] is a non-negative number the host drew uniformly for
     * jitter, so two backends given the same draw wait the same time.
     */
    fun afterFailure(
        policy: RetryPolicy?,
        retryable: Boolean,
        transportError: String?,
        draw: Long,
    ): AlgorithmResult<AfterFailure> {
        made += 1u
        if (policy == null) return AlgorithmResult.Ok(AfterFailure.GiveUp(Signal.SendFailed(transportError)))
        when (val exhausted = retryExhausted(made, policy.maxRetries, retryable)) {
            is AlgorithmResult.Failed -> return exhausted
            is AlgorithmResult.Ok ->
                if (exhausted.value) {
                    return AlgorithmResult.Ok(AfterFailure.GiveUp(Signal.DeliveryExhausted(made, transportError)))
                }
        }
        val backoff = when (val prev = lastBackoffMs) {
            null -> policy.initialBackoffMs
            else -> when (val next = retryNextBackoff(prev, policy.backoffMultiplier, policy.maxBackoffMs)) {
                is AlgorithmResult.Failed -> return next
                is AlgorithmResult.Ok -> next.value
            }
        }
        lastBackoffMs = backoff
        return when (val wait = retryJittered(backoff, policy.jitterPct, draw)) {
            is AlgorithmResult.Failed -> wait
            is AlgorithmResult.Ok -> AlgorithmResult.Ok(AfterFailure.RetryIn(wait.value))
        }
    }
}
