// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh Session F: SOME/IP runtime E2E.
//
// Exercises both client (brake) and server (motor) generated
// TransportRouters end-to-end via vsomeip internal routing (single
// process, no daemon). Both applications run in the same process; the
// routing manager is embedded in the motor (server) application per
// the VSOMEIP_CONFIGURATION JSON.
//
// Coverage:
//   1. Server init:   offer_service + register_message_handler for
//                     compute_force RPC method (0x0101), activate
//                     FireForget (0x0100), getter (0x0200), setter (0x0201)
//   2. Client init:   request_service + register_message_handler for
//                     response correlation
//   3. RPC round-trip: brake send_to_motor (RpcRequest) → vsomeip routes
//                     to motor callback → motor engine receives →
//                     handleServerResponse → vsomeip response → brake
//                     response handler → brake engine receives reply
//   4. FireForget:    brake send_to_motor → vsomeip routes → motor
//                     FireForget handler (method 0x0100) → motor engine
//                     receives service.fire_forget.activate
//   5. FieldRead:     brake send_to_motor (FieldRead) → vsomeip routes to
//                     getter handler (0x0200) → motor engine receives
//                     field.get.vehicle_speed; pending request stashed
//                     for field.notify reply correlation
//   6. FieldWrite:    brake send_to_motor (FieldWrite) → setter handler
//                     (0x0201) → motor engine receives field.set.target_speed
//
// VSOMEIP_CONFIGURATION must point to vsomeip_e2e_test.json (internal
// routing, service discovery disabled, motor as routing manager).

#include "brake_someip_multi_sm.h"
#include "brake_someip_multi_transport.h"
#include "motor_someip_multi_sm.h"
#include "motor_someip_multi_transport.h"

#include "MeshTestUtils.h"
#include "SomeipTestUtils.h"
#include "mesh/MeshDispatch.h"

#include <chrono>
#include <condition_variable>
#include <cstdio>
#include <mutex>
#include <string>
#include <thread>

namespace {

using namespace SCE::Test::Mesh;

int run_test() {
    namespace brake_gen = SCE::Generated::brake_someip_multi;
    namespace motor_gen = SCE::Generated::motor_someip_multi;
    using PK = SCE::Mesh::PatternKind;
    using BrakeRouterT = brake_gen::TransportRouter<TestSenderEngine>;
    using MotorRouterT = motor_gen::TransportRouter<TestSenderEngine>;

    wipe_stale_vsomeip_sockets();

    // ── Motor (server) must init first — it is the routing manager. ──
    TestSenderEngine motor_engine;
    MotorRouterT motor_router(motor_engine);
    MESH_TEST_REQUIRE(motor_router.init(), "motor router init failed");

    // ── Brake (client) connects to motor. ──
    TestSenderEngine brake_engine;
    BrakeRouterT brake_router(brake_engine);
    MESH_TEST_REQUIRE(brake_router.init(), "brake router init failed");

    // Instead of sleeping, wait for vsomeip to announce motor's service
    // as available on brake's app. `register_availability_handler` fires
    // synchronously with the current status when it is called on an
    // already-available service, and asynchronously when the state later
    // changes — so registering after init() is safe regardless of
    // handshake ordering.
    {
        std::mutex availability_m;
        std::condition_variable availability_cv;
        bool service_available = false;
        brake_router.motor_app_->register_availability_handler(
            brake_gen::SOMEIP_SERVICE_MOTOR, brake_gen::SOMEIP_INSTANCE_MOTOR,
            [&](vsomeip::service_t, vsomeip::instance_t, bool is_available) {
                if (!is_available) return;
                std::lock_guard<std::mutex> lock(availability_m);
                service_available = true;
                availability_cv.notify_all();
            });

        std::unique_lock<std::mutex> lock(availability_m);
        MESH_TEST_REQUIRE(availability_cv.wait_for(
                              lock, std::chrono::seconds(10),
                              [&] { return service_available; }),
                          "vsomeip service motor_control did not become "
                          "available within 10s (routing manager handshake stuck)");
    }

    // ── 1. RPC round-trip: brake → motor → brake ──────────────────────
    {
        SCE::Mesh::MeshEnvelope env;
        env.id = SCE::uuid::v7();
        env.source = "test";
        env.type = "service.request.compute_force";
        env.pattern = PK::RpcRequest;
        env.datacontenttype = SCE::Mesh::PayloadCodec::Json;
        std::string payload = R"({"force":100})";
        env.data.assign(payload.begin(), payload.end());
        // Correlation setup: generate UUID, register reply event
        auto cid = SCE::uuid::v7();
        env.correlation_id = cid;
        {
            std::lock_guard<std::mutex> lock(brake_router.correlation_mutex_);
            brake_router.pending_rpcs_[BrakeRouterT::CorrelationKey{cid}] =
                "service.response.compute_force";
        }

        const bool sent = brake_router.route_send("#motor", env);
        MESH_TEST_REQUIRE(sent, "brake route_send RpcRequest returned false");

        // Motor receives the request via register_message_handler.
        MESH_TEST_REQUIRE(motor_engine.received_.wait_for([](const auto& v) {
                    return !v.empty() &&
                           v.back().type == "service.request.compute_force";
                }),
                "motor engine did not receive RPC request via SOME/IP");

        // Motor responds via handleServerResponse.
        std::array<uint8_t, 16> server_cid{};
        {
            std::lock_guard<std::mutex> lock(motor_router.server_pending_mutex_);
            MESH_TEST_REQUIRE(!motor_router.pending_server_requests_.empty(),
                    "motor did not store pending request");
            server_cid = motor_router.pending_server_requests_.begin()->first.id;
        }

        SCE::Mesh::MeshEnvelope resp;
        resp.id = {};
        resp.source = "motor";
        resp.type = "service.response.compute_force";
        resp.pattern = PK::RpcReply;
        resp.datacontenttype = SCE::Mesh::PayloadCodec::Json;
        std::string resp_payload = R"({"result":42})";
        resp.data.assign(resp_payload.begin(), resp_payload.end());
        resp.correlation_id = server_cid;

        MESH_TEST_REQUIRE(motor_router.handleServerResponse(resp),
                "handleServerResponse returned false");

        // Brake receives the correlated reply.
        MESH_TEST_REQUIRE(brake_engine.received_.wait_for([](const auto& v) {
                    return !v.empty() &&
                           v.back().type == "service.response.compute_force";
                }),
                "brake engine did not receive RPC reply via SOME/IP");

        // Verify payload survived CBOR round-trip.
        {
            std::lock_guard<std::mutex> lock(brake_engine.received_.m);
            MESH_TEST_REQUIRE(brake_engine.received_.events.back().data.find("42") != std::string::npos,
                    "reply payload did not survive SOME/IP CBOR round-trip");
        }
    }

    // ── 2. FireForget: brake → motor FireForget handler → motor engine ─
    //
    // SCE_MESH.md §8.3 acid check: motor's <transition event="service.fire_forget.activate">
    // must fire when brake sends the event. Before Session G Task 3 the
    // server only registered compute_force (0x0101), so FireForget 0x0100
    // was dropped by vsomeip and the motor engine never observed it.
    motor_engine.received_.clear();
    {
        SCE::Mesh::MeshEnvelope env;
        env.id = SCE::uuid::v7();
        env.source = "test";
        env.type = "service.fire_forget.activate";
        env.pattern = PK::FireForget;
        env.datacontenttype = SCE::Mesh::PayloadCodec::Json;
        std::string payload = R"({"reason":"emergency"})";
        env.data.assign(payload.begin(), payload.end());

        MESH_TEST_REQUIRE(brake_router.route_send("#motor", env),
                          "brake route_send FireForget returned false");
        MESH_TEST_REQUIRE(motor_engine.received_.wait_for([](const auto& v) {
                    return !v.empty() &&
                           v.back().type == "service.fire_forget.activate";
                }),
                "motor engine did not receive FireForget via SOME/IP server handler");
        {
            std::lock_guard<std::mutex> lock(motor_engine.received_.m);
            MESH_TEST_REQUIRE(motor_engine.received_.events.back().data.find("emergency")
                                  != std::string::npos,
                    "FireForget payload did not survive SOME/IP CBOR round-trip");
        }
    }

    // ── 3. FieldRead (getter): brake → motor getter handler → motor engine ─
    //
    // SCE_MESH.md §8.3 acid check: motor's
    // `<transition event="field.get.vehicle_speed">` must fire when brake
    // sends on the vsomeip getter method (0x0200). Session H closed the
    // gap where the server only registered RPC + FireForget methods, so
    // getter/setter messages were dropped by vsomeip.
    motor_engine.received_.clear();
    {
        SCE::Mesh::MeshEnvelope env;
        env.id = SCE::uuid::v7();
        env.source = "test";
        env.type = "field.get.vehicle_speed";
        env.pattern = PK::FieldRead;
        env.datacontenttype = SCE::Mesh::PayloadCodec::None;

        MESH_TEST_REQUIRE(brake_router.route_send("#motor", env),
                          "brake route_send FieldRead returned false");
        MESH_TEST_REQUIRE(motor_engine.received_.wait_for([](const auto& v) {
                    return !v.empty() &&
                           v.back().type == "field.get.vehicle_speed";
                }),
                "motor engine did not receive FieldRead via SOME/IP server getter handler");
        {
            std::lock_guard<std::mutex> lock(motor_router.server_pending_mutex_);
            MESH_TEST_REQUIRE(!motor_router.pending_server_requests_.empty(),
                    "motor did not stash pending getter request for reply correlation");
        }
    }

    // ── 4. FieldWrite (setter): brake → motor setter handler → motor engine ─
    motor_engine.received_.clear();
    {
        SCE::Mesh::MeshEnvelope env;
        env.id = SCE::uuid::v7();
        env.source = "test";
        env.type = "field.set.target_speed";
        env.pattern = PK::FieldWrite;
        env.datacontenttype = SCE::Mesh::PayloadCodec::Json;
        std::string payload = R"({"value":55})";
        env.data.assign(payload.begin(), payload.end());

        MESH_TEST_REQUIRE(brake_router.route_send("#motor", env),
                          "brake route_send FieldWrite returned false");
        MESH_TEST_REQUIRE(motor_engine.received_.wait_for([](const auto& v) {
                    return !v.empty() &&
                           v.back().type == "field.set.target_speed";
                }),
                "motor engine did not receive FieldWrite via SOME/IP server setter handler");
        {
            std::lock_guard<std::mutex> lock(motor_engine.received_.m);
            MESH_TEST_REQUIRE(motor_engine.received_.events.back().data.find("55")
                                  != std::string::npos,
                    "FieldWrite payload did not survive SOME/IP CBOR round-trip");
        }
    }

    brake_router.shutdown();
    motor_router.shutdown();
    std::printf("SCE Mesh SOME/IP runtime E2E: PASS\n");
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
