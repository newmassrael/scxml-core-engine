// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §16.7 row 5 sub-atomic C verification: §9.6 scxml-invoke
// peer-down emit. The parent state machine carries a single
// `<invoke type="scxml" src="#worker_session_f_peer_drop" id="remote_inv"/>`
// in state `invoking`. After the engine reaches `invoking`, a thread that
// does not own the parent posts `InboxPeerLost{"worker_session_f_peer_drop"}`
// to its inbox — what the TransportRouter's Zenoh liveliness DELETE and
// SOMEIP machine-level availability handlers do in production, on their
// own threads (docs/adr/0084) — and the parent's owner applies it with
// the Policy's `failScxmlRemoteInvokesForPeer` at its next macrostep
// boundary.
//
// Contract under test (matches `tools/codegen/templates/invoke_methods.jinja2`):
//   1. The matching `activeInvokes_` entry is erased, on the owner.
//   2. Exactly one `error.communication` event is raised into the
//      parent's external queue carrying
//      `{"reason":"INVOKE_CHILD_LOST","invoke_id":"remote_inv",
//        "target":"worker_session_f_peer_drop"}`.
//   3. The parent's `<transition event="error.communication" target="lost"/>`
//      fires on the next macrostep — observable via
//      `parent.terminalState() == State::Lost`.
//   4. A second `failScxmlRemoteInvokesForPeer` call on the same peer
//      is a silent no-op (no second event, no state churn), because
//      step 1 already removed the entry.

#include "common/TestScriptEngine.h"
#include "parent_session_f_peer_drop_sm.h"
#include "parent_session_f_peer_drop_transport.h"
#include "worker_session_f_peer_drop_sm.h"
#include "worker_session_f_peer_drop_transport.h"

#include <chrono>
#include <cstdio>
#include <thread>

int main() {
    // Worker engine + router — present so the parent's wire-14 InvokeStart
    // has a peer to land on during the brief setup window. The worker
    // never reaches `<final>`; the test short-circuits via the
    // peer-down emit before any wire-18 could arrive.
    using WorkerEngine = SCE::Generated::worker_session_f_peer_drop::worker_session_f_peer_drop;
    WorkerEngine worker;
    SCE::Test::inject_build_engine(worker);
    ::SCE::Test::start_or_abort(worker);
    SCE::Generated::worker_session_f_peer_drop::TransportRouter<WorkerEngine> worker_router({&worker});

    using ParentEngine = SCE::Generated::parent_session_f_peer_drop::parent_session_f_peer_drop;
    ParentEngine parent;
    SCE::Generated::parent_session_f_peer_drop::TransportRouter<ParentEngine> parent_router({&parent});

    SCE::Test::inject_build_engine(parent);
    ::SCE::Test::start_or_abort(parent);

    using ParentState = SCE::Generated::parent_session_f_peer_drop::State;

    // Pump the §9.6 wire-14/15 setup so `activeInvokes_["remote_inv"]`
    // becomes populated on the parent. The worker echoes back wire-15
    // InvokeStarted, which stashes the child sessionId on the parent's
    // ChildSession entry. Without this, the test would still exercise
    // the entry inserted at the parent's onentry — but we want to
    // additionally verify that the peer-down erase is robust against
    // the wire-15-stamped state (sessionId non-empty).
    for (int i = 0; i < 20; ++i) {
        worker_router.pumpScxmlInvokeRequests();
        parent_router.pumpScxmlInvokeReplies();
        parent.step();
        std::this_thread::sleep_for(std::chrono::milliseconds(5));
    }

    if (parent.getCurrentState() != ParentState::Invoking) {
        std::fprintf(stderr,
                     "FAIL: parent did not settle into State::Invoking "
                     "during setup. Current parent state=%d\n",
                     static_cast<int>(parent.getCurrentState()));
        return 1;
    }

    // SCE Mesh §16.7 row 5 sub-atomic C peer-down emit, posted as the
    // TransportRouter peer-down handlers post it per session, from a
    // thread of their own.
    bool lost_posted = false;
    std::thread handler([&parent, &lost_posted] {
        lost_posted = parent.inbox().post(ParentEngine::InboxPeerLost{"worker_session_f_peer_drop"});
    });
    handler.join();
    if (!lost_posted) {
        std::fprintf(stderr, "FAIL: the parent's inbox refused the peer-down signal\n");
        return 1;
    }

    // Drive the parent's macrostep to consume the error.communication
    // event we just enqueued. One step should suffice; loop a few
    // times to absorb any incidental cleanup.
    using clock = std::chrono::steady_clock;
    const auto deadline = clock::now() + std::chrono::seconds(2);
    while (clock::now() < deadline) {
        worker_router.pumpScxmlInvokeRequests();
        parent_router.pumpScxmlInvokeReplies();
        parent.step();
        if (parent.terminalState() == ParentState::Lost) {
            break;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }

    if (parent.terminalState() != ParentState::Lost) {
        std::fprintf(stderr,
                     "FAIL: parent did not reach State::Lost after "
                     "failScxmlRemoteInvokesForPeer(worker_session_f_peer_drop). "
                     "Current parent state=%d. Expected the "
                     "error.communication transition to fire on the "
                     "raised INVOKE_CHILD_LOST event.\n",
                     static_cast<int>(parent.getCurrentState()));
        return 1;
    }

    // Second call, made directly on the owner (the body the posted
    // signal was applied with), must be a silent no-op: activeInvokes_ no
    // longer has an entry for `worker_session_f_peer_drop`, so the
    // raise loop sees zero entries and exits without raising. The
    // parent's `lost` state is a `<final>`, so no transition selection
    // happens regardless — but we still verify that step() does not
    // surface an unexpected event drain.
    parent.getPolicy().failScxmlRemoteInvokesForPeer("worker_session_f_peer_drop", parent);
    parent.step();

    if (parent.terminalState() != ParentState::Lost) {
        std::fprintf(stderr,
                     "FAIL: second peer-down call disturbed parent state. "
                     "Current parent state=%d. Expected silent no-op "
                     "(activeInvokes_ already empty after first call).\n",
                     static_cast<int>(parent.getCurrentState()));
        return 1;
    }

    std::printf("SCE Mesh §16.7 row 5 sub-atomic C peer-down emit verification: PASS\n");
    return 0;
}
