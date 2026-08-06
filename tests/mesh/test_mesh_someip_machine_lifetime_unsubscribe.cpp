// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE-VERIFIES: mesh-13
//
// SCE Mesh §mesh-13 machine-lifetime subscribe / unsubscribe, observed as
// frames on the wire.
//
// `mesh_someip_machine_lifetime_subscribe_verification` is compile-only
// (static_asserts over the resolved ids), so before this file the SOME/IP
// machine-lifetime axis had no runtime coverage at all: nothing checked
// that the deploy.yaml `subscriptions:` entry reached the wire, or that
// its paired retraction did.
//
// SOME/IP is where the retraction is observable as a FRAME rather than
// as an absence. A subscription is an SD SubscribeEventgroup addressed to
// the offering application, and vsomeip hands the offerer a callback on
// every transition through `register_subscription_handler` —
// `subscribed == true` on subscribe, `false` on unsubscribe. That is a
// direct read of what went on the wire rather than an inference from what
// stopped arriving, which is what every sibling fixture measures.
//
// Two scenarios, one binary:
//
//   §1 `init()` puts exactly one subscribe on the wire. The subscriber
//      document declares no `<send>` at all, so the only thing that could
//      have produced it is the deploy.yaml `subscriptions:` entry.
//
//   §2 The unsubscribe ARM puts exactly one retraction on the wire,
//      driven through `route_send` while the router is still alive.
//
//      Driving it through `shutdown()` instead does not work, and the
//      failure is instructive: `shutdown()` stops the vsomeip
//      application, and vsomeip reports a departing client to the
//      offerer as an unsubscribe transition. Deleting the
//      machine-lifetime unsubscribe from the template outright leaves a
//      shutdown()-based assertion GREEN — verified by mutation, and the
//      same confound the DDS sibling found on its own teardown path.
//
// What this file deliberately does NOT assert: that a second retraction
// never reaches the wire. vsomeip absorbs a duplicate `unsubscribe()` on
// an already-unsubscribed eventgroup, so removing the
// `machine_subscriptions_armed_` guard leaves the observed count at one
// here (also verified by mutation). The frame-count claim the guard exists
// for is asserted where the retraction is a framed message the test can
// count on a raw socket — `mesh_tcp_machine_lifetime_subscribe` §2c.
//
// Upstream note: the non-deprecated `subscription_handler_sec_t` overload
// of `register_subscription_handler` segfaults inside
// vsomeip 3.7.3 (`application_impl` map insertion) before `start()` is
// reached. The deprecated `(client, uid, gid, subscribed)` overload works,
// so this fixture uses it behind a scoped pragma rather than dropping the
// only frame-level observation available.

#include "brake_someip_machine_lifetime_subscribe_transport.h"

#include "MeshTestUtils.h"
#include "SomeipTestUtils.h"

#include <vsomeip/vsomeip.hpp>

#include <atomic>
#include <chrono>
#include <cstdio>
#include <thread>

namespace {

using namespace SCE::Test::Mesh;
using namespace std::chrono_literals;

namespace brake_gen = SCE::Generated::brake_someip_machine_lifetime_subscribe;

// Resolved from vsomeip_someip_machine_lifetime.json by codegen; the
// harness offers exactly what the subscriber was built to want, so a
// mismatch surfaces as "no subscribe observed" rather than as a silent
// pass against a different eventgroup.
constexpr vsomeip::service_t kService = brake_gen::SOMEIP_SERVICE_MOTOR;
constexpr vsomeip::instance_t kInstance = brake_gen::SOMEIP_INSTANCE_MOTOR;
constexpr vsomeip::eventgroup_t kEventGroup = brake_gen::SOMEIP_EVENT_GROUP_MOTOR_EVENT_NOTIFICATION_VEHICLE_SPEED;

/// Stops and joins the harness on every exit path, so an assertion
/// failure stays legible instead of becoming `terminate called without
/// an active exception`.
struct HarnessRunner {
    std::shared_ptr<vsomeip::application> app;
    std::thread t;

    explicit HarnessRunner(std::shared_ptr<vsomeip::application> a) : app(std::move(a)), t([this] { app->start(); }) {}

    ~HarnessRunner() {
        app->stop();
        if (t.joinable()) {
            t.join();
        }
    }
};

/// Wait until `p` holds or the shared mesh timeout runs out.
template <typename Predicate> bool wait_for(Predicate p) {
    const auto deadline = std::chrono::steady_clock::now() + kDefaultTimeout;
    while (std::chrono::steady_clock::now() < deadline) {
        if (p()) {
            return true;
        }
        std::this_thread::sleep_for(5ms);
    }
    return false;
}

// A negative assertion needs a settle window. Both counts below are
// driven by the same local routing manager that delivered the positive
// halves, so anything a duplicate emitted would land inside it.
constexpr auto kFrameSettle = 500ms;

int run_test() {
    wipe_stale_vsomeip_sockets();

    std::atomic<int> subscribes{0};
    std::atomic<int> unsubscribes{0};

    // The harness owns routing and offers the eventgroup the subscriber
    // wants, so it is the application vsomeip reports subscription
    // transitions to.
    auto harness = vsomeip::runtime::get()->create_application("ml_unsub_harness");
    MESH_TEST_REQUIRE(harness->init(), "vsomeip harness init failed");

    // Order matters, and not for a cosmetic reason: vsomeip keys its
    // subscription handlers off the offered (service, instance), so
    // registering before the offer exists reaches into a map that has no
    // entry for them.
    harness->offer_service(kService, kInstance);
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wdeprecated-declarations"
    harness->register_subscription_handler(
        kService, kInstance, kEventGroup,
        [&subscribes, &unsubscribes](vsomeip::client_t, vsomeip::uid_t, vsomeip::gid_t, bool subscribed) {
            if (subscribed) {
                subscribes.fetch_add(1);
            } else {
                unsubscribes.fetch_add(1);
            }
            return true;  // accept
        });
#pragma GCC diagnostic pop

    HarnessRunner harness_runner(harness);

    // ── §1 init() emits exactly one subscribe ──────────────────
    TestSenderEngine sender;
    brake_gen::TransportRouter<TestSenderEngine> router({&sender});
    MESH_TEST_REQUIRE(router.init(), "subscriber router init failed");

    MESH_TEST_REQUIRE(wait_for([&] { return subscribes.load() > 0; }),
                      "the offering application saw no SubscribeEventgroup — the deploy.yaml "
                      "`subscriptions:` entry did not reach the wire at init()");
    std::this_thread::sleep_for(kFrameSettle);
    MESH_TEST_REQUIRE(subscribes.load() == 1,
                      "init() emitted more than one subscribe for a single `subscriptions:` entry");
    MESH_TEST_REQUIRE(unsubscribes.load() == 0, "a subscribe was immediately retracted");

    // ── §2 the unsubscribe arm, isolated from shutdown() ───────
    // The router stays alive across this assertion on purpose: stopping
    // the vsomeip application makes the client's departure produce the
    // same transition, which is what lets a deleted arm pass.
    auto unsub = make_envelope("event.notification.vehicle_speed", SCE::Mesh::PatternKind::EventUnsubscribe);
    MESH_TEST_REQUIRE(router.route_send("#motor", unsub),
                      "the someip EventUnsubscribe arm refused the machine-lifetime retraction");
    MESH_TEST_REQUIRE(wait_for([&] { return unsubscribes.load() > 0; }),
                      "the offering application saw no unsubscribe — the retraction did not reach "
                      "the wire as an SD frame");
    std::this_thread::sleep_for(kFrameSettle);
    MESH_TEST_REQUIRE(unsubscribes.load() == 1, "one retraction produced more than one unsubscribe transition");
    MESH_TEST_REQUIRE(subscribes.load() == 1, "the retraction was followed by a re-subscribe");

    router.shutdown();
    std::printf("SCE Mesh §13 someip machine-lifetime unsubscribe: PASS\n");
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
