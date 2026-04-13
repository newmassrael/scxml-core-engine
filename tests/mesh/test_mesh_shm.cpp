// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh shm_transport compile verification test.
//
// Validates that generated shared memory transport code compiles
// against runtime headers (ShmSegment, ShmChannel, EventQueueBridge).
// No runtime shm_open calls — compilation success IS the test.

#include "brake_sm.h"
#include "motor_sm.h"
#include "brake_transport.h"
#include "mesh/ShmChannel.h"
#include "mesh/ShmSegment.h"

#include <cstdio>

// Control slot is 12 bytes (offset + length + advance).
static_assert(sizeof(SCE::Mesh::ControlSlot) == 12,
              "ControlSlot must be 12 bytes");

// Verify TransportRouter type is well-formed (shm-only: no template params)
static_assert(sizeof(SCE::Generated::brake::TransportRouter) > 0,
              "TransportRouter must be instantiable");

// Verify channel name constant follows POSIX shm naming
static_assert(SCE::Generated::brake::SHM_CHANNEL_MOTOR[0] == '/',
              "Channel name must start with '/' (POSIX shm requirement)");

// Verify ShmChannel template instantiation with default parameters
static_assert(sizeof(SCE::Mesh::ShmChannel<>) > 0,
              "ShmChannel<> must be instantiable");

int main() {
    std::printf("SCE Mesh shm_transport compile verification: PASS\n");
    return 0;
}
