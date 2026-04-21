// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §9.6 Session F wire round-trip verification.
//
// Closes the silent-broken window declared in SCE_MESH.md §9.6 line 1396,
// but via the transport-present path added in Session F sub-item 2: the
// parent's state-entry calls `engine.performScxmlInvokeStart(...)`, the
// transport router emits a wire-14 `InvokeStart` envelope over the
// `/sce_inv14_<parent>_<worker>` shm channel, the worker router's
// `pumpScxmlInvokeRequests()` answers with wire-20 `InvokeError` carrying
// reason `SESSION_F_NOT_IMPLEMENTED`, and MeshDispatch translates that
// reply back into `error.execution` on the parent engine. The parent's
// `<transition event="error.execution" target="pass"/>` observes the raise
// and transitions to the final state — identical observable shape to the
// A0 local-raise scaffold (`mesh_session_f_not_implemented_verification`)
// but with the full wire path exercised end-to-end.
//
// Single-process test: parent and worker routers share the host process so
// the shm segments exchange data without fork. Startup order matters —
// worker's `wire20_to_parent_` is `Mode::Create`, parent's
// `wire20_from_parent_` is `Mode::Open`, so the worker router must be
// constructed before the parent's. The paired `wire14_*` directions use
// the symmetric discipline (parent creates, worker opens lazily via
// `pumpScxmlInvokeRequests`'s reopen branch).

#include "parent_session_f_wired_sm.h"
#include "parent_session_f_wired_transport.h"
#include "worker_session_f_wired_sm.h"
#include "worker_session_f_wired_transport.h"

#include <chrono>
#include <cstdio>
#include <thread>

int main() {
    // Worker first — its wire20_to_parent_ channel is Mode::Create, the
    // parent opens it in its own ctor. wire14_from_parent_ stays invalid
    // until the parent creates /sce_inv14_parent_worker; pumpScxmlInvokeRequests
    // lazy-reopens on its first call so the race is benign.
    using WorkerEngine = SCE::Generated::worker_session_f_wired::worker_session_f_wired;
    WorkerEngine worker;
    worker.initialize();
    SCE::Generated::worker_session_f_wired::TransportRouter<WorkerEngine> worker_router({&worker});

    using ParentEngine = SCE::Generated::parent_session_f_wired::parent_session_f_wired;
    ParentEngine parent;
    SCE::Generated::parent_session_f_wired::TransportRouter<ParentEngine> parent_router({&parent});

    // Parent ctor installed the wire-14 send callback. `initialize()`
    // enters `waiting`, the remote-invoke onentry block calls
    // `engine.performScxmlInvokeStart("worker_session_f_wired", ...)`,
    // which routes to `wire14_to_worker_session_f_wired_.send(env)`.
    parent.initialize();

    using State = SCE::Generated::parent_session_f_wired::State;
    using clock = std::chrono::steady_clock;
    const auto deadline = clock::now() + std::chrono::seconds(5);
    while (clock::now() < deadline) {
        // Worker drains inbound wire 14 and posts wire 20 responses inline.
        worker_router.pumpScxmlInvokeRequests();
        // Parent drains inbound wire 20; MeshDispatch translates each
        // envelope into an `error.execution` raise on the parent engine.
        parent_router.pumpScxmlInvokeReplies();
        // `step()` consumes the external queue we just populated so the
        // `<transition event="error.execution" target="pass"/>` fires.
        parent.step();
        if (parent.getCurrentState() == State::Pass) {
            std::printf("SCE Mesh §9.6 wire-14/20 round-trip verification: PASS\n");
            return 0;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }

    std::fprintf(stderr,
                 "FAIL: parent did not reach State::Pass within 5s. "
                 "Expected wire 14 InvokeStart to be emitted, worker pump to answer "
                 "with wire 20 InvokeError(SESSION_F_NOT_IMPLEMENTED), parent dispatch "
                 "to raise error.execution, and transition to observe it. "
                 "Current parent state=%d\n",
                 static_cast<int>(parent.getCurrentState()));
    return 1;
}
