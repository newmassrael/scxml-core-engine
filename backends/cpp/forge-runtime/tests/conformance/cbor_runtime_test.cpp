// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// sce/forge/cbor.h — the CBOR items a `sce:encoding="cbor"` codec reads and
// writes (SCE_FORGE.md §4.6.1). The same four properties the Rust runtime's
// own tests pin (backends/rust/forge-runtime/src/cbor.rs): shortest heads on
// write, any valid head length on read, refusal of what an entry cannot be,
// and a skip that takes an unknown value whole and refuses one too deep.
// What a generated codec does with these is the conformance harness's
// (codec_cbor_map), which this file does not repeat.

#include "sce/forge/cbor.h"

#include <cstdint>
#include <cstdio>
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

namespace cbor = ::SCE::Forge::Cbor;
using ::SCE::Forge::SceCursor;
using ::SCE::Forge::VectorSink;
using Bytes = std::vector<std::uint8_t>;

template <typename F> Bytes written(F write) {
    Bytes out;
    VectorSink sink(out);
    CHECK(!write(sink).has_value(), "a write into a growable sink refused");
    return out;
}

SceCursor cursor_over(const Bytes &b) {
    return SceCursor(b.data(), b.size());
}

void every_head_is_written_in_its_shortest_form() {
    // RFC 8949 Appendix A: 0, 23, 24, 255, 256, 65535, 65536, 2^32.
    CHECK(written([](auto &w) { return cbor::write_uint(w, 0); }) == Bytes({0x00}), "0");
    CHECK(written([](auto &w) { return cbor::write_uint(w, 23); }) == Bytes({0x17}), "23");
    CHECK(written([](auto &w) { return cbor::write_uint(w, 24); }) == Bytes({0x18, 0x18}), "24");
    CHECK(written([](auto &w) { return cbor::write_uint(w, 255); }) == Bytes({0x18, 0xff}), "255");
    CHECK(written([](auto &w) { return cbor::write_uint(w, 256); }) == Bytes({0x19, 0x01, 0x00}), "256");
    CHECK(written([](auto &w) { return cbor::write_uint(w, 65535); }) == Bytes({0x19, 0xff, 0xff}), "65535");
    CHECK(written([](auto &w) { return cbor::write_uint(w, 65536); }) == Bytes({0x1a, 0x00, 0x01, 0x00, 0x00}),
          "65536");
    CHECK(written([](auto &w) { return cbor::write_uint(w, std::uint64_t{1} << 32); }) ==
              Bytes({0x1b, 0, 0, 0, 1, 0, 0, 0, 0}),
          "2^32");
    CHECK(written([](auto &w) { return cbor::write_text(w, std::string("a")); }) == Bytes({0x61, 'a'}), "text");
    CHECK(written([](auto &w) { return cbor::write_bytes(w, Bytes({1, 2})); }) == Bytes({0x42, 1, 2}), "bytes");
    CHECK(written([](auto &w) { return cbor::write_bool(w, true); }) == Bytes({0xf5}), "true");
    CHECK(written([](auto &w) { return cbor::write_map_head(w, 2); }) == Bytes({0xa2}), "map head");
}

void a_head_is_read_in_any_valid_length() {
    // 1 written in its 2-byte form still reads as 1.
    const Bytes two{0x18, 0x01};
    auto c = cursor_over(two);
    CHECK(cbor::read_uint(c) == std::optional<std::uint64_t>(1), "1 in a 2-byte head");
    const Bytes nine{0x1b, 0, 0, 0, 1, 0, 0, 0, 0};
    auto d = cursor_over(nine);
    CHECK(cbor::read_uint(d) == std::optional<std::uint64_t>(std::uint64_t{1} << 32), "2^32");
}

void what_the_codec_cannot_read_is_refused() {
    // Indefinite-length map, reserved additional information, another major
    // type, a width the entry cannot hold, a wrong exact length, a text past
    // its bound, a text that is not UTF-8.
    for (const Bytes &b : {Bytes{0xbf}, Bytes{0x1c}, Bytes{0x61, 'a'}}) {
        auto c = cursor_over(b);
        CHECK(!cbor::read_uint(c).has_value(), "an item that is no unsigned integer");
    }
    const Bytes indefinite{0xbf};
    auto m = cursor_over(indefinite);
    CHECK(!cbor::read_map_len(m).has_value(), "indefinite-length map");
    const Bytes wide{0x19, 0x01, 0x00};
    auto u = cursor_over(wide);
    CHECK(!cbor::read_uint_upto(u, 255).has_value(), "256 in a uint8");
    const Bytes short_bytes{0x42, 1, 2};
    auto s = cursor_over(short_bytes);
    CHECK(!cbor::read_bytes_exact(s, 16).has_value(), "two bytes where sixteen are declared");
    const Bytes long_text{0x62, 'a', 'b'};
    auto t = cursor_over(long_text);
    CHECK(!cbor::read_text(t, 1).has_value(), "a text past its bound");
    const Bytes not_utf8{0x61, 0xff};
    auto x = cursor_over(not_utf8);
    CHECK(!cbor::read_text(x, std::nullopt).has_value(), "a text that is not UTF-8");
}

void an_unknown_value_is_skipped_whole_and_a_deep_one_refused() {
    // {1: [2, {3: h'00'}]} then 7: the skip lands on the 7.
    const Bytes nested{0xa1, 0x01, 0x82, 0x02, 0xa1, 0x03, 0x41, 0x00, 0x07};
    auto c = cursor_over(nested);
    CHECK(cbor::skip(c), "a nested value is skipped");
    CHECK(cbor::read_uint(c) == std::optional<std::uint64_t>(7), "the skip lands on the next item");

    Bytes deep(20, 0x81);
    deep.push_back(0x00);
    auto d = cursor_over(deep);
    CHECK(!cbor::skip(d), "twenty nested arrays are past the skip depth");
}

}  // namespace

int main() {
    every_head_is_written_in_its_shortest_form();
    a_head_is_read_in_any_valid_length();
    what_the_codec_cannot_read_is_refused();
    an_unknown_value_is_skipped_whole_and_a_deep_one_refused();
    if (failures != 0) {
        std::fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    std::puts("cbor_runtime_test: all checks passed");
    return 0;
}
