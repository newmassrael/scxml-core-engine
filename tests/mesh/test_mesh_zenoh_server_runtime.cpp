// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh Session F: zenoh server-side runtime E2E.
//
// Exercises the generated server-side TransportRouter against a live zenoh
// peer in the same process. No zenoh router daemon is required — both
// peers open a zenoh::Session in peer mode with an explicit TCP locator
// for deterministic discovery.
//
// Coverage:
//   1. Server queryable:   client session.get → motor queryable callback
//                          receives request, stores Query, dispatches to engine
//   2. handleServerResponse: motor constructs reply envelope with matching
//                          correlation_id → Query::reply → client receives
//   3. CBOR round-trip:    encode on client send → decode on server receive
//                          → re-encode on server reply → decode on client receive
//   4. FieldRead (Session H): client session.get with env.pattern=FieldRead
//                          encoded by client resolvePattern → queryable decodes
//                          and trusts env.pattern → engine fires
//                          <transition event="field.get.position"> → engine
//                          <raise event="field.notify.position"> + injected <send>
//                          → handleServerResponse → Query::reply → client
//   5. FieldWrite (Session H): client session.put with env.pattern=FieldWrite
//                          encoded by client resolvePattern → put subscriber
//                          decodes and trusts env.pattern → engine fires
//                          <transition event="field.set.position">
//
// The test proves that the compile-only server-side infrastructure from
// Session E (declare_queryable, pending_server_queries_, handleServerResponse)
// actually works at runtime — callback threading, Query lifetime management,
// correlation_id keying, and zenoh reply semantics.

#include "motor_zenoh_multi_sm.h"
#include "motor_zenoh_multi_transport.h"

#include "ZenohTestUtils.h"
#include "mesh/MeshDispatch.h"

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
    using MotorRouterT = motor_gen::TransportRouter<TestSenderEngine>;

    // ── Motor (server) side: TestSenderEngine + generated TransportRouter ──
    //
    // router.init() opens the motor session (listen on kListen) and
    // declares the queryable that stores inbound RPC requests and
    // dispatches them to motor_engine. Both pieces come from the
    // generated ecu_motor device block in deploy_zenoh_multi.yaml.
    TestSenderEngine motor_engine;
    MotorRouterT motor_router(motor_engine);
    MESH_TEST_REQUIRE(motor_router.init(), "motor_router.init() failed");

    // ── Client side: raw zenoh session ──
    auto client_session = open_peer(/*connect=*/kListen, /*listen=*/"");

    // Deterministic convergence barrier: wait until the motor router's
    // queryable on ZENOH_SERVER_KEY is reachable from the client side.
    // The motor's init() declared that queryable as a first-class
    // init-time entity, so `declare_matching_listener` on a client-side
    // querier fires the moment zenoh's routing state has propagated
    // that fact — more precise than the liveliness proxy.
    wait_for_queryable(client_session, motor_gen::ZENOH_SERVER_KEY);

    // ── 1. Server queryable receives RPC request ──────────────────────
    //
    // Client sends session.get() with an RpcRequest envelope. The motor's
    // queryable callback decodes it, stores the Query, and dispatches to
    // the motor engine. We verify the engine actually received the event.
    ReceivedEvents client_replies;
    {
        auto req = make_envelope("service.request.compute_force", PK::RpcRequest,
                                 R"({"x":10})");
        auto req_bytes = SCE::Mesh::encodeEnvelope(req);

        zenoh::Session::GetOptions opts;
        opts.payload = zenoh::Bytes(std::move(req_bytes));
        client_session.get(
            zenoh::KeyExpr(motor_gen::ZENOH_SERVER_KEY), "",
            [&client_replies](const zenoh::Reply& reply_msg) {
                if (!reply_msg.is_ok()) {
                    return;
                }
                const auto& sample = reply_msg.get_ok();
                auto bytes = sample.get_payload().as_vector();
                SCE::Mesh::MeshEnvelope resp;
                if (SCE::Mesh::decodeEnvelope(bytes.data(), bytes.size(), resp)) {
                    client_replies.push({resp.type,
                                         std::string(resp.data.begin(), resp.data.end())});
                }
            },
            [] {}, std::move(opts));

        MESH_TEST_REQUIRE(motor_engine.received_.wait_for([](const auto& v) {
                    return !v.empty() && v.back().type == "service.request.compute_force";
                }),
                "motor engine did not receive RPC request from client session.get");
    }

    // ── 2. handleServerResponse → Query::reply → client receives ─────
    //
    // The motor processes the request and constructs a response. In
    // production this flows through the engine's <raise> → mesh send
    // callback → handleServerResponse. Here we call handleServerResponse
    // directly to verify the transport-level machinery: Query lookup by
    // correlation_id, Query::reply, CBOR encode, and client-side decode.
    {
        // Retrieve the correlation_id assigned by the queryable callback.
        std::array<uint8_t, 16> cid{};
        {
            std::lock_guard<std::mutex> lock(motor_router.server_pending_mutex_);
            MESH_TEST_REQUIRE(!motor_router.pending_server_queries_.empty(),
                    "queryable did not store pending Query");
            cid = motor_router.pending_server_queries_.begin()->first.id;
        }

        auto resp = make_envelope("service.response.compute_force", PK::RpcReply,
                                  R"({"result":42})");
        resp.correlation_id = cid;

        const bool replied = motor_router.handleServerResponse(resp);
        MESH_TEST_REQUIRE(replied, "handleServerResponse returned false (correlation miss)");

        // Verify pending query was consumed.
        {
            std::lock_guard<std::mutex> lock(motor_router.server_pending_mutex_);
            MESH_TEST_REQUIRE(motor_router.pending_server_queries_.empty(),
                    "pending Query not erased after reply");
        }

        // Client must receive the reply via zenoh.
        MESH_TEST_REQUIRE(client_replies.wait_for([](const auto& v) {
                    return !v.empty() &&
                           v.back().type == "service.response.compute_force";
                }),
                "client did not receive RPC reply via Query::reply");

        // Verify payload round-trip.
        {
            std::lock_guard<std::mutex> lock(client_replies.m);
            MESH_TEST_REQUIRE(client_replies.events.back().data.find("42") != std::string::npos,
                    "reply payload did not preserve result value");
        }
    }

    // ── 3. Uncorrelated response returns false ───────────────────────
    {
        auto bad_resp = make_envelope("service.response.compute_force", PK::RpcReply);
        bad_resp.correlation_id = SCE::uuid::v7();
        MESH_TEST_REQUIRE(!motor_router.handleServerResponse(bad_resp),
                "handleServerResponse should return false for unknown correlation_id");
    }

    // ── 4. No-correlation_id response returns false ──────────────────
    {
        auto no_cid_resp = make_envelope("service.response.compute_force", PK::RpcReply);
        MESH_TEST_REQUIRE(!motor_router.handleServerResponse(no_cid_resp),
                "handleServerResponse should return false when correlation_id is nullopt");
    }

    // ── 5. FieldRead inbound on server queryable (SCE_MESH.md §8.3) ──
    //
    // Session H: field.get.position is a FieldRead — the client dispatches
    // via session.get with env.pattern=FieldRead pre-encoded on the wire
    // (client-side resolvePattern). The server's queryable decodes the
    // envelope, trusts env.pattern unchanged, and forwards it to the
    // motor engine which fires <transition event="field.get.position">.
    motor_engine.received_.clear();
    ReceivedEvents field_read_replies;
    {
        auto req = make_envelope("field.get.position", PK::FieldRead, R"({"which":"x"})");
        auto req_bytes = SCE::Mesh::encodeEnvelope(req);

        zenoh::Session::GetOptions opts;
        opts.payload = zenoh::Bytes(std::move(req_bytes));
        client_session.get(
            zenoh::KeyExpr(motor_gen::ZENOH_SERVER_KEY), "",
            [&field_read_replies](const zenoh::Reply& reply_msg) {
                if (!reply_msg.is_ok()) return;
                const auto& sample = reply_msg.get_ok();
                auto bytes = sample.get_payload().as_vector();
                SCE::Mesh::MeshEnvelope resp;
                if (SCE::Mesh::decodeEnvelope(bytes.data(), bytes.size(), resp)) {
                    field_read_replies.push({resp.type,
                                             std::string(resp.data.begin(), resp.data.end())});
                }
            },
            [] {}, std::move(opts));

        MESH_TEST_REQUIRE(motor_engine.received_.wait_for([](const auto& v) {
                    return !v.empty() && v.back().type == "field.get.position";
                }),
                "motor engine did not receive field.get.position from client session.get");

        std::array<uint8_t, 16> cid{};
        {
            std::lock_guard<std::mutex> lock(motor_router.server_pending_mutex_);
            MESH_TEST_REQUIRE(!motor_router.pending_server_queries_.empty(),
                    "queryable did not stash pending FieldRead Query");
            cid = motor_router.pending_server_queries_.begin()->first.id;
        }

        // Simulate the engine's paired `<raise event="field.notify.position">`
        // → injected <send> → handleServerResponse route.
        auto notify = make_envelope("field.notify.position", PK::FieldNotify,
                                    R"({"x":42})");
        notify.correlation_id = cid;
        MESH_TEST_REQUIRE(motor_router.handleServerResponse(notify),
                "handleServerResponse rejected FieldNotify reply");

        MESH_TEST_REQUIRE(field_read_replies.wait_for([](const auto& v) {
                    return !v.empty() && v.back().type == "field.notify.position";
                }),
                "client did not receive field.notify reply via Query::reply");
    }

    // ── 6. FieldWrite inbound on server put subscriber (SCE_MESH.md §8.3) ──
    //
    // session.put lands on the subscriber (not the queryable) because
    // Zenoh's queryable only fires for session.get. The subscriber
    // decodes env.pattern=FieldWrite (pre-encoded on the wire by the
    // client's resolvePattern) and forwards unchanged to the engine.
    motor_engine.received_.clear();
    {
        auto req = make_envelope("field.set.position", PK::FieldWrite,
                                 R"({"x":99})");
        auto req_bytes = SCE::Mesh::encodeEnvelope(req);
        client_session.put(zenoh::KeyExpr(motor_gen::ZENOH_SERVER_KEY),
                           zenoh::Bytes(std::move(req_bytes)));

        MESH_TEST_REQUIRE(motor_engine.received_.wait_for([](const auto& v) {
                    return !v.empty() && v.back().type == "field.set.position";
                }),
                "motor engine did not receive field.set.position from client session.put");
        {
            std::lock_guard<std::mutex> lock(motor_engine.received_.m);
            MESH_TEST_REQUIRE(motor_engine.received_.events.back().data.find("99")
                                  != std::string::npos,
                    "FieldWrite payload did not survive Zenoh CBOR round-trip");
        }
    }

    motor_router.shutdown();
    std::printf("SCE Mesh zenoh server-side runtime E2E: PASS\n");
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
