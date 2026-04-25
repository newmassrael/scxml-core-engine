// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §9.6 Session 5c — Zenoh donedata two-host roundtrip (parent).
//
// Sister to test_mesh_zenoh_scxml_invoke_crossdev_parent.cpp (Session 5b
// lifecycle). The parent SCXML (parent_session_f_donedata.scxml) is reused
// verbatim from the shm + custom_tcp baselines — only the deploy.yaml
// rebinds `<invoke>` peer references onto Zenoh, so reaching State::Pass
// here proves that the same `<donedata>` semantics survive a real netns
// + veth + Zenoh TCP round-trip. The three sequential conds embedded in
// the parent SCXML (param.result == 42 → content == 'hello_content' →
// nested object/array/integer) guard the decoded `_event.data` payload;
// any regression in donedata serialisation, Zenoh CBOR framing, or
// EventDataHelper hydration takes the `fail` transition before the
// 15 s deadline.
//
// Listen anchor: deploy.yaml binds parent ECU's zenoh transport to
// tcp/172.16.10.1:17450. The 1-process Session 5 / Session 4b fixtures
// open a separate relay session via open_peer / wait_for_peer_ready;
// here the parent's own listen socket IS the rendezvous point and the
// worker's connect-retry handles a worker-comes-up-first race.

#include "parent_session_f_donedata_sm.h"
#include "parent_session_f_donedata_transport.h"

#include <chrono>
#include <cstdio>
#include <thread>

int main() {
    using ParentEngine =
        SCE::Generated::parent_session_f_donedata::parent_session_f_donedata;
    ParentEngine parent;
    SCE::Generated::parent_session_f_donedata::TransportRouter<ParentEngine>
        parent_router({&parent});
    if (!parent_router.init()) {
        std::fprintf(stderr, "FAIL: parent_router.init() returned false\n");
        return 1;
    }

    // Subscriber-propagation window. The orchestrator's 500 ms covers the
    // worker's pending connect to 172.16.10.1:17450 (which only resolves
    // once *this* listen socket binds, immediately above). This sleep
    // gives the three worker peer sessions time to (a) finish the TCP
    // dial and (b) propagate their `sce/scxml_invoke/c2p/...` Subscriber
    // declarations to this peer's publish-routing table for ALL THREE
    // workers, so the parent SCXML's first <invoke> wire-14 has a route
    // when the entry action fires. Single-worker 5b lifecycle uses the
    // same 500 ms; three workers ride the same gossip propagation, so
    // no extra budget is needed here (Zenoh declarations propagate in
    // O(1) per peer, not O(N) per declaration).
    std::this_thread::sleep_for(std::chrono::milliseconds(500));

    parent.initialize();

    using State = SCE::Generated::parent_session_f_donedata::State;
    using clock = std::chrono::steady_clock;
    const auto deadline = clock::now() + std::chrono::seconds(15);
    while (clock::now() < deadline) {
        // pumpScxmlInvokeReplies is a no-op on the parent — the
        // ZenohScxmlInvokeEndpoint's receive callback dispatches
        // wire-15/16/18/20 inline via dispatchToSession (§14.4
        // thread-safe contract on the Zenoh runtime callback thread).
        // Kept for symmetry with the 1-process fixture.
        parent_router.pumpScxmlInvokeReplies();
        parent.step();
        if (parent.getCurrentState() == State::Pass) {
            std::printf(
                "SCE Mesh §9.6 Session 5c Zenoh donedata crossdev: PASS\n");
            return 0;
        }
        if (parent.getCurrentState() == State::Fail) {
            std::fprintf(stderr,
                         "FAIL: parent took the fail transition. One of the "
                         "three donedata conds did not match the wire-18 "
                         "payload over Zenoh — either the param shape lost "
                         "the name/value pair, the content shape dropped the "
                         "primitive string, or the nested shape regressed "
                         "the canonical-JSON contract (object + array + "
                         "integer).\n");
            return 1;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(20));
    }

    std::fprintf(stderr,
                 "FAIL: parent did not reach State::Pass within 15 s. "
                 "Either Zenoh subscriber declarations never converged "
                 "across the three workers (check `ss -t` inside parent "
                 "netns) or wire-15/18 replies for the current phase did "
                 "not return. Current parent state=%d\n",
                 static_cast<int>(parent.getCurrentState()));
    return 1;
}
