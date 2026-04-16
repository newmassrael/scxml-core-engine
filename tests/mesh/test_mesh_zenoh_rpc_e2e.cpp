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

constexpr const char* kListen = "tcp/127.0.0.1:17449";

int run_test() {
    namespace brake_gen = SCE::Generated::brake_zenoh_multi;
    namespace motor_gen = SCE::Generated::motor_zenoh_multi;
    using PK = SCE::Mesh::PatternKind;
    using BrakeRouterT = brake_gen::TransportRouter<TestSenderEngine>;
    using MotorRouterT = motor_gen::TransportRouter<TestSenderEngine>;

    // ── Motor (server) setup: listen on kListen ──────────────────────
    TestSenderEngine motor_engine;
    MotorRouterT motor_router(motor_engine);

    {
        auto config = zenoh::Config::create_default();
        config.insert_json5("mode", "\"peer\"");
        config.insert_json5("listen/endpoints", std::string("[\"") + kListen + "\"]");
        config.insert_json5("scouting/multicast/enabled", "false");
        motor_router.zenoh_session_.emplace(zenoh::Session::open(std::move(config)));
    }
    motor_router.zenoh_queryable_.emplace(motor_router.zenoh_session_->declare_queryable(
        zenoh::KeyExpr(motor_gen::ZENOH_SERVER_KEY),
        [&motor_router](const zenoh::Query& query) {
            auto payload_opt = query.get_payload();
            if (!payload_opt.has_value()) {
                return;
            }
            auto bytes = payload_opt->get().as_vector();
            SCE::Mesh::MeshEnvelope env;
            if (!SCE::Mesh::decodeEnvelope(bytes.data(), bytes.size(), env)) {
                return;
            }
            auto cid = env.invoke_id.value_or(
                env.correlation_id.value_or(SCE::uuid::v7()));
            {
                std::lock_guard<std::mutex> lock(motor_router.server_pending_mutex_);
                motor_router.pending_server_queries_.insert_or_assign(
                    MotorRouterT::CorrelationKey{cid}, query.clone());
            }
            env.correlation_id = cid;
            env.pattern = SCE::Mesh::PatternKind::RpcRequest;
            (void)motor_router.dispatchToSender(env);
        },
        [] {}));

    // ── Brake (client) setup: connect to motor ───────────────────────
    TestSenderEngine brake_engine;
    BrakeRouterT brake_router(brake_engine);

    {
        auto config = zenoh::Config::create_default();
        config.insert_json5("mode", "\"peer\"");
        config.insert_json5("connect/endpoints", std::string("[\"") + kListen + "\"]");
        config.insert_json5("scouting/multicast/enabled", "false");
        brake_router.zenoh_session_.emplace(zenoh::Session::open(std::move(config)));
    }

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

    // ── 2. FireForget through generated router → motor subscriber ────
    //
    // Verify that the brake router's send_zenoh FireForget path (session.put)
    // also works against the motor's live session.
    {
        ReceivedEvents motor_ff_inbox;
        auto motor_ff_sub = motor_router.zenoh_session_->declare_subscriber(
            zenoh::KeyExpr(motor_gen::ZENOH_SERVER_KEY),
            [&motor_ff_inbox](const zenoh::Sample& sample) {
                auto bytes = sample.get_payload().as_vector();
                SCE::Mesh::MeshEnvelope env;
                if (SCE::Mesh::decodeEnvelope(bytes.data(), bytes.size(), env)) {
                    motor_ff_inbox.push({env.type,
                                         std::string(env.data.begin(), env.data.end())});
                }
            },
            [] {});

        std::this_thread::sleep_for(std::chrono::milliseconds(200));

        auto ff = make_envelope("service.fire_forget.activate", PK::FireForget);
        const bool sent = brake_router.send_zenoh(ff, brake_gen::ZENOH_KEY_MOTOR, "#motor");
        MESH_TEST_REQUIRE(sent, "brake send_zenoh FireForget returned false");

        MESH_TEST_REQUIRE(motor_ff_inbox.wait_for([](const auto& v) {
                    return !v.empty() &&
                           v.back().type == "service.fire_forget.activate";
                }),
                "motor subscriber did not receive FireForget from brake router");
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
