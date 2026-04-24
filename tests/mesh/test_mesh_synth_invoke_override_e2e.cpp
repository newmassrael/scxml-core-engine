// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §9.6.6 rule 3 — cross-partition inline <content> override,
// wire-14/15/18 round-trip. Companion to the §9.6.2 author-declared
// `mesh_session_f_wire_roundtrip`; this variant proves the same wire
// path resolves when the child machine is synthesized from the parent's
// inline `<content>` rather than declared by the author.
//
// The parent emits wire-14 `InvokeStart` on the
// `/sce_p2c_parent_synth_inline_parent_synth_inline__sce_synth_invoke__remote_inv`
// shm channel. The synth's `pumpScxmlInvokeRequests()` routes the
// envelope to its `WorkerSessionHost`, which instantiates an AOT child
// of the synth. Because the synth's initial state is `<final>`,
// `initialize()` immediately settles the child into final; the host
// observes `isFinal()==true` and emits wire-15 `InvokeStarted` followed
// by wire-18 `InvokeDone` on the
// `/sce_c2p_parent_synth_inline__sce_synth_invoke__remote_inv_parent_synth_inline`
// channel. The parent's `pumpScxmlInvokeReplies()` dispatches wire-15
// through `onInvokeStarted` (sessionId stash) and wire-18 through
// `onInvokeDone` (raises `done.invoke.remote_inv`); the parent's
// `<transition event="done.invoke.remote_inv" target="pass"/>` observes
// it and the machine reaches `pass`.
//
// Closes §16.9 F exit criterion #4: "Inline `<content>` synthesized
// child executes on a different partition."

#include "parent_synth_inline_sm.h"
#include "parent_synth_inline_transport.h"
#include "parent_synth_inline__sce_synth_invoke__remote_inv_sm.h"
#include "parent_synth_inline__sce_synth_invoke__remote_inv_transport.h"

#include <chrono>
#include <cstdio>
#include <thread>

int main() {
    // Synth first — its `c2p_to_parent_synth_inline_` channel is
    // Mode::Create; the parent opens that name in its own ctor. The
    // paired `p2c_from_parent_synth_inline_` uses lazy reopen so the
    // startup race is benign.
    using SynthEngine = SCE::Generated::parent_synth_inline__sce_synth_invoke__remote_inv::
        parent_synth_inline__sce_synth_invoke__remote_inv;
    SynthEngine synth;
    synth.initialize();
    SCE::Generated::parent_synth_inline__sce_synth_invoke__remote_inv::
        TransportRouter<SynthEngine> synth_router({&synth});

    using ParentEngine = SCE::Generated::parent_synth_inline::parent_synth_inline;
    ParentEngine parent;
    SCE::Generated::parent_synth_inline::TransportRouter<ParentEngine> parent_router({&parent});

    // Parent ctor installed the wire-14/17/19 send callbacks.
    // `initialize()` enters `waiting`, the remote-invoke onentry calls
    // `engine.performScxmlInvokeStart("parent_synth_inline__sce_synth_invoke__remote_inv", ...)`
    // which publishes wire-14 on the p2c channel.
    parent.initialize();

    using State = SCE::Generated::parent_synth_inline::State;
    using clock = std::chrono::steady_clock;
    const auto deadline = clock::now() + std::chrono::seconds(5);
    while (clock::now() < deadline) {
        // Synth drains inbound wire-14/17/19. wire-14 spawns a child
        // session; since the child reaches <final> during initialize(),
        // the host publishes wire-15 + wire-18 on the same tick.
        synth_router.pumpScxmlInvokeRequests();
        // Parent drains inbound wire-15/16/18/20; MeshDispatch routes
        // to onInvokeStarted / onInvokeDone / raiseExternal as needed.
        parent_router.pumpScxmlInvokeReplies();
        // Consume the parent's external queue — fires `done.invoke.*`.
        parent.step();
        if (parent.getCurrentState() == State::Pass) {
            std::printf("SCE Mesh §9.6.6 rule 3 cross-partition override: PASS\n");
            return 0;
        }
        if (parent.getCurrentState() == State::Fail) {
            std::fprintf(stderr,
                         "FAIL: parent observed error.execution instead of "
                         "done.invoke — synth override is wired but the "
                         "wire-15/18 success path did not complete.\n");
            return 1;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }

    std::fprintf(stderr,
                 "FAIL: parent did not reach State::Pass within 5s. "
                 "Expected wire-14 InvokeStart → wire-15 InvokeStarted + "
                 "wire-18 InvokeDone → done.invoke.remote_inv raise → "
                 "transition to pass. Current parent state=%d\n",
                 static_cast<int>(parent.getCurrentState()));
    return 1;
}
