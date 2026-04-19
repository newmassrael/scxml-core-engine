// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §10.10 OutboundBuffer runtime E2E — SOME/IP late-boot.
//
// Gap 5 Acceptance Fixture (i): closes the "first send lost before
// offer_service" silent-drop on SOME/IP.
//
// Shape:
//   - Brake is the vsomeip routing manager (VSOMEIP_CONFIGURATION flips
//     the `routing:` field to `brake_someip_multi_app`). Brake boots
//     first, creating the routing socket; its `request_service` sits at
//     NOT_AVAILABLE because motor has not yet offered.
//   - `deploy_someip_late_boot.yaml` adds `outbound_buffer:
//     max_pending_per_target: 8` to brake. The generated router
//     therefore constructs a per-target OutboundBuffer, installs
//     vsomeip's `register_availability_handler` at init() (before
//     `start()`), and routes route_send through `admit`.
//   - While motor is still down, brake calls route_send with a
//     FireForget envelope. Pre-§10.10 this envelope would be silently
//     dropped by vsomeip's `send()` on a NOT_AVAILABLE service. Post-
//     §10.10 the buffer captures it (ready_ = false, admit enqueues).
//   - Motor boots second, calls `offer_service`. Brake's availability
//     handler fires asynchronously → markReady → FIFO drain → motor's
//     server-side message handler decodes the envelope → motor_engine
//     observes `service.fire_forget.activate`.
//
// Mutation pin: removing the `register_availability_handler` call from
// the generated `init()` (or the `{% if machine_outbound_buffer %}`
// gate in the template) would leave the buffer forever in
// `ready_ = false` state; motor would never receive the envelope and
// the test's 5-second wait would fail with an explicit diagnostic.
// Verified by commenting out the handler registration locally and
// confirming this test transitions RED.
//
// Sibling references:
//   - test_mesh_someip_runtime.cpp covers the happy-path "motor first,
//     brake second" ordering (no buffering exercised).
//   - test_mesh_zenoh_publisher_first.cpp is the Zenoh counterpart for
//     Gap 5 Fixture (iii) — publisher-first-before-subscriber.

#include "brake_someip_multi_sm.h"
#include "brake_someip_multi_transport.h"
#include "motor_someip_multi_sm.h"
#include "motor_someip_multi_transport.h"

#include "MeshTestUtils.h"
#include "SomeipTestUtils.h"
#include "common/Uuid.h"
#include "mesh/MeshDispatch.h"

#include <chrono>
#include <cstdio>
#include <string>
#include <thread>

namespace {

using namespace SCE::Test::Mesh;

// Time motor intentionally stays down before booting, proving the
// envelope is in the buffer and not yet dispatched. 250 ms is well
// above the 10 ms registry floor for other deploy.yaml knobs and
// leaves slack for CI jitter.
constexpr auto kMotorLateDelay = std::chrono::milliseconds(250);

// Upper bound on motor's receipt after it offers the service. vsomeip's
// availability callback fires within a few ms of `offer_service` in
// internal-routing mode; 5 s is 25× safety margin and matches the
// established pattern in test_mesh_someip_runtime.cpp.
constexpr auto kReceiveTimeout = std::chrono::seconds(5);

int run_test() {
    namespace brake_gen = SCE::Generated::brake_someip_multi;
    namespace motor_gen = SCE::Generated::motor_someip_multi;
    using PK = SCE::Mesh::PatternKind;
    using BrakeRouterT = brake_gen::TransportRouter<TestSenderEngine>;
    using MotorRouterT = motor_gen::TransportRouter<TestSenderEngine>;

    wipe_stale_vsomeip_sockets();

    // ── Brake (client + routing manager) boots FIRST. ─────────────────
    // This is the critical inversion vs test_mesh_someip_runtime.cpp.
    // The runtime config `vsomeip_e2e_someip_late_boot.json` names
    // brake_someip_multi_app as the routing manager, so brake creates
    // the routing socket and its request_service lands on the local
    // routing table before motor has offered anything — exactly the
    // NOT_AVAILABLE state §10.10 must survive.
    TestSenderEngine brake_engine;
    BrakeRouterT brake_router({&brake_engine});
    MESH_TEST_REQUIRE(brake_router.init(), "brake router init failed");

    // Assert the buffer is present and empty before the first send so
    // any future regression that elides the codegen member surfaces at
    // this line rather than hiding in a passing-by-accident happy path.
    MESH_TEST_REQUIRE(brake_router.motor_outbound_.queue_depth() == 0,
                      "outbound buffer should start empty");
    MESH_TEST_REQUIRE(!brake_router.motor_outbound_.ready(),
                      "outbound buffer should start NOT ready (motor not booted)");

    // ── Pre-boot send: queued into the buffer, NOT dispatched. ───────
    {
        SCE::Mesh::MeshEnvelope env;
        env.id = SCE::uuid::v7();
        env.source = "brake_someip_multi";
        env.type = "service.fire_forget.activate";
        env.pattern = PK::FireForget;
        env.datacontenttype = SCE::Mesh::PayloadCodec::Json;
        std::string payload = R"({"phase":"pre-boot"})";
        env.data.assign(payload.begin(), payload.end());

        const bool accepted = brake_router.route_send("#motor", env);
        MESH_TEST_REQUIRE(accepted,
                          "pre-boot route_send must return true — admit "
                          "enqueues when buffer capacity remains");
    }

    // Buffer must now hold the pre-boot envelope — direct verification
    // that the fast path (ready && empty) did NOT fire.
    MESH_TEST_REQUIRE(brake_router.motor_outbound_.queue_depth() == 1,
                      "pre-boot envelope must be buffered, not dispatched");

    // ── Motor (server) boots AFTER the pre-boot send is buffered. ────
    // kMotorLateDelay makes the timing explicit for log reading — the
    // test would also pass without the sleep because the buffer
    // persists across the init() call, but the delay documents the
    // intent and gives CI an unambiguous "motor was genuinely late"
    // signature if this fixture ever needs to be debugged.
    std::this_thread::sleep_for(kMotorLateDelay);
    TestSenderEngine motor_engine;
    MotorRouterT motor_router({&motor_engine});
    MESH_TEST_REQUIRE(motor_router.init(), "motor router init failed");

    // ── Drain verification: motor_engine observes the pre-boot send. ─
    // Brake's availability handler fires asynchronously from a vsomeip
    // worker thread once motor's offer_service lands on the routing
    // manager's offer table. That calls markReady on the buffer, which
    // drains the enqueued envelope through the dispatch closure —
    // vsomeip `app.send` on the outbound path, then vsomeip worker
    // delivers to motor's FireForget handler.
    MESH_TEST_REQUIRE(
        motor_engine.received_.wait_for(
            [](const auto& v) {
                return !v.empty() &&
                       v.back().type == "service.fire_forget.activate";
            },
            kReceiveTimeout),
        "motor engine did not receive the buffered FireForget envelope "
        "after late boot — §10.10 OutboundBuffer drain regression");

    // Post-drain invariant: the buffer must be empty (drain consumed
    // the pre-boot envelope) and ready_ = true (availability fired).
    // A racing admit between drain and this check would be a test bug,
    // not a runtime bug — no concurrent sends happen between the wait
    // above returning and this line.
    MESH_TEST_REQUIRE(brake_router.motor_outbound_.queue_depth() == 0,
                      "buffer should be empty after drain");
    MESH_TEST_REQUIRE(brake_router.motor_outbound_.ready(),
                      "buffer should report ready after availability fired");

    // ── Post-boot send: fast path, no buffering. ─────────────────────
    // Proves markReady did flip the ready_ flag, not just drain once.
    // A regression that accidentally reset ready_ to false after drain
    // would surface here (next admit re-enqueues instead of
    // dispatching, the test would still pass by accident because
    // the buffer is still draining — so assert queue_depth stays at 0).
    {
        SCE::Mesh::MeshEnvelope env;
        env.id = SCE::uuid::v7();
        env.source = "brake_someip_multi";
        env.type = "service.fire_forget.activate";
        env.pattern = PK::FireForget;
        env.datacontenttype = SCE::Mesh::PayloadCodec::Json;
        std::string payload = R"({"phase":"post-boot"})";
        env.data.assign(payload.begin(), payload.end());

        const bool accepted = brake_router.route_send("#motor", env);
        MESH_TEST_REQUIRE(accepted, "post-boot route_send returned false");
        // queue_depth may momentarily be non-zero if admit held the
        // mutex past dispatch completion, but the fast path runs
        // dispatch under the mutex before returning, so by the time
        // admit returns true the envelope has either been dispatched
        // (queue empty) or enqueued (ready was false — regression).
        MESH_TEST_REQUIRE(brake_router.motor_outbound_.queue_depth() == 0,
                          "post-boot send must take the fast path, "
                          "not re-enqueue (ready flag regressed?)");
    }

    // Confirm motor observed at least two FireForget envelopes — the
    // pre-boot buffered one plus the post-boot fast-path one.
    MESH_TEST_REQUIRE(
        motor_engine.received_.wait_for(
            [](const auto& v) {
                std::size_t count = 0;
                for (const auto& ev : v) {
                    if (ev.type == "service.fire_forget.activate") ++count;
                }
                return count >= 2;
            },
            kReceiveTimeout),
        "motor engine did not receive the post-boot FireForget envelope — "
        "availability-driven fast path regression");

    brake_router.shutdown();
    motor_router.shutdown();
    std::printf("SCE Mesh §10.10 SOME/IP late-boot outbound buffer: PASS\n");
    return 0;
}

}  // namespace

int main() {
    try {
        return run_test();
    } catch (const std::exception& ex) {
        std::fprintf(stderr, "FAIL: uncaught exception: %s\n", ex.what());
        return 1;
    }
}
