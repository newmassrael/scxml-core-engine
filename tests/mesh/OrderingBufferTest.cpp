// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE-VERIFIES: mesh-10.6
//
// Unit tests for SCE::Mesh::OrderingBuffer (SCE_MESH.md §10.6).
//
// Mutation guard: patching `admit` to release unconditionally (returning
// `{env}` for every call) must fail at least these tests:
//   * AdmitsOutOfOrderBuffers
//   * GapFillReleasesAllBuffered
//   * DuplicateSeqAfterFastForwardIgnored
// If those regressions are silent, the buffer is providing no ordering.

#include "mesh/OrderingBuffer.h"

#include <cstdint>
#include <gtest/gtest.h>

#include <chrono>
#include <string>
#include <thread>

using SCE::Mesh::MeshEnvelope;
using SCE::Mesh::OrderingBuffer;
using SCE::Mesh::PatternKind;
using SCE::Mesh::PayloadCodec;

namespace {

MeshEnvelope make_env(const std::string &source, std::uint64_t seq) {
    MeshEnvelope env{};
    env.source = source;
    env.type = "t";
    env.pattern = PatternKind::FireForget;
    env.datacontenttype = PayloadCodec::None;
    env.sequence_no = seq;
    return env;
}

// Test default — only the constraint is "long enough that no reorder
// test in this file accidentally trips a tick gap." Mirrors the
// deploy.yaml DEFAULT_GAP_TIMEOUT_MS so a reader who sees 100 ms in
// either place does not have to wonder if the two diverged.
constexpr auto kTestGap = std::chrono::milliseconds(100);

}  // namespace

TEST(OrderingBufferTest, AdmitsInOrderDispatchesImmediately) {
    OrderingBuffer buf{kTestGap};
    auto r1 = buf.admit("a", make_env("a", 1));
    ASSERT_EQ(r1.size(), 1u);
    ASSERT_TRUE(r1[0].sequence_no.has_value());
    EXPECT_EQ(*r1[0].sequence_no, 1u);

    auto r2 = buf.admit("a", make_env("a", 2));
    ASSERT_EQ(r2.size(), 1u);
    EXPECT_EQ(*r2[0].sequence_no, 2u);

    auto r3 = buf.admit("a", make_env("a", 3));
    ASSERT_EQ(r3.size(), 1u);
    EXPECT_EQ(*r3[0].sequence_no, 3u);
}

TEST(OrderingBufferTest, AdmitsOutOfOrderBuffers) {
    OrderingBuffer buf{kTestGap};
    // First envelope anchors next_expected_seq = 1.
    auto r1 = buf.admit("a", make_env("a", 1));
    ASSERT_EQ(r1.size(), 1u);

    // seq=3 arrives before seq=2: must be buffered, not released.
    auto r3 = buf.admit("a", make_env("a", 3));
    EXPECT_TRUE(r3.empty()) << "seq=3 must buffer while seq=2 is missing";
}

TEST(OrderingBufferTest, GapFillReleasesAllBuffered) {
    OrderingBuffer buf{kTestGap};
    EXPECT_EQ(buf.admit("a", make_env("a", 1)).size(), 1u);
    EXPECT_TRUE(buf.admit("a", make_env("a", 4)).empty());
    EXPECT_TRUE(buf.admit("a", make_env("a", 3)).empty());
    // Gap filled by seq=2 — should drain 2, 3, 4 in order.
    auto r2 = buf.admit("a", make_env("a", 2));
    ASSERT_EQ(r2.size(), 3u);
    EXPECT_EQ(*r2[0].sequence_no, 2u);
    EXPECT_EQ(*r2[1].sequence_no, 3u);
    EXPECT_EQ(*r2[2].sequence_no, 4u);
}

TEST(OrderingBufferTest, GapTimeoutFastForwardsAndRaisesOnGap) {
    OrderingBuffer buf(std::chrono::milliseconds(5));
    EXPECT_EQ(buf.admit("a", make_env("a", 1)).size(), 1u);
    EXPECT_TRUE(buf.admit("a", make_env("a", 3)).empty());

    // Before timeout, tick() returns empty on both axes.
    {
        auto r = buf.tick();
        EXPECT_TRUE(r.released.empty());
        EXPECT_TRUE(r.gaps.empty());
    }

    std::this_thread::sleep_for(std::chrono::milliseconds(10));
    auto r = buf.tick();
    ASSERT_EQ(r.released.size(), 1u) << "seq=3 must be released after gap fast-forward";
    EXPECT_EQ(*r.released[0].sequence_no, 3u);
    ASSERT_EQ(r.gaps.size(), 1u) << "one ORDERING_GAP event per timed-out source";
    EXPECT_EQ(r.gaps[0].source, "a");
    EXPECT_EQ(r.gaps[0].lost_lo, 2u);
    EXPECT_EQ(r.gaps[0].lost_hi, 2u);
}

TEST(OrderingBufferTest, PerSenderIsolation) {
    OrderingBuffer buf{kTestGap};
    // Two senders with independent sequence domains; each must be
    // admitted in its own stream without interference.
    EXPECT_EQ(buf.admit("a", make_env("a", 1)).size(), 1u);
    EXPECT_EQ(buf.admit("b", make_env("b", 1)).size(), 1u);
    // A late arrival out of order on 'a' must not be blocked by 'b'.
    EXPECT_TRUE(buf.admit("a", make_env("a", 3)).empty());
    EXPECT_EQ(buf.admit("b", make_env("b", 2)).size(), 1u);
    auto r = buf.admit("a", make_env("a", 2));
    ASSERT_EQ(r.size(), 2u);
    EXPECT_EQ(*r[0].sequence_no, 2u);
    EXPECT_EQ(*r[1].sequence_no, 3u);
}

TEST(OrderingBufferTest, DuplicateSeqAfterFastForwardIgnored) {
    OrderingBuffer buf(std::chrono::milliseconds(5));
    EXPECT_EQ(buf.admit("a", make_env("a", 1)).size(), 1u);
    EXPECT_TRUE(buf.admit("a", make_env("a", 3)).empty());
    std::this_thread::sleep_for(std::chrono::milliseconds(10));
    auto first = buf.tick();
    EXPECT_EQ(first.released.size(), 1u);
    EXPECT_EQ(first.gaps.size(), 1u);

    // Now a straggler seq=2 arrives after the fast-forward. It's
    // below next_expected_seq (=4) and must be dropped silently,
    // NOT released and NOT re-triggering the gap callback.
    auto r = buf.admit("a", make_env("a", 2));
    EXPECT_TRUE(r.empty());
}

// SCE_MESH.md section 10.6.4: a source not heard from before is expected
// from its first number, whatever arrives first, so an envelope that
// overtook the first waits for it and both are released in order. Red if
// the first envelope that arrives anchors the stream: seq=2 is then
// released at once and seq=1 dropped as a straggler, reported to no one.
TEST(OrderingBufferTest, FreshStreamWaitsForItsFirstNumber) {
    OrderingBuffer buf{kTestGap};
    EXPECT_TRUE(buf.admit("a", make_env("a", 2)).empty()) << "seq=2 waits for seq=1";
    auto r = buf.admit("a", make_env("a", 1));
    ASSERT_EQ(r.size(), 2u);
    EXPECT_EQ(*r[0].sequence_no, 1u);
    EXPECT_EQ(*r[1].sequence_no, 2u);
    auto t = buf.tick();
    EXPECT_TRUE(t.released.empty());
    EXPECT_TRUE(t.gaps.empty());
}

// SCE_MESH.md section 10.6.4: a stream joined after its start (a receiver
// whose state was lost) waits for the numbers below its first arrival as
// for any gap, and once the gap has waited out its timeout reports them
// lost and releases what it held. Red if the first arrival anchors the
// stream: seq=7 is then released at once and 1..6 are never reported.
TEST(OrderingBufferTest, StreamJoinedAfterItsStartReportsTheNumbersBeforeItLost) {
    OrderingBuffer buf(std::chrono::milliseconds(5));
    EXPECT_TRUE(buf.admit("a", make_env("a", 7)).empty()) << "seq=7 waits for the numbers below it";
    std::this_thread::sleep_for(std::chrono::milliseconds(10));
    auto r = buf.tick();
    ASSERT_EQ(r.gaps.size(), 1u);
    EXPECT_EQ(r.gaps[0].source, "a");
    EXPECT_EQ(r.gaps[0].lost_lo, 1u);
    EXPECT_EQ(r.gaps[0].lost_hi, 6u);
    ASSERT_EQ(r.released.size(), 1u);
    EXPECT_EQ(*r.released[0].sequence_no, 7u);
    // seq=6 was reported lost, so a copy that comes late is a straggler.
    EXPECT_TRUE(buf.admit("a", make_env("a", 6)).empty());
    auto r8 = buf.admit("a", make_env("a", 8));
    ASSERT_EQ(r8.size(), 1u);
    EXPECT_EQ(*r8[0].sequence_no, 8u);
}
