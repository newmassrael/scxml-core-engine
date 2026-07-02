// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §9.6 Session 5b — SOME/IP scxml-invoke two-host roundtrip (parent).
//
// Companion to test_mesh_someip_scxml_invoke_crossdev_worker. The two
// halves run inside distinct Linux network namespaces wired by veth
// (see tests/mesh/setup_crossdev_netns.sh) so the wire-14/15/18
// lifecycle traverses a real network stack — multicast SD, ARP, and
// IP routing — instead of looping through 127.0.0.1.
//
// Per-binary VSOMEIP_CONFIGURATION:
//   This 2-host fixture cannot use ctest's ENVIRONMENT property the way
//   the 1-process Session 4b roundtrip does, because the two halves
//   need different vsomeip.json files (distinct routing-manager
//   identities + unicast IPs). CMake injects the right path via
//   VSOMEIP_CONFIG_PATH and we setenv() it before TransportRouter::init()
//   so the call into vsomeip's app factory loads the parent config.

#include "scxml_invoke_someip_crossdev_parent_sm.h"
#include "scxml_invoke_someip_crossdev_parent_transport.h"

#include "SomeipTestUtils.h"

#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <thread>

#ifndef VSOMEIP_CONFIG_PATH
#error "VSOMEIP_CONFIG_PATH must be defined by CMake (path to vsomeip_scxml_invoke_crossdev_parent.json)"
#endif

int main() {
    setenv("VSOMEIP_CONFIGURATION", VSOMEIP_CONFIG_PATH, 1);

    // Wipe stale routing-manager UNIX sockets in /tmp left over by
    // crashed prior runs. Filesystem is shared across netns, so a stale
    // socket from any earlier sce_scxml_invoke_someip_crossdev_*
    // invocation could prevent vsomeip from binding here.
    SCE::Test::Mesh::wipe_stale_vsomeip_sockets();

    using ParentEngine = SCE::Generated::scxml_invoke_someip_crossdev_parent::scxml_invoke_someip_crossdev_parent;
    ParentEngine parent;
    SCE::Generated::scxml_invoke_someip_crossdev_parent::TransportRouter<ParentEngine> parent_router({&parent});
    if (!parent_router.init()) {
        std::fprintf(stderr, "FAIL: parent_router.init() returned false\n");
        return 1;
    }

    // SD-convergence window. The orchestrator's 500 ms only covers the
    // worker → parent direction (worker's SD has been advertising its
    // Offer since orchestrator pre-launch). This sleep covers the
    // parent → worker direction: parent's RM needs to multicast its
    // own Offer + receive the worker's Offer + match its pending
    // request_service before the wire-14 send below has a routing
    // target. vsomeip default `initial_delay_max` alone is up to 100 ms
    // on the first SD broadcast; round-trip discovery + service-link
    // setup adds more. The 1-process Session 4b roundtrip absorbs the
    // same convergence with 500 ms after both routers init() — we use
    // the same budget here since the bidirectional discovery has the
    // same physics whether the routers share a process or share a wire.
    std::this_thread::sleep_for(std::chrono::milliseconds(500));

    parent.initialize();

    using State = SCE::Generated::scxml_invoke_someip_crossdev_parent::State;
    using clock = std::chrono::steady_clock;
    const auto deadline = clock::now() + std::chrono::seconds(15);
    while (clock::now() < deadline) {
        // pumpScxmlInvokeReplies is a no-op on the parent side — the
        // endpoint's receive callback dispatches wire-15/18 inline via
        // dispatchToSession (§14.4 thread-safe contract). Kept for
        // symmetry with the 1-process fixture.
        parent_router.pumpScxmlInvokeReplies();
        parent.step();
        if (parent.getCurrentState() == State::Pass) {
            std::printf("SCE Mesh §9.6 Session 5b SOME/IP scxml-invoke crossdev: PASS\n");
            return 0;
        }
        if (parent.getCurrentState() == State::Fail) {
            std::fprintf(stderr, "FAIL: parent observed error.execution — wire-14 "
                                 "reached the worker but the success path "
                                 "(wire-15 + wire-18) did not complete.\n");
            return 1;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(20));
    }

    std::fprintf(stderr,
                 "FAIL: parent did not reach State::Pass within 15 s. "
                 "Either SOME/IP-SD on the veth pair never converged "
                 "(check setup_crossdev_netns.sh multicast route) or the "
                 "worker's wire-15/18 reply did not return. Current parent "
                 "state=%d\n",
                 static_cast<int>(parent.getCurrentState()));
    return 1;
}
