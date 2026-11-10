// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// What the C++ printer prints from a plan (docs/adr/0081 step 4) that no case
// of the catalog can state, compiled and run. Every frame and refusal a catalog
// case states is held in the conformance harness (numerical_reference.json,
// written by tests/forge/conformance/binary_model.py), the one source of byte
// cases; what is here is what a case cannot hold, each with why.
//
// The headers are generated at build time from tests/forge/resources and
// backends/rust/forge-conformance/fixtures:
//
// codec_length_stated (one length of two gated byte strings, which the value
// states where neither is there): a value that states the length beside
// content that says the same, and one that states neither, are each written,
// as the Rust encode's `combine` writes them. A source case is held to its
// decode presenting the value again (law 1 of docs/adr/0058 §5), and the decode
// of either frame presents another value: no length where content says it, and
// the length zero where the frame holds it. Neither is a refusal either.
//
// codec_group_enum (a closed enum in a group of one flag): a frame whose enum
// value the enum does not declare does not decode. The catalog's decode
// refusals (`rejects`) name no undeclared enum value, which only the decode of
// a closed enum gives.
//
// codec_some_arms_say_forward (backends/rust/forge-conformance/fixtures) hands
// its flag input on to a body that says it in some of its values only, and
// answers the input as the body does. The body is a variant whose caller hands
// its tag, which the catalog's model and its cases do not read yet; the
// variant's family is the Rust arm's (forge_source_round_trip) and moves into
// the catalog with that reading.

#include "codec_group_enum.h"
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
using Stated = ::SCE::Generated::CodecLengthStated::CodecLengthStated;
using GroupEnum = ::SCE::Generated::CodecGroupEnum::CodecGroupEnum;

// A length the value states beside content that says the same is written, and
// no length with no content is written as zero; the decode of either frame
// presents another value, so no source case states them.
void a_stated_length_the_content_agrees_with_is_written() {
    const std::vector<std::uint8_t> two{0x01, 0x02};
    const Stated beside(0x00, std::optional<std::uint8_t>(2), Bytes::make(two), std::nullopt);
    const std::vector<std::uint8_t> beside_written{0x01, 0x02, 0x01, 0x02};
    CHECK(beside.encode_to_vec() == std::optional<std::vector<std::uint8_t>>(beside_written),
          "a stated length the content agrees with is written");
    const Stated nothing(0x00, std::nullopt, std::nullopt, std::nullopt);
    const std::vector<std::uint8_t> nothing_written{0x00, 0x00};
    CHECK(nothing.encode_to_vec() == std::optional<std::vector<std::uint8_t>>(nothing_written),
          "no length and no content is written as zero");
}

// A value the closed enum does not declare is refused on decode, and the
// cursor records why.
void an_undeclared_enum_value_does_not_decode() {
    const std::vector<std::uint8_t> undeclared{0x01, 0x07, 0x05};
    SceCursor cursor(undeclared.data(), undeclared.size());
    CHECK(!GroupEnum::decode(cursor).has_value(), "an undeclared enum value does not decode");
    CHECK(cursor.last_refusal() == std::optional<CodecError>(CodecError::UndeclaredEnumValue),
          "the cursor records the undeclared enum value");
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
    a_stated_length_the_content_agrees_with_is_written();
    an_undeclared_enum_value_does_not_decode();
    an_input_a_body_says_in_part_is_forwarded();
    if (failures != 0) {
        std::fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    return 0;
}
