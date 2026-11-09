// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §9.6.3 + docs/adr/0090: the origin of an event between a parent and
// its remote child is an address a reply reaches, in both directions.
//
// The worker's child session sends `ping` to `#_parent`. At the parent the
// event comes from the SCXML Event I/O Processor at the child session's
// endpoint, `mesh://ecu1/worker_session_f_reply/<session>`, which the worker
// wrote in the wire-16 envelope's `reply_to`. The parent sends `pong` to that
// origin with the processor's type; its policy finds the running child that
// answers at that endpoint and the transport carries the event as wire-17. At
// the child `pong` comes from the parent session's endpoint, which the parent
// wrote in the wire-14 request; the child sends `ack` to that origin, its
// adapter recognises the parent's endpoint and the transport carries the event
// as wire-16. The parent passes when `ack` comes from the endpoint `ping` did.

#include "common/TestScriptEngine.h"
#include "parent_session_f_reply_sm.h"
#include "parent_session_f_reply_transport.h"
#include "worker_session_f_reply_sm.h"
#include "worker_session_f_reply_transport.h"

#include <chrono>
#include <cstdio>
#include <thread>

int main() {
    // Worker router first — its `c2p_to_parent_session_f_reply_` channel is
    // Mode::Create; the parent opens that name in its own ctor with lazy
    // reopen, as in mesh_session_f_eventful.
    using WorkerEngine = SCE::Generated::worker_session_f_reply::worker_session_f_reply;
    WorkerEngine worker;
    SCE::Test::inject_build_engine(worker);
    ::SCE::Test::start_or_abort(worker);
    SCE::Generated::worker_session_f_reply::TransportRouter<WorkerEngine> worker_router({&worker});

    using ParentEngine = SCE::Generated::parent_session_f_reply::parent_session_f_reply;
    ParentEngine parent;
    SCE::Generated::parent_session_f_reply::TransportRouter<ParentEngine> parent_router({&parent});

    SCE::Test::inject_build_engine(parent);
    ::SCE::Test::start_or_abort(parent);

    using State = SCE::Generated::parent_session_f_reply::State;
    using clock = std::chrono::steady_clock;
    const auto deadline = clock::now() + std::chrono::seconds(5);
    while (clock::now() < deadline) {
        // Worker: wire-14 spawns the child (its onentry sends `ping`), wire-17
        // hands it `pong`, and the tick lets it answer `ack`.
        worker_router.pumpScxmlInvokeRequests();
        // Parent: wire-15 records the child's endpoint, wire-16 brings `ping`
        // and then `ack`.
        parent_router.pumpScxmlInvokeReplies();
        parent.step();
        if (parent.terminalState() == State::Pass) {
            std::printf("SCE Mesh §9.6.3 reply round trip: PASS\n");
            return 0;
        }
        if (parent.terminalState() == State::Fail) {
            std::fprintf(stderr, "FAIL: the parent reached `fail`: `ping` did not come from the child "
                                 "session's endpoint, the child refused `pong` (it did not come from "
                                 "the parent's endpoint), `ack` came from another origin than `ping`, "
                                 "or a reply raised an error event (no session answered at the "
                                 "endpoint it was sent to).\n");
            return 1;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }

    std::fprintf(stderr,
                 "FAIL: parent did not reach State::Pass within 5s. Expected ping from the "
                 "child's endpoint, pong to it, ack back from the same endpoint. Current "
                 "parent state=%d\n",
                 static_cast<int>(parent.getCurrentState()));
    return 1;
}
