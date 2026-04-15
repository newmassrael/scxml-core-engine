// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "common/Uuid.h"

#include <gtest/gtest.h>

#include <algorithm>
#include <chrono>
#include <cstdint>
#include <thread>
#include <unordered_set>

namespace {

// Pack a UUID byte array into a 128-bit pair for hash-set membership tests.
struct U128 {
    uint64_t hi;
    uint64_t lo;
    bool operator==(const U128 &other) const { return hi == other.hi && lo == other.lo; }
};
struct U128Hash {
    size_t operator()(const U128 &v) const noexcept {
        return std::hash<uint64_t>{}(v.hi) ^ (std::hash<uint64_t>{}(v.lo) << 1);
    }
};

U128 pack(const SCE::uuid::Bytes &b) {
    U128 out{0, 0};
    for (int i = 0; i < 8; ++i) out.hi = (out.hi << 8) | b[i];
    for (int i = 0; i < 8; ++i) out.lo = (out.lo << 8) | b[8 + i];
    return out;
}

// Extract the 48-bit timestamp prefix from a v7 UUID (bytes 0-5, big-endian).
uint64_t timestamp_ms(const SCE::uuid::Bytes &b) {
    return (uint64_t(b[0]) << 40) | (uint64_t(b[1]) << 32) | (uint64_t(b[2]) << 24) |
           (uint64_t(b[3]) << 16) | (uint64_t(b[4]) << 8)  | uint64_t(b[5]);
}

}  // namespace

TEST(UuidTest, V7VersionBitsAreSeven) {
    auto id = SCE::uuid::v7();
    EXPECT_EQ((id[6] & 0xF0) >> 4, 0x7);
}

TEST(UuidTest, V7VariantBitsAreRfc) {
    auto id = SCE::uuid::v7();
    // RFC 9562 §4.1: variant 0b10 in top two bits of byte 8.
    EXPECT_EQ(id[8] & 0xC0, 0x80);
}

TEST(UuidTest, V4VersionBitsAreFour) {
    auto id = SCE::uuid::v4();
    EXPECT_EQ((id[6] & 0xF0) >> 4, 0x4);
}

TEST(UuidTest, V4VariantBitsAreRfc) {
    auto id = SCE::uuid::v4();
    EXPECT_EQ(id[8] & 0xC0, 0x80);
}

TEST(UuidTest, V7TimestampPrefixIsRecent) {
    using namespace std::chrono;
    auto before = duration_cast<milliseconds>(system_clock::now().time_since_epoch()).count();
    auto id = SCE::uuid::v7();
    auto after = duration_cast<milliseconds>(system_clock::now().time_since_epoch()).count();
    auto ts = timestamp_ms(id);
    // Allow ±1 ms slack for clock granularity boundaries.
    EXPECT_GE(ts, static_cast<uint64_t>(before) - 1);
    EXPECT_LE(ts, static_cast<uint64_t>(after) + 1);
}

TEST(UuidTest, V7TimestampPrefixMonotonicAcrossMs) {
    // Across a 50 ms window, prefixes must be non-decreasing — verifying the
    // monotonic-prefix property that gives v7 its log-ordering value.
    auto first = timestamp_ms(SCE::uuid::v7());
    std::this_thread::sleep_for(std::chrono::milliseconds(50));
    auto last = timestamp_ms(SCE::uuid::v7());
    EXPECT_GT(last, first);
}

TEST(UuidTest, V7StrictlyMonotonicIntraMs) {
    // RFC 9562 §6.2 Method 1: rand_a is a monotonic counter so IDs generated
    // back-to-back within a single millisecond are strictly ordered when
    // compared as 128-bit big-endian integers.
    constexpr size_t N = 4000;  // < 4096 (12-bit counter capacity per ms)
    std::vector<U128> ids;
    ids.reserve(N);
    for (size_t i = 0; i < N; ++i) ids.push_back(pack(SCE::uuid::v7()));
    for (size_t i = 1; i < N; ++i) {
        ASSERT_TRUE(ids[i - 1].hi < ids[i].hi ||
                    (ids[i - 1].hi == ids[i].hi && ids[i - 1].lo < ids[i].lo))
            << "v7 not strictly monotonic at index " << i;
    }
}

TEST(UuidTest, V7CounterOverflowAdvancesTimestamp) {
    // Generate >4096 IDs as fast as possible. The counter overflows at 4096
    // within a single ms; the implementation must advance ms (synthetic
    // future) rather than collide. Verifies no duplicates over 10000 IDs.
    constexpr size_t N = 10'000;
    std::unordered_set<U128, U128Hash> seen;
    seen.reserve(N);
    for (size_t i = 0; i < N; ++i) {
        EXPECT_TRUE(seen.insert(pack(SCE::uuid::v7())).second);
    }
    EXPECT_EQ(seen.size(), N);
}

TEST(UuidTest, V7CollisionFreeOneMillion) {
    constexpr size_t N = 1'000'000;
    std::unordered_set<U128, U128Hash> seen;
    seen.reserve(N);
    for (size_t i = 0; i < N; ++i) {
        EXPECT_TRUE(seen.insert(pack(SCE::uuid::v7())).second) << "v7 collision at " << i;
    }
    EXPECT_EQ(seen.size(), N);
}

TEST(UuidTest, V4CollisionFreeOneMillion) {
    constexpr size_t N = 1'000'000;
    std::unordered_set<U128, U128Hash> seen;
    seen.reserve(N);
    for (size_t i = 0; i < N; ++i) {
        EXPECT_TRUE(seen.insert(pack(SCE::uuid::v4())).second) << "v4 collision at " << i;
    }
    EXPECT_EQ(seen.size(), N);
}

// ── RFC 4122 §3 canonical text form ─────────────────────────────

TEST(UuidTest, ToStringCanonicalFormat) {
    // Fixed fixture taken from RFC 4122 §3 example, verifying exact byte →
    // character mapping including dash positions.
    SCE::uuid::Bytes fixture = {
        0xf8, 0x1d, 0x4f, 0xae, 0x7d, 0xec, 0x11, 0xd0,
        0xa7, 0x65, 0x00, 0xa0, 0xc9, 0x1e, 0x6b, 0xf6,
    };
    EXPECT_EQ(SCE::uuid::to_string(fixture), "f81d4fae-7dec-11d0-a765-00a0c91e6bf6");
}

TEST(UuidTest, ToStringLengthIs36) {
    auto id = SCE::uuid::v7();
    EXPECT_EQ(SCE::uuid::to_string(id).size(), 36u);
}

TEST(UuidTest, FromStringRoundTripsToString) {
    auto bytes = SCE::uuid::v7();
    auto text = SCE::uuid::to_string(bytes);
    auto parsed = SCE::uuid::from_string(text);
    ASSERT_TRUE(parsed.has_value());
    EXPECT_EQ(*parsed, bytes);
}

TEST(UuidTest, FromStringAcceptsUppercase) {
    auto lower = SCE::uuid::from_string("f81d4fae-7dec-11d0-a765-00a0c91e6bf6");
    auto upper = SCE::uuid::from_string("F81D4FAE-7DEC-11D0-A765-00A0C91E6BF6");
    ASSERT_TRUE(lower.has_value());
    ASSERT_TRUE(upper.has_value());
    EXPECT_EQ(*lower, *upper);
}

TEST(UuidTest, FromStringRejectsWrongLength) {
    EXPECT_FALSE(SCE::uuid::from_string("").has_value());
    EXPECT_FALSE(SCE::uuid::from_string("f81d4fae").has_value());
    EXPECT_FALSE(SCE::uuid::from_string("f81d4fae-7dec-11d0-a765-00a0c91e6bf6-extra").has_value());
}

TEST(UuidTest, FromStringRejectsMisplacedDashes) {
    // 36 chars total but dashes shifted — rejects instead of silently mapping
    // the wrong bytes.
    EXPECT_FALSE(
        SCE::uuid::from_string("f81d4fae7-dec-11d0-a765-00a0c91e6bf6").has_value());
    EXPECT_FALSE(
        SCE::uuid::from_string("f81d4fa-e7dec-11d0-a765-00a0c91e6bf6").has_value());
}

TEST(UuidTest, FromStringRejectsNonHex) {
    EXPECT_FALSE(
        SCE::uuid::from_string("f81d4fae-7dec-11d0-z765-00a0c91e6bf6").has_value());
    EXPECT_FALSE(
        SCE::uuid::from_string("XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX").has_value());
}
