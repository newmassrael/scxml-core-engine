// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §13: a machine-lifetime subscription over SOME/IP compiles
// against real vsomeip headers.
//
// The subscriber document declares no <send>, so every SOME/IP symbol
// reachable here was emitted from deploy.yaml `subscriptions:` alone. That
// is what makes this a compile test worth having: the Rust-side generation
// test (sce-build/tests/mesh_someip_machine_lifetime_subscribe.rs) asserts
// the text is emitted, and text that names `request_event` proves nothing
// about whether the call type-checks against vsomeip 3.x's overload set.
//
// The three ids are asserted against vsomeip_someip_machine_lifetime.json
// rather than against each other. An operator never writes a numeric id in
// deploy.yaml — inventing one is precisely the failure the name-resolution
// stage exists to prevent — so a generated constant that stopped tracking
// the file would be the regression, and constants compared only to
// constants would not catch it.
//
// Compilation success plus these asserts IS the test; a live SD
// SubscribeEventgroup exchange needs a routing manager and lives in the
// runtime SOME/IP suite.

#include "brake_someip_machine_lifetime_subscribe_sm.h"
#include "brake_someip_machine_lifetime_subscribe_transport.h"
#include "motor_someip_machine_lifetime_subscribe_sm.h"
#include "motor_someip_machine_lifetime_subscribe_transport.h"

#include "mesh/PatternKind.h"

#include <cstdio>
#include <cstring>

namespace gen = SCE::Generated::brake_someip_machine_lifetime_subscribe;
using BrakeEngine = gen::brake_someip_machine_lifetime_subscribe;
using RouterT = gen::TransportRouter<BrakeEngine>;

// ── Structural invariants (compile-time) ────────────────────────

static_assert(sizeof(RouterT) > 0, "TransportRouter must be instantiable for a subscriptions-only machine");

// Service identity resolved from `service: motor_control`.
static_assert(gen::SOMEIP_SERVICE_MOTOR == 0x3100, "Service ID must match vsomeip_someip_machine_lifetime.json");
static_assert(gen::SOMEIP_INSTANCE_MOTOR == 0x0007, "Instance ID must match vsomeip_someip_machine_lifetime.json");

// The pair `request_event` + `subscribe` need. Neither constant exists on
// any outbound send path in this machine — both trace back to the
// `subscriptions:` entry.
static_assert(gen::SOMEIP_EVENT_GROUP_MOTOR_EVENT_NOTIFICATION_VEHICLE_SPEED == 0x0021,
              "Eventgroup ID must match vsomeip.json speed_group");
static_assert(gen::SOMEIP_EVENT_MOTOR_EVENT_NOTIFICATION_VEHICLE_SPEED == 0x8042,
              "Event ID must match speed_group's sole member");

// ── Runtime checks ──────────────────────────────────────────────

#define CHECK(cond, msg)                                                                                               \
    do {                                                                                                               \
        if (!(cond)) {                                                                                                 \
            std::fprintf(stderr, "FAIL: %s (%s:%d)\n", msg, __FILE__, __LINE__);                                       \
            return 1;                                                                                                  \
        }                                                                                                              \
    } while (0)

int main() {
    // A subscription is not a request, so it must not acquire a reply
    // pairing on the way through the topology-inferred RPC table
    // (SCE_MESH.md §13 path B). A machine-lifetime entry that did would
    // leave a correlation slot nothing ever retires.
    CHECK(RouterT::resolveReplyEvent("event.notification.vehicle_speed")[0] == '\0',
          "a machine-lifetime subscription must resolve to an empty reply-event");

    // The publisher side has to offer the eventgroup the subscriber
    // subscribes to, or the deployment compiles as two halves that never
    // meet. Asserting both sides against the same JSON is what makes the
    // pairing checkable at build time — vsomeip checks it nowhere.
    namespace motor_gen = SCE::Generated::motor_someip_machine_lifetime_subscribe;
    using MotorEngine = motor_gen::motor_someip_machine_lifetime_subscribe;
    using MotorRouterT = motor_gen::TransportRouter<MotorEngine>;
    static_assert(sizeof(MotorRouterT) > 0, "Publisher TransportRouter must be instantiable");
    static_assert(motor_gen::SOMEIP_SERVER_SERVICE == 0x3100, "Publisher must offer the service the subscriber wants");
    static_assert(motor_gen::SOMEIP_SERVER_INSTANCES.size() == 1, "Non-pool server offers exactly one instance");
    static_assert(motor_gen::SOMEIP_SERVER_INSTANCES[0] == 0x0007,
                  "Publisher must offer the instance the subscriber requests");

    std::printf("SCE Mesh SOME/IP machine-lifetime subscribe compile verification: PASS\n");
    return 0;
}
