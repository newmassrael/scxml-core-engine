// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Unit tests for SCE::Mesh::CommunicationError (SCE_MESH.md §10.7.1 + §16.7).
//
// Pins the JSON render shape the runtime uses to populate
// `_event.data` on `error.communication` events. Byte-compare on the
// happy path keeps the payload contract stable so SCXML authors can
// guard on specific fields (`_event.data.reason == "ORDERING_GAP"`,
// `_event.data.lost_seq_lo`) without runtime surprises.
//
// Mutation guard: stripping the field-order discipline (e.g. switching
// to unordered insertion) must fail at least:
//   * MissingSequenceMinimalShape
//   * OrderingGapFullShape
//   * PeerPartitionedShape
//   * BarrierTimeoutShape
//   * RegionPartitionedShape
//   * BackpressureDropShape
//   * EnvelopeCorruptShape
//   * DedupWindowOverflowShape
//   * TransportUnavailableShape
//   * SendFailedShape
//   * SendFailedShapeWithTransportError
//   * InvokeChildLostShapeMeshRpc
//   * InvokeChildLostShapeScxmlInvoke
//   * DeliveryExhaustedShapeAfterRetries
//   * DeliveryExhaustedShapeTerminalFastFail
// since those pin byte-exact output.

#include "mesh/CommunicationError.h"

#include <gtest/gtest.h>

#include <string>

using SCE::Mesh::CommunicationError;
using SCE::Mesh::ReasonCode;

namespace {

std::string bytes_to_string(const std::vector<std::uint8_t> &b) {
    return std::string(b.begin(), b.end());
}

}  // namespace

TEST(CommunicationErrorTest, MissingSequenceMinimalShape) {
    // §16.7 row 11 — MISSING_SEQUENCE carries source + envelope_id; no
    // reason-specific extras beyond the baseline.
    CommunicationError err;
    err.reason = ReasonCode::MissingSequence;
    err.source = "motor";
    err.envelope_id = std::array<std::uint8_t, 16>{0x01, 0x82, 0x0b, 0xc0, 0xde, 0xad, 0x7e, 0x50,
                                                   0x81, 0xab, 0xca, 0xfe, 0xba, 0xbe, 0xbe, 0xef};

    const auto out = bytes_to_string(err.toJsonBytes());
    EXPECT_EQ(out, "{\"errorName\":\"communication\","
                   "\"reason\":\"MISSING_SEQUENCE\","
                   "\"source\":\"motor\","
                   "\"envelope_id\":\"01820bc0-dead-7e50-81ab-cafebabebeef\"}");
}

TEST(CommunicationErrorTest, OrderingGapFullShape) {
    // §16.7 row 12 — ORDERING_GAP carries lost_seq_lo + lost_seq_hi
    // plus the baseline `source`. Field order must match declaration
    // order in toJsonBytes so the wire shape is stable.
    CommunicationError err;
    err.reason = ReasonCode::OrderingGap;
    err.source = "motor";
    err.lost_seq_lo = 2;
    err.lost_seq_hi = 5;

    const auto out = bytes_to_string(err.toJsonBytes());
    EXPECT_EQ(out, "{\"errorName\":\"communication\","
                   "\"reason\":\"ORDERING_GAP\","
                   "\"source\":\"motor\","
                   "\"lost_seq_lo\":2,"
                   "\"lost_seq_hi\":5}");
}

TEST(CommunicationErrorTest, PeerPartitionedShape) {
    // §16.7 row 8 — PEER_PARTITIONED carries target + last_seen_ms_ago.
    // No `source` / `envelope_id` because the raise is not triggered
    // by any specific inbound envelope — the observation is the peer's
    // liveliness DELETE sample.
    CommunicationError err;
    err.reason = ReasonCode::PeerPartitioned;
    err.target = "motor";
    err.last_seen_ms_ago = 142;

    const auto out = bytes_to_string(err.toJsonBytes());
    EXPECT_EQ(out, "{\"errorName\":\"communication\","
                   "\"reason\":\"PEER_PARTITIONED\","
                   "\"target\":\"motor\","
                   "\"last_seen_ms_ago\":142}");
}

TEST(CommunicationErrorTest, BarrierTimeoutShape) {
    // §16.7 row 6 — PARALLEL_BARRIER_TIMEOUT carries parallel_id +
    // missing_regions + timeout_ms. No `source` / `envelope_id`
    // because the raise is timer-driven, not inbound-envelope-driven.
    // The runtime raise site lives in `state_machine.jinja2`
    // TimerHooks::arm; this test pins the on-the-wire JSON shape that
    // SCXML authors guard on
    // (`<transition event="error.communication"
    //   cond="_event.data.reason == 'PARALLEL_BARRIER_TIMEOUT' &&
    //         _event.data.parallel_id == 'root'">`).
    CommunicationError err;
    err.reason = ReasonCode::ParallelBarrierTimeout;
    err.parallel_id = "root";
    err.missing_regions = std::vector<std::string>{"right"};
    err.timeout_ms = 150;

    const auto out = bytes_to_string(err.toJsonBytes());
    EXPECT_EQ(out, "{\"errorName\":\"communication\","
                   "\"reason\":\"PARALLEL_BARRIER_TIMEOUT\","
                   "\"parallel_id\":\"root\","
                   "\"missing_regions\":[\"right\"],"
                   "\"timeout_ms\":150}");
}

TEST(CommunicationErrorTest, RegionPartitionedShape) {
    // §16.7 row 13 — REGION_PARTITIONED carries machine + partition,
    // plus the optional last_seen_ms_ago reused from row 8. Orthogonal
    // to row 8: row 8 is machine identity, row 13 is the machine +
    // partition pair surfaced by per-partition liveness. Runtime raise
    // sites live in mesh_transport.h.jinja2 (Zenoh 3-segment
    // `sce/live/<machine>/<partition>` DELETE + SOME/IP region-level
    // availability handler). Authors guard on
    // `_event.data.reason == 'REGION_PARTITIONED' &&
    //  _event.data.machine == '<m>' &&
    //  _event.data.partition == '<p>'`.
    CommunicationError err;
    err.reason = ReasonCode::RegionPartitioned;
    err.last_seen_ms_ago = 142;
    err.machine = "motor";
    err.partition = "motor_right";

    const auto out = bytes_to_string(err.toJsonBytes());
    EXPECT_EQ(out, "{\"errorName\":\"communication\","
                   "\"reason\":\"REGION_PARTITIONED\","
                   "\"last_seen_ms_ago\":142,"
                   "\"machine\":\"motor\","
                   "\"partition\":\"motor_right\"}");
}

TEST(CommunicationErrorTest, BackpressureDropShape) {
    // §16.7 row 9 — BACKPRESSURE_DROP carries transport + target +
    // queue_depth. Raised by `OutboundBuffer::admit` when the per-
    // target queue is full (§10.10 `max_pending_per_target`). Field
    // order on the wire follows CommunicationError's declaration
    // order — target precedes transport, not the catalog row's
    // textual order. Authors guard on
    // `_event.data.reason == 'BACKPRESSURE_DROP' &&
    //  _event.data.target == '<peer>'` to react to a specific peer
    // backing up; `queue_depth` carries the observed buffer depth
    // at the moment of overflow for diagnostics.
    CommunicationError err;
    err.reason = ReasonCode::BackpressureDrop;
    err.target = "motor";
    err.transport = "someip";
    err.queue_depth = 1024;

    const auto out = bytes_to_string(err.toJsonBytes());
    EXPECT_EQ(out, "{\"errorName\":\"communication\","
                   "\"reason\":\"BACKPRESSURE_DROP\","
                   "\"target\":\"motor\","
                   "\"transport\":\"someip\","
                   "\"queue_depth\":1024}");
}

TEST(CommunicationErrorTest, EnvelopeCorruptShape) {
    // §16.7 row 4 — ENVELOPE_CORRUPT carries transport + codec +
    // optional position. Runtime raise sites live at every
    // `decodeEnvelope` call within the codegen TransportRouter
    // (`mesh_transport.h.jinja2`). All SCE transports today wire
    // canonical CBOR so `codec="cbor"` is the only value stamped
    // by current emitters; the field stays string-typed so a
    // future per-binding-codec transport can mark its slot without
    // an enum refactor. `position` stays absent on tinycbor-failed
    // decodes because the parser does not expose a post-failure
    // cursor through decodeEnvelope's bool return.
    CommunicationError err;
    err.reason = ReasonCode::EnvelopeCorrupt;
    err.transport = "someip";
    err.codec = "cbor";

    const auto out = bytes_to_string(err.toJsonBytes());
    EXPECT_EQ(out, "{\"errorName\":\"communication\","
                   "\"reason\":\"ENVELOPE_CORRUPT\","
                   "\"transport\":\"someip\","
                   "\"codec\":\"cbor\"}");
}

TEST(CommunicationErrorTest, EnvelopeCorruptShapeWithPosition) {
    // Same row 4 contract with a populated `position` — proves the
    // optional field renders in declaration order between `codec`
    // and `queue_depth` so future codec backends that report fault
    // offsets stay wire-shape compatible.
    CommunicationError err;
    err.reason = ReasonCode::EnvelopeCorrupt;
    err.transport = "zenoh";
    err.codec = "cbor";
    err.position = 42;

    const auto out = bytes_to_string(err.toJsonBytes());
    EXPECT_EQ(out, "{\"errorName\":\"communication\","
                   "\"reason\":\"ENVELOPE_CORRUPT\","
                   "\"transport\":\"zenoh\","
                   "\"codec\":\"cbor\","
                   "\"position\":42}");
}

TEST(CommunicationErrorTest, InvokeChildLostShapeMeshRpc) {
    // §16.7 row 5 — INVOKE_CHILD_LOST §9.5 mesh-rpc emit. Raised by
    // TransportRouter::shutdown when iterating
    // `invoke_correlation_`'s outstanding entries via
    // `cancelAllPending` (per §10.4.1 row 1704 "Outstanding RPC
    // entries are cancelled with reason: INVOKE_CHILD_LOST" on
    // transport Shutdown). The §9.5 caller stringifies the UUID v7
    // correlation key via `SCE::uuid::to_string` so the wire
    // `invoke_id` is the RFC 4122 canonical 36-char form (mirrors
    // `envelope_id`'s shape so authors using either field key follow
    // the same parsing discipline). Field order on the wire follows
    // `CommunicationError`'s declaration order: `invoke_id` precedes
    // `target`. Authors guard on `_event.data.reason ==
    // 'INVOKE_CHILD_LOST' && _event.data.target == '<peer>'` to
    // react to a specific child device disappearing.
    //
    // Contrast with §9.6 L1393 `error.execution(reason=
    // SESSION_F_TRANSPORT_UNAVAILABLE)`: that case fires at invoke
    // entry-time when no deploy.yaml binding exists for the peer
    // (init-time configuration absence). Row 5 fires AFTER a
    // binding was active and the transport reached Shutdown with
    // outstanding work.
    CommunicationError err;
    err.reason = ReasonCode::InvokeChildLost;
    err.invoke_id = "01820bc0-dead-7e50-81ab-cafebabebeef";
    err.target = "motor";

    const auto out = bytes_to_string(err.toJsonBytes());
    EXPECT_EQ(out, "{\"errorName\":\"communication\","
                   "\"reason\":\"INVOKE_CHILD_LOST\","
                   "\"invoke_id\":\"01820bc0-dead-7e50-81ab-cafebabebeef\","
                   "\"target\":\"motor\"}");
}

TEST(CommunicationErrorTest, InvokeChildLostShapeScxmlInvoke) {
    // §16.7 row 5 — INVOKE_CHILD_LOST §9.6 scxml-invoke emit. The
    // §9.6 path passes the W3C SCXML invokeId string directly (a
    // free-form identifier per W3C SCXML 6.4.1, NOT a UUID). The
    // codegen-emitted `<invoke id="myInvoke">` literal lands in
    // `invoke_id` verbatim. Same wire field name as the §9.5 half,
    // different value shape — authors who see a non-UUID value
    // know the failure was on the §9.6 scxml-invoke axis (W3C
    // `<invoke type="scxml">` over §9.6 worker host).
    CommunicationError err;
    err.reason = ReasonCode::InvokeChildLost;
    err.invoke_id = "myInvoke";
    err.target = "worker";

    const auto out = bytes_to_string(err.toJsonBytes());
    EXPECT_EQ(out, "{\"errorName\":\"communication\","
                   "\"reason\":\"INVOKE_CHILD_LOST\","
                   "\"invoke_id\":\"myInvoke\","
                   "\"target\":\"worker\"}");
}

TEST(CommunicationErrorTest, SendFailedShape) {
    // §16.7 row 2 — SEND_FAILED. Raised by OutboundBuffer at the
    // dispatcher-fail observation points: admit fast path (when the
    // transport API declines a direct send) and markReady drain (per
    // declined envelope in the buffered batch). The §10.4.1 row 1702
    // "Enqueued-but-unsent envelopes are failed individually" clause
    // is satisfied vacuously in OutboundBuffer because `ready_=true`
    // implies `queue.empty()` by construction (admit fast-paths under
    // ready+empty, markReady drains under `mu_`) — there is never a
    // non-empty queue at the moment of an Active→Disconnected edge.
    //
    // Minimal shape — `target` and `transport` only. The dispatcher
    // can decline without a transport-API message attached (e.g. an
    // app-pointer null check inside the SOME/IP dispatcher closure),
    // in which case `transport_error` stays absent and the JSON
    // wire shape contains the bare row-2 baseline. The
    // `SendFailedShapeWithTransportError` test below pins the
    // populated variant.
    //
    // Authors guard on `_event.data.reason == 'SEND_FAILED' &&
    //  _event.data.target == '<peer>'` to react to dropped outbound
    // work for a specific peer.
    CommunicationError err;
    err.reason = ReasonCode::SendFailed;
    err.target = "motor";
    err.transport = "someip";

    const auto out = bytes_to_string(err.toJsonBytes());
    EXPECT_EQ(out, "{\"errorName\":\"communication\","
                   "\"reason\":\"SEND_FAILED\","
                   "\"target\":\"motor\","
                   "\"transport\":\"someip\"}");
}

TEST(CommunicationErrorTest, SendFailedShapeWithTransportError) {
    // §16.7 row 2 Stage 2: the dispatcher's `SendResult::transport_error`
    // (vsomeip "app.send returned false" sentinel, zenoh
    // `ZException::what()`) is relayed verbatim into
    // `CommunicationError::transport_error` so the SCXML author can
    // correlate the loss with transport telemetry. JSON insertion
    // order: `transport` precedes `transport_error` per the source
    // order of the assignments inside `toJsonBytes`.
    //
    // Authors guard on `_event.data.reason == 'SEND_FAILED' &&
    //  _event.data.transport_error == '<api-decline>'` to react to
    // a specific underlying API failure.
    CommunicationError err;
    err.reason = ReasonCode::SendFailed;
    err.target = "motor";
    err.transport = "zenoh";
    err.transport_error = "ZException: closed session";

    const auto out = bytes_to_string(err.toJsonBytes());
    EXPECT_EQ(out, "{\"errorName\":\"communication\","
                   "\"reason\":\"SEND_FAILED\","
                   "\"target\":\"motor\","
                   "\"transport\":\"zenoh\","
                   "\"transport_error\":\"ZException: closed session\"}");
}

TEST(CommunicationErrorTest, DeliveryExhaustedShapeAfterRetries) {
    // §16.7 row 3 — DELIVERY_EXHAUSTED carries target + transport +
    // attempts + (last observed) transport_error. Raised by the
    // RetryingDispatcher when the configured `max_retries` budget is
    // consumed without a successful dispatch (attempts == max_retries+1).
    // JSON insertion order follows CommunicationError's declaration
    // order: target → transport → transport_error → attempts (the new
    // field sits adjacent to transport_error to keep the row 2 / row 3
    // shapes visually congruent for authors comparing them).
    //
    // Authors guard on `_event.data.reason == 'DELIVERY_EXHAUSTED' &&
    //  _event.data.target == 'motor' && _event.data.attempts >= 4` to
    // route into an out-of-band fallback once SCE has given up.
    CommunicationError err;
    err.reason = ReasonCode::DeliveryExhausted;
    err.target = "motor";
    err.transport = "zenoh";
    err.transport_error = "ZException: closed session";
    err.attempts = 4;  // first attempt + 3 retries

    const auto out = bytes_to_string(err.toJsonBytes());
    EXPECT_EQ(out, "{\"errorName\":\"communication\","
                   "\"reason\":\"DELIVERY_EXHAUSTED\","
                   "\"target\":\"motor\","
                   "\"transport\":\"zenoh\","
                   "\"transport_error\":\"ZException: closed session\","
                   "\"attempts\":4}");
}

TEST(CommunicationErrorTest, UnauthorizedShape) {
    // §16.7 row 10 — UNAUTHORIZED carries target + transport +
    // transport_status. Fires when the peer rejects the binding at
    // the trust-boundary handshake (Zenoh TLS denial, SOMEIP SD
    // denial). JSON insertion order follows CommunicationError's
    // declaration order: target → transport → transport_status.
    // transport_status is the new row 10 field — sits adjacent to
    // transport_error / attempts to keep the row 2 / 3 / 10 shapes
    // visually congruent for authors comparing them.
    //
    // Authors guard on `_event.data.reason == 'UNAUTHORIZED' &&
    //  _event.data.target == 'motor'` to surface the trust failure
    // out-of-band; transport_status carries the raw API rejection
    // text (Zenoh `ZException::what()` truncated to the auth-tied
    // substring; SOMEIP SD response code label).
    CommunicationError err;
    err.reason = ReasonCode::Unauthorized;
    err.target = "motor";
    err.transport = "zenoh";
    err.transport_status = "TLS: peer certificate fingerprint mismatch";

    const auto out = bytes_to_string(err.toJsonBytes());
    EXPECT_EQ(out, "{\"errorName\":\"communication\","
                   "\"reason\":\"UNAUTHORIZED\","
                   "\"target\":\"motor\","
                   "\"transport\":\"zenoh\","
                   "\"transport_status\":\"TLS: peer certificate fingerprint mismatch\"}");
}

TEST(CommunicationErrorTest, DeliveryExhaustedShapeTerminalFastFail) {
    // §16.7 row 3 — when the dispatcher classified its first failure
    // as TERMINAL (SendResult.retryable == false), the retry layer
    // fast-fails after a single attempt and DELIVERY_EXHAUSTED reports
    // `attempts == 1`. The author can branch on `attempts == 1` vs
    // `attempts > 1` to distinguish "config error" from "tried and
    // gave up". This pin captures the fast-fail variant; the
    // `transport_error` carries the same sentinel the SOME/IP / Zenoh
    // dispatchers stamp on terminal classification (per the codegen
    // dispatcher closures in mesh_transport.h.jinja2).
    CommunicationError err;
    err.reason = ReasonCode::DeliveryExhausted;
    err.target = "motor";
    err.transport = "someip";
    err.transport_error = "vsomeip app not initialized";
    err.attempts = 1;

    const auto out = bytes_to_string(err.toJsonBytes());
    EXPECT_EQ(out, "{\"errorName\":\"communication\","
                   "\"reason\":\"DELIVERY_EXHAUSTED\","
                   "\"target\":\"motor\","
                   "\"transport\":\"someip\","
                   "\"transport_error\":\"vsomeip app not initialized\","
                   "\"attempts\":1}");
}

TEST(CommunicationErrorTest, TransportUnavailableShape) {
    // §16.7 row 1 — TRANSPORT_UNAVAILABLE carries target + transport.
    // Raised by `OutboundBuffer::markNotReady` on the `true → false`
    // transition (the §10.4.1 "Active → Disconnected" lifecycle edge:
    // SOME/IP availability=false, Zenoh matching=false, TCP RST). No
    // `source` / `envelope_id` because the raise is observed at the
    // transport layer, not triggered by a specific inbound envelope;
    // no `queue_depth` either — row 1 reports the disconnection itself,
    // whereas row 9 BACKPRESSURE_DROP reports queue overflow that may
    // be a downstream consequence. Field order on the wire follows
    // CommunicationError's declaration order: target precedes
    // transport, matching the row-9 BackpressureDropShape pin so the
    // two transport-keyed rows render under the same discipline.
    // Authors guard on `_event.data.reason == 'TRANSPORT_UNAVAILABLE' &&
    //  _event.data.target == '<peer>'` to switch to a fallback path for
    // a specific peer's transport drop.
    CommunicationError err;
    err.reason = ReasonCode::TransportUnavailable;
    err.target = "motor";
    err.transport = "someip";

    const auto out = bytes_to_string(err.toJsonBytes());
    EXPECT_EQ(out, "{\"errorName\":\"communication\","
                   "\"reason\":\"TRANSPORT_UNAVAILABLE\","
                   "\"target\":\"motor\","
                   "\"transport\":\"someip\"}");
}

TEST(CommunicationErrorTest, DedupWindowOverflowShape) {
    // §16.7 row 7 — DEDUP_WINDOW_OVERFLOW carries source + window_size.
    // Raised by the codegen TransportRouter's dedup call site when
    // `DedupRouter::admitWithSignal` returns NovelWithEviction —
    // the spec's "sustained rate exceeds window capacity" condition
    // is observed operationally as "novel id evicted an existing
    // entry". `window_size` echoes `DedupWindow::kCapacity` (256)
    // so authors can correlate the raise with the configured ring
    // depth without parsing extra context.
    CommunicationError err;
    err.reason = ReasonCode::DedupWindowOverflow;
    err.source = "motor";
    err.window_size = 256;

    const auto out = bytes_to_string(err.toJsonBytes());
    EXPECT_EQ(out, "{\"errorName\":\"communication\","
                   "\"reason\":\"DEDUP_WINDOW_OVERFLOW\","
                   "\"source\":\"motor\","
                   "\"window_size\":256}");
}

TEST(CommunicationErrorTest, OptionalFieldsAbsentAreSkipped) {
    // Only `reason` is required; all other fields default to absent
    // and must not appear in the output (not even as `null`).
    CommunicationError err;
    err.reason = ReasonCode::MissingSequence;

    const auto out = bytes_to_string(err.toJsonBytes());
    EXPECT_EQ(out, "{\"errorName\":\"communication\",\"reason\":\"MISSING_SEQUENCE\"}");
}

TEST(CommunicationErrorTest, EnvelopeIdRendersAsCanonicalUuidString) {
    // The envelope_id byte array renders through SCE::uuid::to_string,
    // which produces the RFC 4122 §3 canonical 36-char form. Guard
    // that we haven't regressed to hex-without-dashes or similar.
    CommunicationError err;
    err.reason = ReasonCode::MissingSequence;
    err.envelope_id = std::array<std::uint8_t, 16>{0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
                                                   0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff};

    const auto out = bytes_to_string(err.toJsonBytes());
    EXPECT_NE(out.find("\"envelope_id\":\"00112233-4455-6677-8899-aabbccddeeff\""), std::string::npos)
        << "rendered: " << out;
}

TEST(CommunicationErrorTest, StringEscapesQuotesAndBackslashes) {
    // Raise sites can stamp env.source from deploy.yaml machine names,
    // which today are alphanumeric but the JSON writer must be
    // defensible against a future machine name containing quotes.
    CommunicationError err;
    err.reason = ReasonCode::MissingSequence;
    err.source = std::string("a\"b\\c\n");

    const auto out = bytes_to_string(err.toJsonBytes());
    EXPECT_NE(out.find("\"source\":\"a\\\"b\\\\c\\n\""), std::string::npos) << "rendered: " << out;
}
