// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh DedupRouter — per-envelope duplicate suppression for inbound
// transport callbacks.
//
// SCE_MESH.md §mesh-10.5: transports that cannot guarantee at-most-once
// envelope delivery (Zenoh's reliable mode can still reorder across
// routers, broadcast buses can bridge duplicates) need an
// application-level filter. Each receiving TransportRouter holds one
// DedupRouter per envelope source (`MeshEnvelope.source`, i.e. the
// sender machine name); each source owns a fixed-size window of
// recently-seen envelope ids. On inbound delivery, the router consults
// the window for that source and drops the envelope if the id is
// already present.
//
// The window size is a deploy.yaml property (SCE_MESH.md §mesh-10.5
// "size is configurable; default 256 entries"), reaching this header as
// the `Capacity` template argument that generated code instantiates.
// It is a template argument rather than a constructor parameter because
// the ring is a `std::array` member: a runtime size would move it to the
// heap, and the per-sender window is exactly the place where an
// allocation is unwelcome — it is constructed on the receive path the
// first time a sender is seen.
//
// UUID v7's ms-prefix timestamp makes the ring a time-bounded sliding
// filter: at 1 000 events/sec/sender the default 256 entries is a 256 ms
// window — longer than any practical retransmit interval at the
// transport layer. A deployment whose sender rate or retransmit horizon
// differs scales the number accordingly, paying 16 bytes of memory and
// one comparison per entry per inbound envelope (the scan is linear;
// see `observeWithSignal`).
//
// Scope (what this class is NOT):
//   * No sender eviction. The per-source map grows to the cardinality
//     of distinct senders the receiver has ever observed. Current
//     mesh topologies pin sender identity to the machine roster in
//     deploy.yaml (bounded, small), so this is acceptable; game-scale
//     fan-in would need an eviction policy layered on top.
//   * No cross-session replay defence. An envelope that arrives more
//     than `Capacity` events after its first delivery will pass the
//     filter.
//     §mesh-10.5 calls this out — the DedupWindow is a correctness guard
//     against transport-level re-delivery, not against a malicious
//     replay attacker.
//   * No interaction with RPC correlation. The mesh-rpc reply path
//     (`InvokeCorrelation`) independently guards against
//     duplicate replies via its "at-most-once" callback contract.
//   * Per-router scope (not per-transport). The DedupRouter is keyed
//     on `(env.source, env.id)` alone — not on the transport that
//     delivered the envelope. Every inbound path from a transport
//     that declared `supplies_dedup: false` in transport.rs (today:
//     Zenoh, and SOME/IP bindings without `protocol: tcp`) funnels
//     through the same DedupRouter, so even a hypothetical envelope
//     that re-arrives on a different transport with an identical
//     `(source, id)` pair would be caught by the window. In
//     practice this scenario does not occur: the sender stamps
//     `env.id` once per mesh-send-callback invocation and
//     `route_send` picks exactly one transport per target, so a
//     given wire envelope cannot physically originate on two
//     transports. The per-router scope is a natural fit for today's
//     sender semantics — it is neither a blind spot nor load-bearing
//     defence-in-depth.
//
// Thread-safety: transport callback threads (vsomeip application
// threads, zenoh runtime threads, custom_tcp reader threads, local
// cross-router dispatcher) can call `admit()` concurrently. The
// implementation holds one mutex per DedupRouter across the whole
// admit path.
//
// Scaling: per-sender mutex striping (e.g. one mutex per window)
// would shorten the critical section under high-fan-in fan-out.
// Today's deploy.yaml rosters are small (harness fixtures use 1-3
// senders) and no benchmark has been run on a high-rate inbound
// path, so the single mutex is sufficient until measurement shows
// otherwise — revisit if a Zenoh subscriber stream at > 10 kHz
// contends on this lock.

#pragma once

#include "mesh/MeshUuidKey.h"

#include <array>
#include <cstddef>
#include <mutex>
#include <string>
#include <unordered_map>

namespace SCE::Mesh {

/// Outcome of admitting one envelope id into a window.
///
/// Namespace-scope rather than a member of the window, because it does
/// not vary with capacity: a `DedupRouterT<256>` and a
/// `DedupRouterT<512>` on the same device must report the same three
/// outcomes to the same `raiseCommunicationError` call site.
///
/// SCE_MESH.md §mesh-16.7 row 7 — `NovelWithEviction` distinguishes
/// "novel id, fresh slot" from "novel id, evicted an existing entry".
/// The DEDUP_WINDOW_OVERFLOW raise condition the catalog defines maps to
/// `NovelWithEviction`: the runtime has no oracle for "leaked duplicate
/// older than the window", so eviction at full capacity is the closest
/// observable proxy for "sustained rate exceeds window capacity".
enum class DedupResult {
    Duplicate,
    Novel,
    NovelWithEviction,
};

/// Ring-buffer window of recently-observed envelope ids for a single
/// sender. `observe(id)` returns `true` iff the id was novel (not in
/// the window at call time); in that case the id is also inserted,
/// evicting the oldest entry when the ring is full.
///
/// Populated-slot tracking: the default-constructed ring holds all-zero
/// ids, which would otherwise match an incoming all-zero envelope id
/// (legal under the UUID bit pattern; common in test fixtures). The
/// scan uses `wrapped_` + `head_` to walk only the slots that have
/// actually been written, so a fresh window always admits its first id
/// regardless of its value.
template <std::size_t Capacity> class DedupWindowT {
    static_assert(Capacity > 0, "a zero-length dedup window would admit every duplicate; "
                                "deploy.yaml rejects window_size: 0 before codegen reaches here");

public:
    /// The deploy.yaml-declared window size, re-exposed so tests and the
    /// generated §mesh-16.7 row 7 payload can name the capacity without
    /// repeating the literal.
    static constexpr std::size_t kCapacity = Capacity;

    /// The shared 16-byte UUID key (`MeshUuidKey.h`), not a private
    /// redeclaration — the envelope ids this window holds are the same
    /// values `InvokeCorrelation` and `MeshDeadlineScheduler` key on.
    using Id = MeshUuidKey;

    /// Rich result variant of `observe`. Returns NovelWithEviction iff
    /// the ring was already wrapped (i.e. at capacity) AND the new id
    /// is novel — meaning an existing slot was overwritten by this
    /// call. Novel iff the ring still had unused slots before this
    /// insert. Duplicate iff the id matched any populated slot.
    [[nodiscard]] DedupResult observeWithSignal(const Id &id) noexcept {
        const std::size_t filled = wrapped_ ? kCapacity : head_;
        for (std::size_t i = 0; i < filled; ++i) {
            if (recent_ids_[i] == id) {
                return DedupResult::Duplicate;
            }
        }
        const bool eviction = wrapped_;
        recent_ids_[head_] = id;
        head_ = (head_ + 1) % kCapacity;
        if (head_ == 0) {
            wrapped_ = true;
        }
        return eviction ? DedupResult::NovelWithEviction : DedupResult::Novel;
    }

    /// Backward-compatible bool wrapper. Returns true iff `id` was
    /// novel (NovelWithEviction collapses to true — the dedup-filter
    /// contract is unchanged for callers that do not need the row 7
    /// signal).
    ///
    /// Complexity: O(Capacity). At the default 256 the linear scan over
    /// a 4 KiB array beats hash-set overhead, and there is no sorting to
    /// maintain: the ring is append-only with head-rotation. The cost is
    /// linear in the declared size, which is the trade a deployment
    /// accepts when it raises the window — stated in the deploy schema
    /// so the choice is informed rather than discovered.
    [[nodiscard]] bool observe(const Id &id) noexcept {
        return observeWithSignal(id) != DedupResult::Duplicate;
    }

private:
    std::array<Id, kCapacity> recent_ids_{};
    std::size_t head_ = 0;
    bool wrapped_ = false;
};

/// Per-sender DedupWindow registry. One instance lives on a receiving
/// TransportRouter when at least one of its bound transports declares
/// `supplies_dedup: false` (SCE_MESH.md §mesh-10.5; see
/// `sce-build::mesh::transport::TransportDescriptor::supplies_dedup`).
///
/// Generated TransportRouter code calls `admit(env.source, env.id)`
/// on every inbound envelope that arrived via such a transport.
/// Envelopes from transports with inherent dedup (local, shm, SOME/IP
/// over TCP, custom_tcp) MUST NOT traverse this class — the codegen
/// branches at the inbound call site so those paths are zero-cost.
template <std::size_t Capacity> class DedupRouterT {
public:
    using Window = DedupWindowT<Capacity>;
    using Id = typename Window::Id;

    /// Re-exposed from the window so a call site holding only the router
    /// type — which is what generated code names — can report the
    /// capacity in the §mesh-16.7 row 7 payload.
    static constexpr std::size_t kCapacity = Capacity;

    /// Returns true iff the envelope should proceed to the engine.
    /// Returns false iff the (source, id) pair was already observed
    /// within the window and the envelope should be dropped.
    [[nodiscard]] bool admit(const std::string &source, const Id &id) {
        return admitWithSignal(source, id) != DedupResult::Duplicate;
    }

    /// Rich variant — surfaces the §mesh-16.7 row 7 DEDUP_WINDOW_OVERFLOW
    /// signal (NovelWithEviction). Codegen call sites that own a
    /// `raiseCommunicationError` helper switch on this enum to
    /// (a) drop on Duplicate, (b) proceed silently on Novel,
    /// (c) proceed AND raise DEDUP_WINDOW_OVERFLOW on
    ///     NovelWithEviction.
    [[nodiscard]] DedupResult admitWithSignal(const std::string &source, const Id &id) {
        std::lock_guard<std::mutex> lock(mutex_);
        // `operator[]` value-initializes the window on first insert,
        // i.e. an all-zero ring with head_ = 0 — any non-zero UUID is
        // novel on first observation, which is the intended behavior.
        return windows_[source].observeWithSignal(id);
    }

private:
    std::mutex mutex_;
    std::unordered_map<std::string, Window> windows_;
};

}  // namespace SCE::Mesh
