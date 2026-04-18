// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh Session G: base zenoh single-pattern runtime E2E.
//
// Counterpart to the multi-pattern tests: brake.scxml sends a single
// FireForget event (brake.activate) through the generated zenoh router
// to a raw listener session. Smallest possible runtime proof that
// deploy_zenoh.yaml's transports.zenoh.connect wiring drives
// TransportRouter::init() end-to-end.
//
// Coverage (matches capabilities present in brake.scxml + deploy_zenoh.yaml):
//   - FireForget:  brake send → zenoh session.put → raw subscriber receives CBOR envelope
//
// The multi-pattern / server / rpc_e2e tests cover the remaining
// PatternKind variants against the larger brake_zenoh_multi fixture;
// this file exists to close the runtime-coverage gap for the base
// compile verification (Test #17) that previously had no runtime peer.

#include "brake_transport.h"

#include "ZenohTestUtils.h"

#include <cstdio>
#include <string>
#include <thread>

namespace {

using namespace SCE::Test::Mesh;

// Must match deploy_zenoh.yaml ecu1 transports.zenoh.connect. The test's
// raw listener binds this address so brake's generated init() dials it.
constexpr const char* kListen   = "tcp/127.0.0.1:17447";
// Must match deploy_zenoh.yaml ecu1.machines.brake.bindings.#motor.key.
constexpr const char* kMotorKey = "sce/brake/motor/cmd";

int run_test() {
    namespace brake_gen = SCE::Generated::brake;
    using RouterT = brake_gen::TransportRouter<TestSenderEngine>;

    TestSenderEngine sender;
    RouterT router(sender);

    // Raw receiver up first so the endpoint is accepting before brake dials.
    auto motor_session = open_peer(/*connect=*/"", /*listen=*/kListen);
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

    MESH_TEST_REQUIRE(router.init(), "router.init() failed");

    // Deterministic peer-mesh handshake via a liveliness token exchanged
    // with a throwaway peer. Motor listens at kListen; the handshake
    // peer dials it, and so does brake's router from inside init(). Once
    // motor↔handshake has converged, motor↔brake_router has too.
    auto handshake_session = open_peer(/*connect=*/kListen, /*listen=*/"");
    wait_for_peer_ready(motor_session, handshake_session);

    auto env = make_envelope("brake.activate", SCE::Mesh::PatternKind::FireForget);
    MESH_TEST_REQUIRE(router.send_zenoh(env, kMotorKey, "#motor"),
                      "send_zenoh FireForget returned false");
    MESH_TEST_REQUIRE(motor_inbox.wait_for([](const auto& v) { return !v.empty(); }),
                      "motor subscriber did not receive FireForget envelope");

    {
        std::lock_guard<std::mutex> lk(motor_inbox.m);
        MESH_TEST_REQUIRE(motor_inbox.envelopes.front().type == "brake.activate",
                          "FireForget envelope type mismatch on motor side");
        MESH_TEST_REQUIRE(motor_inbox.envelopes.front().pattern ==
                              SCE::Mesh::PatternKind::FireForget,
                          "FireForget envelope pattern not preserved");
    }

    router.shutdown();
    std::printf("SCE Mesh zenoh base runtime E2E: PASS\n");
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
