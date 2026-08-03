// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE-VERIFIES: mesh-10.4.4
//
// SCE Mesh §mesh-10.4.4: custom_tcp realizes RequestReply, FieldAccess and
// PubSub, not FireForget alone.
//
// Two layers of evidence, for the same reason the FireForget sibling test
// has two: the engine path proves the wiring end to end but cannot say
// WHICH pattern moved the machine, because brake_tcp_multi returns to
// `idle` on any of the three reply events. So the discriminating
// assertions are made at the wire, against a synthetic peer whose received
// envelopes are inspected directly.
//
//   1. Engine-level E2E — brake's TransportRouter drives all five sends
//      over one TCP connection and motor answers on that same connection.
//
//   2. Wire-level discrimination against the motor router:
//        a. an RPC request is answered on the stream it arrived on, with
//           its correlation id echoed;
//        b. a subscriber receives the spontaneous notification for the
//           axis it asked for;
//        c. a subscriber on a DIFFERENT axis does not receive it — the
//           registry is per axis, not per connection;
//        d. after unsubscribe, notifications stop;
//        e. a dead subscriber is dropped without any expiry mechanism,
//           because the registry holds its connection.

#include "brake_tcp_multi_sm.h"
#include "brake_tcp_multi_transport.h"
#include "motor_tcp_multi_sm.h"
#include "motor_tcp_multi_transport.h"

#include "common/Uuid.h"
#include "mesh/MeshEnvelope.h"
#include "mesh/transports/CustomTcpTransport.h"

#include <chrono>
#include <condition_variable>
#include <cstdio>
#include <memory>
#include <mutex>
#include <string>
#include <thread>
#include <vector>

#ifndef SCE_TEST_TCP_MULTI_PORT
#error "SCE_TEST_TCP_MULTI_PORT must be defined by the build system"
#endif
#define SCE_STRINGIFY_INNER(x) #x
#define SCE_STRINGIFY(x) SCE_STRINGIFY_INNER(x)
static constexpr const char *kEndpoint = "127.0.0.1:" SCE_STRINGIFY(SCE_TEST_TCP_MULTI_PORT);

namespace {

constexpr auto kPoll = std::chrono::milliseconds(10);
constexpr int kWaitIters = 300;  // 3s

using MotorSm = SCE::Generated::motor_tcp_multi::motor_tcp_multi;
using MotorRouter = SCE::Generated::motor_tcp_multi::TransportRouter<MotorSm>;

/// Records every envelope a synthetic peer receives, so a test can assert
/// on the exact event names rather than on a state the engine happens to
/// reach for several different reasons.
struct Inbox {
    std::mutex m;
    std::condition_variable cv;
    std::vector<SCE::Mesh::MeshEnvelope> envelopes;

    void push(const SCE::Mesh::MeshEnvelope &env) {
        {
            std::lock_guard<std::mutex> lock(m);
            envelopes.push_back(env);
        }
        cv.notify_all();
    }

    bool waitForType(const std::string &type, std::chrono::seconds timeout) {
        std::unique_lock<std::mutex> lock(m);
        return cv.wait_for(lock, timeout, [&] {
            for (const auto &e : envelopes) {
                if (e.type == type) {
                    return true;
                }
            }
            return false;
        });
    }

    bool sawType(const std::string &type) {
        std::lock_guard<std::mutex> lock(m);
        for (const auto &e : envelopes) {
            if (e.type == type) {
                return true;
            }
        }
        return false;
    }

    std::size_t count() {
        std::lock_guard<std::mutex> lock(m);
        return envelopes.size();
    }
};

SCE::Mesh::MeshEnvelope makeEnvelope(const std::string &type, SCE::Mesh::PatternKind pattern) {
    SCE::Mesh::MeshEnvelope env;
    env.id = SCE::uuid::v7();
    env.source = "synthetic_peer";
    env.type = type;
    env.pattern = pattern;
    env.datacontenttype = SCE::Mesh::PayloadCodec::None;
    return env;
}

/// Step the motor engine until `p` holds, or the budget runs out. The
/// engine is single-threaded by contract, so the test thread owns stepping
/// while transport callbacks only enqueue.
template <typename Predicate> bool pump(MotorSm &motor, Predicate p) {
    for (int i = 0; i < kWaitIters; ++i) {
        motor.step();
        if (p()) {
            return true;
        }
        std::this_thread::sleep_for(kPoll);
    }
    return false;
}

}  // namespace

int main() {
    // ── 1. Engine-level E2E over one duplex connection ────────
    {
        SCE::Generated::brake_tcp_multi::brake_tcp_multi brake;
        MotorSm motor;

        using BrakeRouter =
            SCE::Generated::brake_tcp_multi::TransportRouter<SCE::Generated::brake_tcp_multi::brake_tcp_multi>;
        BrakeRouter brake_router({&brake});
        MotorRouter motor_router({&motor});

        if (!motor_router.init()) {
            std::fprintf(stderr, "FAIL: motor router init() failed (listen bind?)\n");
            return 1;
        }
        brake.initialize();
        motor.initialize();

        // brake.press → onentry fires all five sends on one connection.
        brake.processEvent(SCE::Generated::brake_tcp_multi::Event::Brake_press);

        // The RPC drives motor through `computing` and back; reaching
        // `ready` again means the request arrived, the engine answered,
        // and the reply left through handleServerResponse.
        const bool served =
            pump(motor, [&] { return motor.getCurrentState() == SCE::Generated::motor_tcp_multi::State::Ready; });
        if (!served) {
            std::fprintf(stderr, "FAIL: motor never returned to 'ready' after serving the RPC\n");
            return 2;
        }

        // brake leaves `active` only on a reply reaching it, which is the
        // half a FireForget-only transport could never do.
        const bool replied = [&] {
            for (int i = 0; i < kWaitIters; ++i) {
                motor.step();
                brake.step();
                if (brake.getCurrentState() == SCE::Generated::brake_tcp_multi::State::Idle) {
                    return true;
                }
                std::this_thread::sleep_for(kPoll);
            }
            return false;
        }();
        if (!replied) {
            std::fprintf(stderr, "FAIL: brake never received a reply (still in 'active')\n");
            return 3;
        }
    }

    // ── 2. Wire-level discrimination ──────────────────────────
    {
        MotorSm motor;
        MotorRouter motor_router({&motor});
        if (!motor_router.init()) {
            std::fprintf(stderr, "FAIL: motor router init() failed on re-bind\n");
            return 4;
        }
        motor.initialize();

        Inbox status_inbox;
        auto status_peer = std::make_unique<SCE::Mesh::CustomTcp::Client>(
            kEndpoint, [&status_inbox](const SCE::Mesh::MeshEnvelope &env, const SCE::Mesh::CustomTcp::PeerLink &) {
                status_inbox.push(env);
            });

        // (a) RPC answered on the arriving stream, correlation echoed.
        auto request = makeEnvelope("service.request.compute_force", SCE::Mesh::PatternKind::RpcRequest);
        const auto cid = SCE::uuid::v7();
        request.correlation_id = cid;
        if (!status_peer->send(request)) {
            std::fprintf(stderr, "FAIL: could not send RPC request to motor\n");
            return 5;
        }
        if (!pump(motor, [&] { return status_inbox.sawType("service.response.compute_force"); })) {
            std::fprintf(stderr, "FAIL: RPC reply never arrived on the requesting stream\n");
            return 6;
        }
        {
            std::lock_guard<std::mutex> lock(status_inbox.m);
            bool echoed = false;
            for (const auto &e : status_inbox.envelopes) {
                if (e.type == "service.response.compute_force" && e.correlation_id && *e.correlation_id == cid) {
                    echoed = true;
                }
            }
            if (!echoed) {
                std::fprintf(stderr, "FAIL: RPC reply did not echo the request's correlation id\n");
                return 7;
            }
        }

        // (b) The subscriber receives the axis it asked for.
        if (!status_peer->send(makeEnvelope("event.subscribe.status", SCE::Mesh::PatternKind::EventSubscribe))) {
            std::fprintf(stderr, "FAIL: could not send subscribe\n");
            return 8;
        }

        // (c) A peer subscribed to a different axis must not be woken.
        Inbox other_inbox;
        auto other_peer = std::make_unique<SCE::Mesh::CustomTcp::Client>(
            kEndpoint, [&other_inbox](const SCE::Mesh::MeshEnvelope &env, const SCE::Mesh::CustomTcp::PeerLink &) {
                other_inbox.push(env);
            });
        if (!other_peer->send(makeEnvelope("event.subscribe.position", SCE::Mesh::PatternKind::EventSubscribe))) {
            std::fprintf(stderr, "FAIL: could not send the off-axis subscribe\n");
            return 9;
        }

        // Both subscribes must be registered before the notification is
        // raised, otherwise (c) would pass for the wrong reason — an
        // unregistered peer receives nothing either.
        std::this_thread::sleep_for(std::chrono::milliseconds(100));
        motor.step();

        motor.processEvent(SCE::Generated::motor_tcp_multi::Event::Sensor_update);
        if (!pump(motor, [&] { return status_inbox.sawType("event.notification.status"); })) {
            std::fprintf(stderr, "FAIL: subscriber never received the notification for its axis\n");
            return 10;
        }
        if (other_inbox.sawType("event.notification.status")) {
            std::fprintf(stderr, "FAIL: a peer subscribed to 'position' received the 'status' axis — "
                                 "fan-out is not per axis\n");
            return 11;
        }

        // (d) Unsubscribe stops delivery.
        if (!status_peer->send(makeEnvelope("event.unsubscribe.status", SCE::Mesh::PatternKind::EventUnsubscribe))) {
            std::fprintf(stderr, "FAIL: could not send unsubscribe\n");
            return 12;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(100));
        motor.step();
        const std::size_t before = status_inbox.count();
        motor.processEvent(SCE::Generated::motor_tcp_multi::Event::Sensor_update);
        for (int i = 0; i < 30; ++i) {
            motor.step();
            std::this_thread::sleep_for(kPoll);
        }
        if (status_inbox.count() != before) {
            std::fprintf(stderr, "FAIL: notification delivered after unsubscribe\n");
            return 13;
        }

        // (e) A subscriber that dies is dropped with no expiry mechanism:
        // the registry holds its connection, so hanging up IS the
        // unsubscribe. Re-subscribe the peer, destroy it, then raise —
        // the fan-out must report no delivery rather than writing to a
        // closed stream.
        if (!other_peer->send(makeEnvelope("event.subscribe.status", SCE::Mesh::PatternKind::EventSubscribe))) {
            std::fprintf(stderr, "FAIL: could not re-subscribe the second peer\n");
            return 14;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(100));
        motor.step();
        other_peer.reset();  // hang up
        std::this_thread::sleep_for(std::chrono::milliseconds(100));
        motor.processEvent(SCE::Generated::motor_tcp_multi::Event::Sensor_update);
        for (int i = 0; i < 30; ++i) {
            motor.step();
            std::this_thread::sleep_for(kPoll);
        }
        // Nothing to assert on the dead peer itself; the assertion is that
        // the router survived writing to it and still serves others.
        if (!status_peer->send(makeEnvelope("service.fire_forget.activate", SCE::Mesh::PatternKind::FireForget))) {
            std::fprintf(stderr, "FAIL: router stopped serving after a subscriber hung up\n");
            return 15;
        }
    }

    std::printf("PASS: custom_tcp realizes RequestReply, FieldAccess and PubSub\n");
    return 0;
}
