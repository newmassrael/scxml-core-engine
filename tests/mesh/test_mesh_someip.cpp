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

// TransportRouter is templated on the sender engine (ctor injection
// pattern). Instantiate on the generated brake engine type.
static_assert(sizeof(SCE::Generated::brake::TransportRouter<SCE::Generated::brake::brake>) > 0,
              "TransportRouter<brake> must be instantiable");

// Verify SOME/IP service constants match deploy_someip.yaml.
// Service/instance IDs are per-target (binding-level) and stay
// `SOMEIP_SERVICE_<TARGET>` / `SOMEIP_INSTANCE_<TARGET>`.
static_assert(SCE::Generated::brake::SOMEIP_SERVICE_MOTOR == 0x1234,
              "Service ID must match deploy.yaml");
static_assert(SCE::Generated::brake::SOMEIP_INSTANCE_MOTOR == 0x0001,
              "Instance ID must match deploy.yaml");

// Per-event SOME/IP constants (SCE_MESH.md §14): pattern-matched IDs are
// generated as `SOMEIP_<KIND>_<TARGET>_<EVENT>` so the same target can
// route different events to different methods/event-groups. brake.scxml
// only sends `brake.activate` (FireForget → method_id), so only the
// per-event method constant for that event exists. The legacy "one
// constant per kind per target" shape is gone — each event must list
// itself in deploy.yaml `events:` (or rely on flat-sugar fan-out) to
// produce a constant.
static_assert(SCE::Generated::brake::SOMEIP_METHOD_MOTOR_BRAKE_ACTIVATE == 0x0421,
              "Per-event method ID must match deploy.yaml method_id");

int main() {
    std::printf("SCE Mesh someip_transport compile verification: PASS\n");
    return 0;
}
