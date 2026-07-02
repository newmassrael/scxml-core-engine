// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh zenoh_transport compile verification test.
//
// Validates that generated Zenoh transport code compiles against
// real zenoh-cpp headers. No runtime assertions — compilation
// success IS the test. Runtime verification requires a Zenoh
// router or peer network configuration.

#include "brake_sm.h"
#include "brake_transport.h"
#include "motor_sm.h"

#include <cstdio>
#include <string_view>

// TransportRouter is always templated on the sender engine (ctor
// injection pattern). Instantiate on the generated brake engine type.
static_assert(sizeof(SCE::Generated::brake::TransportRouter<SCE::Generated::brake::brake>) > 0,
              "TransportRouter<brake> must be instantiable");

// Verify Zenoh key expression constant matches deploy_zenoh.yaml
static_assert(std::string_view(SCE::Generated::brake::ZENOH_KEY_MOTOR) == "sce/brake/motor/cmd",
              "Zenoh key expression must match deploy.yaml");

int main() {
    std::printf("SCE Mesh zenoh_transport compile verification: PASS\n");
    return 0;
}
