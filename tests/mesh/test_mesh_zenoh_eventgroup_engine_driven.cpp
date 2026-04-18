// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh Session J: engine-driven eventgroup notification over Zenoh.
//
// Verifies the server-initiated publish path (session.put on the server
// key). Motor receives a "sensor.update" event (no preceding getter/
// setter request), raises field.notify.position spontaneously, and the
// mesh send callback falls through handleServerResponse (no pending
// query to correlate) to publishEventgroupNotify which calls
// session.put on ZENOH_SERVER_KEY. A client subscriber on that key
// receives the notification.
//
// End-to-end chain exercised:
//
//   test dispatches sensor.update → motor engine
//   → driver thread motor.step()
//     → transition: ready --sensor.update--> ready
//     → <raise event="field.notify.position"/>
//       + codegen-injected <send target="#motor">
//     → raiseExternal(Event, "", "", "#motor")
//       → performMeshSend → mesh_send_cb_ → resolvePattern → FieldNotify
//       → handleServerResponse fails (no pending query)
//       → publishEventgroupNotify → session.put(ZENOH_SERVER_KEY, encoded)
//   → client subscriber callback → decodes → notification observed
//
// Motor uses the real SM driven by a step() pump (Session I pattern).

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

    // Bring up the real SM first.
    Motor motor;
    motor.initialize();

    MotorRouterT motor_router(motor);
    MESH_TEST_REQUIRE(motor_router.init(), "motor_router.init() failed");

    // RAII-scoped engine pump (Session I pattern).
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

    // Client session. Peer mode, connects to the motor's listen endpoint.
    auto client_session = open_peer(/*connect=*/kListen, /*listen=*/"");

    // Deterministic convergence barrier. The motor router's init()
    // declared a queryable on ZENOH_SERVER_KEY; once the client-side
    // matching listener fires the client↔motor peer-mesh routing
    // state has converged, which includes the subscriber path this
    // test relies on (motor `session.put` → client subscriber).
    wait_for_queryable(client_session, motor_gen::ZENOH_SERVER_KEY);

    // ── §1. Eventgroup notification: engine-driven ───────────────
    //
    // Subscribe to the motor's server key. Then dispatch sensor.update
    // directly to the motor engine. Motor's transition raises
    // field.notify.position spontaneously → publishEventgroupNotify →
    // session.put → subscriber callback receives the notification.
    ReceivedEvents notify_events;
    auto subscriber = client_session.declare_subscriber(
        zenoh::KeyExpr(motor_gen::ZENOH_SERVER_KEY),
        [&notify_events](const zenoh::Sample& sample) {
            auto bytes = sample.get_payload().as_vector();
            SCE::Mesh::MeshEnvelope env;
            if (!SCE::Mesh::decodeEnvelope(bytes.data(), bytes.size(), env)) return;
            // Only collect field.notify events (ignore any other traffic
            // on the server key — e.g. queryable replies).
            if (env.type.rfind("field.notify.", 0) == 0) {
                notify_events.push({env.type,
                                    std::string(env.data.begin(), env.data.end())});
            }
        },
        [] {});

    // Allow subscriber declaration to propagate.
    std::this_thread::sleep_for(std::chrono::milliseconds(200));

    // Dispatch sensor.update directly to the motor engine — no transport
    // roundtrip needed (we are testing the server-side publish path, not
    // the client→server inbound path).
    {
        SCE::Mesh::MeshEnvelope trigger;
        trigger.id = SCE::uuid::v7();
        trigger.source = "test";
        trigger.type = "sensor.update";
        trigger.pattern = PK::FireForget;
        trigger.datacontenttype = SCE::Mesh::PayloadCodec::None;
        (void)motor_router.dispatchToSender(trigger);
    }

    MESH_TEST_REQUIRE(notify_events.wait_for([](const auto& v) {
                return !v.empty() &&
                       v.back().type == "field.notify.position";
            }),
            "eventgroup notification not received — regression in "
            "spontaneous raise→injected-send→publishEventgroupNotify chain");

    // Teardown: stop pump before transport shutdown.
    pump.running.store(false, std::memory_order_release);
    pump.t.join();
    motor_router.shutdown();
    std::printf("SCE Mesh Session J zenoh eventgroup notification: PASS\n");
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
