// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh MeshDeadlineScheduler — callback-based deadline timer for
// `<invoke type="sce:mesh-rpc">` lifecycle bookkeeping.
//
// SCE_MESH.md §9.5 requires each mesh-rpc invoke to raise
// `error.invoke.<id>` with `rpc_status=DeadlineExceeded` when no reply
// arrives before `_mesh_deadline_ms`. The AOT StaticExecutionEngine
// scheduler accepts only typed `Event` enum values, not arbitrary
// callbacks — runtime-generated `done.invoke.<uuid>` / `error.invoke.<uuid>`
// event names are not in that enum. This class fills the gap: register
// a callback + deadline; the callback fires on its own thread when the
// deadline elapses, or not at all if `cancelDeadline` wins the race.
//
// Design choices:
//   * One std::thread per scheduler (amortised across all in-flight
//     mesh-rpc invokes on this TransportRouter). Timer per invoke is
//     rejected — millisecond-granularity polling on a single thread
//     plus a bounded min-heap is cheaper than O(N) thread creation.
//   * Priority queue keyed on `(deadline, sequence)`: earliest deadline
//     first; sequence number breaks ties deterministically.
//   * Cancellation is O(1) via the authoritative `active_` map — each
//     heap entry carries its own monotonic seq, and the dispatcher
//     compares against `active_[uuid]` to identify stale entries. This
//     matches SchedulerQueueCore's lazy-eviction pattern in the core
//     engine and avoids the O(log n) find-and-remove penalty at
//     cancel time (which is on the engine thread hot path).
//   * Callbacks run outside the mutex. The dispatch thread takes the
//     callback by move, unlocks, and fires — so a slow callback does
//     not block `registerDeadline` / `cancelDeadline` from other
//     threads. Same pattern as InvokeCorrelation.
//
// Thread-safety:
//   * `registerDeadline` / `cancelDeadline` are safe from any thread.
//   * The owning TransportRouter MUST call `shutdown()` (or destroy
//     the scheduler) before tearing down any state that callbacks
//     capture. The destructor joins the timer thread so callbacks
//     cannot still be executing when it returns.

#pragma once

#include <array>
#include <chrono>
#include <condition_variable>
#include <cstddef>
#include <cstdint>
#include <functional>
#include <mutex>
#include <queue>
#include <thread>
#include <unordered_map>
#include <utility>
#include <vector>

namespace SCE::Mesh {

class MeshDeadlineScheduler {
public:
    /// Wire-level invoke id: 16-byte UUID v7 from `MeshEnvelope.invoke_id`.
    /// Mirrors `InvokeCorrelation::Key` so callers can share UUIDs without
    /// conversion.
    using Key = std::array<std::uint8_t, 16>;

    /// Fired on the scheduler thread exactly once per `registerDeadline`
    /// call IFF the deadline elapses before `cancelDeadline` erases the
    /// entry. The callback captures whatever state it needs (e.g. a
    /// pointer to the owning InvokeCorrelation table) — this class is
    /// payload-free.
    using Callback = std::function<void()>;

    MeshDeadlineScheduler()
        : worker_([this]{ run(); }) {}

    ~MeshDeadlineScheduler() { shutdown(); }

    MeshDeadlineScheduler(const MeshDeadlineScheduler&) = delete;
    MeshDeadlineScheduler& operator=(const MeshDeadlineScheduler&) = delete;
    MeshDeadlineScheduler(MeshDeadlineScheduler&&) = delete;
    MeshDeadlineScheduler& operator=(MeshDeadlineScheduler&&) = delete;

    /// Register a deadline for `uuid`. When `delay` elapses, `callback`
    /// is invoked on the scheduler thread with no arguments. A duplicate
    /// registration for the same `uuid` returns `false` and leaves the
    /// earlier entry untouched — callers must `cancelDeadline` first to
    /// replace a pending deadline.
    ///
    /// Returns `false` if the scheduler has already been shut down.
    bool registerDeadline(const Key& uuid,
                          std::chrono::milliseconds delay,
                          Callback callback) {
        const auto deadline = std::chrono::steady_clock::now() + delay;
        {
            std::lock_guard<std::mutex> lock(mutex_);
            if (shutting_down_) return false;
            if (active_.count(uuid) != 0) return false;
            const auto seq = next_seq_++;
            active_.emplace(uuid, seq);
            heap_.push(Entry{deadline, seq, uuid, std::move(callback)});
        }
        cv_.notify_all();
        return true;
    }

    /// Cancel a pending deadline. Returns `true` if an entry was erased,
    /// `false` if nothing was registered for `uuid` (already fired or
    /// never registered). Safe to call from any thread.
    ///
    /// The heap entry is not physically removed — it is identified as
    /// stale the next time it reaches the front, by comparing its seq
    /// to the current `active_[uuid]` (missing after cancel).
    bool cancelDeadline(const Key& uuid) {
        std::lock_guard<std::mutex> lock(mutex_);
        const auto erased = active_.erase(uuid);
        if (erased != 0) cv_.notify_all();
        return erased != 0;
    }

    /// Number of registered deadlines not yet fired or cancelled. Useful
    /// for test assertions and structural probes; not intended for hot
    /// path use. Counts authoritative `active_` entries, not the heap
    /// (which may contain stale entries awaiting lazy eviction).
    std::size_t size() const {
        std::lock_guard<std::mutex> lock(mutex_);
        return active_.size();
    }

    /// Stop the scheduler thread. Pending deadlines are dropped without
    /// firing their callbacks — §9.5 "benign drop" semantics: a router
    /// destroyed mid-flight cannot deliver any further events, so the
    /// observable effect matches cancel.
    void shutdown() {
        {
            std::lock_guard<std::mutex> lock(mutex_);
            if (shutting_down_) return;
            shutting_down_ = true;
        }
        cv_.notify_all();
        if (worker_.joinable()) worker_.join();
    }

private:
    struct Entry {
        std::chrono::steady_clock::time_point deadline;
        std::uint64_t seq;
        Key uuid;
        Callback callback;
    };

    /// Min-heap: earliest deadline first. `std::priority_queue` is a
    /// max-heap by default, so the comparator returns `true` when `a`
    /// should come AFTER `b` — i.e. when `a.deadline > b.deadline`.
    struct Compare {
        bool operator()(const Entry& a, const Entry& b) const noexcept {
            if (a.deadline != b.deadline) return a.deadline > b.deadline;
            return a.seq > b.seq;
        }
    };

    /// FNV-1a over 16 bytes. Same hash family as InvokeCorrelation's
    /// KeyHash — adequate for the small set of in-flight deadlines
    /// typically outstanding at once.
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

    /// True when the front-of-heap entry is still the live registration
    /// for its uuid (i.e. not cancelled, not superseded by a later
    /// register-after-cancel with the same uuid). Must be called with
    /// `mutex_` held.
    bool topIsLive() const {
        const auto& top = heap_.top();
        const auto it = active_.find(top.uuid);
        return it != active_.end() && it->second == top.seq;
    }

    void run() {
        std::unique_lock<std::mutex> lock(mutex_);
        while (!shutting_down_) {
            if (heap_.empty()) {
                cv_.wait(lock, [this]{ return shutting_down_ || !heap_.empty(); });
                continue;
            }
            if (!topIsLive()) {
                heap_.pop();
                continue;
            }
            const auto deadline = heap_.top().deadline;
            if (deadline > std::chrono::steady_clock::now()) {
                cv_.wait_until(lock, deadline);
                continue;
            }
            // Deadline reached. Re-check liveness: cancellation can
            // race with deadline expiry; whichever wins, only one
            // outcome is observable to the caller.
            if (!topIsLive()) {
                heap_.pop();
                continue;
            }
            active_.erase(heap_.top().uuid);
            Callback cb = std::move(const_cast<Entry&>(heap_.top()).callback);
            heap_.pop();
            lock.unlock();
            if (cb) cb();
            lock.lock();
        }
    }

    mutable std::mutex mutex_;
    std::condition_variable cv_;
    std::priority_queue<Entry, std::vector<Entry>, Compare> heap_;
    std::unordered_map<Key, std::uint64_t, KeyHash> active_;
    std::uint64_t next_seq_ = 0;
    bool shutting_down_ = false;
    std::thread worker_;  // declared last so the thread sees all members initialised.
};

}  // namespace SCE::Mesh
