// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §9.6.2 wire-16 ChildEvent + §9.6.4 <finalize> lifecycle.
//
// The worker's `ChildSessionAdapter` engine enters `announce`, whose
// onentry emits `<send target="#_parent" event="ping"/>`. In the new
// `is_remote_invoke_target` codegen shape this routes through
// `engine.performMeshSend("#_parent", ...)`; the adapter's callback
// intercepts and publishes a wire-16 `ChildEvent`. The parent drains
// wire-16 via `pumpScxmlInvokeReplies` → `MeshDispatch::ChildEvent`
// reconstructs `EventWithMetadata` with `_event.origin=child_session_id`.
//
// The parent is in the invoking state (`waiting`) and its `<invoke>`
// declares a `<finalize>` block that assigns `finalized=true`. Per
// W3C §6.4.4 the runtime runs `<finalize>` **before** evaluating
// transition selection for the triggering event, so the cond-guarded
// `<transition event="ping" cond="finalized == true" target="pass">`
// sees the flipped value and reaches `pass`. A subsequent wire-18
// `InvokeDone` would raise `done.invoke.remote_inv`, but the parent
// has already left `waiting` so the event falls on the floor — which
// is exactly the §9.6 cancel contract (no observable failure).

#include "parent_session_f_eventful_sm.h"
#include "parent_session_f_eventful_transport.h"
#include "worker_session_f_eventful_sm.h"
#include "worker_session_f_eventful_transport.h"

#include <chrono>
#include <cstdio>
#include <thread>

int main() {
    // Worker router first — its `c2p_to_parent_session_f_eventful_` channel
    // is Mode::Create; the parent opens that name in its own ctor with lazy
    // reopen so the startup race is benign (same pattern as session_f_wired).
    using WorkerEngine = SCE::Generated::worker_session_f_eventful::worker_session_f_eventful;
    WorkerEngine worker;
    worker.initialize();
    SCE::Generated::worker_session_f_eventful::TransportRouter<WorkerEngine> worker_router({&worker});

    using ParentEngine = SCE::Generated::parent_session_f_eventful::parent_session_f_eventful;
    ParentEngine parent;
    SCE::Generated::parent_session_f_eventful::TransportRouter<ParentEngine> parent_router({&parent});

    // Parent ctor installed wire-14/17/19 send callbacks and the Lua
    // script engine initialiser runs on the first datamodel touch.
    parent.initialize();

    using State = SCE::Generated::parent_session_f_eventful::State;
    using clock = std::chrono::steady_clock;
    const auto deadline = clock::now() + std::chrono::seconds(5);
    while (clock::now() < deadline) {
        // Worker drains inbound wire-14/17/19 and ticks active child
        // sessions. wire-14 spawns the child; the child's onentry emits
        // a wire-16 `ping` through the mesh callback, then the eventless
        // transition advances to `done` and the next host tick emits
        // wire-18 `InvokeDone`.
        worker_router.pumpScxmlInvokeRequests();
        // Parent drains inbound wire-15/16/18/20; MeshDispatch routes
        // ChildEvent to raiseExternal, firing `<finalize>` then the
        // cond-guarded transition on the next macrostep.
        parent_router.pumpScxmlInvokeReplies();
        parent.step();
        if (parent.getCurrentState() == State::Pass) {
            std::printf("SCE Mesh §9.6 eventful+finalize verification: PASS\n");
            return 0;
        }
        if (parent.getCurrentState() == State::Fail) {
            std::fprintf(stderr,
                         "FAIL: parent observed `ping` but either (a) `<finalize>` "
                         "did not run before transition selection so the cond "
                         "evaluated against the initial `finalized=false`, or "
                         "(b) `error.execution` fired (wire-20 / script engine "
                         "error). Inspect the second transition's path for the "
                         "exact ordering breach.\n");
            return 1;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }

    std::fprintf(stderr,
                 "FAIL: parent did not reach State::Pass within 5s. Expected "
                 "wire-16 ping → <finalize> sets finalized=true → cond "
                 "transition to pass. Current parent state=%d\n",
                 static_cast<int>(parent.getCurrentState()));
    return 1;
}
