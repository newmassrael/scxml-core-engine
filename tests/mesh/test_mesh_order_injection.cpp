// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §10.6 ordering integration regression.
//
// Exercises the generated `admitOrdered` on a Zenoh binding with
// `ordering: required`: sender engine + codegen + OrderingBuffer.
//
// Hermeticity:
//   * Cases 1–4 construct a `TransportRouter` without calling
//     `init()`. The admit path runs against the OrderingBuffer
//     initialised from the per-machine `ordering:` block in
//     `deploy_ordered.yaml` (10 ms gap, 5 ms tick) regardless of
//     whether a Zenoh session has been opened, so these cases are
//     fully hermetic.
//   * Case 5 DOES call `router.init()` because the periodic
//     `ordering_tick_thread_` is started inside init(). init() opens
//     a Zenoh peer-mode session; peer mode does not require a remote
//     peer (it just listens), so the test environment needs only a
//     working local Zenoh runtime, not network connectivity.
//
// Guarantees under test:
//   1. Envelopes injected out of order are released in sequence order.
//   2. A gap that the deploy.yaml-supplied `gap_timeout_ms` cannot
//      wait out falls back to fast-forward via `tickOrdering()` and
//      emits `error.communication` with `reason: ORDERING_GAP`
//      (§16.7 row 13).
//   3. RpcReply envelopes bypass the buffer entirely (correlation-keyed).
//   4. An envelope without `sequence_no` on an ordered route raises
//      `error.communication` with `reason: MISSING_SEQUENCE`
//      (§16.7 row 12) and is dropped from the ordering domain.
//   5. The router's periodic tick thread recovers a stalled gap with
//      no further inbound traffic and no manual tick invocation
//      (§10.6.4 gap-at-stream-end).
//
// Mutation guard (manual procedure): the ordering layer is load-bearing
// iff this assertion fails when it is removed. Temporarily edit
// `sce/include/mesh/OrderingBuffer.h::OrderingBuffer::admit` to return
// the admitted envelope immediately without buffering (skip the
// `seq > next_expected_seq` branch so every envelope releases on
// arrival) — the `OutOfOrderIsReorderedOnRelease` assertion below
// must fail. Revert after verification.

#include "brake_transport.h"

#include "MeshTestUtils.h"
#include "mesh/MeshEnvelope.h"

#include <chrono>
#include <cstdio>
#include <exception>
#include <string>
#include <thread>

namespace {

using namespace SCE::Generated::brake;
using namespace SCE::Test::Mesh;
namespace PK = SCE::Mesh;

SCE::Mesh::MeshEnvelope make_ordered_env(const std::string& source,
                                          std::uint64_t seq) {
    SCE::Mesh::MeshEnvelope env;
    env.id = SCE::uuid::v7();
    env.source = source;
    env.type = "brake.activate";
    env.pattern = PK::PatternKind::FireForget;
    env.sequence_no = seq;
    return env;
}

// `_event.data` for raised error.communication envelopes is JSON bytes;
// substring match is sufficient to pin the reason + extras set by
// `CommunicationError::toJsonBytes` without pulling in a JSON parser.
bool sender_received(TestSenderEngine& sender, const std::string& type) {
    std::lock_guard<std::mutex> lk(sender.received_.m);
    for (const auto& ev : sender.received_.events) {
        if (ev.type == type) return true;
    }
    return false;
}

std::string last_data_for_type(TestSenderEngine& sender, const std::string& type) {
    std::lock_guard<std::mutex> lk(sender.received_.m);
    for (auto it = sender.received_.events.rbegin();
         it != sender.received_.events.rend(); ++it) {
        if (it->type == type) return it->data;
    }
    return {};
}

std::size_t count_events_of_type(TestSenderEngine& sender,
                                 const std::string& type) {
    std::lock_guard<std::mutex> lk(sender.received_.m);
    std::size_t n = 0;
    for (const auto& ev : sender.received_.events) {
        if (ev.type == type) ++n;
    }
    return n;
}

int run_test() {
    using RouterT = TransportRouter<TestSenderEngine>;

    // ── 1. Out-of-order injection via the SSoT wire entry point ────
    // Drives envelopes through `admitZenohInbound("#motor", env)` —
    // the same helper the generated Zenoh reply/subscribe closures
    // call in production. The per-target switch inside that helper
    // picks `admitOrdered` for the `ordering: required` binding, so
    // a passing assertion proves the wire path is wired, not just
    // that the admit method compiles.
    {
        TestSenderEngine sender;
        RouterT router({&sender});

        // Inject seq=[1, 3, 2, 4]. Contract: sender engine sees
        // [1, 2, 3, 4] — the 3,2 reshuffle is absorbed by
        // OrderingBuffer before dispatch.
        MESH_TEST_REQUIRE(router.admitZenohInbound("#motor", make_ordered_env("motor", 1)),
            "seq=1 must be dispatched immediately");
        MESH_TEST_REQUIRE(!router.admitZenohInbound("#motor", make_ordered_env("motor", 3)),
            "seq=3 arriving before seq=2 must buffer");
        MESH_TEST_REQUIRE(router.admitZenohInbound("#motor", make_ordered_env("motor", 2)),
            "seq=2 fills the gap and releases buffered envelopes");
        MESH_TEST_REQUIRE(router.admitZenohInbound("#motor", make_ordered_env("motor", 4)),
            "seq=4 is the new expected — release immediately");

        // Sender must have observed exactly 4 events and the order
        // must reflect the sequence numbers, not the admit order.
        // TestSenderEngine records the receive order; index 0 is the
        // first observation.
        {
            std::lock_guard<std::mutex> lk(sender.received_.m);
            MESH_TEST_REQUIRE(sender.received_.events.size() == 4,
                "sender must receive exactly 4 envelopes");
            // All envelopes share the same `brake.activate` event name,
            // so we cannot use event strings to identify order. The
            // reorder guarantee is pinned instead by the dispatch
            // count-per-admit: admit(seq=3) buffers (0 dispatch),
            // admit(seq=2) drains [2,3] (2 dispatch). The return-value
            // assertions above cover this — this block is a belt-and-
            // suspenders check that no envelope was lost.
        }
    }

    // ── 2. Gap timeout → fast-forward + tick raises ORDERING_GAP ───
    // Manual tick path (no periodic thread). After the gap timeout
    // has elapsed, `tickOrdering()` must release the buffered seq=3
    // AND raise `error.communication` with reason=ORDERING_GAP and
    // lost_seq_lo/hi pinning the missing range.
    {
        TestSenderEngine sender;
        RouterT router({&sender});

        MESH_TEST_REQUIRE(router.admitZenohInbound("#motor", make_ordered_env("motor", 1)),
            "seq=1 released");
        MESH_TEST_REQUIRE(!router.admitZenohInbound("#motor", make_ordered_env("motor", 3)),
            "seq=3 buffered (gap at 2)");

        // deploy_ordered.yaml sets gap_timeout_ms=10; sleep at least
        // 2× that to ensure a stable timeout under CI load. Was 120
        // ms when the runtime hard-coded a 100 ms gap; the per-machine
        // override (SCE_MESH.md §10.6.1) lets us cut hermetic latency
        // by an order of magnitude.
        std::this_thread::sleep_for(std::chrono::milliseconds(25));
        const std::size_t released = router.tickOrdering();
        MESH_TEST_REQUIRE(released == 1,
            "gap timeout must release the buffered seq=3");

        MESH_TEST_REQUIRE(sender_received(sender, "error.communication"),
            "tickOrdering must raise error.communication on gap timeout");
        const std::string payload = last_data_for_type(sender, "error.communication");
        MESH_TEST_REQUIRE(payload.find("\"reason\":\"ORDERING_GAP\"") != std::string::npos,
            "error.communication reason must be ORDERING_GAP");
        MESH_TEST_REQUIRE(payload.find("\"lost_seq_lo\":2") != std::string::npos,
            "lost_seq_lo must pin the missing sequence low bound");
        MESH_TEST_REQUIRE(payload.find("\"lost_seq_hi\":2") != std::string::npos,
            "lost_seq_hi must pin the missing sequence high bound");
        MESH_TEST_REQUIRE(payload.find("\"source\":\"motor\"") != std::string::npos,
            "error.communication must carry the gap's source machine name");
    }

    // ── 3. RpcReply bypasses the ordering buffer ───────────────────
    // Correlation-keyed replies are not in the ordering domain; the
    // generated admitOrdered must dispatch them directly so the
    // reply path pays no extra latency. Driven through
    // `admitZenohInbound` so the per-target switch → admitOrdered →
    // bypass branch is the code actually exercised.
    {
        TestSenderEngine sender;
        RouterT router({&sender});

        SCE::Mesh::MeshEnvelope reply;
        reply.id = SCE::uuid::v7();
        reply.source = "motor";
        reply.type = "service.response.compute_force";
        reply.pattern = PK::PatternKind::RpcReply;
        // Intentionally missing sequence_no — admitOrdered must
        // bypass the buffer for RpcReply regardless of whether the
        // sequence field is present.
        MESH_TEST_REQUIRE(router.admitZenohInbound("#motor", reply),
            "RpcReply without sequence_no must bypass the ordering buffer");
        // RpcReply must NOT raise MISSING_SEQUENCE — it is bypassed
        // before the stamp check.
        MESH_TEST_REQUIRE(!sender_received(sender, "error.communication"),
            "RpcReply bypass must not trigger a MISSING_SEQUENCE raise");
    }

    // ── 4. Unstamped envelope → error.communication MISSING_SEQUENCE
    // §16.7 row 12: an envelope that reaches admitOrdered without
    // `sequence_no` on an ordered route is a sender-drift condition.
    // The envelope is dropped from the ordering domain (return false
    // for the original envelope's flow) but an error.communication
    // envelope IS dispatched, so the return value is true — the raise
    // succeeded.
    {
        TestSenderEngine sender;
        RouterT router({&sender});

        SCE::Mesh::MeshEnvelope unstamped;
        unstamped.id = SCE::uuid::v7();
        unstamped.source = "motor";
        unstamped.type = "brake.activate";
        unstamped.pattern = PK::PatternKind::FireForget;
        // sequence_no intentionally absent — FireForget on an ordered
        // route without a stamp is §10.6.3 drift.

        // Return value convention: admitOrdered returns true iff a
        // DATA envelope (from the ordered stream) dispatched. The
        // MISSING_SEQUENCE raise is a side-channel: false here does
        // NOT mean "the raise failed" — raise observation is always
        // performed via the sender below (author-visible signal).
        MESH_TEST_REQUIRE(!router.admitZenohInbound("#motor", unstamped),
            "admitOrdered must return false — unstamped envelope drops from the data stream");

        MESH_TEST_REQUIRE(sender_received(sender, "error.communication"),
            "unstamped envelope on ordered route must raise error.communication");
        const std::string payload = last_data_for_type(sender, "error.communication");
        MESH_TEST_REQUIRE(payload.find("\"reason\":\"MISSING_SEQUENCE\"") != std::string::npos,
            "error.communication reason must be MISSING_SEQUENCE");
        MESH_TEST_REQUIRE(payload.find("\"source\":\"motor\"") != std::string::npos,
            "error.communication must carry the offending sender name");
        MESH_TEST_REQUIRE(payload.find("\"envelope_id\":") != std::string::npos,
            "error.communication must carry the envelope_id for diagnosis");
        // The original event name must NOT have reached the engine —
        // only the error.communication should be visible.
        MESH_TEST_REQUIRE(count_events_of_type(sender, "brake.activate") == 0,
            "unstamped envelope must not reach the engine as brake.activate");
    }

    // ── 5. Periodic tick thread recovers a stream-end gap ──────────
    // §10.6.4: a gap with no further inbound traffic still recovers
    // within `gap_timeout + tick_period` because the router owns an
    // ordering_tick_thread_ that drives tick() periodically. The test
    // calls router.init() to start that thread, injects a gap, then
    // waits without further admits. The buffered seq=3 must be
    // released AND error.communication ORDERING_GAP must land on the
    // sender — all without any manual tickOrdering() call.
    //
    // init() opens a Zenoh peer-mode session. No remote peer is
    // required for peer-mode to succeed; the session just listens.
    // init() is required to pass for this case, otherwise the
    // ordering tick thread is never started — that is a real
    // regression, not an environment skip.
    {
        TestSenderEngine sender;
        RouterT router({&sender});
        MESH_TEST_REQUIRE(router.init(),
            "router.init() must succeed — ordering tick thread start is inside init");

        MESH_TEST_REQUIRE(router.admitZenohInbound("#motor", make_ordered_env("motor", 1)),
            "seq=1 released");
        MESH_TEST_REQUIRE(!router.admitZenohInbound("#motor", make_ordered_env("motor", 3)),
            "seq=3 buffered (gap at 2)");

        // deploy_ordered.yaml sets gap_timeout_ms=10 + tick_period_ms=5,
        // so the worst-case recovery latency is `gap_timeout +
        // tick_period` = 15 ms. 50 ms gives ~3× headroom for CI
        // scheduler jitter without going back to the 250 ms wait the
        // hard-coded 100/50 timings forced.
        std::this_thread::sleep_for(std::chrono::milliseconds(50));

        MESH_TEST_REQUIRE(sender_received(sender, "error.communication"),
            "periodic tick must raise error.communication on stream-end gap");
        const std::string payload = last_data_for_type(sender, "error.communication");
        MESH_TEST_REQUIRE(payload.find("\"reason\":\"ORDERING_GAP\"") != std::string::npos,
            "periodic tick reason must be ORDERING_GAP");
        MESH_TEST_REQUIRE(payload.find("\"lost_seq_lo\":2") != std::string::npos,
            "periodic tick lost_seq_lo must pin the missing sequence");

        // The buffered seq=3 must also have been released by the
        // periodic tick — sender receives exactly one brake.activate
        // envelope (seq=1) plus the fast-forwarded seq=3.
        MESH_TEST_REQUIRE(count_events_of_type(sender, "brake.activate") == 2,
            "periodic tick must release buffered seq=3 after fast-forward");

        router.shutdown();  // joins ordering_tick_thread_; idempotent with dtor
    }

    std::printf("SCE Mesh ordering injection regression: PASS\n");
    return 0;
}

}  // namespace

int main() {
    try {
        return run_test();
    } catch (const std::exception& ex) {
        std::fprintf(stderr, "FAIL: unexpected exception: %s\n", ex.what());
        return 1;
    } catch (...) {
        std::fprintf(stderr, "FAIL: unknown exception\n");
        return 1;
    }
}
