// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §9.6 Session F transport-absent fallback — verifies the
// local-raise path declared in SCE_MESH.md §9.6 line 1396 still fires
// when no mesh transport binding exists between parent and peer.
//
// Spec contract: "Absent the transport binding the parent raises locally
// without wire traffic." The reason carried is
// `SESSION_F_TRANSPORT_UNAVAILABLE` (§10.7.1).
//
// Parent `parent_session_f.scxml` declares
//   <invoke type="scxml" src="#worker_session_f"/>
// pointing at a distinct mesh machine declared in
// `deploy_session_f.yaml`. The classifier in
// `inject_partition_context_for` sets `ScxmlInvokeInfo.remote_mesh_target =
// Some("worker_session_f")`; because this deploy file does NOT attach
// any transport binding between the two peers, the generated
// `performScxmlInvokeStart` returns false and the parent raises
// `error.execution` locally per §9.6 L1396. The parent's
// `<transition event="error.execution" target="pass"/>` observes the raise
// and moves to a <final> state, so a successful initialize() proves the
// fallback chain: classifier → template filter → raise → internal event
// queue → transition → final state.

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
                     "Expected: remote <invoke type=\"scxml\"> with no transport binding "
                     "raises error.execution locally (SESSION_F_TRANSPORT_UNAVAILABLE per "
                     "SCE_MESH.md §9.6/§10.7.1), parent transitions to final.\n",
                     static_cast<int>(state));
        return 1;
    }

    std::printf("SCE Mesh §9.6 transport-absent fallback verification: PASS\n");
    return 0;
}
