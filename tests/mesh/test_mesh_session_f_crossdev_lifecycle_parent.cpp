// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §9.6 Session 3 cross-device lifecycle — parent half.
//
// Consumes the worker's announced ephemeral endpoint (from the
// `MESH_PEER_ENDPOINT` env var that the orchestrator populated by
// reading `LISTEN_ENDPOINT=` off the worker's stderr) and threads it
// into `TransportRouter::init(PortOverride)` so the
// `p2c_to_worker_session_f_wired_` Client dials the live ephemeral
// port instead of the deploy.yaml `"127.0.0.1:0"` placeholder. This is
// the first TransportRouter-level consumer of the PortOverride path
// added in Stage A3; Stage A4's smoke exercised the bare
// `CustomTcp::Client::set_connect_endpoint` API without a router.
//
// The parent's own Server binds on a static CMake-configurable port
// (`SCE_TEST_CROSSDEV_LIFECYCLE_PORT`) so the worker's codegen-baked
// `c2p_to_parent_session_f_wired_` connect endpoint is known at build
// time — no reverse handshake needed.
//
// Observable success: parent reaches `State::Pass`, which the
// parent_session_f_wired.scxml drives via `<transition
// event="done.invoke.*" target="pass"/>` — so the full wire-14
// `InvokeStart` → wire-15 `InvokeStarted` → wire-18 `InvokeDone`
// round-trip landed across the TCP boundary. Failure modes include a
// reached `State::Fail` (parent saw `error.execution`, usually
// meaning the worker-side handshake crashed or the override endpoint
// was wrong) and the 5s timeout (parent never observed
// `done.invoke.*`, likely a framing or dispatch regression).

#include "common/TestScriptEngine.h"
#include "parent_session_f_wired_sm.h"
#include "parent_session_f_wired_transport.h"

#include "mesh/transports/CustomTcpTransport.h"

#include <chrono>
#include <cstdlib>
#include <gtest/gtest.h>
#include <thread>

TEST(CrossdevLifecycle, WireDoneRoundTripLandsOverCustomTcp) {
    const char *worker_ep = std::getenv("MESH_PEER_ENDPOINT");
    ASSERT_NE(worker_ep, nullptr) << "MESH_PEER_ENDPOINT must be set by run_two_process_fixture.sh; "
                                     "running this binary directly without the orchestrator is not "
                                     "supported.";
    ASSERT_NE(*worker_ep, '\0') << "MESH_PEER_ENDPOINT was empty";

    using ParentEngine = SCE::Generated::parent_session_f_wired::parent_session_f_wired;
    using ParentRouter = SCE::Generated::parent_session_f_wired::TransportRouter<ParentEngine>;
    using ParentState = SCE::Generated::parent_session_f_wired::State;

    ParentEngine parent;
    ParentRouter router({&parent});

    SCE::Mesh::CustomTcp::PortOverride port_override;
    port_override.peer_connect_endpoints["worker_session_f_wired"] = worker_ep;
    ASSERT_TRUE(router.init(port_override)) << "parent TransportRouter::init(PortOverride) failed — usually a "
                                               "bind collision on the static SCE_TEST_CROSSDEV_LIFECYCLE_PORT "
                                               "(another ctest entry holding the RESOURCE_LOCK?)";

    // `initialize()` enters `waiting`, whose onentry calls
    // `performScxmlInvokeStart` — this publishes wire-14 on the
    // overridden `p2c_to_worker_session_f_wired_` Client, which dials
    // the worker's ephemeral endpoint.
    SCE::Test::inject_build_engine(parent);
    parent.initialize();

    using clock = std::chrono::steady_clock;
    const auto deadline = clock::now() + std::chrono::seconds(5);
    while (clock::now() < deadline) {
        // Drain inbound wire-15/18 replies and any pending
        // done.invoke external events the router has queued on the
        // engine.
        router.pumpScxmlInvokeReplies();
        parent.step();

        const auto state = parent.getCurrentState();
        if (state == ParentState::Pass) {
            SUCCEED();
            return;
        }
        ASSERT_NE(state, ParentState::Fail) << "parent observed error.execution before reaching pass. "
                                               "Likely the worker-side dispatch raised back through "
                                               "wire-20 instead of wire-18.";
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }

    FAIL() << "parent did not reach State::Pass within 5s. "
              "Current state="
           << static_cast<int>(parent.getCurrentState())
           << ". Expected wire-14 → wire-15 + wire-18 across custom_tcp; "
              "a timeout here typically means the worker never received "
              "wire-14 (PortOverride did not redirect, or the connect "
              "retry budget expired) or the worker's reply dropped on "
              "the parent's static Server.";
}
