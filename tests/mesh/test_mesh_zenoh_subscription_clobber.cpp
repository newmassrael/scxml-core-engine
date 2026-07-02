// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh Gap 2a reproduction: T2 zenoh_subscribers_ unsubscribe clobber.
//
// Pre-Gap-2a codegen keys `zenoh_subscribers_` on target alone while
// `subscription_refcount_` keys on (target, event). Two EventSubscribes
// on the same target (alpha, beta) overwrite the T2 entry on the second
// call, which is benign at steady state because both subscribers would
// carry the shared zenoh keyexpr. The failure surfaces on asymmetric
// unsubscribe: unsubscribing alpha erases the target-keyed T2 entry —
// destroying the shared transport subscriber that beta still needs —
// and subsequent publishes of notification.beta are lost at the wire.
//
// Shape:
//   1. Bring up a raw motor peer on the same listen endpoint.
//   2. Drive the router's mesh-send-callback for subscribe.alpha /
//      subscribe.beta / (optional steady-state publishes) /
//      unsubscribe.alpha.
//   3. Motor publishes notification.beta AFTER the unsubscribe.alpha.
//   4. Expect: sender observes notification.beta. Under the pre-fix
//      codegen this times out because no zenoh subscriber is active.
//
// The SCXML-step path is intentionally bypassed: TestSenderEngine is a
// stub, so we invoke `mesh_send_cb_` directly. That drives the refcount
// block inside `setMeshSendCallback` and reaches `send_zenoh` with the
// same envelope shape an author-driven <send> would produce.

#include "brake_zenoh_subscription_clobber_transport.h"

#include "ZenohTestUtils.h"
#include "mesh/MeshEnvelopeCodec.h"

#include <chrono>
#include <cstdio>
#include <string>
#include <thread>

namespace {

using namespace SCE::Test::Mesh;

constexpr const char *kListen = SCE::Generated::brake_zenoh_subscription_clobber::ZENOH_CONNECT_ENDPOINTS[0];
constexpr const char *kMotorKey = SCE::Generated::brake_zenoh_subscription_clobber::ZENOH_KEY_MOTOR_CLOBBER;

void publish_notify(zenoh::Session &session, const std::string &event_name) {
    auto env = make_envelope(event_name, SCE::Mesh::PatternKind::FireForget);
    auto bytes = SCE::Mesh::encodeEnvelope(env);
    session.put(zenoh::KeyExpr(kMotorKey), zenoh::Bytes(std::move(bytes)));
}

int run_test() {
    namespace brake_gen = SCE::Generated::brake_zenoh_subscription_clobber;
    using RouterT = brake_gen::TransportRouter<TestSenderEngine>;

    TestSenderEngine sender;
    RouterT router({&sender});

    auto motor_session = open_peer(/*connect=*/"", /*listen=*/kListen);

    MESH_TEST_REQUIRE(router.init(), "router.init() failed");

    auto handshake_session = open_peer(/*connect=*/kListen, /*listen=*/"");
    wait_for_peer_ready(motor_session, handshake_session);

    MESH_TEST_REQUIRE(static_cast<bool>(sender.mesh_send_cb_), "setMeshSendCallback was not installed by router ctor");

    // Subscribe alpha → first live (target, event) for #motor_clobber.
    // Under Option B this should take target_sub_live_count_ 0→1 and
    // declare the transport subscriber. Under the pre-fix code this
    // takes subscription_refcount_[target|alpha] 0→1 and insert_or_assign
    // on zenoh_subscribers_[target].
    MESH_TEST_REQUIRE(sender.mesh_send_cb_("#motor_clobber", "event.subscribe.alpha", "", "", ""),
                      "subscribe.alpha callback returned false");

    // Subscribe beta → second distinct (target, event) on the same
    // target. Under the pre-fix code this insert_or_assign clobbers the
    // alpha subscriber handle (which is benign because both target the
    // same keyexpr). Under Option B this increments
    // target_sub_live_count_[#motor_clobber] to 2 and short-circuits
    // before send_zenoh, leaving the existing subscriber intact.
    MESH_TEST_REQUIRE(sender.mesh_send_cb_("#motor_clobber", "event.subscribe.beta", "", "", ""),
                      "subscribe.beta callback returned false");

    // Steady-state proof: both notifications reach the sender engine
    // while both subscriptions are live. This step passes on both pre-
    // fix and post-fix code paths — the bug is lifecycle-shaped, not
    // steady-state-shaped.
    std::this_thread::sleep_for(std::chrono::milliseconds(200));
    publish_notify(motor_session, "event.notification.alpha");
    MESH_TEST_REQUIRE(sender.received_.wait_for(
                          [](const auto &v) { return !v.empty() && v.back().type == "event.notification.alpha"; }),
                      "sender did not observe notification.alpha during steady state");
    publish_notify(motor_session, "event.notification.beta");
    MESH_TEST_REQUIRE(sender.received_.wait_for([](const auto &v) {
        for (const auto &e : v) {
            if (e.type == "event.notification.beta") {
                return true;
            }
        }
        return false;
    }),
                      "sender did not observe notification.beta during steady state");
    sender.received_.clear();

    // Asymmetric unsubscribe: tear down alpha while beta is still live.
    // Under Option B this decrements target_sub_live_count_ to 1 and
    // leaves the transport subscriber alive. Under the pre-fix code
    // this reaches `zenoh_subscribers_.erase(target)` because
    // subscription_refcount_[target|alpha] hit zero — destroying the
    // shared zenoh subscriber beta still depends on.
    MESH_TEST_REQUIRE(sender.mesh_send_cb_("#motor_clobber", "event.unsubscribe.alpha", "", "", ""),
                      "unsubscribe.alpha callback returned false");

    // Give zenoh a moment to undeclare if the pre-fix erase went through
    // (the pre-fix failure mode is observable immediately; the post-fix
    // path has no transport-level change here, so the sleep is a no-op
    // on green runs).
    std::this_thread::sleep_for(std::chrono::milliseconds(200));

    // Post-unsubscribe publish of notification.beta. The RED assertion:
    // beta is still logically subscribed (subscription_refcount_
    // [target|beta] == 1), so the transport subscriber must still
    // deliver. Under the pre-fix code no subscriber is declared on the
    // keyexpr and this wait times out.
    publish_notify(motor_session, "event.notification.beta");
    MESH_TEST_REQUIRE(sender.received_.wait_for([](const auto &v) {
        for (const auto &e : v) {
            if (e.type == "event.notification.beta") {
                return true;
            }
        }
        return false;
    }),
                      "sender did not observe notification.beta AFTER unsubscribe.alpha — "
                      "zenoh_subscribers_ was erased by the alpha unsubscribe while the "
                      "beta subscription was still live (Gap 2a clobber)");

    router.shutdown();
    std::printf("SCE Mesh Gap 2a zenoh subscription clobber: PASS\n");
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
