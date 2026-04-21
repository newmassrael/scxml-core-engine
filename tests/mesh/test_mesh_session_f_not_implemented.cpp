// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §9.6 Session F scaffold — verifies the silent-broken window
// declared in SCE_MESH.md §9.6 line 1396 is closed.
//
// Spec contract: "documents using full remote invoke parse cleanly; at
// runtime they raise `error.execution` with `_event.data.reason ==
// 'SESSION_F_NOT_IMPLEMENTED'` until Session F lands."
//
// Parent `parent_session_f.scxml` declares
//   <invoke type="scxml" src="#worker_session_f"/>
// pointing at a distinct mesh machine declared in
// `deploy_session_f.yaml`. The classifier in
// `inject_partition_context_for` sets `ScxmlInvokeInfo.remote_mesh_target =
// Some("worker_session_f")`; C++ codegen emits a §10.7.1 error.execution
// raise at state-entry whose text names SESSION_F_NOT_IMPLEMENTED. The
// parent's `<transition event="error.execution" target="pass"/>` observes
// the raise and moves to a <final> state, so a successful initialize()
// proves the full chain: classifier → template filter → raise → internal
// event queue → transition → final state.
//
// No transport, no wire traffic. The raise is local to the parent
// session; Session F sub-landings can grow this test to assert against
// structured `_event.data` once the convention is wired into the engine
// (it is currently embedded in the raise text per the same pattern used
// by `INVOKE_SRC_NOT_FOUND` in §9.5).

#include "parent_session_f_sm.h"

#include <cstdio>

int main() {
    SCE::Generated::parent_session_f::parent_session_f sm;
    sm.initialize();

    using State = SCE::Generated::parent_session_f::State;
    const auto state = sm.getCurrentState();
    if (state != State::Pass) {
        std::fprintf(stderr,
                     "FAIL: parent did not reach 'pass' after initialize(); state=%d. "
                     "Expected: remote <invoke type=\"scxml\"> raises error.execution "
                     "with reason SESSION_F_NOT_IMPLEMENTED per SCE_MESH.md §9.6/§10.7.1, "
                     "parent transitions to final.\n",
                     static_cast<int>(state));
        return 1;
    }

    std::printf("SCE Mesh §9.6 SESSION_F_NOT_IMPLEMENTED verification: PASS\n");
    return 0;
}
