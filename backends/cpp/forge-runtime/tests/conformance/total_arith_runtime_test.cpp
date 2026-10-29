// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// sce/forge/algorithm.h — the `Wrapping` and `Saturating` families held to
// their definition (docs/adr/0063, section 1). For a type T of width W that
// holds [m, M], and e the exact integer result:
//
//   wrapping    the r in [m, M] with r = e (mod 2^W);
//   saturating  max(m, min(M, e)).
//
// Up to 32 bits every helper is run over a grid of values (all of them for
// 8 bits) and compared with that definition, computed here by another route:
// e is held exactly as a sign and a magnitude, which every result of two
// 32-bit operands fits, and reduced by `%` on the magnitude. 64 bits have no
// wider type to hold e in, so their cases are written out, each value worked
// from the definition by hand; so are the cases the families exist for at
// every width (the wrap point, saturation on each side, MIN * -1, neg(MIN),
// MIN - 1, and a product past 2^W, which separates computing in the type from
// computing in `int`). The helpers are `constexpr`, and the call shape the
// generator writes is evaluated at compile time below. This target is built
// at C++17, the standard the interface is for; what a generated algorithm
// does with the helpers is the conformance fixtures' (docs/adr/0063,
// section 9).

#include "sce/forge/algorithm.h"

#include <cinttypes>
#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <limits>
#include <type_traits>
#include <vector>

namespace {

namespace Wrapping = ::SCE::Forge::Wrapping;
namespace Saturating = ::SCE::Forge::Saturating;

int failures = 0;

template <typename T> void expect(const char *what, int line, T got, T want) {
    if (got == want) {
        return;
    }
    ++failures;
    if constexpr (std::is_signed_v<T>) {
        std::fprintf(stderr, "FAIL %s:%d %s = %" PRId64 ", expected %" PRId64 "\n", __FILE__, line, what,
                     static_cast<std::int64_t>(got), static_cast<std::int64_t>(want));
    } else {
        std::fprintf(stderr, "FAIL %s:%d %s = %" PRIu64 ", expected %" PRIu64 "\n", __FILE__, line, what,
                     static_cast<std::uint64_t>(got), static_cast<std::uint64_t>(want));
    }
}

// The expected value is braced into the call's own type, so a value the type
// cannot hold is a compile error rather than a silent conversion.
#define EXPECT(call, want) expect(#call, __LINE__, (call), decltype(call){want})

template <typename T> constexpr T kMin = std::numeric_limits<T>::min();
template <typename T> constexpr T kMax = std::numeric_limits<T>::max();

// ---------------------------------------------------------------------------
// The call shape and the types
// ---------------------------------------------------------------------------

// Every helper returns its own type, which is what a generated body stores
// and compares: a helper answering `int` would pass every value check below
// and still change what `==` on its result means.
template <typename T> constexpr bool returnsItsType() {
    return std::is_same_v<decltype(Wrapping::add<T>(0, 0)), T> && std::is_same_v<decltype(Wrapping::sub<T>(0, 0)), T> &&
           std::is_same_v<decltype(Wrapping::mul<T>(0, 0)), T> && std::is_same_v<decltype(Wrapping::neg<T>(0)), T> &&
           std::is_same_v<decltype(Saturating::add<T>(0, 0)), T> &&
           std::is_same_v<decltype(Saturating::sub<T>(0, 0)), T> &&
           std::is_same_v<decltype(Saturating::mul<T>(0, 0)), T>;
}

static_assert(returnsItsType<std::uint8_t>() && returnsItsType<std::uint16_t>() && returnsItsType<std::uint32_t>() &&
              returnsItsType<std::uint64_t>());
static_assert(returnsItsType<std::int8_t>() && returnsItsType<std::int16_t>() && returnsItsType<std::int32_t>() &&
              returnsItsType<std::int64_t>());
static_assert(std::is_same_v<decltype(Saturating::neg<std::int8_t>(0)), std::int8_t> &&
              std::is_same_v<decltype(Saturating::neg<std::int64_t>(0)), std::int64_t>);

// Every helper is `noexcept`: the embedded profile has no exceptions.
static_assert(noexcept(Wrapping::mul<std::uint16_t>(0, 0)) && noexcept(Saturating::mul<std::int64_t>(0, 0)) &&
              noexcept(Saturating::neg<std::int32_t>(0)));

// The shape the generator writes, `SCE::Forge::Wrapping::sub<std::uint32_t>(a, b)`,
// evaluated at compile time: the tick counter of the request
// (docs/adr/0063, Context), which wrapped between `last` and `now`.
// 5 - (2^32 - 2) = 7 - 2^32, which is 7 modulo 2^32.
constexpr std::uint32_t kElapsed = SCE::Forge::Wrapping::sub<std::uint32_t>(5, 0xFFFFFFFEu);
static_assert(kElapsed == 7);
static_assert(SCE::Forge::Saturating::sub<std::uint32_t>(10, kElapsed) == 3);
static_assert(SCE::Forge::Saturating::sub<std::uint32_t>(5, kElapsed) == 0);
static_assert(SCE::Forge::Wrapping::mul<std::uint16_t>(65535, 65535) == 1);
static_assert(SCE::Forge::Saturating::mul<std::int64_t>(kMin<std::int64_t>, -1) == kMax<std::int64_t>);
static_assert(SCE::Forge::Wrapping::neg<std::int8_t>(-128) == -128);

// ---------------------------------------------------------------------------
// The definition, computed by another route
// ---------------------------------------------------------------------------

// An exact integer as its sign and magnitude: every result of two operands of
// at most 32 bits is below 2^64 in magnitude. A zero is never negative.
struct Exact {
    bool negative;
    std::uint64_t magnitude;
};

// |v| for a v of magnitude below 2^63.
std::uint64_t magnitudeOf(std::int64_t v) {
    return v < 0 ? std::uint64_t{0} - static_cast<std::uint64_t>(v) : static_cast<std::uint64_t>(v);
}

Exact exact(std::int64_t v) {
    return Exact{v < 0, magnitudeOf(v)};
}

Exact exactProduct(std::int64_t a, std::int64_t b) {
    const std::uint64_t magnitude = magnitudeOf(a) * magnitudeOf(b);
    return Exact{magnitude != 0 && ((a < 0) != (b < 0)), magnitude};
}

// The r of `T` with r = e (mod 2^W), for a `T` of at most 32 bits.
template <typename T> std::int64_t wrapped(Exact e) {
    constexpr int kBits = std::numeric_limits<T>::digits + (std::is_signed_v<T> ? 1 : 0);
    constexpr std::uint64_t kModulus = std::uint64_t{1} << kBits;
    std::uint64_t r = e.magnitude % kModulus;
    if (e.negative && r != 0) {
        r = kModulus - r;
    }
    if (std::is_signed_v<T> && r >= kModulus / 2) {
        return static_cast<std::int64_t>(r) - static_cast<std::int64_t>(kModulus);
    }
    return static_cast<std::int64_t>(r);
}

// max(m, min(M, e)).
template <typename T> std::int64_t saturated(Exact e) {
    const auto lo = static_cast<std::int64_t>(kMin<T>);
    const auto hi = static_cast<std::int64_t>(kMax<T>);
    if (e.negative) {
        return e.magnitude > magnitudeOf(lo) ? lo : -static_cast<std::int64_t>(e.magnitude);
    }
    return e.magnitude > static_cast<std::uint64_t>(hi) ? hi : static_cast<std::int64_t>(e.magnitude);
}

void gridCase(const char *what, const char *type, std::int64_t a, std::int64_t b, std::int64_t got, std::int64_t want) {
    if (got != want) {
        ++failures;
        std::fprintf(stderr, "FAIL %s %s(%" PRId64 ", %" PRId64 ") = %" PRId64 ", expected %" PRId64 "\n", what, type,
                     a, b, got, want);
    }
}

// Every operation of every pair of `values`, against the definition.
template <typename T> void againstTheDefinition(const char *type, const std::vector<T> &values) {
    for (const T a : values) {
        const Exact negated = exact(-static_cast<std::int64_t>(a));
        gridCase("Wrapping::neg", type, a, 0, Wrapping::neg<T>(a), wrapped<T>(negated));
        if constexpr (std::is_signed_v<T>) {
            gridCase("Saturating::neg", type, a, 0, Saturating::neg<T>(a), saturated<T>(negated));
        }
        for (const T b : values) {
            const Exact sum = exact(static_cast<std::int64_t>(a) + static_cast<std::int64_t>(b));
            const Exact difference = exact(static_cast<std::int64_t>(a) - static_cast<std::int64_t>(b));
            const Exact product = exactProduct(a, b);
            gridCase("Wrapping::add", type, a, b, Wrapping::add<T>(a, b), wrapped<T>(sum));
            gridCase("Wrapping::sub", type, a, b, Wrapping::sub<T>(a, b), wrapped<T>(difference));
            gridCase("Wrapping::mul", type, a, b, Wrapping::mul<T>(a, b), wrapped<T>(product));
            gridCase("Saturating::add", type, a, b, Saturating::add<T>(a, b), saturated<T>(sum));
            gridCase("Saturating::sub", type, a, b, Saturating::sub<T>(a, b), saturated<T>(difference));
            gridCase("Saturating::mul", type, a, b, Saturating::mul<T>(a, b), saturated<T>(product));
        }
    }
}

// Every value of an 8-bit type.
template <typename T> std::vector<T> everyValue() {
    std::vector<T> values;
    for (int v = kMin<T>; v <= kMax<T>; ++v) {
        values.push_back(static_cast<T>(v));
    }
    return values;
}

void every8BitPairMeetsTheDefinition() {
    againstTheDefinition<std::uint8_t>("uint8", everyValue<std::uint8_t>());
    againstTheDefinition<std::int8_t>("int8", everyValue<std::int8_t>());
}

// m, M, 0, 1, -1 and their neighbours, and the operands whose product is the
// first past M (181 * 181 = 32761 and 182 * 182 = 33124 in 16 bits;
// 46340 * 46340 = 2147395600 and 46341 * 46341 = 2147488281 in 32).
void theBoundsOf16And32BitsMeetTheDefinition() {
    againstTheDefinition<std::uint16_t>("uint16", {0, 1, 2, 181, 182, 255, 256, 257, 32767, 32768, 65534, 65535});
    againstTheDefinition<std::int16_t>("int16", {kMin<std::int16_t>, -32767, -256, -182, -181, -2, -1, 0, 1, 2, 181,
                                                 182, 256, 32766, kMax<std::int16_t>});
    againstTheDefinition<std::uint32_t>(
        "uint32", {0, 1, 2, 46340, 46341, 65535, 65536, 65537, 0x7FFFFFFFu, 0x80000000u, 0xFFFFFFFEu, 0xFFFFFFFFu});
    againstTheDefinition<std::int32_t>("int32", {kMin<std::int32_t>, -2147483647, -65536, -46341, -46340, -2, -1, 0, 1,
                                                 2, 46340, 46341, 65536, 2147483646, kMax<std::int32_t>});
}

// ---------------------------------------------------------------------------
// The cases the families exist for, worked by hand
// ---------------------------------------------------------------------------

void theUnsignedWidthsWrapAndSaturateAtTheirBounds() {
    using std::uint16_t;
    using std::uint32_t;
    using std::uint64_t;
    using std::uint8_t;

    // 255 * 255 = 65025 = 254 * 256 + 1.
    EXPECT(Wrapping::add<uint8_t>(255, 1), 0);
    EXPECT(Wrapping::sub<uint8_t>(0, 1), 255);
    EXPECT(Wrapping::mul<uint8_t>(255, 255), 1);
    EXPECT(Wrapping::mul<uint8_t>(16, 16), 0);
    EXPECT(Wrapping::neg<uint8_t>(1), 255);
    EXPECT(Wrapping::neg<uint8_t>(0), 0);
    EXPECT(Saturating::add<uint8_t>(255, 1), 255);
    EXPECT(Saturating::add<uint8_t>(254, 1), 255);
    EXPECT(Saturating::add<uint8_t>(253, 1), 254);
    EXPECT(Saturating::sub<uint8_t>(0, 1), 0);
    EXPECT(Saturating::sub<uint8_t>(1, 1), 0);
    EXPECT(Saturating::sub<uint8_t>(255, 254), 1);
    EXPECT(Saturating::mul<uint8_t>(255, 255), 255);
    EXPECT(Saturating::mul<uint8_t>(15, 17), 255);
    EXPECT(Saturating::mul<uint8_t>(16, 16), 255);

    // 65535 * 65535 = 2^32 - 2^17 + 1, past `int` where `int` is 32 bits: the
    // case that separates computing in the type from computing in `int`.
    EXPECT(Wrapping::add<uint16_t>(65535, 1), 0);
    EXPECT(Wrapping::sub<uint16_t>(0, 1), 65535);
    EXPECT(Wrapping::mul<uint16_t>(65535, 65535), 1);
    EXPECT(Wrapping::mul<uint16_t>(256, 256), 0);
    EXPECT(Wrapping::neg<uint16_t>(1), 65535);
    EXPECT(Saturating::add<uint16_t>(65535, 1), 65535);
    EXPECT(Saturating::add<uint16_t>(65534, 1), 65535);
    EXPECT(Saturating::sub<uint16_t>(0, 65535), 0);
    EXPECT(Saturating::mul<uint16_t>(65535, 65535), 65535);
    EXPECT(Saturating::mul<uint16_t>(255, 257), 65535);
    EXPECT(Saturating::mul<uint16_t>(256, 256), 65535);

    // 0xFFFFFFFF * 0xFFFFFFFF = 2^64 - 2^33 + 1.
    EXPECT(Wrapping::add<uint32_t>(0xFFFFFFFFu, 1), 0);
    EXPECT(Wrapping::sub<uint32_t>(0, 1), 0xFFFFFFFFu);
    EXPECT(Wrapping::mul<uint32_t>(0xFFFFFFFFu, 0xFFFFFFFFu), 1);
    EXPECT(Wrapping::mul<uint32_t>(65536, 65536), 0);
    EXPECT(Wrapping::neg<uint32_t>(1), 0xFFFFFFFFu);
    EXPECT(Saturating::add<uint32_t>(0xFFFFFFFFu, 1), 0xFFFFFFFFu);
    EXPECT(Saturating::sub<uint32_t>(1, 2), 0);
    EXPECT(Saturating::mul<uint32_t>(0xFFFFFFFFu, 0xFFFFFFFFu), 0xFFFFFFFFu);
    EXPECT(Saturating::mul<uint32_t>(65535, 65537), 0xFFFFFFFFu);
    EXPECT(Saturating::mul<uint32_t>(65536, 65536), 0xFFFFFFFFu);

    constexpr uint64_t kM = kMax<uint64_t>;
    EXPECT(Wrapping::add<uint64_t>(kM, 1), 0);
    EXPECT(Wrapping::add<uint64_t>(kM, kM), kM - 1);
    EXPECT(Wrapping::add<uint64_t>(kM - 1, 1), kM);
    EXPECT(Wrapping::add<uint64_t>(0, 0), 0);
    EXPECT(Wrapping::sub<uint64_t>(0, 1), kM);
    EXPECT(Wrapping::sub<uint64_t>(0, kM), 1);
    EXPECT(Wrapping::sub<uint64_t>(1, 2), kM);
    EXPECT(Wrapping::sub<uint64_t>(kM, kM), 0);
    EXPECT(Wrapping::sub<uint64_t>(kM, 0), kM);
    // (2^64 - 1)^2 = 2^128 - 2^65 + 1; (2^32 + 1)(2^32 - 1) = 2^64 - 1.
    EXPECT(Wrapping::mul<uint64_t>(kM, kM), 1);
    EXPECT(Wrapping::mul<uint64_t>(kM, 2), kM - 1);
    EXPECT(Wrapping::mul<uint64_t>(4294967296u, 4294967296u), 0);
    EXPECT(Wrapping::mul<uint64_t>(4294967297u, 4294967295u), kM);
    EXPECT(Wrapping::mul<uint64_t>(1, kM), kM);
    EXPECT(Wrapping::mul<uint64_t>(0, kM), 0);
    EXPECT(Wrapping::neg<uint64_t>(0), 0);
    EXPECT(Wrapping::neg<uint64_t>(1), kM);
    EXPECT(Wrapping::neg<uint64_t>(kM), 1);
    EXPECT(Wrapping::neg<uint64_t>(9223372036854775808u), 9223372036854775808u);
    EXPECT(Saturating::add<uint64_t>(kM, 1), kM);
    EXPECT(Saturating::add<uint64_t>(kM, kM), kM);
    EXPECT(Saturating::add<uint64_t>(kM - 1, 1), kM);
    EXPECT(Saturating::add<uint64_t>(2, kM - 1), kM);
    EXPECT(Saturating::add<uint64_t>(kM - 1, 0), kM - 1);
    EXPECT(Saturating::add<uint64_t>(0, 0), 0);
    EXPECT(Saturating::sub<uint64_t>(0, 1), 0);
    EXPECT(Saturating::sub<uint64_t>(1, kM), 0);
    EXPECT(Saturating::sub<uint64_t>(1, 1), 0);
    EXPECT(Saturating::sub<uint64_t>(kM, 1), kM - 1);
    EXPECT(Saturating::sub<uint64_t>(kM, 0), kM);
    EXPECT(Saturating::mul<uint64_t>(kM, kM), kM);
    EXPECT(Saturating::mul<uint64_t>(kM, 2), kM);
    EXPECT(Saturating::mul<uint64_t>(kM, 1), kM);
    EXPECT(Saturating::mul<uint64_t>(kM, 0), 0);
    EXPECT(Saturating::mul<uint64_t>(0, kM), 0);
    EXPECT(Saturating::mul<uint64_t>(4294967296u, 4294967296u), kM);
    EXPECT(Saturating::mul<uint64_t>(4294967296u, 4294967297u), kM);
    EXPECT(Saturating::mul<uint64_t>(4294967297u, 4294967295u), kM);
    EXPECT(Saturating::mul<uint64_t>(4294967296u, 4294967295u), 18446744069414584320u);
}

void theSignedWidthsWrapAndSaturateAtTheirBounds() {
    using std::int16_t;
    using std::int32_t;
    using std::int64_t;
    using std::int8_t;

    // 127 * 127 = 16129 = 63 * 256 + 1; -128 * -128 = 16384 = 64 * 256.
    EXPECT(Wrapping::add<int8_t>(127, 1), -128);
    EXPECT(Wrapping::sub<int8_t>(-128, 1), 127);
    EXPECT(Wrapping::mul<int8_t>(-128, -1), -128);
    EXPECT(Wrapping::mul<int8_t>(127, 127), 1);
    EXPECT(Wrapping::mul<int8_t>(-128, -128), 0);
    EXPECT(Wrapping::mul<int8_t>(127, 2), -2);
    EXPECT(Wrapping::neg<int8_t>(-128), -128);
    EXPECT(Wrapping::neg<int8_t>(127), -127);
    EXPECT(Saturating::add<int8_t>(127, 1), 127);
    EXPECT(Saturating::add<int8_t>(-128, -1), -128);
    EXPECT(Saturating::add<int8_t>(-128, 127), -1);
    EXPECT(Saturating::sub<int8_t>(-128, 1), -128);
    EXPECT(Saturating::sub<int8_t>(0, -128), 127);
    EXPECT(Saturating::sub<int8_t>(-1, -128), 127);
    EXPECT(Saturating::sub<int8_t>(-2, 127), -128);
    EXPECT(Saturating::mul<int8_t>(-128, -1), 127);
    EXPECT(Saturating::mul<int8_t>(-128, 2), -128);
    EXPECT(Saturating::mul<int8_t>(127, -1), -127);
    EXPECT(Saturating::mul<int8_t>(-64, 2), -128);
    EXPECT(Saturating::mul<int8_t>(64, 2), 127);
    EXPECT(Saturating::neg<int8_t>(-128), 127);
    EXPECT(Saturating::neg<int8_t>(127), -127);
    EXPECT(Saturating::neg<int8_t>(-127), 127);

    // (2^15 - 1)^2 = 2^30 - 2^16 + 1.
    EXPECT(Wrapping::add<int16_t>(32767, 1), -32768);
    EXPECT(Wrapping::sub<int16_t>(-32768, 1), 32767);
    EXPECT(Wrapping::mul<int16_t>(-32768, -1), -32768);
    EXPECT(Wrapping::mul<int16_t>(32767, 32767), 1);
    EXPECT(Wrapping::mul<int16_t>(-32768, -32768), 0);
    EXPECT(Wrapping::neg<int16_t>(-32768), -32768);
    EXPECT(Saturating::add<int16_t>(32767, 1), 32767);
    EXPECT(Saturating::sub<int16_t>(-32768, 1), -32768);
    EXPECT(Saturating::mul<int16_t>(-32768, -1), 32767);
    EXPECT(Saturating::mul<int16_t>(181, 181), 32761);
    EXPECT(Saturating::mul<int16_t>(182, 182), 32767);
    EXPECT(Saturating::mul<int16_t>(-182, 182), -32768);
    EXPECT(Saturating::neg<int16_t>(-32768), 32767);

    // 2^16 * 2^15 = 2^31, the pattern of the least int32.
    constexpr int32_t kMin32 = kMin<int32_t>;
    constexpr int32_t kMax32 = kMax<int32_t>;
    EXPECT(Wrapping::add<int32_t>(kMax32, 1), kMin32);
    EXPECT(Wrapping::sub<int32_t>(kMin32, 1), kMax32);
    EXPECT(Wrapping::mul<int32_t>(kMin32, -1), kMin32);
    EXPECT(Wrapping::mul<int32_t>(kMax32, kMax32), 1);
    EXPECT(Wrapping::mul<int32_t>(65536, 65536), 0);
    EXPECT(Wrapping::mul<int32_t>(65536, 32768), kMin32);
    EXPECT(Wrapping::neg<int32_t>(kMin32), kMin32);
    EXPECT(Saturating::add<int32_t>(kMax32, kMax32), kMax32);
    EXPECT(Saturating::sub<int32_t>(kMin32, 1), kMin32);
    EXPECT(Saturating::mul<int32_t>(kMin32, -1), kMax32);
    EXPECT(Saturating::mul<int32_t>(46340, 46340), 2147395600);
    EXPECT(Saturating::mul<int32_t>(46341, 46341), kMax32);
    EXPECT(Saturating::mul<int32_t>(-46341, 46341), kMin32);
    EXPECT(Saturating::neg<int32_t>(kMin32), kMax32);

    constexpr int64_t kM = kMax<int64_t>;
    constexpr int64_t km = kMin<int64_t>;
    EXPECT(Wrapping::add<int64_t>(kM, 1), km);
    EXPECT(Wrapping::add<int64_t>(km, -1), kM);
    EXPECT(Wrapping::add<int64_t>(kM, kM), -2);
    EXPECT(Wrapping::add<int64_t>(km, km), 0);
    EXPECT(Wrapping::add<int64_t>(km, kM), -1);
    EXPECT(Wrapping::add<int64_t>(kM - 1, 1), kM);
    EXPECT(Wrapping::add<int64_t>(-1, 1), 0);
    EXPECT(Wrapping::sub<int64_t>(km, 1), kM);
    EXPECT(Wrapping::sub<int64_t>(kM, -1), km);
    EXPECT(Wrapping::sub<int64_t>(0, km), km);
    EXPECT(Wrapping::sub<int64_t>(-1, kM), km);
    EXPECT(Wrapping::sub<int64_t>(-1, km), kM);
    EXPECT(Wrapping::sub<int64_t>(km + 1, 1), km);
    EXPECT(Wrapping::sub<int64_t>(0, 1), -1);
    // (2^32 + 1)(2^32 - 1) = 2^64 - 1, which is -1 modulo 2^64.
    EXPECT(Wrapping::mul<int64_t>(km, -1), km);
    EXPECT(Wrapping::mul<int64_t>(kM, 2), -2);
    EXPECT(Wrapping::mul<int64_t>(kM, kM), 1);
    EXPECT(Wrapping::mul<int64_t>(km, km), 0);
    EXPECT(Wrapping::mul<int64_t>(kM, -1), km + 1);
    EXPECT(Wrapping::mul<int64_t>(km + 1, -1), kM);
    EXPECT(Wrapping::mul<int64_t>(-1, -1), 1);
    EXPECT(Wrapping::mul<int64_t>(4294967296, 4294967296), 0);
    EXPECT(Wrapping::mul<int64_t>(4294967297, 4294967295), -1);
    EXPECT(Wrapping::neg<int64_t>(km), km);
    EXPECT(Wrapping::neg<int64_t>(kM), km + 1);
    EXPECT(Wrapping::neg<int64_t>(km + 1), kM);
    EXPECT(Wrapping::neg<int64_t>(0), 0);
    EXPECT(Wrapping::neg<int64_t>(1), -1);
    EXPECT(Wrapping::neg<int64_t>(-1), 1);

    EXPECT(Saturating::add<int64_t>(kM, 1), kM);
    EXPECT(Saturating::add<int64_t>(km, -1), km);
    EXPECT(Saturating::add<int64_t>(kM, kM), kM);
    EXPECT(Saturating::add<int64_t>(km, km), km);
    EXPECT(Saturating::add<int64_t>(km, kM), -1);
    EXPECT(Saturating::add<int64_t>(kM - 1, 1), kM);
    EXPECT(Saturating::add<int64_t>(km + 1, -1), km);
    EXPECT(Saturating::add<int64_t>(kM, -1), kM - 1);
    EXPECT(Saturating::add<int64_t>(0, 0), 0);
    // -1 - m = 2^63 - 1 is M itself; -2 - M = -2^63 - 1 is one past m.
    EXPECT(Saturating::sub<int64_t>(km, 1), km);
    EXPECT(Saturating::sub<int64_t>(kM, -1), kM);
    EXPECT(Saturating::sub<int64_t>(0, km), kM);
    EXPECT(Saturating::sub<int64_t>(-1, km), kM);
    EXPECT(Saturating::sub<int64_t>(-2, km), kM - 1);
    EXPECT(Saturating::sub<int64_t>(0, kM), km + 1);
    EXPECT(Saturating::sub<int64_t>(-1, kM), km);
    EXPECT(Saturating::sub<int64_t>(-2, kM), km);
    EXPECT(Saturating::sub<int64_t>(km + 1, 1), km);
    EXPECT(Saturating::sub<int64_t>(1, 0), 1);
    // 3037000499^2 = 9223372030926249001 is below M = 9223372036854775807;
    // 3037000500^2 = 9223372037000250000 is past it. 2^31 * 2^32 = 2^63.
    EXPECT(Saturating::mul<int64_t>(km, -1), kM);
    EXPECT(Saturating::mul<int64_t>(kM, 2), kM);
    EXPECT(Saturating::mul<int64_t>(kM, -2), km);
    EXPECT(Saturating::mul<int64_t>(km, 2), km);
    EXPECT(Saturating::mul<int64_t>(km, 1), km);
    EXPECT(Saturating::mul<int64_t>(kM, -1), km + 1);
    EXPECT(Saturating::mul<int64_t>(km + 1, -1), kM);
    EXPECT(Saturating::mul<int64_t>(-1, -1), 1);
    EXPECT(Saturating::mul<int64_t>(0, km), 0);
    EXPECT(Saturating::mul<int64_t>(km, 0), 0);
    EXPECT(Saturating::mul<int64_t>(4294967296, 4294967296), kM);
    EXPECT(Saturating::mul<int64_t>(-4294967296, 4294967296), km);
    EXPECT(Saturating::mul<int64_t>(2147483648, 4294967296), kM);
    EXPECT(Saturating::mul<int64_t>(-2147483648, 4294967296), km);
    EXPECT(Saturating::mul<int64_t>(3037000499, 3037000499), 9223372030926249001);
    EXPECT(Saturating::mul<int64_t>(3037000500, 3037000500), kM);
    EXPECT(Saturating::mul<int64_t>(-3037000500, -3037000500), kM);
    EXPECT(Saturating::neg<int64_t>(km), kM);
    EXPECT(Saturating::neg<int64_t>(kM), km + 1);
    EXPECT(Saturating::neg<int64_t>(km + 1), kM);
    EXPECT(Saturating::neg<int64_t>(0), 0);
    EXPECT(Saturating::neg<int64_t>(1), -1);
    EXPECT(Saturating::neg<int64_t>(-1), 1);
}

// The CHECKED product of two uint32 values (SCE_FORGE.md §3.4.1), which the
// total family sits beside: (2^32 - 1)^2 passes INT64_MAX, so a product made in
// std::int64_t would overflow, undefined, inside the helper meant to report it.
// Made in std::uint64_t, it is reported.
void aCheckedUint32ProductPastInt64IsReported() {
    SCE::Forge::AlgorithmFailure f;
    EXPECT(SCE::Forge::Checked::mul<std::uint32_t>(f, 65535u, 65537u), 4294967295u);
    EXPECT(f.failed(), false);
    EXPECT(SCE::Forge::Checked::mul<std::uint32_t>(f, 65536u, 65536u), 0u);
    EXPECT(f.failed(), true);
    SCE::Forge::AlgorithmFailure g;
    EXPECT(SCE::Forge::Checked::mul<std::uint32_t>(g, 4294967295u, 4294967295u), 0u);
    EXPECT(g.failed(), true);
    EXPECT(g.error() == SCE::Forge::AlgorithmError::Overflow, true);
}

}  // namespace

int main() {
    every8BitPairMeetsTheDefinition();
    theBoundsOf16And32BitsMeetTheDefinition();
    theUnsignedWidthsWrapAndSaturateAtTheirBounds();
    theSignedWidthsWrapAndSaturateAtTheirBounds();
    aCheckedUint32ProductPastInt64IsReported();
    if (failures != 0) {
        std::fprintf(stderr, "%d failure(s)\n", failures);
        return 1;
    }
    std::printf("total arithmetic runtime: all cases pass\n");
    return 0;
}
