// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh DDS multi-pattern verification.
//
// The orthogonal sibling of mesh_zenoh_multipattern and
// mesh_tcp_multipattern_verification: the SCXML shape is held constant and
// only the transport changes, so anything that differs here is attributable
// to DDS rather than to the state machines.
//
// Two TransportRouters in one process stand in for two devices. That is
// sound on DDS specifically because every reader carries
// IGNORE_LOCAL(participant) and each router owns its own participant: a
// device never reads its own writes, so co-locating them does not create a
// path the deployed topology would not have.
//
// Coverage:
//   §1 RpcRequest  — brake's request reaches motor's engine, motor's
//                    injected reply returns on the derived `_Reply` topic
//   §2 FieldRead   — same paired leg, reached through field.get
//   §3 PubSub      — a subscribed client receives an eventgroup notify, and
//                    transient-local hands it to a client that subscribed
//                    after the publish

#include "brake_dds_multi_sm.h"
#include "brake_dds_multi_transport.h"
#include "motor_dds_multi_sm.h"
#include "motor_dds_multi_transport.h"

#include "MeshTestUtils.h"
#include "common/Uuid.h"

#include <atomic>
#include <chrono>
#include <cstdio>
#include <thread>

namespace {

using namespace SCE::Test::Mesh;
using namespace std::chrono_literals;

int run_test() {
    namespace brake_gen = SCE::Generated::brake_dds_multi;
    namespace motor_gen = SCE::Generated::motor_dds_multi;
    using PK = SCE::Mesh::PatternKind;
    using Motor = motor_gen::motor_dds_multi;

    // Motor first: the server has to be discoverable before brake's
    // request writer can match anything.
    Motor motor;
    motor.initialize();
    motor_gen::TransportRouter<Motor> motor_router({&motor});
    MESH_TEST_REQUIRE(motor_router.init(), "motor router init failed");

    TestSenderEngine brake_engine;
    brake_gen::TransportRouter<TestSenderEngine> brake_router({&brake_engine});
    MESH_TEST_REQUIRE(brake_router.init(), "brake router init failed");

    // Engine pump: the server's reply is produced by a transition and the
    // codegen-injected <send>, both of which run on step().
    struct EnginePump {
        std::atomic<bool> running{true};
        std::thread t;

        ~EnginePump() {
            running.store(false, std::memory_order_release);
            if (t.joinable()) {
                t.join();
            }
        }
    } pump;

    pump.t = std::thread([&motor, &pump] {
        while (pump.running.load(std::memory_order_acquire)) {
            motor.step();
            std::this_thread::sleep_for(1ms);
        }
    });

    // ── §1. RpcRequest roundtrip ──────────────────────────────────────
    {
        SCE::Mesh::MeshEnvelope env;
        env.id = SCE::uuid::v7();
        env.source = "test";
        env.type = "service.request.compute_force";
        env.pattern = PK::RpcRequest;
        env.datacontenttype = SCE::Mesh::PayloadCodec::Json;
        std::string payload = R"({"force":100})";
        env.data.assign(payload.begin(), payload.end());

        MESH_TEST_REQUIRE(brake_router.route_send("#motor", env), "brake route_send RpcRequest returned false");
        MESH_TEST_REQUIRE(brake_engine.received_.wait_for([](const auto &v) {
            return !v.empty() && v.back().type == "service.response.compute_force";
        }),
                          "RPC reply never returned on the derived _Reply topic");
    }

    // ── §2. FieldRead roundtrip ───────────────────────────────────────
    brake_engine.received_.clear();
    {
        SCE::Mesh::MeshEnvelope env;
        env.id = SCE::uuid::v7();
        env.source = "test";
        env.type = "field.get.position";
        env.pattern = PK::FieldRead;
        env.datacontenttype = SCE::Mesh::PayloadCodec::None;

        MESH_TEST_REQUIRE(brake_router.route_send("#motor", env), "brake route_send FieldRead returned false");
        MESH_TEST_REQUIRE(brake_engine.received_.wait_for(
                              [](const auto &v) { return !v.empty() && v.back().type == "field.notify.position"; }),
                          "FieldRead reply never returned — the paired reply leg is the same one "
                          "RpcRequest uses, so this failing alone means the pattern arm diverged");
    }

    // ── §3. PubSub: subscribe, then receive a spontaneous notify ──────
    brake_engine.received_.clear();
    {
        SCE::Mesh::MeshEnvelope sub;
        sub.id = SCE::uuid::v7();
        sub.source = "test";
        sub.type = "event.subscribe.position";
        sub.pattern = PK::EventSubscribe;
        MESH_TEST_REQUIRE(brake_router.route_send("#motor", sub), "subscribe returned false");

        // A subscribe on DDS creates the notification reader; the publisher
        // learns of it through discovery, so the notify has to wait for that
        // match rather than for an acknowledgement message.
        std::this_thread::sleep_for(500ms);

        SCE::Mesh::MeshEnvelope notify;
        notify.id = SCE::uuid::v7();
        notify.source = "motor";
        notify.type = "field.notify.position";
        notify.pattern = PK::FieldNotify;
        MESH_TEST_REQUIRE(motor_router.publishEventgroupNotify(notify, 0), "publishEventgroupNotify returned false");

        MESH_TEST_REQUIRE(brake_engine.received_.wait_for(
                              [](const auto &v) { return !v.empty() && v.back().type == "field.notify.position"; }),
                          "subscriber never received the eventgroup notification");
    }

    pump.running.store(false, std::memory_order_release);
    pump.t.join();
    brake_router.shutdown();
    motor_router.shutdown();
    std::printf("SCE Mesh DDS multi-pattern: PASS\n");
    return 0;
}

}  // namespace

int main() {
    try {
        return run_test();
    } catch (const std::exception &ex) {
        std::fprintf(stderr, "FAIL: uncaught exception: %s\n", ex.what());
        return 1;
    }
}
