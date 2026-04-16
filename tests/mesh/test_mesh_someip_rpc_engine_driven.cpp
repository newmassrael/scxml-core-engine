// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh Session I: engine-driven RPC / FieldAccess roundtrip over SOME/IP.
//
// Sibling to test_mesh_someip_runtime.cpp. That file binds motor to a
// TestSenderEngine recorder and manually calls
// motor_router.handleServerResponse() — transport-level coverage only.
// This file binds motor to the real motor_someip_multi SM and drives it
// with a step() pump, so the reply is produced by the engine's onentry
// <raise> and codegen-injected <send>.
//
// End-to-end chain exercised here:
//
//   brake route_send (env.invoke_id stamped, simulating mesh-rpc invoker)
//     → vsomeip request → motor register_message_handler
//         → stashes vsomeip message keyed by env.invoke_id
//         → dispatchToSender → motor.raiseExternal(EventWithMetadata)
//             → meta.invokeId = env.invoke_id
//     → driver thread motor.step()
//         → external queue dequeue sets currentEventInvokeId_
//         → transition: ready --service.request.compute_force--> computing
//         → computing onentry: <raise service.response.compute_force>
//                              + codegen-injected <send target="#motor">
//         → raiseExternal(Event, "", "", "#motor")
//             → isMeshTarget → performMeshSend → onMeshSend_
//             → TransportRouter's lambda: resolvePattern → RpcReply
//             → env.invoke_id = parsed(currentEventInvokeId_)
//             → env.correlation_id = env.invoke_id
//             → handleServerResponse → create_response → vsomeip send
//   → brake response handler → dispatchToSender → brake_engine observes
//
// Brake stays as TestSenderEngine: it is the test harness, not the
// system-under-test. The real SM is motor — if ANY link in the motor-side
// chain (transition fire, currentEventInvokeId_ propagation, injected
// <send> emission, resolvePattern, handleServerResponse correlation)
// regresses, the reply never reaches brake_engine.received_.
//
// Coverage:
//   §1 RPC roundtrip: service.request.compute_force → computing onentry
//                     → service.response.compute_force reply
//   §2 FieldRead roundtrip: field.get.vehicle_speed → ready transition
//                     raise → field.notify.vehicle_speed reply via
//                     FieldNotify pattern (create_response path)
//
// For FireForget / FieldWrite (no reply) see test_mesh_someip_runtime.cpp.

#include "brake_someip_multi_sm.h"
#include "brake_someip_multi_transport.h"
#include "motor_someip_multi_sm.h"
#include "motor_someip_multi_transport.h"

#include "MeshTestUtils.h"
#include "SomeipTestUtils.h"
#include "common/Uuid.h"

#include <atomic>
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
    using Motor = motor_gen::motor_someip_multi;
    using BrakeRouterT = brake_gen::TransportRouter<TestSenderEngine>;
    using MotorRouterT = motor_gen::TransportRouter<Motor>;

    wipe_stale_vsomeip_sockets();

    // Motor (server + routing manager) must initialize first. The real
    // SM enters <initial="ready"> before the vsomeip message handler
    // starts delivering; otherwise a racing request could land on an
    // engine that has not executed initial-state entry actions.
    Motor motor;
    motor.initialize();
    MotorRouterT motor_router(motor);
    MESH_TEST_REQUIRE(motor_router.init(), "motor router init failed");

    TestSenderEngine brake_engine;
    BrakeRouterT brake_router(brake_engine);
    MESH_TEST_REQUIRE(brake_router.init(), "brake router init failed");

    // RAII-scoped engine pump. See test_mesh_zenoh_rpc_engine_driven.cpp
    // for the detailed design rationale — identical pattern.
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

    // Wait for vsomeip to announce motor_control as available. Same
    // pattern as test_mesh_someip_runtime.cpp — register a
    // register_availability_handler that flips a condition variable.
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

    // ── §1. RPC roundtrip: engine-driven ──────────────────────────────
    //
    // env.invoke_id simulates an upstream mesh-rpc invoker. Motor's
    // register_message_handler keys pending_server_requests_ by this
    // UUID; the engine surfaces it through currentEventInvokeId_; the
    // mesh callback stamps it back as env.correlation_id on the reply,
    // which is what handleServerResponse looks up.
    //
    // Brake does NOT populate pending_rpcs_: in the engine-driven path
    // motor stamps env.type = "service.response.compute_force" directly
    // before encoding the response payload, so brake's response handler
    // dispatches the correct event even without the type-rewrite table
    // entry. (The pending_rpcs_ rewrite only matters when the wire
    // carries the original request event name — not the case here.)
    {
        SCE::Mesh::MeshEnvelope env;
        env.id = {};
        env.source = "test";
        env.type = "service.request.compute_force";
        env.pattern = PK::RpcRequest;
        env.datacontenttype = SCE::Mesh::PayloadCodec::Json;
        std::string payload = R"({"force":100})";
        env.data.assign(payload.begin(), payload.end());
        env.invoke_id = SCE::uuid::v7();

        MESH_TEST_REQUIRE(brake_router.route_send("#motor", env),
                          "brake route_send RpcRequest returned false");
        MESH_TEST_REQUIRE(brake_engine.received_.wait_for([](const auto& v) {
                    return !v.empty() &&
                           v.back().type == "service.response.compute_force";
                }),
                "engine-driven RPC reply not received — regression in "
                "transition→raise→injected-send→correlation chain");
        // Motor SCXML raises service.response without data, so the reply
        // envelope carries an empty payload — asserting on payload here
        // would be meaningless. Data survival is covered by the FireForget
        // test in test_mesh_someip_runtime.cpp, which uses a payloaded
        // event that IS observable through the recorder.
    }

    // ── §2. FieldRead roundtrip: engine-driven ────────────────────────
    //
    // field.get.vehicle_speed lands on motor's getter message handler
    // (SOMEIP_SERVER_GETTER_FIELD_GET_VEHICLE_SPEED), stashes the request
    // message, dispatches to the engine. Engine fires the ready
    // transition whose body is <raise event="field.notify.vehicle_speed"/>;
    // the injected <send> routes the reply through handleServerResponse
    // → create_response.
    brake_engine.received_.clear();
    {
        SCE::Mesh::MeshEnvelope env;
        env.id = {};
        env.source = "test";
        env.type = "field.get.vehicle_speed";
        env.pattern = PK::FieldRead;
        env.datacontenttype = SCE::Mesh::PayloadCodec::None;
        env.invoke_id = SCE::uuid::v7();

        MESH_TEST_REQUIRE(brake_router.route_send("#motor", env),
                          "brake route_send FieldRead returned false");
        MESH_TEST_REQUIRE(brake_engine.received_.wait_for([](const auto& v) {
                    return !v.empty() &&
                           v.back().type == "field.notify.vehicle_speed";
                }),
                "engine-driven FieldRead reply not received — regression "
                "in transition<raise>→FieldNotify correlation chain");
    }

    // Stop the pump eagerly so shutdown observes a quiet engine; the
    // RAII dtor would still cover any early-return path.
    pump.running.store(false, std::memory_order_release);
    pump.t.join();
    brake_router.shutdown();
    motor_router.shutdown();
    std::printf("SCE Mesh Session I SOME/IP engine-driven RPC/FieldRead: PASS\n");
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
