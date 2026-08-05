// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §9.5 server response deadline, SOME/IP arm.
//
// The zenoh arm (test_mesh_zenoh_server_timeout.cpp) can only assert that
// the server released its own state: a `zenoh::Query` has no
// server-authored failure channel, so expiry is a silent drop and the
// client infers `RpcStatus::Unavailable` from the drop. SOME/IP does have
// such a channel — MT_ERROR (0x81) with return code E_TIMEOUT (0x06) —
// and this file asserts SCE uses it, which is the axis on which the
// implementation leads its reference: neither vsomeip nor `ara::com`
// arms a server-side deadline at all, so in both the server's request
// handle stays parked forever and the client cannot distinguish "the
// server gave up" from "the datagram was lost".
//
// Four scenarios, one binary:
//
//   §1 Leak closure — motor receives an inbound RpcRequest but its
//      recorder engine never emits the paired response. The stashed
//      `pending_server_requests_` entry must be released within
//      `response_deadline_ms + slack`.
//
//   §2 Active notice — the requesting peer observes the timeout as an
//      event, not as silence. Brake's generated client binding must
//      accept the MT_ERROR (the pre-existing message-type gate dropped
//      anything that was not MT_RESPONSE) and must NOT rename it to the
//      declared reply event: a failure renamed to
//      `service.response.compute_force` would report a false success on
//      an empty payload. Brake therefore observes `error.rpc.deadline`,
//      whose `_event.data` names the abandoned call.
//
//   §3 Happy-path cancel — the test calls `handleServerResponse` before
//      the deadline elapses. `cancelDeadline` must run before the map
//      erase so the timer cannot fire on a valid entry and answer
//      MT_ERROR for a request that was answered normally.
//
//   §4 Shutdown while in flight — a request is left pending and
//      `shutdown()` runs before the deadline. The scheduler drain plus
//      `server_shutdown_in_progress_` must suppress the callback; in
//      particular the drain has to precede `server_app_->stop()`, or a
//      late timer would write MT_ERROR into a stopped application.
//
// Both routers bind recorder engines (`TestSenderEngine`). The engine is
// deliberately not the system under test here — what is under test is
// the transport-level lifecycle, and a recorder is exactly the "engine
// that never answers" the deadline exists for.

#include "brake_someip_server_deadline_sm.h"
#include "brake_someip_server_deadline_transport.h"
#include "motor_someip_server_deadline_sm.h"
#include "motor_someip_server_deadline_transport.h"

#include "MeshTestUtils.h"
#include "SomeipTestUtils.h"
#include "common/Uuid.h"
#include "mesh/MeshEnvelopeCodec.h"

#include <chrono>
#include <condition_variable>
#include <cstdio>
#include <mutex>
#include <string>
#include <thread>

namespace {

using namespace SCE::Test::Mesh;

namespace brake_gen = SCE::Generated::brake_someip_server_deadline;
namespace motor_gen = SCE::Generated::motor_someip_server_deadline;

using PK = SCE::Mesh::PatternKind;
using BrakeRouterT = brake_gen::TransportRouter<TestSenderEngine>;
using MotorRouterT = motor_gen::TransportRouter<TestSenderEngine>;

// Must match deploy_someip_server_deadline.yaml server.response_deadline_ms.
constexpr auto kResponseDeadline = std::chrono::milliseconds(500);
// Allowance for scheduler tick + thread wakeup + map erase + the vsomeip
// round trip the MT_ERROR notice takes before brake observes it.
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
// This matters for what the test can see. `route_send(target, env)`
// takes a pre-built envelope and skips correlation registration, so a
// test driving it never populates `pending_rpcs_` and never reaches the
// client-side reply rewrite. Going through the callback is what an
// SCXML author's `<send>` does, so the rewrite — including its refusal
// to rename a failed reply into the declared success event — is on the
// path under test.
bool send_request(TestSenderEngine &brake_engine) {
    return brake_engine.mesh_send_cb_("#motor_someip_server_deadline", "service.request.compute_force",
                                      R"({"force":100})", "", "");
}

// Returns 0 on success so it composes with MESH_TEST_REQUIRE, whose
// failure arm is `return 1` — the macro is usable only inside a function
// with that return shape.
int await_service_available(BrakeRouterT &brake_router) {
    std::mutex m;
    std::condition_variable cv;
    bool available = false;
    brake_router.motor_someip_server_deadline_app_->register_availability_handler(
        brake_gen::SOMEIP_SERVICE_MOTOR_SOMEIP_SERVER_DEADLINE, brake_gen::SOMEIP_INSTANCE_MOTOR_SOMEIP_SERVER_DEADLINE,
        [&](vsomeip::service_t, vsomeip::instance_t, bool is_available) {
            if (!is_available) {
                return;
            }
            std::lock_guard<std::mutex> lock(m);
            available = true;
            cv.notify_all();
        });

    std::unique_lock<std::mutex> lock(m);
    MESH_TEST_REQUIRE(cv.wait_for(lock, std::chrono::seconds(10), [&] { return available; }),
                      "vsomeip service motor_control did not become available "
                      "within 10s (routing manager handshake stuck)");
    return 0;
}

int run_test() {
    wipe_stale_vsomeip_sockets();

    TestSenderEngine motor_engine;
    MotorRouterT motor_router({&motor_engine});
    MESH_TEST_REQUIRE(motor_router.init(), "motor router init failed");

    TestSenderEngine brake_engine;
    BrakeRouterT brake_router({&brake_engine});
    MESH_TEST_REQUIRE(brake_router.init(), "brake router init failed");

    if (const int rc = await_service_available(brake_router); rc != 0) {
        return rc;
    }

    // ── §1 + §2: leak closure and the active notice ───────────────────
    //
    // One request drives both assertions: the server-side release and
    // the client-side observation are two ends of the same expiry, and
    // splitting them across requests would let one pass on the other's
    // envelope.
    {
        MESH_TEST_REQUIRE(motor_router.pending_server_size() == 0, "initial pending_server_size should be zero");
        brake_engine.received_.clear();

        MESH_TEST_REQUIRE(send_request(brake_engine), "brake mesh send callback returned false");

        // The request must actually reach the server before the deadline
        // claim means anything — otherwise a request lost on the wire
        // would satisfy "pending drops to zero" trivially.
        MESH_TEST_REQUIRE(wait_until([&] { return motor_router.pending_server_size() == 1; }, std::chrono::seconds(5)),
                          "inbound request never reached motor's pending_server_requests_ — "
                          "cannot attribute a later empty map to the deadline");

        MESH_TEST_REQUIRE(
            wait_until([&] { return motor_router.pending_server_size() == 0; }, kResponseDeadline + kSlack),
            "stranded pending_server_requests_ entry outlived "
            "response_deadline_ms + slack — the server-side deadline did not fire");

        // §2: the notice, not just the release. Brake must surface the
        // server-authored event name — renaming it to the declared reply
        // event would be a false success, and dropping the MT_ERROR
        // outright would make the timeout indistinguishable from silence.
        MESH_TEST_REQUIRE(brake_engine.received_.wait_for(
                              [](const auto &v) { return !v.empty() && v.back().type == "error.rpc.deadline"; }),
                          "requesting peer never observed the MT_ERROR / E_TIMEOUT notice — "
                          "either the server did not send it, or the client's message-type "
                          "gate dropped it as 'not a response'");

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
                          "a timed-out request must never surface as the declared reply "
                          "event — that reports a false success on an empty payload");
    }

    // ── §3: happy-path cancel ─────────────────────────────────────────
    //
    // Answering before the deadline must both deliver the reply and
    // retire the timer. Reading `deadline_scheduler_size()` immediately
    // after the reply distinguishes "cancel worked" from "the timer
    // fired and its callback found nothing" — those are observationally
    // identical through `pending_server_size()` alone.
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
        reply.source = "motor_someip_server_deadline";
        reply.type = "service.response.compute_force";
        reply.pattern = PK::RpcReply;
        reply.correlation_id = *cid;
        reply.invoke_id = *cid;
        MESH_TEST_REQUIRE(motor_router.handleServerResponse(reply), "handleServerResponse did not correlate the reply");
        MESH_TEST_REQUIRE(motor_router.deadline_scheduler_size() == 0,
                          "cancelDeadline did not release the scheduler entry — a timer left "
                          "armed on an answered request can still emit a spurious MT_ERROR");
        MESH_TEST_REQUIRE(motor_router.pending_server_size() == 0, "answered request must leave the pending map empty");

        MESH_TEST_REQUIRE(brake_engine.received_.wait_for([](const auto &v) {
            return !v.empty() && v.back().type == "service.response.compute_force";
        }),
                          "normal reply not delivered on the happy path");

        // Nothing may follow it: the cancelled timer must not fire later
        // and answer MT_ERROR for a request already answered.
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
    // The scheduler drain runs before `server_app_->stop()`, so the last
    // callback has returned by the time the application goes down. A
    // clean return from shutdown() with a pending entry is the assertion:
    // the failure mode is a hang or a crash inside vsomeip, not a wrong
    // value.
    {
        brake_engine.received_.clear();
        MESH_TEST_REQUIRE(send_request(brake_engine), "brake mesh send callback (shutdown path) returned false");
        MESH_TEST_REQUIRE(wait_until([&] { return motor_router.pending_server_size() == 1; }, std::chrono::seconds(5)),
                          "inbound request never reached motor (shutdown path)");
        motor_router.shutdown();
    }

    brake_router.shutdown();
    std::printf("SCE Mesh §9.5 SOME/IP server response deadline: PASS\n");
    return 0;
}

}  // namespace

int main() {
    try {
        return run_test();
    } catch (const std::exception &ex) {
        std::fprintf(stderr, "SCE Mesh SOME/IP server deadline: exception: %s\n", ex.what());
        return 1;
    }
}
