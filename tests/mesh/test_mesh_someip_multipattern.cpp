// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh Session C: SOME/IP multi-pattern compile verification.
//
// brake_someip_multi.scxml exercises all 4 SOME/IP capability categories:
// FireForget, RequestReply (RPC), PubSub (subscribe/notification), and
// FieldAccess (getter/setter). The generated code must compile against
// real vsomeip 3.7.x headers with every per-field_kind branch instantiated.
// Compilation success IS the test; runtime verification requires a vsomeip
// routing manager and is deferred. SCE_MESH.md Section 13 path B:
// RPC reply pairing is topology-inferred (service.request.X <->
// service.response.X), not declared via sce:* attributes.

#include "brake_someip_multi_sm.h"
#include "motor_someip_multi_sm.h"
#include "brake_someip_multi_transport.h"
#include "motor_someip_multi_transport.h"

#include "mesh/PatternKind.h"

#include <cstdio>
#include <cstring>
#include <string>
#include <string_view>

namespace gen = SCE::Generated::brake_someip_multi;
using PK = SCE::Mesh::PatternKind;
using BrakeEngine = gen::brake_someip_multi;
using RouterT = gen::TransportRouter<BrakeEngine>;

// ── Structural invariants (compile-time) ────────────────────────

static_assert(sizeof(RouterT) > 0,
              "TransportRouter must be instantiable for SOME/IP multi-pattern");

// Service/instance constants (from vsomeip_motor_multi.json via deploy.yaml)
static_assert(gen::SOMEIP_SERVICE_MOTOR == 0x2000,
              "Service ID must match vsomeip_motor_multi.json");
static_assert(gen::SOMEIP_INSTANCE_MOTOR == 0x0001,
              "Instance ID must match vsomeip_motor_multi.json");

// Per-event method constants (FireForget + RPC)
static_assert(gen::SOMEIP_METHOD_MOTOR_SERVICE_FIRE_FORGET_ACTIVATE == 0x0100,
              "FireForget method ID must match vsomeip.json activate");
static_assert(gen::SOMEIP_METHOD_MOTOR_SERVICE_REQUEST_COMPUTE_FORCE == 0x0101,
              "RPC method ID must match vsomeip.json compute_force");

// Per-event event group + event constants (PubSub)
static_assert(gen::SOMEIP_EVENT_GROUP_MOTOR_EVENT_SUBSCRIBE_STATUS == 0x0001,
              "Event group ID must match vsomeip.json status_group");
static_assert(gen::SOMEIP_EVENT_MOTOR_EVENT_SUBSCRIBE_STATUS == 0x8001,
              "Event ID must match vsomeip.json status_group events[0]");

// Per-event getter/setter method constants (FieldAccess)
static_assert(gen::SOMEIP_GETTER_MOTOR_FIELD_GET_VEHICLE_SPEED == 0x0200,
              "Getter ID must match vsomeip.json get_vehicle_speed");
static_assert(gen::SOMEIP_SETTER_MOTOR_FIELD_SET_TARGET_SPEED == 0x0201,
              "Setter ID must match vsomeip.json set_target_speed");

// ── Runtime checks (pattern resolution + RPC reply table) ───────

#define CHECK(cond, msg)                                                        \
    do {                                                                        \
        if (!(cond)) {                                                          \
            std::fprintf(stderr, "FAIL: %s (%s:%d)\n", msg, __FILE__, __LINE__); \
            return 1;                                                           \
        }                                                                       \
    } while (0)

int main() {
    // Force link-time instantiation of every member that touches vsomeip
    // types. If the template is malformed for a given dispatch branch,
    // the take-address expression triggers the error at build time.
    BrakeEngine brake;
    RouterT router(brake);
    (void)&RouterT::route_send;
    (void)&RouterT::resolvePattern;
    (void)&RouterT::resolveReplyEvent;
    (void)&RouterT::dispatchToSender;

    // Pattern resolution: each event must map to the PatternKind dictated
    // by reserved-prefix detection in sce-build/src/mesh/pattern.rs.
    CHECK(RouterT::resolvePattern("service.fire_forget.activate") == PK::FireForget,
          "service.fire_forget.* must resolve to FireForget");
    CHECK(RouterT::resolvePattern("service.request.compute_force") == PK::RpcRequest,
          "service.request.* must resolve to RpcRequest");
    CHECK(RouterT::resolvePattern("event.subscribe.status") == PK::EventSubscribe,
          "event.subscribe.* must resolve to EventSubscribe");
    CHECK(RouterT::resolvePattern("event.unsubscribe.status") == PK::EventUnsubscribe,
          "event.unsubscribe.* must resolve to EventUnsubscribe");
    CHECK(RouterT::resolvePattern("field.get.vehicle_speed") == PK::FieldRead,
          "field.get.* must resolve to FieldRead");
    CHECK(RouterT::resolvePattern("field.set.target_speed") == PK::FieldWrite,
          "field.set.* must resolve to FieldWrite");

    // Topology-inferred RPC pairing: service.request.X must resolve to
    // service.response.X in the build-time reply table (SCE_MESH.md
    // Section 13 path B).
    CHECK(std::strcmp(RouterT::resolveReplyEvent("service.request.compute_force"),
                      "service.response.compute_force") == 0,
          "topology-inferred RPC reply must appear in resolveReplyEvent table");
    // Non-RPC events return empty string.
    CHECK(RouterT::resolveReplyEvent("service.fire_forget.activate")[0] == '\0',
          "FireForget must resolve to empty reply-event");
    CHECK(RouterT::resolveReplyEvent("event.subscribe.status")[0] == '\0',
          "EventSubscribe must resolve to empty reply-event");
    CHECK(RouterT::resolveReplyEvent("field.get.vehicle_speed")[0] == '\0',
          "FieldRead must resolve to empty reply-event");

    // ── Session E: server-side compile verification ──────────────
    // Motor transport header must be includable and the server-side
    // TransportRouter must be instantiable.
    namespace motor_gen = SCE::Generated::motor_someip_multi;
    using MotorEngine = motor_gen::motor_someip_multi;
    using MotorRouterT = motor_gen::TransportRouter<MotorEngine>;
    static_assert(sizeof(MotorRouterT) > 0,
                  "Server TransportRouter must be instantiable for SOME/IP");

    // Server-side SOME/IP constants
    static_assert(motor_gen::SOMEIP_SERVER_SERVICE == 0x2000,
                  "Server service ID must match vsomeip_motor_multi.json");
    static_assert(motor_gen::SOMEIP_SERVER_INSTANCE == 0x0001,
                  "Server instance ID must match vsomeip_motor_multi.json");
    static_assert(motor_gen::SOMEIP_SERVER_METHOD_SERVICE_REQUEST_COMPUTE_FORCE == 0x0101,
                  "Server method ID must match vsomeip.json compute_force");

    std::printf("SCE Mesh SOME/IP multi-pattern compile verification: PASS\n");
    return 0;
}
