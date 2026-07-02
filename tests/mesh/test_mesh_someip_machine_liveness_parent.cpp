// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §16.7 row 8 — machine-level liveness E2E (SOME/IP), parent side.
//
// Hosts the `alpha_machine_liveness` SCE binary (sce-mesh-parent netns,
// 172.16.10.1). Brings up the consolidated SCE app
// `alpha_machine_liveness_sce` (RFC F.X-2 single-partition shape — no
// `_<partition>` infix because alpha has no `partitions:` block) whose
// codegen-emitted `register_availability_handler` watches the peer
// machine beta's liveness service (0x8281). When the worker shuts down
// (clean STOP_OFFER over SD multicast → cross-veth) the handler observes
// `available=false` and raises `error.communication` with reason
// `PEER_PARTITIONED`, target `beta_machine_liveness`. Parent observes
// the raise on its TestSenderEngine event log and exits 0; if the raise
// does not materialise within 8 s of init() the test fails with a
// diagnostic naming the most likely SD-side causes.
//
// Single-partition shape is the load-bearing F.X-4 D4-shape-1 acceptance
// distinguishing this fixture from F.X-3: alpha and beta have NO
// `partitions:` blocks, so row 13 REGION_PARTITIONED CANNOT fire here
// (zero F.X-3 region-liveness participants under the assigner's ≥2
// sibling-partition gate). A row-13 raise on this trace would mean the
// codegen confused the machine-liveness axis with the region-liveness
// axis emission — orthogonality assertion below pins this.

#include "alpha_machine_liveness_transport.h"

#include "MeshTestUtils.h"
#include "SomeipTestUtils.h"

#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <string>
#include <thread>

#ifndef VSOMEIP_CONFIG_PATH
#error "VSOMEIP_CONFIG_PATH must be defined by CMake (path to vsomeip_someip_machine_liveness_alpha.json)"
#endif

namespace {

bool received_peer_partitioned(const auto &events_log) {
    for (const auto &ev : events_log) {
        if (ev.type != "error.communication") {
            continue;
        }
        if (ev.data.find("\"reason\":\"PEER_PARTITIONED\"") != std::string::npos) {
            return true;
        }
    }
    return false;
}

bool received_region_partitioned(const auto &events_log) {
    for (const auto &ev : events_log) {
        if (ev.type != "error.communication") {
            continue;
        }
        if (ev.data.find("\"reason\":\"REGION_PARTITIONED\"") != std::string::npos) {
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
    namespace alpha_gen = SCE::Generated::alpha_machine_liveness;
    using RouterT = alpha_gen::TransportRouter<TestSenderEngine>;

    TestSenderEngine sender;
    RouterT router({&sender});
    MESH_TEST_REQUIRE(router.init(), "parent router.init() returned false");

    // Wait for worker's STOP_OFFER → handler `available=false` →
    // raiseCommunicationError → TestSenderEngine.received_. Worker self-
    // exits 2.5 s after its own LISTEN_READY, which is ~2.0 s after
    // parent's init(). The 8 s budget covers worst-case SD repetitions
    // (3 × repetitions_base_delay = 300 ms) plus generous CI jitter.
    const bool observed = sender.received_.wait_for(
        [](const auto &events) { return received_peer_partitioned(events); }, std::chrono::seconds(8));

    if (!observed) {
        std::fprintf(stderr, "FAIL: parent did not observe error.communication / "
                             "PEER_PARTITIONED within 8 s of init(). Likely causes:\n"
                             "  1. SOMEIP-SD never converged across the veth pair — check\n"
                             "     that setup_crossdev_netns.sh added the multicast route\n"
                             "     (224.0.0.0/4 default) and that worker's vsomeip RM\n"
                             "     emitted the initial Offer for service 0x8281.\n"
                             "  2. Worker did not call shutdown() cleanly — the codegen\n"
                             "     handler only fires on the SD-loss edge; a hard kill\n"
                             "     would force parent to wait for the 5 s ttl expiry.\n"
                             "  3. Codegen emitted the wrong peer-machine\n"
                             "     SCE_MACHINE_LIVENESS_SERVICE_PEER_* constant for alpha —\n"
                             "     verify SCE_MACHINE_LIVENESS_SERVICE_PEER_BETA_MACHINE_LIVENESS\n"
                             "     = 0x8281 in alpha_machine_liveness_transport.h.\n");
        return 1;
    }

    // Pin the row-8 payload extras: target naming matches the codegen
    // template's `err.target = \"{{ peer.machine }}\"` substitution.
    std::lock_guard<std::mutex> lk(sender.received_.m);
    const ReceivedEvent *hit = nullptr;
    for (const auto &ev : sender.received_.events) {
        if (ev.type == "error.communication" && ev.data.find("\"reason\":\"PEER_PARTITIONED\"") != std::string::npos) {
            hit = &ev;
        }
    }
    MESH_TEST_REQUIRE(hit, "expected PEER_PARTITIONED event vanished between wait_for and inspection");
    MESH_TEST_REQUIRE(hit->data.find("\"target\":\"beta_machine_liveness\"") != std::string::npos,
                      "PEER_PARTITIONED payload missing target=\"beta_machine_liveness\"");

    // §16.4 ↔ §16.7 row-orthogonality pin: under F.X-4 D4-shape-1 the
    // single-partition shape has zero F.X-3 region-liveness participants
    // (assigner narrowed to ≥2 sibling partitions per F.X-4 Stage B2),
    // so row 13 REGION_PARTITIONED MUST NOT fire on a row-8 trace. A
    // stray row-13 raise here would mean the codegen confused the
    // machine-axis with the partition-axis emission, or the assigner
    // narrowing regressed.
    MESH_TEST_REQUIRE(!received_region_partitioned(sender.received_.events),
                      "REGION_PARTITIONED fired on a row-8 SOMEIP trace — F.X-4 single-partition "
                      "shape must have zero row-13 participants; a row-13 raise here is a codegen "
                      "axis-confusion regression or an assigner-narrowing regression");

    router.shutdown();
    std::printf("SCE Mesh §16.7 row 8 machine-level liveness PEER_PARTITIONED E2E (SOME/IP): PASS\n");
    return 0;
}
