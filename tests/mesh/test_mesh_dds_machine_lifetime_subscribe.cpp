// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE-VERIFIES: mesh-13
//
// SCE Mesh §13 machine-lifetime subscription over DDS, end to end.
//
// The DDS half of the axis whose custom_tcp half is
// mesh_tcp_machine_lifetime_subscribe_runtime. Both flip a
// `supports_machine_lifetime_subscribe` flag whose contract is "realised
// end to end", and both drive a subscriber document carrying ZERO SCXML
// `<send>` elements — so anything the transport does is attributable to
// deploy.yaml `machines.<name>.subscriptions:` and to nothing else.
//
// What DDS proves that custom_tcp structurally cannot: over custom_tcp a
// subscription IS the connection, so a router that dropped its unsubscribe
// entirely would still stop receiving the moment it hung up, and the
// delivery side of teardown is unobservable. On DDS the subscription is a
// notification reader the router creates and destroys while the
// participant stays up (SCE_MESH.md §mesh-8.2) — so a publish issued after
// `shutdown()` is still a real publish, and whether it arrives is a fact
// about the reader rather than about the link.
//
// Three scenarios, one binary:
//
//   §1 The deploy.yaml-driven subscribe delivers. Neither brake document
//      sends anything, so the reader that receives the notification was
//      created by the subscription entry.
//
//   §2 The unsubscribe ARM destroys the reader, driven on its own rather
//      than through `shutdown()`. This distinction is not cosmetic and was
//      found by mutation: deleting the machine-lifetime unsubscribe from
//      the template leaves this fixture green if the assertion goes
//      through `shutdown()`, because the participant teardown later in the
//      same call destroys the reader anyway. Routing an `EventUnsubscribe`
//      through `route_send` directly is what isolates the arm from
//      everything else `shutdown()` does.
//
//      The control router pins WHY delivery stopped: a second subscriber
//      that is never touched receives the same publish, so the subject's
//      silence is its destroyed reader and not a publish that failed, a
//      discovery race, or an engine that stopped stepping.
//
//   §3 `shutdown()` stops delivery and is idempotent. `~TransportRouter`
//      calls it after any explicit call, so a second unsubscribe runs on
//      every router that is torn down properly — here it would be a second
//      `unsubscribe()` on an already-destroyed reader. Note what this does
//      NOT claim: on DDS the machine-lifetime unsubscribe emitted at
//      shutdown is indistinguishable in its effect from the participant
//      teardown that follows it in the same call, so §3 covers
//      `shutdown()` as a whole. The arm itself is §2's job.

#include "brake_dds_machine_lifetime_subscribe_transport.h"
#include "motor_dds_machine_lifetime_subscribe_sm.h"
#include "motor_dds_machine_lifetime_subscribe_transport.h"

#include "MeshTestUtils.h"

#include <chrono>
#include <cstdio>
#include <mutex>
#include <thread>

namespace {

using namespace SCE::Test::Mesh;
using namespace std::chrono_literals;

namespace brake_gen = SCE::Generated::brake_dds_machine_lifetime_subscribe;
namespace motor_gen = SCE::Generated::motor_dds_machine_lifetime_subscribe;

using MotorSm = motor_gen::motor_dds_machine_lifetime_subscribe;

// The subscription declared in deploy_dds_machine_lifetime_subscribe.yaml.
constexpr const char *kSubscribedEvent = "event.notification.status";

constexpr auto kPoll = 10ms;
constexpr int kWaitIters = 300;  // 3 s
// DDS discovery is asynchronous; readers and writers match through the
// participant discovery protocol rather than through a connect() the
// caller can wait on. Only used to settle matching before a publish, or to
// bound a negative assertion.
constexpr auto kDiscovery = 600ms;

std::size_t count_of(TestSenderEngine &engine) {
    std::lock_guard<std::mutex> lock(engine.received_.m);
    return engine.received_.events.size();
}

/// Step the motor engine until `p` holds or the budget runs out.
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

/// Drive the publisher and give the fabric time to deliver, without
/// asserting anything about who received it.
void publish_notification(MotorSm &motor) {
    motor.processEvent(motor_gen::Event::Sensor_update);
    for (int i = 0; i < 60; ++i) {
        motor.step();
        std::this_thread::sleep_for(kPoll);
    }
}

int run_test() {
    // Motor first: the publisher has to be discoverable before either
    // subscriber's reader can match it.
    MotorSm motor;
    motor.initialize();
    motor_gen::TransportRouter<MotorSm> motor_router({&motor});
    MESH_TEST_REQUIRE(motor_router.init(), "motor router init failed");

    // Two subscribers off the same deploy.yaml declaration. `subject` is
    // torn down mid-test; `control` never is, and exists so the negative
    // assertion in §2 has a reason attached to it.
    TestSenderEngine subject_engine;
    brake_gen::TransportRouter<TestSenderEngine> subject_router({&subject_engine});
    MESH_TEST_REQUIRE(subject_router.init(), "subject brake router init failed");

    TestSenderEngine control_engine;
    brake_gen::TransportRouter<TestSenderEngine> control_router({&control_engine});
    MESH_TEST_REQUIRE(control_router.init(), "control brake router init failed");

    std::this_thread::sleep_for(kDiscovery);

    // ── §1 the deploy.yaml-driven subscribe delivers ──────────
    motor.processEvent(motor_gen::Event::Sensor_update);
    MESH_TEST_REQUIRE(pump(motor, [&] { return count_of(subject_engine) > 0 && count_of(control_engine) > 0; }),
                      "the notification never reached a subscriber — the deploy.yaml subscription "
                      "created no notification reader");
    {
        std::lock_guard<std::mutex> lock(subject_engine.received_.m);
        MESH_TEST_REQUIRE(subject_engine.received_.events.front().type == kSubscribedEvent,
                          "subscriber engine received an event other than the subscribed one");
    }

    const std::size_t control_before = count_of(control_engine);
    const std::size_t subject_before = count_of(subject_engine);

    // ── §2 the unsubscribe arm alone destroys the reader ──────
    // Driven through `route_send` rather than `shutdown()` on purpose:
    // `shutdown()` also tears the participant down, which destroys the
    // reader regardless, so an assertion made through it survives deleting
    // the unsubscribe entirely. This is the same envelope the
    // machine-lifetime teardown emits — same type, same pattern, same
    // target — so what it exercises is the arm that teardown depends on.
    {
        SCE::Mesh::MeshEnvelope unsub;
        unsub.id = SCE::uuid::v7();
        unsub.source = "brake_dds_machine_lifetime_subscribe";
        unsub.type = kSubscribedEvent;
        unsub.pattern = SCE::Mesh::PatternKind::EventUnsubscribe;
        unsub.datacontenttype = SCE::Mesh::PayloadCodec::None;
        MESH_TEST_REQUIRE(subject_router.route_send("#motor", unsub),
                          "the dds arm rejected an EventUnsubscribe for the subscribed target");
    }
    std::this_thread::sleep_for(kDiscovery);

    publish_notification(motor);

    MESH_TEST_REQUIRE(count_of(control_engine) > control_before,
                      "the control subscriber received nothing either — the publish itself failed, "
                      "so the subject's silence below would prove nothing about its unsubscribe");
    MESH_TEST_REQUIRE(count_of(subject_engine) == subject_before,
                      "an unsubscribed router still received a notification — the dds unsubscribe "
                      "arm did not destroy the notification reader");

    // ── §3 shutdown() stops delivery and is idempotent ────────
    // The destructor calls it again on every properly torn-down router, so
    // a second unsubscribe on an already-destroyed reader is the ordinary
    // path, not an edge case. Scope note: on DDS the unsubscribe emitted
    // here cannot be told apart from the participant teardown that follows
    // it, so this covers `shutdown()` as a whole — the arm itself is §2.
    subject_router.shutdown();
    subject_router.shutdown();
    const std::size_t control_mid = count_of(control_engine);
    publish_notification(motor);
    MESH_TEST_REQUIRE(count_of(control_engine) > control_mid,
                      "the fabric stopped serving the control subscriber after two shutdown() calls "
                      "on the subject — the repeated teardown was not inert");
    MESH_TEST_REQUIRE(count_of(subject_engine) == subject_before, "the twice-shut-down router received a notification");

    control_router.shutdown();
    motor_router.shutdown();
    std::printf("SCE Mesh DDS machine-lifetime subscribe runtime E2E: PASS\n");
    return 0;
}

}  // namespace

int main() {
    try {
        return run_test();
    } catch (const std::exception &ex) {
        std::fprintf(stderr, "FAIL: uncaught exception: %s\n", ex.what());
        return 1;
    }
}
