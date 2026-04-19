// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh end-to-end payload test (SCE_MESH.md Section 7.5).
//
// Verifies the full data path across processes:
//   parent:  brake_payload <send><param name="force" expr="42"/></send>
//            → MeshEnvelope.data = {"force":["42"]} (JSON bytes)
//            → encodeEnvelope (CBOR) → sendRaw → arena
//            → control ring entry
//   child:   drain → raiseExternal(event, data) → external queue
//            → step() → _event.data populated
//            → transition cond="_event.data.force == 42" matches
//            → motor_payload reaches 'verified' state
//
// This closes the "wire format defined" → "data reaches SCXML guards" gap.
// Without this test, prior "data forwarding" work was not verified E2E.

#include "brake_payload_sm.h"
#include "motor_payload_sm.h"
#include "brake_payload_transport.h"

#include "mesh/ShmChannel.h"

#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <thread>

// Verify deploy.yaml config override reached the generated template.
// deploy_shm_payload.yaml specifies:
//   shm_arena_bytes: 131072
//   shm_ring_capacity: 512
// These must override the library defaults (65536 / 256).
static_assert(
    SCE::Generated::brake_payload::SHM_ARENA_BYTES_MOTOR_PAYLOAD == 131072,
    "deploy.yaml shm_arena_bytes override failed to reach generated code");
static_assert(
    SCE::Generated::brake_payload::SHM_RING_CAPACITY_MOTOR_PAYLOAD == 512,
    "deploy.yaml shm_ring_capacity override failed to reach generated code");

#include <sys/wait.h>
#include <unistd.h>

namespace {

constexpr auto POLL_INTERVAL = std::chrono::milliseconds(10);
constexpr int OPEN_RETRIES = 200;   // 2 s total
constexpr int DRAIN_RETRIES = 200;  // 2 s to observe the event

int runReceiver() {
    for (int attempt = 0; attempt < OPEN_RETRIES; ++attempt) {
        // Match the sender's template parameters via the generated typedef.
        using Channel = SCE::Generated::brake_payload::MotorPayloadShmChannel;
        Channel channel(
            SCE::Generated::brake_payload::SHM_CHANNEL_MOTOR_PAYLOAD,
            Channel::Mode::Open);
        if (!channel.valid()) {
            std::this_thread::sleep_for(POLL_INTERVAL);
            continue;
        }

        SCE::Generated::motor_payload::motor_payload motor;
        motor.initialize();

        for (int drain = 0; drain < DRAIN_RETRIES; ++drain) {
            std::size_t drained =
                channel.drain<SCE::Generated::motor_payload::motor_payloadPolicy>(motor);
            if (drained > 0) {
                motor.step();
                auto state = motor.getCurrentState();
                if (state == SCE::Generated::motor_payload::State::Verified) {
                    return 0;  // success — payload reached _event.data
                }
                if (state == SCE::Generated::motor_payload::State::Wrong_payload) {
                    std::fprintf(stderr, "receiver: payload mismatch "
                                 "(cond '_event.data.force == 42' failed)\n");
                    return 21;
                }
                std::fprintf(stderr, "receiver: unexpected state %d after drain\n",
                             static_cast<int>(state));
                return 22;
            }
            std::this_thread::sleep_for(POLL_INTERVAL);
        }
        return 23;  // timeout waiting for event
    }
    return 24;  // timeout waiting for channel
}

}  // namespace

int main() {
    pid_t pid = ::fork();
    if (pid < 0) {
        std::perror("fork");
        return 101;
    }

    if (pid == 0) {
        std::exit(runReceiver());
    }

    SCE::Generated::brake_payload::brake_payload brake;
    brake.initialize();

    using BrakeEngine = SCE::Generated::brake_payload::brake_payload;
    SCE::Generated::brake_payload::TransportRouter<BrakeEngine> router({&brake});

    std::this_thread::sleep_for(std::chrono::milliseconds(50));

    // Triggering brake.press enters 'braking', whose <onentry> fires
    // <send target="#motor_payload"><param name="force" expr="42"/></send>.
    // The param is assembled into JSON by SCXML runtime, carried through
    // the shm wire format, and must surface as _event.data.force on
    // the receiver side.
    brake.processEvent(SCE::Generated::brake_payload::Event::Brake_press);

    int status = 0;
    if (::waitpid(pid, &status, 0) < 0) {
        std::perror("waitpid");
        return 102;
    }

    if (!WIFEXITED(status)) {
        std::fprintf(stderr, "FAIL: receiver did not exit normally (status=%d)\n", status);
        return 104;
    }

    int rc = WEXITSTATUS(status);
    if (rc != 0) {
        std::fprintf(stderr, "FAIL: receiver exit code %d\n", rc);
        return 105;
    }

    std::printf("SCE Mesh shm_transport payload E2E verification: PASS\n");
    return 0;
}
