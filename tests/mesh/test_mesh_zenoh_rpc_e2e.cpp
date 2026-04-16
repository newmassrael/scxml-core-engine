// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh Session F: mesh-rpc over Zenoh end-to-end runtime.
//
// Full client↔server round-trip through generated TransportRouter code:
//   brake (client) send_zenoh RpcRequest
//     → Zenoh session.get → motor queryable callback
//     → motor engine receives request
//     → motor mesh_send_cb_ (simulates injected <send>)
//       → resolvePattern → RpcReply
//       → correlation bridge (invoke_id → correlation_id)
//       → handleServerResponse → Query::reply
//     → brake on_reply closure → type rewrite → brake dispatchToSender
//     → brake engine receives reply event
//
// Both routers run in the same process with peer-mode sessions connected
// via explicit TCP locator (no daemon). The test uses TestSenderEngine
// doubles — the focus is on transport-level correctness, not SCXML state
// machine semantics (those are covered by the local transport invoke tests).
//
// Coverage:
//   - Client send_zenoh RpcRequest → session.get with CBOR payload
//   - Server queryable → pending query storage (keyed by invoke_id)
//   - resolvePattern for server response events → RpcReply
//   - Correlation bridge: invoke_id → correlation_id in mesh send callback
//   - handleServerResponse → Query::reply → CBOR encode
//   - Client on_reply → reply-event type rewrite → dispatchToSender
//   - invoke_id round-trip across transport boundary

#include "brake_zenoh_multi_sm.h"
#include "brake_zenoh_multi_transport.h"
#include "motor_zenoh_multi_sm.h"
#include "motor_zenoh_multi_transport.h"

#include "ZenohTestUtils.h"
#include "common/Uuid.h"
#include "mesh/MeshDispatch.h"

#include <cstdio>
#include <string>
#include <thread>

namespace {

using namespace SCE::Test::Mesh;

int run_test() {
    namespace brake_gen = SCE::Generated::brake_zenoh_multi;
    namespace motor_gen = SCE::Generated::motor_zenoh_multi;
    using PK = SCE::Mesh::PatternKind;
    using BrakeRouterT = brake_gen::TransportRouter<TestSenderEngine>;
    using MotorRouterT = motor_gen::TransportRouter<TestSenderEngine>;

    // Both routers bring up their own zenoh session through the generated
    // init() — motor listens, brake connects — driven entirely by the
    // two-device transports block in deploy_zenoh_multi.yaml. Bring up
    // the listener first so the endpoint is accepting before brake dials.
    TestSenderEngine motor_engine;
    MotorRouterT motor_router(motor_engine);
    MESH_TEST_REQUIRE(motor_router.init(), "motor_router.init() failed");

    TestSenderEngine brake_engine;
    BrakeRouterT brake_router(brake_engine);
    MESH_TEST_REQUIRE(brake_router.init(), "brake_router.init() failed");

    // Peer discovery stabilization.
    std::this_thread::sleep_for(std::chrono::seconds(1));

    // ── 1. RPC round-trip: brake send_zenoh → motor queryable →
    //       motor mesh_send_cb_ → resolvePattern → handleServerResponse
    //       → brake on_reply → brake engine ──────────────────────────
    //
    // brake_router.send_zenoh with RpcRequest pattern triggers
    // session.get(). The motor's queryable fires on a zenoh runtime
    // thread, decodes the envelope, stores the Query (keyed by
    // invoke_id), and dispatches the request to motor_engine.received_.
    // After motor_engine observes the request, we call motor_engine's
    // mesh_send_cb_ to simulate the engine's injected <send> for the
    // response. This exercises the full resolvePattern → RpcReply
    // → correlation bridge → handleServerResponse path.
    {
        auto invoke_id = SCE::uuid::v7();
        auto req = make_envelope("service.request.compute_force", PK::RpcRequest,
                                 R"({"input":"brake_force"})");
        req.invoke_id = invoke_id;
        const bool sent = brake_router.send_zenoh(req, brake_gen::ZENOH_KEY_MOTOR, "#motor");
        MESH_TEST_REQUIRE(sent, "brake send_zenoh RpcRequest returned false");

        // Motor receives the request via queryable → dispatchToSender.
        MESH_TEST_REQUIRE(motor_engine.received_.wait_for([](const auto& v) {
                    return !v.empty() && v.back().type == "service.request.compute_force";
                }),
                "motor engine did not receive RPC request through Zenoh queryable");

        // Verify the pending query is stored (keyed by invoke_id).
        {
            std::lock_guard<std::mutex> lock(motor_router.server_pending_mutex_);
            MESH_TEST_REQUIRE(!motor_router.pending_server_queries_.empty(),
                    "motor queryable did not store pending Query");
        }

        // Motor responds through the mesh send callback — the same path
        // the real engine takes when the injected <send target="#motor"
        // event="service.response.compute_force"/> fires. The invokeId
        // parameter carries the inbound request's invoke_id (which the
        // engine propagates via currentEventInvokeId_).
        auto invoke_id_str = SCE::uuid::to_string(invoke_id);
        MESH_TEST_REQUIRE(
                motor_engine.mesh_send_cb_(
                    "#motor", "service.response.compute_force",
                    R"({"result":42})", "", invoke_id_str),
                "motor mesh_send_cb_ returned false for server response "
                "(resolvePattern or correlation bridge failure)");

        // Brake must receive the reply. The brake router's send_zenoh
        // RpcRequest path installs an on_reply closure that:
        //   1. Decodes the CBOR response envelope
        //   2. Rewrites env.type to the resolveReplyEvent result
        //      ("service.response.compute_force")
        //   3. Sets env.pattern = RpcReply
        //   4. Calls dispatchToSender → brake_engine.raiseExternal
        MESH_TEST_REQUIRE(brake_engine.received_.wait_for([](const auto& v) {
                    return !v.empty() &&
                           v.back().type == "service.response.compute_force";
                }),
                "brake engine did not receive reply event "
                "'service.response.compute_force' from on_reply closure");

        // Verify payload survived the double CBOR round-trip
        // (brake encode → motor decode → motor encode → brake decode).
        {
            std::lock_guard<std::mutex> lock(brake_engine.received_.m);
            MESH_TEST_REQUIRE(brake_engine.received_.events.back().data.find("42") != std::string::npos,
                    "reply payload did not survive double CBOR round-trip");
        }

        motor_engine.received_.clear();
        brake_engine.received_.clear();
    }

    // ── 2. FireForget: brake router → motor generated subscriber → motor engine ─
    //
    // SCE_MESH.md §8.3 acid check: brake's session.put lands on the motor
    // server key, the generated `zenoh_server_fire_forget_sub_` picks it
    // up, and motor_engine observes the event through dispatchToSender.
    // Before Session G Task 3 the server had only declare_queryable so
    // session.put was dropped; this test would have required a raw
    // subscriber workaround to observe the envelope.
    {
        auto ff = make_envelope("service.fire_forget.activate", PK::FireForget,
                                R"({"reason":"emergency"})");
        const bool sent = brake_router.send_zenoh(ff, brake_gen::ZENOH_KEY_MOTOR, "#motor");
        MESH_TEST_REQUIRE(sent, "brake send_zenoh FireForget returned false");
        MESH_TEST_REQUIRE(motor_engine.received_.wait_for([](const auto& v) {
                    return !v.empty() &&
                           v.back().type == "service.fire_forget.activate";
                }),
                "motor engine did not receive FireForget via generated subscriber");
        {
            std::lock_guard<std::mutex> lock(motor_engine.received_.m);
            MESH_TEST_REQUIRE(motor_engine.received_.events.back().data.find("emergency")
                                  != std::string::npos,
                    "FireForget payload did not survive Zenoh CBOR round-trip");
        }
    }

    brake_router.shutdown();
    motor_router.shutdown();
    std::printf("SCE Mesh mesh-rpc over Zenoh E2E: PASS\n");
    return 0;
}

}  // namespace

int main() {
    try {
        return run_test();
    } catch (const std::exception& ex) {
        std::fprintf(stderr, "FAIL: uncaught exception: %s\n", ex.what());
        return 1;
    }
}
