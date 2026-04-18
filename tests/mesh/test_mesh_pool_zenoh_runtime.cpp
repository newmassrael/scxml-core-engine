// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §14.4 Zenoh pool runtime verification.
//
// Proves the runtime KeyExpr substitution path in the generated
// `TransportRouter::invokeMeshRpc` Zenoh pool branch reaches an actual
// peer queryable — `test_mesh_pool_compile.cpp` only confirmed the
// code compiles. Without this runtime check a regression in the
// placeholder-scan / JSON decode / pool_key.replace flow would
// produce a well-formed binary that silently sends queries into the
// void (no peer declares the un-substituted or mis-substituted key),
// masked by the deadline path that otherwise raises error.invoke.
//
// End-to-end chain exercised here:
//
//   engine pool_zenoh_client.step()
//     → query.gamer transition: idle → calling_gamer
//     → calling_gamer onentry:
//          _mesh_params["id"].push_back("42")
//          performMeshInvoke("#gamer_pool", "invoke_0", ..., data)
//     → TransportRouter::invokeMeshRpc Zenoh pool branch
//          decodes env.data JSON, extracts id="42"
//          pool_key = "sce/player/{id}"  →  "sce/player/42"
//          zenoh_session_.get("sce/player/42", ...)
//     → harness queryable on the client's connect endpoint receives query
//          copies env.invoke_id into reply envelope (for correlation)
//          replies via Query::reply
//     → router on_reply: resp.type := resolveReplyEvent(req.type)
//                        resp.pattern := RpcReply
//                        admitZenohInbound → admitInbound → dispatchToSender
//     → dispatchToSender: invoke_correlation_.handleReply(uuid, Ok, data)
//          → deliver callback raises done.invoke on the engine
//     → engine step(): calling_gamer → ok
//
// Failure modes surfaced here and nowhere else:
//   - pool_json decode fails (missing `id` param or wrong shape)
//   - placeholder-scan misses `{id}` token in pool_key
//   - KeyExpr construction rejects substituted string
//   - harness declares a different literal key (proves substitution ran)
//   - reply env.invoke_id is not propagated back → deadline-only path

#include "pool_zenoh_client_sm.h"
#include "pool_zenoh_client_transport.h"

#include "ZenohTestUtils.h"
#include "mesh/MeshEnvelopeCodec.h"

#include <atomic>
#include <chrono>
#include <cstdio>
#include <string>
#include <thread>

namespace {

using namespace SCE::Test::Mesh;

// Mirrors the client's connect endpoint (deploy_pool_zenoh.yaml ecu1):
// the harness queryable below binds this address so the generated client
// dials it. CMake RESOURCE_LOCK zenoh_pool_port_17452 serializes this
// test against anything else that binds that port — there is no other
// fixture on it today, so the lock is reserved for future co-location safety.
constexpr const char* kListen =
    SCE::Generated::pool_zenoh_client::ZENOH_CONNECT_ENDPOINTS[0];

// The pool's key template is `sce/player/{id}` with the sample
// invoke passing `id="42"`. This literal MUST match what the
// runtime substitution produces — a mismatch is the signal the
// runtime concat path silently mutated or skipped the placeholder.
constexpr const char* kSubstitutedKey = "sce/player/42";

int run_test() {
    namespace client_gen = SCE::Generated::pool_zenoh_client;
    using Client = client_gen::pool_zenoh_client;
    using ClientRouterT = client_gen::TransportRouter<Client>;

    // Harness queryable: stands in for the pool receiver. Listens on
    // exactly the substituted KeyExpr the runtime concat is expected
    // to produce. If the substitution emits anything else (e.g. the
    // template token survives, or a different numeric token lands in
    // the key), zenoh_session_.get will not route to this queryable
    // and the engine never reaches State::Ok — the surviving assertion
    // below catches that with a single, unambiguous failure point.
    auto harness_session = open_peer(/*connect=*/"", /*listen=*/kListen);

    auto queryable = harness_session.declare_queryable(
        zenoh::KeyExpr(kSubstitutedKey),
        [](zenoh::Query& query) {
            // Decode the inbound envelope so we can copy env.invoke_id
            // into the reply. invoke_correlation_.handleReply on the
            // client side keys on the UUID the client stamped at
            // invoke time — the reply must preserve it verbatim or
            // the correlation table entry never clears.
            auto payload_opt = query.get_payload();
            if (!payload_opt) return;
            auto bytes = payload_opt->get().as_vector();
            SCE::Mesh::MeshEnvelope req;
            if (!SCE::Mesh::decodeEnvelope(bytes.data(), bytes.size(), req)) return;

            SCE::Mesh::MeshEnvelope reply;
            reply.id = SCE::uuid::v7();
            reply.invoke_id = req.invoke_id;
            reply.source = "gamer_pool_harness";
            reply.type = "service.response.ping";
            reply.pattern = SCE::Mesh::PatternKind::RpcReply;
            reply.rpc_status = SCE::Mesh::RpcStatus::Ok;
            auto reply_bytes = SCE::Mesh::encodeEnvelope(reply);

            zenoh::Query::ReplyOptions reply_opts;
            query.reply(query.get_keyexpr(), zenoh::Bytes(std::move(reply_bytes)),
                        std::move(reply_opts));
        },
        []() noexcept {});

    // Bring the client SM + router up AFTER the harness queryable
    // is declared. Order matters: init() opens the client's zenoh
    // session and connects to 17452 where the harness must already
    // be accepting, otherwise the 1-second handshake wait below
    // cannot be trusted to converge.
    Client client;
    client.initialize();

    ClientRouterT client_router(client);
    MESH_TEST_REQUIRE(client_router.init(), "client_router.init() failed");

    // Peer handshake. Must come before raising the event so the
    // outbound session.get isn't fired while the transport is still
    // in SYN_SENT and drops to the on_drop path. The client router is
    // generated and exposes no raw session, so a throwaway probe peer
    // joins the same listen endpoint as the client and installs a
    // matching listener on kSubstitutedKey. When the probe sees the
    // harness queryable, the client router — co-located on the same
    // listen endpoint under zenoh peer mode — sees it too.
    auto probe_session = open_peer(/*connect=*/kListen, /*listen=*/"");
    wait_for_queryable(probe_session, kSubstitutedKey);

    // Driver thread pumps the external queue (same RAII pattern as
    // test_mesh_zenoh_rpc_engine_driven.cpp — Session I precedent).
    // The on_reply closure admits inbound envelopes from a zenoh
    // runtime thread; step() dequeues and runs the transition.
    struct EnginePump {
        std::atomic<bool> running{true};
        std::thread t;
        ~EnginePump() {
            running.store(false, std::memory_order_release);
            if (t.joinable()) t.join();
        }
    } pump;
    pump.t = std::thread([&client, &pump] {
        while (pump.running.load(std::memory_order_acquire)) {
            client.step();
            std::this_thread::sleep_for(std::chrono::milliseconds(1));
        }
    });

    // Drive the mesh-rpc: idle → calling_gamer onentry fires the
    // pool invoke. The invoke's dispatch runs on step() (the raise
    // is enqueued externally here), the pool branch builds the
    // substituted KeyExpr, and session.get lands on the harness.
    client.raiseExternal(Client::Event::Query_gamer);

    // Wait for the engine to reach State::Ok within the shared mesh
    // test timeout. The pool dispatch path's on_reply runs on a zenoh
    // runtime thread; admitInbound → dispatchToSender →
    // invoke_correlation_.handleReply raises done.invoke which the
    // driver thread consumes on the next step() tick. Polling (rather
    // than a condvar) keeps the engine API untouched — the generated
    // `pool_zenoh_clientPolicy` exposes no state-transition signal.
    using std::chrono::steady_clock;
    const auto deadline = steady_clock::now() + kDefaultTimeout;
    while (steady_clock::now() < deadline) {
        if (client.getCurrentState() == Client::State::Ok) break;
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }

    MESH_TEST_REQUIRE(client.getCurrentState() == Client::State::Ok,
                     "pool_zenoh_client did not reach State::Ok — "
                     "regression in the §14.4 runtime KeyExpr "
                     "substitution, pool dispatch, or "
                     "admitZenohInbound → handleReply chain");

    pump.running.store(false, std::memory_order_release);
    pump.t.join();
    client_router.shutdown();
    std::printf("SCE Mesh §14.4 Zenoh pool runtime verification: PASS\n");
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
