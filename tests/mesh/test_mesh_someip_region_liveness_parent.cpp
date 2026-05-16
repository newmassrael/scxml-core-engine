// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §16.4 / §16.7 row 13 — region-partition liveness E2E (SOME/IP), parent side.
//
// Hosts the `brake_left_part` SCE binary (sce-mesh-parent netns,
// 172.16.10.1). Brings up the consolidated SCE app
// `brake_region_liveness_brake_left_part_sce` whose codegen-emitted
// `register_availability_handler` watches the sibling brake_right_part's
// liveness service (0x8181). When the worker side shuts down (clean
// STOP_OFFER over SD multicast → cross-veth) the handler observes the
// `available=false` edge and raises `error.communication` with reason
// `REGION_PARTITIONED`, machine `brake_region_liveness`, partition
// `brake_right_part`. Parent observes the raise on its TestSenderEngine
// event log and exits 0; if the raise does not materialise within
// 8 s of init() (worker exits at ~2.5 s after LISTEN_READY, parent
// starts ~500 ms after LISTEN_READY, so parent has ~6 s of headroom)
// the test fails with a diagnostic naming the most likely SD-side
// causes.

#include "brake_region_liveness_transport.h"

#include "MeshTestUtils.h"
#include "SomeipTestUtils.h"

#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <string>
#include <thread>

#ifndef VSOMEIP_CONFIG_PATH
#error "VSOMEIP_CONFIG_PATH must be defined by CMake (path to vsomeip_someip_region_liveness_left.json)"
#endif

namespace {

bool received_region_partitioned(const auto& events_log) {
    for (const auto& ev : events_log) {
        if (ev.type != "error.communication") continue;
        if (ev.data.find("\"reason\":\"REGION_PARTITIONED\"") != std::string::npos) {
            return true;
        }
    }
    return false;
}

bool received_peer_partitioned(const auto& events_log) {
    for (const auto& ev : events_log) {
        if (ev.type != "error.communication") continue;
        if (ev.data.find("\"reason\":\"PEER_PARTITIONED\"") != std::string::npos) {
            return true;
        }
    }
    return false;
}

}  // namespace

int main() {
    setenv("VSOMEIP_CONFIGURATION", VSOMEIP_CONFIG_PATH, 1);

    SCE::Test::Mesh::wipe_stale_vsomeip_sockets();

    using namespace SCE::Test::Mesh;
    namespace brake_gen = SCE::Generated::brake_region_liveness::P_brake_left_part;
    using RouterT = brake_gen::TransportRouter<TestSenderEngine>;

    TestSenderEngine sender;
    RouterT router({&sender});
    MESH_TEST_REQUIRE(router.init(), "parent router.init() returned false");

    // Wait for worker's STOP_OFFER → handler `available=false` →
    // raiseCommunicationError → TestSenderEngine.received_. Worker self-
    // exits 2.5 s after its own LISTEN_READY, which is ~2.0 s after
    // parent's init(). The 8 s budget covers worst-case SD repetitions
    // (3 × repetitions_base_delay = 300 ms) plus generous CI jitter.
    const bool observed = sender.received_.wait_for(
        [](const auto& events) {
            return received_region_partitioned(events);
        },
        std::chrono::seconds(8));

    if (!observed) {
        std::fprintf(stderr,
            "FAIL: parent did not observe error.communication / "
            "REGION_PARTITIONED within 8 s of init(). Likely causes:\n"
            "  1. SOMEIP-SD never converged across the veth pair — check\n"
            "     that setup_crossdev_netns.sh added the multicast route\n"
            "     (224.0.0.0/4 default) and that worker's vsomeip RM\n"
            "     emitted the initial Offer for service 0x8181.\n"
            "  2. Worker did not call shutdown() cleanly — the codegen\n"
            "     handler only fires on the SD-loss edge; a hard kill\n"
            "     would force parent to wait for the 5 s ttl expiry.\n"
            "  3. Codegen emitted the wrong sibling SCE_LIVENESS_SERVICE_PEER_*\n"
            "     constant for brake_left_part — verify\n"
            "     SCE_LIVENESS_SERVICE_PEER_BRAKE_RIGHT_PART = 0x8181\n"
            "     in brake_region_liveness_transport.h.\n");
        return 1;
    }

    // Pin the row-13 payload extras: machine + partition naming match the
    // codegen template's `err.machine = \"{{ machine_name }}\"` /
    // `err.partition = \"{{ peer.partition }}\"` substitution.
    std::lock_guard<std::mutex> lk(sender.received_.m);
    const ReceivedEvent* hit = nullptr;
    for (const auto& ev : sender.received_.events) {
        if (ev.type == "error.communication" &&
            ev.data.find("\"reason\":\"REGION_PARTITIONED\"") != std::string::npos) {
            hit = &ev;
        }
    }
    MESH_TEST_REQUIRE(hit, "expected REGION_PARTITIONED event vanished between wait_for and inspection");
    MESH_TEST_REQUIRE(
        hit->data.find("\"machine\":\"brake_region_liveness\"") != std::string::npos,
        "REGION_PARTITIONED payload missing machine=\"brake_region_liveness\"");
    MESH_TEST_REQUIRE(
        hit->data.find("\"partition\":\"brake_right_part\"") != std::string::npos,
        "REGION_PARTITIONED payload missing partition=\"brake_right_part\"");

    // §16.4 row-orthogonality pin: row 8 PEER_PARTITIONED is deferred
    // for SOMEIP (RFC F.X-3 D10 / F.X-4 scope). It must NOT fire on a
    // row-13 trace — vsomeip's `(service, instance)` keying has no
    // segment-count ambiguity to disambiguate, so a stray row 8 raise
    // here would mean the codegen confused machine-axis with partition-
    // axis emission.
    MESH_TEST_REQUIRE(
        !received_peer_partitioned(sender.received_.events),
        "PEER_PARTITIONED fired on a row-13 SOMEIP trace — F.X-3 explicitly "
        "defers row 8 SOMEIP to F.X-4; this is a codegen axis-confusion regression");

    router.shutdown();
    std::printf(
        "SCE Mesh §16.4 region-partition liveness REGION_PARTITIONED E2E (SOME/IP): PASS\n");
    return 0;
}
