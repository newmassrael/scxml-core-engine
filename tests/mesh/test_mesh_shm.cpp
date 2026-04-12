// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh Phase 2 shm_transport compile verification test.
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

// Verify ShmEvent wire format
static_assert(sizeof(SCE::Mesh::ShmEvent) == 64,
              "ShmEvent must be 64 bytes for cache-line alignment");

// Verify ShmTransportRouter type is well-formed
static_assert(sizeof(SCE::Generated::brake::ShmTransportRouter) > 0,
              "ShmTransportRouter must be instantiable");

// Verify channel name constant follows POSIX shm naming
static_assert(SCE::Generated::brake::SHM_CHANNEL_MOTOR[0] == '/',
              "Channel name must start with '/' (POSIX shm requirement)");

// Verify ShmChannel template instantiation
static_assert(sizeof(SCE::Mesh::ShmChannel<256>) > 0,
              "ShmChannel<256> must be instantiable");

// Verify ShmEvent construction from event name
static_assert(SCE::Mesh::ShmEvent::MAX_NAME_LEN == 63,
              "ShmEvent name buffer must hold 63 chars + null");

int main() {
    // Verify ShmEvent round-trip at runtime
    SCE::Mesh::ShmEvent event("brake.activate");
    if (std::strcmp(event.name, "brake.activate") != 0) {
        std::printf("ShmEvent round-trip FAILED\n");
        return 1;
    }

    std::printf("SCE Mesh shm_transport compile verification: PASS\n");
    return 0;
}
