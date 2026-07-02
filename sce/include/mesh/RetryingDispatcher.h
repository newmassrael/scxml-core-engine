// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh RetryingDispatcher — §mesh-16.7 row 3 DELIVERY_EXHAUSTED retry layer.
//
// Sits above the OutboundBuffer's dispatcher boundary (RFC Q1=(a)
// wrap-above): the generated TransportRouter constructs this class
// once per opt-in `(machine, target)` binding whose deploy.yaml
// `retry:` block declared `max_retries > 0`, and the buffer's
// `Dispatcher` closure routes through `send_with_retry` instead of
// calling the raw transport send directly. The OutboundBuffer never
// learns about retry state — its "ready⟹empty" invariant and SEND_FAILED
// emit contract stay intact, and SEND_FAILED fires only when the
// retry layer is disabled (deploy.yaml omits the section).
//
// Behaviour summary (per RFC Q1..Q8):
//   * First failure: call the inner dispatcher synchronously. On
//     `SendResult::success()` return success and clean up no state.
//     On `failure(...)` with `retryable=true` and `max_retries>0`:
//     stash the envelope + attempt counter under `state_mutex_` and
//     schedule the first retry via the shared `MeshDeadlineScheduler`;
//     return `SendResult::success()` to the OutboundBuffer so it
//     does NOT raise SEND_FAILED (per Q6=(c) "SEND_FAILED only when
//     max_retries==0; DELIVERY_EXHAUSTED otherwise").
//   * Retry callback fires on the scheduler thread: re-call the
//     inner dispatcher. Success ⇒ erase state. Failure ⇒ bump
//     attempts, schedule next retry (if budget remains) or emit
//     DELIVERY_EXHAUSTED via the raise callback.
//   * Terminal classification: when the inner dispatcher stamps
//     `SendResult.retryable=false` (e.g. SOME/IP "app not
//     initialized" — the codegen sentinel for an unrecoverable
//     config state), the retry layer fast-fails with
//     `attempts=1`. The author can branch on `_event.data.attempts==1`
//     vs `>1` to distinguish "config error" from "tried and gave up".
//   * `cancelEnvelopeRetry(envelope_id)` (Q7) erases pending state
//     and the scheduler entry — used by §mesh-9.5 deadline preemption so
//     a mid-backoff envelope does not race the deadline's own
//     `error.invoke.<id>` raise.
//   * Same envelope id across attempts (Q5): retries reuse the
//     stashed `MeshEnvelope` verbatim — including the UUID v7 `id`
//     — so receiver-side §mesh-10.5 dedup correctly drops at-most-once.
//
// Backoff schedule (per RFC §Q2):
//   * Attempt 1 fails ⇒ wait `initial_backoff`
//   * Attempt 2 fails ⇒ wait `min(prev * multiplier, max_backoff)`
//   * Each interval is randomised by `±jitter_pct%` to avoid
//     thundering-herd on a transport recovery edge. Jitter source
//     is a per-instance `std::mt19937` seeded from a `std::random_device`
//     draw at construction time (deterministic across reruns of a
//     single binary, randomised across separate binary launches).
//
// Thread-safety:
//   * `send_with_retry` is safe from any thread — engine thread, the
//     OutboundBuffer's `markReady` drain thread, or any future
//     parallel-engine drain path. The `state_mutex_` protects the
//     per-envelope retry map; the inner dispatcher is invoked OUTSIDE
//     that mutex so the transport's own internal locking does not
//     compose with ours.
//   * The retry callback fires on the `MeshDeadlineScheduler` thread.
//     It enters `state_mutex_` briefly to pull the envelope copy,
//     releases the mutex, calls the inner dispatcher, re-enters the
//     mutex to update state or schedule the next attempt. The raise
//     callback (DELIVERY_EXHAUSTED) is invoked OUTSIDE the mutex so a
//     slow raise (engine queue lock) does not block subsequent
//     `send_with_retry` calls from other threads.
//   * Shutdown: when the owning TransportRouter destroys the
//     scheduler, any pending retry entries are silently dropped
//     (mirrors `MeshDeadlineScheduler::shutdown` semantics). The
//     RetryingDispatcher itself owns no thread.

#pragma once

#include "mesh/CommunicationError.h"
#include "mesh/MeshDeadlineScheduler.h"
#include "mesh/MeshEnvelope.h"
#include "mesh/MeshUuidKey.h"
#include "mesh/OutboundBuffer.h"

#include <algorithm>
#include <chrono>
#include <cstdint>
#include <functional>
#include <mutex>
#include <random>
#include <string>
#include <unordered_map>
#include <utility>

namespace SCE::Mesh {

class RetryingDispatcher {
public:
    /// Inner transport-send closure. Identical shape to the
    /// OutboundBuffer dispatcher (`SendResult(MeshEnvelope)`); the
    /// RetryingDispatcher wraps one of these and is itself installed
    /// as the OutboundBuffer's dispatcher.
    using InnerDispatcher = OutboundBuffer::Dispatcher;

    /// Raise callback used when the retry budget is exhausted.
    /// Mirrors the OutboundBuffer's `ErrorRaiseCallback` shape so the
    /// generated TransportRouter can pass `raiseCommunicationError`
    /// for both `OutboundBuffer` and `RetryingDispatcher` without an
    /// adapter.
    using ErrorRaiseCallback = std::function<void(CommunicationError)>;

    /// Policy carrying the deploy.yaml-derived retry parameters.
    /// `transport` and `target` are baked here so the DELIVERY_EXHAUSTED
    /// raise can populate the §mesh-16.7 row 3 extras without the runtime
    /// having to re-derive them from the envelope.
    struct Policy {
        std::uint32_t max_retries;
        std::chrono::milliseconds initial_backoff;
        double backoff_multiplier;
        std::chrono::milliseconds max_backoff;
        std::uint32_t jitter_pct;  // 0..=100
        std::string transport;
        std::string target;
    };

    RetryingDispatcher(MeshDeadlineScheduler &scheduler, Policy policy, InnerDispatcher inner, ErrorRaiseCallback raise)
        : scheduler_(scheduler), policy_(std::move(policy)), inner_(std::move(inner)), raise_(std::move(raise)),
          rng_(std::random_device{}()) {}

    RetryingDispatcher(const RetryingDispatcher &) = delete;
    RetryingDispatcher &operator=(const RetryingDispatcher &) = delete;
    RetryingDispatcher(RetryingDispatcher &&) = delete;
    RetryingDispatcher &operator=(RetryingDispatcher &&) = delete;

    /// Dispatcher entry point — installed AS the OutboundBuffer's
    /// dispatcher closure. Returns `SendResult::success()` whenever the
    /// envelope has been accepted for delivery (either succeeded
    /// synchronously or queued for retry), so the OutboundBuffer's
    /// `admit`/`markReady` paths do not raise SEND_FAILED. Returns
    /// `SendResult::failure(...)` (passing through the inner result)
    /// when no retries are configured AND the inner dispatcher
    /// declined — the OutboundBuffer then raises SEND_FAILED per
    /// Stage 1/2 behaviour. This branch is only reachable when
    /// `policy_.max_retries == 0`, which the deploy.yaml validator
    /// rejects at parse time; the codepath survives as a defensive
    /// no-op in case a future caller constructs the RetryingDispatcher
    /// programmatically with `max_retries=0`.
    ///
    /// SCE_MESH.md §mesh-16.7 row 3: entry to the DELIVERY_EXHAUSTED retry
    /// layer — one synchronous attempt, then queue for exponential-backoff
    /// retry; exhaustion raises `error.communication` reason DELIVERY_EXHAUSTED.
    SendResult send_with_retry(const MeshEnvelope &env) {
        SendResult result = inner_(env);
        if (result.ok) {
            return SendResult::success();
        }
        if (policy_.max_retries == 0) {
            // Defensive: deploy.yaml rejects this, but a programmatic
            // construction may bypass it. Pass through so the
            // OutboundBuffer raises SEND_FAILED per Stage 1/2.
            return result;
        }
        if (!result.retryable) {
            // Terminal: fast-fail with attempts=1 and DELIVERY_EXHAUSTED.
            emitExhausted(env, /*attempts=*/1, std::move(result.transport_error));
            return SendResult::success();
        }
        // Transient with retries available: stash + schedule first retry.
        const auto &key = env.id;
        std::chrono::milliseconds next_backoff;
        {
            std::lock_guard<std::mutex> lock(state_mutex_);
            // If a prior retry chain for this same envelope id is
            // still in flight (the §mesh-10.5 envelope id is meant to be
            // unique per envelope, so this should not happen — but
            // be robust to a buggy caller), refuse to re-enter and
            // surface the original failure so SEND_FAILED fires.
            if (state_.count(key) != 0) {
                return result;
            }
            RetryState st;
            st.env = env;
            st.attempts = 1;
            st.last_transport_error = std::move(result.transport_error);
            st.next_backoff = policy_.initial_backoff;
            next_backoff = jitter(st.next_backoff);
            state_.emplace(key, std::move(st));
        }
        const bool scheduled = scheduler_.registerDeadline(key, next_backoff, [this, key]() { onRetryFire(key); });
        if (!scheduled) {
            // Scheduler refused (shutting down or duplicate). Roll
            // back state and fail through to SEND_FAILED so the
            // envelope is not silently dropped.
            std::lock_guard<std::mutex> lock(state_mutex_);
            state_.erase(key);
            return SendResult::failure();
        }
        return SendResult::success();
    }

    /// Cancel a pending retry for `envelope_id` (RFC Q7 — used by
    /// §mesh-9.5 deadline preemption so a mid-backoff envelope is not
    /// raced by the deadline's own `error.invoke.<id>` raise).
    /// Idempotent: returns `true` when an entry was erased, `false`
    /// when the id was never registered or already exhausted/fired.
    bool cancelEnvelopeRetry(const MeshDeadlineScheduler::Key &envelope_id) {
        {
            std::lock_guard<std::mutex> lock(state_mutex_);
            if (state_.erase(envelope_id) == 0) {
                return false;
            }
        }
        scheduler_.cancelDeadline(envelope_id);
        return true;
    }

    /// Number of envelopes with a pending retry slot. Useful for
    /// test assertions; not intended for hot path use.
    std::size_t pendingCount() const {
        std::lock_guard<std::mutex> lock(state_mutex_);
        return state_.size();
    }

private:
    struct RetryState {
        MeshEnvelope env;
        std::uint32_t attempts;  // count of attempts ALREADY MADE
        std::optional<std::string> last_transport_error;
        std::chrono::milliseconds next_backoff;
    };

    /// SCE_MESH.md §mesh-16.7 row 3: the retry-timer callback that re-sends a
    /// queued envelope, re-arms exponential backoff, and raises
    /// DELIVERY_EXHAUSTED once `max_retries` attempts are reached.
    void onRetryFire(const MeshDeadlineScheduler::Key &key) {
        MeshEnvelope env_copy;
        {
            std::lock_guard<std::mutex> lock(state_mutex_);
            auto it = state_.find(key);
            if (it == state_.end()) {
                return;  // cancelled while we were waiting
            }
            env_copy = it->second.env;
        }
        SendResult result = inner_(env_copy);
        if (result.ok) {
            std::lock_guard<std::mutex> lock(state_mutex_);
            state_.erase(key);
            return;
        }
        // Failure on retry. Re-enter the mutex to update state.
        bool exhausted = false;
        std::uint32_t exhausted_attempts = 0;
        std::optional<std::string> exhausted_error;
        MeshEnvelope exhausted_env;
        std::chrono::milliseconds next_backoff{0};
        {
            std::lock_guard<std::mutex> lock(state_mutex_);
            auto it = state_.find(key);
            if (it == state_.end()) {
                // Cancelled between the unlock above and re-acquire.
                return;
            }
            auto &st = it->second;
            st.attempts += 1;
            st.last_transport_error = result.transport_error;
            if (!result.retryable || st.attempts > policy_.max_retries) {
                // Exhausted: total attempts is st.attempts (1 initial
                // + up to max_retries additional). Hand the data out
                // and erase before releasing the lock so a concurrent
                // cancelEnvelopeRetry sees the post-exhaustion state.
                exhausted = true;
                exhausted_attempts = st.attempts;
                exhausted_error = std::move(st.last_transport_error);
                exhausted_env = std::move(st.env);
                state_.erase(it);
            } else {
                // Bump backoff with multiplier cap.
                const auto raw_next =
                    static_cast<long long>(static_cast<double>(st.next_backoff.count()) * policy_.backoff_multiplier);
                const auto capped = std::min<long long>(raw_next, policy_.max_backoff.count());
                st.next_backoff = std::chrono::milliseconds(capped);
                next_backoff = jitter(st.next_backoff);
            }
        }
        if (exhausted) {
            emitExhausted(exhausted_env, exhausted_attempts, std::move(exhausted_error));
            return;
        }
        const bool scheduled = scheduler_.registerDeadline(key, next_backoff, [this, key]() { onRetryFire(key); });
        if (!scheduled) {
            // Scheduler is shutting down. Drop state — §mesh-9.5 "benign
            // drop" matches MeshDeadlineScheduler::shutdown semantics.
            std::lock_guard<std::mutex> lock(state_mutex_);
            state_.erase(key);
        }
    }

    void emitExhausted(const MeshEnvelope &env, std::uint32_t attempts,
                       std::optional<std::string> last_transport_error) {
        if (!raise_) {
            return;
        }
        CommunicationError err;
        err.reason = ::SCE::Mesh::ReasonCode::DeliveryExhausted;
        err.target = policy_.target;
        err.transport = policy_.transport;
        if (last_transport_error) {
            err.transport_error = std::move(*last_transport_error);
        }
        err.attempts = static_cast<std::int64_t>(attempts);
        // Surface the envelope id so authors can correlate the
        // exhaustion event with the originating <send>. Reuses the
        // existing CommunicationError.envelope_id baseline field
        // rather than adding a row-3 specific one.
        err.envelope_id = env.id;
        raise_(std::move(err));
    }

    /// Apply ±jitter_pct% randomisation to a backoff interval.
    /// `0` ⇒ no jitter (deterministic). `100` ⇒ full ±100% (interval
    /// chosen uniformly in `[0, 2*computed]`). The lower clamp at
    /// `1ms` keeps the scheduler thread from spinning on a
    /// rounded-to-zero jitter draw when the computed value is small.
    std::chrono::milliseconds jitter(std::chrono::milliseconds computed) {
        if (policy_.jitter_pct == 0) {
            return computed;
        }
        const auto base = computed.count();
        const auto delta_max = (base * static_cast<long long>(policy_.jitter_pct)) / 100;
        std::lock_guard<std::mutex> lock(rng_mutex_);
        std::uniform_int_distribution<long long> dist(-delta_max, delta_max);
        const auto jittered = base + dist(rng_);
        return std::chrono::milliseconds(std::max<long long>(1, jittered));
    }

    MeshDeadlineScheduler &scheduler_;
    Policy policy_;
    InnerDispatcher inner_;
    ErrorRaiseCallback raise_;

    mutable std::mutex state_mutex_;
    std::unordered_map<MeshDeadlineScheduler::Key, RetryState, MeshUuidKeyHash> state_;

    std::mutex rng_mutex_;
    std::mt19937 rng_;
};

}  // namespace SCE::Mesh
