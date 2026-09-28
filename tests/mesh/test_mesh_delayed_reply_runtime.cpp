// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE-VERIFIES: mesh-9.5
//
// SCE_MESH.md §9.5 mesh-rpc round trip whose reply is a DELAYED <send> to a
// Mesh peer, run twice: with the peer written in `target`, and read from a
// `targetexpr` (§scxml-6.2.4: the same value routes the same way).
//
// W3C SCXML 6.2: a delay postpones a send; it does not change where the send
// goes. The C++ runtime used to schedule a delayed `#<peer>` send into the
// sender's own external queue, and to deliver a delayed `targetexpr` peer at
// once — the first never reached the peer, the second never waited. Checked
// for each pair:
//
//   1. Before the delay elapses the requester has NOT been answered: the
//      delay is honoured, not ignored by an immediate delivery.
//   2. After it elapses and the responder's scheduler is ticked, the reply
//      crosses to the requester and is accepted as the answer to ITS request,
//      which needs the invokeid captured when the send was made
//      (SCE_MESH.md §10.7). An event without an invokeid is processed in
//      between, so an invokeid read at the deadline would be the wrong one.

#include "brake_delayed_reply_expr_sm.h"
#include "brake_delayed_reply_expr_transport.h"
#include "brake_delayed_reply_sm.h"
#include "brake_delayed_reply_transport.h"
#include "motor_delayed_reply_expr_sm.h"
#include "motor_delayed_reply_expr_transport.h"
#include "motor_delayed_reply_sm.h"
#include "motor_delayed_reply_transport.h"

#include "common/TestScriptEngine.h"

#include <chrono>
#include <cstdio>
#include <thread>

namespace {

namespace Gen = SCE::Generated;

/// The peer written in `target`.
struct WrittenPeer {
    static constexpr const char *label = "target";
    static constexpr const char *brakeAddress = "#brake_delayed_reply";
    static constexpr const char *motorAddress = "#motor_delayed_reply";
    using Brake = Gen::brake_delayed_reply::brake_delayed_reply;
    using Motor = Gen::motor_delayed_reply::motor_delayed_reply;
    using BrakeState = Gen::brake_delayed_reply::State;
    using MotorState = Gen::motor_delayed_reply::State;
    using BrakeEvent = Gen::brake_delayed_reply::Event;
    using MotorEvent = Gen::motor_delayed_reply::Event;
    using BrakeRouter = Gen::brake_delayed_reply::TransportRouter<Brake, Motor>;
    using MotorRouter = Gen::motor_delayed_reply::TransportRouter<Motor, Brake>;
};

/// The peer read from a `targetexpr`.
struct EvaluatedPeer {
    static constexpr const char *label = "targetexpr";
    static constexpr const char *brakeAddress = "#brake_delayed_reply_expr";
    static constexpr const char *motorAddress = "#motor_delayed_reply_expr";
    using Brake = Gen::brake_delayed_reply_expr::brake_delayed_reply_expr;
    using Motor = Gen::motor_delayed_reply_expr::motor_delayed_reply_expr;
    using BrakeState = Gen::brake_delayed_reply_expr::State;
    using MotorState = Gen::motor_delayed_reply_expr::State;
    using BrakeEvent = Gen::brake_delayed_reply_expr::Event;
    using MotorEvent = Gen::motor_delayed_reply_expr::Event;
    using BrakeRouter = Gen::brake_delayed_reply_expr::TransportRouter<Brake, Motor>;
    using MotorRouter = Gen::motor_delayed_reply_expr::TransportRouter<Motor, Brake>;
};

template <typename Pair> int roundTrip() {
    using BrakeState = typename Pair::BrakeState;
    using MotorState = typename Pair::MotorState;

    typename Pair::Brake brake;
    typename Pair::Motor motor;
    typename Pair::BrakeRouter brake_router({&brake}, motor);
    typename Pair::MotorRouter motor_router({&motor}, brake);

    brake_router.linkTo(Pair::motorAddress, [&motor_router](const SCE::Mesh::MeshEnvelope &env) {
        return motor_router.dispatchToSession(env, 0);
    });
    motor_router.linkTo(Pair::brakeAddress, [&brake_router](const SCE::Mesh::MeshEnvelope &env) {
        return brake_router.dispatchToSession(env, 0);
    });

    // The `targetexpr` responder evaluates its target; a machine that needs no
    // script engine takes none.
    SCE::Test::inject_build_engine(brake);
    SCE::Test::inject_build_engine(motor);
    brake.initialize();
    motor.initialize();

    brake.processEvent(Pair::BrakeEvent::Go);
    if (brake.getCurrentState() != BrakeState::Computing) {
        std::printf("FAIL [%s]: brake did not enter Computing (state=%d)\n", Pair::label,
                    static_cast<int>(brake.getCurrentState()));
        return 1;
    }

    motor.tick();
    if (motor.getCurrentState() != MotorState::Replying) {
        std::printf("FAIL [%s]: motor did not enter Replying (state=%d)\n", Pair::label,
                    static_cast<int>(motor.getCurrentState()));
        return 2;
    }

    // Another event reaches the responder before the reply is due. It carries
    // no invokeid, so it replaces the current event's: a reply that read the
    // invokeid at the deadline instead of at the send would answer nobody.
    motor.processEvent(Pair::MotorEvent::Service_request_compute_force);
    if (motor.getCurrentState() != MotorState::Replying) {
        std::printf("FAIL [%s]: motor left Replying on an event it has no transition for (state=%d)\n", Pair::label,
                    static_cast<int>(motor.getCurrentState()));
        return 6;
    }

    // The delay has not elapsed: nothing may have reached the requester yet.
    motor.tick();
    brake.step();
    if (brake.getCurrentState() != BrakeState::Computing) {
        std::printf("FAIL [%s]: brake left Computing before the reply's delay elapsed (state=%d)\n", Pair::label,
                    static_cast<int>(brake.getCurrentState()));
        return 3;
    }

    // Wait until the responder's own scheduler says the reply is due, bounded
    // well below the requester's deadline.
    const auto giveUp = std::chrono::steady_clock::now() + std::chrono::seconds(2);
    while (motor.timeUntilNextScheduled().value_or(std::chrono::milliseconds::zero()) >
           std::chrono::milliseconds::zero()) {
        if (std::chrono::steady_clock::now() > giveUp) {
            std::printf("FAIL [%s]: the reply never came due in the responder's scheduler\n", Pair::label);
            return 4;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(5));
    }
    motor.tick();
    brake.step();

    if (brake.terminalState() != BrakeState::Ok) {
        std::printf("FAIL [%s]: brake did not reach Ok on the delayed reply (state=%d). A delayed <send> to a Mesh "
                    "peer must reach that peer when its delay elapses, with the invokeid of the request it "
                    "answers.\n",
                    Pair::label, static_cast<int>(brake.getCurrentState()));
        return 5;
    }
    return 0;
}

}  // namespace

int main() {
    if (const int rc = roundTrip<WrittenPeer>(); rc != 0) {
        return rc;
    }
    if (const int rc = roundTrip<EvaluatedPeer>(); rc != 0) {
        return 10 + rc;
    }
    std::printf("SCE Mesh §9.5 delayed reply round-trip (target, targetexpr): PASS\n");
    return 0;
}
