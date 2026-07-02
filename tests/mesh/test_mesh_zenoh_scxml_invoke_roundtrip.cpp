// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §9.6 Session 5 — Zenoh scxml-invoke single-process E2E.
//
// Single-process proof that the §9.6 wire-14/20 lifecycle travels
// over Zenoh:
//   wire-14 InvokeStart  (parent → worker, key sce/scxml_invoke/p2c/...)
//   wire-15 InvokeStarted (worker → parent, key sce/scxml_invoke/c2p/...)
//   wire-18 InvokeDone    (worker → parent, key sce/scxml_invoke/c2p/...)
//
// Both TransportRouters run in one process. Each opens its own
// `zenoh::Session` configured by deploy_scxml_invoke_zenoh.yaml's
// `transports.zenoh.connect:` block. Unlike the SOME/IP §9.6
// fixture, there is no SCE-namespaced `<machine>[_<partition>]_sce_app_`:
// Zenoh has no §13 OEM boundary equivalent that would require a
// separate session. Both endpoints share the device-wide
// `zenoh_session_` and the SCE-reserved §9.6 namespace is carved
// out via the `sce/scxml_invoke/...` key-expression prefix.
//
// Convergence: the relay session listens on the address both
// routers dial; once a liveliness round-trip via the relay
// completes, the peer mesh has propagated and §9.6 traffic can
// flow. Mirrors the `mesh_zenoh_runtime` motor↔brake handshake
// shape with one extra peer (worker router instead of a single
// brake router).

#include "common/TestScriptEngine.h"
#include "scxml_invoke_zenoh_parent_sm.h"
#include "scxml_invoke_zenoh_parent_transport.h"
#include "scxml_invoke_zenoh_worker_sm.h"
#include "scxml_invoke_zenoh_worker_transport.h"

#include "ZenohTestUtils.h"

#include <chrono>
#include <cstdio>
#include <thread>

int main() {
    using namespace SCE::Test::Mesh;

    // Mirrors deploy_scxml_invoke_zenoh.yaml ecu1.transports.zenoh.connect.
    // The relay session below binds this address so both routers'
    // generated init() reaches a listener via peer mesh routing.
    constexpr const char *kListen = "tcp/127.0.0.1:17448";

    // Test fixture relay listener. Multicast disabled in `open_peer`
    // so peer discovery happens deterministically through this
    // explicit endpoint — the same pattern `mesh_zenoh_runtime` uses
    // to anchor `motor_session ↔ brake_router` convergence.
    auto relay_session = open_peer(/*connect=*/"", /*listen=*/kListen);

    // Parent router opens its own zenoh::Session inside init();
    // the §9.6 endpoint emplace + start() runs there too, declaring
    // the Publisher on `sce/scxml_invoke/p2c/<parent>/<worker>` and
    // the Subscriber on `sce/scxml_invoke/c2p/<worker>/<parent>`.
    using ParentEngine = SCE::Generated::scxml_invoke_zenoh_parent::scxml_invoke_zenoh_parent;
    ParentEngine parent;
    SCE::Generated::scxml_invoke_zenoh_parent::TransportRouter<ParentEngine> parent_router({&parent});
    if (!parent_router.init()) {
        std::fprintf(stderr, "FAIL: parent_router.init() returned false\n");
        return 1;
    }

    using WorkerEngine = SCE::Generated::scxml_invoke_zenoh_worker::scxml_invoke_zenoh_worker;
    WorkerEngine worker;
    SCE::Test::inject_build_engine(worker);
    worker.initialize();
    SCE::Generated::scxml_invoke_zenoh_worker::TransportRouter<WorkerEngine> worker_router({&worker});
    if (!worker_router.init()) {
        std::fprintf(stderr, "FAIL: worker_router.init() returned false\n");
        return 1;
    }

    // Convergence barrier. A throwaway probe peer dials the relay
    // and exchanges a liveliness token with `relay_session`; once
    // observed, every pairwise edge sharing the relay listen
    // endpoint has converged — including parent_router ↔ relay
    // and worker_router ↔ relay (and through gossip-derived
    // direct links, parent_router ↔ worker_router).
    auto handshake_session = open_peer(/*connect=*/kListen, /*listen=*/"");
    wait_for_peer_ready(relay_session, handshake_session);

    // Brief subscriber-propagation window. `wait_for_peer_ready`
    // confirms peer-mesh routing has converged; the ScxmlInvokeEndpoint
    // Subscribers declared inside each router's init() still need
    // their key declarations to fan out to peers' publish-routing
    // tables. A 200 ms settle is well over observed convergence
    // (~50 ms locally) and matches the propagation delay
    // `test_mesh_zenoh_eventgroup_engine_driven` budgets after a
    // late `declare_subscriber`.
    std::this_thread::sleep_for(std::chrono::milliseconds(200));

    // Parent entering `waiting` emits wire-14 InvokeStart through
    // `scxml_invoke_to_<worker>_->send`. The worker's receive
    // callback stages the envelope; `worker_router.pumpScxmlInvokeRequests()`
    // below drains the queue, instantiates the child, observes
    // `isFinal()==true` at the end of `initialize()`, and publishes
    // wire-15 + wire-18 in a single tick.
    SCE::Test::inject_build_engine(parent);
    parent.initialize();

    using State = SCE::Generated::scxml_invoke_zenoh_parent::State;
    using clock = std::chrono::steady_clock;
    const auto deadline = clock::now() + std::chrono::seconds(10);
    while (clock::now() < deadline) {
        worker_router.pumpScxmlInvokeRequests();
        // pumpScxmlInvokeReplies is a no-op on the parent side —
        // the endpoint's receive callback dispatches wire-15/16/18/20
        // inline via `dispatchToSession` (§14.4 thread-safe contract
        // on the Zenoh runtime callback thread). The call is kept for
        // symmetry with shm/custom_tcp/someip §9.6 fixtures.
        parent_router.pumpScxmlInvokeReplies();
        parent.step();
        if (parent.getCurrentState() == State::Pass) {
            std::printf("SCE Mesh §9.6 Session 5 Zenoh scxml-invoke roundtrip: PASS\n");
            return 0;
        }
        if (parent.getCurrentState() == State::Fail) {
            std::fprintf(stderr, "FAIL: parent observed error.execution — the wire "
                                 "is present but the wire-15/18 success path did "
                                 "not complete over Zenoh.\n");
            return 1;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(20));
    }

    std::fprintf(stderr,
                 "FAIL: parent did not reach State::Pass within 10s. "
                 "Expected wire-14 InvokeStart → wire-15 InvokeStarted + "
                 "wire-18 InvokeDone → done.invoke.remote_inv → transition "
                 "to pass. Current parent state=%d\n",
                 static_cast<int>(parent.getCurrentState()));
    return 1;
}
