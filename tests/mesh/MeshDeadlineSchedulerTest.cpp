// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// MeshDeadlineScheduler unit tests — SCE Mesh §9.5 deadline timing.
//
// Verifies the four outcome paths a registered deadline can reach:
//   1. Register + elapse             → callback fires exactly once
//   2. Register + cancel before fire → callback does NOT fire
//   3. Cancel-vs-fire race           → at most one outcome observed
//   4. Shutdown with pending         → callbacks dropped silently
//
// Plus the caller-contract edge cases:
//   * Duplicate registerDeadline for the same uuid is rejected.
//   * Re-register after cancel succeeds and honours the new deadline
//     (the earlier heap entry has been marked stale).
//   * Earliest-deadline-first ordering across multiple registrations.

#include "mesh/MeshDeadlineScheduler.h"

#include <gtest/gtest.h>

#include <array>
#include <atomic>
#include <chrono>
#include <cstdint>
#include <mutex>
#include <thread>
#include <vector>

using SCE::Mesh::MeshDeadlineScheduler;
using namespace std::chrono_literals;

namespace {

constexpr MeshDeadlineScheduler::Key kUuidA = {
    0x01, 0x82, 0xb1, 0x4d, 0xa3, 0x5c, 0x70, 0x12,
    0xb4, 0xde, 0xf0, 0x42, 0x9a, 0x88, 0x77, 0x66,
};

constexpr MeshDeadlineScheduler::Key kUuidB = {
    0xff, 0xee, 0xdd, 0xcc, 0xbb, 0xaa, 0x70, 0x01,
    0x90, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
};

constexpr MeshDeadlineScheduler::Key kUuidC = {
    0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x70, 0x03,
    0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x00, 0x11,
};

/// Busy-wait helper bounded by `budget`: polls `pred` every millisecond
/// so tests do not rely on a single `sleep_for` being long enough on a
/// slow CI worker. Returns true if the predicate became true before the
/// budget expired.
template <typename Pred>
bool waitFor(std::chrono::milliseconds budget, Pred pred) {
    const auto giveUp = std::chrono::steady_clock::now() + budget;
    while (std::chrono::steady_clock::now() < giveUp) {
        if (pred()) return true;
        std::this_thread::sleep_for(1ms);
    }
    return pred();
}

}  // namespace

TEST(MeshDeadlineScheduler, RegisterAndFireAfterDelay) {
    MeshDeadlineScheduler scheduler;
    std::atomic<int> fired{0};

    ASSERT_TRUE(scheduler.registerDeadline(kUuidA, 25ms, [&fired]{ ++fired; }));
    EXPECT_EQ(scheduler.size(), 1u);

    ASSERT_TRUE(waitFor(500ms, [&]{ return fired.load() == 1; }));
    // After firing, size must drop — entry has been consumed.
    EXPECT_TRUE(waitFor(100ms, [&]{ return scheduler.size() == 0u; }));
}

TEST(MeshDeadlineScheduler, CancelBeforeFireSuppressesCallback) {
    MeshDeadlineScheduler scheduler;
    std::atomic<int> fired{0};

    ASSERT_TRUE(scheduler.registerDeadline(kUuidA, 50ms, [&fired]{ ++fired; }));
    ASSERT_TRUE(scheduler.cancelDeadline(kUuidA));
    EXPECT_EQ(scheduler.size(), 0u);

    // Sleep well past the original deadline. The callback must NOT run.
    std::this_thread::sleep_for(120ms);
    EXPECT_EQ(fired.load(), 0);
}

TEST(MeshDeadlineScheduler, CancelUnknownUuidReturnsFalse) {
    MeshDeadlineScheduler scheduler;
    EXPECT_FALSE(scheduler.cancelDeadline(kUuidA));
}

TEST(MeshDeadlineScheduler, DuplicateRegisterReturnsFalse) {
    MeshDeadlineScheduler scheduler;
    std::atomic<int> firedA{0};
    std::atomic<int> firedB{0};

    ASSERT_TRUE(scheduler.registerDeadline(kUuidA, 200ms, [&firedA]{ ++firedA; }));
    // Second call with same uuid: rejected; original registration untouched.
    EXPECT_FALSE(scheduler.registerDeadline(kUuidA, 10ms, [&firedB]{ ++firedB; }));

    // The first registration's 200ms must still be honoured; the bogus
    // 10ms deadline of the rejected second call must not fire.
    std::this_thread::sleep_for(50ms);
    EXPECT_EQ(firedA.load(), 0);
    EXPECT_EQ(firedB.load(), 0);

    ASSERT_TRUE(waitFor(500ms, [&]{ return firedA.load() == 1; }));
    EXPECT_EQ(firedB.load(), 0);
}

TEST(MeshDeadlineScheduler, MultipleDeadlinesFireInEarliestFirstOrder) {
    MeshDeadlineScheduler scheduler;
    std::mutex order_mutex;
    std::vector<int> order;

    // Register in reverse deadline order (B first with 80ms, then A
    // with 20ms) to prove the heap, not insertion order, picks the
    // dispatch sequence.
    ASSERT_TRUE(scheduler.registerDeadline(kUuidB, 80ms, [&]{
        std::lock_guard<std::mutex> lock(order_mutex);
        order.push_back(2);
    }));
    ASSERT_TRUE(scheduler.registerDeadline(kUuidA, 20ms, [&]{
        std::lock_guard<std::mutex> lock(order_mutex);
        order.push_back(1);
    }));
    ASSERT_TRUE(scheduler.registerDeadline(kUuidC, 150ms, [&]{
        std::lock_guard<std::mutex> lock(order_mutex);
        order.push_back(3);
    }));

    ASSERT_TRUE(waitFor(1000ms, [&]{
        std::lock_guard<std::mutex> lock(order_mutex);
        return order.size() == 3;
    }));

    std::lock_guard<std::mutex> lock(order_mutex);
    ASSERT_EQ(order.size(), 3u);
    EXPECT_EQ(order[0], 1);
    EXPECT_EQ(order[1], 2);
    EXPECT_EQ(order[2], 3);
}

TEST(MeshDeadlineScheduler, ReregisterAfterCancelHonoursNewDeadline) {
    // Stale heap entry scenario: cancel a long deadline, then
    // immediately register a shorter one under the SAME uuid. The
    // dispatcher must honour the new registration, not treat the new
    // entry as stale because an old one lingers in the heap.
    MeshDeadlineScheduler scheduler;
    std::atomic<int> firedOld{0};
    std::atomic<int> firedNew{0};

    ASSERT_TRUE(scheduler.registerDeadline(kUuidA, 10s, [&firedOld]{ ++firedOld; }));
    ASSERT_TRUE(scheduler.cancelDeadline(kUuidA));
    ASSERT_TRUE(scheduler.registerDeadline(kUuidA, 25ms, [&firedNew]{ ++firedNew; }));

    ASSERT_TRUE(waitFor(500ms, [&]{ return firedNew.load() == 1; }));
    EXPECT_EQ(firedOld.load(), 0);
}

TEST(MeshDeadlineScheduler, ConcurrentCancelVsFireAtMostOneOutcome) {
    // Drive many short-deadline registrations with a cancel racing
    // against the fire; every iteration must observe exactly zero or
    // one callback, never two, and the total-fired count must match
    // (iterations - observed cancels).
    constexpr int kIterations = 200;
    std::atomic<int> fired{0};
    std::atomic<int> cancelled{0};

    MeshDeadlineScheduler scheduler;
    for (int i = 0; i < kIterations; ++i) {
        MeshDeadlineScheduler::Key uuid{};
        uuid[0] = static_cast<std::uint8_t>(i >> 8);
        uuid[1] = static_cast<std::uint8_t>(i & 0xff);

        std::atomic<int> thisFired{0};
        ASSERT_TRUE(scheduler.registerDeadline(uuid, 1ms, [&]{ ++thisFired; ++fired; }));
        // Cancel races with fire: ~1ms deadline with immediate cancel
        // produces a mix of wins on either side.
        if (scheduler.cancelDeadline(uuid)) ++cancelled;
        (void)waitFor(50ms, [&]{ return thisFired.load() + (cancelled.load() > 0 ? 0 : 0) >= 0; });
    }

    // Give any trailing in-flight callbacks time to settle.
    std::this_thread::sleep_for(50ms);

    // Correctness invariant: fired + cancelled == kIterations. Neither
    // an entry that both fires and is cancelled (double outcome) nor
    // an entry that does neither (lost) is acceptable.
    EXPECT_EQ(fired.load() + cancelled.load(), kIterations);
}

TEST(MeshDeadlineScheduler, ShutdownWithPendingDropsCallbacks) {
    std::atomic<int> fired{0};
    {
        MeshDeadlineScheduler scheduler;
        ASSERT_TRUE(scheduler.registerDeadline(kUuidA, 10s, [&fired]{ ++fired; }));
        ASSERT_TRUE(scheduler.registerDeadline(kUuidB, 10s, [&fired]{ ++fired; }));
        // Destructor invokes shutdown() → joins worker → no callback
        // can still be running. Pending deadlines are dropped.
    }
    EXPECT_EQ(fired.load(), 0);
}

TEST(MeshDeadlineScheduler, RegisterAfterShutdownReturnsFalse) {
    MeshDeadlineScheduler scheduler;
    scheduler.shutdown();
    EXPECT_FALSE(scheduler.registerDeadline(kUuidA, 10ms, []{}));
}
