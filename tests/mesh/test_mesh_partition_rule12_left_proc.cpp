// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §14 rule 12 / §16.5 — Root partition process for the
// inter-partition `ParallelRegionDone` (wire 21) E2E harness.
//
// Spawned by `test_mesh_partition_rule12_e2e.cpp` via fork+exec. Drives
// the locally-hosted region (`left`) into its `<final>`, then loops
// pumping the inbound shm channel for the wire-21 envelope from the
// NonRoot partition (`motor_right`). The §16.5
// `ParallelCompletionTracker` aggregates the local + remote completions
// and raises `done.state.root` into the SM's external queue when the
// per-`<parallel>` threshold is reached. The `<transition event=
// "done.state.root" target="all_done"/>` on `<parallel id="root">`
// then drives the SM into `<final id="all_done">`, which the
// assertion below observes via `isInFinalState() &&
// getCurrentState() == State::All_done`.
//
// Exits 0 on success; non-zero on timeout or SCXML state mismatch. The
// driver process treats any non-zero exit as a hard failure and prints
// the captured stderr.

#include "motor_partition_sm.h"
#include "motor_partition_transport.h"

#include <chrono>
#include <cstdio>
#include <thread>

namespace gen = SCE::Generated::motor_partition::P_motor_left;

namespace {

// 4 seconds upper bound: NonRoot launches first and immediately drives
// `finalize_right`, so wire-21 should land within milliseconds. The
// generous bound absorbs CI scheduling jitter without masking a real
// deadlock — a stuck tracker still surfaces as a non-zero exit.
constexpr int MAX_ITERATIONS = 400;
constexpr auto POLL_INTERVAL = std::chrono::milliseconds(10);

}  // namespace

int main() {
    gen::motor_partition sm;
    sm.initialize();

    using Router = gen::TransportRouter<gen::motor_partition>;
    Router router({&sm});

    // Drive the local region (`left`) into its `<final>`. The Root
    // branch of `mesh/cpp/parallel_final.jinja2` calls
    // `engine.tracker_root_.onLocalRegionComplete("left")` from the
    // entry actions of `<final id="left_done">`.
    sm.processEvent(gen::Event::Ping_left);
    sm.step();
    sm.processEvent(gen::Event::Finalize_left);
    sm.step();

    for (int i = 0; i < MAX_ITERATIONS; ++i) {
        std::size_t drained = router.pumpWire21Inbound();
        if (drained > 0) {
            // The tracker raised `done.state.root` synchronously inside
            // `onRemoteRegionComplete` once the threshold count is hit;
            // step the SM so the queued event drives the
            // `<parallel id="root">` exit transition into `<final
            // id="all_done">`.
            sm.step();
        }
        if (sm.isInFinalState() && sm.getCurrentState() == gen::State::All_done) {
            std::printf("rule12 left (Root): PASS — done.state.root raised, SM in <final id=\"all_done\">\n");
            return 0;
        }
        std::this_thread::sleep_for(POLL_INTERVAL);
    }

    std::fprintf(stderr,
                 "rule12 left (Root): FAIL — wire-21 did not arrive within %d * %lld ms timeout. "
                 "currentState=%d isFinal=%d\n",
                 MAX_ITERATIONS, static_cast<long long>(POLL_INTERVAL.count()), static_cast<int>(sm.getCurrentState()),
                 static_cast<int>(sm.isInFinalState()));
    return 11;
}
