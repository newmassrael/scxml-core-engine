// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE-VERIFIES: mesh-10.12
//
// The causal route through the generated C++ router, once per transport
// (docs/adr/0085, sections 3.2 and 9.2). One source, compiled against each
// `deploy_causal_<transport>.yaml` fixture: the brake's `#motor` binding is
// the route of a causal group of the two machines. The motor's side is a
// `SCE::Mesh::CausalRoute` the test holds, whose envelopes reach the brake's
// router through the entry point that transport delivers to:
//
//   * custom_tcp, zenoh_keyexpr, dds, someip: `dispatchToSession`, where each
//     of their inbound handlers ends (no socket, session, domain or vsomeip
//     application is opened);
//   * shm (SCE_CAUSAL_SHM): the channel the motor's router creates for its
//     binding back, which the brake's router drains through `pumpCausalShm`;
//     what the brake releases is read off the brake's own channel;
//   * local (SCE_CAUSAL_LOCAL): the brake writes only through the motor's
//     router, which `linkTo` stands in for and records.
//
// What it holds the router to, and the change that turns each red:
//
//   1. a send is offered, its verdict queued for the host, and nothing
//      leaves before `causalCommitted` (red if the send callback bypasses the
//      route, as a plain route_send would);
//   2. the motor's changes are delivered once and in causal order, a change
//      past a gap held until the gap fills (red if `dispatchToSession` does
//      not give the peer's envelope to `takeCausal` first);
//   3. a vector reaches the route and never the engine (red if pattern 23 is
//      dispatched to the engine);
//   4. an envelope of another epoch raises CAUSAL_VIEW_MISMATCH and is not
//      delivered (red if `settleCausal` drops the raise);
//   5. shm and local: the release and the period's vector leave on the
//      transport (red if `pumpCausalShm` drains into the engine directly, or
//      an unlinked causal local target is written into its engine).

// The brake's router. On a SOME/IP pool (SCE_CAUSAL_SOMEIP_POOL) the sender
// is `brake_causal_pool`, whose send carries the `<param>` the pool reads.
#if defined(SCE_CAUSAL_SOMEIP_POOL)
#include "brake_causal_pool_transport.h"
#define SCE_CAUSAL_NAMESPACE brake_causal_pool
#define SCE_CAUSAL_SEND_DATA R"({"unit":2})"
#else
#include "brake_transport.h"
#define SCE_CAUSAL_NAMESPACE brake
#define SCE_CAUSAL_SEND_DATA ""
#endif

#include "MeshTestUtils.h"
#include "mesh/CausalRoute.h"
#include "mesh/MeshEnvelope.h"

#include <cstdint>
#include <cstdio>
#include <exception>
#include <optional>
#include <string>
#include <variant>
#include <vector>

#ifndef SCE_CAUSAL_TRANSPORT_KIND
#error "SCE_CAUSAL_TRANSPORT_KIND names the fixture's transport (\"custom_tcp\", \"shm\", ...)"
#endif

namespace {

using namespace SCE::Generated::SCE_CAUSAL_NAMESPACE;
using namespace SCE::Test::Mesh;

SCE::Mesh::CausalGroup pair() {
    return SCE::Mesh::CausalGroup{"pair", 0, {"ecu1/brake", "ecu1/motor"}};
}

SCE::Mesh::CausalReplica replica(std::uint8_t n) {
    SCE::Mesh::CausalReplica out;
    out.id.fill(n);
    return out;
}

/// The motor's change named `type`, as its route releases it once its host
/// commits the journal record: the stamped envelope, or none.
std::optional<SCE::Mesh::MeshEnvelope> motorChange(SCE::Mesh::CausalRoute &motor, const std::string &type) {
    SCE::Mesh::MeshEnvelope env;
    env.id = SCE::uuid::v7();
    env.source = "motor";
    env.type = type;
    env.pattern = SCE::Mesh::PatternKind::FireForget;
    env.datacontenttype = SCE::Mesh::PayloadCodec::None;
    const auto offered = motor.offer(env);
    if (!offered) {
        return std::nullopt;
    }
    const auto *verdict = std::get_if<SCE::Mesh::CausalVerdict>(&*offered);
    if (verdict == nullptr || verdict->kind != SCE::Mesh::CausalVerdict::Kind::Accepted) {
        return std::nullopt;
    }
    auto released = motor.committed(verdict->dot);
    if (released.size() != 1) {
        return std::nullopt;
    }
    return released.front();
}

std::vector<std::string> received(TestSenderEngine &sender) {
    std::lock_guard<std::mutex> lk(sender.received_.m);
    std::vector<std::string> types;
    for (const auto &event : sender.received_.events) {
        types.push_back(event.type);
    }
    return types;
}

bool raised(TestSenderEngine &sender, const std::string &reason) {
    std::lock_guard<std::mutex> lk(sender.received_.m);
    for (const auto &event : sender.received_.events) {
        if (event.type == "error.communication" && event.data.find(reason) != std::string::npos) {
            return true;
        }
    }
    return false;
}

int run_test() {
#if defined(SCE_CAUSAL_LOCAL)
    using RouterT = TransportRouter<TestSenderEngine, TestSenderEngine>;
    TestSenderEngine sender;
    TestSenderEngine motor_engine;
    RouterT router({&sender}, motor_engine);
    std::vector<SCE::Mesh::MeshEnvelope> linked;
    router.linkTo("#motor", [&linked](const SCE::Mesh::MeshEnvelope &env) {
        linked.push_back(env);
        return true;
    });
#else
    using RouterT = TransportRouter<TestSenderEngine>;
#if defined(SCE_CAUSAL_SHM)
    // The motor's router would create this channel for its binding back; the
    // test creates it first, so the brake's router opens it at once.
    MotorShmChannel motor_out(CAUSAL_SHM_FROM_MOTOR, MotorShmChannel::Mode::Create);
    MESH_TEST_REQUIRE(motor_out.valid(), "the motor's channel to the brake is created");
#endif
    TestSenderEngine sender;
    RouterT router({&sender});
#endif

    // The motor's envelopes reach the brake's router as its transport
    // delivers them.
    const auto deliver = [&](const SCE::Mesh::MeshEnvelope &env) {
#if defined(SCE_CAUSAL_SHM)
        if (!motor_out.send(env)) {
            return false;
        }
        return router.pumpCausalShm() == 1;
#else
        (void)router.dispatchToSession(env, 0);
        return true;
#endif
    };

    MESH_TEST_REQUIRE(router.startCausal("#motor", SCE::Mesh::CausalStart::newIncarnation(replica(1)), 64, 100),
                      "the causal binding starts");
    MESH_TEST_REQUIRE(!router.startCausal("#nobody", SCE::Mesh::CausalStart::newIncarnation(replica(1)), 64, 100),
                      "a target that is not a causal binding does not start");

#if defined(SCE_CAUSAL_SOMEIP_POOL)
    // ── 0. A pool's vector goes to each member's instance ───────────
    // docs/adr/0085, sections 3.2 and 3.3: the view's motor serves pool
    // instance 2, so the route's vector goes there, while a change goes to
    // the instance its `<param>` names. A vector carries no such `<param>`:
    // sent the binding's one way, it would reach no instance at all. Red if
    // `sendCausalToPeers` sends it the binding's one way.
    MESH_TEST_REQUIRE(CAUSAL_POOL_INSTANCES_MOTOR.size() == 1 && CAUSAL_POOL_INSTANCES_MOTOR[0] == 2,
                      "the deploy places the view's motor on pool instance 2");
    {
        SCE::Mesh::CausalRoute probe("brake_causal_pool", "someip", pair());
        probe.start(SCE::Mesh::CausalStart::newIncarnation(replica(3)), 64, 100);
        const auto vector = probe.announce(SCE::uuid::v7(), 0);
        MESH_TEST_REQUIRE(vector.has_value(), "a route writes its vector");
        MESH_TEST_REQUIRE(router.sendCausalToPeers("#motor", *vector) == 1,
                          "the vector is sent to the one member's instance");
    }
#endif

    // ── 1. A send is offered, and leaves only once committed ─────────
    MESH_TEST_REQUIRE(static_cast<bool>(sender.mesh_send_cb_), "the router installs its send callback");
    MESH_TEST_REQUIRE(sender.mesh_send_cb_("#motor", "brake.activate", SCE_CAUSAL_SEND_DATA, "", ""),
                      "the send is answered");
    auto verdicts = router.takeCausalVerdicts();
    MESH_TEST_REQUIRE(verdicts.size() == 1, "one verdict per send");
    MESH_TEST_REQUIRE(verdicts[0].first == "#motor", "the verdict names its binding");
    MESH_TEST_REQUIRE(verdicts[0].second.kind == SCE::Mesh::CausalVerdict::Kind::Accepted, "the change is accepted");
    MESH_TEST_REQUIRE(verdicts[0].second.dot.seq == 1, "the first change takes number 1");
    MESH_TEST_REQUIRE(router.takeCausalVerdicts().empty(), "a verdict is taken once");
#if defined(SCE_CAUSAL_LOCAL)
    MESH_TEST_REQUIRE(linked.empty(), "nothing leaves before the journal commit");
#elif defined(SCE_CAUSAL_SHM)
    MotorShmChannel brake_out(SHM_CHANNEL_MOTOR, MotorShmChannel::Mode::Open);
    MESH_TEST_REQUIRE(brake_out.valid(), "the brake's channel to the motor exists");
    std::vector<SCE::Mesh::MeshEnvelope> written;
    const auto drainBrake = [&]() {
        (void)brake_out.drainWith([&written](const SCE::Mesh::MeshEnvelope &env) { written.push_back(env); }, []() {});
    };
    drainBrake();
    MESH_TEST_REQUIRE(written.empty(), "nothing leaves before the journal commit");
#endif
    MESH_TEST_REQUIRE(router.causalCommitted("#motor", verdicts[0].second.dot), "the commit names a causal binding");
#if defined(SCE_CAUSAL_LOCAL)
    MESH_TEST_REQUIRE(linked.size() == 1, "the committed change leaves through the motor's router");
    MESH_TEST_REQUIRE(linked[0].causal_epoch.has_value() && linked[0].causal_vector.has_value(),
                      "it leaves with its tag");
#elif defined(SCE_CAUSAL_SHM)
    drainBrake();
    MESH_TEST_REQUIRE(written.size() == 1, "the committed change leaves on the brake's channel");
    MESH_TEST_REQUIRE(written[0].causal_vector.has_value(), "it leaves with its tag");
#endif

    // ── 2. The motor's changes: once, in causal order ────────────────
    SCE::Mesh::CausalRoute motor("motor", SCE_CAUSAL_TRANSPORT_KIND, pair());
    motor.start(SCE::Mesh::CausalStart::newIncarnation(replica(2)), 64, 100);
    const auto first = motorChange(motor, "motor.first");
    const auto second = motorChange(motor, "motor.second");
    MESH_TEST_REQUIRE(first && second, "the motor's route releases its changes");
    MESH_TEST_REQUIRE(deliver(*second), "the second change reaches the router");
    MESH_TEST_REQUIRE(received(sender).empty(), "a change past a gap is held");
    MESH_TEST_REQUIRE(deliver(*first), "the first change reaches the router");
    MESH_TEST_REQUIRE((received(sender) == std::vector<std::string>{"motor.first", "motor.second"}),
                      "the gap filled releases both, in order");
    MESH_TEST_REQUIRE(deliver(*first), "the first change reaches the router again");
    MESH_TEST_REQUIRE(received(sender).size() == 2, "a change delivered already is dropped");

    // ── 3. A vector reaches the route, never the engine ─────────────
    const auto vector = motor.announce(SCE::uuid::v7(), 0);
    MESH_TEST_REQUIRE(vector.has_value(), "the motor's route writes its vector");
    MESH_TEST_REQUIRE(deliver(*vector), "the vector reaches the router");
    MESH_TEST_REQUIRE(received(sender).size() == 2, "a vector never reaches the engine");

    // ── 4. Another epoch is raised, not delivered ───────────────────
    SCE::Mesh::MeshEnvelope foreign = *first;
    foreign.id = SCE::uuid::v7();
    foreign.type = "motor.foreign";
    foreign.causal_epoch = 1;
    MESH_TEST_REQUIRE(deliver(foreign), "the envelope of another epoch reaches the router");
    MESH_TEST_REQUIRE(raised(sender, "CAUSAL_VIEW_MISMATCH"), "another epoch raises row 18");
    for (const auto &type : received(sender)) {
        MESH_TEST_REQUIRE(type != "motor.foreign", "an envelope of another epoch is not delivered");
    }

    // ── 5. The period's vector leaves on the transport ──────────────
    router.causalTick(1000);
#if defined(SCE_CAUSAL_LOCAL)
    // Before it: the release of step 1, and the answer to the motor's vector
    // of step 3, which lacked that change.
    MESH_TEST_REQUIRE(linked.size() == 3, "the answer to a vector and the period's vector both leave");
    MESH_TEST_REQUIRE(linked[1].type == "brake.activate", "a vector is answered with the change it lacks");
    MESH_TEST_REQUIRE(linked[2].pattern == SCE::Mesh::PatternKind::CausalVector,
                      "the period's vector leaves through the motor's router");
#elif defined(SCE_CAUSAL_SHM)
    drainBrake();
    MESH_TEST_REQUIRE(written.size() == 3, "the answer to a vector and the period's vector both leave");
    MESH_TEST_REQUIRE(written[1].type == "brake.activate", "a vector is answered with the change it lacks");
    MESH_TEST_REQUIRE(written[2].pattern == SCE::Mesh::PatternKind::CausalVector,
                      "the period's vector leaves on the brake's channel");
#endif

    std::printf("SCE Mesh causal route through the generated router (%s): PASS\n", SCE_CAUSAL_TRANSPORT_KIND);
    return 0;
}

}  // namespace

int main() {
    try {
        return run_test();
    } catch (const std::exception &ex) {
        std::fprintf(stderr, "FAIL: unexpected exception: %s\n", ex.what());
        return 1;
    } catch (...) {
        std::fprintf(stderr, "FAIL: unknown exception\n");
        return 1;
    }
}
