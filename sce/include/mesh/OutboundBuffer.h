// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh OutboundBuffer — per-target readiness-gated outbound admit layer.
//
// SCE_MESH.md §10.10: transports whose peer may not yet be ready at the
// moment of `route_send` (SOME/IP before `offer_service` completes,
// Zenoh before any subscriber declares on a PUT-style keyexpr) silently
// drop outbound envelopes with no `error.communication`. A buffer sits
// between `route_send` and the transport-specific send function: if the
// target is not ready, the envelope is queued; when the transport's
// native readiness primitive fires (vsomeip `register_availability_handler`,
// Zenoh `Publisher::declare_matching_listener`), the buffer drains in
// FIFO order.
//
// SCE_MESH.md §10.4.1 transport-lifecycle "Active → Disconnected"
// observer: the same readiness primitive that drives drain also
// witnesses transport loss. When `markNotReady()` observes a true→false
// transition (transport was Active and is now Disconnected — TCP RST,
// SOME/IP availability=false, Zenoh peer-drop), the buffer raises
// `error.communication` with `reason = "TRANSPORT_UNAVAILABLE"`
// (§16.7 row 1). The initial Ready→Active anchor (the buffer's seed
// state is `ready_=false`, the first transition fires `markReady`) does
// not emit because no Active lifecycle phase preceded it.
//
// Row 2 `SEND_FAILED` is emitted at every dispatcher-fail observation
// point: the `admit` fast path (dispatcher under `ready=true ∧ queue
// empty`) and the `markReady` drain loop (each queued envelope's
// dispatch attempt). The §10.4.1 "Enqueued-but-unsent envelopes are
// failed individually on Active→Disconnected" clause is satisfied by
// construction in OutboundBuffer's design: `ready_=true` implies
// `queue.empty()` (the fast path dispatches; the drain holds `mu_`
// for its entire scope so admits cannot enqueue mid-drain). There is
// therefore no non-empty queue at the moment of `markNotReady`;
// `markNotReady` emits Row 1 only. The dispatcher closure surfaces
// the underlying transport-API decline through `SendResult` (whose
// `transport_error` value populates the §16.7 row 2
// `CommunicationError::transport_error` field): SOME/IP stamps a
// sentinel when `app.send()` returns false (vsomeip exposes no errno
// equivalent), Zenoh stamps `ZException::what()` when
// `Publisher::put` throws.
//
// OutboundBuffer is the third generic SCE mesh primitive, a sibling of
// §10.5 `DedupRouter` (inbound duplicate suppression) and §10.6
// `OrderingBuffer` (inbound reorder). All three share the same shape:
//   * generic SCE-owned header, consumed by codegen via jinja;
//   * per-sender or per-target keying axis bounded by deploy.yaml roster;
//   * single transport-agnostic admit entry; state transition API fed
//     by transport-specific callbacks.
//
// Scope (what this class is NOT):
//   * No retry policy. Transport-level send failure observed at the
//     dispatcher boundary (returning false from admit fast path or
//     markReady drain) is surfaced as `error.communication` with
//     `reason = "SEND_FAILED"` (§16.7 row 2) and the envelope is
//     considered consumed — the buffer does not re-enqueue or retry.
//     SCE_MESH.md §16.7 row 3 `DELIVERY_EXHAUSTED` (retry-exhausted)
//     is orthogonal and lives in a future retry-layer atomic.
//   * No age-based drop. Overflow policy is fixed at
//     `BACKPRESSURE_DROP` (§16.7 row 9) + drop-newest. Grammar
//     additions (`max_age_ms`, `overflow: drop_oldest`) are deferred
//     until a consumer lands.
//   * No cross-target serialization. Each OutboundBuffer instance is
//     per-target (deploy.yaml roster axis); admit + drain are
//     serialized only within one target. A router with N opt-in
//     targets holds N independent buffers.
//   * No routing_id / sequence_no stamp. SCE_MESH.md §10.9 invariant 1
//     ("every envelope leaving a TransportRouter carries routing_id")
//     is honored by the dispatch callback capturing the router's
//     stamp path — a buffered envelope has not yet left the router,
//     and the stamp happens at dispatch time.
//
// Thread-safety: `admit()` runs on the thread that owns route_send
// (typically the engine step loop). `markReady()` / `markNotReady()`
// run on transport callback threads (vsomeip application threads,
// zenoh runtime threads). The buffer holds one mutex across admit and
// transition paths; the dispatch callback is invoked under the mutex
// to preserve FIFO order (a concurrent `admit` whose fast path would
// otherwise race past an in-progress drain sees the lock held and
// serializes). Transport send functions are non-blocking at the API
// boundary (vsomeip enqueues to its worker thread; zenoh
// `publisher.put` is queue-to-runtime), so the critical section does
// not become a throughput bottleneck. Mirrors DedupRouter's
// single-mutex posture; per-target striping would be the optimization
// path if a measured bottleneck appeared.

#pragma once

#include "mesh/CommunicationError.h"
#include "mesh/MeshEnvelope.h"

#include <cstddef>
#include <deque>
#include <functional>
#include <mutex>
#include <optional>
#include <string>
#include <utility>
#include <vector>

namespace SCE::Mesh {

/// Dispatcher result for an attempted outbound send.
///
/// SCE_MESH.md §16.7 row 2 SEND_FAILED carries a `transport_error`
/// extra naming the underlying transport-API error surface so the
/// SCXML author can correlate the loss with transport telemetry. The
/// dispatcher closure populates `transport_error` on failure (vsomeip
/// `app.send()` returning false yields a sentinel; zenoh
/// `Publisher::put` throwing yields `ZException::what()`); the
/// OutboundBuffer relays it untouched into the
/// `CommunicationError::transport_error` field at the raise site.
///
/// On the happy path a dispatcher returns `success()` and the
/// transport_error stays empty — the field is therefore opt-in:
/// successful sends never allocate a string.
struct SendResult {
    bool ok = false;
    /// Populated only when `!ok`. Absent when the dispatcher cannot
    /// surface a transport-API message (rare; e.g. SOME/IP returns
    /// false with no errno mapping today — the codegen lambda still
    /// stamps a sentinel so this stays populated, but the field
    /// remains optional for future dispatchers that can't).
    std::optional<std::string> transport_error;

    [[nodiscard]] static SendResult success() {
        return SendResult{true, std::nullopt};
    }

    [[nodiscard]] static SendResult failure(std::string err) {
        return SendResult{false, std::move(err)};
    }

    [[nodiscard]] static SendResult failure() {
        return SendResult{false, std::nullopt};
    }
};

/// Per-target readiness-gated outbound admit layer.
///
/// One instance per opt-in `(machine, target)` pair. Generated
/// TransportRouter code constructs this in its ctor with a dispatcher
/// closure (bound to the target-specific transport send function) and
/// an error raise closure (bound to `raiseCommunicationError`). The
/// transport's native readiness callback calls `markReady()` /
/// `markNotReady()` on the 0↔1 transition.
///
/// The class is copy- and move- disabled because the captured closures
/// reference router-scoped state; the router owns the buffer by value.
class OutboundBuffer {
public:
    /// Transport-specific send. Returns a `SendResult` whose `ok`
    /// flag indicates whether the transport accepted the envelope and
    /// whose `transport_error` (when present) names the underlying
    /// API decline so the SCE_MESH.md §16.7 row 2 raise can populate
    /// `CommunicationError::transport_error`. Called under the buffer
    /// mutex for FIFO preservation — see class-level thread-safety
    /// note.
    using Dispatcher = std::function<SendResult(const MeshEnvelope&)>;

    /// Error raise closure. Called when admit observes buffer overflow
    /// (queue depth >= `max_pending`). Invoked OUTSIDE the mutex so a
    /// slow `raiseCommunicationError` path cannot stall concurrent
    /// admits on the same target.
    using ErrorRaise = std::function<void(CommunicationError)>;

    OutboundBuffer(std::string target,
                   std::size_t max_pending,
                   std::string transport_name,
                   Dispatcher dispatch,
                   ErrorRaise raise_error)
        : target_(std::move(target)),
          transport_name_(std::move(transport_name)),
          max_pending_(max_pending),
          dispatch_(std::move(dispatch)),
          raise_error_(std::move(raise_error)) {}

    OutboundBuffer(const OutboundBuffer&)            = delete;
    OutboundBuffer& operator=(const OutboundBuffer&) = delete;
    OutboundBuffer(OutboundBuffer&&)                 = delete;
    OutboundBuffer& operator=(OutboundBuffer&&)      = delete;

    /// Admit an outbound envelope. Three paths:
    ///   * ready + empty queue ⇒ dispatch immediately (fast path).
    ///   * not ready ⇒ enqueue (up to `max_pending`); on overflow,
    ///     raise `error.communication` with reason `BACKPRESSURE_DROP`
    ///     (§16.7 row 9) and drop the newest envelope.
    ///   * ready + non-empty queue (transient mid-drain) ⇒ enqueue to
    ///     preserve FIFO; the in-progress drain will release it.
    ///
    /// SCE_MESH.md §16.7 row 2: when the fast path's dispatcher
    /// returns a non-ok `SendResult`, `error.communication` with
    /// `reason = "SEND_FAILED"` is raised — the transport API said
    /// no, the envelope is gone, and the SCXML author must observe
    /// the loss. The dispatcher's `transport_error` (when populated)
    /// is relayed into the raised `CommunicationError::transport_error`
    /// field for transport-telemetry correlation. Raised OUTSIDE the
    /// buffer mutex.
    ///
    /// Returns `true` if the envelope was dispatched-or-buffered
    /// successfully; `false` if it was dropped on overflow OR if
    /// the fast-path dispatch was declined by the transport API.
    [[nodiscard]] bool admit(const MeshEnvelope& env) {
        bool overflow = false;
        std::size_t depth_at_overflow = 0;
        bool send_failed = false;
        std::optional<std::string> send_failed_transport_error;
        bool accepted = false;
        {
            std::lock_guard<std::mutex> lock(mu_);
            if (ready_ && queue_.empty()) {
                // Fast path: dispatch under the lock so a racing
                // markReady cannot slip buffered envelopes past this
                // direct send out of FIFO.
                SendResult result = dispatch_(env);
                accepted = result.ok;
                if (!result.ok) {
                    send_failed = true;
                    send_failed_transport_error =
                        std::move(result.transport_error);
                }
            } else if (queue_.size() >= max_pending_) {
                overflow = true;
                depth_at_overflow = queue_.size();
            } else {
                queue_.push_back(env);
                accepted = true;
            }
        }
        if (overflow) {
            CommunicationError err;
            err.reason = "BACKPRESSURE_DROP";
            err.transport = transport_name_;
            err.target = target_;
            err.queue_depth = static_cast<std::int64_t>(depth_at_overflow);
            raise_error_(std::move(err));
            return false;
        }
        if (send_failed) {
            CommunicationError err;
            err.reason = "SEND_FAILED";
            err.target = target_;
            err.transport = transport_name_;
            if (send_failed_transport_error) {
                err.transport_error =
                    std::move(*send_failed_transport_error);
            }
            raise_error_(std::move(err));
        }
        return accepted;
    }

    /// Transport readiness became true. Drain queued envelopes in FIFO
    /// order through the dispatcher. Idempotent: calling markReady on an
    /// already-ready buffer is a no-op (queue is empty by invariant).
    ///
    /// SCE_MESH.md §16.7 row 2: each drained envelope whose dispatcher
    /// returns a non-ok `SendResult` raises `error.communication`
    /// with `reason = "SEND_FAILED"` and the dispatcher's
    /// `transport_error` (when populated) is relayed into the raised
    /// `CommunicationError::transport_error` field. The raises run
    /// OUTSIDE the buffer mutex (after the drain releases `mu_`),
    /// preserving §10.10 lock-discipline; the per-envelope
    /// transport_error strings are captured into a vector under the
    /// mutex during the drain and the raises are emitted one-to-one
    /// in a single post-drain loop.
    ///
    /// The dispatcher runs under the mutex so a concurrent `admit` whose
    /// fast path would otherwise race ahead cannot interleave with the
    /// drain. See class-level thread-safety note for the rationale that
    /// this does not become a throughput bottleneck in practice.
    void markReady() {
        // §10.10 lock-discipline: capture per-envelope transport_error
        // strings under the mutex during drain, emit the raises after
        // the mutex releases. A vector<optional<string>> preserves the
        // "this envelope's API decline had a message" / "this one did
        // not" distinction one-to-one with the drain sequence, so each
        // post-drain raise carries the correct (or absent)
        // transport_error.
        std::vector<std::optional<std::string>> failures;
        {
            std::lock_guard<std::mutex> lock(mu_);
            ready_ = true;
            while (!queue_.empty()) {
                MeshEnvelope env = std::move(queue_.front());
                queue_.pop_front();
                SendResult result = dispatch_(env);
                if (!result.ok) {
                    failures.push_back(std::move(result.transport_error));
                }
            }
        }
        for (auto& transport_error : failures) {
            CommunicationError err;
            err.reason = "SEND_FAILED";
            err.target = target_;
            err.transport = transport_name_;
            if (transport_error) {
                err.transport_error = std::move(*transport_error);
            }
            raise_error_(std::move(err));
        }
    }

    /// Transport readiness became false. Subsequent admits enqueue
    /// until the next `markReady`.
    ///
    /// SCE_MESH.md §10.4.1 + §16.7 row 1: a `true → false` transition
    /// is the "Active → Disconnected" lifecycle edge and raises
    /// `error.communication` with `reason = "TRANSPORT_UNAVAILABLE"`.
    /// Repeated `markNotReady` calls while already not-ready are
    /// idempotent and DO NOT re-emit. The initial seed state
    /// `ready_=false` therefore does NOT emit on the first call: no
    /// Active phase preceded it, so there is no transition.
    ///
    /// The §10.4.1 disconnect-drain "Enqueued-but-unsent envelopes
    /// are failed individually" clause is satisfied vacuously:
    /// `ready_=true` implies `queue.empty()` in OutboundBuffer's
    /// design (admit fast-paths under ready+empty; markReady's drain
    /// holds `mu_` so concurrent admits cannot land in the
    /// "ready + non-empty queue" branch during drain). The queue
    /// at the disconnect moment is therefore empty by construction.
    /// Row 2 emit lives at the dispatcher-fail observation points
    /// (`admit` fast path, `markReady` drain).
    ///
    /// The raise closure runs OUTSIDE the buffer mutex to preserve
    /// §10.10 lock-discipline.
    void markNotReady() {
        bool was_ready = false;
        {
            std::lock_guard<std::mutex> lock(mu_);
            was_ready = ready_;
            ready_ = false;
        }
        if (was_ready) {
            CommunicationError err;
            err.reason = "TRANSPORT_UNAVAILABLE";
            err.target = target_;
            err.transport = transport_name_;
            raise_error_(std::move(err));
        }
    }

    /// Current queue depth. Test-only accessor — production code reads
    /// the envelope through `admit` / readiness callbacks, not this.
    [[nodiscard]] std::size_t queue_depth() const {
        std::lock_guard<std::mutex> lock(mu_);
        return queue_.size();
    }

    /// Readiness flag snapshot. Test-only; no ordering guarantee with
    /// concurrent markReady / markNotReady calls on other threads.
    [[nodiscard]] bool ready() const {
        std::lock_guard<std::mutex> lock(mu_);
        return ready_;
    }

private:
    const std::string target_;
    const std::string transport_name_;
    const std::size_t max_pending_;
    Dispatcher dispatch_;
    ErrorRaise raise_error_;

    mutable std::mutex mu_;
    std::deque<MeshEnvelope> queue_;
    bool ready_ = false;
};

}  // namespace SCE::Mesh
