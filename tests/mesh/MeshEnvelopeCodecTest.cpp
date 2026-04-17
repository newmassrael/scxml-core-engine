// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "mesh/MeshEnvelope.h"
#include "mesh/MeshEnvelopeCodec.h"

#include <gtest/gtest.h>

#include <array>
#include <cstdint>
#include <string>
#include <vector>

using SCE::Mesh::MeshEnvelope;
using SCE::Mesh::PatternKind;
using SCE::Mesh::PayloadCodec;
using SCE::Mesh::RpcStatus;

namespace {

constexpr std::array<uint8_t, 16> kSampleId = {
    0x01, 0x82, 0xb1, 0x4d, 0xa3, 0x5c, 0x70, 0x12,
    0xb4, 0xde, 0xf0, 0x42, 0x9a, 0x88, 0x77, 0x66,
};

constexpr std::array<uint8_t, 16> kSampleCorrelation = {
    0xff, 0xee, 0xdd, 0xcc, 0xbb, 0xaa, 0x70, 0x01,
    0x90, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
};

MeshEnvelope minimalEnvelope() {
    MeshEnvelope env{};
    env.id = kSampleId;
    env.source = "ecu_brake";
    env.type = "brake.activate";
    env.pattern = PatternKind::FireForget;
    env.datacontenttype = PayloadCodec::Json;
    env.data = {'{', '"', 'v', '"', ':', '4', '2', '}'};
    return env;
}

MeshEnvelope fullEnvelope() {
    MeshEnvelope env = minimalEnvelope();
    env.pattern = PatternKind::RpcReply;
    env.subject = "service.compute_force";
    env.correlation_id = kSampleCorrelation;
    env.reply_to = "#brake_service";
    env.invoke_id = kSampleCorrelation;
    env.rpc_status = RpcStatus::DeadlineExceeded;
    env.rpc_error_message = "remote took too long";
    env.deadline_unix_ms = 1'713'000'000'000ULL;
    env.sequence_no = 42ULL;
    return env;
}

}  // namespace

// ── Round-trip tests ────────────────────────────────────────────────────

TEST(MeshEnvelopeCodecTest, MinimalRoundTrip) {
    auto env = minimalEnvelope();
    auto bytes = SCE::Mesh::encodeEnvelope(env);
    ASSERT_FALSE(bytes.empty());

    MeshEnvelope decoded;
    ASSERT_TRUE(SCE::Mesh::decodeEnvelope(bytes.data(), bytes.size(), decoded));
    EXPECT_EQ(env, decoded);
}

TEST(MeshEnvelopeCodecTest, FullRoundTrip) {
    auto env = fullEnvelope();
    auto bytes = SCE::Mesh::encodeEnvelope(env);
    ASSERT_FALSE(bytes.empty());

    MeshEnvelope decoded;
    ASSERT_TRUE(SCE::Mesh::decodeEnvelope(bytes.data(), bytes.size(), decoded));
    EXPECT_EQ(env, decoded);
    EXPECT_TRUE(decoded.subject.has_value());
    EXPECT_TRUE(decoded.correlation_id.has_value());
    EXPECT_TRUE(decoded.rpc_status.has_value());
    EXPECT_EQ(*decoded.rpc_status, RpcStatus::DeadlineExceeded);
}

TEST(MeshEnvelopeCodecTest, EmptyDataRoundTrip) {
    auto env = minimalEnvelope();
    env.datacontenttype = PayloadCodec::None;
    env.data.clear();
    auto bytes = SCE::Mesh::encodeEnvelope(env);
    ASSERT_FALSE(bytes.empty());

    MeshEnvelope decoded;
    ASSERT_TRUE(SCE::Mesh::decodeEnvelope(bytes.data(), bytes.size(), decoded));
    EXPECT_TRUE(decoded.data.empty());
    EXPECT_EQ(env, decoded);
}

TEST(MeshEnvelopeCodecTest, OptionalsAbsentStayAbsent) {
    auto env = minimalEnvelope();
    auto bytes = SCE::Mesh::encodeEnvelope(env);

    MeshEnvelope decoded;
    ASSERT_TRUE(SCE::Mesh::decodeEnvelope(bytes.data(), bytes.size(), decoded));
    EXPECT_FALSE(decoded.subject);
    EXPECT_FALSE(decoded.correlation_id);
    EXPECT_FALSE(decoded.reply_to);
    EXPECT_FALSE(decoded.invoke_id);
    EXPECT_FALSE(decoded.rpc_status);
    EXPECT_FALSE(decoded.rpc_error_message);
    EXPECT_FALSE(decoded.deadline_unix_ms);
    EXPECT_FALSE(decoded.sequence_no);
}

TEST(MeshEnvelopeCodecTest, EmptyTextStringRoundTrip) {
    auto env = minimalEnvelope();
    env.source = "";
    env.subject = std::string("");
    auto bytes = SCE::Mesh::encodeEnvelope(env);
    ASSERT_FALSE(bytes.empty());

    MeshEnvelope decoded;
    ASSERT_TRUE(SCE::Mesh::decodeEnvelope(bytes.data(), bytes.size(), decoded));
    EXPECT_EQ(decoded.source, "");
    EXPECT_TRUE(decoded.subject.has_value());
    EXPECT_EQ(*decoded.subject, "");
    EXPECT_EQ(env, decoded);
}

TEST(MeshEnvelopeCodecTest, LargePayloadRoundTrip) {
    // Forces the encoder past its 256-byte stack buffer into the heap pass.
    auto env = minimalEnvelope();
    env.data.assign(4096, 0xAB);
    auto bytes = SCE::Mesh::encodeEnvelope(env);
    ASSERT_FALSE(bytes.empty());

    MeshEnvelope decoded;
    ASSERT_TRUE(SCE::Mesh::decodeEnvelope(bytes.data(), bytes.size(), decoded));
    EXPECT_EQ(decoded.data.size(), 4096u);
    EXPECT_EQ(env, decoded);
}

// ── Forward-compatibility ───────────────────────────────────────────────

TEST(MeshEnvelopeCodecTest, UnknownIntegerKeySkipped) {
    // Hand-craft a CBOR map that contains all required keys plus an unknown
    // key (99) carrying a text string. A correctly forward-compatible
    // decoder skips the unknown entry and decodes the rest.
    //
    // CBOR map of 7 entries (a7) =
    //   0:bytes16 1:"a" 2:"b" 3:1(uint) 4:1(uint) 5:bytes(0) 99:"future"
    //
    // Wire bytes built explicitly so the test would catch a regression in
    // tinycbor's advance-past-unknown behavior.
    std::vector<uint8_t> wire;
    wire.push_back(0xA7);                          // map(7)

    // 0: id (bytes16)
    wire.push_back(0x00);                          // key 0
    wire.push_back(0x50);                          // bytes(16)
    for (auto b : kSampleId) wire.push_back(b);

    // 1: source = "a"
    wire.push_back(0x01);
    wire.push_back(0x61); wire.push_back('a');

    // 2: type = "b"
    wire.push_back(0x02);
    wire.push_back(0x61); wire.push_back('b');

    // 3: pattern = 1 (FireForget)
    wire.push_back(0x03);
    wire.push_back(0x01);

    // 4: datacontenttype = 0 (None)
    wire.push_back(0x04);
    wire.push_back(0x00);

    // 5: data = empty bytes
    wire.push_back(0x05);
    wire.push_back(0x40);                          // bytes(0)

    // 99: unknown key with text string "future" (6 bytes)
    wire.push_back(0x18); wire.push_back(99);      // uint(99) — single byte form
    wire.push_back(0x66); wire.push_back('f'); wire.push_back('u');
    wire.push_back('t');  wire.push_back('u'); wire.push_back('r');
    wire.push_back('e');

    MeshEnvelope decoded;
    ASSERT_TRUE(SCE::Mesh::decodeEnvelope(wire.data(), wire.size(), decoded));
    EXPECT_EQ(decoded.source, "a");
    EXPECT_EQ(decoded.type, "b");
    EXPECT_EQ(decoded.pattern, PatternKind::FireForget);
    EXPECT_TRUE(decoded.data.empty());
}

// ── Negative cases ──────────────────────────────────────────────────────

TEST(MeshEnvelopeCodecTest, MissingRequiredKeyFails) {
    // Map of 5 entries: drop the required 'data' field (key 5).
    std::vector<uint8_t> wire;
    wire.push_back(0xA5);  // map(5)

    wire.push_back(0x00);
    wire.push_back(0x50);
    for (auto b : kSampleId) wire.push_back(b);

    wire.push_back(0x01); wire.push_back(0x61); wire.push_back('a');
    wire.push_back(0x02); wire.push_back(0x61); wire.push_back('b');
    wire.push_back(0x03); wire.push_back(0x01);
    wire.push_back(0x04); wire.push_back(0x00);

    MeshEnvelope decoded;
    EXPECT_FALSE(SCE::Mesh::decodeEnvelope(wire.data(), wire.size(), decoded));
}

TEST(MeshEnvelopeCodecTest, EmptyInputFails) {
    MeshEnvelope decoded;
    EXPECT_FALSE(SCE::Mesh::decodeEnvelope(nullptr, 0, decoded));

    uint8_t dummy = 0;
    EXPECT_FALSE(SCE::Mesh::decodeEnvelope(&dummy, 0, decoded));
}

TEST(MeshEnvelopeCodecTest, NonMapInputFails) {
    // CBOR uint(42) at root — well-formed but not a map.
    uint8_t wire[] = {0x18, 0x2A};
    MeshEnvelope decoded;
    EXPECT_FALSE(SCE::Mesh::decodeEnvelope(wire, sizeof(wire), decoded));
}

TEST(MeshEnvelopeCodecTest, CorruptBytesFail) {
    // Map header claims 3 entries but stream ends mid-key.
    uint8_t wire[] = {0xA3, 0x00};
    MeshEnvelope decoded;
    EXPECT_FALSE(SCE::Mesh::decodeEnvelope(wire, sizeof(wire), decoded));
}

TEST(MeshEnvelopeCodecTest, InvalidPatternKindRejected) {
    // pattern=99 is out of the 1-9 valid range.
    std::vector<uint8_t> wire;
    wire.push_back(0xA6);
    // key 0: id
    wire.push_back(0x00); wire.push_back(0x50);
    for (auto b : kSampleId) wire.push_back(b);
    // key 1: source
    wire.push_back(0x01); wire.push_back(0x61); wire.push_back('x');
    // key 2: type
    wire.push_back(0x02); wire.push_back(0x61); wire.push_back('y');
    // key 3: pattern = 99 (INVALID)
    wire.push_back(0x03); wire.push_back(0x18); wire.push_back(99);
    // key 4: datacontenttype = 0
    wire.push_back(0x04); wire.push_back(0x00);
    // key 5: data = empty
    wire.push_back(0x05); wire.push_back(0x40);

    MeshEnvelope decoded;
    EXPECT_FALSE(SCE::Mesh::decodeEnvelope(wire.data(), wire.size(), decoded));
}

TEST(MeshEnvelopeCodecTest, InvalidPayloadCodecRejected) {
    std::vector<uint8_t> wire;
    wire.push_back(0xA6);
    wire.push_back(0x00); wire.push_back(0x50);
    for (auto b : kSampleId) wire.push_back(b);
    wire.push_back(0x01); wire.push_back(0x61); wire.push_back('x');
    wire.push_back(0x02); wire.push_back(0x61); wire.push_back('y');
    wire.push_back(0x03); wire.push_back(0x01);  // FireForget
    // key 4: datacontenttype = 55 (INVALID)
    wire.push_back(0x04); wire.push_back(0x18); wire.push_back(55);
    wire.push_back(0x05); wire.push_back(0x40);

    MeshEnvelope decoded;
    EXPECT_FALSE(SCE::Mesh::decodeEnvelope(wire.data(), wire.size(), decoded));
}

TEST(MeshEnvelopeCodecTest, InvalidRpcStatusRejected) {
    // RpcStatus has gaps (e.g. 2 is not defined). Inject rpc_status=2.
    auto env = minimalEnvelope();
    auto bytes = SCE::Mesh::encodeEnvelope(env);
    // Build a map with rpc_status=2 appended.
    std::vector<uint8_t> wire;
    wire.push_back(0xA7);  // map(7) = 6 required + 1 optional
    // Copy body from minimal (skip the 0xA6 header byte).
    wire.insert(wire.end(), bytes.begin() + 1, bytes.end());
    // key 10: rpc_status = 2 (INVALID — no such code)
    wire.push_back(0x0A); wire.push_back(0x02);

    MeshEnvelope decoded;
    EXPECT_FALSE(SCE::Mesh::decodeEnvelope(wire.data(), wire.size(), decoded));
}

// ── Wire-stability ──────────────────────────────────────────────────────
//
// Pinning the leading byte of a known minimal envelope ensures we did not
// silently drift to e.g. an indefinite-length map, which would still
// round-trip but break the canonical-CBOR contract.

TEST(MeshEnvelopeCodecTest, MinimalEnvelopeUsesDefiniteMap) {
    auto bytes = SCE::Mesh::encodeEnvelope(minimalEnvelope());
    ASSERT_FALSE(bytes.empty());
    // Major type 5 (map) + 6 entries — required-fields-only envelope.
    EXPECT_EQ(bytes[0], 0xA6);
}

TEST(MeshEnvelopeCodecTest, FullEnvelopeMapHeaderEntries) {
    auto bytes = SCE::Mesh::encodeEnvelope(fullEnvelope());
    ASSERT_FALSE(bytes.empty());
    // 6 required + 8 optionals = 14 entries (single-byte map header form).
    EXPECT_EQ(bytes[0], 0xAE);
}

TEST(MeshEnvelopeCodecTest, SequenceNoRoundTrip) {
    // §10.6.3 — sequence_no is an optional uint64 on CBOR key 14. Round-trip
    // covers the stamped case; OptionalsAbsentStayAbsent covers the absent
    // case for the default-ordered route.
    auto env = minimalEnvelope();
    env.sequence_no = 123456789ULL;

    auto bytes = SCE::Mesh::encodeEnvelope(env);
    ASSERT_FALSE(bytes.empty());

    MeshEnvelope decoded;
    ASSERT_TRUE(SCE::Mesh::decodeEnvelope(bytes.data(), bytes.size(), decoded));
    ASSERT_TRUE(decoded.sequence_no.has_value());
    EXPECT_EQ(*decoded.sequence_no, 123456789ULL);
    EXPECT_EQ(env, decoded);
}

TEST(MeshEnvelopeCodecTest, GoldenBytesForFixedEnvelope) {
    // A fully-deterministic envelope and the exact CBOR bytes it must
    // produce. Any change to the wire format — encoder bug, key reorder,
    // tinycbor regression — flips at least one byte and fails this test.
    //
    // Envelope: id=kSampleId, source="ecu", type="evt", FireForget+Json,
    //           data={0xAA,0xBB}, no optionals.
    MeshEnvelope env{};
    env.id = kSampleId;
    env.source = "ecu";
    env.type = "evt";
    env.pattern = PatternKind::FireForget;
    env.datacontenttype = PayloadCodec::Json;
    env.data = {0xAA, 0xBB};

    const std::vector<uint8_t> expected = {
        0xA6,                                            // map(6)
        0x00, 0x50,                                      // key 0, bstr(16)
        0x01, 0x82, 0xb1, 0x4d, 0xa3, 0x5c, 0x70, 0x12,
        0xb4, 0xde, 0xf0, 0x42, 0x9a, 0x88, 0x77, 0x66,  // kSampleId
        0x01, 0x63, 'e', 'c', 'u',                       // key 1, tstr(3) "ecu"
        0x02, 0x63, 'e', 'v', 't',                       // key 2, tstr(3) "evt"
        0x03, 0x01,                                      // key 3, uint(1) FireForget
        0x04, 0x01,                                      // key 4, uint(1) Json
        0x05, 0x42, 0xAA, 0xBB,                          // key 5, bstr(2) {AA,BB}
    };

    auto bytes = SCE::Mesh::encodeEnvelope(env);
    EXPECT_EQ(bytes, expected);
}
