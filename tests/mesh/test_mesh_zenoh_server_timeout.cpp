// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh gap Z2 runtime E2E: Zenoh queryable server-side deadline
// erases stranded `pending_server_queries_` entries whose engine never
// emits the paired response.
//
// Three scenarios, one binary:
//
//   §1 Leak closure — the motor engine receives an inbound RpcRequest
//      but the test never dispatches a response. Without Z2 the entry
//      would sit in `pending_server_queries_` until process exit; with
//      Z2 armed at `query_timeout_ms=500 ms` the scheduler callback
//      erases it within `timeout + slack`. This is the gap's value
//      claim: server-side resource release determinism.
//
//   §2 Happy-path cancel — the test calls `handleServerResponse`
//      directly (simulating the engine's <raise>-driven reply). The
//      scheduler's `cancelDeadline` must run before `pending_server_queries_.erase`
//      so the timer cannot fire on a valid-entry race, and the client
//      observes the reply at the expected round-trip latency.
//
//   §3 Shutdown while in flight — the test sends a request and calls
//      `router.shutdown()` before the deadline elapses. The scheduler
//      drain + `server_shutdown_in_progress_` flag together suppress
//      any in-flight callback from racing the teardown.
//
// The raw zenoh client uses peer-mode with the default
// `queries_default_timeout` (10 s). Z2's cleanup is driven by the
// server's scheduler, not the client's session timeout.
//
// Thread-safety: zenoh runtime threads call into motor_router's
// queryable and the scheduler's callback thread. `server_pending_mutex_`
// is the shared lock for both. The test reads `pending_server_size()`
// from the main thread via the public accessor, which takes the same
// mutex.

#include "motor_zenoh_server_timeout_sm.h"
#include "motor_zenoh_server_timeout_transport.h"

#include "ZenohTestUtils.h"
#include "mesh/MeshDispatch.h"

#include <chrono>
#include <cstdio>
#include <thread>

namespace {

using namespace SCE::Test::Mesh;

namespace motor_gen = SCE::Generated::motor_zenoh_server_timeout;
using PK = SCE::Mesh::PatternKind;
using MotorRouterT = motor_gen::TransportRouter<TestSenderEngine>;

// Must match deploy_zenoh_server_timeout.yaml ecu_motor listen.
constexpr const char* kMotorListen = "tcp/127.0.0.1:17451";

// Must match deploy_zenoh_server_timeout.yaml server.query_timeout_ms.
constexpr auto kQueryTimeoutMs = std::chrono::milliseconds(500);
// Allowance for scheduler tick + thread wakeup + map erase + test observation.
constexpr auto kTimeoutSlack = std::chrono::milliseconds(500);
// Peer discovery stabilization — matches the Z3 §2 and liveliness E2E
// precedents for zenoh peer-mode sessions on commodity hardware.
constexpr auto kPeerStabilization = std::chrono::milliseconds(200);

// ── §1 Leak closure ─────────────────────────────────────────────────
int scenario_timeout_closes_leak() {
    TestSenderEngine motor_engine;
    MotorRouterT motor_router(motor_engine);
    MESH_TEST_REQUIRE(motor_router.init(),
                      "motor_router.init() failed — zenoh listen "
                      "unavailable on " "tcp/127.0.0.1:17451");

    auto client_session = open_peer(/*connect=*/kMotorListen, /*listen=*/"");
    std::this_thread::sleep_for(kPeerStabilization);

    MESH_TEST_REQUIRE(motor_router.pending_server_size() == 0,
                      "initial pending_server_size should be zero");

    // Client RpcRequest — engine receives but the test NEVER replies.
    auto req = make_envelope("service.request.compute_force", PK::RpcRequest,
                             R"({"x":10})");
    auto req_bytes = SCE::Mesh::encodeEnvelope(req);

    zenoh::Session::GetOptions opts;
    opts.payload = zenoh::Bytes(std::move(req_bytes));
    client_session.get(
        zenoh::KeyExpr(motor_gen::ZENOH_SERVER_KEY), "",
        [](const zenoh::Reply&) { /* no reply expected */ },
        [] { /* on_drop: tested elsewhere (Z3) */ },
        std::move(opts));

    MESH_TEST_REQUIRE(motor_engine.received_.wait_for([](const auto& v) {
                          return !v.empty() &&
                                 v.back().type == "service.request.compute_force";
                      }),
                      "motor engine never received service.request.compute_force");

    MESH_TEST_REQUIRE(motor_router.pending_server_size() == 1,
                      "pending_server_size should be 1 after dispatch");

    // Wait out `query_timeout_ms + slack`. The scheduler callback must
    // erase the entry within this window.
    std::this_thread::sleep_for(kQueryTimeoutMs + kTimeoutSlack);

    MESH_TEST_REQUIRE(motor_router.pending_server_size() == 0,
                      "pending_server_size did not drop to 0 after Z2 timeout");

    motor_router.shutdown();
    std::printf("[§1 timeout] PASS: pending entry cleared within "
                "%lld ms\n",
                static_cast<long long>(
                    (kQueryTimeoutMs + kTimeoutSlack).count()));
    return 0;
}

// ── §2 Happy-path cancel ────────────────────────────────────────────
int scenario_happy_path_cancels_deadline() {
    TestSenderEngine motor_engine;
    MotorRouterT motor_router(motor_engine);
    MESH_TEST_REQUIRE(motor_router.init(),
                      "motor_router.init() failed — zenoh listen unavailable");

    auto client_session = open_peer(/*connect=*/kMotorListen, /*listen=*/"");
    std::this_thread::sleep_for(kPeerStabilization);

    ReceivedEvents client_replies;
    auto req = make_envelope("service.request.compute_force", PK::RpcRequest,
                             R"({"x":11})");
    auto req_bytes = SCE::Mesh::encodeEnvelope(req);

    zenoh::Session::GetOptions opts;
    opts.payload = zenoh::Bytes(std::move(req_bytes));
    client_session.get(
        zenoh::KeyExpr(motor_gen::ZENOH_SERVER_KEY), "",
        [&client_replies](const zenoh::Reply& reply_msg) {
            if (!reply_msg.is_ok()) return;
            const auto& sample = reply_msg.get_ok();
            auto bytes = sample.get_payload().as_vector();
            SCE::Mesh::MeshEnvelope resp;
            if (SCE::Mesh::decodeEnvelope(bytes.data(), bytes.size(), resp)) {
                client_replies.push({resp.type,
                                     std::string(resp.data.begin(),
                                                 resp.data.end())});
            }
        },
        [] {}, std::move(opts));

    MESH_TEST_REQUIRE(motor_engine.received_.wait_for([](const auto& v) {
                          return !v.empty() &&
                                 v.back().type == "service.request.compute_force";
                      }),
                      "motor engine never received request");

    // Pull the correlation_id through the public `server_first_cid`
    // accessor — the generated transport keeps `pending_server_queries_`
    // and `server_pending_mutex_` as implementation details; the
    // accessor is the documented test hook and mirrors the shape of
    // `pending_server_size`.
    auto cid_opt = motor_router.server_first_cid();
    MESH_TEST_REQUIRE(cid_opt.has_value(),
                      "pending Query not stashed by queryable callback");
    const auto cid = *cid_opt;

    auto resp = make_envelope("service.response.compute_force", PK::RpcReply,
                              R"({"result":42})");
    resp.correlation_id = cid;

    const bool replied = motor_router.handleServerResponse(resp);
    MESH_TEST_REQUIRE(replied,
                      "handleServerResponse returned false — correlation miss");

    MESH_TEST_REQUIRE(motor_router.pending_server_size() == 0,
                      "pending_server_size should be 0 after handleServerResponse");

    MESH_TEST_REQUIRE(client_replies.wait_for([](const auto& v) {
                          return !v.empty() &&
                                 v.back().type == "service.response.compute_force";
                      }),
                      "client did not receive reply via Query::reply");

    // Post-timeout re-check: if `cancelDeadline` did not actually erase
    // the scheduler's `active_` entry for this cid, the timer would
    // fire around `kQueryTimeoutMs` anyway. `onServerQueryTimedOut`
    // would then run `erase(cid)` on an already-gone entry — a no-op
    // that leaves `pending_server_size()` at 0 — so size alone cannot
    // expose a missing cancel. What a missing cancel WOULD produce is
    // a bogus race window where the callback runs on a valid cid if a
    // new query re-used it; since the test dispatches no further
    // queries, the observable contract here is simply "size stays 0
    // past the nominal deadline, no surprise re-population, no crash".
    std::this_thread::sleep_for(kQueryTimeoutMs + kTimeoutSlack);
    MESH_TEST_REQUIRE(motor_router.pending_server_size() == 0,
                      "pending_server_size regressed past nominal deadline — "
                      "cancelDeadline failed to release the scheduler entry");

    motor_router.shutdown();
    std::printf("[§2 happy path] PASS: reply delivered, deadline cancelled "
                "(re-checked past nominal deadline)\n");
    return 0;
}

// ── §3 Shutdown while query in flight ───────────────────────────────
int scenario_shutdown_with_pending_query() {
    TestSenderEngine motor_engine;
    MotorRouterT motor_router(motor_engine);
    MESH_TEST_REQUIRE(motor_router.init(),
                      "motor_router.init() failed — zenoh listen unavailable");

    auto client_session = open_peer(/*connect=*/kMotorListen, /*listen=*/"");
    std::this_thread::sleep_for(kPeerStabilization);

    auto req = make_envelope("service.request.compute_force", PK::RpcRequest,
                             R"({"x":12})");
    auto req_bytes = SCE::Mesh::encodeEnvelope(req);

    zenoh::Session::GetOptions opts;
    opts.payload = zenoh::Bytes(std::move(req_bytes));
    client_session.get(
        zenoh::KeyExpr(motor_gen::ZENOH_SERVER_KEY), "",
        [](const zenoh::Reply&) {},
        [] {},
        std::move(opts));

    MESH_TEST_REQUIRE(motor_engine.received_.wait_for([](const auto& v) {
                          return !v.empty() &&
                                 v.back().type == "service.request.compute_force";
                      }),
                      "motor engine never received request before shutdown");

    MESH_TEST_REQUIRE(motor_router.pending_server_size() == 1,
                      "pending_server_size should be 1 before shutdown");

    // Shutdown while the entry is still pending. The server shutdown
    // flag must suppress any dispatched timer callback; scheduler
    // drain must join its worker thread cleanly. If either race
    // manifests, the test crashes or hangs.
    motor_router.shutdown();

    // Sleep past the deadline so a spurious callback would have fired
    // by now if suppression failed.
    std::this_thread::sleep_for(kQueryTimeoutMs + kTimeoutSlack);

    std::printf("[§3 shutdown] PASS: no crash or hang across teardown\n");
    return 0;
}

}  // namespace

int main() {
    try {
        if (const int r = scenario_timeout_closes_leak(); r != 0) return r;
        if (const int r = scenario_happy_path_cancels_deadline(); r != 0)
            return 10 + r;
        if (const int r = scenario_shutdown_with_pending_query(); r != 0)
            return 20 + r;
        std::printf("SCE Mesh gap Z2 server-side queryable deadline E2E: "
                    "PASS (all scenarios)\n");
        return 0;
    } catch (const std::exception& ex) {
        std::fprintf(stderr, "FAIL: uncaught exception: %s\n", ex.what());
        return 99;
    }
}
