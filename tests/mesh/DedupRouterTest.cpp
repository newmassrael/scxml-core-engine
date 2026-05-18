// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// DedupRouter / DedupWindow unit tests — SCE_MESH.md §10.5.
//
// The window is a 256-entry ring of recently-observed UUIDs per sender.
// These tests pin down the four guarantees §10.5 promises:
//
//   (1) seeing the same id twice → the second call is rejected,
//   (2) 256 distinct ids all pass,
//   (3) the 257th distinct id evicts the oldest (so the first id
//       becomes novel again),
//   (4) the per-sender map gives each source its own ring — admitting
//       the same id on two distinct sources returns true both times.
//
// Plus a coarse concurrency smoke test so `-fsanitize=thread` gets a
// chance to see the internal mutex in action. The fine-grained race
// behaviour (e.g. the exact set of winners when N threads race to
// observe the same id) is NOT a contract — only that no thread is
// served stale state and that no duplicate ever passes twice.

#include "mesh/DedupRouter.h"

#include <gtest/gtest.h>

#include <array>
#include <atomic>
#include <cstdint>
#include <thread>
#include <vector>

using SCE::Mesh::DedupRouter;
using SCE::Mesh::DedupWindow;

namespace {

// Build a UUID-shaped id from a single 32-bit discriminator. Chosen so
// every test id is distinct from the default-constructed all-zero ring
// slots (the low bytes never vanish for `n > 0`).
DedupWindow::Id id_of(std::uint32_t n) {
    DedupWindow::Id out{};
    out[0] = 0xA0;  // fixed non-zero prefix
    out[12] = static_cast<std::uint8_t>((n >> 24) & 0xff);
    out[13] = static_cast<std::uint8_t>((n >> 16) & 0xff);
    out[14] = static_cast<std::uint8_t>((n >> 8) & 0xff);
    out[15] = static_cast<std::uint8_t>(n & 0xff);
    return out;
}

}  // namespace

// ── DedupWindow ───────────────────────────────────────────────

TEST(DedupWindow, NovelIdIsAdmitted) {
    DedupWindow w;
    EXPECT_TRUE(w.observe(id_of(1)));
}

TEST(DedupWindow, AllZeroIdIsAdmittedOnEmptyWindow) {
    // Regression guard: the default-constructed ring holds all-zero ids
    // internally. Naively scanning every slot would match the incoming
    // zero id on construction and falsely drop it. Real test fixtures
    // and SCXML boilerplate emit envelopes with id = {}, so this path
    // MUST be admitted the first time.
    DedupWindow w;
    const DedupWindow::Id zeros{};
    EXPECT_TRUE(w.observe(zeros));
    EXPECT_FALSE(w.observe(zeros));  // second is still correctly a duplicate
}

TEST(DedupWindow, SameIdTwiceIsRejected) {
    DedupWindow w;
    EXPECT_TRUE(w.observe(id_of(42)));
    EXPECT_FALSE(w.observe(id_of(42)));
}

TEST(DedupWindow, CapacityDistinctIdsAllAdmitted) {
    DedupWindow w;
    for (std::uint32_t n = 1; n <= DedupWindow::kCapacity; ++n) {
        EXPECT_TRUE(w.observe(id_of(n))) << "id " << n << " rejected unexpectedly";
    }
}

TEST(DedupWindow, OldestIdEvictedAfterCapacity) {
    // §10.5 sliding-window invariant: the (kCapacity + 1)-th distinct
    // id replaces the first, so re-observing id 1 is now novel again.
    DedupWindow w;
    for (std::uint32_t n = 1; n <= DedupWindow::kCapacity; ++n) {
        ASSERT_TRUE(w.observe(id_of(n))) << "fill failed at " << n;
    }
    EXPECT_TRUE(w.observe(id_of(DedupWindow::kCapacity + 1)))
        << "(kCapacity + 1)-th distinct id should be novel";
    EXPECT_TRUE(w.observe(id_of(1))) << "first id should have been evicted and so be novel again";
}

TEST(DedupWindow, RecentIdsStillFilterAfterWrap) {
    // After one wrap, the most recent `kCapacity` ids (2..kCapacity+1)
    // must still be filtered — only the oldest has been evicted.
    DedupWindow w;
    for (std::uint32_t n = 1; n <= DedupWindow::kCapacity + 1; ++n) {
        ASSERT_TRUE(w.observe(id_of(n))) << "fill failed at " << n;
    }
    for (std::uint32_t n = 2; n <= DedupWindow::kCapacity + 1; ++n) {
        EXPECT_FALSE(w.observe(id_of(n))) << "recent id " << n << " leaked";
    }
}

// ── DedupRouter ───────────────────────────────────────────────

TEST(DedupRouter, DuplicateFromSameSenderIsDropped) {
    DedupRouter r;
    EXPECT_TRUE(r.admit("motor", id_of(7)));
    EXPECT_FALSE(r.admit("motor", id_of(7)));
}

TEST(DedupRouter, SameIdFromDistinctSendersBothAdmitted) {
    // §10.5 key invariant: the dedup window is keyed on
    // (env.source, env.id). Two senders independently generating
    // identical ids (rare for UUID v7 but legal under the wire
    // contract) MUST NOT collide on a shared window.
    DedupRouter r;
    EXPECT_TRUE(r.admit("motor", id_of(7)));
    EXPECT_TRUE(r.admit("brake", id_of(7)));
}

TEST(DedupRouter, EachSenderGetsFullCapacity) {
    DedupRouter r;
    for (std::uint32_t n = 1; n <= DedupWindow::kCapacity; ++n) {
        ASSERT_TRUE(r.admit("motor", id_of(n)));
        ASSERT_TRUE(r.admit("brake", id_of(n)));
    }
    // Duplicates still dropped per sender.
    EXPECT_FALSE(r.admit("motor", id_of(1)));
    EXPECT_FALSE(r.admit("brake", id_of(1)));
}

TEST(DedupWindow, ObserveWithSignalEnumPathing) {
    // §16.7 row 7 (DEDUP_WINDOW_OVERFLOW): pre-wrap novel inserts
    // return Novel; the (kCapacity + 1)-th distinct id returns
    // NovelWithEviction because it overwrites slot 0; a re-observed
    // recent id returns Duplicate. All three enum arms must be
    // reachable from a single window instance so the codegen call
    // site has a closed switch.
    DedupWindow w;
    for (std::uint32_t n = 1; n <= DedupWindow::kCapacity; ++n) {
        ASSERT_EQ(w.observeWithSignal(id_of(n)), DedupWindow::Result::Novel)
            << "id " << n << " inside pre-wrap window must be Novel";
    }
    EXPECT_EQ(w.observeWithSignal(id_of(DedupWindow::kCapacity + 1)),
              DedupWindow::Result::NovelWithEviction)
        << "first eviction must surface NovelWithEviction";
    EXPECT_EQ(w.observeWithSignal(id_of(DedupWindow::kCapacity)),
              DedupWindow::Result::Duplicate)
        << "still-resident recent id must surface Duplicate";
}

TEST(DedupRouter, AdmitWithSignalRaisesOverflowOnFirstEviction) {
    // The DedupRouter wraps admitWithSignal — the codegen TransportRouter
    // calls this and stamps `error.communication / DEDUP_WINDOW_OVERFLOW`
    // when the result is NovelWithEviction.
    DedupRouter r;
    for (std::uint32_t n = 1; n <= DedupWindow::kCapacity; ++n) {
        ASSERT_EQ(r.admitWithSignal("motor", id_of(n)),
                  DedupWindow::Result::Novel);
    }
    EXPECT_EQ(r.admitWithSignal("motor", id_of(DedupWindow::kCapacity + 1)),
              DedupWindow::Result::NovelWithEviction);
}

TEST(DedupRouter, BoolAdmitContractUnchanged) {
    // The legacy bool admit() collapses NovelWithEviction to true, so
    // existing call sites that just check "should I dispatch?" keep
    // their behaviour. Regression guard against a refactor that flips
    // the eviction case to false.
    DedupRouter r;
    for (std::uint32_t n = 1; n <= DedupWindow::kCapacity + 1; ++n) {
        EXPECT_TRUE(r.admit("motor", id_of(n)))
            << "bool admit must accept all novel ids including the "
               "post-wrap (kCapacity+1)-th";
    }
    EXPECT_FALSE(r.admit("motor", id_of(DedupWindow::kCapacity)))
        << "still-resident recent id must be filtered";
}

TEST(DedupRouter, ConcurrentAdmitsNeverDoubleAdmit) {
    // Stress the internal mutex: many threads all observe the same
    // 256-id batch. Every (source, id) pair must be admitted exactly
    // once across the whole run, regardless of thread scheduling.
    constexpr int kThreads = 8;
    constexpr int kIdsPerThread = 32;
    constexpr int kTotalIds = kThreads * kIdsPerThread;  // 256

    DedupRouter r;
    std::atomic<int> admitted{0};
    std::vector<std::thread> workers;
    workers.reserve(kThreads);
    for (int t = 0; t < kThreads; ++t) {
        workers.emplace_back([&, t]() {
            for (int i = 0; i < kTotalIds; ++i) {
                if (r.admit("broadcaster", id_of(static_cast<std::uint32_t>(i)))) {
                    admitted.fetch_add(1, std::memory_order_relaxed);
                }
            }
            (void)t;
        });
    }
    for (auto& w : workers) {
        w.join();
    }
    EXPECT_EQ(admitted.load(), kTotalIds);
}
