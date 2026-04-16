// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh Session D: zenoh multi-pattern peer-mode runtime E2E.
//
// Exercises the generated send_zenoh dispatcher against a live zenoh
// peer in the same process. No zenoh router daemon is required — both
// peers open a zenoh::Session in peer mode with an explicit TCP
// locator for deterministic discovery.
//
// Coverage:
//   - FireForget:       send_zenoh → session.put       → raw subscriber  receives envelope
//   - RpcRequest:       send_zenoh → session.get       → raw queryable   replies via Query::reply
//                       → on_reply closure rewrites env.type = reply-event, pattern = RpcReply
//   - EventSubscribe:   send_zenoh → declare_subscriber → raw publisher put → subscriber callback
//   - FieldRead:        send_zenoh → session.get       → raw queryable   replies (FieldNotify)
//
// The receive path on the sender side is driven by the TransportRouter's
// `dispatchToSender` helper, which calls MeshDispatch::dispatchEnvelope
// against the sender engine that was bound in the router ctor. For this
// test we stand up a minimal TestSenderEngine that records every envelope
// it is asked to dispatch — that replaces the earlier pattern of reaching
// into the router's internal receive_callback_ member.

#include "brake_zenoh_multi_transport.h"

#include "ZenohTestUtils.h"
#include "mesh/MeshDispatch.h"

#include <cstdio>
#include <string>
#include <thread>

namespace {

using namespace SCE::Test::Mesh;
namespace PK = SCE::Mesh;

constexpr const char* kMotorKey = "sce/brake/motor";
// Must match deploy_zenoh_multi.yaml ecu_motor transports.zenoh.listen.
constexpr const char* kListen   = "tcp/127.0.0.1:17447";

int run_test() {
    using namespace SCE::Generated::brake_zenoh_multi;
    using RouterT = TransportRouter<TestSenderEngine>;

    // ── Sender side: TestSenderEngine + generated TransportRouter ──────
    TestSenderEngine sender;
    RouterT router(sender);

    // ── Order matters: bring up the listener first, then the connector,
    //    so the TCP endpoint is accepting before the peer dials. ──────
    auto motor_session = open_peer(/*connect=*/"", /*listen=*/kListen);

    // router.init() opens brake's zenoh session with the mode/connect
    // endpoints generated from deploy_zenoh_multi.yaml (ecu_brake device).
    MESH_TEST_REQUIRE(router.init(), "router.init() failed");

    // FireForget / FieldWrite receive point — plain subscriber on the
    // motor key. Record whatever arrives so the test can assert on it.
    CapturedEvents motor_inbox;
    auto motor_subscriber = motor_session.declare_subscriber(
        zenoh::KeyExpr(kMotorKey),
        [&motor_inbox](const zenoh::Sample& sample) {
            auto bytes = sample.get_payload().as_vector();
            SCE::Mesh::MeshEnvelope env;
            if (SCE::Mesh::decodeEnvelope(bytes.data(), bytes.size(), env)) {
                motor_inbox.push(env);
            }
        },
        [] {});

    // RPC / FieldRead receive point — queryable on the same key. Echoes
    // request type back with a small JSON payload so the sender can
    // verify the reply round-trip.
    auto motor_queryable = motor_session.declare_queryable(
        zenoh::KeyExpr(kMotorKey),
        [](const zenoh::Query& query) {
            std::string req_type = "unknown";
            if (auto payload = query.get_payload(); payload) {
                auto bytes = payload->get().as_vector();
                SCE::Mesh::MeshEnvelope req;
                if (SCE::Mesh::decodeEnvelope(bytes.data(), bytes.size(), req)) {
                    req_type = req.type;
                }
            }
            auto resp_env = make_envelope(
                req_type + ".response", SCE::Mesh::PatternKind::FireForget,
                std::string(R"({"result":42})"));
            auto bytes = SCE::Mesh::encodeEnvelope(resp_env);
            query.reply(zenoh::KeyExpr(kMotorKey), zenoh::Bytes(std::move(bytes)));
        },
        [] {});

    // Peer discovery stabilization.
    std::this_thread::sleep_for(std::chrono::seconds(1));

    // ── 1. FireForget: send_zenoh → session.put → motor subscriber ─────
    {
        auto env = make_envelope("service.fire_forget.activate",
                                 SCE::Mesh::PatternKind::FireForget);
        const bool ok = router.send_zenoh(env, kMotorKey, "#motor");
        MESH_TEST_REQUIRE(ok, "send_zenoh FireForget returned false");
        MESH_TEST_REQUIRE(motor_inbox.wait_for([](const auto& v) { return !v.empty(); }),
                "motor subscriber did not receive FireForget envelope");
        std::lock_guard<std::mutex> lk(motor_inbox.m);
        MESH_TEST_REQUIRE(motor_inbox.envelopes.front().type == "service.fire_forget.activate",
                "FireForget envelope type mismatch on motor side");
        MESH_TEST_REQUIRE(motor_inbox.envelopes.front().pattern == SCE::Mesh::PatternKind::FireForget,
                "FireForget envelope pattern not preserved");
        motor_inbox.envelopes.clear();
    }

    // ── 2. RpcRequest: send_zenoh → session.get → queryable → on_reply ─
    {
        auto env = make_envelope("service.request.compute_force",
                                 SCE::Mesh::PatternKind::RpcRequest);
        const bool ok = router.send_zenoh(env, kMotorKey, "#motor");
        MESH_TEST_REQUIRE(ok, "send_zenoh RpcRequest returned false");

        MESH_TEST_REQUIRE(sender.received_.wait_for([](const auto& v) {
                    return !v.empty() && v.back().type == "service.response.compute_force";
                }),
                "sender engine did not see paired reply 'service.response.compute_force'");
        sender.received_.clear();
    }

    // ── 3. EventSubscribe: sender subscribes, receiver publishes ─────
    {
        auto env = make_envelope("event.subscribe.status",
                                 SCE::Mesh::PatternKind::EventSubscribe);
        const bool ok = router.send_zenoh(env, kMotorKey, "#motor");
        MESH_TEST_REQUIRE(ok, "send_zenoh EventSubscribe returned false");
        std::this_thread::sleep_for(std::chrono::milliseconds(200));

        auto notify = make_envelope("event.notification.status",
                                    SCE::Mesh::PatternKind::FireForget);
        auto bytes = SCE::Mesh::encodeEnvelope(notify);
        motor_session.put(zenoh::KeyExpr(kMotorKey), zenoh::Bytes(std::move(bytes)));

        MESH_TEST_REQUIRE(sender.received_.wait_for([](const auto& v) {
                    return !v.empty() && v.back().type == "event.notification.status";
                }),
                "sender engine did not see 'event.notification.status' from subscriber");
        sender.received_.clear();
    }

    // ── 4. FieldRead: structural mirror of RpcRequest ──────────────────
    {
        auto env = make_envelope("field.get.position",
                                 SCE::Mesh::PatternKind::FieldRead);
        const bool ok = router.send_zenoh(env, kMotorKey, "#motor");
        MESH_TEST_REQUIRE(ok, "send_zenoh FieldRead returned false");
        MESH_TEST_REQUIRE(sender.received_.wait_for([](const auto& v) {
                    return !v.empty() && v.back().type == "field.get.position.response";
                }),
                "sender engine did not see FieldRead reply echo");
        sender.received_.clear();
    }

    // ── 5. EventUnsubscribe: handle map entry drops on erase ───────────
    {
        auto env = make_envelope("event.unsubscribe.status",
                                 SCE::Mesh::PatternKind::EventUnsubscribe);
        const bool ok = router.send_zenoh(env, kMotorKey, "#motor");
        MESH_TEST_REQUIRE(ok, "send_zenoh EventUnsubscribe returned false");
        MESH_TEST_REQUIRE(router.zenoh_subscribers_.count("#motor") == 0,
                "subscriber handle not erased on EventUnsubscribe");
    }

    router.shutdown();
    std::printf("SCE Mesh zenoh multi-pattern runtime E2E: PASS\n");
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
