// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §16.7 row 8 — machine-level liveness E2E (SOME/IP), worker side.
//
// Hosts the `beta_machine_liveness` SCE binary (sce-mesh-worker netns,
// 172.16.10.2). Brings up the consolidated SCE app
// `beta_machine_liveness_sce` so its §16.7 row 8 liveness service
// (0x8281) is offered over SOMEIP-SD, lets the orchestrator's
// SETTLE_MS window plus an extra steady-state hold elapse so the
// parent has time to converge AVAILABLE for 0x8281, then shuts the
// router down cleanly — vsomeip emits STOP_OFFER on shutdown which
// propagates the `available=false` edge to the parent's
// register_availability_handler.
//
// Sleep budget breakdown (vsomeip SD physics):
//   - Orchestrator SETTLE_MS = 500 ms after LISTEN_READY → parent boots.
//   - Parent init() + RM start + SD Find/Offer round-trip → ~100-300 ms.
//   - Parent's handler observes AVAILABLE for 0x8281 → ~ T=800 ms.
// We sleep 2500 ms after LISTEN_READY before shutdown, leaving ~1500 ms
// of post-AVAILABLE steady state so a slow CI node still converges
// AVAILABLE before the loss edge fires.

#include "beta_machine_liveness_transport.h"

#include "MeshTestUtils.h"
#include "SomeipTestUtils.h"

#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <thread>

#ifndef VSOMEIP_CONFIG_PATH
#error "VSOMEIP_CONFIG_PATH must be defined by CMake (path to vsomeip_someip_machine_liveness_beta.json)"
#endif

int main() {
    setenv("VSOMEIP_CONFIGURATION", VSOMEIP_CONFIG_PATH, 1);

    SCE::Test::Mesh::wipe_stale_vsomeip_sockets();

    using namespace SCE::Test::Mesh;
    namespace beta_gen = SCE::Generated::beta_machine_liveness;
    using RouterT = beta_gen::TransportRouter<TestSenderEngine>;

    TestSenderEngine sender;
    RouterT router({&sender});

    if (!router.init()) {
        std::fprintf(stderr, "FAIL: worker router.init() returned false\n");
        return 1;
    }

    // Orchestrator handshake: parent does not start until LISTEN_READY
    // appears on stderr.
    std::fprintf(stderr, "LISTEN_READY\n");
    std::fflush(stderr);

    // Steady-state hold so parent converges AVAILABLE for 0x8281 before
    // the loss edge fires. See sleep budget breakdown in the file header.
    std::this_thread::sleep_for(std::chrono::milliseconds(2500));

    // Clean shutdown drives vsomeip's STOP_OFFER (synchronous SD message
    // before the RM tears down). Parent's register_availability_handler
    // receives the resulting `available=false` edge promptly — this is
    // what materializes the row-8 PEER_PARTITIONED raise on parent.
    router.shutdown();
    return 0;
}
