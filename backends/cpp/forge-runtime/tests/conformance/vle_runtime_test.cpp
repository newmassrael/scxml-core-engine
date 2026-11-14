// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// sce/forge/codec.h — the cursor's VLE readers on their own. The wire of a
// `vle_u<N>` field is a ZInt64 at every width, as `zenoh` and `zenoh-pico` read
// it, and the type bounds the value: a narrow read accepts every encoding of a
// value it holds, the padded ones included, and refuses a value past its width
// (`last_vle_overflow()`), never a longer encoding. What a generated codec does
// with them is judged by the conformance harness (codec_vle_zint_u16, _u32 and
// _u64).

#include "sce/forge/codec.h"

#include <cstddef>
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

struct Accepted {
    std::vector<std::uint8_t> wire;
    std::uint64_t value;
};

void a_u16_reads_each_encoding_of_a_value_it_holds_alike() {
    const std::vector<Accepted> cases = {
        {{0x81, 0x80, 0x80, 0x00}, 1}, {{0x81, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x00}, 1},
        {{0xFF, 0x7F}, 0x3FFF},        {{0xFF, 0xFF, 0x80, 0x00}, 0x3FFF},
        {{0x80, 0x80, 0x01}, 0x4000},  {{0x80, 0x80, 0x81, 0x80, 0x00}, 0x4000},
        {{0xFF, 0xFF, 0x03}, 0xFFFF},  {{0xFF, 0xFF, 0x83, 0x80, 0x80, 0x80, 0x80, 0x80, 0x00}, 0xFFFF},
    };
    for (const Accepted &c : cases) {
        SceCursor cursor(c.wire.data(), c.wire.size());
        const std::optional<std::uint64_t> read = cursor.read_vle_u16();
        CHECK(read.has_value() && *read == c.value, "a u16 reads every encoding of a value it holds");
        CHECK(!cursor.last_vle_overflow(), "an accepted read is no overflow");
        CHECK(cursor.remaining() == 0, "the read takes the whole ZInt");
    }
}

void a_u32_and_a_u64_read_a_padded_encoding_of_a_value_they_hold() {
    const std::vector<std::uint8_t> nine = {0x81, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x00};
    SceCursor u32(nine.data(), nine.size());
    const std::optional<std::uint64_t> read32 = u32.read_vle_u32();
    CHECK(read32.has_value() && *read32 == 1, "a u32 reads 1 in nine bytes");
    SceCursor u64(nine.data(), nine.size());
    const std::optional<std::uint64_t> read64 = u64.read_vle_u64();
    CHECK(read64.has_value() && *read64 == 1, "a u64 reads 1 in nine bytes");
    const std::vector<std::uint8_t> padded_max = {0xFF, 0xFF, 0xFF, 0xFF, 0x8F, 0x80, 0x80, 0x80, 0x00};
    SceCursor max32(padded_max.data(), padded_max.size());
    const std::optional<std::uint64_t> read_max = max32.read_vle_u32();
    CHECK(read_max.has_value() && *read_max == 0xFFFFFFFFULL, "a u32 reads its largest value padded to nine bytes");
    const std::vector<std::uint8_t> all_ones(9, 0xFF);
    SceCursor max64(all_ones.data(), all_ones.size());
    const std::optional<std::uint64_t> read_all = max64.read_vle_u64();
    CHECK(read_all.has_value() && *read_all == 0xFFFFFFFFFFFFFFFFULL, "a u64 reads its largest value in nine bytes");
}

void a_narrow_read_refuses_the_first_value_past_its_width() {
    const std::vector<std::uint8_t> past16 = {0x80, 0x80, 0x04};
    SceCursor u16(past16.data(), past16.size());
    CHECK(!u16.read_vle_u16().has_value() && u16.last_vle_overflow(), "65536 is past a u16");
    const std::vector<std::uint8_t> past32 = {0x80, 0x80, 0x80, 0x80, 0x10};
    SceCursor u32(past32.data(), past32.size());
    CHECK(!u32.read_vle_u32().has_value() && u32.last_vle_overflow(), "2^32 is past a u32");
    // A ninth byte with its high bit set is data, not a flag asking for a tenth.
    const std::vector<std::uint8_t> high = {0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x01};
    SceCursor high32(high.data(), high.size());
    CHECK(!high32.read_vle_u32().has_value() && high32.last_vle_overflow(), "2^63 is past a u32");
    SceCursor high64(high.data(), high.size());
    const std::optional<std::uint64_t> read = high64.read_vle_u64();
    CHECK(read.has_value() && *read == (std::uint64_t{1} << 63), "a ninth byte's high bit is bit 63");
    CHECK(high64.remaining() == 1, "the read stops after the ninth byte");
}

void a_chain_the_buffer_ends_inside_needs_more_bytes() {
    const std::vector<std::uint8_t> short_chain = {0x81, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80};
    SceCursor u16(short_chain.data(), short_chain.size());
    CHECK(!u16.read_vle_u16().has_value() && !u16.last_vle_overflow(), "a short chain is no overflow");
    SceCursor u64(short_chain.data(), short_chain.size());
    CHECK(!u64.read_vle_u64().has_value() && !u64.last_vle_overflow(), "a short chain is no overflow");
}

}  // namespace

int main() {
    a_u16_reads_each_encoding_of_a_value_it_holds_alike();
    a_u32_and_a_u64_read_a_padded_encoding_of_a_value_they_hold();
    a_narrow_read_refuses_the_first_value_past_its_width();
    a_chain_the_buffer_ends_inside_needs_more_bytes();
    if (failures != 0) {
        std::fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    std::puts("vle_runtime_test: all checks passed");
    return 0;
}
