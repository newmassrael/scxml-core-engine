// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
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

constexpr const char* kListen = "tcp/127.0.0.1:17448";

int run_test() {
    namespace motor_gen = SCE::Generated::motor_zenoh_multi;
    using PK = SCE::Mesh::PatternKind;
    using MotorRouterT = motor_gen::TransportRouter<TestSenderEngine>;

    // ── Motor (server) side: TestSenderEngine + generated TransportRouter ──
    TestSenderEngine motor_engine;
    MotorRouterT motor_router(motor_engine);

    // Open motor's zenoh session in peer mode (listening). Cannot call
    // router.init() because the generated init() uses default config
    // (no peer mode, no explicit locator). Manual setup mirrors the
    // init() path but with test-friendly peer configuration.
    {
        auto config = zenoh::Config::create_default();
        config.insert_json5("mode", "\"peer\"");
        config.insert_json5("listen/endpoints", std::string("[\"") + kListen + "\"]");
        config.insert_json5("scouting/multicast/enabled", "false");
        motor_router.zenoh_session_.emplace(zenoh::Session::open(std::move(config)));
    }

    // Declare server-side queryable (same callback as generated init()).
    // The queryable receives inbound RPC requests, stores the Query for
    // later reply, and dispatches the request to the motor engine.
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
            auto cid = env.correlation_id.value_or(SCE::uuid::v7());
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

    // ── Client side: raw zenoh session ──
    auto client_session = open_peer(/*connect=*/kListen, /*listen=*/"");

    // Peer discovery stabilization.
    std::this_thread::sleep_for(std::chrono::seconds(1));

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
