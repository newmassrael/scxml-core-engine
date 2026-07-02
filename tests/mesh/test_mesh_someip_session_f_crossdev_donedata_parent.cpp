// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §9.6 Session 5c — SOME/IP donedata two-host roundtrip (parent).
//
// Sister to test_mesh_someip_scxml_invoke_crossdev_parent.cpp (5b
// lifecycle). The parent SCXML is the same parent_session_f_donedata.scxml
// the shm + custom_tcp + zenoh donedata fixtures already validate;
// only the deploy.yaml rebinds peer references onto SOME/IP. Reaching
// State::Pass implies the same three donedata shapes (param object,
// primitive content, nested object/array/integer) survive a netns +
// veth + SOMEIP-SD multicast + vsomeip routing-manager round-trip
// with semantically equivalent decoded `_event.data` to the shm baseline.
//
// Per-binary VSOMEIP_CONFIGURATION:
//   This 2-host fixture cannot use ctest's ENVIRONMENT property the
//   way the 1-process Session 4b roundtrip does, because the two halves
//   need different vsomeip.json (parent on 172.16.10.1, worker on
//   172.16.10.2 with three application identities). CMake injects the
//   right path via VSOMEIP_CONFIG_PATH and we setenv() it before
//   TransportRouter::init() so vsomeip's app factory loads the parent
//   config. Same pattern as 5b lifecycle.

#include "parent_session_f_donedata_sm.h"
#include "parent_session_f_donedata_transport.h"

#include "SomeipTestUtils.h"

#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <thread>

#ifndef VSOMEIP_CONFIG_PATH
#error "VSOMEIP_CONFIG_PATH must be defined by CMake (path to vsomeip_session_f_donedata_crossdev_parent.json)"
#endif

int main() {
    setenv("VSOMEIP_CONFIGURATION", VSOMEIP_CONFIG_PATH, 1);

    // Wipe stale routing-manager UNIX sockets in /tmp left over by
    // crashed prior runs. /tmp is shared across netns, so a stale
    // socket from any earlier sce_session_f_donedata_someip_crossdev_*
    // invocation could prevent vsomeip from binding here. Same
    // defence-in-depth pattern as Session 5b lifecycle.
    SCE::Test::Mesh::wipe_stale_vsomeip_sockets();

    using ParentEngine = SCE::Generated::parent_session_f_donedata::parent_session_f_donedata;
    ParentEngine parent;
    SCE::Generated::parent_session_f_donedata::TransportRouter<ParentEngine> parent_router({&parent});
    if (!parent_router.init()) {
        std::fprintf(stderr, "FAIL: parent_router.init() returned false\n");
        return 1;
    }

    // SD-convergence window. The orchestrator's 500 ms covers the
    // worker → parent direction (worker's RM has been advertising its
    // three Offers since pre-launch). This sleep covers the parent →
    // worker direction: parent's RM needs to multicast its own Offer +
    // receive the worker's three Offers + match its three pending
    // request_service calls before any of the wire-14 sends below has
    // a routing target. The 1-process Session 4b roundtrip absorbs
    // similar SD physics with 500 ms; three Offers from the worker
    // RM still arrive in one cyclic_offer_delay window (500 ms), so
    // the same budget covers all three.
    std::this_thread::sleep_for(std::chrono::milliseconds(500));

    parent.initialize();

    using State = SCE::Generated::parent_session_f_donedata::State;
    using clock = std::chrono::steady_clock;
    const auto deadline = clock::now() + std::chrono::seconds(20);
    while (clock::now() < deadline) {
        // pumpScxmlInvokeReplies is a no-op on the parent — the
        // SomeipScxmlInvokeEndpoint's receive callback dispatches
        // wire-15/18 inline via dispatchToSession (§14.4 thread-safe
        // contract on vsomeip's RM thread).
        parent_router.pumpScxmlInvokeReplies();
        parent.step();
        if (parent.getCurrentState() == State::Pass) {
            std::printf("SCE Mesh §9.6 Session 5c SOME/IP donedata crossdev: PASS\n");
            return 0;
        }
        if (parent.getCurrentState() == State::Fail) {
            std::fprintf(stderr, "FAIL: parent took the fail transition. One of the "
                                 "three donedata conds did not match the wire-18 "
                                 "payload over SOME/IP — either the param shape "
                                 "lost the name/value pair, the content shape "
                                 "dropped the primitive string, or the nested shape "
                                 "regressed the canonical-JSON contract.\n");
            return 1;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(20));
    }

    std::fprintf(stderr,
                 "FAIL: parent did not reach State::Pass within 20 s. "
                 "Either SOME/IP-SD on the veth pair never converged "
                 "(check setup_crossdev_netns.sh multicast route) or "
                 "the worker's RM did not publish all three Offers in "
                 "time, or vsomeip mishandled the three-app routing-"
                 "manager-client multiplexing on the worker side. "
                 "Current parent state=%d\n",
                 static_cast<int>(parent.getCurrentState()));
    return 1;
}
