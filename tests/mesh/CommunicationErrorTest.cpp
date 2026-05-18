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
// since those pin byte-exact output.

#include "mesh/CommunicationError.h"

#include <gtest/gtest.h>

#include <string>

using SCE::Mesh::CommunicationError;

namespace {

std::string bytes_to_string(const std::vector<std::uint8_t>& b) {
    return std::string(b.begin(), b.end());
}

}  // namespace

TEST(CommunicationErrorTest, MissingSequenceMinimalShape) {
    // §16.7 row 11 — MISSING_SEQUENCE carries source + envelope_id; no
    // reason-specific extras beyond the baseline.
    CommunicationError err;
    err.reason = "MISSING_SEQUENCE";
    err.source = "motor";
    err.envelope_id = std::array<std::uint8_t, 16>{
        0x01, 0x82, 0x0b, 0xc0, 0xde, 0xad, 0x7e, 0x50,
        0x81, 0xab, 0xca, 0xfe, 0xba, 0xbe, 0xbe, 0xef};

    const auto out = bytes_to_string(err.toJsonBytes());
    EXPECT_EQ(out,
              "{\"errorName\":\"communication\","
              "\"reason\":\"MISSING_SEQUENCE\","
              "\"source\":\"motor\","
              "\"envelope_id\":\"01820bc0-dead-7e50-81ab-cafebabebeef\"}");
}

TEST(CommunicationErrorTest, OrderingGapFullShape) {
    // §16.7 row 12 — ORDERING_GAP carries lost_seq_lo + lost_seq_hi
    // plus the baseline `source`. Field order must match declaration
    // order in toJsonBytes so the wire shape is stable.
    CommunicationError err;
    err.reason = "ORDERING_GAP";
    err.source = "motor";
    err.lost_seq_lo = 2;
    err.lost_seq_hi = 5;

    const auto out = bytes_to_string(err.toJsonBytes());
    EXPECT_EQ(out,
              "{\"errorName\":\"communication\","
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
    err.reason = "PEER_PARTITIONED";
    err.target = "motor";
    err.last_seen_ms_ago = 142;

    const auto out = bytes_to_string(err.toJsonBytes());
    EXPECT_EQ(out,
              "{\"errorName\":\"communication\","
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
    err.reason = "PARALLEL_BARRIER_TIMEOUT";
    err.parallel_id = "root";
    err.missing_regions = std::vector<std::string>{"right"};
    err.timeout_ms = 150;

    const auto out = bytes_to_string(err.toJsonBytes());
    EXPECT_EQ(out,
              "{\"errorName\":\"communication\","
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
    err.reason = "REGION_PARTITIONED";
    err.last_seen_ms_ago = 142;
    err.machine = "motor";
    err.partition = "motor_right";

    const auto out = bytes_to_string(err.toJsonBytes());
    EXPECT_EQ(out,
              "{\"errorName\":\"communication\","
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
    err.reason = "BACKPRESSURE_DROP";
    err.target = "motor";
    err.transport = "someip";
    err.queue_depth = 1024;

    const auto out = bytes_to_string(err.toJsonBytes());
    EXPECT_EQ(out,
              "{\"errorName\":\"communication\","
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
    err.reason = "ENVELOPE_CORRUPT";
    err.transport = "someip";
    err.codec = "cbor";

    const auto out = bytes_to_string(err.toJsonBytes());
    EXPECT_EQ(out,
              "{\"errorName\":\"communication\","
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
    err.reason = "ENVELOPE_CORRUPT";
    err.transport = "zenoh";
    err.codec = "cbor";
    err.position = 42;

    const auto out = bytes_to_string(err.toJsonBytes());
    EXPECT_EQ(out,
              "{\"errorName\":\"communication\","
              "\"reason\":\"ENVELOPE_CORRUPT\","
              "\"transport\":\"zenoh\","
              "\"codec\":\"cbor\","
              "\"position\":42}");
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
    err.reason = "DEDUP_WINDOW_OVERFLOW";
    err.source = "motor";
    err.window_size = 256;

    const auto out = bytes_to_string(err.toJsonBytes());
    EXPECT_EQ(out,
              "{\"errorName\":\"communication\","
              "\"reason\":\"DEDUP_WINDOW_OVERFLOW\","
              "\"source\":\"motor\","
              "\"window_size\":256}");
}

TEST(CommunicationErrorTest, OptionalFieldsAbsentAreSkipped) {
    // Only `reason` is required; all other fields default to absent
    // and must not appear in the output (not even as `null`).
    CommunicationError err;
    err.reason = "MISSING_SEQUENCE";

    const auto out = bytes_to_string(err.toJsonBytes());
    EXPECT_EQ(out, "{\"errorName\":\"communication\",\"reason\":\"MISSING_SEQUENCE\"}");
}

TEST(CommunicationErrorTest, EnvelopeIdRendersAsCanonicalUuidString) {
    // The envelope_id byte array renders through SCE::uuid::to_string,
    // which produces the RFC 4122 §3 canonical 36-char form. Guard
    // that we haven't regressed to hex-without-dashes or similar.
    CommunicationError err;
    err.reason = "MISSING_SEQUENCE";
    err.envelope_id = std::array<std::uint8_t, 16>{
        0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
        0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff};

    const auto out = bytes_to_string(err.toJsonBytes());
    EXPECT_NE(out.find("\"envelope_id\":\"00112233-4455-6677-8899-aabbccddeeff\""),
              std::string::npos)
        << "rendered: " << out;
}

TEST(CommunicationErrorTest, StringEscapesQuotesAndBackslashes) {
    // Raise sites can stamp env.source from deploy.yaml machine names,
    // which today are alphanumeric but the JSON writer must be
    // defensible against a future machine name containing quotes.
    CommunicationError err;
    err.reason = "MISSING_SEQUENCE";
    err.source = std::string("a\"b\\c\n");

    const auto out = bytes_to_string(err.toJsonBytes());
    EXPECT_NE(out.find("\"source\":\"a\\\"b\\\\c\\n\""), std::string::npos)
        << "rendered: " << out;
}
