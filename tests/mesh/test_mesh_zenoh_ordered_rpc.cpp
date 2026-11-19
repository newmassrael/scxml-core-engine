// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §10.6.3 runtime verification: a mesh-rpc request on an ordered
// binding carries its sequence number.
//
// The receiving half of an ordered binding (`admitOrdered`, §10.6.4) lets
// only RpcReply and FieldNotify past its buffer; any other envelope that
// arrives without `sequence_no` is dropped and raised as
// `error.communication` with reason `missingSequence` (§16.7 row 11). A
// request that leaves without its stamp is therefore never seen by an
// ordered receiver, and a generated C++ machine could not make a mesh-rpc
// request over such a binding. This test drives a real request through the
// generated `TransportRouter` and reads the stamp off the wire.
//
// End-to-end chain exercised here:
//
//   engine brake_zenoh_ordered_rpc.step()
//     → go: idle → computing
//     → computing onentry: <send event="service.request.calibrate"/>
//          → mesh send callback → stampSequence → route_send → session.get
//     → computing invoke: performMeshInvoke("#motor_zenoh_ordered_rpc", ...)
//          → invokeMeshRpc → stampSequence → route_send → session.get
//     → harness queryable on the client's connect endpoint records both
//          requests, and answers the one that carries an invoke_id
//     → router on_reply → admitZenohInbound → admitOrdered (RpcReply
//          passes the buffer) → handleReply → done.invoke
//     → engine step(): computing → ok
//
// Both requests leave on the one binding, so they share its counter: the
// harness must see the sequence numbers 1 and 2 between them. Which of the
// two takes 1 depends on whether the engine runs the onentry `<send>` or the
// invoke first, so the test reads the pair, not their order.

#include "brake_zenoh_ordered_rpc_sm.h"
#include "brake_zenoh_ordered_rpc_transport.h"
#include "common/TestScriptEngine.h"

#include "ZenohTestUtils.h"
#include "mesh/MeshEnvelopeCodec.h"

#include <atomic>
#include <chrono>
#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <exception>
#include <mutex>
#include <optional>
#include <string>
#include <thread>
#include <vector>

namespace {

using namespace SCE::Test::Mesh;

// Mirrors the client's connect endpoint (deploy_zenoh_ordered_rpc.yaml): the
// harness queryable below listens here so the generated client dials it.
// CMake RESOURCE_LOCK zenoh_ordered_rpc_port_17458 serializes the test
// against anything else that binds the port.
constexpr const char *kListen = SCE::Generated::brake_zenoh_ordered_rpc::ZENOH_CONNECT_ENDPOINTS[0];

// The ordered binding's `key:` in deploy_zenoh_ordered_rpc.yaml.
constexpr const char *kKey = "sce/brake_ordered_rpc/motor/cmd";

constexpr const char *kSendRequest = "service.request.calibrate";
constexpr const char *kInvokeRequest = "service.request.compute_force";

// The sequence number the harness saw on the request of `type`, if it saw
// that request and the request carried one. The caller holds `captured.m`.
std::optional<std::optional<std::uint64_t>> stamp_of(const std::vector<SCE::Mesh::MeshEnvelope> &envelopes,
                                                     const std::string &type) {
    for (const auto &env : envelopes) {
        if (env.type == type) {
            return env.sequence_no;
        }
    }
    return std::nullopt;
}

int run_test() {
    namespace client_gen = SCE::Generated::brake_zenoh_ordered_rpc;
    using Client = client_gen::brake_zenoh_ordered_rpc;
    using ClientRouterT = client_gen::TransportRouter<Client>;

    // Every request the harness receives, decoded, in arrival order.
    // Declared before the harness session so it outlives the queryable
    // callback that writes it.
    CapturedEvents captured;

    auto harness_session = open_peer(/*connect=*/"", /*listen=*/kListen);

    auto queryable = harness_session.declare_queryable(
        zenoh::KeyExpr(kKey),
        [&captured](zenoh::Query &query) {
            auto payload_opt = query.get_payload();
            if (!payload_opt) {
                return;
            }
            auto bytes = payload_opt->get().as_vector();
            SCE::Mesh::MeshEnvelope req;
            if (!SCE::Mesh::decodeEnvelope(bytes.data(), bytes.size(), req)) {
                return;
            }
            captured.push(req);

            // Only the invoke's request is answered: it is the one a reply
            // can retire (its invoke_id keys the requester's correlation
            // entry). The `<send>`'s request goes unanswered, and its query
            // ends without a reply, which the router ignores for a request
            // that carries no invoke_id.
            if (!req.invoke_id) {
                return;
            }
            SCE::Mesh::MeshEnvelope reply;
            reply.id = SCE::uuid::v7();
            reply.invoke_id = req.invoke_id;
            // The binding's responder set (§14.6) is its target alone, so
            // the reply names that machine as a generated server would.
            reply.source = "motor_zenoh_ordered_rpc";
            reply.type = "service.response.compute_force";
            reply.pattern = SCE::Mesh::PatternKind::RpcReply;
            reply.rpc_status = SCE::Mesh::RpcStatus::Ok;
            auto reply_bytes = SCE::Mesh::encodeEnvelope(reply);

            zenoh::Query::ReplyOptions reply_opts;
            query.reply(query.get_keyexpr(), zenoh::Bytes(std::move(reply_bytes)), std::move(reply_opts));
        },
        []() noexcept {});

    // The client comes up after the harness queryable is declared, so its
    // init() dials an endpoint that is already accepting.
    Client client;
    ::SCE::Test::start_or_abort(client);

    ClientRouterT client_router({&client});
    MESH_TEST_REQUIRE(client_router.init(), "client_router.init() failed");

    // Peer handshake before the first request: a probe peer that joins the
    // same listen endpoint waits until the harness queryable matches, which
    // under zenoh peer mode means the co-located client sees it too.
    auto probe_session = open_peer(/*connect=*/kListen, /*listen=*/"");
    wait_for_queryable(probe_session, kKey);

    // The driver thread owns the engine (ARCHITECTURE.md "Engine
    // Threading"): it steps it, and publishes in `reachedOk` where the run
    // ended; this thread reads only that while the driver runs.
    struct EnginePump {
        std::atomic<bool> running{true};
        std::atomic<bool> reachedOk{false};
        std::thread t;

        ~EnginePump() {
            running.store(false, std::memory_order_release);
            if (t.joinable()) {
                t.join();
            }
        }
    } pump;

    pump.t = std::thread([&client, &pump] {
        while (pump.running.load(std::memory_order_acquire)) {
            client.step();
            if (client.terminalState() == Client::State::Ok) {
                pump.reachedOk.store(true, std::memory_order_release);
            }
            std::this_thread::sleep_for(std::chrono::milliseconds(1));
        }
    });

    MESH_TEST_REQUIRE(client.inbox().post(Client::InboxExternalEvent{Client::EventWithMetadata(Client::Event::Go)}),
                      "the client's inbox refused the event that starts the requests");

    using std::chrono::steady_clock;
    const auto deadline = steady_clock::now() + kDefaultTimeout;
    while (steady_clock::now() < deadline) {
        if (pump.reachedOk.load(std::memory_order_acquire)) {
            break;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }

    // The driver stops before the verdict, so nothing reads the engine while
    // it runs.
    pump.running.store(false, std::memory_order_release);
    pump.t.join();
    MESH_TEST_REQUIRE(pump.reachedOk.load(std::memory_order_acquire),
                      "brake_zenoh_ordered_rpc did not reach State::Ok: the mesh-rpc request or its reply "
                      "did not cross the ordered binding");

    // Both requests reached the harness; the `<send>`'s may trail the
    // invoke's, since the two queries are independent.
    const bool both_arrived =
        captured.wait_for([](const std::vector<SCE::Mesh::MeshEnvelope> &envelopes) { return envelopes.size() >= 2; });
    MESH_TEST_REQUIRE(both_arrived, "the harness did not receive both requests on the ordered binding");

    std::optional<std::optional<std::uint64_t>> send_stamp;
    std::optional<std::optional<std::uint64_t>> invoke_stamp;
    std::size_t received = 0;
    {
        std::lock_guard<std::mutex> lock(captured.m);
        received = captured.envelopes.size();
        send_stamp = stamp_of(captured.envelopes, kSendRequest);
        invoke_stamp = stamp_of(captured.envelopes, kInvokeRequest);
    }
    client_router.shutdown();

    MESH_TEST_REQUIRE(received == 2, "the binding carried a request beyond the `<send>` and the invoke");
    MESH_TEST_REQUIRE(send_stamp.has_value(), "the harness did not receive the `<send>`'s request");
    MESH_TEST_REQUIRE(invoke_stamp.has_value(), "the harness did not receive the mesh-rpc request");
    MESH_TEST_REQUIRE(send_stamp->has_value(), "the `<send>`'s request left the ordered binding without sequence_no");
    MESH_TEST_REQUIRE(invoke_stamp->has_value(),
                      "the mesh-rpc request left the ordered binding without sequence_no; an ordered "
                      "receiver drops it as missingSequence");

    const std::uint64_t first = **send_stamp < **invoke_stamp ? **send_stamp : **invoke_stamp;
    const std::uint64_t second = **send_stamp < **invoke_stamp ? **invoke_stamp : **send_stamp;
    MESH_TEST_REQUIRE(first == 1 && second == 2,
                      "the `<send>` and the mesh-rpc request on one binding must draw 1 and 2 from its one "
                      "counter");

    std::printf("SCE Mesh §10.6.3 ordered mesh-rpc stamp verification: PASS\n");
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
