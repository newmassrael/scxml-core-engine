// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §16.5 L3500 — NonRoot partition process for the barrier-
// timeout runtime E2E pair (fires + cancels).
//
// Forked by the fires / cancels drivers via `execv` with a single
// argv selector:
//
//   "silent" — fires test. Constructs the TransportRouter so the
//              outbound shm channel is created (the Root side needs
//              it open to succeed on its own Mode::Open retries),
//              then holds the process alive without ever driving the
//              `right` region to its `<final>`. No wire-21 envelope
//              is ever sent.
//   "sends"  — cancels test. Identical to
//              `test_mesh_partition_rule12_right_proc.cpp`: drives
//              the region to final, the NonRoot branch of
//              `parallel_final.jinja2` synchronously dispatches the
//              wire-21 envelope, then holds the channel alive briefly
//              so the Root can drain it.

#include "motor_partition_sm.h"
#include "motor_partition_transport.h"

#include <chrono>
#include <cstdio>
#include <cstring>
#include <string>
#include <thread>

namespace gen = SCE::Generated::motor_partition;

int main(int argc, char* argv[]) {
    if (argc < 2) {
        std::fprintf(stderr, "right_proc: usage: %s <silent|sends>\n", argv[0]);
        return 64;
    }
    const std::string mode = argv[1];
    const bool sends = (mode == "sends");
    const bool silent = (mode == "silent");
    if (!sends && !silent) {
        std::fprintf(stderr, "right_proc: unknown mode '%s'\n", mode.c_str());
        return 65;
    }

    gen::motor_partition sm;
    sm.initialize();

    using Router = gen::TransportRouter<gen::motor_partition>;
    Router router({&sm});

    if (sends) {
        sm.processEvent(gen::Event::Ping_right);
        sm.step();
        sm.processEvent(gen::Event::Finalize_right);
        sm.step();
    }

    // Hold the shm segment alive long enough for the Root side to
    // complete its assertion. The Root's iteration budget caps at
    // ~800 ms (80 * 10 ms); 1000 ms guarantees the channel outlives
    // any scheduling jitter, whether the Root ends up observing the
    // timer fire (silent mode) or the convergence (sends mode).
    std::this_thread::sleep_for(std::chrono::milliseconds(1000));

    std::printf("barrier_timeout right (%s): PASS%s\n",
                mode.c_str(),
                sends ? " — wire-21 envelope dispatched"
                      : " — channel held open, no envelope sent");
    return 0;
}
