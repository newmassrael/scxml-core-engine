// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// sce/forge/cbor.h — the CBOR items a `sce:encoding="cbor"` codec reads and
// writes (SCE_FORGE.md §4.6.1, docs/adr/0101). The properties the Rust
// runtime's own tests pin (backends/rust/forge-runtime/src/cbor.rs): shortest
// heads and floats on write, every well-formed definite-length encoding of a
// value on read, a signed integer and a float at every head and precision
// boundary ("Conformance" item 2), an indefinite length refused at any depth,
// each refusal named by its class, and a skip that takes an unknown value
// whole and refuses one too deep. What a generated codec does with these is
// the conformance harness's (codec_cbor_map), which this file does not repeat.

#include "sce/forge/cbor.h"

#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <limits>
#include <string_view>
#include <utility>
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
using ::SCE::Forge::CodecError;
using ::SCE::Forge::Decoded;
using ::SCE::Forge::SceCursor;
using ::SCE::Forge::VectorSink;
using Bytes = std::vector<std::uint8_t>;

/// Whether `read` holds `value`.
template <typename T> bool holds(const Decoded<T> &read, const T &value) {
    return read.has_value() && *read == value;
}

/// Whether `read` is refused as `kind`, the kind the Rust reader answers for
/// the same bytes.
template <typename T> bool refused_as(const Decoded<T> &read, CodecError kind) {
    return !read.has_value() && read.error() == kind;
}

template <typename F> Bytes written(F write) {
    Bytes out;
    VectorSink sink(out);
    CHECK(!write(sink).has_value(), "a write into a growable sink refused");
    return out;
}

SceCursor cursor_over(const Bytes &b) {
    return SceCursor(b.data(), b.size());
}

/// The bits of `x`, which tell -0.0 from 0.0 where `==` does not.
std::uint64_t bits_of(double x) {
    std::uint64_t bits = 0;
    std::memcpy(&bits, &x, sizeof bits);
    return bits;
}

/// Whether `read` over `b` holds `value` and reads `b` whole.
template <typename T, typename R> bool reads_whole(const Bytes &b, R read, const T &value) {
    auto c = cursor_over(b);
    const auto got = read(c);
    return holds(got, value) && c.remaining() == 0;
}

/// An `int64`, the widest signed type.
Decoded<std::int64_t> int64(SceCursor &c) {
    return cbor::read_int_within(c, std::numeric_limits<std::int64_t>::min(), std::numeric_limits<std::int64_t>::max());
}

/// An `int8`.
Decoded<std::int64_t> int8(SceCursor &c) {
    return cbor::read_int_within(c, -128, 127);
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
    CHECK(written([](auto &w) { return cbor::write_array_head(w, 2); }) == Bytes({0x82}), "array head");
    CHECK(written([](auto &w) { return cbor::write_array_head(w, 24); }) == Bytes({0x98, 0x18}), "array head 24");
    // A record's key is written from a literal, as the generated codec writes it.
    CHECK(written([](auto &w) { return cbor::write_text(w, "seq"); }) == Bytes({0x63, 's', 'e', 'q'}), "key");
}

void a_signed_integer_is_written_in_the_shortest_head_of_its_sign() {
    const std::pair<std::int64_t, Bytes> cases[] = {
        {0, {0x00}},
        {23, {0x17}},
        {24, {0x18, 0x18}},
        {255, {0x18, 0xff}},
        {256, {0x19, 0x01, 0x00}},
        {65535, {0x19, 0xff, 0xff}},
        {65536, {0x1a, 0x00, 0x01, 0x00, 0x00}},
        {4294967295LL, {0x1a, 0xff, 0xff, 0xff, 0xff}},
        {4294967296LL, {0x1b, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00}},
        {std::numeric_limits<std::int64_t>::max(), {0x1b, 0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff}},
        {-1, {0x20}},
        {-24, {0x37}},
        {-25, {0x38, 0x18}},
        {-256, {0x38, 0xff}},
        {-257, {0x39, 0x01, 0x00}},
        {-500, {0x39, 0x01, 0xf3}},
        {-65536, {0x39, 0xff, 0xff}},
        {-65537, {0x3a, 0x00, 0x01, 0x00, 0x00}},
        {-4294967296LL, {0x3a, 0xff, 0xff, 0xff, 0xff}},
        {-4294967297LL, {0x3b, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00}},
        {std::numeric_limits<std::int64_t>::min(), {0x3b, 0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff}},
    };
    for (const auto &[value, bytes] : cases) {
        const auto v = value;
        CHECK(written([v](auto &w) { return cbor::write_int(w, v); }) == bytes, "a signed integer's bytes");
        CHECK(reads_whole(bytes, int64, value), "a signed integer read back");
    }
}

void a_signed_integer_is_read_from_a_longer_head_and_held_to_its_type() {
    // -1 with its argument in a one-byte head, and 1 in an eight-byte one.
    CHECK(reads_whole(Bytes{0x38, 0x00}, int64, std::int64_t{-1}), "-1 in a 2-byte head");
    CHECK(reads_whole(Bytes{0x1b, 0, 0, 0, 0, 0, 0, 0, 1}, int64, std::int64_t{1}), "1 in a 9-byte head");
    // An int8 holds -128 and 127, and neither -129 nor 128.
    CHECK(reads_whole(Bytes{0x38, 0x7f}, int8, std::int64_t{-128}), "-128 in an int8");
    CHECK(reads_whole(Bytes{0x18, 0x7f}, int8, std::int64_t{127}), "127 in an int8");
    const Bytes below{0x38, 0x80};
    auto b = cursor_over(below);
    CHECK(refused_as(int8(b), CodecError::CborOutOfRange), "-129 is past an int8");
    const Bytes above{0x18, 0x80};
    auto a = cursor_over(above);
    CHECK(refused_as(int8(a), CodecError::CborOutOfRange), "128 is past an int8");
    // An int64 holds neither -2^63 - 1 nor 2^63.
    const Bytes least{0x3b, 0x80, 0, 0, 0, 0, 0, 0, 0};
    auto l = cursor_over(least);
    CHECK(refused_as(int64(l), CodecError::CborOutOfRange), "-2^63 - 1 is past an int64");
    const Bytes greatest{0x1b, 0x80, 0, 0, 0, 0, 0, 0, 0};
    auto g = cursor_over(greatest);
    CHECK(refused_as(int64(g), CodecError::CborOutOfRange), "2^63 is past an int64");
    // A text, a float and a tagged item are no integer.
    for (const Bytes &bytes : {Bytes{0x61, 'a'}, Bytes{0xf9, 0x3c, 0x00}, Bytes{0xc2, 0x41, 0x01}}) {
        auto c = cursor_over(bytes);
        CHECK(refused_as(int64(c), CodecError::CborWrongType), "an item that is no integer");
    }
}

void an_unsigned_type_takes_no_negative_integer_and_no_bignum() {
    for (const Bytes &bytes : {Bytes{0x20}, Bytes{0xc2, 0x41, 0x01}, Bytes{0x61, 'a'}}) {
        auto c = cursor_over(bytes);
        CHECK(refused_as(cbor::read_uint(c), CodecError::CborWrongType), "an item that is no unsigned integer");
    }
}

void a_float_is_written_in_the_shortest_precision_that_holds_it_exactly() {
    const std::pair<double, Bytes> cases[] = {
        {0.0, {0xf9, 0x00, 0x00}},
        {-0.0, {0xf9, 0x80, 0x00}},
        {1.0, {0xf9, 0x3c, 0x00}},
        {1.5, {0xf9, 0x3e, 0x00}},
        // The largest half, the least normal half and the least subnormal half.
        {65504.0, {0xf9, 0x7b, 0xff}},
        {0.00006103515625, {0xf9, 0x04, 0x00}},
        {5.9604644775390625e-8, {0xf9, 0x00, 0x01}},
        {std::numeric_limits<double>::infinity(), {0xf9, 0x7c, 0x00}},
        {-std::numeric_limits<double>::infinity(), {0xf9, 0xfc, 0x00}},
        // Past a half's range, or between its values: a single holds it.
        {65536.0, {0xfa, 0x47, 0x80, 0x00, 0x00}},
        {100000.0, {0xfa, 0x47, 0xc3, 0x50, 0x00}},
        {3.4028234663852886e38, {0xfa, 0x7f, 0x7f, 0xff, 0xff}},
        // Only a double holds it.
        {1.1, {0xfb, 0x3f, 0xf1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9a}},
        {1.0e300, {0xfb, 0x7e, 0x37, 0xe4, 0x3c, 0x88, 0x00, 0x75, 0x9c}},
    };
    for (const auto &[value, bytes] : cases) {
        const auto x = value;
        CHECK(written([x](auto &w) { return cbor::write_float(w, x); }) == bytes, "a float's bytes");
        auto c = cursor_over(bytes);
        const auto read = cbor::read_float(c);
        CHECK(read.has_value() && bits_of(*read) == bits_of(value) && c.remaining() == 0, "a float read back");
    }
    // The one NaN, whatever NaN is given.
    const Bytes nan{0xf9, 0x7e, 0x00};
    CHECK(written([](auto &w) { return cbor::write_float(w, std::numeric_limits<double>::quiet_NaN()); }) == nan,
          "a quiet NaN");
    CHECK(written([](auto &w) { return cbor::write_float(w, -std::numeric_limits<double>::quiet_NaN()); }) == nan,
          "a negative NaN");
    CHECK(written([](auto &w) { return cbor::write_float(w, std::numeric_limits<double>::signaling_NaN()); }) == nan,
          "a signalling NaN");
    // A float32 is written as the double it is exactly: 0.1f needs a single.
    CHECK(written([](auto &w) { return cbor::write_float(w, static_cast<double>(0.1f)); }) ==
              Bytes({0xfa, 0x3d, 0xcc, 0xcc, 0xcd}),
          "0.1f");
}

void a_float_is_read_from_a_wider_precision_than_it_needs() {
    // 1.5 as a single and as a double.
    CHECK(reads_whole(Bytes{0xfa, 0x3f, 0xc0, 0x00, 0x00}, cbor::read_float, 1.5), "1.5 as a single");
    CHECK(reads_whole(Bytes{0xfb, 0x3f, 0xf8, 0, 0, 0, 0, 0, 0}, cbor::read_float, 1.5), "1.5 as a double");
    // A float32 given as the double that is exactly 0.1f, and as a half.
    CHECK(reads_whole(Bytes{0xfb, 0x3f, 0xb9, 0x99, 0x99, 0xa0, 0, 0, 0}, cbor::read_float32, 0.1f),
          "0.1f as a double");
    CHECK(reads_whole(Bytes{0xf9, 0x3e, 0x00}, cbor::read_float32, 1.5f), "1.5f as a half");
    // A NaN of any precision is a NaN of either type.
    const Bytes half_nan{0xf9, 0x7e, 0x00};
    auto h = cursor_over(half_nan);
    const auto as_double = cbor::read_float(h);
    CHECK(as_double.has_value() && std::isnan(*as_double), "a half NaN is a float64 NaN");
    const Bytes double_nan{0xfb, 0x7f, 0xf8, 0, 0, 0, 0, 0, 0};
    auto d = cursor_over(double_nan);
    const auto as_single = cbor::read_float32(d);
    CHECK(as_single.has_value() && std::isnan(*as_single), "a double NaN is a float32 NaN");
}

void a_float_that_no_single_is_is_no_float32_and_an_integer_is_no_float() {
    // 0.1 as a double is between two singles.
    const Bytes tenth{0xfb, 0x3f, 0xb9, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9a};
    auto t = cursor_over(tenth);
    CHECK(refused_as(cbor::read_float32(t), CodecError::CborOutOfRange), "0.1 is no float32");
    // An integer, a truth value and null are no float.
    for (const Bytes &bytes : {Bytes{0x01}, Bytes{0xf5}, Bytes{0xf6}}) {
        auto c = cursor_over(bytes);
        CHECK(refused_as(cbor::read_float(c), CodecError::CborWrongType), "an item that is no float");
    }
}

void a_truth_value_is_neither_null_nor_a_float_and_a_small_simple_value_has_one_form() {
    const Bytes no{0xf4};
    auto n = cursor_over(no);
    CHECK(holds(cbor::read_bool(n), false), "false");
    const Bytes yes{0xf5};
    auto y = cursor_over(yes);
    CHECK(holds(cbor::read_bool(y), true), "true");
    // `null`, a half whose bits are 20, and the simple value 32.
    for (const Bytes &bytes : {Bytes{0xf6}, Bytes{0xf9, 0x00, 0x14}, Bytes{0xf8, 0x20}}) {
        auto c = cursor_over(bytes);
        CHECK(refused_as(cbor::read_bool(c), CodecError::CborWrongType), "an item that is no truth value");
    }
    // `false` in the two-byte form, and a break code standing alone: not
    // well-formed.
    for (const Bytes &bytes : {Bytes{0xf8, 0x14}, Bytes{0xff}}) {
        auto c = cursor_over(bytes);
        CHECK(refused_as(cbor::read_bool(c), CodecError::CborMalformed), "an item that is not well-formed");
    }
}

void an_array_head_is_read_as_its_count_and_no_other_item_is() {
    const Bytes three{0x83};
    auto c = cursor_over(three);
    CHECK(holds(cbor::read_array_len(c), std::uint64_t{3}), "an array of three");
    const Bytes wide{0x98, 0x18};
    auto d = cursor_over(wide);
    CHECK(holds(cbor::read_array_len(d), std::uint64_t{24}), "an array of 24");
    // An indefinite-length array, and a map where an array stands.
    const Bytes indefinite{0x9f};
    auto i = cursor_over(indefinite);
    CHECK(refused_as(cbor::read_array_len(i), CodecError::CborMalformed), "indefinite-length array");
    const Bytes map{0xa1};
    auto m = cursor_over(map);
    CHECK(refused_as(cbor::read_array_len(m), CodecError::CborWrongType), "a map is no array");
}

void a_head_is_read_in_any_valid_length() {
    // 1 written in its 2-byte form still reads as 1.
    const Bytes two{0x18, 0x01};
    auto c = cursor_over(two);
    CHECK(holds(cbor::read_uint(c), std::uint64_t{1}), "1 in a 2-byte head");
    const Bytes nine{0x1b, 0, 0, 0, 1, 0, 0, 0, 0};
    auto d = cursor_over(nine);
    CHECK(holds(cbor::read_uint(d), std::uint64_t{1} << 32), "2^32");
}

void what_the_codec_cannot_read_is_refused_by_its_kind() {
    // Indefinite-length map and reserved additional information: not a head
    // the codec reads.
    for (const Bytes &b : {Bytes{0xbf}, Bytes{0x1c}}) {
        auto c = cursor_over(b);
        CHECK(refused_as(cbor::read_uint(c), CodecError::CborMalformed), "a head the codec does not read");
    }
    const Bytes indefinite{0xbf};
    auto m = cursor_over(indefinite);
    CHECK(refused_as(cbor::read_map_len(m), CodecError::CborMalformed), "indefinite-length map");
    // Another major type, a width the entry cannot hold, a wrong exact
    // length, a text past its bound, a text that is not UTF-8, an item cut
    // short.
    const Bytes text{0x61, 'a'};
    auto k = cursor_over(text);
    CHECK(refused_as(cbor::read_uint(k), CodecError::CborWrongType), "a text is no unsigned integer");
    const Bytes wide{0x19, 0x01, 0x00};
    auto u = cursor_over(wide);
    CHECK(refused_as(cbor::read_uint_upto(u, 255), CodecError::CborOutOfRange), "256 in a uint8");
    const Bytes short_bytes{0x42, 1, 2};
    auto s = cursor_over(short_bytes);
    CHECK(refused_as(cbor::read_bytes_exact(s, 16), CodecError::CborWrongLength),
          "two bytes where sixteen are declared");
    const Bytes long_text{0x62, 'a', 'b'};
    auto t = cursor_over(long_text);
    CHECK(refused_as(cbor::read_text(t, 1), CodecError::CborOutOfRange), "a text past its bound");
    const Bytes not_utf8{0x61, 0xff};
    auto x = cursor_over(not_utf8);
    CHECK(refused_as(cbor::read_text(x, std::nullopt), CodecError::InvalidUtf8), "a text that is not UTF-8");
    const Bytes cut{0x19, 0x01};
    auto y = cursor_over(cut);
    CHECK(refused_as(cbor::read_uint(y), CodecError::NeedMoreBytes), "a head cut short");
}

void an_indefinite_length_is_outside_the_profile_at_any_depth() {
    // A text in chunks, a byte string in chunks.
    const Bytes chunked_text{0x7f, 0x61, 0x61, 0x61, 0x62, 0xff};
    auto t = cursor_over(chunked_text);
    CHECK(refused_as(cbor::read_text(t, std::nullopt), CodecError::CborMalformed), "a text in chunks");
    const Bytes chunked_bytes{0x5f, 0x41, 0x01, 0xff};
    auto b = cursor_over(chunked_bytes);
    CHECK(refused_as(cbor::read_bytes(b, std::nullopt), CodecError::CborMalformed), "a byte string in chunks");
    // An indefinite array skipped, and an indefinite text inside a map skipped.
    for (const Bytes &bytes : {Bytes{0x9f, 0x01, 0x02, 0xff}, Bytes{0xa1, 0x01, 0x7f, 0x61, 0x61, 0xff}}) {
        auto c = cursor_over(bytes);
        CHECK(refused_as(cbor::skip(c), CodecError::CborMalformed), "an indefinite length skipped");
    }
}

constexpr std::string_view kPaymentCases[] = {"card", "cash", "voucher"};

void a_union_is_a_map_of_one_entry_keyed_by_its_variant_name() {
    // `cash`, a case that holds no record.
    const Bytes cash{0xa1, 0x64, 'c', 'a', 's', 'h', 0xa0};
    CHECK(written([](auto &w) {
              if (auto e = cbor::write_variant(w, "cash")) {
                  return e;
              }
              return cbor::write_map_head(w, 0);
          }) == cash,
          "the cash case");
    auto c = cursor_over(cash);
    CHECK(holds(cbor::read_variant(c, kPaymentCases), std::size_t{1}), "the cash case is the second");
    CHECK(holds(cbor::read_map_len(c), std::uint64_t{0}), "its record is the empty map");
    CHECK(c.remaining() == 0, "the union is read whole");
}

void a_union_map_is_judged_for_one_entry_then_for_its_key() {
    const std::pair<Bytes, CodecError> cases[] = {
        // No entry, and two entries refused before the key is read: no key
        // follows.
        {{0xa0}, CodecError::CborUnionEntries},
        {{0xa2}, CodecError::CborUnionEntries},
        // A name no case declares.
        {{0xa1, 0x64, 'c', 'o', 'i', 'n', 0xa0}, CodecError::CborUnknownVariant},
        // A key that is not a text, and an item that is no map.
        {{0xa1, 0x01, 0xa0}, CodecError::CborWrongType},
        {{0x80}, CodecError::CborWrongType},
        // An indefinite union map.
        {{0xbf, 0x64, 'c', 'a', 's', 'h', 0xa0, 0xff}, CodecError::CborMalformed},
    };
    for (const auto &[bytes, kind] : cases) {
        auto c = cursor_over(bytes);
        CHECK(refused_as(cbor::read_variant(c, kPaymentCases), kind), "a union map refused by its kind");
    }
}

void a_case_that_holds_no_record_is_read_from_any_map_of_text_keys() {
    // The empty map an encode writes, and a map of keys the case does not
    // declare, each stepped over with its value: both are the case. The second
    // is followed by a 7 the read leaves.
    const std::pair<Bytes, std::size_t> empty[] = {
        {{0xa0}, 0},
        {{0xa1, 0x61, 'x', 0x82, 0x01, 0x02, 0x07}, 1},
    };
    for (const auto &[bytes, left] : empty) {
        auto c = cursor_over(bytes);
        CHECK(cbor::read_empty_case(c).has_value(), "an empty case's map is read");
        CHECK(c.remaining() == left, "an empty case's map is read whole and no further");
    }
    // A key that is no text, and `null` or an array where the map stands.
    const Bytes refused[] = {{0xa1, 0x01, 0x01}, {0xf6}, {0x80}};
    for (const auto &bytes : refused) {
        auto c = cursor_over(bytes);
        CHECK(refused_as(cbor::read_empty_case(c), CodecError::CborWrongType), "no map of text keys is no empty case");
    }
}

void an_absent_element_is_null_and_a_present_one_is_left_for_its_read() {
    CHECK(written([](auto &w) { return cbor::write_null(w); }) == Bytes{0xf6}, "null is f6");
    // `null`, then the element 7: the first is taken, the second left.
    const Bytes two{0xf6, 0x07};
    auto c = cursor_over(two);
    CHECK(holds(cbor::take_null(c), true), "null is taken");
    CHECK(holds(cbor::take_null(c), false), "an element is no null");
    CHECK(c.remaining() == 1, "an element is left for its read");
    CHECK(holds(cbor::read_uint(c), std::uint64_t{7}), "the element is read after");
    // `false` is no `null`, and nothing at all is an input that ended.
    const Bytes no{0xf4};
    auto f = cursor_over(no);
    CHECK(holds(cbor::take_null(f), false) && f.remaining() == 1, "false is no null and is left");
    const Bytes none;
    auto e = cursor_over(none);
    CHECK(refused_as(cbor::take_null(e), CodecError::NeedMoreBytes), "an input that ended has no item");
}

void an_unknown_value_is_skipped_whole_and_a_deep_one_refused() {
    // {1: [2, {3: h'00'}]} then 7: the skip lands on the 7.
    const Bytes nested{0xa1, 0x01, 0x82, 0x02, 0xa1, 0x03, 0x41, 0x00, 0x07};
    auto c = cursor_over(nested);
    CHECK(cbor::skip(c).has_value(), "a nested value is skipped");
    CHECK(holds(cbor::read_uint(c), std::uint64_t{7}), "the skip lands on the next item");

    Bytes deep(20, 0x81);
    deep.push_back(0x00);
    auto d = cursor_over(deep);
    CHECK(refused_as(cbor::skip(d), CodecError::CborTooDeep), "twenty nested arrays are past the skip depth");
}

}  // namespace

int main() {
    every_head_is_written_in_its_shortest_form();
    a_signed_integer_is_written_in_the_shortest_head_of_its_sign();
    a_signed_integer_is_read_from_a_longer_head_and_held_to_its_type();
    an_unsigned_type_takes_no_negative_integer_and_no_bignum();
    a_float_is_written_in_the_shortest_precision_that_holds_it_exactly();
    a_float_is_read_from_a_wider_precision_than_it_needs();
    a_float_that_no_single_is_is_no_float32_and_an_integer_is_no_float();
    a_truth_value_is_neither_null_nor_a_float_and_a_small_simple_value_has_one_form();
    a_head_is_read_in_any_valid_length();
    an_array_head_is_read_as_its_count_and_no_other_item_is();
    what_the_codec_cannot_read_is_refused_by_its_kind();
    an_indefinite_length_is_outside_the_profile_at_any_depth();
    a_union_is_a_map_of_one_entry_keyed_by_its_variant_name();
    a_union_map_is_judged_for_one_entry_then_for_its_key();
    a_case_that_holds_no_record_is_read_from_any_map_of_text_keys();
    an_absent_element_is_null_and_a_present_one_is_left_for_its_read();
    an_unknown_value_is_skipped_whole_and_a_deep_one_refused();
    if (failures != 0) {
        std::fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    std::puts("cbor_runtime_test: all checks passed");
    return 0;
}
