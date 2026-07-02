// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §9.6.5 autoforward="true" verification (wire-17 ParentEvent).
//
// The parent declares `<invoke autoforward="true">` on the worker.
// During the parent's macrostep, every external event must be replayed
// onto the child via wire-17. The test driver injects a `trigger`
// external event; the parent's `forwardToAutoforwardChildren` routes
// through `engine.performScxmlInvokeParentEvent(peer, invokeId, "trigger",
// data, sendId)` — which publishes a wire-17 envelope on the
// `/sce_p2c_<parent>_<worker>` channel.
//
// The worker's inbound pump dispatches wire-17 to
// `WorkerSessionHost::onWire17`, which locates the child adapter by
// `env.subject` and calls `adapter->raiseExternal("trigger", data, "")`.
// The child's `<transition event="trigger" target="done"/>` fires;
// the next host tick observes `isFinal()` and emits wire-18 `InvokeDone`.
// The parent's `MeshDispatch` raises `done.invoke.remote_inv`, the
// pass transition fires, and we reach `State::Pass`.

#include "common/TestScriptEngine.h"
#include "parent_session_f_autoforward_sm.h"
#include "parent_session_f_autoforward_transport.h"
#include "worker_session_f_autoforward_sm.h"
#include "worker_session_f_autoforward_transport.h"

#include <chrono>
#include <cstdio>
#include <thread>

int main() {
    using WorkerEngine = SCE::Generated::worker_session_f_autoforward::worker_session_f_autoforward;
    WorkerEngine worker;
    SCE::Test::inject_build_engine(worker);
    worker.initialize();
    SCE::Generated::worker_session_f_autoforward::TransportRouter<WorkerEngine> worker_router({&worker});

    using ParentEngine = SCE::Generated::parent_session_f_autoforward::parent_session_f_autoforward;
    ParentEngine parent;
    SCE::Generated::parent_session_f_autoforward::TransportRouter<ParentEngine> parent_router({&parent});

    SCE::Test::inject_build_engine(parent);
    parent.initialize();

    // The parent is now in `waiting` and has emitted wire-14 `InvokeStart`.
    // Pump a handful of iterations to give the worker time to receive
    // wire-14, spawn the adapter, and publish wire-15 back to the parent
    // (which stashes the child session id for wire-17 routing).
    using ParentState = SCE::Generated::parent_session_f_autoforward::State;
    using ParentEvent = SCE::Generated::parent_session_f_autoforward::Event;
    using clock = std::chrono::steady_clock;

    for (int i = 0; i < 20; ++i) {
        worker_router.pumpScxmlInvokeRequests();
        parent_router.pumpScxmlInvokeReplies();
        parent.step();
        std::this_thread::sleep_for(std::chrono::milliseconds(5));
    }

    // Inject the autoforward trigger. The parent's macrostep selects the
    // targetless `<transition event="trigger"/>` AND the autoforward
    // closure fires (W3C §6.4.6 — both run, autoforward does not consume
    // the event).
    parent.raiseExternal(ParentEngine::EventWithMetadata(ParentEvent::Trigger));

    const auto deadline = clock::now() + std::chrono::seconds(5);
    while (clock::now() < deadline) {
        worker_router.pumpScxmlInvokeRequests();
        parent_router.pumpScxmlInvokeReplies();
        parent.step();
        if (parent.getCurrentState() == ParentState::Pass) {
            std::printf("SCE Mesh §9.6.5 autoforward verification: PASS\n");
            return 0;
        }
        if (parent.getCurrentState() == ParentState::Fail) {
            std::fprintf(stderr, "FAIL: parent reached `fail`. Expected autoforward "
                                 "to forward `trigger` to child → child reaches "
                                 "`done` → wire-18 → done.invoke.* → pass. Likely "
                                 "`error.execution` fired (wire-20 or forward "
                                 "callback uninstalled).\n");
            return 1;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }

    std::fprintf(stderr,
                 "FAIL: parent did not reach State::Pass within 5s. "
                 "Current parent state=%d\n",
                 static_cast<int>(parent.getCurrentState()));
    return 1;
}
