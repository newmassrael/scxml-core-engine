// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §9.6.2 wire-18 donedata surfacing — W3C SCXML 5.5 / 6.3.1.
//
// The parent sequentially invokes two workers:
//
//   1. `worker_session_f_donedata_param` reaches a top-level `<final>`
//      whose `<donedata>` is a single `<param name="result" expr="42"/>`.
//      `entry_exit_actions.jinja2` evaluates it via `DoneDataHelper::
//      evaluateParams`, stashes the JSON object on the engine through
//      `StaticExecutionEngine::stashDonedataAtFinal`. The worker-side
//      `WorkerSessionHost::emitWire18` reads `ChildSessionAdapter::
//      getDonedata()` (which forwards to `engine_.donedataAtFinal()`),
//      fills `env.data` and publishes. The parent's `onInvokeDone`
//      threads the bytes into `createDoneInvokeEvent(..., data)`, and
//      the parent's `setPolicyMetadata` re-hydrates `_event.data` via
//      `EventDataHelper::jsonStringToScriptValue` so the cond
//      `_event.data.result == 42` evaluates against a typed Lua table.
//
//   2. `worker_session_f_donedata_content` reaches a top-level `<final>`
//      whose `<donedata>` is `<content expr="'hello_content'"/>`. The
//      `evaluateContent` branch produces a JSON string; the round-trip
//      yields a plain Lua string on the parent side and the cond
//      `_event.data == 'hello_content'` passes.
//
// Any fall-through done.invoke takes a deterministic `fail` transition
// so regressions surface immediately rather than as a 5s timeout. The
// fixture closes the c06fae17 deferral (IChildSession::getDonedata()
// returned empty) and the f3aa83ba (void)donedata marker in the
// wire-18 receive branch.

#include "parent_session_f_donedata_sm.h"
#include "parent_session_f_donedata_transport.h"
#include "worker_session_f_donedata_param_sm.h"
#include "worker_session_f_donedata_param_transport.h"
#include "worker_session_f_donedata_content_sm.h"
#include "worker_session_f_donedata_content_transport.h"

#include <chrono>
#include <cstdio>
#include <thread>

int main() {
    // Workers first — each opens its c2p channel in Mode::Create so the
    // parent's Mode::OpenOrCreate observes the arena already present. Same
    // startup-race handling as the other §9.6 fixtures.
    using ParamWorker = SCE::Generated::worker_session_f_donedata_param::worker_session_f_donedata_param;
    ParamWorker worker_param;
    worker_param.initialize();
    SCE::Generated::worker_session_f_donedata_param::TransportRouter<ParamWorker>
        worker_param_router({&worker_param});

    using ContentWorker = SCE::Generated::worker_session_f_donedata_content::worker_session_f_donedata_content;
    ContentWorker worker_content;
    worker_content.initialize();
    SCE::Generated::worker_session_f_donedata_content::TransportRouter<ContentWorker>
        worker_content_router({&worker_content});

    using ParentEngine = SCE::Generated::parent_session_f_donedata::parent_session_f_donedata;
    ParentEngine parent;
    SCE::Generated::parent_session_f_donedata::TransportRouter<ParentEngine> parent_router({&parent});
    parent.initialize();

    using ParentState = SCE::Generated::parent_session_f_donedata::State;
    using clock = std::chrono::steady_clock;
    const auto deadline = clock::now() + std::chrono::seconds(5);
    while (clock::now() < deadline) {
        worker_param_router.pumpScxmlInvokeRequests();
        worker_content_router.pumpScxmlInvokeRequests();
        parent_router.pumpScxmlInvokeReplies();
        parent.step();
        if (parent.getCurrentState() == ParentState::Pass) {
            std::printf("SCE Mesh §9.6.2 wire-18 donedata verification: PASS\n");
            return 0;
        }
        if (parent.getCurrentState() == ParentState::Fail) {
            std::fprintf(stderr,
                         "FAIL: parent reached `fail`. Check the cond "
                         "`_event.data.result == 42` (param branch) or "
                         "`_event.data == 'hello_content'` (content "
                         "branch). Empty `_event.data` means the "
                         "stash / getDonedata / emitWire18 / onInvokeDone "
                         "chain dropped the payload.\n");
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
