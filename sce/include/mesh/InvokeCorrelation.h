// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh InvokeCorrelation — in-flight `<invoke type="sce:mesh-rpc">`
// registry.
//
// SCE_MESH.md §mesh-9.5: a `sce:mesh-rpc` invoke is a short-lived RPC layered
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
// map of `uuid → {target, responders, deliver}` entries and invoke each deliver
// callback at most once with the right `RpcStatus`. The `target`
// field is the deploy.yaml peer machine name the invoke is bound to
// — carried alongside the callback so the §mesh-10.4.1 row 1704
// shutdown-time §mesh-16.7 row 5 `INVOKE_CHILD_LOST` raise can surface
// it per outstanding entry without the caller having to maintain a
// parallel reverse index.
//
// Thread-safety: one mutex guards the whole map. Reply and deadline
// arrive on transport / scheduler threads; `<cancel>` runs on the
// engine thread. Whichever handler erases the entry first wins; the
// loser looks up a missing key and returns false. Per §mesh-9.5 this is
// the intended degradation — a cancelled invoke never raises
// `done`/`error`, and a late reply after a deadline is silently
// dropped.

#pragma once

#include "mesh/MeshUuidKey.h"
#include "mesh/RpcStatus.h"

#include <algorithm>
#include <array>
#include <cstddef>
#include <cstdint>
#include <functional>
#include <mutex>
#include <string>
#include <unordered_map>
#include <utility>
#include <vector>

namespace SCE::Mesh {

/// SCE_MESH.md §mesh-9.5: the `<invoke type="sce:mesh-rpc">` request/reply
/// correlation table — maps a wire-level invoke id to its pending parent
/// callback and matches replies, deadlines, and cancels back to the
/// originating single-round-trip RPC.
class InvokeCorrelation {
public:
    /// Wire-level invoke id: the 16-byte UUID v7 carried in
    /// `MeshEnvelope.invoke_id` on both the request and its reply.
    /// Equal across the matched request/reply pair; the parent-side
    /// SCXML invoke id string is captured by the `DeliverCallback`
    /// closure (not stored here), keeping this class payload-free.
    /// Aliases the shared SCE::Mesh::MeshUuidKey single-source type
    /// (see `mesh/MeshUuidKey.h`) so InvokeCorrelation, MeshDeadlineScheduler,
    /// and RetryingDispatcher reach the same key concept without
    /// inversion-via-duplication.
    using Key = MeshUuidKey;

    /// Invoked exactly once per successful [`registerInvoke`]:
    ///
    /// * on a matching [`handleReply`] — `status` reflects the
    ///   envelope's `rpc_status` (`Ok` → fire `done.invoke.<id>`,
    ///   anything else → `error.invoke.<id>` with the status as
    ///   part of the §mesh-10.7 structured error), `data` holds the
    ///   reply payload bytes (possibly empty, codec-encoded per
    ///   `MeshEnvelope.datacontenttype`).
    /// * on [`handleDeadline`] — `status = RpcStatus::DeadlineExceeded`,
    ///   `data` is empty.
    ///
    /// **Not** invoked on [`handleCancel`]: W3C SCXML `<cancel>` does
    /// not raise `done`/`error` on the cancelled invoke, so the
    /// correlation table simply erases the entry and drops the
    /// callback without firing it.
    using DeliverCallback = std::function<void(RpcStatus, std::vector<std::uint8_t>)>;

    /// Outcome of [`handleReply`]. Three cases rather than a bool
    /// because the caller must distinguish "not ours" from "not
    /// allowed": SCE_MESH.md §mesh-16.7 raises `error.communication`
    /// with `reason = "RPC_REPLY_FROM_UNDECLARED_PEER"` only for the
    /// latter. A reply for an id this router never issued (or one
    /// already retired by cancel / deadline) is ordinary traffic and
    /// stays silent.
    enum class ReplyOutcome {
        /// The entry matched and its deliver callback has fired.
        Delivered,
        /// No live entry for `uuid` — unknown, or already retired.
        NoSuchInvoke,
        /// The entry is live, but `replier` is not in its responder
        /// set. **The entry is left in place**: the request stays
        /// answerable by a declared responder.
        ReplierNotDeclared,
    };

    /// Register an in-flight invoke. `target` is the deploy.yaml peer
    /// machine name the invoke is bound to — stored alongside the
    /// callback so the §mesh-10.4.1 row 1704 shutdown-time §mesh-16.7 row 5
    /// `INVOKE_CHILD_LOST` raise can surface it without a parallel
    /// reverse index.
    ///
    /// `responders` is the SCE_MESH.md §mesh-14.6 responder set —
    /// the machine names (no leading `#`) whose RpcReply may retire
    /// this entry. It comes from the binding's `reply_from:` and
    /// defaults to the single target the invoke was sent to. An empty
    /// set is a caller contract violation and is refused: a
    /// correlation entry nobody may answer would hang until its
    /// deadline, so codegen emitting an empty set must fail loudly
    /// rather than silently.
    ///
    /// Returns `false` if `uuid` is already registered — that is also
    /// a caller contract violation (an invoke id must be unique per
    /// parent) — or if `responders` is empty. In both cases nothing is
    /// inserted and any first registration is left undisturbed.
    bool registerInvoke(const Key &uuid, std::string target, std::vector<std::string> responders,
                        DeliverCallback deliver) {
        if (responders.empty()) {
            return false;
        }
        std::lock_guard<std::mutex> lock(mutex_);
        auto [it, inserted] =
            pending_.emplace(uuid, Entry{std::move(target), std::move(responders), std::move(deliver)});
        (void)it;
        return inserted;
    }

    /// `RpcReply` envelope arrived from `replier` (the envelope's
    /// `source`, i.e. a machine name with no leading `#`).
    ///
    /// SCE_MESH.md §mesh-14.6: a correlation entry is a one-shot
    /// resource — whoever matches it retires it, and the request can
    /// then never be answered by anyone else. So the responder set is
    /// checked BEFORE the entry is erased. A reply from outside the set
    /// leaves the entry live and returns
    /// [`ReplyOutcome::ReplierNotDeclared`]; without that ordering any
    /// peer that learned an invoke id could retire another peer's
    /// pending request.
    ///
    /// On a match the deliver callback is moved out, the entry erased,
    /// the mutex released, and only then is the callback fired — the
    /// transport thread calling this must not block other correlation
    /// operations while the engine processes the event.
    ReplyOutcome handleReply(const Key &uuid, const std::string &replier, RpcStatus status,
                             std::vector<std::uint8_t> data) {
        DeliverCallback cb;
        {
            std::lock_guard<std::mutex> lock(mutex_);
            auto it = pending_.find(uuid);
            if (it == pending_.end()) {
                return ReplyOutcome::NoSuchInvoke;
            }
            const auto &allowed = it->second.responders;
            if (std::find(allowed.begin(), allowed.end(), replier) == allowed.end()) {
                return ReplyOutcome::ReplierNotDeclared;
            }
            cb = std::move(it->second.deliver);
            pending_.erase(it);
        }
        if (cb) {
            cb(status, std::move(data));
        }
        return ReplyOutcome::Delivered;
    }

    /// Author `<cancel>` hit this invoke. Erases the entry without
    /// firing the deliver callback. Returns `true` if the entry
    /// existed (i.e. there was something to cancel).
    bool handleCancel(const Key &uuid) {
        std::lock_guard<std::mutex> lock(mutex_);
        return pending_.erase(uuid) != 0;
    }

    /// Fail an in-flight invoke from a condition this router observed
    /// itself — a deadline expiring, a Zenoh query terminating without
    /// a reply, a transport dropping. Fires `deliver(status, {})` and
    /// erases. Returns `false` if a concurrent reply or cancel already
    /// erased the entry — a benign race whose loser drops silently.
    ///
    /// Deliberately NOT routed through [`handleReply`]: there is no
    /// replier here, so there is no responder set to check. The
    /// authority is this router itself, which is precisely what the
    /// §mesh-14.6 gate exists to verify for envelopes that came off the
    /// wire. Routing local failures through the gate would either need
    /// a fake replier name or a bypass flag — both of which turn the
    /// gate into something a caller can talk its way past.
    bool failLocally(const Key &uuid, RpcStatus status) {
        DeliverCallback cb;
        {
            std::lock_guard<std::mutex> lock(mutex_);
            auto it = pending_.find(uuid);
            if (it == pending_.end()) {
                return false;
            }
            cb = std::move(it->second.deliver);
            pending_.erase(it);
        }
        if (cb) {
            cb(status, {});
        }
        return true;
    }

    /// Deadline timer fired before a reply arrived. Thin alias for
    /// [`failLocally`] with `RpcStatus::DeadlineExceeded`.
    bool handleDeadline(const Key &uuid) {
        return failLocally(uuid, RpcStatus::DeadlineExceeded);
    }

    std::size_t size() const {
        std::lock_guard<std::mutex> lock(mutex_);
        return pending_.size();
    }

    bool contains(const Key &uuid) const {
        std::lock_guard<std::mutex> lock(mutex_);
        return pending_.find(uuid) != pending_.end();
    }

    /// Cancel every outstanding entry and invoke `on_each` once per
    /// erased entry with the entry's `(uuid, target)`. The deliver
    /// callbacks are NOT fired (same erase-without-delivery semantics
    /// as `handleCancel`) — `on_each` is the caller's parallel
    /// notification hook. The map is empty when this method returns.
    ///
    /// SCE_MESH.md §mesh-10.4.1 row 1704: transport-shutdown failure of
    /// outstanding RPC entries. The caller (TransportRouter::shutdown)
    /// uses `on_each` to raise §mesh-16.7 row 5 `INVOKE_CHILD_LOST` per
    /// outstanding entry, carrying `invoke_id` (uuid) and `target`.
    ///
    /// All entries are moved out of `pending_` under the mutex into
    /// a local snapshot, the mutex is released, and `on_each` is
    /// invoked per snapshot entry. This avoids holding `mutex_` while
    /// the caller-supplied notification runs (which may invoke
    /// SCXML-side raise paths that grab unrelated locks), preserving
    /// §mesh-10.10 lock-discipline.
    void cancelAllPending(const std::function<void(const Key &, const std::string &target)> &on_each) {
        std::vector<std::pair<Key, std::string>> snapshot;
        {
            std::lock_guard<std::mutex> lock(mutex_);
            snapshot.reserve(pending_.size());
            for (auto &[uuid, entry] : pending_) {
                snapshot.emplace_back(uuid, std::move(entry.target));
            }
            pending_.clear();
        }
        if (!on_each) {
            return;
        }
        for (const auto &[uuid, target] : snapshot) {
            on_each(uuid, target);
        }
    }

    /// Cancel every outstanding entry whose `target` equals `peer` and
    /// invoke `on_each` once per erased entry with the entry's `uuid`.
    /// Same erase-without-delivery semantics as `cancelAllPending` —
    /// the deliver callback is NOT fired; `on_each` is the caller's
    /// parallel notification hook. Entries for OTHER targets stay
    /// live. Returns the number of entries erased.
    ///
    /// SCE_MESH.md §mesh-16.7 row 5 post-init peer-drop fast-path: when a
    /// peer transitions Active→Disconnected (Zenoh liveliness DELETE
    /// or SOME/IP availability=false) the outstanding §mesh-9.5 mesh-rpc
    /// invokes targeting that peer cannot complete; failing them
    /// here is strictly sooner than waiting for the full
    /// `TransportRouter::shutdown` Lifecycle:Shutdown sweep.
    ///
    /// Same lock-discipline as `cancelAllPending`: snapshot under the
    /// mutex, release, fire `on_each` per snapshot entry. Two-pass
    /// loop because `std::unordered_map::erase(const Key&)` invalidates
    /// the iterator into a heterogeneous-key bucket; collect keys
    /// first, then erase.
    std::size_t cancelAllPendingForTarget(const std::string &peer, const std::function<void(const Key &)> &on_each) {
        std::vector<Key> snapshot;
        {
            std::lock_guard<std::mutex> lock(mutex_);
            for (const auto &[uuid, entry] : pending_) {
                if (entry.target == peer) {
                    snapshot.push_back(uuid);
                }
            }
            for (const Key &uuid : snapshot) {
                pending_.erase(uuid);
            }
        }
        if (on_each) {
            for (const Key &uuid : snapshot) {
                on_each(uuid);
            }
        }
        return snapshot.size();
    }

private:
    /// Aliases the shared SCE::Mesh::MeshUuidKeyHash so InvokeCorrelation,
    /// MeshDeadlineScheduler, and RetryingDispatcher use the same FNV-1a
    /// hash function for their UUID-v7-keyed maps (see `mesh/MeshUuidKey.h`
    /// for the design rationale + uniformity contract).
    using KeyHash = MeshUuidKeyHash;

    /// Per-entry payload: the peer machine name the invoke is bound
    /// to plus the deliver callback. Target is stored explicitly
    /// (not closed over in the callback) so `cancelAllPending` can
    /// surface it for §mesh-16.7 row 5 emit without the caller maintaining
    /// a parallel reverse index from uuid → target.
    struct Entry {
        std::string target;
        /// SCE_MESH.md §mesh-14.6 responder set: machine names (no
        /// leading `#`) whose RpcReply may retire this entry. Always
        /// non-empty — `registerInvoke` refuses an empty set.
        std::vector<std::string> responders;
        DeliverCallback deliver;
    };

    mutable std::mutex mutex_;
    std::unordered_map<Key, Entry, KeyHash> pending_;
};

}  // namespace SCE::Mesh
