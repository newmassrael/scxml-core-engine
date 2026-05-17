// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// NonRoot wire-21 (`ParallelRegionDone`) sender-side dispatch benchmark —
// next_session_mesh_aot_textbook_debt item #5 (regression guard half).
//
// Item #5 calls out the 5-layer dispatch chain from a NonRoot partition's
// region-final entry to the wire-21 shm send: `policy::executeEntryActions
// → engine.triggerParallelRegionRemoteSend → SM ctor closure →
// sendParallelRegionDone method → parallel_region_done_callback_ → router
// lambda → shm.send`. The collapse is gated on "profiler flags this site
// OR wire-21 latency budget regresses". Until this benchmark existed,
// neither half of the gate had a measurement source — the memo's gate (b)
// was a dead trigger. This file installs the measurement so a future
// template edit that grows the chain (e.g. adds a 4th `std::function`
// hop) surfaces as a ns/op regression.
//
// MIRRORS `tools/codegen/templates/state_machine.jinja2:778-836` —
// the `_role_stats.has_nonroot` NonRoot block (`sendParallelRegionDone`
// method + `parallel_region_done_callback_` member +
// `setParallelRegionDoneCallback` installer). Sync this file when that
// template's NonRoot block changes shape. A signature drift trips at
// compile time; a body drift (e.g. an extra hop inserted before the
// final callback) drifts the perf baseline but does not break the
// build — the bench is the only signal for body drift.
//
// Why hand-rolled and not generated-test-driven:
//   1. Stable perf baseline across W3C test additions/changes (a real
//      generated SM such as test417_worker would silently shift the
//      baseline whenever its SCXML changed).
//   2. Isolation — no fixture build dependency, runs in any mesh-enabled
//      build.
//   3. Mirrors `StaticExecutionEngineBenchmark.cpp` (#3) pattern,
//      which uses an identical MinimalPolicy approach for the same
//      family of `std::function` hooks.
//
// Measurement strategy:
//   A. `BM_SendParallelRegionDoneDirect` — `sm.sendParallelRegionDone(...)`
//      direct call. Isolates L4→L5: `MeshEnvelope` construction +
//      member assignments + the single `parallel_region_done_callback_`
//      std::function dispatch + the router-side lambda body.
//   B. `BM_TriggerParallelRegionRemoteSend` — `sm.triggerParallelRegionRemoteSend(...)`
//      via the base engine entry point. Captures L2→L5: the engine
//      trampoline + the `onParallelRegionRemoteSend_` std::function
//      dispatch + the SM ctor closure body + sendParallelRegionDone()
//      + the second std::function dispatch.
//   C. The (B - A) delta is the cost the 5-layer indirection adds
//      beyond raw envelope construction — i.e. the cost that a
//      collapse would actually save. Tracking both lets a reviewer
//      see whether a slowdown lives in envelope building or in the
//      indirection chain.
//
// Out of scope:
//   - L1 (`policy::executeEntryActions`) — requires a generated policy
//     branch that emits `engine.triggerParallelRegionRemoteSend(...)`.
//     Adding the entire policy to this fixture would couple the bench
//     to template emission shape, defeating purpose 1 above. The cost
//     L1→L2 is one direct call (the engine method call); use B as the
//     proxy for "everything past the policy boundary".
//   - L6 (`shm.send` / transport) — replaced with a no-op callback. The
//     actual transport latency is dominated by IPC syscalls
//     (microseconds) and is independent of the indirection chain. A
//     full-stack measurement would be a separate `mesh_*` ctest, not a
//     micro-benchmark.
//   - Failure paths (`runtime_error` throw when callback is unset or
//     returns false). Cold path, not what a regression guard tracks.

#include "static/StaticExecutionEngine.h"
#include "mesh/MeshEnvelope.h"
#include "mesh/PatternKind.h"
#include "mesh/PayloadCodec.h"

#include <benchmark/benchmark.h>

#include <cstdint>
#include <functional>
#include <optional>
#include <string>
#include <utility>

namespace {

// Minimal Policy matching the shape required by `StaticExecutionEngine` —
// identical pattern to `StaticExecutionEngineBenchmark.cpp` (#3). The
// Policy is irrelevant to wire-21 dispatch (the chain bypasses the
// Policy entirely past the engine entry point), so a no-op shell
// satisfies all concept requirements.
struct MinimalPolicy {
    enum class State : std::uint8_t { S0, Done };
    enum class Event : std::uint8_t { NONE, Tick };

    static constexpr bool HAS_PARALLEL_STATES = false;
    static constexpr bool NEEDS_SCRIPT_ENGINE = false;

    mutable State lastTransitionSourceState_{};
    mutable bool lastTransitionIsInternal_ = false;
    mutable bool lastTransitionIsTargetless_ = false;

    MinimalPolicy() = default;

    [[nodiscard]] static constexpr State initialState() noexcept { return State::S0; }
    [[nodiscard]] static constexpr bool isFinalState(State s) noexcept { return s == State::Done; }
    [[nodiscard]] static constexpr std::optional<State> getParent(State) noexcept { return std::nullopt; }
    [[nodiscard]] static constexpr bool isCompoundState(State) noexcept { return false; }
    [[nodiscard]] static constexpr State getInitialChild(State s) noexcept { return s; }

    [[nodiscard]] std::string getEventName(Event) const { return ""; }
    [[nodiscard]] std::optional<Event> getEventFromName(const std::string&) const { return std::nullopt; }
};

// Hand-rolled SM mirroring the NonRoot wire-21 block emitted by
// `tools/codegen/templates/state_machine.jinja2:778-836`. The ctor
// installs the `setParallelRegionRemoteSendCallback` closure (L2→L3
// hop) that forwards into `sendParallelRegionDone` (L3→L4).
//
// `sendParallelRegionDone` is a byte-for-byte mirror of the template's
// emitted body modulo the `throw` paths (which are cold-cold and not
// being measured). Routing logic stays identical to the template:
// optional check on env fields, then dispatch through the
// `parallel_region_done_callback_` std::function (L4→L5 hop).
class Wire21SenderSm : public ::SCE::Static::StaticExecutionEngine<MinimalPolicy> {
public:
    Wire21SenderSm() {
        this->setParallelRegionRemoteSendCallback(
            [this](const std::string& parallel_id, const std::string& region_id,
                   const std::string& donedata) {
                this->sendParallelRegionDone(parallel_id, region_id, donedata);
            });
    }

    void sendParallelRegionDone(const std::string& parallel_id,
                                const std::string& region_id,
                                const std::string& donedata) {
        ::SCE::Mesh::MeshEnvelope env;
        env.pattern = ::SCE::Mesh::PatternKind::ParallelRegionDone;
        env.parallel_id = parallel_id;
        env.region_id = region_id;
        env.data.assign(donedata.begin(), donedata.end());
        env.datacontenttype = ::SCE::Mesh::PayloadCodec::Json;
        if (parallel_region_done_callback_) {
            parallel_region_done_callback_(env);
        }
    }

    void setParallelRegionDoneCallback(
        std::function<bool(const ::SCE::Mesh::MeshEnvelope&)> cb) {
        parallel_region_done_callback_ = std::move(cb);
    }

private:
    std::function<bool(const ::SCE::Mesh::MeshEnvelope&)> parallel_region_done_callback_;
};

// Inputs are kept on the stack outside the benchmark loop so per-iter
// string construction / SBO churn does not contaminate the dispatch
// measurement. `donedata` is empty — matches the
// `state.donedata`-absent branch in `parallel_final.jinja2:42` which
// is the common case.
const std::string kParallelId = "bench_parallel";
const std::string kRegionId   = "bench_region";
const std::string kDonedata   = "";

}  // namespace

// ─── A. L4→L5 — `sendParallelRegionDone` direct call ─────────────────
//
// Envelope construction (5 field assignments) + 1 std::function
// dispatch into the installed no-op callback. This is the minimum
// possible cost for a wire-21 send: any consumer that uses the SM
// directly (skipping the engine-level trampoline) pays at least this.
static void BM_SendParallelRegionDoneDirect(benchmark::State& state) {
    Wire21SenderSm sm;
    sm.setParallelRegionDoneCallback(
        [](const ::SCE::Mesh::MeshEnvelope& env) -> bool {
            // DoNotOptimize on a non-const pointer — the const-ref overload
            // is deprecated in google-benchmark (under -Werror it fails).
            const auto* env_ptr = &env;
            benchmark::DoNotOptimize(env_ptr);
            return true;
        });
    for (auto _ : state) {
        sm.sendParallelRegionDone(kParallelId, kRegionId, kDonedata);
    }
}
BENCHMARK(BM_SendParallelRegionDoneDirect);

// ─── B. L2→L5 — `triggerParallelRegionRemoteSend` full chain ─────────
//
// Engine trampoline → onParallelRegionRemoteSend_ std::function dispatch
// → SM ctor closure body → sendParallelRegionDone() (which itself does
// envelope construction + the parallel_region_done_callback_ std::function
// dispatch). Two std::function hops in this path vs. one in (A).
static void BM_TriggerParallelRegionRemoteSend(benchmark::State& state) {
    Wire21SenderSm sm;
    sm.setParallelRegionDoneCallback(
        [](const ::SCE::Mesh::MeshEnvelope& env) -> bool {
            // DoNotOptimize on a non-const pointer — the const-ref overload
            // is deprecated in google-benchmark (under -Werror it fails).
            const auto* env_ptr = &env;
            benchmark::DoNotOptimize(env_ptr);
            return true;
        });
    for (auto _ : state) {
        sm.triggerParallelRegionRemoteSend(kParallelId, kRegionId, kDonedata);
    }
}
BENCHMARK(BM_TriggerParallelRegionRemoteSend);

BENCHMARK_MAIN();
