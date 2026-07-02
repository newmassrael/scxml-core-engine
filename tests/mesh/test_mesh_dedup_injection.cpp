// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §10.5 dedup integration regression.
//
// Exercises the generated `admitInbound` on a Zenoh-gated
// TransportRouter end-to-end: sender engine + codegen + DedupRouter.
// No live zenoh session is needed — the router's dedup path runs
// whether or not `init()` has opened a session, so the test stays
// hermetic and fast.
//
// The guarantee under test is: when the same envelope (identified by
// `(source, id)` per §10.5) is injected twice, the second injection is
// silently dropped and the sender engine sees it exactly once.
//
// Mutation guard (manual procedure): the dedup filter is load-bearing
// iff this assertion fails when it is removed. Temporarily edit
// `sce/include/mesh/DedupRouter.h::DedupRouter::admit` to always
// return `true` (or replace the generated-router wrapper
// `admitInbound` in `tools/codegen/templates/mesh/cpp/mesh_transport.h.jinja2`
// so it unconditionally calls `dispatchToSession`) — either mutation
// must break the "duplicate envelope must be dropped" assertion below
// and the `DuplicateFromSameSenderIsDropped` unit test in
// `DedupRouterTest.cpp`. Revert after verification.

#include "brake_transport.h"

#include "MeshTestUtils.h"
#include "mesh/MeshEnvelope.h"

#include <cstdio>
#include <exception>

namespace {

using namespace SCE::Generated::brake;
using namespace SCE::Test::Mesh;
namespace PK = SCE::Mesh;

int run_test() {
    using RouterT = TransportRouter<TestSenderEngine>;
    TestSenderEngine sender;
    RouterT router({&sender});

    // ── 1. First inject of a fresh envelope ────────────────────────────
    SCE::Mesh::MeshEnvelope env;
    env.id = SCE::uuid::v7();
    env.source = "motor";
    env.type = "brake.activate";
    env.pattern = PK::PatternKind::FireForget;

    MESH_TEST_REQUIRE(router.admitInbound(env, 0), "first inject of a novel envelope must reach the engine");
    {
        std::lock_guard<std::mutex> lk(sender.received_.m);
        MESH_TEST_REQUIRE(sender.received_.events.size() == 1,
                          "sender must record exactly one event after the first inject");
    }

    // ── 2. Second inject of the same envelope is the duplicate case ──
    // §10.5: `admit()` returns false on a repeat (source, id); the
    // envelope never reaches dispatchToSession.
    MESH_TEST_REQUIRE(!router.admitInbound(env, 0),
                      "duplicate envelope (same source+id) must be dropped by DedupRouter");
    {
        std::lock_guard<std::mutex> lk(sender.received_.m);
        MESH_TEST_REQUIRE(sender.received_.events.size() == 1, "duplicate inject must NOT reach the engine");
    }

    // ── 3. A fresh id from the same source is admitted again ────────
    env.id = SCE::uuid::v7();
    MESH_TEST_REQUIRE(router.admitInbound(env, 0), "novel id from the same source must be admitted");
    {
        std::lock_guard<std::mutex> lk(sender.received_.m);
        MESH_TEST_REQUIRE(sender.received_.events.size() == 2, "novel id must reach the engine");
    }

    // ── 4. Same id from a different source is admitted ────────────
    // DedupRouter keys on (source, id), so the cross-source case must
    // not collide even when the ids happen to match.
    env.source = "telemetry";
    MESH_TEST_REQUIRE(router.admitInbound(env, 0), "per-sender window must isolate identical ids across sources");

    std::printf("SCE Mesh dedup injection regression: PASS\n");
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
