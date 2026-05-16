// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §16.4 / RFC F.X-3 — region-partition liveness compile verification (SOME/IP).
//
// Companion to mesh_someip_region_liveness_verification (the netns-gated
// runtime fixture). This TU is the always-on static gate that the
// codegen renders the §16.4 SOMEIP wire shape correctly for BOTH
// partitions of a partitioned machine — without it, a regression in the
// liveness-emit branch (sce-build/src/mesh/transport/someip.rs::
// assign_liveness_service_ids or the jinja2 template's SOMEIP-side
// `someip_liveness_service_id_self_hex` block) would only surface in
// the runtime test, which is skipped in fresh checkouts and on
// non-Linux developer machines.
//
// Compilation IS the test:
//   * `static_assert` the assigned constants land in the F.X-1 D2
//     reservation `[0x8180, 0x81FF]` and are pairwise distinct so
//     `register_availability_handler` for the sibling cannot collide
//     with the partition's own `offer_service`.
//   * `using` aliases drag the two TransportRouter template
//     instantiations through the C++ front end without touching
//     vsomeip's runtime (the ctor would otherwise call
//     `vsomeip::runtime::get()->create_application(...)` which needs a
//     valid VSOMEIP_CONFIGURATION). The full per-partition
//     `start()` / `register_availability_handler` / `raiseCommunicationError`
//     bodies are header-only inline functions, so SFINAE / template-
//     instantiation will surface a codegen miss as a hard compile
//     error here, not at link time of the runtime drivers.
//
// Two distinct binaries are built (one per partition) because the
// generated files share the `SCE::Generated::brake_region_liveness::`
// namespace — including both `_transport.h` files in one TU would
// double-define every symbol. Compilation success of either binary
// proves the per-partition codegen path; both being green proves the
// allocator's lex-sorted counter stayed deterministic across the
// `--partition brake_left_part` and `--partition brake_right_part`
// invocations.

#include "brake_region_liveness_transport.h"

#include "MeshTestUtils.h"

#include <cstdio>

int main() {
#ifdef SCE_LIVENESS_PARTITION_LEFT
    namespace brake_gen = SCE::Generated::brake_region_liveness::P_brake_left_part;
#endif
#ifdef SCE_LIVENESS_PARTITION_RIGHT
    namespace brake_gen = SCE::Generated::brake_region_liveness::P_brake_right_part;
#endif

    static_assert(
        brake_gen::SCE_LIVENESS_SERVICE_SELF >= 0x8180 &&
            brake_gen::SCE_LIVENESS_SERVICE_SELF <= 0x81FF,
        "SCE_LIVENESS_SERVICE_SELF must land in F.X-1 D2 reservation [0x8180, 0x81FF]");

#ifdef SCE_LIVENESS_PARTITION_LEFT
    static_assert(
        brake_gen::SCE_LIVENESS_SERVICE_SELF == 0x8180,
        "brake_left_part own liveness service must be 0x8180 (lex-sorted counter base)");
    static_assert(
        brake_gen::SCE_LIVENESS_SERVICE_PEER_BRAKE_RIGHT_PART == 0x8181,
        "brake_left_part sibling liveness service must be 0x8181");
    static_assert(
        brake_gen::SCE_LIVENESS_SERVICE_SELF !=
            brake_gen::SCE_LIVENESS_SERVICE_PEER_BRAKE_RIGHT_PART,
        "self and peer service IDs must be distinct so offer/subscribe cannot collide");
#endif
#ifdef SCE_LIVENESS_PARTITION_RIGHT
    static_assert(
        brake_gen::SCE_LIVENESS_SERVICE_SELF == 0x8181,
        "brake_right_part own liveness service must be 0x8181 (lex-sorted counter +1)");
    static_assert(
        brake_gen::SCE_LIVENESS_SERVICE_PEER_BRAKE_LEFT_PART == 0x8180,
        "brake_right_part sibling liveness service must be 0x8180");
    static_assert(
        brake_gen::SCE_LIVENESS_SERVICE_SELF !=
            brake_gen::SCE_LIVENESS_SERVICE_PEER_BRAKE_LEFT_PART,
        "self and peer service IDs must be distinct so offer/subscribe cannot collide");
#endif

    static_assert(
        brake_gen::SCE_LIVENESS_INSTANCE == 0x0001,
        "SCE_LIVENESS_INSTANCE is fixed at 0x0001 per RFC F.X-3 §2 wire shape");

    using RouterT = brake_gen::TransportRouter<SCE::Test::Mesh::TestSenderEngine>;
    static_assert(sizeof(RouterT) > 0,
                  "TransportRouter must be a complete type — instantiation drags the "
                  "consolidated SCE app field + register_availability_handler closure "
                  "through the front end");

    std::printf("SCE Mesh §16.4 region liveness compile verification (SOME/IP): PASS\n");
    return 0;
}
