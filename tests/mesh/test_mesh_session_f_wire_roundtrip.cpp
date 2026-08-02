// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE-VERIFIES: mesh-9.6
//
// SCE Mesh §9.6 Session F full lifecycle verification.
//
// The parent's state-entry emits wire-14 `InvokeStart` via the
// `/sce_p2c_<parent>_<worker>` shm channel. The worker's
// `pumpScxmlInvokeRequests()` routes the envelope to the
// `WorkerSessionHost`, which instantiates an AOT child of
// `worker_session_f_wired`. Because the child's initial state is a
// `<final>`, `initialize()` immediately settles the child into final;
// the host observes `isFinal()==true` and emits wire-15 `InvokeStarted`
// followed by wire-18 `InvokeDone` on the `/sce_c2p_<worker>_<parent>`
// channel. The parent's `pumpScxmlInvokeReplies()` dispatches wire-15
// through `onInvokeStarted` (sessionId stash) and wire-18 through
// `onInvokeDone` (raises `done.invoke.remote_inv`); the parent's
// `<transition event="done.invoke.*" target="pass"/>` observes it and
// the machine reaches `pass` — the shortest success path through the
// §9.6 parent/child lifecycle exercised end-to-end.

#include "common/TestScriptEngine.h"
#include "parent_session_f_wired_sm.h"
#include "parent_session_f_wired_transport.h"
#include "worker_session_f_wired_sm.h"
#include "worker_session_f_wired_transport.h"

#include <chrono>
#include <cstdio>
#include <thread>

int main() {
    // Worker first — its `c2p_to_parent_session_f_wired_` channel is
    // Mode::Create; the parent opens that name in its own ctor. The
    // paired `p2c_from_parent_session_f_wired_` uses lazy reopen so the
    // startup race is benign.
    using WorkerEngine = SCE::Generated::worker_session_f_wired::worker_session_f_wired;
    WorkerEngine worker;
    SCE::Test::inject_build_engine(worker);
    worker.initialize();
    SCE::Generated::worker_session_f_wired::TransportRouter<WorkerEngine> worker_router({&worker});

    using ParentEngine = SCE::Generated::parent_session_f_wired::parent_session_f_wired;
    ParentEngine parent;
    SCE::Generated::parent_session_f_wired::TransportRouter<ParentEngine> parent_router({&parent});

    // Parent ctor installed the wire-14/17/19 send callbacks. `initialize()`
    // enters `waiting`, the remote-invoke onentry calls
    // `engine.performScxmlInvokeStart("worker_session_f_wired", ...)`
    // which publishes wire-14 on the `p2c_to_worker_session_f_wired_` channel.
    SCE::Test::inject_build_engine(parent);
    parent.initialize();

    using State = SCE::Generated::parent_session_f_wired::State;
    using clock = std::chrono::steady_clock;
    const auto deadline = clock::now() + std::chrono::seconds(5);
    while (clock::now() < deadline) {
        // Worker drains inbound wire-14/17/19. wire-14 spawns a child
        // session; since the child reaches <final> during initialize(),
        // the host publishes wire-15 + wire-18 on the same tick.
        worker_router.pumpScxmlInvokeRequests();
        // Parent drains inbound wire-15/16/18/20; MeshDispatch routes
        // to onInvokeStarted / onInvokeDone / raiseExternal as needed.
        parent_router.pumpScxmlInvokeReplies();
        // Consume the parent's external queue — fires `done.invoke.*`.
        parent.step();
        if (parent.getCurrentState() == State::Pass) {
            std::printf("SCE Mesh §9.6 full lifecycle verification: PASS\n");
            return 0;
        }
        if (parent.getCurrentState() == State::Fail) {
            std::fprintf(stderr, "FAIL: parent observed error.execution instead of "
                                 "done.invoke — transport is wired but the wire-15/18 "
                                 "success path did not complete.\n");
            return 1;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }

    std::fprintf(stderr,
                 "FAIL: parent did not reach State::Pass within 5s. "
                 "Expected wire-14 InvokeStart → wire-15 InvokeStarted + wire-18 "
                 "InvokeDone → done.invoke.remote_inv raise → transition to pass. "
                 "Current parent state=%d\n",
                 static_cast<int>(parent.getCurrentState()));
    return 1;
}
