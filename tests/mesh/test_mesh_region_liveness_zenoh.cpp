// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §16.4 / §16.7 row 13 — region-partition liveness runtime
// E2E (Zenoh transport).
//
// Orthogonal counterpart to `test_mesh_zenoh_liveliness.cpp` (row 8).
// Verifies the `REGION_PARTITIONED` raise path end-to-end:
//   - brake_region_liveness is compiled with `--partition
//     brake_left_part`. The generated router declares two liveliness
//     tokens — the machine-level `sce/live/brake_region_liveness`
//     (row 8 axis, unchanged) and the partition-level
//     `sce/live/brake_region_liveness/brake_left_part` (row 13 axis,
//     new). The subscriber listens on `sce/live/**` and
//     disambiguates segment count.
//   - A raw zenoh peer in the test simulates the sibling
//     `brake_right_part` partition; it declares a token at
//     `sce/live/brake_region_liveness/brake_right_part` and holds
//     it.
//   - The raw peer's token is undeclared. Zenoh emits a DELETE
//     sample for the 3-segment key; brake's subscriber callback
//     raises `error.communication` with reason
//     `REGION_PARTITIONED`, machine `brake_region_liveness`,
//     partition `brake_right_part`, and last_seen_ms_ago > 0.
//   - Bonus: PEER_PARTITIONED (row 8) must NOT fire in this path —
//     the sibling peer's machine-level token is never declared, so
//     only the partition DELETE is observed. This pins the
//     orthogonality the spec promises.
//
// Hermeticity: requires a local zenoh runtime — peer mode, no
// router. Port 17456 (distinct from 17447-17455 used by existing
// zenoh runtime fixtures) is serialized through ctest RESOURCE_LOCK
// so multiple test runs cannot interfere.

#include "brake_region_liveness_transport.h"

#include "ZenohTestUtils.h"

#include <chrono>
#include <cstdio>
#include <string>
#include <thread>

namespace {

using namespace SCE::Test::Mesh;

// Mirrors brake's connect endpoint (deploy_zenoh_region_liveness.yaml
// ecu_brake), pinned to the peer-side listen on the same address. Raw
// peer in this test binds the listen so brake's generated Zenoh
// session can connect and hear the wildcard subscriber samples.
constexpr const char* kListen =
    SCE::Generated::brake_region_liveness::P_brake_left_part::ZENOH_CONNECT_ENDPOINTS[0];

// deploy_zenoh_region_liveness.yaml lease_ms. DELETE-sample delivery
// budget is `lease_ms + small zenoh-internal jitter`. 3× lease_ms
// matches the row-8 sibling test (`test_mesh_zenoh_liveliness.cpp`).
constexpr int kLeaseMs = 200;

int run_test() {
    namespace brake_gen = SCE::Generated::brake_region_liveness::P_brake_left_part;
    using RouterT = brake_gen::TransportRouter<TestSenderEngine>;

    TestSenderEngine sender;
    RouterT brake_router({&sender});

    // Bring up the simulated sibling partition FIRST so brake's
    // connect succeeds and the wildcard subscriber populates
    // `peer_partition_last_seen_` with a non-zero anchor before the
    // DELETE sample arrives. Without the PUT-before-DELETE sequence,
    // `last_seen_ms_ago` would be spec-allowed but absent, obscuring
    // the full row-13 shape the test proves.
    auto sibling_session = open_peer(/*connect=*/"", /*listen=*/kListen);

    // 3-segment key: `sce/live/<machine>/<partition>`. Machine =
    // `brake_region_liveness` (matches the deploy.yaml machine name);
    // partition = `brake_right_part` (the sibling to the one this
    // binary represents). The test never declares a 2-segment token
    // for this peer — row 8 must not fire for the sibling partition.
    auto sibling_partition_token = sibling_session.liveliness_declare_token(
        zenoh::KeyExpr("sce/live/brake_region_liveness/brake_right_part"));

    MESH_TEST_REQUIRE(brake_router.init(), "brake_router.init() failed");

    // Give zenoh time to propagate the sibling partition's PUT to
    // brake's subscriber so `peer_partition_last_seen_` has a
    // non-zero anchor. If the DELETE races ahead of the PUT,
    // `last_seen_ms_ago` is omitted — spec-legal but this test
    // deliberately exercises the with-anchor branch.
    std::this_thread::sleep_for(std::chrono::milliseconds(kLeaseMs));

    // No error.communication should have surfaced while the sibling
    // partition was alive. A premature raise here would indicate the
    // subscriber is misinterpreting PUT samples or not self-filtering
    // on either of brake's own two tokens
    // (`sce/live/brake_region_liveness` or
    // `sce/live/brake_region_liveness/brake_left_part`).
    {
        std::lock_guard<std::mutex> lk(sender.received_.m);
        for (const auto& ev : sender.received_.events) {
            MESH_TEST_REQUIRE(ev.type != "error.communication",
                              "error.communication raised before sibling partition dropped");
        }
    }

    // Undeclare the sibling partition token. Zenoh emits a DELETE
    // sample on the 3-segment key; brake's subscriber disambiguates
    // segment count and routes to the REGION_PARTITIONED raise path.
    // The sibling's Zenoh session stays open so this isolates the
    // signal-under-test from session-teardown noise.
    std::move(sibling_partition_token).undeclare();

    // Observation window: lease_ms + generous jitter. Matches the
    // row-8 sibling test's sleep sizing.
    const bool observed = sender.received_.wait_for(
        [](const auto& events) {
            for (const auto& ev : events) {
                if (ev.type != "error.communication") continue;
                if (ev.data.find("\"reason\":\"REGION_PARTITIONED\"") != std::string::npos) {
                    return true;
                }
            }
            return false;
        },
        std::chrono::seconds(5));

    MESH_TEST_REQUIRE(observed,
                      "brake did not raise error.communication with "
                      "reason REGION_PARTITIONED within 5 s of sibling "
                      "partition drop");

    // Pin the extras: machine `brake_region_liveness`, partition
    // `brake_right_part`, and last_seen_ms_ago present.
    std::lock_guard<std::mutex> lk(sender.received_.m);
    const ReceivedEvent* hit = nullptr;
    for (const auto& ev : sender.received_.events) {
        if (ev.type == "error.communication" &&
            ev.data.find("\"reason\":\"REGION_PARTITIONED\"") != std::string::npos) {
            hit = &ev;
        }
    }
    MESH_TEST_REQUIRE(hit, "expected event vanished between wait_for and inspection");
    MESH_TEST_REQUIRE(
        hit->data.find("\"machine\":\"brake_region_liveness\"") != std::string::npos,
        "REGION_PARTITIONED payload missing machine=\"brake_region_liveness\"");
    MESH_TEST_REQUIRE(
        hit->data.find("\"partition\":\"brake_right_part\"") != std::string::npos,
        "REGION_PARTITIONED payload missing partition=\"brake_right_part\"");
    MESH_TEST_REQUIRE(
        hit->data.find("\"last_seen_ms_ago\":") != std::string::npos,
        "REGION_PARTITIONED payload missing last_seen_ms_ago "
        "(PUT sample was not observed before DELETE)");

    // §16.4 orthogonality pin: PEER_PARTITIONED (row 8) must NOT
    // fire on this trace. The sibling peer never declared a 2-
    // segment token, so no row-8 signal is legitimately available.
    // If row-8 raised anyway, the subscriber's segment-count
    // discrimination is broken — row 13 would be masquerading as
    // row 8 or the self-filter is off.
    for (const auto& ev : sender.received_.events) {
        MESH_TEST_REQUIRE(
            ev.data.find("\"reason\":\"PEER_PARTITIONED\"") == std::string::npos,
            "PEER_PARTITIONED fired on a region-partition trace — "
            "segment-count discrimination regression");
    }

    brake_router.shutdown();
    std::printf("SCE Mesh §16.4 region liveliness REGION_PARTITIONED E2E: PASS\n");
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
