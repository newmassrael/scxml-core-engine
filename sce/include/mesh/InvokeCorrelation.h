// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh InvokeCorrelation — in-flight `<invoke type="sce:mesh-rpc">`
// registry.
//
// SCE_MESH.md §9.5: a `sce:mesh-rpc` invoke is a short-lived RPC layered
// on the W3C SCXML invoke lifecycle (`done.invoke.<id>` /
// `error.invoke.<id>` / `<cancel>`). Request and reply envelopes carry
// the same `invoke_id` (UUID v7 bytes, `MeshEnvelope.invoke_id`) so the
// parent engine can match a reply back to its originating invoke.
//
// This class is that registry. One instance lives per TransportRouter
// (i.e. per sender engine with mesh-rpc invokes). The generated
// codegen (F3) populates it on invoke entry and consults it on reply
// / cancel / deadline; the class itself knows nothing about codegen,
// envelopes, or schedulers — its only job is to keep a thread-safe
// map of `uuid → deliver` callbacks and invoke each callback at most
// once with the right `RpcStatus`.
//
// Thread-safety: one mutex guards the whole map. Reply and deadline
// arrive on transport / scheduler threads; `<cancel>` runs on the
// engine thread. Whichever handler erases the entry first wins; the
// loser looks up a missing key and returns false. Per §9.5 this is
// the intended degradation — a cancelled invoke never raises
// `done`/`error`, and a late reply after a deadline is silently
// dropped.

#pragma once

#include "mesh/RpcStatus.h"

#include <array>
#include <cstddef>
#include <cstdint>
#include <functional>
#include <mutex>
#include <unordered_map>
#include <utility>
#include <vector>

namespace SCE::Mesh {

class InvokeCorrelation {
public:
    /// Wire-level invoke id: the 16-byte UUID v7 carried in
    /// `MeshEnvelope.invoke_id` on both the request and its reply.
    /// Equal across the matched request/reply pair; the parent-side
    /// SCXML invoke id string is captured by the `DeliverCallback`
    /// closure (not stored here), keeping this class payload-free.
    using Key = std::array<std::uint8_t, 16>;

    /// Invoked exactly once per successful [`registerInvoke`]:
    ///
    /// * on a matching [`handleReply`] — `status` reflects the
    ///   envelope's `rpc_status` (`Ok` → fire `done.invoke.<id>`,
    ///   anything else → `error.invoke.<id>` with the status as
    ///   part of the §10.6 structured error), `data` holds the
    ///   reply payload bytes (possibly empty, codec-encoded per
    ///   `MeshEnvelope.datacontenttype`).
    /// * on [`handleDeadline`] — `status = RpcStatus::DeadlineExceeded`,
    ///   `data` is empty.
    ///
    /// **Not** invoked on [`handleCancel`]: W3C SCXML `<cancel>` does
    /// not raise `done`/`error` on the cancelled invoke, so the
    /// correlation table simply erases the entry and drops the
    /// callback without firing it.
    using DeliverCallback =
        std::function<void(RpcStatus, std::vector<std::uint8_t>)>;

    /// Register an in-flight invoke. Returns `false` if `uuid` is
    /// already registered — that is a caller contract violation (an
    /// invoke id must be unique per parent), and the duplicate is
    /// dropped without disturbing the first registration.
    bool registerInvoke(const Key& uuid, DeliverCallback deliver) {
        std::lock_guard<std::mutex> lock(mutex_);
        auto [it, inserted] = pending_.emplace(uuid, std::move(deliver));
        (void)it;
        return inserted;
    }

    /// `RpcReply` envelope arrived. If `uuid` is live, moves the
    /// deliver callback out of the map, erases the entry, releases
    /// the mutex, and then fires the callback with `status` and
    /// `data`. Returns `true` on hit, `false` if the entry was
    /// already erased by cancel, deadline, or unknown id.
    ///
    /// The callback runs *outside* the mutex: the transport thread
    /// that calls this should not block other correlation-table
    /// operations while the engine processes the event.
    bool handleReply(const Key& uuid, RpcStatus status,
                     std::vector<std::uint8_t> data) {
        DeliverCallback cb;
        {
            std::lock_guard<std::mutex> lock(mutex_);
            auto it = pending_.find(uuid);
            if (it == pending_.end()) {
                return false;
            }
            cb = std::move(it->second);
            pending_.erase(it);
        }
        if (cb) {
            cb(status, std::move(data));
        }
        return true;
    }

    /// Author `<cancel>` hit this invoke. Erases the entry without
    /// firing the deliver callback. Returns `true` if the entry
    /// existed (i.e. there was something to cancel).
    bool handleCancel(const Key& uuid) {
        std::lock_guard<std::mutex> lock(mutex_);
        return pending_.erase(uuid) != 0;
    }

    /// Deadline timer fired before a reply arrived. Fires
    /// `deliver(RpcStatus::DeadlineExceeded, {})` and erases.
    /// Returns `false` if a concurrent reply or cancel already
    /// erased the entry — a benign race whose loser drops silently.
    bool handleDeadline(const Key& uuid) {
        return handleReply(uuid, RpcStatus::DeadlineExceeded, {});
    }

    std::size_t size() const {
        std::lock_guard<std::mutex> lock(mutex_);
        return pending_.size();
    }

    bool contains(const Key& uuid) const {
        std::lock_guard<std::mutex> lock(mutex_);
        return pending_.find(uuid) != pending_.end();
    }

private:
    /// FNV-1a over 16 bytes. Cheaper than SipHash and adequate here:
    /// the map only holds actively in-flight invokes, bounded by how
    /// many mesh-rpc invokes a single parent has outstanding
    /// simultaneously — typically single digits, never adversarially
    /// large. The key is a random UUID v7 so distribution quality of
    /// FNV-1a is not load-bearing against a hostile workload.
    struct KeyHash {
        std::size_t operator()(const Key& k) const noexcept {
            std::size_t h = 14695981039346656037ULL;
            for (std::uint8_t b : k) {
                h ^= b;
                h *= 1099511628211ULL;
            }
            return h;
        }
    };

    mutable std::mutex mutex_;
    std::unordered_map<Key, DeliverCallback, KeyHash> pending_;
};

}  // namespace SCE::Mesh
