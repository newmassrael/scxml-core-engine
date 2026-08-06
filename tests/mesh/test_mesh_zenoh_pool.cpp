// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE-VERIFIES: mesh-14.4
//
// SCE Mesh §14.4 Zenoh open pool on the `<send>` path, end to end.
//
// §14.4 says a pool placeholder is "resolved at `<send>` / `<invoke>`
// time". `mesh_pool_zenoh_runtime_verification` covers the `<invoke>`
// half; this covers the `<send>` half, and the two halves fail very
// differently. An `<invoke>` that addresses the wrong KeyExpr reaches
// no queryable and eventually surfaces as `error.invoke`. A `<send>`
// is a Zenoh put: no peer subscribes to an unsubstituted key, put
// reports nothing, and the event is gone with no trace anywhere in the
// process. That is the failure this file exists to make visible.
//
// What is asserted, and why each half is needed:
//
//   §1 A `<send>` with `playerId == "42"` puts on
//      `sce/zenoh_pool/player/42`. The observer is a WILDCARD
//      subscriber, not one bound to the expected key: a subscriber on
//      the expected key alone reports "nothing arrived" for every
//      wrong address equally, whereas the wildcard reports which
//      address the send actually took. An unsubstituted key names
//      itself in the failure message.
//
//   §2 Re-entering the SAME `<send>` site after `<assign>`-ing
//      `playerId` to "7" puts on `sce/zenoh_pool/player/7`. §1 alone
//      is satisfied by a generator that baked one substituted key in
//      at build time; one emitted call site producing two addresses
//      cannot be a constant.
//
//   §3 A `<send>` carrying no `id` `<param>` puts nothing at all.
//      There is no value to substitute, so the only address available
//      is one no peer subscribes to — the fail-closed half, and the
//      reason the assertion is "the wildcard saw nothing" rather than
//      "the expected key saw nothing".
//
// The whole chain runs through the engine rather than through
// `route_send` directly: onentry → `<param>` evaluation against the
// Lua datamodel → `buildEventDataJson` → the mesh send callback →
// `route_send` → the pool arm. A test that hand-builds the envelope
// proves the arm but leaves the segment above it unproven, and that
// segment is where the placeholder value actually comes from.

#include "pool_sender_zenoh_sm.h"
#include "pool_sender_zenoh_transport.h"

#include "ZenohTestUtils.h"
#include "common/TestScriptEngine.h"

#include <algorithm>
#include <chrono>
#include <cstdio>
#include <mutex>
#include <string>
#include <thread>
#include <vector>

namespace {

using namespace SCE::Test::Mesh;
using namespace std::chrono_literals;

namespace sender_gen = SCE::Generated::pool_sender_zenoh;
using Sender = sender_gen::pool_sender_zenoh;

// Mirrors the sender's connect endpoint (deploy_zenoh_pool.yaml): the
// observer binds it so the generated init() dials an endpoint that is
// already accepting.
constexpr const char *kListen = sender_gen::ZENOH_CONNECT_ENDPOINTS[0];

// Every address the binding's `sce/zenoh_pool/player/{id}` can take,
// including the unsubstituted one — `{id}` is an ordinary key chunk to
// Zenoh, so `**` matches it and a regression names itself.
constexpr const char *kWildcard = "sce/zenoh_pool/player/**";

// Spelled out rather than derived from the binding, so a change to the
// substitution rule surfaces here as a red test instead of as two
// halves that agree with each other and with nothing else.
constexpr const char *kKeyFirst = "sce/zenoh_pool/player/42";
constexpr const char *kKeyRotated = "sce/zenoh_pool/player/7";

/// Every key expression the wildcard subscriber saw, in arrival order.
struct WireLog {
    std::mutex m;
    std::vector<std::string> keys;

    void push(std::string key) {
        std::lock_guard<std::mutex> lk(m);
        keys.push_back(std::move(key));
    }

    std::size_t size() {
        std::lock_guard<std::mutex> lk(m);
        return keys.size();
    }

    bool contains(const std::string &key) {
        std::lock_guard<std::mutex> lk(m);
        return std::find(keys.begin(), keys.end(), key) != keys.end();
    }

    /// Renders what did arrive, so a failure reports the address the
    /// send actually took instead of only that the expected one was
    /// missing.
    std::string describe() {
        std::lock_guard<std::mutex> lk(m);
        if (keys.empty()) {
            return "<nothing on the wire>";
        }
        std::string out;
        for (const auto &k : keys) {
            if (!out.empty()) {
                out += ", ";
            }
            out += k;
        }
        return out;
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
    // Observer first: the sender's init() dials this endpoint.
    auto observer_session = open_peer(/*connect=*/"", /*listen=*/kListen);

    WireLog wire;
    auto observer = observer_session.declare_subscriber(
        zenoh::KeyExpr(kWildcard),
        [&wire](const zenoh::Sample &sample) { wire.push(std::string(sample.get_keyexpr().as_string_view())); },
        []() noexcept {});

    Sender sender;
    SCE::Test::inject_build_engine(sender);
    sender.initialize();

    sender_gen::TransportRouter<Sender> router({&sender});
    MESH_TEST_REQUIRE(router.init(), "sender router init failed");

    // Deterministic peer-mesh convergence: a throwaway peer dials the
    // same endpoint the router dialed from inside init(), so once
    // observer <-> probe has converged the observer <-> router edge has
    // too. Without it a put can be issued before routing state has
    // propagated and is dropped with no error — the same silent loss
    // this test is written to detect, arriving from the wrong cause.
    auto probe_session = open_peer(/*connect=*/kListen, /*listen=*/"");
    wait_for_peer_ready(observer_session, probe_session);

    // ── §1 the runtime `<param>` value becomes the address ────
    sender.processEvent(Sender::Event::Ping_send);
    MESH_TEST_REQUIRE(wait_for([&] { return wire.size() >= 1; }),
                      "a <send> to an open pool binding put nothing on the wire");
    MESH_TEST_REQUIRE(wire.contains(kKeyFirst),
                      ("a <send> to the open pool did not reach sce/zenoh_pool/player/42 — the `{id}` "
                       "placeholder was not resolved from the <param> value at <send> time. Observed: " +
                       wire.describe())
                          .c_str());

    // ── §2 the same site, a different runtime value ───────────
    sender.processEvent(Sender::Event::Ping_reset);
    sender.processEvent(Sender::Event::Ping_rotate);
    sender.processEvent(Sender::Event::Ping_send);
    MESH_TEST_REQUIRE(wait_for([&] { return wire.size() >= 2; }), "the second <send> put nothing on the wire");
    MESH_TEST_REQUIRE(wire.contains(kKeyRotated),
                      ("re-entering the same <send> site with playerId == '7' did not reach "
                       "sce/zenoh_pool/player/7 — the address is fixed rather than assembled per send. "
                       "Observed: " +
                       wire.describe())
                          .c_str());

    // ── §3 no placeholder value: nothing reaches the wire ─────
    const std::size_t before_forget = wire.size();
    sender.processEvent(Sender::Event::Ping_reset);
    sender.processEvent(Sender::Event::Ping_forget);
    // A negative assertion needs a settle window; anything this send
    // put would arrive well inside it, since §1 and §2 arrived over the
    // same converged link.
    std::this_thread::sleep_for(300ms);
    MESH_TEST_REQUIRE(wire.size() == before_forget,
                      ("a <send> carrying no `id` <param> still reached the wire — an open pool with no "
                       "value to substitute has no address to send to. Observed: " +
                       wire.describe())
                          .c_str());

    router.shutdown();
    // `~TransportRouter` runs shutdown() again on every properly torn
    // down router, so the second pass is the ordinary path.
    router.shutdown();

    std::printf("SCE Mesh §14.4 Zenoh pool <send> runtime verification: PASS\n");
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
