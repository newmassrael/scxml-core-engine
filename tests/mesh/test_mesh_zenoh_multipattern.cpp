// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh Session D: zenoh multi-pattern compile verification.
//
// brake_zenoh_multi.scxml exercises every Zenoh capability category:
// FireForget, RpcRequest (with sce:reply-event), EventSubscribe,
// FieldRead, FieldWrite. All five branches of the generated send_zenoh
// switch must instantiate and compile against zenoh-cpp. Compilation
// success IS the test; runtime behaviour is covered by
// test_mesh_zenoh_multipattern_runtime.cpp.

#include "brake_zenoh_multi_sm.h"
#include "motor_zenoh_multi_sm.h"
#include "brake_zenoh_multi_transport.h"

#include "mesh/PatternKind.h"

#include <cstdio>
#include <cstring>
#include <string>
#include <string_view>

namespace gen = SCE::Generated::brake_zenoh_multi;
using PK = SCE::Mesh::PatternKind;
using BrakeEngine = gen::brake_zenoh_multi;
using RouterT = gen::TransportRouter<BrakeEngine>;

// Structural invariants that the compiler can check without running the code.
static_assert(sizeof(RouterT) > 0,
              "TransportRouter must be instantiable for zenoh multi-pattern");
static_assert(std::string_view(gen::ZENOH_KEY_MOTOR) == "sce/brake/motor",
              "Zenoh key expression must match deploy.yaml");

// resolvePattern / resolveReplyEvent take std::string, so they can't be
// constexpr — check them at runtime. A failure returns 1 (ctest FAIL);
// success prints PASS and returns 0.
#define CHECK(cond, msg)                                                        \
    do {                                                                        \
        if (!(cond)) {                                                          \
            std::fprintf(stderr, "FAIL: %s (%s:%d)\n", msg, __FILE__, __LINE__); \
            return 1;                                                           \
        }                                                                       \
    } while (0)

int main() {
    // Force link-time instantiation of every member that touches zenoh-cpp
    // types. If the template is malformed for a given switch case, the
    // take-address expression triggers the error at build time.
    BrakeEngine brake;
    RouterT router(brake);
    (void)&RouterT::route_send;
    (void)&RouterT::send_zenoh;
    (void)&RouterT::resolvePattern;
    (void)&RouterT::resolveReplyEvent;
    (void)&RouterT::dispatchToSender;

    // Pattern resolution table — each event the SCXML emits must map to
    // the PatternKind dictated by reserved-prefix detection in
    // sce-build/src/mesh/pattern.rs.
    CHECK(RouterT::resolvePattern("service.fire_forget.activate") == PK::FireForget,
          "service.fire_forget.* must resolve to FireForget");
    CHECK(RouterT::resolvePattern("service.request.compute_force") == PK::RpcRequest,
          "service.request.* must resolve to RpcRequest");
    CHECK(RouterT::resolvePattern("event.subscribe.status") == PK::EventSubscribe,
          "event.subscribe.* must resolve to EventSubscribe");
    CHECK(RouterT::resolvePattern("field.get.position") == PK::FieldRead,
          "field.get.* must resolve to FieldRead");
    CHECK(RouterT::resolvePattern("field.set.position") == PK::FieldWrite,
          "field.set.* must resolve to FieldWrite");

    // sce:reply-event="force.computed" on the service.request.compute_force
    // send must be captured verbatim in the build-time reply table.
    CHECK(std::strcmp(RouterT::resolveReplyEvent("service.request.compute_force"),
                      "force.computed") == 0,
          "sce:reply-event must appear in resolveReplyEvent table");
    // Unmapped events return an empty string so callers can branch safely.
    CHECK(RouterT::resolveReplyEvent("service.fire_forget.activate")[0] == '\0',
          "non-RPC events must resolve to empty reply-event");

    std::printf("SCE Mesh zenoh multi-pattern compile verification: PASS\n");
    return 0;
}
