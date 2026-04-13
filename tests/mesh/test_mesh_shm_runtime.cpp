// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh Phase 2 shm_transport end-to-end runtime test.
//
// Cross-process verification of the full send path:
//   parent:  brake <send target="#motor"/> → engine → router.wireTo lambda
//            → ShmChannel::send → shm page
//   child:   ShmChannel::open (waits for READY_MAGIC handshake)
//            → channel.drain<motorPolicy>(motor_engine)
//            → motor.processEvent(Brake_activate)
//            → motor transitions running → stopped
//
// Parent and child communicate over /sce_brake_motor shared memory.
// Parent owns (shm_unlink on exit); child attaches read/write.

#include "brake_sm.h"
#include "motor_sm.h"
#include "brake_transport.h"

#include "mesh/ShmChannel.h"

#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <thread>

#include <sys/wait.h>
#include <unistd.h>

namespace {

constexpr auto POLL_INTERVAL = std::chrono::milliseconds(10);
constexpr int OPEN_RETRIES = 200;   // 2 s total — parent constructs router quickly
constexpr int DRAIN_RETRIES = 200;  // 2 s to observe the event

int runReceiver() {
    // Retry open until the sender has finished constructing the channel
    // (ShmChannel returns valid()==false until READY_MAGIC is visible).
    for (int attempt = 0; attempt < OPEN_RETRIES; ++attempt) {
        // Use the typedef exported from the sender's generated header so
        // arena/ring template parameters match the sender exactly (the shm
        // layout is template-parameterized; mismatch = incompatible
        // segment size). SCE_MESH.md Section 7.5.
        using Channel = SCE::Generated::brake::MotorShmChannel;
        Channel channel(SCE::Generated::brake::SHM_CHANNEL_MOTOR,
                        Channel::Mode::Open);
        if (!channel.valid()) {
            std::this_thread::sleep_for(POLL_INTERVAL);
            continue;
        }

        SCE::Generated::motor::motor motor;
        motor.initialize();

        for (int drain = 0; drain < DRAIN_RETRIES; ++drain) {
            // drain() enqueues events via raiseExternal but does NOT step
            // the engine (W3C SCXML 3.12 — transport layer does not drive
            // macrostep timing). The application owns step() scheduling.
            std::size_t drained = channel.drain<SCE::Generated::motor::motorPolicy>(motor);
            if (drained > 0) {
                motor.step();  // process the external queue we just filled
                if (motor.getCurrentState() == SCE::Generated::motor::State::Stopped) {
                    return 0;  // success
                }
                std::fprintf(stderr, "receiver: drained %zu but motor state=%d\n",
                             drained, static_cast<int>(motor.getCurrentState()));
                return 11;
            }
            std::this_thread::sleep_for(POLL_INTERVAL);
        }
        return 12;  // timeout waiting for an event on the channel
    }
    return 13;  // timeout waiting for the channel to become valid
}

}  // namespace

int main() {
    pid_t pid = ::fork();
    if (pid < 0) {
        std::perror("fork");
        return 101;
    }

    if (pid == 0) {
        // Child: receiver. Exit with status; parent checks via waitpid.
        std::exit(runReceiver());
    }

    // Parent: sender. The router MUST outlive waitpid so that the
    // shared-memory segment stays mapped (and the name stays linked)
    // until the child has finished draining and exited. Otherwise
    // shm_unlink on the parent side can race ahead of a slow-to-schedule
    // child's shm_open and cause a flaky ENOENT → OPEN_RETRIES timeout.
    SCE::Generated::brake::brake brake;
    brake.initialize();

    // Sender-first ctor injection: brake (sender) is bound at construction.
    using BrakeEngine = SCE::Generated::brake::brake;
    SCE::Generated::brake::TransportRouter<BrakeEngine> router(brake);

    // Give the receiver a brief window to reach its drain loop before we
    // publish the event. Not required for correctness (the receiver
    // polls try_pop), but reduces wasted polls on fast machines.
    std::this_thread::sleep_for(std::chrono::milliseconds(50));

    // Triggering Brake_press drives brake into 'braking', whose <onentry>
    // fires <send target="#motor" event="brake.activate"/>. The engine's
    // onMeshSend callback (installed by router.wireTo above) routes this
    // through the shared-memory channel to the child receiver.
    brake.processEvent(SCE::Generated::brake::Event::Brake_press);

    int status = 0;
    if (::waitpid(pid, &status, 0) < 0) {
        std::perror("waitpid");
        return 102;
    }

    if (!WIFEXITED(status)) {
        std::fprintf(stderr, "FAIL: receiver did not exit normally (status=%d)\n", status);
        return 104;
    }

    int receiver_rc = WEXITSTATUS(status);
    if (receiver_rc != 0) {
        std::fprintf(stderr, "FAIL: receiver exit code %d\n", receiver_rc);
        return 105;
    }

    std::printf("SCE Mesh shm_transport runtime verification: PASS\n");
    return 0;
}
