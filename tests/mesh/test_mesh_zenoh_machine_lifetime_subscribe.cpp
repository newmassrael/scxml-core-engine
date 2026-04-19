// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh Z5a machine-lifetime subscription runtime E2E.
//
// Closes gap Z5a (mesh_zenoh_gaps_roadmap.md): deploy.yaml
// `machines.<name>.subscriptions:` now reaches the transport layer
// on a subscriptions-only machine. Before the topology synthesis
// landed, a brake that declared a subscription but issued no SCXML
// <send> to the subscription source had no matching `route_send`
// arm, so the init()-time subscribe envelope dropped into a no-op.
// This fixture pins the post-fix behaviour: the generated router
// declares the Zenoh subscriber purely from deploy.yaml, and a burst
// of distinct envelopes published on the matching key all reach the
// TestSenderEngine.
//
// Shape:
//   - brake_zenoh_machine_lifetime_subscribe.scxml carries ZERO <send>s;
//     the transition on event.notification.status is the only hook
//     the sender engine exposes for inbound delivery.
//   - router.init() emits the machine-lifetime subscribe through
//     route_send("#motor", sub_env), synthesised into the zenoh arm
//     by build-time target synthesis (SCE_MESH.md §13, Z5a).
//   - A raw zenoh::Session acting as motor publishes five distinct
//     event.notification.status envelopes at 30 ms cadence.
//   - The TestSenderEngine observes all five — drop count must be
//     zero for the machine-lifetime column of the §13 trade-off
//     table to reflect delivered semantics.

#include "brake_zenoh_machine_lifetime_subscribe_transport.h"

#include "ZenohTestUtils.h"
#include "mesh/MeshEnvelopeCodec.h"

#include <chrono>
#include <cstdio>
#include <string>
#include <thread>

namespace {

using namespace SCE::Test::Mesh;

// Mirrors brake's connect endpoint, pinned in deploy_zenoh_machine_lifetime_subscribe.yaml
// to motor's ecu_motor listen. The raw motor peer co-locates on the same
// address so the Zenoh handshake converges deterministically.
constexpr const char* kListen =
    SCE::Generated::brake_zenoh_machine_lifetime_subscribe::ZENOH_CONNECT_ENDPOINTS[0];
// Must match deploy_zenoh_machine_lifetime_subscribe.yaml binding key for #motor.
constexpr const char* kMotorKey = "sce/brake_lifetime/motor/status";

constexpr int kBurstCount = 5;
constexpr auto kBurstCadence = std::chrono::milliseconds(30);
constexpr auto kSubscribePropagation = std::chrono::milliseconds(200);

int run_test() {
    namespace brake_gen = SCE::Generated::brake_zenoh_machine_lifetime_subscribe;
    using RouterT = brake_gen::TransportRouter<TestSenderEngine>;

    TestSenderEngine sender;
    RouterT router({&sender});

    // Bring up the listener first so brake's connect succeeds.
    auto motor_session = open_peer(/*connect=*/"", /*listen=*/kListen);

    // The subscribe is authored at init()-time, via deploy.yaml-driven
    // synthesis rather than a manual send_zenoh() call. If Z5a
    // regresses, init() still returns true but route_send("#motor", env)
    // drops silently — the failure surfaces later when the motor
    // envelopes never reach the TestSenderEngine.
    MESH_TEST_REQUIRE(router.init(), "router.init() failed");

    // Deterministic peer-mesh handshake: the harness session joins the
    // same listen endpoint and exchanges a liveliness token with motor.
    // Same pattern as test_mesh_zenoh_base_subscribe.cpp.
    auto handshake_session = open_peer(/*connect=*/kListen, /*listen=*/"");
    wait_for_peer_ready(motor_session, handshake_session);

    // Propagation grace — declare_subscriber runs inside route_send
    // during init(), but Zenoh's routing table convergence is
    // asynchronous. 200 ms matches the base_subscribe fixture.
    std::this_thread::sleep_for(kSubscribePropagation);

    // Publish five distinct envelopes on the subscribed key.
    // Distinctness verifies this is not a single-sample fluke — each
    // envelope carries a fresh UUID via make_envelope, so dedup cannot
    // collapse them, and each arrives through the same subscriber
    // callback declared at init() time.
    for (int i = 0; i < kBurstCount; ++i) {
        auto notify = make_envelope("event.notification.status",
                                    SCE::Mesh::PatternKind::FireForget);
        auto bytes = SCE::Mesh::encodeEnvelope(notify);
        motor_session.put(zenoh::KeyExpr(kMotorKey), zenoh::Bytes(std::move(bytes)));
        if (i + 1 < kBurstCount) {
            std::this_thread::sleep_for(kBurstCadence);
        }
    }

    MESH_TEST_REQUIRE(
        sender.received_.wait_for([](const auto& v) {
            if (v.size() < static_cast<std::size_t>(kBurstCount)) {
                return false;
            }
            for (const auto& ev : v) {
                if (ev.type != "event.notification.status") {
                    return false;
                }
            }
            return true;
        }),
        "machine-lifetime subscription did not deliver the full burst "
        "through route_send synthesised from deploy.yaml");

    router.shutdown();
    std::printf("SCE Mesh zenoh machine-lifetime subscribe runtime E2E: PASS\n");
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
