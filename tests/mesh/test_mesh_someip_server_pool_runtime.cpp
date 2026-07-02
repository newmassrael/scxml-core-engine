// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §14.4 server-side multi-instance pool runtime dispatch
// verification.
//
// Companion to test_mesh_someip_server_pool_compile.cpp. The compile test
// only proves the template renders + static_asserts on the generated array
// shape. This test proves the *behavioral* contract of the §14.4 pool:
//
//   * `init()` issues one offer_service per declared instance (1 and 2).
//   * vsomeip dispatches inbound RPC messages to the per-(service,
//     instance, method) handler that `init()` registered, with the
//     handler's captured `server_instance` matching the wire instance.
//   * `session_index_for_instance(server_instance)` resolves to the
//     correct sessions_ slot.
//   * `dispatchToSession(env, session_idx)` routes the envelope to the
//     intended SCXML session and NOT to any sibling.
//
// Without this test the §14.4 codegen surface (sessions_ array,
// session_index_for_instance, per-instance handler lambdas) is reachable
// only via static_assert — a regression that wired both instances to
// sessions_[0], or that mis-captured `server_instance` in the handler
// closure, would compile clean and slide through all 48 mesh ctests.
//
// Test shape (transport-level, no engine SM):
//   * Two TestSenderEngine slots stand in for sessions_[0] and sessions_[1].
//   * Raw vsomeip client (no generated client SCXML — keeps the fixture
//     focused on server-side dispatch) requests motor_service on both
//     declared instances.
//   * After both instances become available, send one request to each
//     instance and assert the engine that received it matches the
//     instance-to-session map declared in motor_pool_transport.h.
//
// Engine-driven coverage (full SCXML round-trip with reply correlation
// per instance) is a separate scope — this fixture closes the dispatch
// proof that §14.4 server pool previously left at compile-only.
//
// Mutation guard (manual procedure): the per-instance dispatch is
// load-bearing iff this test fails when the codegen template
// `tools/codegen/templates/mesh/cpp/mesh_transport.h.jinja2` is
// mutated to short-circuit the instance map. Three mutations to try:
//   (a) Hard-code `session_idx = 0` instead of calling
//       `session_index_for_instance(server_instance)` — the second
//       wait_for (slot_b receiving the instance-2 request) must time
//       out at 5s.
//   (b) Capture a constant `0x0001` in the handler lambda instead of
//       `server_instance` — same failure mode, slot_b never observed.
//   (c) Comment out the per-instance loop body (offer_service +
//       register_message_handler for one of the instances) — the
//       availability handshake for the un-offered instance must time
//       out at 10s.
// Revert after verification.

#include "motor_pool_sm.h"
#include "motor_pool_transport.h"

#include "MeshTestUtils.h"
#include "SomeipTestUtils.h"
#include "mesh/MeshEnvelopeCodec.h"

#include <vsomeip/vsomeip.hpp>

#include <atomic>
#include <chrono>
#include <condition_variable>
#include <cstdio>
#include <mutex>
#include <thread>

namespace {

using namespace SCE::Test::Mesh;

int run_test() {
    namespace gen = SCE::Generated::motor_pool;
    using PK = SCE::Mesh::PatternKind;
    using RouterT = gen::TransportRouter<TestSenderEngine>;

    wipe_stale_vsomeip_sockets();

    // Two slots — one per declared pool instance. The router's ctor
    // brace-init binds sessions_[0] = slot_a (instance 1) and
    // sessions_[1] = slot_b (instance 2) per the
    // SOMEIP_SERVER_INSTANCES array order. session_index_for_instance
    // must resolve in lockstep with this binding for the assertions
    // below to mean what they claim.
    TestSenderEngine slot_a, slot_b;
    RouterT router({&slot_a, &slot_b});
    MESH_TEST_REQUIRE(router.init(), "motor_pool router init failed (offer_service for "
                                     "instance 1 and 2 + register_message_handler must "
                                     "complete before client requests)");

    // Raw vsomeip client — bypassing the generated client path keeps the
    // fixture focused on the server-side dispatch chain. The client's
    // application name MUST match an entry in vsomeip_e2e_motor_pool.json
    // so the routing manager assigns the configured id (0x1111).
    auto client = vsomeip::runtime::get()->create_application("motor_pool_e2e_client");
    MESH_TEST_REQUIRE(client->init(), "vsomeip client init failed");

    std::mutex availability_m;
    std::condition_variable availability_cv;
    bool inst1_avail = false;
    bool inst2_avail = false;
    client->register_availability_handler(gen::SOMEIP_SERVER_SERVICE, gen::SOMEIP_SERVER_INSTANCES[0],
                                          [&](vsomeip::service_t, vsomeip::instance_t, bool is_available) {
                                              if (!is_available) {
                                                  return;
                                              }
                                              std::lock_guard<std::mutex> lock(availability_m);
                                              inst1_avail = true;
                                              availability_cv.notify_all();
                                          });
    client->register_availability_handler(gen::SOMEIP_SERVER_SERVICE, gen::SOMEIP_SERVER_INSTANCES[1],
                                          [&](vsomeip::service_t, vsomeip::instance_t, bool is_available) {
                                              if (!is_available) {
                                                  return;
                                              }
                                              std::lock_guard<std::mutex> lock(availability_m);
                                              inst2_avail = true;
                                              availability_cv.notify_all();
                                          });

    client->request_service(gen::SOMEIP_SERVER_SERVICE, gen::SOMEIP_SERVER_INSTANCES[0]);
    client->request_service(gen::SOMEIP_SERVER_SERVICE, gen::SOMEIP_SERVER_INSTANCES[1]);

    std::thread client_thread([&] { client->start(); });

    {
        std::unique_lock<std::mutex> lock(availability_m);
        MESH_TEST_REQUIRE(
            availability_cv.wait_for(lock, std::chrono::seconds(10), [&] { return inst1_avail && inst2_avail; }),
            "both motor_service instances did not become available within "
            "10s — server-side pool offer_service did not reach the routing "
            "manager for both instances (§14.4 init() loop regression)");
    }

    // Helper: build a CBOR-encoded RpcRequest envelope and wrap it in a
    // vsomeip request message bound to the given instance.
    auto build_request = [&](vsomeip::instance_t instance) {
        SCE::Mesh::MeshEnvelope env;
        env.id = SCE::uuid::v7();
        env.source = "motor_pool_e2e_client";
        env.type = "service.request.compute";
        env.pattern = PK::RpcRequest;
        env.datacontenttype = SCE::Mesh::PayloadCodec::None;
        env.invoke_id = SCE::uuid::v7();
        auto bytes = SCE::Mesh::encodeEnvelope(env);

        auto req = vsomeip::runtime::get()->create_request();
        req->set_service(gen::SOMEIP_SERVER_SERVICE);
        req->set_instance(instance);
        req->set_method(gen::SOMEIP_SERVER_METHOD_SERVICE_REQUEST_COMPUTE);
        auto payload = vsomeip::runtime::get()->create_payload();
        payload->set_data(bytes.data(), bytes.size());
        req->set_payload(payload);
        return req;
    };

    // ── §14.4 invariant 1: instance 1 dispatches to sessions_[0] only ──
    client->send(build_request(gen::SOMEIP_SERVER_INSTANCES[0]));
    MESH_TEST_REQUIRE(slot_a.received_.wait_for(
                          [](const auto &v) { return !v.empty() && v.back().type == "service.request.compute"; }),
                      "sessions_[0] did not receive the instance-1 request — "
                      "session_index_for_instance(1) → 0 dispatch chain regression");
    {
        std::lock_guard<std::mutex> lk(slot_b.received_.m);
        MESH_TEST_REQUIRE(slot_b.received_.events.empty(), "sessions_[1] received instance-1 traffic — "
                                                           "cross-session leak; handler lambda may have "
                                                           "captured the wrong server_instance");
    }

    // ── §14.4 invariant 2: instance 2 dispatches to sessions_[1] only ──
    client->send(build_request(gen::SOMEIP_SERVER_INSTANCES[1]));
    MESH_TEST_REQUIRE(slot_b.received_.wait_for(
                          [](const auto &v) { return !v.empty() && v.back().type == "service.request.compute"; }),
                      "sessions_[1] did not receive the instance-2 request — "
                      "session_index_for_instance(2) → 1 dispatch chain regression");
    {
        std::lock_guard<std::mutex> lk(slot_a.received_.m);
        MESH_TEST_REQUIRE(slot_a.received_.events.size() == 1, "sessions_[0] received instance-2 traffic — "
                                                               "cross-session leak");
    }

    // ── §14.4 invariant 3: per-instance request stash is independent ──
    // pending_server_requests_ stores one entry per inbound RPC; two
    // instances handled means two entries with distinct correlation_ids
    // (env.invoke_id was unique per request).
    {
        std::lock_guard<std::mutex> lock(router.server_pending_mutex_);
        MESH_TEST_REQUIRE(router.pending_server_requests_.size() == 2,
                          "pool router must hold one pending server request "
                          "per instance — pending_server_requests_ keying "
                          "must not alias across pool sessions");
    }

    client->stop();
    if (client_thread.joinable()) {
        client_thread.join();
    }
    router.shutdown();
    std::printf("SCE Mesh §14.4 server pool runtime: PASS\n");
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
