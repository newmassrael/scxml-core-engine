// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// A variant whose caller hands its tag, printed from its plan (docs/adr/0081
// step 4), and the codecs that hold one, compiled and run.
//
// The headers are generated at build time from the documents the Rust arm runs
// (backends/rust/forge-conformance/fixtures/codec_source_parent_tag.scxml and
// codec_every_arm_says_parent.scxml, with the codecs they import), and this
// file holds the C++ codecs to the frames
// backends/rust/forge-conformance/tests/forge_source_round_trip.rs holds the
// Rust ones to.
//
// codec_source_leaf has no tag of its own: its parent derives the flag
// `hdr.kind` (bit 0) from the arm it holds, 0 for the session open arm (a
// 16-bit version) and 1 for the session close arm (an 8-bit reason), and hands
// the decode what it read there. codec_every_arm_says_parent writes the flag
// `header.M` (bit 6) as the arm's tag and `header.N` (bit 5) as the arm says
// its flag input. codec_some_arms_say_parent holds the same flags over a
// variant one of whose arms says the input: in a value of that arm `N` is held
// to it, and the encode refuses one that says otherwise before its first byte
// (docs/adr/0065 §6 R4e); in a value of the other arm `N` is the value's own.

#include "codec_every_arm_says_parent.h"
#include "codec_some_arms_say_parent.h"
#include "codec_source_parent_tag.h"

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

using ::SCE::Forge::SceCursor;
using Leaf = ::SCE::Generated::CodecSourceLeaf::CodecSourceLeaf;
using ParentTag = ::SCE::Generated::CodecSourceParentTag::CodecSourceParentTag;
using EveryArmSays = ::SCE::Generated::CodecEveryArmSays::CodecEveryArmSays;
using EveryArmSaysParent = ::SCE::Generated::CodecEveryArmSaysParent::CodecEveryArmSaysParent;
using SomeArmsSayParent = ::SCE::Generated::CodecSomeArmsSayParent::CodecSomeArmsSayParent;

// Decode `frame` as `Codec` and write the value back.
template <class Codec> std::optional<std::vector<std::uint8_t>> round_trip(const std::vector<std::uint8_t> &frame) {
    SceCursor cursor(frame.data(), frame.size());
    const std::optional<Codec> decoded = Codec::decode(cursor);
    if (!decoded.has_value() || cursor.remaining() != 0) {
        return std::nullopt;
    }
    return decoded->encode_to_vec();
}

// A frame the parent decodes is written back byte for byte, its flag the tag
// of the arm the leaf holds.
void a_parent_writes_the_tag_of_the_arm_its_leaf_holds() {
    const std::vector<std::vector<std::uint8_t>> frames{
        {0x00, 0x00, 0x07},
        {0x80, 0x00, 0x07},
        {0x01, 0x04},
        {0x81, 0x04},
    };
    for (const auto &frame : frames) {
        CHECK(round_trip<ParentTag>(frame) == frame, "a parent-tag frame is written back byte for byte");
    }
}

// The leaf answers the tag it was decoded under, which is the one its parent
// writes for the arm it holds.
void a_leaf_answers_the_tag_it_was_decoded_under() {
    const std::vector<std::uint8_t> open{0x00, 0x07};
    SceCursor open_cursor(open.data(), open.size());
    const std::optional<Leaf> opened = Leaf::decode(open_cursor, 0);
    CHECK(opened.has_value() && opened->tag() == 0, "the open arm answers tag 0");

    const std::vector<std::uint8_t> close{0x04};
    SceCursor close_cursor(close.data(), close.size());
    const std::optional<Leaf> closed = Leaf::decode(close_cursor, 1);
    CHECK(closed.has_value() && closed->tag() == 1, "the close arm answers tag 1");
    CHECK(closed.has_value() && closed->encode_to_vec() == close, "the leaf writes its arm alone, not its tag");
}

// A flag every arm says, at opposite polarities, and the arm's tag, both
// derived into the parent's header.
void a_parent_writes_the_flag_every_arm_says() {
    const std::vector<std::vector<std::uint8_t>> frames{
        {0x60, 7, 1, 'a'},
        {0x40, 7},
        {0x00, 1, 2},
        {0x20},
    };
    for (const auto &frame : frames) {
        CHECK(round_trip<EveryArmSaysParent>(frame) == frame, "an every-arm-says frame is written back byte for byte");
    }
}

// A frame of either arm is written back byte for byte: `N` as the arm that
// says it says it, and as the value's own in the arm that does not.
void a_parent_writes_the_flag_some_arms_say() {
    const std::vector<std::vector<std::uint8_t>> frames{
        {0x60, 7, 1, 'a'},
        {0x40, 7},
        {0x00, 7},
        {0x20, 7},
    };
    for (const auto &frame : frames) {
        CHECK(round_trip<SomeArmsSayParent>(frame) == frame, "a some-arms-say frame is written back byte for byte");
    }
}

// A value whose `N` says otherwise than the arm that says it is refused before
// the first byte, and the sink holds nothing.
void a_flag_the_arm_says_otherwise_is_refused_before_the_first_byte() {
    const std::vector<std::uint8_t> frame{0x40, 7};
    SceCursor cursor(frame.data(), frame.size());
    const std::optional<SomeArmsSayParent> decoded = SomeArmsSayParent::decode(cursor);
    CHECK(decoded.has_value(), "the near arm without a suffix decodes");
    if (!decoded.has_value()) {
        return;
    }
    // `N` set, while the arm holds no suffix and so says it clear.
    const SomeArmsSayParent contradicted(static_cast<std::uint8_t>(0x60), decoded->body());
    std::vector<std::uint8_t> bytes;
    ::SCE::Forge::VectorSink sink(bytes);
    const std::optional<::SCE::Forge::CodecError> refused = contradicted.encode(sink);
    CHECK(refused == std::optional<::SCE::Forge::CodecError>(::SCE::Forge::CodecError::PresentIfMismatch),
          "the encode refuses a flag the arm says otherwise");
    CHECK(bytes.empty(), "a refusal leaves the sink untouched");
    CHECK(!contradicted.encode_to_vec().has_value(), "encode_to_vec answers no bytes for a refused value");
}

}  // namespace

int main() {
    a_parent_writes_the_tag_of_the_arm_its_leaf_holds();
    a_leaf_answers_the_tag_it_was_decoded_under();
    a_parent_writes_the_flag_every_arm_says();
    a_parent_writes_the_flag_some_arms_say();
    a_flag_the_arm_says_otherwise_is_refused_before_the_first_byte();
    if (failures != 0) {
        std::fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    std::puts("caller_tag_variant_generated_test: OK");
    return 0;
}
