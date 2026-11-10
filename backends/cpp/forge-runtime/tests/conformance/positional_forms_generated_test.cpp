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
//
// codec_group_enum holds an enum and an integer in one group of a flag: a frame
// with the group, or without it, is written back byte for byte, and a frame
// whose enum value the enum does not declare does not decode.
//
// codec_chain_has_input gates a field on a chain's membership or on a flag
// input the caller hands, which no group holds: the encode tests the entries
// of the chain the value presents, and refuses a value the gate contradicts.
//
// codec_some_arms_say_forward (backends/rust/forge-conformance/fixtures) hands
// its flag input on to a body that says it in some of its values only, and
// answers the input as the body does.

#include "codec_chain_has_input.h"
#include "codec_encoded_length_checked.h"
#include "codec_group_enum.h"
#include "codec_length_shared.h"
#include "codec_length_stated.h"
#include "codec_some_arms_say_forward.h"

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
using GroupEnum = ::SCE::Generated::CodecGroupEnum::CodecGroupEnum;

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

// An enum a group holds is read and written as its carrier; a value the enum
// does not declare is refused on decode.
void an_enum_a_group_holds_is_written_back() {
    const std::vector<std::vector<std::uint8_t>> frames{
        {0x01, 0x02, 0x05},
        {0x01, 0x40, 0x00},
        {0x00},
    };
    for (const auto &frame : frames) {
        CHECK(round_trip<GroupEnum>(frame) == frame, "a group-enum frame is written back byte for byte");
    }
    const std::vector<std::uint8_t> undeclared{0x01, 0x07, 0x05};
    SceCursor cursor(undeclared.data(), undeclared.size());
    CHECK(!GroupEnum::decode(cursor).has_value(), "an undeclared enum value does not decode");
    CHECK(cursor.last_refusal() == std::optional<CodecError>(CodecError::UndeclaredEnumValue),
          "the cursor records the undeclared enum value");
}

// A field gated on a chain's membership or on an input the caller hands is
// written where either holds; a value that holds it where neither does, or
// lacks it where one does, is refused before the first byte.
void a_membership_no_group_holds_is_tested_on_the_entries() {
    namespace Has = ::SCE::Generated::CodecChainHasInput;
    // One entry of kind 7 (its `more` flag clear), then `priority`.
    const std::vector<std::uint8_t> kind7{0x07, 0x00, 0x05, 0x09};
    SceCursor cursor(kind7.data(), kind7.size());
    const std::optional<Has::CodecChainHasInput> decoded = Has::CodecChainHasInput::decode(cursor, 0);
    CHECK(decoded.has_value() && cursor.remaining() == 0, "a frame whose chain holds kind 7 decodes");
    if (decoded.has_value()) {
        std::vector<std::uint8_t> bytes;
        ::SCE::Forge::VectorSink sink(bytes);
        CHECK(!Has::Wire::encode_source(*decoded, sink, 0).has_value(), "the value is written");
        CHECK(bytes == kind7, "the frame is written back byte for byte");
    }
    // One entry of kind 3, and no `priority`.
    const std::vector<std::uint8_t> kind3{0x03, 0x00, 0x05};
    SceCursor plain_cursor(kind3.data(), kind3.size());
    const std::optional<Has::CodecChainHasInput> plain = Has::CodecChainHasInput::decode(plain_cursor, 0);
    CHECK(plain.has_value() && plain_cursor.remaining() == 0, "a frame without the gate's kind decodes");
    if (plain.has_value()) {
        std::vector<std::uint8_t> bytes;
        ::SCE::Forge::VectorSink sink(bytes);
        CHECK(!Has::Wire::encode_source(*plain, sink, 0).has_value(), "the value is written where urgent is clear");
        CHECK(bytes == kind3, "the frame is written back byte for byte");
        std::vector<std::uint8_t> refused;
        ::SCE::Forge::VectorSink refused_sink(refused);
        CHECK(Has::Wire::encode_source(*plain, refused_sink, 1) ==
                  std::optional<CodecError>(CodecError::PresentIfMismatch),
              "a value without priority is refused where urgent is set");
        CHECK(refused.empty(), "a refusal leaves the sink untouched");
    }
}

// An input a body always there says in some of its values only is answered as
// the body answers it: the arm that reads it says it, the other says nothing.
// A frame of either arm is written back byte for byte.
void an_input_a_body_says_in_part_is_forwarded() {
    namespace Fwd = ::SCE::Generated::CodecSomeArmsSayForward;

    struct Case {
        std::vector<std::uint8_t> frame;
        std::uint8_t n;
        std::optional<std::uint8_t> said;
    };

    const std::vector<Case> cases{
        {{0x40, 7, 1, 'a'}, 1, std::optional<std::uint8_t>(1)},
        {{0x40, 7}, 0, std::optional<std::uint8_t>(0)},
        {{0x00, 7}, 0, std::nullopt},
    };
    for (const auto &c : cases) {
        SceCursor cursor(c.frame.data(), c.frame.size());
        const std::optional<Fwd::CodecSomeArmsSayForward> decoded = Fwd::CodecSomeArmsSayForward::decode(cursor, c.n);
        CHECK(decoded.has_value() && cursor.remaining() == 0, "a forwarding frame decodes");
        if (!decoded.has_value()) {
            continue;
        }
        CHECK(decoded->encode_to_vec() == c.frame, "a forwarding frame is written back byte for byte");
        CHECK(Fwd::input_n(*decoded) == c.said, "the input is what the body's arm says of it");
    }
}

}  // namespace

int main() {
    a_shared_length_frame_is_written_back();
    two_byte_strings_of_one_length_are_held_to_it();
    a_stated_length_frame_is_written_back();
    a_stated_length_is_held_to_the_content_there();
    an_encoded_length_passes_the_body_s_refusal_on();
    an_enum_a_group_holds_is_written_back();
    a_membership_no_group_holds_is_tested_on_the_entries();
    an_input_a_body_says_in_part_is_forwarded();
    if (failures != 0) {
        std::fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    return 0;
}
