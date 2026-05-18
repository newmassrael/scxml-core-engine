// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// OutboundBuffer unit tests — SCE_MESH.md §10.10 + §16.7 rows 1, 2, 9.
//
// Sibling of DedupRouter / OrderingBuffer unit tests in Bucket 1
// (Core primitives). Existing E2E coverage exercises the DRAIN path
// (`mesh_someip_late_boot` and `mesh_zenoh_publisher_first` verify
// that envelopes buffered while the transport is not ready survive
// and reach the peer after `markReady()` fires). This file covers
// the three raise paths the buffer owns:
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
//   * §16.7 row 2 SEND_FAILED (per-send api-fail half):
//     The dispatcher returns a non-ok `SendResult` either at
//     `admit`'s fast path (direct send under ready_=true + empty
//     queue) or per envelope in `markReady`'s drain loop. Each
//     non-ok return raises one `error.communication` with
//     `reason="SEND_FAILED"`; the dispatcher's `transport_error`
//     (when populated) is relayed into
//     `CommunicationError::transport_error` so the SCXML author can
//     correlate the loss with transport telemetry. The §10.4.1
//     disconnect-drain clause is satisfied vacuously by
//     OutboundBuffer's "ready_=true ⟹ queue empty" invariant — the
//     queue at the Active→Disconnected edge is empty by construction
//     so `markNotReady` emits Row 1 only.
//
// The byte-shape unit pins for the raised errors live in
// `CommunicationErrorTest::BackpressureDropShape`,
// `CommunicationErrorTest::TransportUnavailableShape`, and
// `CommunicationErrorTest::SendFailedShape`; this file proves each
// raise FIRES under the right precondition and that the captured
// fields match the catalog.

#include "mesh/CommunicationError.h"
#include "mesh/MeshEnvelope.h"
#include "mesh/OutboundBuffer.h"

#include <gtest/gtest.h>

#include <optional>
#include <string>
#include <vector>

using SCE::Mesh::CommunicationError;
using SCE::Mesh::MeshEnvelope;
using SCE::Mesh::OutboundBuffer;
using SCE::Mesh::SendResult;

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
        /* dispatch        */ [](const MeshEnvelope&) { return SendResult::success(); },
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
        [](const MeshEnvelope&) { return SendResult::success(); },
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
        [](const MeshEnvelope&) { return SendResult::success(); },
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
        [](const MeshEnvelope&) { return SendResult::success(); },
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

TEST(OutboundBufferTest, AdmitFastPathDispatchFailRaisesRow2) {
    // §16.7 row 2: when admit's fast path runs the dispatcher and the
    // transport API declines the envelope (return false), the buffer
    // raises `error.communication` with `reason="SEND_FAILED"` so the
    // SCXML author observes the dropped outbound work. The envelope
    // is considered consumed — the buffer does not re-enqueue or
    // retry (§10.10 contract; row 3 DELIVERY_EXHAUSTED is orthogonal).
    ErrorSink sink;
    OutboundBuffer buf(
        /* target          */ "motor",
        /* max_pending     */ 4,
        /* transport_name  */ "someip",
        /* dispatch        */ [](const MeshEnvelope&) { return SendResult::failure(); },
        /* raise_error     */ std::ref(sink));

    buf.markReady();  // Ready→Active so admit takes fast path

    MeshEnvelope env{};
    EXPECT_FALSE(buf.admit(env))
        << "dispatcher declined: admit returns false to caller";

    ASSERT_EQ(sink.call_count, 1);
    EXPECT_EQ(sink.last->reason, "SEND_FAILED");
    ASSERT_TRUE(sink.last->target.has_value());
    EXPECT_EQ(*sink.last->target, "motor");
    ASSERT_TRUE(sink.last->transport.has_value());
    EXPECT_EQ(*sink.last->transport, "someip");
    EXPECT_FALSE(sink.last->transport_error.has_value())
        << "dispatcher returned bare failure(): transport_error stays absent";
}

TEST(OutboundBufferTest, AdmitFastPathDispatchFailRelaysTransportError) {
    // §16.7 row 2 Stage 2: the dispatcher's SendResult::transport_error
    // (vsomeip "app.send returned false" sentinel, zenoh
    // ZException::what(), etc.) is relayed verbatim into the raised
    // CommunicationError::transport_error field. Authors guard on
    // `_event.data.transport_error` to surface the raw API decline in
    // log / telemetry pipelines without losing fidelity through
    // SCE-authored prose.
    ErrorSink sink;
    OutboundBuffer buf(
        "motor", /*max_pending*/ 4, "zenoh",
        [](const MeshEnvelope&) {
            return SendResult::failure("ZException: closed session");
        },
        std::ref(sink));

    buf.markReady();
    MeshEnvelope env{};
    EXPECT_FALSE(buf.admit(env));

    ASSERT_EQ(sink.call_count, 1);
    EXPECT_EQ(sink.last->reason, "SEND_FAILED");
    ASSERT_TRUE(sink.last->transport_error.has_value());
    EXPECT_EQ(*sink.last->transport_error, "ZException: closed session");
}

TEST(OutboundBufferTest, AdmitFastPathDispatchSuccessDoesNotRaise) {
    // Symmetry pin: a successful fast-path dispatch (dispatcher
    // returns true) is the no-op happy path — no error event.
    // Guards against regression where an unconditional Row 2 emit
    // would spam SCXML authors on every successful send.
    ErrorSink sink;
    OutboundBuffer buf(
        "motor", /*max_pending*/ 4, "someip",
        [](const MeshEnvelope&) { return SendResult::success(); },
        std::ref(sink));

    buf.markReady();
    MeshEnvelope env{};
    EXPECT_TRUE(buf.admit(env));
    EXPECT_EQ(sink.call_count, 0) << "no error events on successful dispatch";
}

TEST(OutboundBufferTest, MarkReadyDrainDispatchFailRaisesRow2PerEnvelope) {
    // §16.7 row 2 emitted during drain: when markReady drains queued
    // envelopes after the transport reaches Active, each envelope
    // whose dispatcher returns false raises its own SEND_FAILED
    // event. The raises are deferred to AFTER the drain releases
    // `mu_` (§10.10 lock-discipline) but counted under the mutex
    // during the drain itself.
    ErrorSink sink;
    OutboundBuffer buf(
        "motor", /*max_pending*/ 8, "someip",
        [](const MeshEnvelope&) { return SendResult::failure(); },  // always declines
        std::ref(sink));

    // Seed three envelopes under ready_=false. They wait for the
    // reconnect (markReady) and only then exercise the dispatcher.
    MeshEnvelope env{};
    EXPECT_TRUE(buf.admit(env));
    EXPECT_TRUE(buf.admit(env));
    EXPECT_TRUE(buf.admit(env));
    EXPECT_EQ(buf.queue_depth(), 3u);
    EXPECT_EQ(sink.call_count, 0) << "admits under ready_=false enqueue silently";

    buf.markReady();

    EXPECT_EQ(buf.queue_depth(), 0u) << "drain consumes the queue even on failure";
    ASSERT_EQ(sink.call_count, 3) << "one SEND_FAILED per failed drain dispatch";
    EXPECT_EQ(sink.last->reason, "SEND_FAILED");
    ASSERT_TRUE(sink.last->target.has_value());
    EXPECT_EQ(*sink.last->target, "motor");
    ASSERT_TRUE(sink.last->transport.has_value());
    EXPECT_EQ(*sink.last->transport, "someip");
}

TEST(OutboundBufferTest, MarkReadyDrainDispatchFailRelaysPerEnvelopeTransportError) {
    // §16.7 row 2 Stage 2 drain coverage: each declined envelope's
    // transport_error string is captured one-to-one with the drain
    // order and relayed verbatim into the matching post-drain
    // CommunicationError::transport_error field. Sequence-tagged
    // errors prove the §10.10 capture-then-emit ordering does not
    // collapse or reorder per-envelope diagnostics.
    int call_index = 0;
    ErrorSink sink;
    std::vector<std::optional<std::string>> captured_errors;
    OutboundBuffer buf(
        "motor", /*max_pending*/ 8, "zenoh",
        [&](const MeshEnvelope&) {
            const int idx = call_index++;
            return SendResult::failure(
                "ZException env#" + std::to_string(idx));
        },
        [&](CommunicationError err) {
            captured_errors.push_back(err.transport_error);
            sink(std::move(err));
        });

    MeshEnvelope env{};
    EXPECT_TRUE(buf.admit(env));
    EXPECT_TRUE(buf.admit(env));
    EXPECT_TRUE(buf.admit(env));
    EXPECT_EQ(buf.queue_depth(), 3u);

    buf.markReady();

    ASSERT_EQ(sink.call_count, 3);
    ASSERT_EQ(captured_errors.size(), 3u);
    ASSERT_TRUE(captured_errors[0].has_value());
    EXPECT_EQ(*captured_errors[0], "ZException env#0");
    ASSERT_TRUE(captured_errors[1].has_value());
    EXPECT_EQ(*captured_errors[1], "ZException env#1");
    ASSERT_TRUE(captured_errors[2].has_value());
    EXPECT_EQ(*captured_errors[2], "ZException env#2");
}

TEST(OutboundBufferTest, MarkReadyDrainMixedSuccessAndFailureRaisesPerFailure) {
    // Drain through a dispatcher whose result depends on envelope
    // sequence (alternating succeed/fail). Pins that the failure-
    // count is exactly the number of declined envelopes, not the
    // total drain depth — successful dispatches must not spuriously
    // raise Row 2.
    int call_index = 0;
    ErrorSink sink;
    OutboundBuffer buf(
        "motor", /*max_pending*/ 8, "someip",
        [&](const MeshEnvelope&) {
            // Indices: 0=ok, 1=fail, 2=ok, 3=fail
            return ((call_index++ % 2) == 0)
                       ? SendResult::success()
                       : SendResult::failure();
        },
        std::ref(sink));

    MeshEnvelope env{};
    EXPECT_TRUE(buf.admit(env));
    EXPECT_TRUE(buf.admit(env));
    EXPECT_TRUE(buf.admit(env));
    EXPECT_TRUE(buf.admit(env));
    EXPECT_EQ(buf.queue_depth(), 4u);

    buf.markReady();

    EXPECT_EQ(call_index, 4) << "drain visits every queued envelope";
    EXPECT_EQ(sink.call_count, 2) << "exactly the two declined envelopes raise";
    EXPECT_EQ(sink.last->reason, "SEND_FAILED");
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
        [](const MeshEnvelope&) { return SendResult::success(); },
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
