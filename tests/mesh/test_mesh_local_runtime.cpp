// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh local_transport end-to-end runtime test.
//
// Exercises the full send path:
//   brake <send target="#motor" event="brake.activate"/>
//     → StaticExecutionEngine::raiseExternal (mesh target detected)
//     → onMeshSend callback (installed by TransportRouter ctor)
//     → TransportRouter ctor lambda
//     → motor.processEvent(Brake_activate)
//     → motor transitions running → stopped
//
// Compile alone does not prove this chain works — previous revisions
// silently enqueued to the external queue and the motor never moved.
// This test fails loudly if any link in the chain regresses.

#include "brake_sm.h"
#include "brake_transport.h"
#include "motor_sm.h"

#include <cstdio>

int main() {
    SCE::Generated::brake::brake brake;
    SCE::Generated::motor::motor motor;

    // Session-first ctor injection: `brake` (hosted session 0) is
    // bound at construction and the mesh-send callback is installed on
    // it in the ctor body — no separate wireTo() step.
    using BrakeEngine = SCE::Generated::brake::brake;
    using MotorEngine = SCE::Generated::motor::motor;
    SCE::Generated::brake::TransportRouter<BrakeEngine, MotorEngine> router({&brake}, motor);

    brake.initialize();
    motor.initialize();

    if (motor.getCurrentState() != SCE::Generated::motor::State::Running) {
        std::printf("FAIL: motor did not enter 'running' on initialize (state=%d)\n",
                    static_cast<int>(motor.getCurrentState()));
        return 1;
    }

    // Triggering Brake_press drives brake into 'braking', whose <onentry>
    // fires <send target="#motor" event="brake.activate"/>. The router
    // enqueues to motor's external queue but does NOT step motor —
    // macrostep timing is the application's responsibility.
    brake.processEvent(SCE::Generated::brake::Event::Brake_press);

    // Application-driven macrostep: drain motor's external queue now.
    motor.step();

    if (motor.getCurrentState() != SCE::Generated::motor::State::Stopped) {
        std::printf("FAIL: motor did not transition to 'stopped' (state=%d). "
                    "Mesh send callback likely not wired.\n",
                    static_cast<int>(motor.getCurrentState()));
        return 2;
    }

    std::printf("SCE Mesh local_transport runtime verification: PASS\n");
    return 0;
}
