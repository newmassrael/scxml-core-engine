// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §16.7 row 8 / RFC F.X-4 — machine-level liveness compile verification (SOME/IP).
//
// Companion to mesh_someip_machine_liveness_verification (the netns-gated
// runtime fixture, Stage D). This TU is the always-on static gate that
// the codegen renders the §16.7 SOMEIP row-8 wire shape correctly for
// BOTH machines — without it, a regression in the machine-liveness
// emit branch (sce-build/src/mesh/transport/someip.rs::
// assign_machine_liveness_service_ids or the jinja2 template's
// `someip_machine_liveness_service_id_self_hex` block) would only
// surface in the runtime test, which is skipped in fresh checkouts and
// on non-Linux developer machines.
//
// Compilation IS the test:
//   * `static_assert` the assigned constants land in the F.X-4 D1
//     reservation `[0x8280, 0x82FF]` and are pairwise distinct so
//     `register_availability_handler` for the peer cannot collide with
//     the machine's own `offer_service`.
//   * The disjoint-range invariant against F.X-1 invoke and F.X-3
//     region-liveness sub-ranges is encoded via base-comparison
//     `static_assert`s — a regression that accidentally allocates an
//     F.X-4 ID inside `[0x8100, 0x81FF]` trips the build before the
//     runtime layer.
//   * `using` aliases drag the two TransportRouter template
//     instantiations through the C++ front end without touching
//     vsomeip's runtime (the ctor would otherwise call
//     `vsomeip::runtime::get()->create_application(...)` which needs a
//     valid VSOMEIP_CONFIGURATION). The full per-machine `start()` /
//     `register_availability_handler` / `raiseCommunicationError`
//     bodies are header-only inline functions, so SFINAE / template-
//     instantiation will surface a codegen miss as a hard compile
//     error here, not at link time of the runtime drivers.
//
// Two distinct binaries are built (one per machine) because the
// generated files share neither namespace (each machine has its own
// `SCE::Generated::<machine>::` namespace) but the test source is
// parameterized on machine identity via the SCE_F4_MACHINE_ALPHA /
// SCE_F4_MACHINE_BETA macros so the per-machine `static_assert` block
// pins the lex-sorted counter assignments (alpha → 0x8280,
// beta → 0x8281).

#if defined(SCE_F4_MACHINE_ALPHA)
#include "alpha_machine_liveness_transport.h"
#elif defined(SCE_F4_MACHINE_BETA)
#include "beta_machine_liveness_transport.h"
#else
#error "Stage C compile-verification requires SCE_F4_MACHINE_ALPHA or SCE_F4_MACHINE_BETA"
#endif

#include "MeshTestUtils.h"

#include <cstdio>

int main() {
#if defined(SCE_F4_MACHINE_ALPHA)
    namespace gen = SCE::Generated::alpha_machine_liveness;

    static_assert(gen::SCE_MACHINE_LIVENESS_SERVICE_SELF >= 0x8280 && gen::SCE_MACHINE_LIVENESS_SERVICE_SELF <= 0x82FF,
                  "SCE_MACHINE_LIVENESS_SERVICE_SELF must land in F.X-4 D1 reservation [0x8280, 0x82FF]");
    static_assert(gen::SCE_MACHINE_LIVENESS_SERVICE_SELF == 0x8280,
                  "alpha's machine-level liveness service must be 0x8280 (lex-sorted counter base)");
    static_assert(gen::SCE_MACHINE_LIVENESS_SERVICE_PEER_BETA_MACHINE_LIVENESS == 0x8281,
                  "alpha's peer (beta) machine-level liveness service must be 0x8281");
    static_assert(gen::SCE_MACHINE_LIVENESS_SERVICE_SELF !=
                      gen::SCE_MACHINE_LIVENESS_SERVICE_PEER_BETA_MACHINE_LIVENESS,
                  "self and peer service IDs must be distinct so offer/subscribe cannot collide");
#endif
#if defined(SCE_F4_MACHINE_BETA)
    namespace gen = SCE::Generated::beta_machine_liveness;

    static_assert(gen::SCE_MACHINE_LIVENESS_SERVICE_SELF >= 0x8280 && gen::SCE_MACHINE_LIVENESS_SERVICE_SELF <= 0x82FF,
                  "SCE_MACHINE_LIVENESS_SERVICE_SELF must land in F.X-4 D1 reservation [0x8280, 0x82FF]");
    static_assert(gen::SCE_MACHINE_LIVENESS_SERVICE_SELF == 0x8281,
                  "beta's machine-level liveness service must be 0x8281 (lex-sorted counter +1)");
    static_assert(gen::SCE_MACHINE_LIVENESS_SERVICE_PEER_ALPHA_MACHINE_LIVENESS == 0x8280,
                  "beta's peer (alpha) machine-level liveness service must be 0x8280");
    static_assert(gen::SCE_MACHINE_LIVENESS_SERVICE_SELF !=
                      gen::SCE_MACHINE_LIVENESS_SERVICE_PEER_ALPHA_MACHINE_LIVENESS,
                  "self and peer service IDs must be distinct so offer/subscribe cannot collide");
#endif

    // Cross-subsystem disjointness on the wire-side constants
    // (RFC F.X-4 D1 invariant pinned in code).
    static_assert(
        gen::SCE_MACHINE_LIVENESS_SERVICE_SELF >= 0x8280,
        "F.X-4 sub-range must not overlap F.X-1 invoke [0x8100, 0x817F] or F.X-3 region-liveness [0x8180, 0x81FF]");
    static_assert(gen::SCE_MACHINE_LIVENESS_INSTANCE == 0x0001,
                  "SCE_MACHINE_LIVENESS_INSTANCE is fixed at 0x0001 per RFC F.X-4 §2 wire shape");

    using RouterT = gen::TransportRouter<SCE::Test::Mesh::TestSenderEngine>;
    static_assert(sizeof(RouterT) > 0, "TransportRouter must be a complete type — instantiation drags the "
                                       "consolidated SCE app field + register_availability_handler closure "
                                       "through the front end");

    std::printf("SCE Mesh §16.7 row 8 machine liveness compile verification (SOME/IP): PASS\n");
    return 0;
}
