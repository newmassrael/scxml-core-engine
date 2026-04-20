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
// Scope (what this class is NOT):
//   * No barrier timeout firing. §16.5 L3500 specifies that an
//     author-configured finite timeout raises `error.communication`
//     with `reason=PARALLEL_BARRIER_TIMEOUT` when regions remain silent.
//     That is a scheduler-driven concern (timer fire → engine.raise) and
//     the atomic rule-12 bundle covers only the configuration-level
//     validator gate (`mesh/partition-barrier-timeout-without-root`).
//     Runtime timer integration follows independently and is tracked in
//     the §16.5 spec without pinning a timeline here.
//   * No envelope encoding/decoding. The tracker sees only `region_id`
//     strings; envelope construction lives in codegen-emitted send sites
//     and envelope decoding lives in `MeshDispatch::dispatchEnvelope`.
//   * Not thread-safe. Generated SM has a single event loop; all tracker
//     calls run on that thread. Transport inbound callbacks must hop
//     onto the SM thread before invoking `onRemoteRegionComplete`.
//   * Single-shot per region activation (§16.5 L3498): a duplicate
//     region id is silently ignored so that at-least-once transport
//     redelivery does not over-count completions.
//
// Header-only primitive. The generated SM instantiates one tracker per
// hosted `<parallel>` as a direct member (value semantics); no heap
// allocation is required.

#pragma once

#include <cstddef>
#include <functional>
#include <set>
#include <string>
#include <utility>

namespace SCE::Mesh {

/// Per-`<parallel>` completion tracker owned by the root partition.
class ParallelCompletionTracker {
public:
    using OnCompleteCallback = std::function<void()>;

    /// Construct a tracker for a `<parallel>` with `expected_region_count`
    /// regions total. `on_complete` fires exactly once per activation
    /// when every region has reported.
    ///
    /// `expected_region_count == 0` is degenerate (a `<parallel>` with
    /// no regions is not a valid SCXML construct) — the tracker fires
    /// immediately on first call to `maybeFire()`, which codegen does
    /// at `<parallel>` entry as a correctness guard for the pathological
    /// input. Passing a no-op callback is legal for tests.
    ParallelCompletionTracker(std::size_t expected_region_count,
                              OnCompleteCallback on_complete)
        : expected_count_(expected_region_count),
          on_complete_(std::move(on_complete)),
          fired_(false) {}

    /// Mark a region that lives in the root partition's own address
    /// space as complete. Called from the generated SM's region-final
    /// entry branch.
    void onLocalRegionComplete(const std::string &region_id) {
        completed_.insert(region_id);
        maybeFire();
    }

    /// Mark a region hosted in a sibling (non-root) partition as
    /// complete. Called from `MeshDispatch::dispatchEnvelope` on
    /// wire-21 `ParallelRegionDone` envelopes.
    void onRemoteRegionComplete(const std::string &region_id) {
        completed_.insert(region_id);
        maybeFire();
    }

    /// Reset for a fresh `<parallel>` activation (§16.5 L3498). Called
    /// from the generated SM's `<parallel>` entry branch before any
    /// region can report completion.
    void reset() {
        completed_.clear();
        fired_ = false;
    }

    /// Introspection for tests and diagnostic logging. Not part of the
    /// production dispatch path.
    std::size_t completedCount() const { return completed_.size(); }
    std::size_t expectedCount() const { return expected_count_; }
    bool hasFired() const { return fired_; }

private:
    /// Fire the callback if the completion threshold has been reached.
    /// Single-shot: subsequent calls are no-ops within one activation.
    void maybeFire() {
        if (fired_) return;
        if (completed_.size() < expected_count_) return;
        fired_ = true;
        if (on_complete_) on_complete_();
    }

    std::size_t expected_count_;
    OnCompleteCallback on_complete_;
    std::set<std::string> completed_;
    bool fired_;
};

}  // namespace SCE::Mesh
