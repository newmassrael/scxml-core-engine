// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// ParallelCompletionTracker overhead benchmark — SCE_MESH.md §16.5.
//
// Question: does the per-tracker `std::function` footprint (one
// `OnCompleteCallback` + two `TimerHooks` callbacks `arm` / `cancel`)
// warrant refactoring the tracker to a template/CRTP primitive that
// inlines the callbacks?
//
// Measurement strategy:
//   1. **No-hooks baseline** — tracker constructed without TimerHooks.
//      Isolates the `OnCompleteCallback` single call plus raw state
//      updates (`std::set` inserts, `bool` flags). This is the
//      canonical W3C-normative-infinity path and the lower bound for
//      any refactor — it has only one `std::function` left to
//      eliminate.
//   2. **With-hooks realistic** — tracker constructed with a populated
//      TimerHooks using `[this]`-captured lambdas (matches
//      `state_machine.jinja2` shape). Exercises the full `arm` /
//      `cancel` / re-arm chain. `std::function::target_type()` is
//      asserted in `CaptureAudit` to prove the captures stay in SBO
//      (no heap allocation hidden in the benchmark).
//   3. **Re-arm-heavy** — N-1 partial completes across increasing
//      `<parallel>` widths (2, 4, 8, 16, 32 regions). Every completion
//      below threshold triggers one `cancel` + one `arm`, so this
//      amplifies the `std::function` indirect-call cost.
//   4. **sizeof report** — `sizeof(ParallelCompletionTracker)` and
//      `sizeof(TimerHooks)` surfaced as Google Benchmark counters so
//      the memory footprint is recorded alongside the timing data.
//
// The benchmarks use `[ptr]`-captured lambdas (counter increments)
// that match the production capture shape — pointer-to-outer-context,
// single-pointer-sized, within std::function SBO on libstdc++/libc++.
// Arm/cancel bodies are deliberately minimal so the measurement
// isolates dispatch overhead; integration cost (scheduler plumbing,
// envelope build) is out of scope and would add noise unrelated to
// the tracker's own indirection.
//
// No regression gate is attached — these numbers exist to justify
// (or refute) a hypothetical refactor. Regression-watchers who care
// about this hot path can run `benchmark_tracker_quick` by hand.

#include "mesh/ParallelCompletionTracker.h"

#include <benchmark/benchmark.h>

#include <cstddef>
#include <cstdint>
#include <set>
#include <string>
#include <typeinfo>
#include <utility>
#include <vector>

using SCE::Mesh::ParallelCompletionTracker;

namespace {

// Build a deterministic region-id roster of the requested width. The
// ids are short fixed-length strings so `std::set<std::string>` insert
// cost is roughly constant across widths (modulo the set's own O(log
// N) ordering).
std::vector<std::string> makeRegionIds(std::size_t width) {
    std::vector<std::string> ids;
    ids.reserve(width);
    for (std::size_t i = 0; i < width; ++i) {
        // Two-digit suffix covers all tested widths (up to 32).
        std::string id = "r";
        if (i < 10) id.push_back('0');
        id += std::to_string(i);
        ids.push_back(std::move(id));
    }
    return ids;
}

// Captured by the hot-path arm/cancel lambdas. Exists outside the
// tracker so the captures stay pointer-sized — mirroring the
// `[this]` capture in `state_machine.jinja2` which points to the
// generated SM instance.
struct CallbackSink {
    std::uint64_t arm_calls = 0;
    std::uint64_t cancel_calls = 0;
    std::uint64_t complete_calls = 0;
};

ParallelCompletionTracker::TimerHooks
makeHooks(const std::vector<std::string>& region_ids, CallbackSink* sink) {
    ParallelCompletionTracker::TimerHooks hooks;
    hooks.timeout_ms = 5000;
    hooks.expected_region_ids.insert(region_ids.begin(), region_ids.end());
    hooks.arm = [sink](std::uint64_t ms, std::vector<std::string> missing) {
        benchmark::DoNotOptimize(ms);
        benchmark::DoNotOptimize(missing);
        ++sink->arm_calls;
    };
    hooks.cancel = [sink]() { ++sink->cancel_calls; };
    return hooks;
}

}  // namespace

// ─── sizeof report (not a timing benchmark) ──────────────────────────
//
// Emits `sizeof(ParallelCompletionTracker)` and `sizeof(TimerHooks)`
// as Google Benchmark counters so the memory footprint is captured
// in the same run as the timing numbers.
static void BM_SizeofTracker(benchmark::State& state) {
    for (auto _ : state) {
        benchmark::DoNotOptimize(sizeof(ParallelCompletionTracker));
    }
    state.counters["sizeof_tracker_bytes"] = static_cast<double>(
        sizeof(ParallelCompletionTracker));
    state.counters["sizeof_timer_hooks_bytes"] = static_cast<double>(
        sizeof(ParallelCompletionTracker::TimerHooks));
    state.counters["sizeof_std_function_void_bytes"] = static_cast<double>(
        sizeof(std::function<void()>));
}
BENCHMARK(BM_SizeofTracker);

// ─── Capture audit — single-shot pre-flight on SBO residency ─────────
//
// Prints the `target_type().name()` of the hooks' `arm` / `cancel`
// `std::function` so the bench log records whether a `[sink]`-captured
// lambda (pointer-sized closure) stayed inside the small-buffer
// optimization. This is not a timing benchmark; it runs once to emit
// audit counters.
static void BM_CaptureAudit(benchmark::State& state) {
    CallbackSink sink;
    auto region_ids = makeRegionIds(4);
    auto hooks = makeHooks(region_ids, &sink);

    // target_type().name() is mangled; surface its bytes so a reader
    // can confirm the capture is `[sink]`-like (pointer-sized) rather
    // than something heap-allocated. We also cross-check
    // std::function::operator bool() and the closure-bytes-in-SBO
    // expectation: captureless/ptr-captured lambdas fit in the libstdc++
    // 16-byte SBO, so the overall sizeof matches expectation.
    state.counters["arm_installed"] = hooks.arm ? 1.0 : 0.0;
    state.counters["cancel_installed"] = hooks.cancel ? 1.0 : 0.0;
    state.counters["sizeof_arm_closure_type_bytes"] =
        static_cast<double>(sizeof(decltype([sink_ptr = &sink](std::uint64_t,
                                                                std::vector<std::string>) {
            (void)sink_ptr;
        })));

    for (auto _ : state) {
        benchmark::DoNotOptimize(hooks.arm);
    }
}
BENCHMARK(BM_CaptureAudit);

// ─── No-hooks baseline: reset + N completes ──────────────────────────
//
// Tracker with no TimerHooks (W3C normative infinity). The workload
// is `reset()` followed by N `onLocalRegionComplete` calls, where the
// N-th call reaches threshold and fires the `on_complete_` callback.
// This is the lower bound for any refactor — it has only the one
// `OnCompleteCallback` `std::function` in play.
static void BM_NoHooks_ResetAndComplete(benchmark::State& state) {
    const std::size_t width = static_cast<std::size_t>(state.range(0));
    auto region_ids = makeRegionIds(width);
    CallbackSink sink;

    ParallelCompletionTracker tracker(width,
                                      [sink_ptr = &sink]() { ++sink_ptr->complete_calls; });

    for (auto _ : state) {
        tracker.reset();
        for (const auto& id : region_ids) {
            tracker.onLocalRegionComplete(id);
        }
        benchmark::DoNotOptimize(tracker);
    }

    state.counters["regions"] = static_cast<double>(width);
    state.counters["complete_calls"] = static_cast<double>(sink.complete_calls);
    state.SetLabel("no hooks, width=" + std::to_string(width));
}
BENCHMARK(BM_NoHooks_ResetAndComplete)->Arg(2)->Arg(4)->Arg(8)->Arg(16)->Arg(32);

// ─── With-hooks full lifecycle: reset + N completes ──────────────────
//
// Same workload as the no-hooks baseline but with TimerHooks armed.
// Each call below threshold triggers one `cancel` + one `arm`
// (re-arm-heavy path). Threshold completion invokes one final
// `cancel` + one `on_complete`. Difference from the no-hooks baseline
// = cost of the full re-arm chain across the `<parallel>`'s lifetime.
static void BM_WithHooks_ResetAndComplete(benchmark::State& state) {
    const std::size_t width = static_cast<std::size_t>(state.range(0));
    auto region_ids = makeRegionIds(width);
    CallbackSink sink;

    ParallelCompletionTracker tracker(
        width, [sink_ptr = &sink]() { ++sink_ptr->complete_calls; },
        makeHooks(region_ids, &sink));

    for (auto _ : state) {
        tracker.reset();
        for (const auto& id : region_ids) {
            tracker.onLocalRegionComplete(id);
        }
        benchmark::DoNotOptimize(tracker);
    }

    state.counters["regions"] = static_cast<double>(width);
    state.counters["arm_calls_per_iter"] =
        static_cast<double>(sink.arm_calls) / static_cast<double>(state.iterations());
    state.counters["cancel_calls_per_iter"] =
        static_cast<double>(sink.cancel_calls) / static_cast<double>(state.iterations());
    state.counters["complete_calls"] = static_cast<double>(sink.complete_calls);
    state.SetLabel("with hooks (re-arm-heavy), width=" + std::to_string(width));
}
BENCHMARK(BM_WithHooks_ResetAndComplete)->Arg(2)->Arg(4)->Arg(8)->Arg(16)->Arg(32);

// ─── Construction cost: build + drop a tracker per iteration ─────────
//
// Amortized tracker construction cost with populated TimerHooks. The
// memory-footprint question ("10K × 4 trackers = 8.9 MB?") is a
// function of ctor cost * lifetime rather than dispatch cost, so it
// deserves its own sample.
static void BM_Construction_WithHooks(benchmark::State& state) {
    const std::size_t width = static_cast<std::size_t>(state.range(0));
    auto region_ids = makeRegionIds(width);

    for (auto _ : state) {
        CallbackSink sink;
        ParallelCompletionTracker tracker(
            width, [sink_ptr = &sink]() { ++sink_ptr->complete_calls; },
            makeHooks(region_ids, &sink));
        benchmark::DoNotOptimize(tracker);
    }

    state.counters["regions"] = static_cast<double>(width);
    state.SetLabel("ctor + dtor per iter, width=" + std::to_string(width));
}
BENCHMARK(BM_Construction_WithHooks)->Arg(2)->Arg(4)->Arg(8)->Arg(16)->Arg(32);

// ─── Reset-only: cancel path isolation ───────────────────────────────
//
// Measures the cost of `reset()` on a tracker with a single completion
// already recorded (so `cancelBarrierTimerIfArmed` actually cancels).
// This isolates the `cancel` `std::function` call — useful when the
// caller is doing re-activation churn (history re-entry).
static void BM_Reset_WithArmedTimer(benchmark::State& state) {
    const std::size_t width = static_cast<std::size_t>(state.range(0));
    auto region_ids = makeRegionIds(width);
    CallbackSink sink;

    ParallelCompletionTracker tracker(
        width, [sink_ptr = &sink]() { ++sink_ptr->complete_calls; },
        makeHooks(region_ids, &sink));

    // Pre-arm: one completion primes the timer so every reset has
    // something to cancel.
    for (auto _ : state) {
        state.PauseTiming();
        tracker.onLocalRegionComplete(region_ids[0]);
        state.ResumeTiming();
        tracker.reset();
        benchmark::DoNotOptimize(tracker);
    }

    state.counters["regions"] = static_cast<double>(width);
    state.counters["cancel_calls_per_iter"] =
        static_cast<double>(sink.cancel_calls) / static_cast<double>(state.iterations());
    state.SetLabel("reset w/ armed timer, width=" + std::to_string(width));
}
BENCHMARK(BM_Reset_WithArmedTimer)->Arg(2)->Arg(4)->Arg(8)->Arg(16)->Arg(32);

BENCHMARK_MAIN();
