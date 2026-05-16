// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §16.8 seed — Root partition registration for W3C test417.
//
// Hosts the `s1p11` region plus the §16.5 ParallelCompletionTracker for
// `<parallel id="s1p1">`. Drives the local region into its `<final>`
// (NULL transitions run during `initialize()`), then pumps the inbound
// wire-21 shm channel until the NonRoot partition's
// `ParallelRegionDone` lands, the tracker raises
// `Done_state_s1p1`, and the SM drives the parallel's
// `<transition event="done.state.s1p1" target="pass"/>` through the
// macrostep into `<final id="pass">`. Returns 0 on success; non-zero
// on the `<final id="fail">` timeout path (remote never reported) or
// on max-iteration expiry.
//
// Compiled once per partition binary via the w3c_dist_partition_runner
// source — the per-partition binary includes exactly one of
// Test417_Main.h / Test417_Worker.h via the CMake-supplied
// `SCE_W3C_DIST_PARTITION_HEADER` preprocessor selection, dodging the
// namespace collision that §16.8 arch-debt #4 tracks.

#pragma once

#include "PartitionRunner.h"
#include "test417_sm.h"
#include "test417_transport.h"

#include <chrono>
#include <cstdio>
#include <string>
#include <thread>

namespace SCE::W3C::Dist {

struct Test417_Main : public PartitionBase {
    // Registry keys (matched verbatim against manifest partition names
    // and against the --test-id / --partition CLI args).
    static constexpr const char *TEST_ID = "417";
    static constexpr const char *PARTITION = "test417_main";

    int run(const std::string & /*deploy_dir*/) override {
        namespace gen = SCE::Generated::test417::P_test417_main;

        gen::test417 sm;
        // Router ctor opens the inbound wire-21 shm channel in
        // `Mode::Open`. The NonRoot partition creates the matching
        // segment in `Mode::Create` before emitting — launch ordering
        // is handled by run_distributed.cpp.
        gen::TransportRouter<gen::test417> router({&sm});

        // `initialize()` enters `s1` → `s1p1` → (`s1p11`, `s1p12`),
        // then runs the NULL transition chain inside the local region
        // (`s1p111` → `s1p11final`). On entry to `s1p11final` the
        // codegen's local-complete callback calls
        // `tracker_s1p1_.onLocalRegionComplete("s1p11")`. The `s1`
        // onentry's `<send event="timeout" delay="1s"/>` is scheduled
        // here; we pump the scheduler via `tick()` below so the
        // timer can fire and drive `<final id="fail">` if the remote
        // partition never reports.
        sm.initialize();

        // Bounded wait for the NonRoot's wire-21 envelope. The
        // deadline is ~3 s at 10 ms polls — the test's own timeout is
        // 1 s, so a slow worker surfaces first as the SM reaching
        // `Fail` (returning 11). The outer cap catches harness bugs
        // where both the worker and the timeout send fail to land.
        constexpr int MAX_ITERATIONS = 300;
        constexpr auto POLL_INTERVAL = std::chrono::milliseconds(10);

        for (int i = 0; i < MAX_ITERATIONS; ++i) {
            std::size_t drained = router.pumpWire21Inbound();
            if (drained > 0) {
                // Remote region reported. `onParallelRegionDone` has
                // already updated the tracker; `step()` drains the
                // `done.state.s1p1` raise the tracker emits synchronously.
                sm.step();
            }
            // `tick()` pumps the scheduler (fires the 1 s `timeout`
            // send when due) and runs one step. Short-circuits once
            // the SM reaches a top-level final.
            sm.tick();

            if (sm.getCurrentState() == gen::State::Pass) {
                std::fprintf(stdout,
                             "test417 main: PASS — done.state.s1p1 landed, SM in <final id=\"pass\">\n");
                return 0;
            }
            if (sm.getCurrentState() == gen::State::Fail) {
                std::fprintf(stderr,
                             "test417 main: FAIL — timeout raised before both regions converged "
                             "(remote wire-21 did not arrive within 1 s)\n");
                return 11;
            }

            std::this_thread::sleep_for(POLL_INTERVAL);
        }

        std::fprintf(stderr,
                     "test417 main: FAIL — %d * %lld ms elapsed without Pass/Fail; "
                     "currentState=%d\n",
                     MAX_ITERATIONS,
                     static_cast<long long>(POLL_INTERVAL.count()),
                     static_cast<int>(sm.getCurrentState()));
        return 12;
    }
};

inline static PartitionRegistrar<Test417_Main> registrar_Test417_Main;

}  // namespace SCE::W3C::Dist
