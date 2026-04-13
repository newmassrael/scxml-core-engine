// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh Phase 3 someip_transport compile verification test.
//
// Validates that generated SOME/IP transport code compiles against
// real vsomeip 3.7.x headers. No runtime assertions — compilation
// success IS the test. Runtime verification requires vsomeip routing
// manager and network configuration.

#include "brake_sm.h"
#include "motor_sm.h"
#include "brake_transport.h"

#include <cstdio>

// Verify TransportRouter type is well-formed (no template params — someip only)
static_assert(sizeof(SCE::Generated::brake::TransportRouter) > 0,
              "TransportRouter must be instantiable");

// Verify SOME/IP service constants match deploy_someip.yaml
static_assert(SCE::Generated::brake::SOMEIP_SERVICE_MOTOR == 0x1234,
              "Service ID must match deploy.yaml");
static_assert(SCE::Generated::brake::SOMEIP_INSTANCE_MOTOR == 0x0001,
              "Instance ID must match deploy.yaml");
static_assert(SCE::Generated::brake::SOMEIP_METHOD_MOTOR == 0x0421,
              "Method ID must match deploy.yaml");

// Session C: multi-pattern constants (RPC, PubSub, FieldAccess)
static_assert(SCE::Generated::brake::SOMEIP_EVENT_GROUP_MOTOR == 0x0001,
              "Event group ID must match deploy.yaml");
static_assert(SCE::Generated::brake::SOMEIP_EVENT_MOTOR == 0x8001,
              "Event ID must match deploy.yaml");
static_assert(SCE::Generated::brake::SOMEIP_GETTER_MOTOR == 0x0100,
              "Getter method ID must match deploy.yaml");
static_assert(SCE::Generated::brake::SOMEIP_SETTER_MOTOR == 0x0101,
              "Setter method ID must match deploy.yaml");

int main() {
    std::printf("SCE Mesh someip_transport compile verification: PASS\n");
    return 0;
}
