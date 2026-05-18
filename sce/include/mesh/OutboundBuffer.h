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
// OutboundBuffer is the third generic SCE mesh primitive, a sibling of
// §10.5 `DedupRouter` (inbound duplicate suppression) and §10.6
// `OrderingBuffer` (inbound reorder). All three share the same shape:
//   * generic SCE-owned header, consumed by codegen via jinja;
//   * per-sender or per-target keying axis bounded by deploy.yaml roster;
//   * single transport-agnostic admit entry; state transition API fed
//     by transport-specific callbacks.
//
// Scope (what this class is NOT):
//   * No retry policy. Transport-level send failure after readiness is
//     surfaced by the dispatch callback returning false; the buffer
//     does not re-enqueue or retry (SCE_MESH.md §16.7 row 2
//     `SEND_FAILED` and row 3 `DELIVERY_EXHAUSTED` are orthogonal).
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
#include <string>
#include <utility>

namespace SCE::Mesh {

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
    /// Transport-specific send. Returns `true` on accepted-by-transport
    /// (a later transport failure surfaces through its own error path,
    /// not by a delayed return). Called under the buffer mutex for
    /// FIFO preservation — see class-level thread-safety note.
    using Dispatcher = std::function<bool(const MeshEnvelope&)>;

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
    /// Returns `true` if the envelope was dispatched or buffered;
    /// `false` if it was dropped on overflow.
    [[nodiscard]] bool admit(const MeshEnvelope& env) {
        bool overflow = false;
        std::size_t depth_at_overflow = 0;
        bool accepted = false;
        {
            std::lock_guard<std::mutex> lock(mu_);
            if (ready_ && queue_.empty()) {
                // Fast path: dispatch under the lock so a racing
                // markReady cannot slip buffered envelopes past this
                // direct send out of FIFO.
                accepted = dispatch_(env);
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
        return accepted;
    }

    /// Transport readiness became true. Drain queued envelopes in FIFO
    /// order through the dispatcher. Idempotent: calling markReady on an
    /// already-ready buffer is a no-op (queue is empty by invariant).
    ///
    /// The dispatcher runs under the mutex so a concurrent `admit` whose
    /// fast path would otherwise race ahead cannot interleave with the
    /// drain. See class-level thread-safety note for the rationale that
    /// this does not become a throughput bottleneck in practice.
    void markReady() {
        std::lock_guard<std::mutex> lock(mu_);
        ready_ = true;
        while (!queue_.empty()) {
            MeshEnvelope env = std::move(queue_.front());
            queue_.pop_front();
            (void)dispatch_(env);
        }
    }

    /// Transport readiness became false. Subsequent admits enqueue
    /// until the next `markReady`. Does not clear the queue — envelopes
    /// buffered while temporarily ready remain, so a readiness flicker
    /// does not lose in-flight work.
    ///
    /// SCE_MESH.md §10.4.1 + §16.7 row 1: a `true → false` transition
    /// is the "Active → Disconnected" lifecycle edge and raises
    /// `error.communication` with `reason = "TRANSPORT_UNAVAILABLE"`.
    /// Repeated `markNotReady` calls while already not-ready are
    /// idempotent and DO NOT re-emit — Row 1 fires per-transition,
    /// not per-callback (a transport callback that re-asserts the
    /// same state is not a new transport fault). The initial seed
    /// state `ready_=false` therefore does NOT emit on the first
    /// `markNotReady`: no Active phase preceded the call, so there
    /// is no transition.
    ///
    /// The raise closure is invoked OUTSIDE the buffer mutex to
    /// preserve the §10.10 lock-discipline contract (raise paths
    /// must never run under `mu_`, mirroring `admit`'s overflow
    /// raise).
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
