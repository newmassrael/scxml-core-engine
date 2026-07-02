// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE_MESH.md §9.5 mesh-rpc deadline expiry runtime verification.
//
// brake_invoke declares _mesh_deadline_ms=50. This test sends the
// RpcRequest to motor but never steps motor, so motor never replies.
// After 100ms the MeshDeadlineScheduler fires handleDeadline on
// the scheduler thread → InvokeCorrelation delivers
// error.invoke.<id> with RpcStatus::DeadlineExceeded to brake's
// external queue. brake.step() consumes the event and transitions
// Computing → Failed.
//
// SCE_THREAD_SAFE is required: the scheduler thread's raiseExternal
// push and the main thread's step() pop share the EventQueueManager,
// and sleep_for alone does not establish happens-before per the C++
// memory model. The define enables the queue's internal mutex.

#include "brake_invoke_sm.h"
#include "brake_invoke_transport.h"
#include "motor_invoke_sm.h"
#include "motor_invoke_transport.h"

#include <chrono>
#include <cstdio>
#include <thread>

int main() {
    SCE::Generated::brake_invoke::brake_invoke brake;
    SCE::Generated::motor_invoke::motor_invoke motor;

    SCE::Generated::brake_invoke::TransportRouter<decltype(brake), decltype(motor)> brake_router({&brake}, motor);
    SCE::Generated::motor_invoke::TransportRouter<decltype(motor), decltype(brake)> motor_router({&motor}, brake);

    brake_router.linkTo("#motor_invoke", [&motor_router](const SCE::Mesh::MeshEnvelope &env) {
        return motor_router.dispatchToSession(env, 0);
    });
    motor_router.linkTo("#brake_invoke", [&brake_router](const SCE::Mesh::MeshEnvelope &env) {
        return brake_router.dispatchToSession(env, 0);
    });

    brake.initialize();
    motor.initialize();

    // Drive brake into Computing. The mesh-rpc invoke arms a 50ms
    // deadline and sends the RpcRequest to motor's external queue.
    brake.processEvent(SCE::Generated::brake_invoke::Event::Go);

    if (brake.getCurrentState() != SCE::Generated::brake_invoke::State::Computing) {
        std::printf("FAIL: brake did not enter Computing (state=%d)\n", static_cast<int>(brake.getCurrentState()));
        return 1;
    }

    // DO NOT step motor — the request sits unconsumed. The 50ms
    // deadline expires on the scheduler thread, which fires
    // error.invoke on brake's external queue.
    std::this_thread::sleep_for(std::chrono::milliseconds(150));

    // Drain brake: the deadline-fired error.invoke should drive
    // Computing → Failed.
    brake.step();

    if (brake.getCurrentState() != SCE::Generated::brake_invoke::State::Failed) {
        std::printf("FAIL: brake did not reach Failed after deadline (state=%d). "
                    "MeshDeadlineScheduler or InvokeCorrelation::handleDeadline "
                    "likely regressed.\n",
                    static_cast<int>(brake.getCurrentState()));
        return 2;
    }

    std::printf("SCE Mesh §9.5 mesh-rpc deadline expiry: PASS\n");
    return 0;
}
