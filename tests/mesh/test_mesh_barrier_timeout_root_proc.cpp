// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §16.5 L3500 — Root partition process for the barrier-
// timeout runtime E2E pair (fires + cancels).
//
// Forked by `test_mesh_barrier_timeout_fires_e2e.cpp` and the sibling
// cancels driver via `execv` with a single argv selector:
//
//   "timeout"    — fires test. Drives the local `left` region to
//                  `<final>`, then polls waiting for the scheduler to
//                  pop the §16.7 row 6 `error.communication` timer
//                  event. Asserts the SM settles into
//                  `<final id="timeout_failed">`.
//   "convergence" — cancels test. Same local drive, but ALSO drains
//                   the inbound wire-21 channel; when the NonRoot's
//                   `ParallelRegionDone` arrives the tracker reaches
//                   threshold, cancels the timer, and raises
//                   `done.state.root`. Asserts `<final id="all_done">`.
//
// Exits 0 on success; non-zero on timeout or state mismatch.

#include "motor_partition_sm.h"
#include "motor_partition_transport.h"

#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>
#include <thread>

namespace gen = SCE::Generated::motor_partition;

namespace {

// 800 ms upper bound: the barrier timer fires at 150 ms
// (`deploy_partitions_barrier_fast.yaml`), and the convergence path
// drains wire-21 within milliseconds. Generous tail absorbs CI
// scheduler jitter without masking a real deadlock.
constexpr int MAX_ITERATIONS = 80;
constexpr auto POLL_INTERVAL = std::chrono::milliseconds(10);

}  // namespace

int main(int argc, char* argv[]) {
    if (argc < 2) {
        std::fprintf(stderr, "root_proc: usage: %s <timeout|convergence>\n", argv[0]);
        return 64;
    }
    const std::string mode = argv[1];
    const bool expect_timeout = (mode == "timeout");
    const bool expect_convergence = (mode == "convergence");
    if (!expect_timeout && !expect_convergence) {
        std::fprintf(stderr, "root_proc: unknown mode '%s'\n", mode.c_str());
        return 65;
    }

    gen::motor_partition sm;
    sm.initialize();

    using Router = gen::TransportRouter<gen::motor_partition>;
    Router router({&sm});

    // Drive the local region (`left`) into its `<final>`. Both modes
    // take the same local path — the barrier timer arms on this first
    // completion. The branching happens later on whether the remote
    // side ever answers.
    sm.processEvent(gen::Event::Ping_left);
    sm.step();
    sm.processEvent(gen::Event::Finalize_left);
    sm.step();

    const gen::State expected_state =
        expect_timeout ? gen::State::Timeout_failed : gen::State::All_done;
    const char* expected_label =
        expect_timeout ? "timeout_failed" : "all_done";

    for (int i = 0; i < MAX_ITERATIONS; ++i) {
        if (expect_convergence) {
            // Convergence mode drains wire-21 so the NonRoot's
            // ParallelRegionDone reaches the tracker before the timer
            // fires. The cancel-then-raise path inside the tracker
            // keeps the timer event off the external queue entirely.
            (void)router.pumpWire21Inbound();
        }
        // Canonical polling API. `tick()` pumps the base engine's
        // `PullScheduler` and dispatches any raised events through
        // the normal microstep. The short-circuit guard inside
        // `tick()` is `isGlobalFinalState()` (top-level-final only),
        // so a Root partition sitting in a `<parallel>` whose local
        // region has reached a regional `<final>` while the remote
        // sibling is still pending continues to pump — which is
        // exactly the window the §16.5 barrier timer needs.
        sm.tick();

        if (sm.isInFinalState() && sm.getCurrentState() == expected_state) {
            std::printf("barrier_timeout root (%s): PASS — reached <final id=\"%s\">\n",
                        mode.c_str(), expected_label);
            return 0;
        }
        std::this_thread::sleep_for(POLL_INTERVAL);
    }

    std::fprintf(stderr,
                 "barrier_timeout root (%s): FAIL — did not reach <final id=\"%s\"> "
                 "within %d * %lld ms. currentState=%d isFinal=%d\n",
                 mode.c_str(), expected_label, MAX_ITERATIONS,
                 static_cast<long long>(POLL_INTERVAL.count()),
                 static_cast<int>(sm.getCurrentState()),
                 static_cast<int>(sm.isInFinalState()));
    return expect_timeout ? 11 : 12;
}
