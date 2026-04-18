// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh gap Z3 runtime E2E: Zenoh `session.get` on_drop wiring
// early-cancels a mesh-rpc invoke whose reply never arrives.
//
// Two scenarios — same Z3 code path, different zenoh trigger:
//
//   §1 No peer at all — brake opens its peer-mode session with no other
//      zenoh peer in the process. Queries have no route anywhere; zenoh
//      terminates them effectively immediately and fires on_drop. This
//      path exercises the `queries_default_timeout` fast-path (zenoh
//      short-circuits when the routing table is empty for the keyexpr).
//
//   §2 Peer present but silent — a raw zenoh peer (spun up inside the
//      test) connects to brake's listen endpoint and declares a
//      queryable on brake's keyexpr, but the queryable callback
//      deliberately never replies. brake's query reaches the peer,
//      the peer ignores it, and brake's session times out at
//      `queries_default_timeout` (300 ms from zenoh_on_drop_session.json5).
//      This path verifies the value claim of the Z3 fix: when a real
//      deployed peer misbehaves, on_drop fires at the configured
//      short timeout rather than the 10 s `_mesh_deadline_ms` budget.
//
// In both scenarios Z3 translates on_drop into `error.invoke.<id>` via
// `RpcStatus::Unavailable`; SCE code path is identical. Testing both
// confirms the Z3 wiring is insensitive to the specific zenoh trigger
// that surfaced the query termination.
//
// Thread-safety: zenoh runtime thread raises error.invoke on brake's
// external queue while the main thread drives step(). SCE_THREAD_SAFE
// enables the queue's internal mutex — sleep_for alone does not
// establish happens-before per the C++ memory model. Same rationale as
// `test_mesh_invoke_deadline_expiry`.

#include "brake_zenoh_on_drop_sm.h"
#include "brake_zenoh_on_drop_transport.h"

#include "ZenohTestUtils.h"

#include <atomic>
#include <chrono>
#include <cstdio>
#include <mutex>
#include <thread>
#include <vector>

namespace {

using namespace SCE::Test::Mesh;

using BrakeSm = SCE::Generated::brake_zenoh_on_drop::brake_zenoh_on_drop;
using BrakeState = SCE::Generated::brake_zenoh_on_drop::State;
using BrakeEvent = SCE::Generated::brake_zenoh_on_drop::Event;
using RouterT = SCE::Generated::brake_zenoh_on_drop::TransportRouter<BrakeSm>;

// Observation budget: zenoh queries_default_timeout (300 ms from
// zenoh_on_drop_session.json5) + on_drop scheduling slack + step() +
// jitter. The SCXML `_mesh_deadline_ms` is 10 000 ms, so any arrival
// under ~2 s proves on_drop — not the SCE deadline scheduler — drove
// the cancellation.
constexpr auto kZ3ObservationBudget = std::chrono::milliseconds(2000);

// Mirrors brake's own listen endpoint (deploy_zenoh_on_drop.yaml ecu_brake):
// the raw motor peer in §2 dials this address to sit on brake's routing table.
constexpr const char* kBrakeListen =
    SCE::Generated::brake_zenoh_on_drop::ZENOH_LISTEN_ENDPOINTS[0];
// Must match deploy_zenoh_on_drop.yaml brake binding key.
constexpr const char* kMotorKey = "sce/brake_on_drop/motor/compute_force";

// §1 No peer at all — brake's session has nothing to route queries to.
// Zenoh short-circuits the query and fires on_drop effectively
// immediately; empirically well under 10 ms on commodity hardware.
int scenario_no_peer() {
    BrakeSm brake;
    RouterT brake_router(brake);

    if (!brake_router.init()) {
        std::fprintf(stderr,
                     "FAIL [§1 no-peer]: brake_router.init() failed — zenoh "
                     "session open could not bind to %s.\n", kBrakeListen);
        return 1;
    }

    brake.initialize();

    const auto start = std::chrono::steady_clock::now();
    brake.processEvent(BrakeEvent::Go);
    while (std::chrono::steady_clock::now() - start < kZ3ObservationBudget) {
        if (brake.getCurrentState() == BrakeState::Failed) break;
        brake.step();
        if (brake.getCurrentState() == BrakeState::Failed) break;
        std::this_thread::sleep_for(std::chrono::milliseconds(20));
    }
    const auto elapsed_ms = std::chrono::duration_cast<std::chrono::milliseconds>(
        std::chrono::steady_clock::now() - start).count();

    if (brake.getCurrentState() != BrakeState::Failed) {
        std::fprintf(stderr, "FAIL [§1 no-peer]: brake stuck at state=%d after %lld ms.\n",
                     static_cast<int>(brake.getCurrentState()),
                     static_cast<long long>(elapsed_ms));
        brake_router.shutdown();
        return 2;
    }

    brake_router.shutdown();
    std::printf("[§1 no-peer] PASS: error.invoke observed in %lld ms\n",
                static_cast<long long>(elapsed_ms));
    return 0;
}

// §2 Peer present but silent — brake listens first, THEN a raw zenoh
// peer connects and declares a queryable on brake's keyexpr whose
// callback deliberately never replies. brake's query routes to the
// peer, sits unserved, and fires on_drop at `queries_default_timeout`
// (300 ms from zenoh_on_drop_session.json5). Documents the
// "deployed-but-misbehaving" failure shape production workloads hit
// most often.
//
// Order matters: brake_router.init() must listen BEFORE the raw peer
// attempts to connect, otherwise zenoh's retry/scouting interleaves
// unpredictably with the stabilization sleep and the queryable may
// never reach brake's routing table before Go is dispatched.
int scenario_peer_silent() {
    BrakeSm brake;
    RouterT brake_router(brake);

    if (!brake_router.init()) {
        std::fprintf(stderr,
                     "FAIL [§2 peer-silent]: brake_router.init() failed — "
                     "zenoh session open could not bind to %s. "
                     "Leftover listener from §1?\n", kBrakeListen);
        return 1;
    }

    brake.initialize();

    // Raw peer connects into brake's listen. Must happen AFTER
    // brake_router.init() so the TCP listener is up; otherwise the
    // peer retries in a background thread on an indeterminate schedule.
    auto motor_session = open_peer(/*connect=*/kBrakeListen, /*listen=*/"");

    // `query_hits` is the diagnostic that distinguishes this scenario
    // from §1: if the callback never fires, the peer discovery race
    // degenerated this test back to the no-peer fast-path and the
    // "peer-silent" claim is unproven. The final assertion enforces
    // the callback DID fire.
    //
    // `held_queries` keeps each Query alive via `Query::clone()` past
    // the callback's scope. Without cloning, zenoh interprets the
    // callback returning without `reply()` as "no more responses from
    // this queryable" and surfaces query termination to brake in
    // milliseconds — that would collapse this scenario into §1's
    // fast-path. Holding the clone keeps the server side of the query
    // open; brake's side therefore terminates via
    // `queries_default_timeout` (300 ms), which is the path Z3 is
    // designed to ride in production (a peer that's alive but slow).
    std::atomic<int> query_hits{0};
    std::mutex held_mutex;
    std::vector<zenoh::Query> held_queries;

    [[maybe_unused]] auto silent_queryable = motor_session.declare_queryable(
        zenoh::KeyExpr(kMotorKey),
        [&query_hits, &held_mutex, &held_queries](const zenoh::Query& q) {
            query_hits.fetch_add(1, std::memory_order_relaxed);
            std::lock_guard<std::mutex> lk(held_mutex);
            held_queries.push_back(q.clone());
        },
        []() noexcept {});

    // Stabilization: zenoh peer discovery completes asynchronously
    // after session open + queryable declare. 200 ms matches the
    // mesh_zenoh_liveliness E2E sleep and has been empirically
    // adequate on commodity hardware.
    std::this_thread::sleep_for(std::chrono::milliseconds(200));

    const auto start = std::chrono::steady_clock::now();
    brake.processEvent(BrakeEvent::Go);
    while (std::chrono::steady_clock::now() - start < kZ3ObservationBudget) {
        if (brake.getCurrentState() == BrakeState::Failed) break;
        brake.step();
        if (brake.getCurrentState() == BrakeState::Failed) break;
        std::this_thread::sleep_for(std::chrono::milliseconds(20));
    }
    const auto elapsed_ms = std::chrono::duration_cast<std::chrono::milliseconds>(
        std::chrono::steady_clock::now() - start).count();

    if (brake.getCurrentState() != BrakeState::Failed) {
        std::fprintf(stderr,
                     "FAIL [§2 peer-silent]: brake stuck at state=%d after "
                     "%lld ms (queryable hits=%d).\n",
                     static_cast<int>(brake.getCurrentState()),
                     static_cast<long long>(elapsed_ms),
                     query_hits.load(std::memory_order_relaxed));
        brake_router.shutdown();
        return 2;
    }

    // Enforce the scenario's distinguishing claim: the queryable
    // callback MUST have been invoked. If not, zenoh short-circuited
    // the query before reaching the peer — the test then proves the
    // same thing as §1 and the extra setup is noise.
    const int hits = query_hits.load(std::memory_order_relaxed);
    if (hits == 0) {
        std::fprintf(stderr,
                     "FAIL [§2 peer-silent]: queryable callback never fired "
                     "(hits=0). Scenario degenerated to §1 — peer discovery "
                     "incomplete when Go was dispatched. Increase the 200 ms "
                     "stabilization sleep or investigate zenoh routing.\n");
        brake_router.shutdown();
        return 3;
    }

    brake_router.shutdown();
    std::printf("[§2 peer-silent] PASS: error.invoke observed in %lld ms "
                "after %d queryable hit(s)\n",
                static_cast<long long>(elapsed_ms), hits);
    return 0;
}

}  // namespace

int main() {
    try {
        if (const int r1 = scenario_no_peer(); r1 != 0) return r1;
        if (const int r2 = scenario_peer_silent(); r2 != 0) return 10 + r2;
        std::printf("SCE Mesh gap Z3 on_drop early-cancel E2E: PASS "
                    "(both scenarios)\n");
        return 0;
    } catch (const std::exception& ex) {
        std::fprintf(stderr, "FAIL: uncaught exception: %s\n", ex.what());
        return 1;
    }
}
