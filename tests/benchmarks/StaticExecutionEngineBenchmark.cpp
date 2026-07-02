// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// StaticExecutionEngine std::function footprint benchmark —
// next_session_mesh_aot_textbook_debt item #3.
//
// Question: the 5 mesh-facing `std::function` members on the engine base
//   (`onMeshSend_`, `onMeshInvoke_`, `onMeshCancel_`,
//    `onParallelRegionLocalComplete_`, `onParallelRegionRemoteSend_`)
// are carried unconditionally. A W3C-only SM with no mesh involvement
// still pays the memory cost. Does that warrant a CRTP/mixin split so
// non-mesh SMs drop the hooks?
//
// Measurement strategy — memory-only:
//   1. `sizeof(StaticExecutionEngine<MinimalPolicy>)` absolute.
//   2. Per-hook sizeof breakdown. SBO residency matters: libstdc++
//      gives `std::function` a 16 B SBO slot, so pointer-sized captures
//      (`[this]`, `[ptr]`) stay inline and introduce no heap traffic
//      per install. A future edit that enlarges a capture past SBO
//      would leak into `engine_without_hooks_estimate_bytes`.
//   3. 10K-session scale (the realistic MMORPG upper bound referenced
//      in `session_tracker_stdfunction_benchmark_done.md`): allocate
//      a pool and report actual bytes vs. arithmetic.
//
// Why no dispatch benchmark: the 5 mesh hooks fire only from the
// parallel-final (§16.5) / mesh-send (§9.5) / mesh-invoke (§9.5)
// codepaths. On the base `raiseExternal → queue → drain` hot path
// they are not consulted — an uninstalled hook is an empty
// `std::function` whose `operator bool()` is never queried in this
// path. "Cost of an unused hook per macrostep" is 0 cycles by
// construction, so per-event timing reduces to the engine's own
// framing overhead (measured by the `benchmark_statemachine` suite).
// Memory (sizeof + cache pressure) is the only axis on which these
// hooks impose cost on non-mesh SMs.
//
// Out of scope:
//   - `completionCallback_` (W3C §6.4) and `onHttpSend_` (W3C C.2) are
//     native engine concerns, not mesh debt.
//   - Dispatch cost of the mesh hooks when populated — covered by
//     `ParallelCompletionTracker` benchmark and the `mesh_*` ctest
//     fixtures.
//
// Capture-shape caveat: this fixture never installs a hook; if a
// consumer installs one with a heap-backed capture, that is a
// per-install cost (new/delete of the control block), not a per-SM
// footprint and not visible here.
//
// Benchmark lands as a regression guard — future edits that grow a
// mesh-facing `std::function` (e.g. richer signature) will surface as
// an increase in `sizeof_engine_bytes` + `mmorpg_scale_10k_mb`.

#include "static/StaticExecutionEngine.h"

#include <benchmark/benchmark.h>

#include <cstddef>
#include <cstdint>
#include <functional>
#include <optional>
#include <string>
#include <vector>

namespace {

// Minimal Policy satisfying `EventNamingPolicy` + the engine's
// member-variable static_asserts. The Policy body contributes a few
// bytes (3 mutable trackers); engine-side `std::function` members
// live on the base and are independent of the Policy's shape.
struct MinimalPolicy {
    enum class State : std::uint8_t { S0, Done };
    enum class Event : std::uint8_t { NONE, Tick };

    static constexpr bool HAS_PARALLEL_STATES = false;
    static constexpr bool NEEDS_SCRIPT_ENGINE = false;

    mutable State lastTransitionSourceState_{};
    mutable bool lastTransitionIsInternal_ = false;
    mutable bool lastTransitionIsTargetless_ = false;

    MinimalPolicy() = default;

    [[nodiscard]] static constexpr State initialState() noexcept {
        return State::S0;
    }

    [[nodiscard]] static constexpr bool isFinalState(State s) noexcept {
        return s == State::Done;
    }

    [[nodiscard]] static constexpr std::optional<State> getParent(State) noexcept {
        return std::nullopt;
    }

    [[nodiscard]] static constexpr bool isCompoundState(State) noexcept {
        return false;
    }

    [[nodiscard]] static constexpr State getInitialChild(State s) noexcept {
        return s;
    }

    [[nodiscard]] std::string getEventName(Event) const {
        return "";
    }

    [[nodiscard]] std::optional<Event> getEventFromName(const std::string &) const {
        return std::nullopt;
    }
};

using Engine = ::SCE::Static::StaticExecutionEngine<MinimalPolicy>;

// Per-hook sizeof totals. The 5 mesh-facing `std::function` types on
// `StaticExecutionEngine.h` L447-464 — kept here as constants so the
// counter output shows the arithmetic independently of platform sizeof.
constexpr std::size_t kMeshSendHookBytes = sizeof(::SCE::Static::MeshSendCallback);
constexpr std::size_t kMeshInvokeHookBytes = sizeof(::SCE::Static::MeshInvokeCallback);
constexpr std::size_t kMeshCancelHookBytes = sizeof(::SCE::Static::MeshCancelCallback);
constexpr std::size_t kParallelLocalHookBytes = sizeof(std::function<void(const std::string &, const std::string &)>);
constexpr std::size_t kParallelRemoteHookBytes =
    sizeof(std::function<void(const std::string &, const std::string &, const std::string &)>);
constexpr std::size_t kMeshHookTotalBytes = kMeshSendHookBytes + kMeshInvokeHookBytes + kMeshCancelHookBytes +
                                            kParallelLocalHookBytes + kParallelRemoteHookBytes;

// Realistic MMORPG upper bound from `session_tracker_stdfunction_benchmark_done.md`.
constexpr std::size_t kMmorpgScaleSessions = 10000;

}  // namespace

// ─── sizeof report (not a timing benchmark) ──────────────────────────
static void BM_SizeofEngine(benchmark::State &state) {
    std::size_t engine_bytes = sizeof(Engine);
    for (auto _ : state) {
        benchmark::DoNotOptimize(engine_bytes);
    }
    state.counters["sizeof_engine_bytes"] = static_cast<double>(sizeof(Engine));
    state.counters["sizeof_policy_bytes"] = static_cast<double>(sizeof(MinimalPolicy));
    state.counters["sizeof_mesh_hooks_total_bytes"] = static_cast<double>(kMeshHookTotalBytes);
    state.counters["engine_without_mesh_hooks_est_bytes"] = static_cast<double>(sizeof(Engine) - kMeshHookTotalBytes);
    state.counters["mmorpg_scale_10k_total_mb"] =
        static_cast<double>(sizeof(Engine) * kMmorpgScaleSessions) / (1024.0 * 1024.0);
    state.counters["mmorpg_scale_10k_mesh_hooks_mb"] =
        static_cast<double>(kMeshHookTotalBytes * kMmorpgScaleSessions) / (1024.0 * 1024.0);
}

BENCHMARK(BM_SizeofEngine);

// ─── per-hook breakdown (SBO residency audit) ────────────────────────
//
// Surfaces each mesh hook's std::function sizeof so a future edit that
// pushes a capture past SBO (introducing per-install heap traffic) is
// visible in the counter delta.
static void BM_HookSizesByField(benchmark::State &state) {
    std::size_t total = kMeshHookTotalBytes;
    for (auto _ : state) {
        benchmark::DoNotOptimize(total);
    }
    state.counters["mesh_send_callback_bytes"] = static_cast<double>(kMeshSendHookBytes);
    state.counters["mesh_invoke_callback_bytes"] = static_cast<double>(kMeshInvokeHookBytes);
    state.counters["mesh_cancel_callback_bytes"] = static_cast<double>(kMeshCancelHookBytes);
    state.counters["parallel_local_complete_bytes"] = static_cast<double>(kParallelLocalHookBytes);
    state.counters["parallel_remote_send_bytes"] = static_cast<double>(kParallelRemoteHookBytes);
}

BENCHMARK(BM_HookSizesByField);

// ─── 10K-scale allocation footprint ──────────────────────────────────
//
// Allocates an engine pool at MMORPG session scale (10K), confirming
// runtime arithmetic matches `sizeof(Engine) * N`. Pool is reserved
// upfront so realloc churn does not contaminate the measurement.
static void BM_MmorpgScaleAllocate(benchmark::State &state) {
    for (auto _ : state) {
        std::vector<Engine> pool;
        pool.reserve(kMmorpgScaleSessions);
        for (std::size_t i = 0; i < kMmorpgScaleSessions; ++i) {
            pool.emplace_back();
        }
        benchmark::DoNotOptimize(pool);
    }
    state.counters["sessions"] = static_cast<double>(kMmorpgScaleSessions);
    state.counters["expected_bytes_total"] = static_cast<double>(sizeof(Engine) * kMmorpgScaleSessions);
}

BENCHMARK(BM_MmorpgScaleAllocate);

BENCHMARK_MAIN();
