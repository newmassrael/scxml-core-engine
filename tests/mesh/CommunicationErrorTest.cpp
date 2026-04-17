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
    // §16.7 row 12 — MISSING_SEQUENCE carries source + envelope_id; no
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
    // §16.7 row 13 — ORDERING_GAP carries lost_seq_lo + lost_seq_hi
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
    // §16.7 row 9 — PEER_PARTITIONED carries target + last_seen_ms_ago.
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
