// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §9.6.2 wire-18 cross-device donedata — parent half.
//
// Consumes three `MESH_PEER_ENDPOINT_worker_session_f_donedata_<shape>`
// env vars (populated by run_two_process_fixture.sh from the worker's
// multi-peer `LISTEN_ENDPOINT_<peer>=` lines) and threads all three
// into a single `PortOverride::peer_connect_endpoints` map before
// `TransportRouter::init`. The three overrides redirect each of
// `p2c_to_worker_session_f_donedata_{param,content,nested}_` Clients
// away from the deploy.yaml `"127.0.0.1:0"` placeholder to the live
// ephemeral ports the worker announced.
//
// Assertion model: `parent_session_f_donedata.scxml` already guards
// each donedata shape with a cond on the decoded `_event.data`
// (equivalent to `_event.data.result == 42`, `_event.data ==
// 'hello_content'`, and the nested object/array/integer probe). If
// ANY cond fails, the parent takes the `fail` transition and reaches
// `State::Fail`; if ALL three phases pass sequentially, the parent
// reaches `State::Pass`. Therefore reaching Pass is observable
// evidence that the decoded `_event.data` carried by wire-18 over
// custom_tcp is semantically equivalent to the shm baseline — which
// is exactly the Stage B Q3 criterion (decoded round-trip, not
// byte-identical wire, because CBOR + length-prefix framing is
// transport-specific and cannot match the shm baseline verbatim).
//
// Timeout budget: 10s covers three sequential invokes including the
// Client retry budget (Stage A1 documented 1s per dial).

#include "common/TestScriptEngine.h"
#include "parent_session_f_donedata_sm.h"
#include "parent_session_f_donedata_transport.h"

#include "mesh/transports/CustomTcpTransport.h"

#include <chrono>
#include <cstdlib>
#include <gtest/gtest.h>
#include <thread>

TEST(CrossdevDonedata, ThreeShapesSurviveWire18OverCustomTcp) {
    const char *ep_param = std::getenv("MESH_PEER_ENDPOINT_worker_session_f_donedata_param");
    const char *ep_content = std::getenv("MESH_PEER_ENDPOINT_worker_session_f_donedata_content");
    const char *ep_nested = std::getenv("MESH_PEER_ENDPOINT_worker_session_f_donedata_nested");
    ASSERT_NE(ep_param, nullptr) << "param peer endpoint env var missing";
    ASSERT_NE(ep_content, nullptr) << "content peer endpoint env var missing";
    ASSERT_NE(ep_nested, nullptr) << "nested peer endpoint env var missing";

    using ParentEngine = SCE::Generated::parent_session_f_donedata::parent_session_f_donedata;
    using ParentRouter = SCE::Generated::parent_session_f_donedata::TransportRouter<ParentEngine>;
    using ParentState = SCE::Generated::parent_session_f_donedata::State;

    ParentEngine parent;
    ParentRouter router({&parent});

    SCE::Mesh::CustomTcp::PortOverride port_override;
    port_override.peer_connect_endpoints["worker_session_f_donedata_param"] = ep_param;
    port_override.peer_connect_endpoints["worker_session_f_donedata_content"] = ep_content;
    port_override.peer_connect_endpoints["worker_session_f_donedata_nested"] = ep_nested;
    ASSERT_TRUE(router.init(port_override)) << "parent TransportRouter::init(PortOverride) failed — likely a "
                                               "bind collision on SCE_TEST_CROSSDEV_DONEDATA_PORT (parallel "
                                               "ctest holding the RESOURCE_LOCK?)";

    SCE::Test::inject_build_engine(parent);
    parent.initialize();

    using clock = std::chrono::steady_clock;
    const auto deadline = clock::now() + std::chrono::seconds(10);
    while (clock::now() < deadline) {
        router.pumpScxmlInvokeReplies();
        parent.step();

        const auto state = parent.getCurrentState();
        if (state == ParentState::Pass) {
            SUCCEED();
            return;
        }
        ASSERT_NE(state, ParentState::Fail) << "parent took the fail transition. One of the three donedata "
                                               "conds did not match the wire-18 payload: either the param "
                                               "shape lost the name/value pair, the content shape dropped "
                                               "the primitive string, or the nested shape regressed the "
                                               "canonical-JSON contract (object + array + integer).";
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }

    FAIL() << "parent did not reach Pass within 10s. "
              "Current state="
           << static_cast<int>(parent.getCurrentState())
           << ". Expected three sequential wire-14 → wire-18 round-trips "
              "(param → content → nested). A timeout usually means the "
              "current phase's PortOverride did not redirect the dial or "
              "the worker's reply did not reach the parent's Server.";
}
