// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE-VERIFIES: mesh-14.4
//
// SCE Mesh §14.4 SOME/IP bounded pool on the `<send>` path, end to end.
//
// §14.4 resolves a pool placeholder at `<send>` and `<invoke>` time
// alike, and the SOME/IP `<send>` leg fails worse than any other pool
// path. An unresolved KeyExpr has no subscriber and the sample is
// dropped; an unresolved SOME/IP instance id is not absent, it is the
// binding's declared default — so the request is delivered, in full, to
// a member the author did not address. Neither end reports anything.
//
// The observer is a raw vsomeip application offering the service on
// BOTH declared instances with one message handler per (service,
// instance, method), so "which member received it" is read directly
// rather than inferred. A generated router cannot stand in: it binds
// one instance, and the entire claim here is about telling two apart.
//
// What is asserted, and why each half is needed:
//
//   §1 A `<send>` with `unitId == 1` reaches instance 1 and NOT
//      instance 2. The negative half is what makes it a selection
//      test — a router hardwired to the binding default satisfies the
//      positive half alone, which is precisely the defect.
//
//   §2 Re-entering the SAME `<send>` site after `<assign>`-ing
//      `unitId` to 2 reaches instance 2, and instance 1's count does
//      not move. §1 alone is satisfied by a fixed id that happens to
//      equal the first value; one emitted call site addressing two
//      members cannot be a constant.
//
//   §3 A value outside the declared `instances:` set reaches neither.
//      This is the fail-closed half of "bounded": init() issued
//      request_service only for the declared ids, so vsomeip would
//      discard an undeclared one inside its own routing with no error
//      surface at all — the send has to refuse before that.
//
// The chain runs through the engine rather than through `route_send`
// directly: onentry → `<param>` evaluated against the Lua datamodel →
// buildEventDataJson → the mesh send callback → route_send → the pool
// arm. A test that hand-builds the envelope proves the arm but leaves
// the segment above it — the one the placeholder value comes from —
// unproven.

#include "pool_sender_someip_sm.h"
#include "pool_sender_someip_transport.h"

#include "MeshTestUtils.h"
#include "SomeipTestUtils.h"
#include "common/TestScriptEngine.h"
#include "mesh/MeshEnvelopeCodec.h"

#include <vsomeip/vsomeip.hpp>

#include <atomic>
#include <chrono>
#include <condition_variable>
#include <cstdio>
#include <mutex>
#include <string>
#include <thread>

namespace {

using namespace SCE::Test::Mesh;
using namespace std::chrono_literals;

namespace sender_gen = SCE::Generated::pool_sender_someip;
using Sender = sender_gen::pool_sender_someip;

// Must match deploy_someip_pool.yaml `instances:`. Spelled out rather
// than derived from the generated header so a change to the declared
// set surfaces here as a red test instead of as two halves that agree
// with each other and with nothing else.
constexpr vsomeip::instance_t kInstanceFirst = 0x0001;
constexpr vsomeip::instance_t kInstanceRotated = 0x0002;

/// One pool member's arrival counter.
struct MemberObserver {
    std::atomic<int> received{0};
    std::mutex m;
    std::string last_type;
};

/// Stops and joins the harness application on every exit path.
///
/// `MESH_TEST_REQUIRE` returns straight out of the enclosing function,
/// so a joinable `std::thread` left in scope turns a legible assertion
/// failure into `terminate called without an active exception` — the
/// diagnosis this file exists to produce, destroyed on its way out.
struct HarnessRunner {
    std::shared_ptr<vsomeip::application> app;
    std::thread t;

    explicit HarnessRunner(std::shared_ptr<vsomeip::application> a) : app(std::move(a)), t([this] { app->start(); }) {}

    ~HarnessRunner() {
        app->stop();
        if (t.joinable()) {
            t.join();
        }
    }
};

/// Wait until `p` holds or the shared mesh timeout runs out.
template <typename Predicate> bool wait_for(Predicate p) {
    const auto deadline = std::chrono::steady_clock::now() + kDefaultTimeout;
    while (std::chrono::steady_clock::now() < deadline) {
        if (p()) {
            return true;
        }
        std::this_thread::sleep_for(5ms);
    }
    return false;
}

int run_test() {
    wipe_stale_vsomeip_sockets();

    // ── Harness first: it is the routing manager. ──────────────
    auto harness = vsomeip::runtime::get()->create_application("pool_send_harness");
    MESH_TEST_REQUIRE(harness->init(), "vsomeip harness init failed");

    MemberObserver first;
    MemberObserver rotated;

    auto observe = [](MemberObserver &member) {
        return [&member](const std::shared_ptr<vsomeip::message> &msg) {
            auto pl = msg->get_payload();
            if (!pl) {
                return;
            }
            SCE::Mesh::MeshEnvelope env;
            if (!SCE::Mesh::decodeEnvelope(pl->get_data(), pl->get_length(), env)) {
                return;
            }
            {
                std::lock_guard<std::mutex> lk(member.m);
                member.last_type = env.type;
            }
            member.received.fetch_add(1);
        };
    };

    harness->register_message_handler(sender_gen::SOMEIP_SERVICE_CONTROLLER_POOL, kInstanceFirst,
                                      sender_gen::SOMEIP_METHOD_CONTROLLER_POOL_SERVICE_FIRE_FORGET_PING,
                                      observe(first));
    harness->register_message_handler(sender_gen::SOMEIP_SERVICE_CONTROLLER_POOL, kInstanceRotated,
                                      sender_gen::SOMEIP_METHOD_CONTROLLER_POOL_SERVICE_FIRE_FORGET_PING,
                                      observe(rotated));
    harness->offer_service(sender_gen::SOMEIP_SERVICE_CONTROLLER_POOL, kInstanceFirst);
    harness->offer_service(sender_gen::SOMEIP_SERVICE_CONTROLLER_POOL, kInstanceRotated);

    HarnessRunner harness_runner(harness);

    // ── Sender: engine + generated router. ─────────────────────
    Sender sender;
    SCE::Test::inject_build_engine(sender);
    sender.initialize();

    sender_gen::TransportRouter<Sender> router({&sender});
    MESH_TEST_REQUIRE(router.init(), "sender router init failed — request_service for the declared "
                                     "pool instances did not complete");

    // Both members must be available before the first send: vsomeip
    // discards a request for a service it has not yet resolved, which
    // would produce the same "nothing arrived" signal the pool defect
    // produces and make the test lie about its cause.
    {
        std::mutex availability_m;
        std::condition_variable availability_cv;
        bool first_avail = false;
        bool rotated_avail = false;
        router.controller_pool_app_->register_availability_handler(
            sender_gen::SOMEIP_SERVICE_CONTROLLER_POOL, kInstanceFirst,
            [&](vsomeip::service_t, vsomeip::instance_t, bool is_available) {
                if (!is_available) {
                    return;
                }
                std::lock_guard<std::mutex> lk(availability_m);
                first_avail = true;
                availability_cv.notify_all();
            });
        router.controller_pool_app_->register_availability_handler(
            sender_gen::SOMEIP_SERVICE_CONTROLLER_POOL, kInstanceRotated,
            [&](vsomeip::service_t, vsomeip::instance_t, bool is_available) {
                if (!is_available) {
                    return;
                }
                std::lock_guard<std::mutex> lk(availability_m);
                rotated_avail = true;
                availability_cv.notify_all();
            });
        std::unique_lock<std::mutex> lk(availability_m);
        MESH_TEST_REQUIRE(availability_cv.wait_for(lk, 10s, [&] { return first_avail && rotated_avail; }),
                          "both declared pool instances did not become available within 10s — the "
                          "init() request_service loop did not reach the routing manager for both");
    }

    // ── §1 the runtime `<param>` value selects instance 1 ──────
    sender.processEvent(Sender::Event::Ping_send);
    MESH_TEST_REQUIRE(wait_for([&] { return first.received.load() > 0; }),
                      "a <send> with inst == 1 never reached pool instance 1");
    {
        std::lock_guard<std::mutex> lk(first.m);
        MESH_TEST_REQUIRE(first.last_type == "service.fire_forget.ping",
                          "pool instance 1 received an unexpected event type");
    }
    MESH_TEST_REQUIRE(rotated.received.load() == 0, "a <send> addressed to instance 1 also reached instance 2");

    // ── §2 the same site, a different runtime value ────────────
    const int first_after_send = first.received.load();
    sender.processEvent(Sender::Event::Ping_reset);
    sender.processEvent(Sender::Event::Ping_rotate);
    sender.processEvent(Sender::Event::Ping_send);
    MESH_TEST_REQUIRE(wait_for([&] { return rotated.received.load() > 0; }),
                      "re-entering the same <send> site with inst == 2 never reached pool instance 2 — "
                      "the instance id is fixed at the binding default rather than read from the "
                      "<param> value at <send> time");
    MESH_TEST_REQUIRE(first.received.load() == first_after_send,
                      "a <send> addressed to instance 2 also reached instance 1 — dispatch is not "
                      "keyed on the runtime member value");

    // ── §3 an undeclared member is refused, and sends nothing ──
    const int first_before_stray = first.received.load();
    const int rotated_before_stray = rotated.received.load();
    sender.processEvent(Sender::Event::Ping_reset);
    sender.processEvent(Sender::Event::Ping_stray);
    sender.processEvent(Sender::Event::Ping_send);
    // A negative assertion needs a settle window; anything this send
    // put on the wire would arrive well inside it, since §1 and §2
    // arrived over the same already-resolved service.
    std::this_thread::sleep_for(500ms);
    MESH_TEST_REQUIRE(first.received.load() == first_before_stray && rotated.received.load() == rotated_before_stray,
                      "a <send> naming a member outside the declared `instances:` set still reached a "
                      "pool member — a bounded pool that resolves an unknown member has no bound");

    router.shutdown();
    // `~TransportRouter` runs shutdown() again on every properly torn
    // down router, so the second pass is the ordinary path.
    router.shutdown();

    std::printf("SCE Mesh §14.4 SOME/IP pool <send> runtime verification: PASS\n");
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
