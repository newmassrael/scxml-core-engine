// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §9.6 Session 5b — Zenoh scxml-invoke two-host roundtrip (parent).
//
// Companion to test_mesh_zenoh_scxml_invoke_crossdev_worker. The two
// halves run inside distinct Linux network namespaces wired by veth
// (see tests/mesh/setup_crossdev_netns.sh) so the wire-14/15/18
// lifecycle traverses a real network stack — TCP across
// 172.16.10.1/.2 — instead of looping through 127.0.0.1.
//
// This half hosts the listen anchor: deploy_scxml_invoke_zenoh_crossdev.yaml
// configures `transports.zenoh.listen: tcp/172.16.10.1:17449` for the
// parent ECU, so init() opens a Zenoh peer session bound to that
// endpoint. The 1-process Session 5 fixture instead opens a separate
// relay session in the test driver (open_peer + wait_for_peer_ready);
// here the parent's own listen socket is the rendezvous point and
// Zenoh's connect-retry on the worker side handles a worker-comes-up-
// first race without a third process.

#include "scxml_invoke_zenoh_crossdev_parent_sm.h"
#include "scxml_invoke_zenoh_crossdev_parent_transport.h"

#include <chrono>
#include <cstdio>
#include <thread>

int main() {
    using ParentEngine = SCE::Generated::scxml_invoke_zenoh_crossdev_parent::scxml_invoke_zenoh_crossdev_parent;
    ParentEngine parent;
    SCE::Generated::scxml_invoke_zenoh_crossdev_parent::TransportRouter<ParentEngine> parent_router({&parent});
    if (!parent_router.init()) {
        std::fprintf(stderr, "FAIL: parent_router.init() returned false\n");
        return 1;
    }

    // Subscriber-propagation window. Even with the orchestrator's
    // 500 ms settle on the worker side, the worker's pending zenoh
    // connect to 172.16.10.1:17449 only resolves once *this* listen
    // socket binds — which is right above. Sleep here gives the
    // worker time to (a) finish establishing its TCP link and
    // (b) propagate its `sce/scxml_invoke/c2p/...` Subscriber
    // declaration to this peer's publish-routing table. The
    // 1-process Session 5 fixture absorbs the same convergence with
    // 200 ms after wait_for_peer_ready; 500 ms is the conservative
    // 2-host budget — round-trip declarations across veth dominate
    // the loopback case.
    std::this_thread::sleep_for(std::chrono::milliseconds(500));

    parent.initialize();

    using State = SCE::Generated::scxml_invoke_zenoh_crossdev_parent::State;
    using clock = std::chrono::steady_clock;
    const auto deadline = clock::now() + std::chrono::seconds(15);
    while (clock::now() < deadline) {
        // pumpScxmlInvokeReplies is a no-op on the parent — the
        // ZenohScxmlInvokeEndpoint's receive callback dispatches
        // wire-15/16/18/20 inline via dispatchToSession (§14.4
        // thread-safe contract on the Zenoh runtime callback thread).
        // Kept for symmetry with the 1-process fixture.
        parent_router.pumpScxmlInvokeReplies();
        parent.step();
        if (parent.getCurrentState() == State::Pass) {
            std::printf("SCE Mesh §9.6 Session 5b Zenoh scxml-invoke crossdev: PASS\n");
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
                 "Either the worker's TCP connect to 172.16.10.1:17449 "
                 "never landed (check `ss -t` inside parent netns) or "
                 "Zenoh subscriber declarations did not propagate before "
                 "wire-14 fired. Current parent state=%d\n",
                 static_cast<int>(parent.getCurrentState()));
    return 1;
}
