// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh Session I: engine-driven RPC / FieldAccess roundtrip over Zenoh.
//
// Sibling to test_mesh_zenoh_server_runtime.cpp. That file uses
// TestSenderEngine (a recorder) and manually calls
// motor_router.handleServerResponse() — so it validates the transport
// layer (queryable → CBOR decode → dispatchToSender) but never actually
// drives a state machine through a transition, raise, and injected
// <send>. This file closes that gap with a real motor_zenoh_multi SM.
//
// End-to-end chain exercised here:
//
//   client session.get (env.invoke_id stamped, simulating a mesh-rpc invoker)
//     → motor queryable callback (zenoh runtime thread)
//         → stashes Query keyed by env.invoke_id
//         → dispatchToSender → motor.raiseExternal(EventWithMetadata)
//             → meta.invokeId = env.invoke_id  [MeshDispatch.h:84-86]
//     → driver thread motor.step()
//         → external queue dequeue sets currentEventInvokeId_  [StaticExecutionEngine.h:757]
//         → transition: ready --service.request.compute_force--> computing
//         → computing onentry: <raise service.response.compute_force>
//                              + codegen-injected <send target="#motor">
//         → raiseExternal(Event, "", "", "#motor")
//             → isMeshTarget → performMeshSend → onMeshSend_
//             → TransportRouter's lambda: resolvePattern → RpcReply
//             → env.invoke_id = parsed(currentEventInvokeId_)
//             → env.correlation_id = env.invoke_id
//             → handleServerResponse → Query::reply (CBOR encode)
//   → client on_reply decodes → reply event observed
//
// Any regression in the engine-side half of this chain (raise suppression,
// injected <send> missing, invoke_id propagation dropped, mesh_send_cb_
// mis-routing a FieldNotify/RpcReply) surfaces here and nowhere else.
//
// Coverage:
//   §1 RPC roundtrip: service.request.compute_force → computing onentry
//                     → service.response.compute_force reply
//   §2 FieldRead roundtrip: field.get.position → ready transition raise
//                     → field.notify.position reply via FieldNotify pattern
//
// For FireForget / FieldWrite (no reply) see test_mesh_zenoh_server_runtime.cpp
// — those patterns intentionally have no client-observable return, so
// recorder-style transport assertions are the only textbook observation.

#include "motor_zenoh_multi_sm.h"
#include "motor_zenoh_multi_transport.h"

#include "ZenohTestUtils.h"
#include "common/Uuid.h"
#include "mesh/MeshEnvelopeCodec.h"

#include <atomic>
#include <chrono>
#include <cstdio>
#include <string>
#include <thread>

namespace {

using namespace SCE::Test::Mesh;

// Must match deploy_zenoh_multi.yaml ecu_motor transports.zenoh.listen.
constexpr const char* kListen = "tcp/127.0.0.1:17447";

int run_test() {
    namespace motor_gen = SCE::Generated::motor_zenoh_multi;
    using PK = SCE::Mesh::PatternKind;
    using Motor = motor_gen::motor_zenoh_multi;
    using MotorRouterT = motor_gen::TransportRouter<Motor>;

    // Bring up the real SM first. initialize() enters the <initial="ready">
    // configuration so the transport callback (declared in router.init())
    // is never observed by a not-yet-initialized engine.
    Motor motor;
    motor.initialize();

    MotorRouterT motor_router(motor);
    MESH_TEST_REQUIRE(motor_router.init(), "motor_router.init() failed");

    // Driver thread pumps the engine's external queue. The queryable
    // enqueues inbound events onto externalQueue_ from a zenoh runtime
    // thread; step() drains internal+external queues and executes
    // transitions. EventQueueManager is mutex-protected
    // (core/EventQueueManager.h) so the cross-thread raise/step is safe.
    //
    // RAII-scoped so every control-flow exit (including MESH_TEST_REQUIRE's
    // early return) stops the pump and joins before std::thread's
    // terminate-on-joinable-destruction can fire.
    struct EnginePump {
        std::atomic<bool> running{true};
        std::thread t;
        ~EnginePump() {
            running.store(false, std::memory_order_release);
            if (t.joinable()) t.join();
        }
    } pump;
    pump.t = std::thread([&motor, &pump] {
        while (pump.running.load(std::memory_order_acquire)) {
            motor.step();
            std::this_thread::sleep_for(std::chrono::milliseconds(1));
        }
    });

    // Client session. Peer mode, connects to the motor's listen endpoint
    // declared in deploy_zenoh_multi.yaml; multicast scouting off so the
    // test is hermetic.
    auto client_session = open_peer(/*connect=*/kListen, /*listen=*/"");
    std::this_thread::sleep_for(std::chrono::seconds(1));  // peer handshake

    // ── §1. RPC roundtrip: engine-driven ──────────────────────────────
    //
    // Set env.invoke_id on the outbound request. This simulates an
    // upstream <invoke type="sce:mesh-rpc"> which stamps invoke_id on the
    // envelope before it leaves the sender. The server's queryable stashes
    // the Query keyed by this UUID; dispatchEnvelope forwards the UUID as
    // EventWithMetadata.invokeId; the engine surfaces it through
    // currentEventInvokeId_; the injected <send>'s resolvePattern returns
    // RpcReply and the mesh callback stamps it back as env.correlation_id,
    // which is exactly what handleServerResponse needs.
    ReceivedEvents rpc_replies;
    {
        auto req = make_envelope("service.request.compute_force", PK::RpcRequest,
                                 R"({"force":100})");
        req.invoke_id = SCE::uuid::v7();
        auto req_bytes = SCE::Mesh::encodeEnvelope(req);

        zenoh::Session::GetOptions opts;
        opts.payload = zenoh::Bytes(std::move(req_bytes));
        client_session.get(
            zenoh::KeyExpr(motor_gen::ZENOH_SERVER_KEY), "",
            [&rpc_replies](const zenoh::Reply& reply_msg) {
                if (!reply_msg.is_ok()) return;
                const auto& sample = reply_msg.get_ok();
                auto bytes = sample.get_payload().as_vector();
                SCE::Mesh::MeshEnvelope resp;
                if (SCE::Mesh::decodeEnvelope(bytes.data(), bytes.size(), resp)) {
                    rpc_replies.push({resp.type,
                                      std::string(resp.data.begin(), resp.data.end())});
                }
            },
            [] {}, std::move(opts));

        MESH_TEST_REQUIRE(rpc_replies.wait_for([](const auto& v) {
                    return !v.empty() &&
                           v.back().type == "service.response.compute_force";
                }),
                "engine-driven RPC reply not received — regression in "
                "transition→raise→injected-send→correlation chain");
    }

    // ── §2. FieldRead roundtrip: engine-driven ────────────────────────
    //
    // Same chain as RPC but with pattern=FieldRead → paired FieldNotify
    // reply. The motor SCXML's transition on field.get.position carries
    // <raise event="field.notify.position"/>; the codegen's
    // inject_server_response_sends adds the synthetic <send> that routes
    // the reply through handleServerResponse.
    ReceivedEvents field_replies;
    {
        auto req = make_envelope("field.get.position", PK::FieldRead,
                                 R"({"axis":"x"})");
        req.invoke_id = SCE::uuid::v7();
        auto req_bytes = SCE::Mesh::encodeEnvelope(req);

        zenoh::Session::GetOptions opts;
        opts.payload = zenoh::Bytes(std::move(req_bytes));
        client_session.get(
            zenoh::KeyExpr(motor_gen::ZENOH_SERVER_KEY), "",
            [&field_replies](const zenoh::Reply& reply_msg) {
                if (!reply_msg.is_ok()) return;
                const auto& sample = reply_msg.get_ok();
                auto bytes = sample.get_payload().as_vector();
                SCE::Mesh::MeshEnvelope resp;
                if (SCE::Mesh::decodeEnvelope(bytes.data(), bytes.size(), resp)) {
                    field_replies.push({resp.type,
                                        std::string(resp.data.begin(), resp.data.end())});
                }
            },
            [] {}, std::move(opts));

        MESH_TEST_REQUIRE(field_replies.wait_for([](const auto& v) {
                    return !v.empty() &&
                           v.back().type == "field.notify.position";
                }),
                "engine-driven FieldRead reply not received — regression in "
                "transition<raise>→FieldNotify correlation chain");
    }

    // Teardown order matters: stop the engine pump BEFORE shutting down
    // the transport (which closes the zenoh session). If we tore down
    // the session first a final in-flight step() could race with the
    // Query object handleServerResponse is about to reply on.
    //
    // RAII-scoped EnginePump dtor handles the stop+join for any
    // early-return path; here we do it eagerly so the shutdown sequence
    // below observes a quiet engine. The pump's dtor becomes a no-op on
    // the happy path (joinable() is false after this block).
    pump.running.store(false, std::memory_order_release);
    pump.t.join();
    motor_router.shutdown();
    std::printf("SCE Mesh Session I zenoh engine-driven RPC/FieldRead: PASS\n");
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
