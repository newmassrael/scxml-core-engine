// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE-VERIFIES: mesh-13
//
// SCE Mesh §13 machine-lifetime subscription over custom_tcp, end to end.
//
// This is the fixture the `supports_machine_lifetime_subscribe` flag in
// sce-build/src/mesh/transport/mod.rs waited on. The subscribe mechanism
// itself is transport-generic — `init()` builds an `EventSubscribe`
// envelope and hands it to the same `route_send` an SCXML `<send>` uses —
// but "transport-generic" is a structural argument, and the flag's
// contract is realised end to end. So the assertions below are made on
// the wire, against frames whose only possible origin is deploy.yaml.
//
// brake_tcp_machine_lifetime_subscribe.scxml carries ZERO `<send>`
// elements. Every framed envelope its router writes therefore comes from
// `machines.brake_tcp_machine_lifetime_subscribe.subscriptions:`. That is
// what makes §1's frame identity attributable — there is no second
// candidate producer to rule out.
//
// Three layers:
//
//   §1 Wire identity — a synthetic `CustomTcp::Server` standing in for
//      motor reads the subscribe frame directly: its `type` is the
//      NOTIFICATION event name (machine-lifetime subscriptions name what
//      they want to receive, not an `event.subscribe.*` verb) and its
//      pattern is `EventSubscribe`. Exactly one frame — an init() that
//      emitted the subscription twice would still deliver, so counting is
//      the only way to see it.
//
//   §2 Reverse leg + shutdown symmetry — notifications pushed back on the
//      observed link reach the engine, and `shutdown()` emits the paired
//      `EventUnsubscribe` frame. The unsubscribe half has a generated
//      emission site and, until this fixture, no reader: over custom_tcp
//      hanging up is itself an unsubscribe, so a router that silently
//      dropped the explicit frame would look identical from the server's
//      registry. Reading the frame off the wire before any teardown is
//      what separates the two. Shutdown is also asserted idempotent,
//      because `~TransportRouter` calls it after any explicit call and a
//      second unsubscribe would be a frame the server must not have to
//      tolerate.
//
//   §3 Engine E2E against the real motor router, plus the negative that
//      gives §1 its meaning: a peer that connects but never subscribes
//      receives nothing from the same fan-out that serves brake. Without
//      it, "brake received the notification" would also be satisfied by a
//      transport that broadcast to every open connection, which would
//      make the subscription itself dead weight.

#include "brake_tcp_machine_lifetime_subscribe_transport.h"
#include "motor_tcp_machine_lifetime_subscribe_sm.h"
#include "motor_tcp_machine_lifetime_subscribe_transport.h"

#include "MeshTestUtils.h"
#include "mesh/transports/CustomTcpTransport.h"

#include <chrono>
#include <cstdio>
#include <mutex>
#include <string>
#include <thread>
#include <vector>

#ifndef SCE_TEST_TCP_MACHINE_LIFETIME_PORT
#error "SCE_TEST_TCP_MACHINE_LIFETIME_PORT must be defined by the build system"
#endif
#define SCE_STRINGIFY_INNER(x) #x
#define SCE_STRINGIFY(x) SCE_STRINGIFY_INNER(x)
static constexpr const char *kEndpoint = "127.0.0.1:" SCE_STRINGIFY(SCE_TEST_TCP_MACHINE_LIFETIME_PORT);

namespace {

using namespace SCE::Test::Mesh;

namespace brake_gen = SCE::Generated::brake_tcp_machine_lifetime_subscribe;
namespace motor_gen = SCE::Generated::motor_tcp_machine_lifetime_subscribe;

using BrakeRouter = brake_gen::TransportRouter<TestSenderEngine>;
using MotorSm = motor_gen::motor_tcp_machine_lifetime_subscribe;
using MotorRouter = motor_gen::TransportRouter<MotorSm>;

// The subscription declared in deploy_tcp_machine_lifetime_subscribe.yaml.in.
constexpr const char *kSubscribedEvent = "event.notification.status";

constexpr int kBurstCount = 5;
constexpr auto kPoll = std::chrono::milliseconds(10);
constexpr int kWaitIters = 300;  // 3 s
// Grace for a frame to cross loopback and be decoded on the server's read
// thread. Only ever used to bound a *negative* assertion or to settle a
// registry write before the next step; positive assertions all wait on a
// predicate.
constexpr auto kSettle = std::chrono::milliseconds(150);

/// Records every envelope a synthetic peer receives together with the link
/// it arrived on, so the test can both assert on frame identity and answer
/// on the very stream the subscribe travelled.
struct WireLog {
    std::mutex m;
    std::condition_variable cv;
    std::vector<SCE::Mesh::MeshEnvelope> frames;
    SCE::Mesh::CustomTcp::PeerLink last_link;

    void push(const SCE::Mesh::MeshEnvelope &env, const SCE::Mesh::CustomTcp::PeerLink &link) {
        {
            std::lock_guard<std::mutex> lock(m);
            frames.push_back(env);
            last_link = link;
        }
        cv.notify_all();
    }

    bool waitForCount(std::size_t n, std::chrono::seconds timeout = kDefaultTimeout) {
        std::unique_lock<std::mutex> lock(m);
        return cv.wait_for(lock, timeout, [&] { return frames.size() >= n; });
    }

    std::size_t count() {
        std::lock_guard<std::mutex> lock(m);
        return frames.size();
    }

    SCE::Mesh::MeshEnvelope at(std::size_t i) {
        std::lock_guard<std::mutex> lock(m);
        return frames.at(i);
    }

    SCE::Mesh::CustomTcp::PeerLink link() {
        std::lock_guard<std::mutex> lock(m);
        return last_link;
    }
};

/// Step the motor engine until `p` holds or the budget runs out. The engine
/// is single-threaded by contract, so the test thread owns stepping while
/// transport callbacks only enqueue.
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

/// §1 + §2 — the subscribe and unsubscribe frames, read directly off the
/// wire by a synthetic server standing in for motor.
int wire_identity_and_shutdown_symmetry() {
    WireLog wire;
    SCE::Mesh::CustomTcp::Server motor_stub(
        kEndpoint, [&wire](const SCE::Mesh::MeshEnvelope &env, const SCE::Mesh::CustomTcp::PeerLink &link) {
            wire.push(env, link);
        });
    MESH_TEST_REQUIRE(motor_stub.valid(), "synthetic motor server failed to bind");

    TestSenderEngine brake_engine;
    BrakeRouter router({&brake_engine});

    // The subscribe is authored at init() time from deploy.yaml, not by any
    // SCXML <send> — the fixture document has none.
    MESH_TEST_REQUIRE(router.init(), "brake router.init() failed (dial?)");

    // §1 Frame identity.
    MESH_TEST_REQUIRE(wire.waitForCount(1), "init() put no frame on the wire — the deploy.yaml "
                                            "subscription never reached route_send");
    {
        const auto sub = wire.at(0);
        MESH_TEST_REQUIRE(sub.type == kSubscribedEvent,
                          "subscribe frame names the wrong event; a machine-lifetime subscribe "
                          "carries the notification event name it wants to receive");
        MESH_TEST_REQUIRE(sub.pattern == SCE::Mesh::PatternKind::EventSubscribe,
                          "frame reached the wire with a pattern other than EventSubscribe — the "
                          "server's registry keys on this and would never register the peer");
    }

    // Exactly one. A duplicate registration is invisible downstream (the
    // registry de-duplicates by link), so only the frame count can see it.
    std::this_thread::sleep_for(kSettle);
    MESH_TEST_REQUIRE(wire.count() == 1, "init() emitted more than one frame for a single "
                                         "deploy.yaml subscription entry");

    // §2a Reverse leg: the stream the subscribe arrived on carries the
    // notifications back.
    auto link = wire.link();
    MESH_TEST_REQUIRE(link.valid(), "no live peer link recorded for the subscribe frame");
    for (int i = 0; i < kBurstCount; ++i) {
        auto notify = make_envelope(kSubscribedEvent, SCE::Mesh::PatternKind::FireForget);
        MESH_TEST_REQUIRE(link.send(notify), "could not push a notification back on the subscribe stream");
    }
    MESH_TEST_REQUIRE(brake_engine.received_.wait_for([](const auto &v) {
        if (v.size() < static_cast<std::size_t>(kBurstCount)) {
            return false;
        }
        for (const auto &ev : v) {
            if (ev.type != kSubscribedEvent) {
                return false;
            }
        }
        return true;
    }),
                      "the subscriber engine did not receive the full notification burst");

    // §2b Shutdown emits the paired unsubscribe — before any socket
    // teardown, so the frame is observed rather than inferred.
    router.shutdown();
    MESH_TEST_REQUIRE(wire.waitForCount(2), "shutdown() emitted no unsubscribe frame");
    {
        const auto unsub = wire.at(1);
        MESH_TEST_REQUIRE(unsub.type == kSubscribedEvent, "unsubscribe frame names a different event than the "
                                                          "subscribe it retires — the server keys both on the "
                                                          "same axis and would leave the registration in place");
        MESH_TEST_REQUIRE(unsub.pattern == SCE::Mesh::PatternKind::EventUnsubscribe,
                          "shutdown frame is not an EventUnsubscribe");
    }

    // §2c Idempotence. `~TransportRouter` calls shutdown() after any
    // explicit call, so a non-idempotent shutdown puts a second
    // unsubscribe on the wire for every router that is shut down properly.
    router.shutdown();
    std::this_thread::sleep_for(kSettle);
    MESH_TEST_REQUIRE(wire.count() == 2, "a second shutdown() emitted another unsubscribe frame — "
                                         "shutdown must be idempotent because the destructor calls it");

    return 0;
}

/// §3 — engine-to-engine against the real motor router, with the negative
/// that makes the positive mean something.
int engine_e2e_and_unsubscribed_peer() {
    MotorSm motor;
    MotorRouter motor_router({&motor});
    MESH_TEST_REQUIRE(motor_router.init(), "motor router.init() failed (listen bind?)");
    motor.initialize();

    // A peer that connects and stays silent. It shares the fan-out path
    // with brake and differs on exactly one axis: it never subscribed.
    WireLog silent;
    SCE::Mesh::CustomTcp::Client silent_peer(
        kEndpoint, [&silent](const SCE::Mesh::MeshEnvelope &env, const SCE::Mesh::CustomTcp::PeerLink &link) {
            silent.push(env, link);
        });

    TestSenderEngine brake_engine;
    BrakeRouter brake_router({&brake_engine});
    MESH_TEST_REQUIRE(brake_router.init(), "brake router.init() failed (dial?)");

    // Let both the subscribe and the silent peer's connection settle into
    // the server before the notification is raised. Without this the
    // negative assertion could pass because the silent peer had not
    // finished connecting, which is a different reason than the one under
    // test.
    std::this_thread::sleep_for(kSettle);
    motor.step();

    motor.processEvent(motor_gen::Event::Sensor_update);

    MESH_TEST_REQUIRE(pump(motor,
                           [&] {
                               std::lock_guard<std::mutex> lock(brake_engine.received_.m);
                               return !brake_engine.received_.events.empty();
                           }),
                      "the notification never reached the subscriber engine over the "
                      "deploy.yaml-driven registration");
    {
        std::lock_guard<std::mutex> lock(brake_engine.received_.m);
        MESH_TEST_REQUIRE(brake_engine.received_.events.front().type == kSubscribedEvent,
                          "subscriber engine received an event other than the subscribed one");
    }

    MESH_TEST_REQUIRE(silent.count() == 0, "a peer that never subscribed received the notification — "
                                           "fan-out is per connection, not per subscription, which "
                                           "would make the subscription declaration dead weight");

    // The assertion above is `count() == 0`, which a peer that never
    // managed to connect would also satisfy — and then it would prove
    // nothing about the registry. Pair it with the retirement of that
    // doubt: the same peer subscribes and is served on the same
    // connection. Delivery here means the silence above was the absence
    // of a subscription, not the absence of a link.
    MESH_TEST_REQUIRE(silent_peer.send(make_envelope("event.subscribe.status", SCE::Mesh::PatternKind::EventSubscribe)),
                      "the silent peer's connection was never usable — the negative assertion above "
                      "held for the wrong reason");
    std::this_thread::sleep_for(kSettle);
    motor.step();
    motor.processEvent(motor_gen::Event::Sensor_update);
    MESH_TEST_REQUIRE(pump(motor, [&] { return silent.count() > 0; }),
                      "the formerly silent peer subscribed on a live connection and still received "
                      "nothing — the negative assertion above cannot be attributed to the missing "
                      "subscription");

    brake_router.shutdown();
    motor_router.shutdown();
    return 0;
}

}  // namespace

int main() {
    try {
        if (const int rc = wire_identity_and_shutdown_symmetry(); rc != 0) {
            return rc;
        }
        // The synthetic server is destroyed with its scope; SO_REUSEADDR
        // lets the real motor router take the same port immediately.
        if (const int rc = engine_e2e_and_unsubscribed_peer(); rc != 0) {
            return rc;
        }
        std::printf("SCE Mesh custom_tcp machine-lifetime subscribe runtime E2E: PASS\n");
        return 0;
    } catch (const std::exception &ex) {
        std::fprintf(stderr, "FAIL: uncaught exception: %s\n", ex.what());
        return 1;
    }
}
