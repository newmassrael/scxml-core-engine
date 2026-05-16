// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §14 rule 12 / §16.5 — NonRoot partition process for the
// inter-partition `ParallelRegionDone` (wire 21) E2E harness.
//
// Spawned by `test_mesh_partition_rule12_e2e.cpp` via fork+exec. Drives
// the locally-hosted region (`right`) into its `<final>`; the NonRoot
// branch of `mesh/cpp/parallel_final.jinja2` then calls
// `engine.sendParallelRegionDone("root", "right", ...)`, which the
// `TransportRouter` ctor wired through the partition's outbound
// wire-21 shm channel (`/sce_p21_motor_right_motor_left`). The Root
// process opens the corresponding inbound channel and aggregates the
// remote completion into its `ParallelCompletionTracker`.
//
// This process holds the shm channel alive briefly after the send so
// the Root has time to drain it; without that grace period the
// shm_unlink on dtor could race ahead of the slow-to-schedule reader.

#include "motor_partition_sm.h"
#include "motor_partition_transport.h"

#include <chrono>
#include <cstdio>
#include <thread>

namespace gen = SCE::Generated::motor_partition::P_motor_right;

int main() {
    gen::motor_partition sm;
    sm.initialize();

    // The TransportRouter ctor opens the outbound shm channel
    // (`Mode::Create`) and installs the wire-21 callback on the SM.
    using Router = gen::TransportRouter<gen::motor_partition>;
    Router router({&sm});

    // Drive the local region (`right`) into its `<final>`. The NonRoot
    // branch of `mesh/cpp/parallel_final.jinja2` synchronously calls
    // `engine.sendParallelRegionDone("root", "right", ...)`, which
    // routes through the wire-21 callback into
    // `wire21_to_motor_left_.send(env)`.
    sm.processEvent(gen::Event::Ping_right);
    sm.step();
    sm.processEvent(gen::Event::Finalize_right);
    sm.step();

    // Hold the shm segment alive long enough for the Root reader to
    // open and drain it. The Root process retries `Mode::Open` on each
    // `pumpWire21Inbound()` call, so a brief grace period suffices even
    // when the Root is slow to launch. SCE_MESH.md §10.10's outbound
    // backpressure semantics do not apply here — wire-21 is a one-shot
    // single-envelope flow per region activation.
    std::this_thread::sleep_for(std::chrono::milliseconds(2000));

    std::printf("rule12 right (NonRoot): PASS — wire-21 envelope dispatched\n");
    return 0;
}
