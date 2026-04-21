// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §9.6 wire-19 InvokeCancel verification.
//
// The parent is in `waiting` with an active `<invoke>` that has spawned
// a child session on the worker. The child is in `running` — no final,
// no outbound events. Injecting a `stop` event drives the parent
// through `<transition target="cancelled">`; the waiting state's
// `onexit` emits wire-19 `InvokeCancel`. On the worker side,
// `WorkerSessionHost::onWire19` invokes `adapter->cancel()` and erases
// the session. The parent reaches `cancelled` without ever seeing
// `error.execution`, which is the observable success signal.

#include "parent_session_f_cancel_sm.h"
#include "parent_session_f_cancel_transport.h"
#include "worker_session_f_cancel_sm.h"
#include "worker_session_f_cancel_transport.h"

#include <chrono>
#include <cstdio>
#include <thread>

int main() {
    using WorkerEngine = SCE::Generated::worker_session_f_cancel::worker_session_f_cancel;
    WorkerEngine worker;
    worker.initialize();
    SCE::Generated::worker_session_f_cancel::TransportRouter<WorkerEngine> worker_router({&worker});

    using ParentEngine = SCE::Generated::parent_session_f_cancel::parent_session_f_cancel;
    ParentEngine parent;
    SCE::Generated::parent_session_f_cancel::TransportRouter<ParentEngine> parent_router({&parent});

    parent.initialize();

    using ParentState = SCE::Generated::parent_session_f_cancel::State;
    using ParentEvent = SCE::Generated::parent_session_f_cancel::Event;
    using clock = std::chrono::steady_clock;

    // Settle handshake: wire-14 → worker spawns adapter, adapter in
    // `running`, publishes wire-15 back. Parent stashes child session id.
    for (int i = 0; i < 20; ++i) {
        worker_router.pumpScxmlInvokeRequests();
        parent_router.pumpScxmlInvokeReplies();
        parent.step();
        std::this_thread::sleep_for(std::chrono::milliseconds(5));
    }

    // Drive the cancel by injecting `stop`.
    parent.raiseExternal(ParentEngine::EventWithMetadata(ParentEvent::Stop));

    const auto deadline = clock::now() + std::chrono::seconds(5);
    while (clock::now() < deadline) {
        worker_router.pumpScxmlInvokeRequests();
        parent_router.pumpScxmlInvokeReplies();
        parent.step();
        if (parent.getCurrentState() == ParentState::Cancelled) {
            std::printf("SCE Mesh §9.6 wire-19 cancel verification: PASS\n");
            return 0;
        }
        if (parent.getCurrentState() == ParentState::Fail) {
            std::fprintf(stderr,
                         "FAIL: parent hit `error.execution` before reaching "
                         "`cancelled`. Likely the onexit wire-19 emit raised an "
                         "error path back onto the parent (check performScxmlInvokeCancel "
                         "callback wiring).\n");
            return 1;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }

    std::fprintf(stderr,
                 "FAIL: parent did not reach State::Cancelled within 5s. "
                 "Current parent state=%d\n",
                 static_cast<int>(parent.getCurrentState()));
    return 1;
}
