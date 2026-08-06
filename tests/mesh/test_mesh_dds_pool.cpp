// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE-VERIFIES: mesh-14.4
//
// SCE Mesh §14.4 DDS bounded pool, end to end.
//
// The compile fixture (deploy_pool.yaml) proves all three pool carriers
// render into one header. This one proves the DDS carrier DISPATCHES:
// that the runtime `<param>` value selects which of the declared
// `members:` endpoints a send lands on.
//
// Why the observers are raw `Dds::Server`s rather than generated
// routers: a generated router binds one topic, and the whole claim here
// is about telling two topics apart. Each member's request topic gets
// its own server, so "which endpoint received it" is directly readable
// instead of being inferred.
//
// Three scenarios, one binary:
//
//   §1 A send with `corner=rear` reaches the `rear` member AND NOT the
//      `front_left` one. The negative half is what makes this a
//      selection test — a template that always dispatched through
//      `sensor_pool_[0]` would satisfy the positive half alone.
//
//   §2 The complementary send with `corner=front_left` reaches the other
//      member. Together with §1 this rules out a fixed slot in either
//      direction, which one scenario cannot: a router hardwired to the
//      member named in the last send would pass §1 by itself.
//
//   §3 An undeclared member value is refused, and nothing is written to
//      either topic. This is the fail-closed half of "bounded": the
//      endpoint for an undeclared member was never built, so a router
//      that created one on demand would be writing through a writer
//      that has not finished discovery — a sample a VOLATILE writer
//      drops with no error, which is exactly the silent-loss failure
//      the bounded shape exists to prevent.

#include "pool_sender_dds_transport.h"

#include "MeshTestUtils.h"
#include "mesh/transports/DdsTransport.h"

#include <atomic>
#include <chrono>
#include <cstdio>
#include <string>
#include <thread>

namespace {

using namespace SCE::Test::Mesh;
using namespace std::chrono_literals;

namespace sender_gen = SCE::Generated::pool_sender_dds;

// Must match deploy_dds_pool.yaml: the substituted topics codegen built
// the two endpoints on. Spelled out rather than derived so a change to
// the substitution rule shows up here as a red test rather than as two
// halves that agree with each other and with nothing else.
constexpr const char *kTopicFrontLeft = "SceDdsPool/front_left/Data";
constexpr const char *kTopicRear = "SceDdsPool/rear/Data";
constexpr std::uint32_t kDomainId = 79;
constexpr const char *kDdsConfig =
    "<CycloneDDS><Domain id=\"any\"><Discovery><Tag>sce_mesh_dds_pool</Tag></Discovery></Domain></CycloneDDS>";
constexpr const char *kPartition = "sce_mesh_dds_pool";

constexpr auto kPoll = 10ms;
// DDS discovery is asynchronous; readers and writers match through the
// participant discovery protocol rather than through a connect() the
// caller can wait on. Used to settle matching before a send, and to
// bound a negative assertion.
constexpr auto kDiscovery = 600ms;
constexpr int kWaitIters = 300;  // 3 s

/// One member topic's observer.
struct MemberObserver {
    std::atomic<int> received{0};
    std::string last_type;
};

/// Build the envelope the pool dispatch path reads its member value
/// from. `route_send` cannot see `<param>` entries directly — pool
/// placeholder values travel on `env.data` as JSON (SCE_MESH.md §14.4),
/// which is the same shape the generated invoke path produces.
SCE::Mesh::MeshEnvelope pool_envelope(const char *member_value) {
    SCE::Mesh::MeshEnvelope env;
    env.id = SCE::uuid::v7();
    env.source = "pool_sender_dds";
    env.type = "service.request.ping";
    env.pattern = SCE::Mesh::PatternKind::FireForget;
    env.datacontenttype = SCE::Mesh::PayloadCodec::Json;
    const std::string payload = std::string("{\"corner\":\"") + member_value + "\"}";
    env.data.assign(payload.begin(), payload.end());
    return env;
}

/// Wait until `p` holds or the budget runs out.
template <typename Predicate> bool wait_for(Predicate p) {
    for (int i = 0; i < kWaitIters; ++i) {
        if (p()) {
            return true;
        }
        std::this_thread::sleep_for(kPoll);
    }
    return false;
}

int run_test() {
    // Observers first: a server has to be discoverable before the
    // sender's writers can match it.
    SCE::Mesh::Dds::QosOverlay overlay;
    overlay.partition = kPartition;
    SCE::Mesh::Dds::Participant participant(kDomainId, kDdsConfig, overlay);
    MESH_TEST_REQUIRE(participant.valid(), "observer participant failed to join the domain");

    MemberObserver front_left;
    MemberObserver rear;

    SCE::Mesh::Dds::Server front_left_server(participant, kTopicFrontLeft,
                                             [&front_left](const SCE::Mesh::MeshEnvelope &env) {
                                                 front_left.last_type = env.type;
                                                 front_left.received.fetch_add(1);
                                             });
    MESH_TEST_REQUIRE(front_left_server.valid(), "front_left observer failed to build its endpoints");

    SCE::Mesh::Dds::Server rear_server(participant, kTopicRear, [&rear](const SCE::Mesh::MeshEnvelope &env) {
        rear.last_type = env.type;
        rear.received.fetch_add(1);
    });
    MESH_TEST_REQUIRE(rear_server.valid(), "rear observer failed to build its endpoints");

    TestSenderEngine engine;
    sender_gen::TransportRouter<TestSenderEngine> router({&engine});
    MESH_TEST_REQUIRE(router.init(), "sender router init failed — the pool endpoints were not built");

    std::this_thread::sleep_for(kDiscovery);

    // ── §1 the runtime value selects the `rear` member ────────
    MESH_TEST_REQUIRE(router.route_send("#sensor", pool_envelope("rear")),
                      "the dds pool arm refused a declared member value");
    MESH_TEST_REQUIRE(wait_for([&] { return rear.received.load() > 0; }),
                      "the sample never reached the `rear` member — the pool endpoint for a "
                      "declared member either was not built at init() or was not selected");
    MESH_TEST_REQUIRE(rear.last_type == "service.request.ping", "the `rear` member received an unexpected event type");
    // The negative half. Without it, a router that always dispatched
    // through `sensor_pool_[0]` would still pass the assertion above.
    MESH_TEST_REQUIRE(front_left.received.load() == 0,
                      "a send addressed to `rear` also reached `front_left` — the member value did "
                      "not select the endpoint");

    // ── §2 the complementary member, so neither slot is fixed ──
    const int rear_after_first = rear.received.load();
    MESH_TEST_REQUIRE(router.route_send("#sensor", pool_envelope("front_left")),
                      "the dds pool arm refused the second declared member value");
    MESH_TEST_REQUIRE(wait_for([&] { return front_left.received.load() > 0; }),
                      "the sample never reached the `front_left` member");
    MESH_TEST_REQUIRE(rear.received.load() == rear_after_first,
                      "a send addressed to `front_left` also reached `rear` — dispatch is not "
                      "keyed on the member value");

    // ── §3 an undeclared member is refused, and writes nothing ──
    const int front_left_before = front_left.received.load();
    const int rear_before = rear.received.load();
    MESH_TEST_REQUIRE(!router.route_send("#sensor", pool_envelope("undeclared_corner")),
                      "the dds pool arm accepted a member outside the declared `members:` set — a "
                      "bounded pool that resolves an unknown member has no bound");
    // A missing param is the same fail-closed case: nothing to select on.
    {
        SCE::Mesh::MeshEnvelope no_param = pool_envelope("rear");
        const std::string empty_payload = "{}";
        no_param.data.assign(empty_payload.begin(), empty_payload.end());
        MESH_TEST_REQUIRE(!router.route_send("#sensor", no_param),
                          "the dds pool arm accepted an envelope carrying no member value");
    }
    std::this_thread::sleep_for(kDiscovery);
    MESH_TEST_REQUIRE(front_left.received.load() == front_left_before && rear.received.load() == rear_before,
                      "a refused send still put a sample on a member topic");

    router.shutdown();
    // Idempotent teardown: `~TransportRouter` calls shutdown() again on
    // every properly torn-down router, so a second pass over the member
    // endpoints is the ordinary path rather than an edge case.
    router.shutdown();

    std::printf("[dds-pool] all scenarios passed\n");
    return 0;
}

}  // namespace

int main() {
    return run_test();
}
