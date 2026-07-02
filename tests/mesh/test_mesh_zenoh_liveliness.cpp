// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh Zenoh liveliness runtime E2E (SCE Mesh §16.7 row 8).
//
// Verifies the PEER_PARTITIONED path end-to-end:
//   - brake_router.init() declares a liveliness token at
//     `sce/live/brake_zenoh_liveliness` and subscribes on `sce/live/**`.
//   - A raw zenoh peer in the test simulates motor; it declares a token
//     at `sce/live/motor` and holds it.
//   - The raw peer's session drops. Zenoh emits a DELETE sample for
//     `sce/live/motor`; brake's subscriber callback raises
//     `error.communication` with reason PEER_PARTITIONED, target
//     "motor", and last_seen_ms_ago > 0.
//
// Hermeticity: requires a local zenoh runtime — peer mode, no router.
// Port 17448 is serialized through ctest RESOURCE_LOCK so multiple
// test runs cannot interfere.

#include "brake_zenoh_liveliness_transport.h"

#include "ZenohTestUtils.h"

#include <chrono>
#include <cstdio>
#include <string>
#include <thread>

namespace {

using namespace SCE::Test::Mesh;

// Mirrors brake's connect endpoint (deploy_zenoh_liveliness.yaml ecu_brake),
// pinned to motor's ecu_motor listen. Putting the raw motor peer on the same
// address is what keeps the Zenoh routing state hermetic for this E2E.
constexpr const char *kListen = SCE::Generated::brake_zenoh_liveliness::ZENOH_CONNECT_ENDPOINTS[0];
// deploy_zenoh_liveliness.yaml lease_ms. The DELETE-sample delivery
// budget is `lease_ms + small zenoh-internal jitter`. Test tolerates up
// to 3× lease_ms before declaring flake, matching the pattern used in
// mesh_order_injection case 5.
constexpr int kLeaseMs = 200;

int run_test() {
    namespace brake_gen = SCE::Generated::brake_zenoh_liveliness;
    using RouterT = brake_gen::TransportRouter<TestSenderEngine>;

    TestSenderEngine sender;
    RouterT brake_router({&sender});

    // Bring up motor's raw peer first so brake's connect succeeds.
    // Motor declares only the liveliness token — no publisher, no
    // subscriber. The scope holding `motor_session` below is how the
    // test drops the token: letting `motor_session` go out of scope
    // emits the DELETE sample.
    auto motor_session = open_peer(/*connect=*/"", /*listen=*/kListen);
    auto motor_token = motor_session.liveliness_declare_token(zenoh::KeyExpr("sce/live/motor"));

    MESH_TEST_REQUIRE(brake_router.init(), "brake_router.init() failed");

    // Give zenoh time to propagate motor's token PUT to brake's
    // subscriber so peer_last_seen_ has a non-zero anchor; otherwise the
    // DELETE sample would race ahead of any PUT and last_seen_ms_ago
    // would be omitted (spec-allowed but not what this test proves).
    std::this_thread::sleep_for(std::chrono::milliseconds(kLeaseMs));

    // No error.communication should have surfaced while peers were both
    // alive. A premature raise here would indicate the subscriber is
    // misinterpreting PUT samples or not self-filtering on
    // `sce/live/brake_zenoh_liveliness`.
    {
        std::lock_guard<std::mutex> lk(sender.received_.m);
        for (const auto &ev : sender.received_.events) {
            MESH_TEST_REQUIRE(ev.type != "error.communication", "error.communication raised before motor dropped");
        }
    }

    // Undeclare motor's token. Zenoh emits a DELETE sample to every
    // intersecting liveliness subscriber, which includes brake's. The
    // session stays open so this isolates the signal-under-test from
    // session-teardown noise.
    std::move(motor_token).undeclare();

    // Observation window: lease_ms + generous jitter. Matches the
    // sleep sizing for mesh_order_injection case 5.
    const bool observed = sender.received_.wait_for(
        [](const auto &events) {
            for (const auto &ev : events) {
                if (ev.type != "error.communication") {
                    continue;
                }
                // Substring match keeps the test JSON-parser-free.
                // reason field check is the definitive signal; the
                // other extras are asserted below only when the
                // PEER_PARTITIONED envelope is in hand.
                if (ev.data.find("\"reason\":\"PEER_PARTITIONED\"") != std::string::npos) {
                    return true;
                }
            }
            return false;
        },
        std::chrono::seconds(5));

    MESH_TEST_REQUIRE(observed, "brake did not raise error.communication with "
                                "reason PEER_PARTITIONED within 5 s of motor drop");

    // Pin the extras: target "motor" (from the keyexpr suffix) and
    // last_seen_ms_ago present (because motor's PUT was observed during
    // the initial stabilization sleep above).
    std::lock_guard<std::mutex> lk(sender.received_.m);
    const auto *hit = static_cast<const ReceivedEvent *>(nullptr);
    for (const auto &ev : sender.received_.events) {
        if (ev.type == "error.communication" && ev.data.find("\"reason\":\"PEER_PARTITIONED\"") != std::string::npos) {
            hit = &ev;
        }
    }
    MESH_TEST_REQUIRE(hit, "expected event vanished between wait_for and inspection");
    MESH_TEST_REQUIRE(hit->data.find("\"target\":\"motor\"") != std::string::npos,
                      "PEER_PARTITIONED payload missing target=\"motor\"");
    MESH_TEST_REQUIRE(hit->data.find("\"last_seen_ms_ago\":") != std::string::npos,
                      "PEER_PARTITIONED payload missing last_seen_ms_ago "
                      "(PUT sample was not observed before DELETE)");

    brake_router.shutdown();
    std::printf("SCE Mesh zenoh liveliness PEER_PARTITIONED E2E: PASS\n");
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
