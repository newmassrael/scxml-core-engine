// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §9.6 Session 4b — SOME/IP scxml-invoke single-process E2E.
//
// Single-process proof that the §9.6 wire-14/20 lifecycle travels
// over SOME/IP:
//   wire-14 InvokeStart  (parent → worker, method 0x0014)
//   wire-15 InvokeStarted (worker → parent, method 0x0015)
//   wire-18 InvokeDone    (worker → parent, method 0x0018)
//
// Both TransportRouters run in one process. Each instantiates the
// consolidated `<machine>[_<partition>]_sce_app_` vsomeip application
// (RFC F.X-2) distinct from any per-`<send>`-target application
// (§13 OEM boundary). vsomeip.json names parent's app as the routing
// manager, so the worker's client application attaches through it; the
// ctor order below initialises parent fully before worker connects.
//
// VSOMEIP_CONFIGURATION points to vsomeip_scxml_invoke.json (SD
// disabled, dedicated `network: sce_scxml_invoke_someip` socket
// namespace so parallel ctest does not race sibling mesh_someip_*
// fixtures).

#include "common/TestScriptEngine.h"
#include "scxml_invoke_someip_parent_sm.h"
#include "scxml_invoke_someip_parent_transport.h"
#include "scxml_invoke_someip_worker_sm.h"
#include "scxml_invoke_someip_worker_transport.h"

#include "SomeipTestUtils.h"

#include <chrono>
#include <cstdio>
#include <thread>

int main() {
    SCE::Test::Mesh::wipe_stale_vsomeip_sockets();

    // Parent carries the routing manager identity
    // (`scxml_invoke_someip_parent_sce`, per
    // vsomeip_scxml_invoke.json's `routing` field). Its app must be
    // inited and its dispatch thread started before worker's
    // `create_application` attaches — mirrors the mesh_someip_runtime
    // ordering where motor (routing manager) initialised first.
    using ParentEngine = SCE::Generated::scxml_invoke_someip_parent::scxml_invoke_someip_parent;
    ParentEngine parent;
    SCE::Generated::scxml_invoke_someip_parent::TransportRouter<ParentEngine> parent_router({&parent});
    if (!parent_router.init()) {
        std::fprintf(stderr, "FAIL: parent_router.init() returned false\n");
        return 1;
    }

    using WorkerEngine = SCE::Generated::scxml_invoke_someip_worker::scxml_invoke_someip_worker;
    WorkerEngine worker;
    SCE::Test::inject_build_engine(worker);
    worker.initialize();
    SCE::Generated::scxml_invoke_someip_worker::TransportRouter<WorkerEngine> worker_router({&worker});
    if (!worker_router.init()) {
        std::fprintf(stderr, "FAIL: worker_router.init() returned false\n");
        return 1;
    }

    // Brief settling window for the routing manager to link both
    // offered services and both requested services. ScxmlInvokeEndpoint
    // does not surface availability callbacks (by design — the
    // lifecycle is caller-driven, not event-driven), so the test uses
    // the deterministic polling loop below to cover the handshake
    // delay. mesh_someip_runtime uses `register_availability_handler`
    // for the same purpose on a <send>-target pattern that does expose
    // availability; the §9.6 endpoint does not, and a fixed sleep +
    // retry pattern matches how production <invoke> lifecycles behave
    // (no synchronous availability coupling with the invoke call).
    std::this_thread::sleep_for(std::chrono::milliseconds(500));

    // Parent entering `waiting` emits wire-14 InvokeStart through
    // `scxml_invoke_to_<worker>_.send`. The worker's receive callback
    // stages the envelope; `worker_router.pumpScxmlInvokeRequests()`
    // below drains the queue, instantiates the child, observes
    // `isFinal()==true` at the end of `initialize()`, and publishes
    // wire-15 + wire-18 in a single tick.
    SCE::Test::inject_build_engine(parent);
    parent.initialize();

    using State = SCE::Generated::scxml_invoke_someip_parent::State;
    using clock = std::chrono::steady_clock;
    const auto deadline = clock::now() + std::chrono::seconds(10);
    while (clock::now() < deadline) {
        worker_router.pumpScxmlInvokeRequests();
        // pumpScxmlInvokeReplies is a no-op on the parent side — the
        // endpoint's receive callback dispatches wire-15/16/18/20
        // inline via `dispatchToSession` (§14.4 thread-safe contract).
        // The call is kept for symmetry with shm/custom_tcp fixtures.
        parent_router.pumpScxmlInvokeReplies();
        parent.step();
        if (parent.getCurrentState() == State::Pass) {
            std::printf("SCE Mesh §9.6 Session 4b SOME/IP scxml-invoke roundtrip: PASS\n");
            return 0;
        }
        if (parent.getCurrentState() == State::Fail) {
            std::fprintf(stderr, "FAIL: parent observed error.execution — the wire "
                                 "is present but the wire-15/18 success path did "
                                 "not complete over SOME/IP.\n");
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
