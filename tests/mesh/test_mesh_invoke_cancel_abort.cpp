// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE_MESH.md §9.5 mesh-rpc cancel via abort transition.
//
// brake_invoke enters Computing (mesh-rpc invoke armed), then receives
// the `abort` event. The transition Computing → Aborted triggers the
// generated onexit which calls cancelMeshRpc — removing the correlation
// entry and cancelling the deadline. A late reply from motor (stepped
// AFTER the abort) must be silently dropped: no done.invoke or
// error.invoke should fire on brake.
//
// Validates W3C SCXML 3.12.1 document-order priority: <transition
// event="abort"/> appears before done.invoke / error.invoke in the
// SCXML, so abort wins if both are pending simultaneously.

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

    brake.processEvent(SCE::Generated::brake_invoke::Event::Go);

    if (brake.getCurrentState() != SCE::Generated::brake_invoke::State::Computing) {
        std::printf("FAIL: brake did not enter Computing (state=%d)\n",
                    static_cast<int>(brake.getCurrentState()));
        return 1;
    }

    // Abort: exits Computing → Aborted. The onexit cancelMeshRpc
    // removes the correlation entry and the armed deadline.
    brake.processEvent(SCE::Generated::brake_invoke::Event::Abort);

    if (brake.getCurrentState() != SCE::Generated::brake_invoke::State::Aborted) {
        std::printf("FAIL: brake did not reach Aborted (state=%d)\n",
                    static_cast<int>(brake.getCurrentState()));
        return 2;
    }

    // Late reply: motor processes the RpcRequest and sends the
    // response. It routes through linkTo → brake_router.dispatchToSender.
    // Since the correlation entry was cancelled, handleReply returns
    // false and the envelope falls through to dispatchEnvelope — but
    // brake is in the Aborted final state and ignores unknown events.
    motor.step();

    // Step brake one more time to confirm no done.invoke/error.invoke
    // was enqueued by the late reply (the correlation was cancelled).
    brake.step();

    if (brake.getCurrentState() != SCE::Generated::brake_invoke::State::Aborted) {
        std::printf("FAIL: brake left Aborted after late reply (state=%d). "
                    "cancelMeshRpc did not erase the correlation entry.\n",
                    static_cast<int>(brake.getCurrentState()));
        return 3;
    }

    std::printf("SCE Mesh §9.5 mesh-rpc cancel via abort: PASS\n");
    return 0;
}
