// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE-VERIFIES: mesh-9.5
//
// SCE Mesh §9.5 server response deadline, custom_tcp arm.
//
// The SOME/IP arm answers expiry through a slot the protocol reserved for
// it (MT_ERROR / E_TIMEOUT). custom_tcp reserves nothing, because SCE
// defines the framing itself — so the notice here is an ordinary framed
// `MeshEnvelope`, and what makes it correct is not a message-type field
// but WHERE it goes: back on the stream the request arrived on
// (§mesh-10.4.4). That stream is the stashed `pending_server_links_`
// entry, so the notice reaches the connection that asked and no other,
// without any addressing decision at expiry time.
//
// Four scenarios, one binary:
//
//   §1 Leak closure — motor receives an inbound RpcRequest but its
//      recorder engine never emits the paired response. The stashed
//      `pending_server_links_` entry must be released within
//      `response_deadline_ms + slack`.
//
//   §2 Active notice — the requesting peer observes the timeout as an
//      event, not as silence, and NOT as its declared reply event: a
//      failure renamed to `service.response.compute_force` would report
//      a false success on an empty payload. Brake therefore observes
//      `error.rpc.deadline`, whose `_event.data` names the abandoned
//      call. The same request also proves the client-side correlation
//      entry is retired — a notice that reached the engine while leaving
//      `pending_rpcs_` populated would look identical from the outside
//      and grow the table once per request forever.
//
//   §3 Happy-path cancel — the test calls `handleServerResponse` before
//      the deadline elapses. `cancelDeadline` must run before the map
//      erase so the timer cannot fire on a valid entry and write a
//      timeout notice onto a stream that was answered normally.
//
//   §4 Shutdown while in flight — a request is left pending and
//      `shutdown()` runs before the deadline. The scheduler drain plus
//      `server_shutdown_in_progress_` must suppress the callback; in
//      particular the drain has to precede the TCP server teardown, or a
//      late timer would write into a shut-down listener.
//
// Both routers bind recorder engines (`TestSenderEngine`). The engine is
// deliberately not the system under test — what is under test is the
// transport-level lifecycle, and a recorder is exactly the "engine that
// never answers" the deadline exists for.

#include "brake_tcp_server_deadline_sm.h"
#include "brake_tcp_server_deadline_transport.h"
#include "motor_tcp_server_deadline_sm.h"
#include "motor_tcp_server_deadline_transport.h"

#include "MeshTestUtils.h"
#include "common/Uuid.h"

#include <chrono>
#include <cstdio>
#include <string>
#include <thread>

namespace {

using namespace SCE::Test::Mesh;

namespace brake_gen = SCE::Generated::brake_tcp_server_deadline;
namespace motor_gen = SCE::Generated::motor_tcp_server_deadline;

using PK = SCE::Mesh::PatternKind;
using BrakeRouterT = brake_gen::TransportRouter<TestSenderEngine>;
using MotorRouterT = motor_gen::TransportRouter<TestSenderEngine>;

// Must match deploy_tcp_server_deadline.yaml server.response_deadline_ms.
constexpr auto kResponseDeadline = std::chrono::milliseconds(500);
// Allowance for scheduler tick + thread wakeup + map erase + the loopback
// round trip the notice takes before brake observes it.
constexpr auto kSlack = std::chrono::milliseconds(1500);

// Poll a predicate until it holds or the budget runs out. Used for the
// server-side observations, which are state reads rather than event
// deliveries and so have no condition variable to wait on.
template <typename Pred> bool wait_until(Pred &&pred, std::chrono::milliseconds budget) {
    const auto deadline = std::chrono::steady_clock::now() + budget;
    while (std::chrono::steady_clock::now() < deadline) {
        if (pred()) {
            return true;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }
    return pred();
}

// Send one RpcRequest from brake to motor through the router's own mesh
// send callback — the hook the engine invokes for
// `<send event="service.request.compute_force" target="#motor"/>`.
//
// This matters for what the test can see. `route_send(target, env)` takes
// a pre-built envelope and skips correlation registration, so a test
// driving it never populates `pending_rpcs_` and never reaches the
// client-side reply rewrite. Going through the callback is what an SCXML
// author's `<send>` does, so the rewrite — including its refusal to
// rename a failed reply into the declared success event — is on the path
// under test.
bool send_request(TestSenderEngine &brake_engine) {
    return brake_engine.mesh_send_cb_("#motor_tcp_server_deadline", "service.request.compute_force", R"({"force":100})",
                                      "", "");
}

int run_test() {
    TestSenderEngine motor_engine;
    MotorRouterT motor_router({&motor_engine});
    MESH_TEST_REQUIRE(motor_router.init(), "motor router init failed");

    TestSenderEngine brake_engine;
    BrakeRouterT brake_router({&brake_engine});
    MESH_TEST_REQUIRE(brake_router.init(), "brake router init failed");

    // ── §1 + §2: leak closure and the active notice ───────────────────
    //
    // One request drives both assertions: the server-side release and the
    // client-side observation are two ends of the same expiry, and
    // splitting them across requests would let one pass on the other's
    // envelope.
    {
        MESH_TEST_REQUIRE(motor_router.pending_server_size() == 0, "initial pending_server_size should be zero");
        MESH_TEST_REQUIRE(brake_router.pending_client_rpc_size() == 0,
                          "initial pending_client_rpc_size should be zero");
        brake_engine.received_.clear();

        MESH_TEST_REQUIRE(send_request(brake_engine), "brake mesh send callback returned false");

        // Registration, asserted before the reply can arrive (the
        // deadline is 500 ms away). Without this the "retired" check
        // below would pass trivially on a client that never registered
        // the entry at all — zero is zero either way.
        MESH_TEST_REQUIRE(brake_router.pending_client_rpc_size() == 1,
                          "the outbound request did not register a correlation entry — the "
                          "retirement assertion below would then be vacuous");

        // The request must actually reach the server before the deadline
        // claim means anything — otherwise a request lost on the wire
        // would satisfy "pending drops to zero" trivially.
        MESH_TEST_REQUIRE(wait_until([&] { return motor_router.pending_server_size() == 1; }, std::chrono::seconds(5)),
                          "inbound request never reached motor's pending_server_links_ — "
                          "cannot attribute a later empty map to the deadline");

        MESH_TEST_REQUIRE(
            wait_until([&] { return motor_router.pending_server_size() == 0; }, kResponseDeadline + kSlack),
            "stranded pending_server_links_ entry outlived response_deadline_ms + slack — "
            "the server-side deadline did not fire");

        // §2: the notice, not just the release. Brake must surface the
        // server-authored event name — renaming it to the declared reply
        // event would be a false success, and never writing it would make
        // the timeout indistinguishable from silence.
        MESH_TEST_REQUIRE(brake_engine.received_.wait_for(
                              [](const auto &v) { return !v.empty() && v.back().type == "error.rpc.deadline"; }),
                          "requesting peer never observed the timeout notice — either the server "
                          "did not write it back on the request's stream, or the client dropped it");

        // The reason text names the abandoned call, which is what makes
        // the notice actionable when several requests are outstanding.
        {
            std::string data;
            brake_engine.received_.wait_for([&](const auto &v) {
                for (const auto &ev : v) {
                    if (ev.type == "error.rpc.deadline") {
                        data = ev.data;
                        return true;
                    }
                }
                return false;
            });
            MESH_TEST_REQUIRE(data.find("service.request.compute_force") != std::string::npos,
                              "timeout notice must name the abandoned call in _event.data");
        }

        MESH_TEST_REQUIRE(brake_engine.received_.wait_for(
                              [](const auto &v) {
                                  for (const auto &ev : v) {
                                      if (ev.type == "service.response.compute_force") {
                                          return false;
                                      }
                                  }
                                  return true;
                              },
                              std::chrono::seconds(0)),
                          "a timed-out request must never surface as the declared reply event — "
                          "that reports a false success on an empty payload");

        // A failed reply is still an answer, so it retires the one-shot
        // correlation entry. Asserted directly because the failure mode is
        // silent: the event reaches the engine either way, and only this
        // read distinguishes "correlated" from "merely arrived".
        MESH_TEST_REQUIRE(brake_router.pending_client_rpc_size() == 0,
                          "the timeout notice reached the engine without retiring its correlation "
                          "entry — pending_rpcs_ would grow once per request for the life of the process");
    }

    // ── §3: happy-path cancel ─────────────────────────────────────────
    //
    // Answering before the deadline must both deliver the reply and retire
    // the timer. Reading `deadline_scheduler_size()` immediately after the
    // reply distinguishes "cancel worked" from "the timer fired and its
    // callback found nothing" — those are observationally identical
    // through `pending_server_size()` alone.
    {
        brake_engine.received_.clear();
        MESH_TEST_REQUIRE(send_request(brake_engine), "brake mesh send callback (happy path) returned false");
        MESH_TEST_REQUIRE(
            wait_until([&] { return motor_router.server_first_cid().has_value(); }, std::chrono::seconds(5)),
            "inbound request never reached motor (happy path)");

        const auto cid = motor_router.server_first_cid();
        MESH_TEST_REQUIRE(cid.has_value(), "server_first_cid must expose the pending correlation");

        SCE::Mesh::MeshEnvelope reply;
        reply.id = SCE::uuid::v7();
        reply.source = "motor_tcp_server_deadline";
        reply.type = "service.response.compute_force";
        reply.pattern = PK::RpcReply;
        reply.correlation_id = *cid;
        reply.invoke_id = *cid;
        MESH_TEST_REQUIRE(motor_router.handleServerResponse(reply), "handleServerResponse did not correlate the reply");
        MESH_TEST_REQUIRE(motor_router.deadline_scheduler_size() == 0,
                          "cancelDeadline did not release the scheduler entry — a timer left armed "
                          "on an answered request can still write a spurious timeout notice");
        MESH_TEST_REQUIRE(motor_router.pending_server_size() == 0, "answered request must leave the pending map empty");

        MESH_TEST_REQUIRE(brake_engine.received_.wait_for([](const auto &v) {
            return !v.empty() && v.back().type == "service.response.compute_force";
        }),
                          "normal reply not delivered on the happy path");

        // Nothing may follow it: the cancelled timer must not fire later
        // and write a notice for a request already answered.
        std::this_thread::sleep_for(kResponseDeadline + std::chrono::milliseconds(300));
        MESH_TEST_REQUIRE(brake_engine.received_.wait_for(
                              [](const auto &v) {
                                  for (const auto &ev : v) {
                                      if (ev.type == "error.rpc.deadline") {
                                          return false;
                                      }
                                  }
                                  return true;
                              },
                              std::chrono::seconds(0)),
                          "a cancelled deadline still emitted a timeout notice");
    }

    // ── §4: shutdown while a request is in flight ─────────────────────
    //
    // The scheduler drain runs before the custom_tcp server teardown, so
    // the last callback has returned by the time the listener goes down.
    // A clean return from shutdown() with a pending entry is the
    // assertion: the failure mode is a hang or a write into a closed
    // socket, not a wrong value.
    {
        brake_engine.received_.clear();
        MESH_TEST_REQUIRE(send_request(brake_engine), "brake mesh send callback (shutdown path) returned false");
        MESH_TEST_REQUIRE(wait_until([&] { return motor_router.pending_server_size() == 1; }, std::chrono::seconds(5)),
                          "inbound request never reached motor (shutdown path)");
        motor_router.shutdown();
    }

    brake_router.shutdown();
    std::printf("SCE Mesh §9.5 custom_tcp server response deadline: PASS\n");
    return 0;
}

}  // namespace

int main() {
    try {
        return run_test();
    } catch (const std::exception &ex) {
        std::fprintf(stderr, "SCE Mesh custom_tcp server deadline: exception: %s\n", ex.what());
        return 1;
    }
}
