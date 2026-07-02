// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §10.10 OutboundBuffer runtime E2E — Zenoh publisher-first.
//
// Gap 5 Acceptance Fixture (iii): closes the "FireForget / FieldWrite
// published before any subscriber declares" silent-drop on Zenoh.
//
// Shape:
//   - Raw motor peer boots first (listener side of the shared 17454
//     port). No subscriber declared yet on the brake-side keyexpr, so
//     brake's generated router will observe MatchingStatus::matching
//     == false after it opens its own session.
//   - Brake's generated router opens its Zenoh session, declares a
//     Publisher on the bound keyexpr, installs a matching_listener on
//     that publisher, and seeds readiness from
//     get_matching_status() — which returns false because no
//     subscriber matches yet.
//   - Brake calls route_send with a FireForget envelope. Pre-§10.10
//     this would emit `session.put` immediately and Zenoh (no
//     retention) would drop the sample. Post-§10.10 the route_send's
//     zenoh arm checks env.pattern ∈ {FireForget, FieldWrite} and
//     routes through OutboundBuffer::admit — which enqueues because
//     ready_ == false.
//   - Raw motor peer declares a subscriber on brake's keyexpr.
//     Brake's matching_listener fires on the PUBLISHER side as the
//     subscriber appears on the wire; markReady drains the buffer
//     through the dispatch closure (publisher.put), and the raw
//     subscriber callback observes the previously-buffered envelope.
//
// Mutation pin: removing `declare_matching_listener` from the
// generated `init()` (or the `{% if machine_outbound_buffer %}` gate
// in the template) would leave the buffer stuck at ready_ = false;
// the subscriber would never observe the envelope and the 5-second
// wait would fail with an explicit diagnostic.
//
// Sibling references:
//   - test_mesh_someip_late_boot.cpp is the SOME/IP counterpart.
//   - test_mesh_zenoh_base_subscribe.cpp covers subscriber-first
//     ordering (no buffering needed — Zenoh publish reaches a
//     subscriber that declared first by construction).
//   - ZenohTestUtils.h::wait_for_queryable uses the same
//     declare_matching_listener primitive (on Querier rather than
//     Publisher) for test synchronization — orthogonal consumer of
//     the same Zenoh API.

#include "brake_zenoh_publisher_first_transport.h"

#include "ZenohTestUtils.h"
#include "mesh/MeshEnvelopeCodec.h"

#include <chrono>
#include <cstdio>
#include <string>
#include <thread>

namespace {

using namespace SCE::Test::Mesh;

// Mirrors brake's connect endpoint, pinned in deploy_zenoh_publisher_first.yaml
// to motor's ecu_motor listen. The raw motor peer co-locates on the same
// address so the Zenoh peer-mesh handshake converges deterministically.
constexpr const char *kListen = SCE::Generated::brake_zenoh_publisher_first::ZENOH_CONNECT_ENDPOINTS[0];

// Must match deploy_zenoh_publisher_first.yaml binding key for #motor.
constexpr const char *kMotorKey = "sce/brake_pub_first/motor/cmd";

// Upper bound on the buffered envelope's arrival after the subscriber
// declares. Zenoh's matching_listener typically fires within tens of
// ms on loopback peer-mesh; 5 s is the same slack as the SOME/IP
// late-boot fixture and other zenoh runtime tests.
constexpr auto kReceiveTimeout = std::chrono::seconds(5);

int run_test() {
    namespace brake_gen = SCE::Generated::brake_zenoh_publisher_first;
    using RouterT = brake_gen::TransportRouter<TestSenderEngine>;

    // ── Motor listener side boots first. ─────────────────────────────
    // The raw motor peer listens so brake's connect succeeds. Without
    // a subscriber on kMotorKey yet, brake's publisher will observe
    // MatchingStatus::matching == false and the buffer stays gated.
    auto motor_session = open_peer(/*connect=*/"", /*listen=*/kListen);

    // ── Brake (publisher) boots. ─────────────────────────────────────
    // TestSenderEngine stands in for the SCXML engine — the fixture's
    // assertion is on the MOTOR-side subscriber observing the drained
    // envelope, so the brake-side sender is only needed to satisfy the
    // router's template signature.
    TestSenderEngine brake_engine;
    RouterT brake_router({&brake_engine});
    MESH_TEST_REQUIRE(brake_router.init(), "brake router init failed");

    // Readiness must start false: no subscriber has declared on
    // kMotorKey, so the matching listener reports false, get_matching_status
    // confirms, and the OutboundBuffer stays in the unready state.
    MESH_TEST_REQUIRE(!brake_router.motor_outbound_.ready(), "outbound buffer should start NOT ready "
                                                             "(no subscriber on kMotorKey yet)");
    MESH_TEST_REQUIRE(brake_router.motor_outbound_.queue_depth() == 0, "outbound buffer should start empty");

    // ── Pre-subscriber send: buffered, not published. ────────────────
    {
        auto env = make_envelope("warmup.tick", SCE::Mesh::PatternKind::FireForget);
        std::string payload = R"({"phase":"pre-subscriber"})";
        env.data.assign(payload.begin(), payload.end());
        env.datacontenttype = SCE::Mesh::PayloadCodec::Json;

        const bool accepted = brake_router.route_send("#motor", env);
        MESH_TEST_REQUIRE(accepted, "pre-subscriber route_send must return true — "
                                    "admit enqueues when buffer capacity remains");
    }

    MESH_TEST_REQUIRE(brake_router.motor_outbound_.queue_depth() == 1,
                      "pre-subscriber envelope must be buffered, not published");

    // ── Motor declares the subscriber. ───────────────────────────────
    // CapturedEvents holds the inbound envelopes as they arrive. The
    // subscriber decodes each payload (brake's publisher.put carries
    // the CBOR envelope bytes produced by SCE::Mesh::encodeEnvelope)
    // and pushes the decoded MeshEnvelope so the wait_for predicate
    // can assert on env.type / env.data symmetrically with the
    // outbound side.
    CapturedEvents received;
    auto subscriber = motor_session.declare_subscriber(
        zenoh::KeyExpr(kMotorKey),
        [&received](const zenoh::Sample &sample) {
            auto bytes = sample.get_payload().as_vector();
            SCE::Mesh::MeshEnvelope env;
            if (SCE::Mesh::decodeEnvelope(bytes.data(), bytes.size(), env)) {
                received.push(env);
            }
        },
        // on_drop: subscriber handle teardown (RAII). No per-sample
        // state to unwind; the CapturedEvents ownership outlives the
        // subscriber handle.
        []() noexcept {});

    // ── Drain verification: the subscriber receives the buffered env. ─
    // Brake's matching_listener fires asynchronously from a zenoh
    // runtime thread once motor's subscribe propagates through the
    // peer-mesh. That calls markReady on the buffer, which drains the
    // enqueued envelope through the dispatch closure —
    // publisher.put(encoded), routed to the subscriber callback above.
    MESH_TEST_REQUIRE(
        received.wait_for([](const auto &v) { return !v.empty() && v.back().type == "warmup.tick"; }, kReceiveTimeout),
        "motor subscriber did not receive the buffered FireForget "
        "envelope after declaring — §10.10 OutboundBuffer drain regression");

    // Post-drain invariants.
    MESH_TEST_REQUIRE(brake_router.motor_outbound_.queue_depth() == 0, "buffer should be empty after drain");
    MESH_TEST_REQUIRE(brake_router.motor_outbound_.ready(), "buffer should report ready after matching_listener fired");

    // ── Post-subscriber send: fast path, no buffering. ───────────────
    // Proves markReady flipped ready_ to true, not just drained once.
    {
        auto env = make_envelope("warmup.tick", SCE::Mesh::PatternKind::FireForget);
        std::string payload = R"({"phase":"post-subscriber"})";
        env.data.assign(payload.begin(), payload.end());
        env.datacontenttype = SCE::Mesh::PayloadCodec::Json;

        const bool accepted = brake_router.route_send("#motor", env);
        MESH_TEST_REQUIRE(accepted, "post-subscriber route_send returned false");
        MESH_TEST_REQUIRE(brake_router.motor_outbound_.queue_depth() == 0,
                          "post-subscriber send must take the fast path, "
                          "not re-enqueue (ready flag regressed?)");
    }

    MESH_TEST_REQUIRE(received.wait_for(
                          [](const auto &v) {
                              std::size_t count = 0;
                              for (const auto &ev : v) {
                                  if (ev.type == "warmup.tick") {
                                      ++count;
                                  }
                              }
                              return count >= 2;
                          },
                          kReceiveTimeout),
                      "motor subscriber did not receive the post-subscriber envelope — "
                      "matching-listener-driven fast path regression");

    brake_router.shutdown();
    std::printf("SCE Mesh §10.10 Zenoh publisher-first outbound buffer: PASS\n");
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
