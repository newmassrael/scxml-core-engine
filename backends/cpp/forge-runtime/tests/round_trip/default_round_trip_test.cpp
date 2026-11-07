// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// RFC variant-default-uniformity cpp half — runtime round-trip
// property test. Mirrors backends/rust/forge-conformance/tests/forge_default_round_trip.rs
// for the C++ backend: compiles the generated codecs into the test binary
// and runs `T{}.encode_to_vec().decode()` to prove the default-arm fallback defect
// cannot recur on the cpp branch either.
//
// Critical invariants verified:
//   1. Encoding a freshly default-constructed outer codec produces 3 wire
//      bytes (1-byte arm-B header + 2-byte uint16 payload).
//   2. The first byte's low 2 bits encode arm B's MID (0x02), so the
//      decoder's dispatch table routes back to arm B.
//   3. Decoding consumes every emitted byte (cursor.remaining() == 0).
//   4. The decoded variant's index() == 1 (arm B's std::variant slot).
//   5. Re-encoding the decoded value produces byte-equal output.

#include <cassert>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <vector>

#include "codec_default_marker_arm_a.h"
#include "codec_default_marker_arm_b.h"
#include "codec_embed_variant_dispatch.h"
#include "codec_quantity_temperature.h"
#include "codec_variant_default_marker.h"

using ::SCE::Forge::SceCursor;
using ::SCE::Generated::CodecEmbedVariantDispatch::CodecEmbedVariantDispatch;
using ::SCE::Generated::CodecQuantityTemperature::CodecQuantityTemperature;
using ::SCE::Generated::CodecVariantDefaultMarker::CodecVariantDefaultMarker;

static int failures = 0;

#define EXPECT(cond, msg)                                                                                              \
    do {                                                                                                               \
        if (!(cond)) {                                                                                                 \
            ++failures;                                                                                                \
            std::fprintf(stderr, "FAIL %s:%d %s\n", __FILE__, __LINE__, msg);                                          \
        }                                                                                                              \
    } while (0)

int main() {
    const CodecVariantDefaultMarker original{};
    const auto bytes = original.encode_to_vec();

    EXPECT(bytes.size() == 3, "default-emit + arm B (uint16 payload) must produce 3 wire bytes");
    EXPECT((bytes[0] & 0x03) == 0x02, "first byte low 2 bits must encode arm B's MID (0x02) — "
                                      "if 0 the inner Default zero-filled the header byte; "
                                      "β-cpp emission contract is broken");

    SceCursor cursor(bytes.data(), bytes.size());
    const auto decoded = CodecVariantDefaultMarker::decode(cursor);
    EXPECT(decoded.has_value(), "freshly-constructed codec must decode without error");
    EXPECT(cursor.remaining() == 0, "decode must consume every emitted byte; leftover means an "
                                    "arm-type mismatch on dispatch");

    if (decoded.has_value()) {
        // std::variant indices: 0 = ArmA, 1 = ArmB, 2 = Default catch-all.
        EXPECT(decoded->body().index() == 1, "round-trip must land in arm B (the marked-default arm); "
                                             "index 0 = legacy first-arm picked; index 2 = catch-all "
                                             "fallback (inner Default zero-filled the dispatch byte)");

        const auto re_encoded = decoded->encode_to_vec();
        EXPECT(re_encoded == bytes, "re-encoding the decoded value must produce byte-equal output");

        // Also verify the sink-based encode contract directly: round-trip
        // through a caller-owned VectorSink + SpanSink must produce
        // byte-identical output to the heap facade.
        constexpr std::size_t kMaxBytes = CodecVariantDefaultMarker::MAX_ENCODED_BYTES;
        {
            std::vector<std::uint8_t> via_vec_sink;
            via_vec_sink.reserve(kMaxBytes);
            ::SCE::Forge::VectorSink vs(via_vec_sink);
            EXPECT(!decoded->encode(vs).has_value(), "VectorSink-backed encode must succeed (infallible)");
            EXPECT(via_vec_sink == bytes, "VectorSink encode bytes must equal facade encode_to_vec output");
        }
        {
            std::vector<std::uint8_t> span_buf;
            span_buf.resize(kMaxBytes);
            ::SCE::Forge::SpanSink ss(span_buf.data(), span_buf.size());
            EXPECT(!decoded->encode(ss).has_value(),
                   "SpanSink-backed encode must succeed when buffer >= MAX_ENCODED_BYTES");
            span_buf.resize(ss.position());
            EXPECT(span_buf == bytes, "SpanSink encode prefix must equal facade encode_to_vec output");
        }
        // Bounded-buffer BufferOverflow path: a SpanSink sized strictly
        // smaller than the actual encoded length must surface the typed
        // error (validates the encode-side typed error contract).
        if (!bytes.empty()) {
            std::vector<std::uint8_t> tiny;
            tiny.resize(bytes.size() - 1);
            ::SCE::Forge::SpanSink ss(tiny.data(), tiny.size());
            auto err = decoded->encode(ss);
            EXPECT(err.has_value() && *err == ::SCE::Forge::CodecError::BufferOverflow,
                   "SpanSink encode must surface BufferOverflow when cap < bytes.size()");
        }
    }

    // A parent whose flag dispatches the embedded variant derives the flag
    // from the arm the body holds: the arm's tag, masked to the flag's bits,
    // or the tag the catch-all holds.
    {
        namespace Marker = ::SCE::Generated::CodecVariantDefaultMarker;
        using ArmA = ::SCE::Generated::CodecDefaultMarkerArmA::CodecDefaultMarkerArmA;
        using ArmB = ::SCE::Generated::CodecDefaultMarkerArmB::CodecDefaultMarkerArmB;
        const CodecEmbedVariantDispatch held_b{0x00, CodecVariantDefaultMarker{}};
        EXPECT(held_b.encode_to_vec() == (std::vector<std::uint8_t>{0x02, 0x02, 0x00, 0x00}),
               "the parent's kind is arm B's tag, whatever the header presents");
        const CodecEmbedVariantDispatch held_a{0xFC, CodecVariantDefaultMarker{Marker::CodecVariantDefaultMarkerVariant{
                                                         std::in_place_index<0>, ArmA{0x01, 0x07}}}};
        const std::vector<std::uint8_t> a_frame{0xFD, 0x01, 0x07};
        EXPECT(held_a.encode_to_vec() == a_frame, "arm A sets kind to 1 and keeps the header's other bits");
        const CodecEmbedVariantDispatch unknown{
            0x00, CodecVariantDefaultMarker{Marker::CodecVariantDefaultMarkerVariant{
                      Marker::CodecVariantDefaultMarkerDefault{0x07, ArmB{0x03, 0x0000}}}}};
        EXPECT(unknown.encode_to_vec() == (std::vector<std::uint8_t>{0x03, 0x03, 0x00, 0x00}),
               "the catch-all's tag, masked to kind's two bits, is the parent's kind");

        SceCursor dispatch_cursor(a_frame.data(), a_frame.size());
        const auto decoded_a = CodecEmbedVariantDispatch::decode(dispatch_cursor);
        EXPECT(decoded_a.has_value() && decoded_a->body().body().index() == 0,
               "the parent's kind dispatches the embedded variant to arm A");
        EXPECT(decoded_a.has_value() && decoded_a->encode_to_vec() == a_frame, "and encodes back to it");
    }

    // A physical quantity is the raw value the wire holds, read and set in
    // its unit through `raw * scale + offset`; the wire is the raw value.
    {
        CodecQuantityTemperature reading{static_cast<std::int8_t>(-10)};
        EXPECT(reading.raw_temp_phys() == -45.0, "raw -10 is -10 * 0.5 - 40 degrees");
        EXPECT(reading.encode_to_vec() == (std::vector<std::uint8_t>{0xF6}), "the wire holds the raw value");

        reading.set_raw_temp_phys(-20.0);
        EXPECT(reading.raw_temp() == 40, "-20 degrees is raw (-20 + 40) / 0.5");
        const std::vector<std::uint8_t> frame{0x28};
        EXPECT(reading.encode_to_vec() == frame, "and the wire holds the raw value set");

        SceCursor quantity_cursor(frame.data(), frame.size());
        const auto decoded_q = CodecQuantityTemperature::decode(quantity_cursor);
        EXPECT(decoded_q.has_value() && decoded_q->raw_temp_phys() == -20.0,
               "the decoded raw value reads back in degrees");
    }

    if (failures == 0) {
        std::puts("OK default_round_trip");
        return EXIT_SUCCESS;
    }
    std::fprintf(stderr, "%d assertion(s) failed\n", failures);
    return EXIT_FAILURE;
}
