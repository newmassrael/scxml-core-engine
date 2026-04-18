// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh base zenoh EventSubscribe runtime E2E.
//
// Counterpart to mesh_zenoh_runtime (FireForget-only): verifies that
// PatternKind::EventSubscribe on the base zenoh codegen path actually
// wires declare_subscriber correctly. Previously the base fixture had
// zero EventSubscribe runtime coverage — mesh_zenoh_multipattern_runtime
// covered it on a different fixture.
//
// Shape:
//   - brake_zenoh_base_subscribe.scxml enters idle → <send event=
//     "event.subscribe.status" target="#motor"/> → the generated router
//     calls send_zenoh(EventSubscribe) which declares a subscriber on
//     the binding key.
//   - A raw zenoh::Session acting as motor publishes an
//     event.notification.status envelope on the same key.
//   - Brake's subscriber callback decodes, rewrites pattern to
//     EventNotify, and admitInbound → dispatchEnvelope raises the event
//     on the TestSenderEngine.
//   - Test waits for the TestSenderEngine to observe the notification.

#include "brake_zenoh_base_subscribe_transport.h"

#include "ZenohTestUtils.h"
#include "mesh/MeshEnvelopeCodec.h"

#include <chrono>
#include <cstdio>
#include <string>
#include <thread>

namespace {

using namespace SCE::Test::Mesh;

// Mirrors brake's connect endpoint, which deploy_zenoh_base_subscribe.yaml
// pins to motor's ecu_motor listen — co-locating the raw motor peer on the
// same address is what makes the handshake converge.
constexpr const char* kListen   =
    SCE::Generated::brake_zenoh_base_subscribe::ZENOH_CONNECT_ENDPOINTS[0];
// Must match deploy_zenoh_base_subscribe.yaml binding key for #motor.
constexpr const char* kMotorKey = "sce/brake_base_subscribe/motor/status";

int run_test() {
    namespace brake_gen = SCE::Generated::brake_zenoh_base_subscribe;
    using RouterT = brake_gen::TransportRouter<TestSenderEngine>;

    TestSenderEngine sender;
    RouterT router(sender);

    // Bring up the listener first so brake's connect succeeds.
    auto motor_session = open_peer(/*connect=*/"", /*listen=*/kListen);

    MESH_TEST_REQUIRE(router.init(), "router.init() failed");

    // Deterministic peer-mesh handshake: the harness session joins the
    // same listen endpoint and exchanges a liveliness token with motor.
    // The router, motor, and harness share the kListen listener, so
    // convergence of the raw-session pair proves the router↔motor edge
    // has also converged.
    auto handshake_session = open_peer(/*connect=*/kListen, /*listen=*/"");
    wait_for_peer_ready(motor_session, handshake_session);

    // Drive brake's subscribe side directly via send_zenoh. Using the
    // router API (not the SM step) isolates this test to the transport
    // layer: resolvePattern is a static function, so the specific
    // pattern for the key event was already validated in the build-time
    // compile test; here we only care that declare_subscriber wires the
    // wire path.
    auto sub_env = make_envelope("event.subscribe.status",
                                 SCE::Mesh::PatternKind::EventSubscribe);
    MESH_TEST_REQUIRE(router.send_zenoh(sub_env, kMotorKey, "#motor"),
                      "send_zenoh EventSubscribe returned false");

    // Allow declare_subscriber to propagate.
    std::this_thread::sleep_for(std::chrono::milliseconds(200));

    // Motor-side publish — a raw put on the same key.
    auto notify = make_envelope("event.notification.status",
                                SCE::Mesh::PatternKind::FireForget);
    auto bytes = SCE::Mesh::encodeEnvelope(notify);
    motor_session.put(zenoh::KeyExpr(kMotorKey), zenoh::Bytes(std::move(bytes)));

    MESH_TEST_REQUIRE(sender.received_.wait_for([](const auto& v) {
                return !v.empty() &&
                       v.back().type == "event.notification.status";
            }),
            "sender engine did not see 'event.notification.status' "
            "through base zenoh subscriber path");

    router.shutdown();
    std::printf("SCE Mesh zenoh base subscribe runtime E2E: PASS\n");
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
