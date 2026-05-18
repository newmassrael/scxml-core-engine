// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// OutboundBuffer unit tests — SCE_MESH.md §10.10 + §16.7 rows 1 + 9.
//
// Sibling of DedupRouter / OrderingBuffer unit tests in Bucket 1
// (Core primitives). Existing E2E coverage exercises the DRAIN path
// (`mesh_someip_late_boot` and `mesh_zenoh_publisher_first` verify
// that envelopes buffered while the transport is not ready survive
// and reach the peer after `markReady()` fires). This file covers
// the two raise paths the buffer owns:
//
//   * §16.7 row 9 BACKPRESSURE_DROP: `OutboundBuffer::admit` raises
//     `error.communication` with `reason="BACKPRESSURE_DROP"` and
//     drops the newest envelope when the per-target queue is full.
//   * §16.7 row 1 TRANSPORT_UNAVAILABLE: `OutboundBuffer::markNotReady`
//     raises `error.communication` with `reason="TRANSPORT_UNAVAILABLE"`
//     on the `true → false` transition (the §10.4.1 "Active →
//     Disconnected" lifecycle edge driven by SOME/IP availability=false,
//     Zenoh matching=false, etc.). The initial `ready_=false` seed
//     state does NOT emit on the first `markNotReady`: no Active phase
//     preceded it, so no transition fires.
//
// The byte-shape unit pins for the raised errors live in
// `CommunicationErrorTest::BackpressureDropShape` and
// `CommunicationErrorTest::TransportUnavailableShape`; this file
// proves each raise FIRES under the right precondition and that the
// captured fields match the catalog. The four tests together close
// the row 1 + row 9 entries of §16.7 at the same E2E + byte-shape
// ratification level rows 6 / 8 / 11 / 12 / 13 already enjoy.

#include "mesh/CommunicationError.h"
#include "mesh/MeshEnvelope.h"
#include "mesh/OutboundBuffer.h"

#include <gtest/gtest.h>

#include <optional>
#include <string>

using SCE::Mesh::CommunicationError;
using SCE::Mesh::MeshEnvelope;
using SCE::Mesh::OutboundBuffer;

namespace {

// Capture shim for the OutboundBuffer::ErrorRaise callback. The raise
// fires OUTSIDE the buffer's internal mutex (§10.10 thread-safety
// note), so storing the captured error in a plain optional is safe
// in a single-threaded test.
struct ErrorSink {
    std::optional<CommunicationError> last;
    int call_count = 0;

    void operator()(CommunicationError err) {
        last = std::move(err);
        ++call_count;
    }
};

}  // namespace

TEST(OutboundBufferTest, BackpressureOverflowRaisesRow9) {
    // The buffer defaults to `ready_ = false`, so the first
    // `max_pending` admits enqueue. The `max_pending+1`-th admit
    // observes `queue_.size() >= max_pending_` and triggers the row 9
    // raise. Setting `max_pending` to 2 makes the overflow point
    // deterministic and the captured `queue_depth` predictable.
    ErrorSink sink;
    OutboundBuffer buf(
        /* target          */ "motor",
        /* max_pending     */ 2,
        /* transport_name  */ "someip",
        /* dispatch        */ [](const MeshEnvelope&) { return true; },
        /* raise_error     */ std::ref(sink));

    MeshEnvelope env{};

    EXPECT_TRUE(buf.admit(env));    // queue depth 1, accepted
    EXPECT_TRUE(buf.admit(env));    // queue depth 2 (== max_pending), accepted
    EXPECT_EQ(buf.queue_depth(), 2u);
    EXPECT_EQ(sink.call_count, 0);  // no overflow yet

    EXPECT_FALSE(buf.admit(env));   // queue_.size() >= max_pending_ → overflow + drop
    EXPECT_EQ(buf.queue_depth(), 2u) << "newest dropped — depth unchanged";

    ASSERT_EQ(sink.call_count, 1) << "exactly one raise per overflow admit";
    ASSERT_TRUE(sink.last.has_value());
    EXPECT_EQ(sink.last->reason, "BACKPRESSURE_DROP");
    ASSERT_TRUE(sink.last->target.has_value());
    EXPECT_EQ(*sink.last->target, "motor");
    ASSERT_TRUE(sink.last->transport.has_value());
    EXPECT_EQ(*sink.last->transport, "someip");
    ASSERT_TRUE(sink.last->queue_depth.has_value());
    EXPECT_EQ(*sink.last->queue_depth, 2)
        << "depth captured at the moment of overflow, before drop";
}

TEST(OutboundBufferTest, SustainedOverflowRaisesPerAdmit) {
    // §16.7 catalog scope: "Multiple conditions observed within a
    // single microstep produce multiple events (one per condition);
    // coalescing is not permitted because authors rely on
    // one-to-one condition-to-event mapping." Each overflow admit
    // must fire its own raise — a saturated sender cannot be
    // silently throttled into a single error event.
    ErrorSink sink;
    OutboundBuffer buf(
        "motor", /*max_pending*/ 1, "zenoh",
        [](const MeshEnvelope&) { return true; },
        std::ref(sink));

    MeshEnvelope env{};
    EXPECT_TRUE(buf.admit(env));    // queue depth 1
    EXPECT_FALSE(buf.admit(env));   // overflow #1
    EXPECT_FALSE(buf.admit(env));   // overflow #2
    EXPECT_FALSE(buf.admit(env));   // overflow #3

    EXPECT_EQ(sink.call_count, 3) << "one raise per overflow admit, no coalescing";
}

TEST(OutboundBufferTest, MarkNotReadyFromInitialStateDoesNotRaise) {
    // §16.7 row 1 + §10.4.1 lifecycle: the seed state is `ready_=false`
    // (a transport that has never reached "Active"). Transport callbacks
    // wired before `start()` — vsomeip `register_availability_handler`,
    // Zenoh `declare_matching_listener` — may legitimately fire the
    // false→false anchor edge as part of the Ready→Active warm-up
    // sequence. That is NOT a transport fault and must not surface as
    // `error.communication`. Only the `Active → Disconnected` transition
    // (true→false after a real markReady) raises Row 1.
    ErrorSink sink;
    OutboundBuffer buf(
        "motor", /*max_pending*/ 4, "someip",
        [](const MeshEnvelope&) { return true; },
        std::ref(sink));

    buf.markNotReady();   // initial false → false: no transition
    buf.markNotReady();   // still false → false: still no transition
    buf.markNotReady();

    EXPECT_EQ(sink.call_count, 0)
        << "row 1 fires per Active→Disconnected transition, not per callback";
    EXPECT_FALSE(buf.ready());
}

TEST(OutboundBufferTest, MarkNotReadyAfterReadyRaisesRow1) {
    // §16.7 row 1 + §10.4.1 "Active → Disconnected": once the
    // transport has reached Active (markReady fired), the next
    // markNotReady is the disconnection edge and raises
    // `error.communication` with `reason="TRANSPORT_UNAVAILABLE"`
    // carrying the target + transport that lost reachability.
    //
    // markReady's queue-drain semantics are independent — the queue
    // is empty here so drain is a no-op; this test isolates the
    // lifecycle transition signal from the FIFO drain path.
    ErrorSink sink;
    OutboundBuffer buf(
        "motor", /*max_pending*/ 4, "zenoh",
        [](const MeshEnvelope&) { return true; },
        std::ref(sink));

    buf.markReady();                     // Ready → Active anchor
    EXPECT_EQ(sink.call_count, 0)
        << "Ready→Active is the entry edge; only Active→Disconnected raises";

    buf.markNotReady();                  // Active → Disconnected → row 1
    ASSERT_EQ(sink.call_count, 1);
    ASSERT_TRUE(sink.last.has_value());
    EXPECT_EQ(sink.last->reason, "TRANSPORT_UNAVAILABLE");
    ASSERT_TRUE(sink.last->target.has_value());
    EXPECT_EQ(*sink.last->target, "motor");
    ASSERT_TRUE(sink.last->transport.has_value());
    EXPECT_EQ(*sink.last->transport, "zenoh");
    EXPECT_FALSE(sink.last->queue_depth.has_value())
        << "row 1 reports the disconnection itself, not queue state";
    EXPECT_FALSE(sink.last->source.has_value())
        << "row 1 is observed at transport layer, not bound to an inbound envelope";
}

TEST(OutboundBufferTest, RepeatedMarkNotReadyRaisesPerTransitionOnly) {
    // Idempotent re-call discipline: a transport callback that asserts
    // not-ready while the buffer is already not-ready does NOT emit a
    // duplicate Row 1. Only the actual lifecycle transition counts.
    // This also models a flicker pattern (Active → Disconnected →
    // Active → Disconnected) — each true→false edge is a distinct
    // disconnection event and earns its own raise.
    ErrorSink sink;
    OutboundBuffer buf(
        "motor", /*max_pending*/ 4, "someip",
        [](const MeshEnvelope&) { return true; },
        std::ref(sink));

    buf.markReady();
    buf.markNotReady();   // transition #1 → raise
    buf.markNotReady();   // already not-ready: no raise
    buf.markNotReady();   // still no raise

    EXPECT_EQ(sink.call_count, 1) << "transitions are deduplicated by the buffer";

    buf.markReady();      // Disconnected → Active reconnect: no raise per §10.4.1
    buf.markNotReady();   // transition #2 → raise

    EXPECT_EQ(sink.call_count, 2)
        << "each Active→Disconnected edge raises; reconnect is transparent";
}
