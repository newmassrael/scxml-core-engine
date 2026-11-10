// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Derived numbers the C++ printer prints from their plan (docs/adr/0081 step
// 4) that the encode can refuse, compiled and run.
//
// The headers are generated at build time from tests/forge/resources:
// codec_length_shared (one length of two byte strings always there),
// codec_length_stated (one length of two gated byte strings, which the value
// states where neither is there) and codec_encoded_length_checked (the encoded
// length of a codec_length_shared body, in a 16-bit field). Each frame and
// each refusal is the one the Rust encode's `combine` gives the same value:
// the first measure always there fixes the number and every other one there
// must say the same; where every measure may be absent the first one there
// does; and a stated number beside content must be what the content says. A
// refusal is made before the first byte, so the sink holds nothing.

#include "codec_encoded_length_checked.h"
#include "codec_length_shared.h"
#include "codec_length_stated.h"

#include <cstdint>
#include <cstdio>
#include <optional>
#include <vector>

namespace {

int failures = 0;

#define CHECK(cond, what)                                                                                              \
    do {                                                                                                               \
        if (!(cond)) {                                                                                                 \
            ++failures;                                                                                                \
            std::fprintf(stderr, "FAIL %s:%d %s\n", __FILE__, __LINE__, what);                                         \
        }                                                                                                              \
    } while (0)

using ::SCE::Forge::CodecError;
using ::SCE::Forge::SceCursor;
using Bytes = ::SCE::Forge::Bytes<255>;
using Shared = ::SCE::Generated::CodecLengthShared::CodecLengthShared;
using Stated = ::SCE::Generated::CodecLengthStated::CodecLengthStated;
using Checked = ::SCE::Generated::CodecEncodedLengthChecked::CodecEncodedLengthChecked;

const std::vector<std::uint8_t> kTwo{0x01, 0x02};
const std::vector<std::uint8_t> kOther{0x03, 0x04};
const std::vector<std::uint8_t> kOne{0x03};

// Decode `frame` as `Codec` and write the value back.
template <class Codec> std::optional<std::vector<std::uint8_t>> round_trip(const std::vector<std::uint8_t> &frame) {
    SceCursor cursor(frame.data(), frame.size());
    const std::optional<Codec> decoded = Codec::decode(cursor);
    if (!decoded.has_value() || cursor.remaining() != 0) {
        return std::nullopt;
    }
    return decoded->encode_to_vec();
}

// What `value` answers when it is encoded into an empty sink, and the bytes the
// sink then holds.
template <class Codec> std::optional<CodecError> encoded(const Codec &value, std::vector<std::uint8_t> &bytes) {
    ::SCE::Forge::VectorSink sink(bytes);
    return value.encode(sink);
}

// A refused value leaves the sink untouched and answers no bytes.
template <class Codec> void refused_as(const Codec &value, CodecError expected, const char *what) {
    std::vector<std::uint8_t> bytes;
    CHECK(encoded(value, bytes) == std::optional<CodecError>(expected), what);
    CHECK(bytes.empty(), "a refusal leaves the sink untouched");
    CHECK(!value.encode_to_vec().has_value(), "encode_to_vec answers no bytes for a refused value");
}

Bytes bytes_of(const std::vector<std::uint8_t> &content) {
    return *Bytes::make(content);
}

// A frame is written back byte for byte, the length the first byte string's.
void a_shared_length_frame_is_written_back() {
    const std::vector<std::vector<std::uint8_t>> frames{
        {0x02, 'a', 'b', 'c', 'd'},
        {0x00},
    };
    for (const auto &frame : frames) {
        CHECK(round_trip<Shared>(frame) == frame, "a shared-length frame is written back byte for byte");
    }
}

// Two byte strings of one length are written with it; of two lengths, refused.
void two_byte_strings_of_one_length_are_held_to_it() {
    const Shared agreeing(bytes_of(kTwo), bytes_of(kOther));
    const std::vector<std::uint8_t> expected{0x02, 0x01, 0x02, 0x03, 0x04};
    CHECK(agreeing.encode_to_vec() == std::optional<std::vector<std::uint8_t>>(expected),
          "byte strings of one length are written after it");
    refused_as(Shared(bytes_of(kTwo), bytes_of(kOne)), CodecError::InconsistentSource,
               "byte strings of two lengths are refused as inconsistent");
}

// A frame of either, both or neither byte string is written back byte for
// byte; where neither is there, the value states the length the frame held.
void a_stated_length_frame_is_written_back() {
    const std::vector<std::vector<std::uint8_t>> frames{
        {0x01, 0x02, 'a', 'b'},
        {0x02, 0x01, 'z'},
        {0x03, 0x01, 'x', 'y'},
        {0x00, 0x05},
    };
    for (const auto &frame : frames) {
        CHECK(round_trip<Stated>(frame) == frame, "a stated-length frame is written back byte for byte");
    }
    const std::vector<std::uint8_t> neither{0x00, 0x05};
    SceCursor cursor(neither.data(), neither.size());
    const std::optional<Stated> decoded = Stated::decode(cursor);
    CHECK(decoded.has_value() && decoded->span_len() == std::optional<std::uint8_t>(5),
          "the value states the length where no content is there");
    const std::vector<std::uint8_t> with{0x01, 0x02, 'a', 'b'};
    SceCursor with_cursor(with.data(), with.size());
    const std::optional<Stated> with_content = Stated::decode(with_cursor);
    CHECK(with_content.has_value() && !with_content->span_len().has_value(),
          "the value states no length where content says it");
}

// The first byte string there says the length, the other is held to it, and a
// length the value states is held to it where there is content.
void a_stated_length_is_held_to_the_content_there() {
    const Stated both(0x00, std::nullopt, Bytes::make(kTwo), Bytes::make(kOther));
    const std::vector<std::uint8_t> both_written{0x03, 0x02, 0x01, 0x02, 0x03, 0x04};
    CHECK(both.encode_to_vec() == std::optional<std::vector<std::uint8_t>>(both_written),
          "two byte strings of one length are written after it");
    refused_as(Stated(0x00, std::nullopt, Bytes::make(kTwo), Bytes::make(kOne)), CodecError::InconsistentSource,
               "two byte strings there of two lengths are refused as inconsistent");

    const Stated beside(0x00, std::optional<std::uint8_t>(2), Bytes::make(kTwo), std::nullopt);
    const std::vector<std::uint8_t> beside_written{0x01, 0x02, 0x01, 0x02};
    CHECK(beside.encode_to_vec() == std::optional<std::vector<std::uint8_t>>(beside_written),
          "a stated length the content agrees with is written");
    refused_as(Stated(0x00, std::optional<std::uint8_t>(7), Bytes::make(kTwo), std::nullopt),
               CodecError::InconsistentSource, "a stated length the content contradicts is refused as inconsistent");

    const Stated alone(0x00, std::optional<std::uint8_t>(9), std::nullopt, std::nullopt);
    const std::vector<std::uint8_t> alone_written{0x00, 0x09};
    CHECK(alone.encode_to_vec() == std::optional<std::vector<std::uint8_t>>(alone_written),
          "a stated length with no content is written as stated");
    const Stated nothing(0x00, std::nullopt, std::nullopt, std::nullopt);
    const std::vector<std::uint8_t> nothing_written{0x00, 0x00};
    CHECK(nothing.encode_to_vec() == std::optional<std::vector<std::uint8_t>>(nothing_written),
          "no length and no content is written as zero");
}

// The encoded length is counted from the body, and a body that refuses refuses
// the parent before its first byte.
void an_encoded_length_passes_the_body_s_refusal_on() {
    const std::vector<std::uint8_t> frame{0x00, 0x03, 0x01, 0x01, 0x02};
    CHECK(round_trip<Checked>(frame) == frame, "an encoded-length frame is written back byte for byte");
    const std::vector<std::uint8_t> a{0x0A};
    const std::vector<std::uint8_t> b{0x0B};
    const Checked agreeing(Shared(bytes_of(a), bytes_of(b)));
    const std::vector<std::uint8_t> expected{0x00, 0x03, 0x01, 0x0A, 0x0B};
    CHECK(agreeing.encode_to_vec() == std::optional<std::vector<std::uint8_t>>(expected),
          "the length is the count of the bytes the body encodes to");
    refused_as(Checked(Shared(bytes_of(kTwo), bytes_of(kOne))), CodecError::InconsistentSource,
               "a body that refuses refuses the parent");
}

}  // namespace

int main() {
    a_shared_length_frame_is_written_back();
    two_byte_strings_of_one_length_are_held_to_it();
    a_stated_length_frame_is_written_back();
    a_stated_length_is_held_to_the_content_there();
    an_encoded_length_passes_the_body_s_refusal_on();
    if (failures != 0) {
        std::fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    return 0;
}
