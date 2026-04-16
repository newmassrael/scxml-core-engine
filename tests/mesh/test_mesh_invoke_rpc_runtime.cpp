// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE_MESH.md §9.5 mesh-rpc round-trip runtime verification.
//
// Two routers host brake_invoke and motor_invoke in the same process.
// brake enters `computing` on Go, whose <invoke type="sce:mesh-rpc">
// registers a correlation and emits service.request.compute_force.
// The forward envelope flows through brake_router.route_send →
// motor_router.dispatchToSender (linkTo) → motor's external queue.
// motor.step() transitions Ready → Replying, whose onentry emits
// service.response.compute_force; the reply routes back through
// motor_router.route_send → brake_router.dispatchToSender (linkTo),
// whose InvokeCorrelation table fires the done.invoke deliver callback
// and raises done.invoke.<id> on brake. A second brake.step() consumes
// that event and drives Computing → Ok.
//
// Any link in the chain regressing — linkTo not plumbed into
// route_send, dispatchToSender missing the correlation branch, the
// auto-propagation of _event.invokeid dropping the UUID, or the
// Invoke pattern inference misclassifying service.response.* — surfaces
// here as brake stuck outside State::Ok.

#include "brake_invoke_sm.h"
#include "brake_invoke_transport.h"
#include "motor_invoke_sm.h"
#include "motor_invoke_transport.h"

#include <cstdio>

int main() {
    SCE::Generated::brake_invoke::brake_invoke brake;
    SCE::Generated::motor_invoke::motor_invoke motor;

    SCE::Generated::brake_invoke::TransportRouter<decltype(brake), decltype(motor)>
        brake_router(brake, motor);
    SCE::Generated::motor_invoke::TransportRouter<decltype(motor), decltype(brake)>
        motor_router(motor, brake);

    // Cross-link: each router hands inbound envelopes to its peer's
    // dispatchToSender so brake_router's InvokeCorrelation sees the
    // reply and motor_router's sender_ receives the request.
    brake_router.linkTo("#motor_invoke",
                        [&motor_router](const SCE::Mesh::MeshEnvelope& env) {
                            return motor_router.dispatchToSender(env);
                        });
    motor_router.linkTo("#brake_invoke",
                        [&brake_router](const SCE::Mesh::MeshEnvelope& env) {
                            return brake_router.dispatchToSender(env);
                        });

    brake.initialize();
    motor.initialize();

    if (motor.getCurrentState() != SCE::Generated::motor_invoke::State::Ready) {
        std::printf("FAIL: motor did not enter Ready on initialize (state=%d)\n",
                    static_cast<int>(motor.getCurrentState()));
        return 1;
    }
    if (brake.getCurrentState() != SCE::Generated::brake_invoke::State::Idle) {
        std::printf("FAIL: brake did not enter Idle on initialize (state=%d)\n",
                    static_cast<int>(brake.getCurrentState()));
        return 2;
    }

    // Drive brake into Computing. Entering Computing fires the
    // mesh-rpc invoke onentry, enqueuing the RpcRequest on motor.
    brake.processEvent(SCE::Generated::brake_invoke::Event::Go);

    if (brake.getCurrentState() != SCE::Generated::brake_invoke::State::Computing) {
        std::printf("FAIL: brake did not transition to Computing (state=%d)\n",
                    static_cast<int>(brake.getCurrentState()));
        return 3;
    }

    // Macrostep motor: consumes the RpcRequest, enters Replying, whose
    // onentry emits the response. The response routes through
    // motor_router → linkTo → brake_router.dispatchToSender →
    // InvokeCorrelation → done.invoke raised on brake's external queue.
    motor.step();

    if (motor.getCurrentState() != SCE::Generated::motor_invoke::State::Replying) {
        std::printf("FAIL: motor did not transition to Replying (state=%d)\n",
                    static_cast<int>(motor.getCurrentState()));
        return 4;
    }

    // Macrostep brake: consume done.invoke.<id> and reach the Ok final.
    brake.step();

    if (brake.getCurrentState() != SCE::Generated::brake_invoke::State::Ok) {
        std::printf("FAIL: brake did not reach Ok on reply (state=%d). "
                    "Mesh-rpc correlation or auto-propagated invoke_id "
                    "likely regressed.\n",
                    static_cast<int>(brake.getCurrentState()));
        return 5;
    }

    std::printf("SCE Mesh §9.5 mesh-rpc round-trip: PASS\n");
    return 0;
}
