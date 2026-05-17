// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh ParallelCompletionTracker — distributed `<parallel>`-final
// barrier tracker (SCE_MESH.md §16.5).
//
// The root partition of a distributed `<parallel>` owns one tracker per
// `<parallel>` element it claims via deploy.yaml
// `partitions.<name>.hosts_parallel_roots:` (§14 rule 12). The tracker
// aggregates:
//   1. Local region completions (regions that live in the root partition's
//      own address space) — reported via `onLocalRegionComplete(region_id)`
//      from the generated SM when the region's `<final>` `<onentry>` runs.
//   2. Remote region completions (regions hosted in sibling partitions) —
//      reported via `onRemoteRegionComplete(region_id)` from the inbound
//      wire-21 `ParallelRegionDone` dispatch path in `MeshDispatch.h`.
//
// When the number of completed regions reaches the `<parallel>`'s total
// region count, the tracker invokes the `onComplete` callback once. The
// generated SM's callback raises `Event::Done_state_<parallel_id>` into
// the machine's own external queue — preserving W3C §3.7 semantics.
//
// Re-entry of the `<parallel>` (via `<history>` or a fresh enter-set
// computation that re-activates the parallel) is handled by calling
// `reset()` at `<parallel>` entry in the generated SM — §16.5 L3498:
// "Re-entry of the parallel via history or new enter-set computation
// resets the tracker and starts a fresh activation."
//
// Barrier-timeout (§16.5 L3500):
//   When the owning partition declares `barrier_timeout_ms:` in
//   deploy.yaml, the generated SM ctor populates [`TimerHooks`] so the
//   tracker can arm a finite timer at the first region completion of
//   each activation. Re-arms on every subsequent completion so that
//   [`TimerHooks::arm`] receives an up-to-date `missing_regions` vector
//   (the wire-21 `_event.data` payload is captured at arm time; stale
//   payload on partial progress is the failure mode this re-arm
//   forestalls). Cancels at threshold so the done.state raise wins the
//   race. Fires `TimerHooks::onTimeout` (installed closure) when the
//   timer elapses before threshold — that closure then raises
//   `error.communication` (reason `PARALLEL_BARRIER_TIMEOUT`, §16.7
//   row 6) via the SM's PullScheduler. Absent `TimerHooks` (W3C
//   normative infinity) the tracker never arms a timer.
//
// Scope (what this class is NOT):
//   * No direct coupling to the SM's scheduler. [`TimerHooks`] carries
//     the arm / cancel / onTimeout callbacks installed by the SM ctor;
//     the tracker stays header-only and non-template even though the
//     scheduler itself is `PullScheduler<Event>` (per-SM Event enum).
//   * No envelope encoding/decoding. The tracker sees only `region_id`
//     strings; envelope construction lives in codegen-emitted send sites
//     and envelope decoding lives in `MeshDispatch::dispatchEnvelope`.
//     `missing_regions` is surfaced as a `std::vector<std::string>` to
//     the arm callback, which formats the `_event.data` JSON via
//     `SCE::Mesh::CommunicationError::toJsonBytes`.
//   * Not thread-safe. Generated SM has a single event loop; all tracker
//     calls run on that thread. Transport inbound callbacks must hop
//     onto the SM thread before invoking `onRemoteRegionComplete`. The
//     `PullScheduler` is likewise pulled from the SM step — timer fire
//     never races threshold cancellation on the same thread.
//   * Single-shot per region activation (§16.5 L3498): a duplicate
//     region id is silently ignored so that at-least-once transport
//     redelivery does not over-count completions.
//
// Header-only primitive. The generated SM instantiates one tracker per
// hosted `<parallel>` as a direct member (value semantics); no heap
// allocation is required.

#pragma once

#include <cstddef>
#include <cstdint>
#include <functional>
#include <optional>
#include <set>
#include <string>
#include <utility>
#include <vector>

namespace SCE::Mesh {

/// Per-`<parallel>` completion tracker owned by the root partition.
class ParallelCompletionTracker {
public:
    using OnCompleteCallback = std::function<void()>;

    /// Invoked on every completion *before threshold* to (re-)arm the
    /// §16.5 barrier timer. Receives the author-declared timeout and a
    /// freshly-computed `missing_regions` vector so the captured
    /// `_event.data.missing_regions` matches the tracker's current
    /// state. Implementation routes to the SM's `PullScheduler`.
    using ArmTimerCallback =
        std::function<void(std::uint64_t timeout_ms,
                           std::vector<std::string> missing_regions)>;

    /// Invoked at threshold and at `reset()` to cancel any pending
    /// §16.5 barrier timer. Safe to call when no timer is armed (the
    /// scheduler's `cancelEvent` returns false for unknown send ids).
    using CancelTimerCallback = std::function<void()>;

    /// Hook bundle for the §16.5 barrier-timeout runtime (L3500).
    /// Default-constructed instance disables the timer entirely — the
    /// tracker behaves exactly like the pre-L3500 threshold-only
    /// primitive. Emitted only when deploy.yaml declares
    /// `barrier_timeout_ms:` on the root-claiming partition; absent
    /// declaration ⇒ W3C normative infinity ⇒ no TimerHooks.
    struct TimerHooks {
        /// Author-declared finite timeout (ms). The arm callback
        /// receives this verbatim.
        std::uint64_t timeout_ms{0};
        /// Author-declared region id roster. `missing_regions` is
        /// computed as `expected_region_ids - completed_` on every
        /// (re-)arm. Sorted on insertion so the arm callback sees a
        /// deterministic order.
        std::set<std::string> expected_region_ids;
        ArmTimerCallback arm;
        CancelTimerCallback cancel;

        // Explicit constructors keep the type default- AND aggregate-
        // initializable under C++17, where the `enabled()` method
        // disqualifies the class from aggregate init (C++20 relaxes
        // this, but SCE's public headers target C++17).
        TimerHooks() = default;
        TimerHooks(std::uint64_t timeout,
                   std::set<std::string> region_ids,
                   ArmTimerCallback arm_fn,
                   CancelTimerCallback cancel_fn)
            : timeout_ms(timeout),
              expected_region_ids(std::move(region_ids)),
              arm(std::move(arm_fn)),
              cancel(std::move(cancel_fn)) {}

        [[nodiscard]] bool enabled() const noexcept {
            return arm != nullptr && cancel != nullptr;
        }
    };

    /// Construct a threshold-only tracker — no barrier-timeout timer
    /// (W3C normative infinity). Matches the pre-§16.5-L3500 primitive
    /// that every existing unit test and every partition without
    /// `barrier_timeout_ms:` relies on.
    ///
    /// `expected_region_count == 0` is degenerate (a `<parallel>` with
    /// no regions is not a valid SCXML construct) — the tracker fires
    /// immediately on the first call to either `onLocalRegionComplete`
    /// or `onRemoteRegionComplete`, which codegen does at `<parallel>`
    /// entry as a correctness guard for the pathological input. Passing
    /// a no-op callback is legal for tests.
    ParallelCompletionTracker(std::size_t expected_region_count,
                              OnCompleteCallback on_complete)
        : expected_count_(expected_region_count),
          on_complete_(std::move(on_complete)),
          timer_hooks_(),
          missing_(),
          fired_(false),
          timer_armed_(false) {}

    /// Construct a tracker with §16.5 L3500 barrier-timeout hooks. The
    /// SM ctor passes a populated [`TimerHooks`] when deploy.yaml
    /// declares `barrier_timeout_ms:` on the root-claiming partition.
    ///
    /// `missing_` is seeded from `timer_hooks_.expected_region_ids` (the
    /// member is initialized after `timer_hooks_` per the declaration
    /// order, so the moved-in roster is observable here). Maintaining
    /// `missing_` incrementally as completions arrive replaces a per-
    /// re-arm O(N) recomputation with O(log N) per completion — see
    /// `computeMissingRegions` for the read path. Empty
    /// `expected_region_ids` (the threshold-only ctor + the disabled
    /// timer case) leaves `missing_` empty, preserving the
    /// pre-incremental behaviour bit-for-bit.
    ParallelCompletionTracker(std::size_t expected_region_count,
                              OnCompleteCallback on_complete,
                              TimerHooks timer_hooks)
        : expected_count_(expected_region_count),
          on_complete_(std::move(on_complete)),
          timer_hooks_(std::move(timer_hooks)),
          missing_(timer_hooks_.expected_region_ids),
          fired_(false),
          timer_armed_(false) {}

    /// Mark a region that lives in the root partition's own address
    /// space as complete. Called from the generated SM's region-final
    /// entry branch.
    void onLocalRegionComplete(const std::string &region_id) {
        onRegionComplete(region_id);
    }

    /// Mark a region hosted in a sibling (non-root) partition as
    /// complete. Called from `MeshDispatch::dispatchEnvelope` on
    /// wire-21 `ParallelRegionDone` envelopes.
    void onRemoteRegionComplete(const std::string &region_id) {
        onRegionComplete(region_id);
    }

    /// Reset for a fresh `<parallel>` activation (§16.5 L3498). Called
    /// from the generated SM's `<parallel>` entry branch before any
    /// region can report completion. Cancels any barrier timer still
    /// armed from a prior activation. Re-seeds `missing_` from the
    /// roster so the incremental invariant
    /// (`missing_ == expected_region_ids - completed_`) is restored
    /// for the fresh activation.
    void reset() {
        cancelBarrierTimerIfArmed();
        completed_.clear();
        missing_ = timer_hooks_.expected_region_ids;
        fired_ = false;
    }

    /// Introspection for tests and diagnostic logging. Not part of the
    /// production dispatch path.
    std::size_t completedCount() const { return completed_.size(); }
    std::size_t expectedCount() const { return expected_count_; }
    bool hasFired() const { return fired_; }
    bool barrierTimerArmed() const { return timer_armed_; }

private:
    /// Single internal path reached from both local and remote
    /// completion notifications. Threshold wins the race: on the step
    /// that reaches `expected_count_`, the barrier timer is cancelled
    /// *before* `on_complete_` fires, so the `done.state.<parallel>`
    /// raise always precedes any timer-driven `error.communication`.
    /// The degenerate `expected_count_ == 0` input is preserved from
    /// the pre-§16.5-L3500 primitive — any completion call immediately
    /// satisfies the threshold and fires `on_complete_`.
    void onRegionComplete(const std::string &region_id) {
        // §16.5 L3498 single-shot: duplicate inserts are a no-op.
        completed_.insert(region_id);
        // Mirror into the incremental missing-set so re-arm is O(log N)
        // rather than O(N) per completion (was O(N²) over a full barrier
        // lifecycle). Erase is a no-op when `region_id` is not in
        // `missing_` (already-completed duplicate, or completion for an
        // id not in the timer roster — both legal under the at-least-once
        // transport contract).
        missing_.erase(region_id);

        if (completed_.size() >= expected_count_) {
            cancelBarrierTimerIfArmed();
            maybeFire();
            return;
        }

        // Not yet threshold. Re-arm the barrier timer with the current
        // `missing_` snapshot so the captured `_event.data.missing_regions`
        // matches the tracker's post-completion state.
        rearmBarrierTimer();
    }

    /// Fire the callback if the completion threshold has been reached.
    /// Single-shot: subsequent calls are no-ops within one activation.
    void maybeFire() {
        if (fired_) return;
        if (completed_.size() < expected_count_) return;
        fired_ = true;
        if (on_complete_) on_complete_();
    }

    /// Snapshot the incrementally-maintained `missing_` set as a sorted
    /// `std::vector`. The invariant
    /// (`missing_ == expected_region_ids - completed_`, refreshed by
    /// `onRegionComplete` and `reset`) replaces the original O(N)
    /// recomputation with a single O(N) copy on the dump side and a
    /// single O(log N) erase on the update side — turning a full
    /// barrier lifecycle from O(N² log N) to O(N log N). Disabled-timer
    /// short-circuit preserves the pre-incremental contract (no payload
    /// to deliver when no arm callback is installed).
    std::vector<std::string> computeMissingRegions() const {
        if (!timer_hooks_.enabled()) return {};
        return std::vector<std::string>(missing_.begin(), missing_.end());
    }

    /// Cancel + re-arm with fresh `missing_regions`. No-op when
    /// `TimerHooks` is disabled (infinity timeout) or when the
    /// degenerate `expected_count_ == 0` `<parallel>` would never
    /// accumulate anything for the timer to guard.
    void rearmBarrierTimer() {
        if (!timer_hooks_.enabled()) return;
        if (expected_count_ == 0) return;
        cancelBarrierTimerIfArmed();
        auto missing = computeMissingRegions();
        timer_hooks_.arm(timer_hooks_.timeout_ms, std::move(missing));
        timer_armed_ = true;
    }

    void cancelBarrierTimerIfArmed() {
        if (!timer_armed_) return;
        if (timer_hooks_.cancel) timer_hooks_.cancel();
        timer_armed_ = false;
    }

    std::size_t expected_count_;
    OnCompleteCallback on_complete_;
    TimerHooks timer_hooks_;
    std::set<std::string> completed_;
    /// Incremental mirror of `expected_region_ids - completed_`. Seeded
    /// from `timer_hooks_.expected_region_ids` at construction and on
    /// `reset()`; shrunk by one element per `onRegionComplete` call.
    /// Empty whenever `timer_hooks_` carries no roster (threshold-only
    /// tracker or default-constructed `TimerHooks`), in which case
    /// `computeMissingRegions` short-circuits before reading this set.
    std::set<std::string> missing_;
    bool fired_;
    bool timer_armed_;
};

}  // namespace SCE::Mesh
