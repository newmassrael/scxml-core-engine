// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// OutboundBuffer unit tests — SCE_MESH.md §10.10 + §16.7 row 9.
//
// Sibling of DedupRouter / OrderingBuffer unit tests in Bucket 1
// (Core primitives). Existing E2E coverage exercises the DRAIN path
// (`mesh_someip_late_boot` and `mesh_zenoh_publisher_first` verify
// that envelopes buffered while the transport is not ready survive
// and reach the peer after `markReady()` fires). This file covers
// the OVERFLOW path — the §16.7 row 9 contract that
// `OutboundBuffer::admit` raises `error.communication` with
// `reason="BACKPRESSURE_DROP"` and drops the newest envelope when
// the per-target queue is full.
//
// The byte-shape unit pin for the raised error lives in
// `CommunicationErrorTest::BackpressureDropShape`; this file proves
// the raise FIRES under the right precondition and that the
// captured fields match the catalog. The two together close the
// row 9 entry of §16.7 at the same E2E + byte-shape ratification
// level rows 6 / 8 / 11 / 12 / 13 already enjoy.

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
