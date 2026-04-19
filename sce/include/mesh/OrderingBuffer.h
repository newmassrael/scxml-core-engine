// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh OrderingBuffer — per-sender sequence-ordered admit layer for
// transports that do not supply native per-source FIFO.
//
// SCE_MESH.md §10.6: bindings that declare `ordering: required` on a
// transport whose `supplies_ordering=false` (Zenoh, SOME/IP-UDP, DDS)
// stamp outbound envelopes with a monotonic `sequence_no` and route
// inbound envelopes through this buffer before reaching the engine.
//
// The buffer holds out-of-order envelopes until the expected next
// sequence arrives (sorted release), and fast-forwards past gaps on
// timeout (raising `error.communication.gap`). It is a sibling of
// `DedupRouter`: both are per-source, both are deploy.yaml-bounded,
// both keep non-evicting state because the sender roster is pinned
// at deploy time.
//
// Scope (what this class is NOT):
//   * No cross-source ordering. Each source has an independent
//     sequence domain — the receiver does not serialize multiple
//     senders into a global order.
//   * No buffering for unstamped envelopes. The generated admit path
//     drops envelopes that reach an active OrderingBuffer without a
//     `sequence_no` and raises `error.communication.missing_sequence`
//     (SCE_MESH.md §10.6.3). Topology guarantees all senders on an
//     ordered route stamp the field; a missing value signals an
//     out-of-sync sender, not a legitimate unordered message.
//   * No eviction of sender state. `deploy.yaml` pins the machine
//     roster, so `state_` is bounded by the number of sender
//     machines that actually send on an ordered route. Same argument
//     as `DedupRouter`.
//   * No interaction with §10.5 dedup. The codegen pipes admit paths
//     as dedup → ordering → dispatch; each layer is independent.
//
// Thread-safety: transport callback threads may call `admit()` and
// `tick()` concurrently. A single mutex covers the whole admit path;
// per-sender striping is an optimization that has not been measured
// as necessary — same rationale as DedupRouter. The mutex is released
// before return on both methods — no caller code (dispatch, raise)
// runs while the lock is held, so a slow engine cannot stall pending
// admits on the same buffer.

#pragma once

#include "mesh/MeshEnvelope.h"

#include <chrono>
#include <cstddef>
#include <cstdint>
#include <map>
#include <mutex>
#include <string>
#include <unordered_map>
#include <vector>

namespace SCE::Mesh {

/// A closed range of sequence numbers [lo, hi] that `tick()`
/// fast-forwarded past because the head envelope had been blocked
/// longer than `gap_timeout_`. Reported alongside released envelopes
/// so the caller can raise `error.communication` with reason
/// `ORDERING_GAP` (SCE_MESH.md §16.7 row 12) outside the buffer mutex.
struct OrderingGapEvent {
    std::string source;
    std::uint64_t lost_lo;
    std::uint64_t lost_hi;
};

/// Outcome of a single `tick()` call. `released` is the set of
/// envelopes that became contiguous with `next_expected_seq` during
/// this tick (including those released by a gap fast-forward);
/// `gaps` is the list of fast-forwarded ranges, one entry per source
/// that timed out during this tick. Both vectors are ordered by the
/// `state_` map iteration order (per-source); callers that need a
/// different order should sort explicitly.
///
/// Returning data (as opposed to invoking a callback) deliberately
/// keeps the buffer mutex narrow — the caller performs any engine
/// dispatch and `error.communication` raising OUTSIDE the lock, so a
/// slow or reentrant sender cannot stall every pending admit on the
/// same buffer.
struct OrderingTickResult {
    std::vector<MeshEnvelope> released;
    std::vector<OrderingGapEvent> gaps;
};

/// Per-source reorder buffer. Admit returns envelopes ready for
/// dispatch, in sequence order; tick fires gap timeouts that would
/// otherwise stall indefinitely when no further traffic arrives.
class OrderingBuffer {
public:
    /// Clock facility; `std::chrono::steady_clock` is monotonic and not
    /// affected by wall-clock adjustments, which matters when a gap
    /// timeout is the only thing preventing an engine from stalling.
    using Clock = std::chrono::steady_clock;

    /// Construct with an explicit gap timeout. The buffer carries no
    /// fallback constant: the value comes from deploy.yaml
    /// `machines.<name>.ordering.gap_timeout_ms` (SCE_MESH.md §10.6.1)
    /// — single source of truth — and the generated `TransportRouter`
    /// initializes the buffer with that value verbatim. Tests pass the
    /// timeout explicitly (e.g. 5 ms) to keep runtime short.
    explicit OrderingBuffer(std::chrono::milliseconds gap_timeout) noexcept
        : gap_timeout_(gap_timeout) {}

    /// Admit an envelope whose sender has stamped `sequence_no`.
    /// Returns envelopes ready to be dispatched to the engine in
    /// sequence order; an empty vector means the envelope was
    /// buffered (waiting for a gap to fill), and a late-arrival
    /// envelope whose sequence is below `next_expected_seq` is
    /// dropped silently (returned vector is empty).
    ///
    /// Pre-condition: `env.sequence_no.has_value()`. Unstamped
    /// envelopes are rejected by the generated admit path before
    /// reaching here (SCE_MESH.md §10.6.3) — this class does not
    /// re-check because the router's branch already guarantees the
    /// invariant.
    [[nodiscard]] std::vector<MeshEnvelope>
    admit(const std::string& source, MeshEnvelope env) {
        std::lock_guard<std::mutex> lock(mutex_);
        auto& state = state_[source];
        const std::uint64_t seq = *env.sequence_no;
        const auto now = Clock::now();

        if (!state.initialized) {
            // Receiver just joined the stream — anchor to the first
            // seen sequence. Without this, a receiver that subscribes
            // mid-stream (sender already at seq=7) would otherwise
            // wait `gap_timeout` for seq=1..6 before delivering
            // seq=7, which is not meaningful: 1..6 were not missed,
            // they were never destined for this receiver.
            state.next_expected_seq = seq;
            state.initialized = true;
        }

        if (seq < state.next_expected_seq) {
            // Late arrival (either a fast-forward straggler or a
            // duplicate already admitted). Silent drop — the engine
            // has already advanced past this point.
            return {};
        }

        if (seq > state.next_expected_seq) {
            state.pending.emplace(seq, Pending{std::move(env), now});
            return {};
        }

        // seq == next_expected_seq: release this one, then drain
        // contiguous higher sequences buffered from prior admits.
        std::vector<MeshEnvelope> released;
        released.push_back(std::move(env));
        state.next_expected_seq = seq + 1;
        drainContiguous(state, released);
        return released;
    }

    /// Fire gap timeouts for every source whose oldest buffered
    /// envelope has been waiting longer than `gap_timeout_`. For each
    /// timed-out gap, records an `OrderingGapEvent` in the returned
    /// result with the inclusive `[lost_lo, lost_hi]` range of
    /// skipped sequence numbers, advances `next_expected_seq` past
    /// the gap, and drains contiguous higher sequences.
    ///
    /// Returns both the released envelopes AND the gap events so the
    /// caller can perform any engine-side work (dispatch the data
    /// stream, raise `error.communication` with reason ORDERING_GAP)
    /// OUTSIDE the buffer mutex. Callback-under-mutex would expose
    /// every pending admit on the buffer to the sender engine's
    /// latency — the data-only return keeps the critical section
    /// narrow and independent of caller policy.
    ///
    /// Must be called periodically; generated `TransportRouter`s
    /// schedule this alongside their normal step loop so gap
    /// recovery doesn't depend on continued inbound traffic.
    [[nodiscard]] OrderingTickResult tick() {
        std::lock_guard<std::mutex> lock(mutex_);
        OrderingTickResult result;
        const auto now = Clock::now();
        for (auto& [source, state] : state_) {
            if (state.pending.empty()) continue;
            const auto& [head_seq, head_pending] = *state.pending.begin();
            if (head_seq == state.next_expected_seq) {
                // A stale state: head is the expected seq but somehow
                // didn't get released in admit. Drain it. This guards
                // against a future refactor that might enqueue the
                // expected-seq path by mistake. Not a gap — no
                // OrderingGapEvent recorded.
                result.released.push_back(std::move(state.pending.begin()->second.env));
                state.pending.erase(state.pending.begin());
                state.next_expected_seq += 1;
                drainContiguous(state, result.released);
                continue;
            }
            if (now - head_pending.arrived_at < gap_timeout_) {
                continue;  // not yet stale
            }
            const std::uint64_t lost_lo = state.next_expected_seq;
            const std::uint64_t lost_hi = head_seq - 1;
            state.next_expected_seq = head_seq;
            result.gaps.push_back({source, lost_lo, lost_hi});
            // Release the former head + any contiguous higher seqs.
            result.released.push_back(std::move(state.pending.begin()->second.env));
            state.pending.erase(state.pending.begin());
            state.next_expected_seq += 1;
            drainContiguous(state, result.released);
        }
        return result;
    }

private:
    struct Pending {
        MeshEnvelope env;
        Clock::time_point arrived_at;
    };

    struct PerSender {
        std::uint64_t next_expected_seq = 0;
        std::map<std::uint64_t, Pending> pending;
        bool initialized = false;
    };

    /// Drain any buffered envelopes whose sequence number is now
    /// contiguous with `state.next_expected_seq`. The released vector
    /// is grown in place so the admit/tick paths can return the full
    /// in-order batch to the caller. Must be called with `mutex_` held.
    void drainContiguous(PerSender& state, std::vector<MeshEnvelope>& released) {
        while (!state.pending.empty()) {
            auto it = state.pending.begin();
            if (it->first != state.next_expected_seq) break;
            released.push_back(std::move(it->second.env));
            state.next_expected_seq += 1;
            state.pending.erase(it);
        }
    }

    std::chrono::milliseconds gap_timeout_;
    std::mutex mutex_;
    // Bounded by deploy.yaml machine roster — see class-level comment.
    std::unordered_map<std::string, PerSender> state_;
};

}  // namespace SCE::Mesh
