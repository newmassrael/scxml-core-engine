// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh gap S6: SOME/IP EventUnsubscribe E2E verification.
//
// Proves that the generated `app.unsubscribe(...)` call on
// PatternKind::EventUnsubscribe actually retracts the subscription so
// subsequent publisher notifications do NOT reach the subscriber.
// The codegen path for this was added in Session C (skip_receive_handler
// flag stops duplicate register_message_handler on the shared
// (service, instance, event_id) triple), but no test exercised the
// unsubscribe tail end-to-end until this fixture landed. Closes the
// silent-broken-hook risk flagged in feedback_silently_broken_hooks.md.
//
// Fixture pair is dedicated (not shared with the Session C multi
// fixture) so this test owns its assertions end-to-end. The new pair
// exercises only subscribe/unsubscribe on a single eventgroup —
// writing this test exposed a pre-existing bug in
// `inject_server_model_mutations` (fix committed separately) where
// eventgroup-only servers missed the synthetic <send> injection.
//
// Shape (mirrors test_mesh_someip_eventgroup_engine_driven.cpp):
//   - Brake is TestSenderEngine — the mesh send callback is invoked
//     directly for EventSubscribe / EventUnsubscribe, so we do not
//     need a real SM on the sender side. brake_engine.received_
//     captures every envelope delivered through the generated
//     register_message_handler → dispatchToSender path.
//   - Motor is a real SM driven by a step() pump. `sensor.update`
//     dispatched via motor_router.dispatchToSender fires the onentry
//     <raise event="field.notify.vehicle_speed"/>, which injects a
//     synthetic <send target="#motor"> routed through
//     publishEventgroupNotify → app.notify on speed_group.
//
// Negative-assertion protocol:
//   1. Drain barrier: subscribe → publish N → require N received.
//      Proves the notify path was live before unsubscribe, so a
//      post-unsubscribe zero count is not a false positive
//      (`feedback_built_but_unconsumed.md` — a notify path that
//      was never live to begin with would pass this test trivially
//      without the barrier).
//   2. Unsubscribe → publish M → wait past last publish + slack.
//   3. Assert received count unchanged from the drain-barrier baseline.
//
// Limits of the approach (recorded so future readers do not overtune):
//   The post-unsubscribe absence is asserted via a fixed slack after
//   the last publish. vsomeip does not expose a hook that signals
//   "all subscribers have observed the retraction", so the slack is
//   heuristic. Under extreme CI load a late in-flight notify could
//   still arrive past the slack and cause a false pass. The drain
//   barrier is the primary correctness guard; the slack is an
//   additional timing margin on top.

#include "brake_someip_unsub_sm.h"
#include "brake_someip_unsub_transport.h"
#include "motor_someip_unsub_sm.h"
#include "motor_someip_unsub_transport.h"

#include "MeshTestUtils.h"
#include "SomeipTestUtils.h"
#include "common/Uuid.h"

#include <atomic>
#include <chrono>
#include <condition_variable>
#include <cstdio>
#include <mutex>
#include <string>
#include <thread>

namespace {

using namespace SCE::Test::Mesh;

// ── Timing budget (single pane of glass) ────────────────────────
// All sleep/deadline values live together so a reader can size the
// whole test budget at a glance. Adjust as a group if vsomeip timing
// characteristics change; adjusting one in isolation is exactly the
// flake shape these named constants prevent.

// vsomeip subscribe/unsubscribe propagation budget. Internal routing
// (service-discovery disabled) finishes well under this in practice;
// raising this does not mask any correctness bug, it only extends
// test runtime on slow CI hosts.
constexpr auto kVsomeipPropagationSlack = std::chrono::milliseconds(100);

// Shorter settle after the first subscribe — the drain barrier below
// retries the notify up to kDrainBarrierDeadline so this only needs
// to clear the "subscribe hasn't been dispatched yet" window.
constexpr auto kSubscribeSettle = std::chrono::milliseconds(30);

// Drain barrier deadline. Must exceed the slowest plausible round
// trip of (dispatchToSender → motor.step → raise → mesh send →
// publishEventgroupNotify → vsomeip notify → brake handler) times
// the drain count. kDefaultTimeout (5s) is plenty.
constexpr auto kDrainBarrierDeadline = kDefaultTimeout;

// After unsubscribe, the publisher drains M envelopes. Wait past the
// last expected publish by this slack to catch in-flight packets that
// might arrive after the publish call returns but before vsomeip
// finishes retracting the subscription internally. See header
// comment for why this is heuristic rather than hook-based.
constexpr auto kPostUnsubscribeSlack = std::chrono::milliseconds(300);

// ── Fixture constants ───────────────────────────────────────────
constexpr int kInitialNotifyCount = 3;
constexpr int kPostUnsubNotifyCount = 3;

// Count envelopes whose event name matches the eventgroup the test
// subscribes to. Declared at function scope (above first use) so the
// reader does not have to scroll down to understand the predicate.
auto count_vehicle_speed = [](const std::vector<ReceivedEvent> &v) {
    int n = 0;
    for (const auto &ev : v) {
        if (ev.type == "field.notify.vehicle_speed") {
            ++n;
        }
    }
    return n;
};

// Dispatch one sensor.update trigger to motor's engine via the router.
// Inlining this would duplicate the envelope-construction boilerplate
// six times (three pre-, three post-unsubscribe).
template <typename MotorRouterT> void dispatch_sensor_update(MotorRouterT &motor_router) {
    SCE::Mesh::MeshEnvelope trigger;
    trigger.id = SCE::uuid::v7();
    trigger.source = "test";
    trigger.type = "sensor.update";
    trigger.pattern = SCE::Mesh::PatternKind::FireForget;
    trigger.datacontenttype = SCE::Mesh::PayloadCodec::None;
    (void)motor_router.dispatchToSession(trigger, 0);
}

int run_test() {
    namespace brake_gen = SCE::Generated::brake_someip_unsub;
    namespace motor_gen = SCE::Generated::motor_someip_unsub;
    using Motor = motor_gen::motor_someip_unsub;
    using BrakeRouterT = brake_gen::TransportRouter<TestSenderEngine>;
    using MotorRouterT = motor_gen::TransportRouter<Motor>;

    wipe_stale_vsomeip_sockets();

    // Motor (server + routing manager) must initialize first. Router
    // destructors call shutdown() via RAII (generated code), so we
    // rely on that across every exit path (including exceptions)
    // rather than duplicating explicit `shutdown()` at each return
    // site.
    Motor motor;
    motor.initialize();
    MotorRouterT motor_router({&motor});
    MESH_TEST_REQUIRE(motor_router.init(), "motor router init failed");

    TestSenderEngine brake_engine;
    BrakeRouterT brake_router({&brake_engine});
    MESH_TEST_REQUIRE(brake_router.init(), "brake router init failed");

    // Document intent explicitly: the drain barrier below asserts
    // exactly kInitialNotifyCount, not a lower bound, so a fresh
    // received_ is the contract. Brake is a new TestSenderEngine
    // so this is no-op today, but a future refactor that reuses a
    // stale sender would silently inflate the baseline — this
    // clear is that guard.
    brake_engine.received_.clear();

    // RAII-scoped engine pump drives motor's step() loop. Brake is a
    // TestSenderEngine (no step) so only motor needs pumping. The
    // destructor joins the thread on every exit path, including the
    // exception-propagation path that the enclosing try in main()
    // catches.
    struct EnginePump {
        std::atomic<bool> running{true};
        std::thread t;

        ~EnginePump() {
            running.store(false, std::memory_order_release);
            if (t.joinable()) {
                t.join();
            }
        }
    } pump;

    pump.t = std::thread([&motor, &pump] {
        while (pump.running.load(std::memory_order_acquire)) {
            motor.step();
            std::this_thread::sleep_for(std::chrono::milliseconds(1));
        }
    });

    // Wait for vsomeip service availability.
    {
        std::mutex availability_m;
        std::condition_variable availability_cv;
        bool service_available = false;
        brake_router.motor_app_->register_availability_handler(
            brake_gen::SOMEIP_SERVICE_MOTOR, brake_gen::SOMEIP_INSTANCE_MOTOR,
            [&](vsomeip::service_t, vsomeip::instance_t, bool is_available) {
                if (!is_available) {
                    return;
                }
                std::lock_guard<std::mutex> lock(availability_m);
                service_available = true;
                availability_cv.notify_all();
            });

        std::unique_lock<std::mutex> lock(availability_m);
        MESH_TEST_REQUIRE(availability_cv.wait_for(lock, std::chrono::seconds(10), [&] { return service_available; }),
                          "vsomeip motor service did not become available "
                          "within 10s");
    }

    // Fire EventSubscribe via the generated mesh send callback. Exercises
    // the full generated path (request_event + subscribe inside send_someip).
    MESH_TEST_REQUIRE(brake_engine.mesh_send_cb_ != nullptr, "brake_engine has no mesh send callback — router ctor did "
                                                             "not install the hook");
    MESH_TEST_REQUIRE(brake_engine.mesh_send_cb_("#motor", "event.subscribe.speed", "", "", ""),
                      "send event.subscribe.speed returned false");

    std::this_thread::sleep_for(kSubscribeSettle);

    // Drain barrier: drive motor to publish N notifications and require
    // all N to reach brake. This is the primary correctness guard — if
    // the notify path is not live, the post-unsubscribe absence check
    // passes trivially (false pass). See header comment.
    for (int i = 0; i < kInitialNotifyCount; ++i) {
        dispatch_sensor_update(motor_router);
    }

    MESH_TEST_REQUIRE(
        brake_engine.received_.wait_for([](const auto &v) { return count_vehicle_speed(v) >= kInitialNotifyCount; },
                                        std::chrono::duration_cast<std::chrono::seconds>(kDrainBarrierDeadline)),
        "brake did not receive the initial notifications — notify "
        "path was not live before unsubscribe (drain barrier)");

    int baseline_count = 0;
    {
        std::lock_guard<std::mutex> lock(brake_engine.received_.m);
        baseline_count = count_vehicle_speed(brake_engine.received_.events);
    }

    // Fire EventUnsubscribe via the mesh send callback. The paired
    // unsubscribe event is auto-generated by codegen (auto-symmetry)
    // for every declared EventSubscribe. resolvePattern on brake maps
    // "event.unsubscribe.speed" to EventUnsubscribe, and send_someip
    // invokes app.unsubscribe on the same eventgroup.
    MESH_TEST_REQUIRE(brake_engine.mesh_send_cb_("#motor", "event.unsubscribe.speed", "", "", ""),
                      "send event.unsubscribe.speed returned false");

    std::this_thread::sleep_for(kVsomeipPropagationSlack);

    // Publisher drains M more envelopes AFTER unsubscribe. If unsubscribe
    // is working, NONE of these should reach brake's handler.
    for (int i = 0; i < kPostUnsubNotifyCount; ++i) {
        dispatch_sensor_update(motor_router);
    }

    // Wait past the last expected publish + slack, then assert the count
    // has not grown. Publishing is synchronous from the engine's POV but
    // vsomeip delivery is async; the slack catches any late packets.
    std::this_thread::sleep_for(kPostUnsubscribeSlack);

    int final_count = 0;
    {
        std::lock_guard<std::mutex> lock(brake_engine.received_.m);
        final_count = count_vehicle_speed(brake_engine.received_.events);
    }

    // Formatted message carries baseline/final counts so a failure log
    // points at the exact delta without requiring a re-run with extra
    // logging. MESH_TEST_REQUIRE would fit the boolean but not the
    // format, so a plain if/fprintf here preserves context.
    if (final_count != baseline_count) {
        std::fprintf(stderr,
                     "FAIL: notification count grew after unsubscribe "
                     "(baseline=%d, final=%d) — app.unsubscribe did not "
                     "retract the subscription (gap S6 regression)\n",
                     baseline_count, final_count);
        return 1;
    }

    // Routers + pump tear down via RAII on return.
    std::printf("SCE Mesh SOME/IP EventUnsubscribe E2E (gap S6): PASS\n");
    return 0;
}

}  // namespace

int main() {
    try {
        return run_test();
    } catch (const std::exception &ex) {
        // Router + pump destructors handle their cleanup on unwind;
        // no explicit shutdown needed here.
        std::fprintf(stderr, "FAIL: uncaught exception: %s\n", ex.what());
        return 1;
    }
}
