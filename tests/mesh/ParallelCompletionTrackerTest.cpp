// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// ParallelCompletionTracker unit tests — SCE_MESH.md §16.5.
//
// Covers:
//   1. Threshold firing on local-only completion path (single-partition
//      shouldn't instantiate the tracker, but the path is used by the
//      root branch when all regions happen to live on root).
//   2. Threshold firing on mixed local+remote completion.
//   3. Single-shot per activation (duplicate region reports do not
//      re-fire the callback, §16.5 L3498).
//   4. Reset semantics (re-entry starts a fresh activation).
//   5. §16.5 L3500 barrier-timeout hooks: arm on first completion,
//      re-arm with fresh `missing_regions` on subsequent completions,
//      cancel at threshold, cancel on reset.

#include "mesh/ParallelCompletionTracker.h"

#include <cstdint>
#include <gtest/gtest.h>
#include <vector>

using SCE::Mesh::ParallelCompletionTracker;

TEST(ParallelCompletionTracker, FiresOnAllLocalComplete) {
    int fire_count = 0;
    ParallelCompletionTracker tracker(2, [&] { ++fire_count; });

    tracker.onLocalRegionComplete("left");
    EXPECT_FALSE(tracker.hasFired());
    EXPECT_EQ(tracker.completedCount(), 1u);

    tracker.onLocalRegionComplete("right");
    EXPECT_TRUE(tracker.hasFired());
    EXPECT_EQ(fire_count, 1);
}

TEST(ParallelCompletionTracker, FiresOnMixedLocalRemoteComplete) {
    int fire_count = 0;
    ParallelCompletionTracker tracker(3, [&] { ++fire_count; });

    tracker.onLocalRegionComplete("local_region");
    tracker.onRemoteRegionComplete("remote_a");
    EXPECT_FALSE(tracker.hasFired());

    tracker.onRemoteRegionComplete("remote_b");
    EXPECT_TRUE(tracker.hasFired());
    EXPECT_EQ(fire_count, 1);
}

TEST(ParallelCompletionTracker, DuplicateRegionReportsAreIgnored) {
    // §16.5 L3498: "single-shot per region activation". At-least-once
    // transport re-delivery must not over-count.
    int fire_count = 0;
    ParallelCompletionTracker tracker(2, [&] { ++fire_count; });

    tracker.onRemoteRegionComplete("left");
    tracker.onRemoteRegionComplete("left");  // duplicate re-delivery
    tracker.onRemoteRegionComplete("left");  // still a duplicate
    EXPECT_FALSE(tracker.hasFired());
    EXPECT_EQ(tracker.completedCount(), 1u);

    tracker.onLocalRegionComplete("right");
    EXPECT_TRUE(tracker.hasFired());
    EXPECT_EQ(fire_count, 1);
}

TEST(ParallelCompletionTracker, FiresExactlyOncePerActivation) {
    // Once threshold is reached, further reports do not re-fire. The
    // generated SM calls reset() on `<parallel>` re-entry for a fresh
    // activation.
    int fire_count = 0;
    ParallelCompletionTracker tracker(1, [&] { ++fire_count; });

    tracker.onLocalRegionComplete("only");
    EXPECT_TRUE(tracker.hasFired());
    EXPECT_EQ(fire_count, 1);

    // Post-fire reports are silently absorbed.
    tracker.onRemoteRegionComplete("spurious");
    EXPECT_EQ(fire_count, 1);
}

TEST(ParallelCompletionTracker, ResetStartsFreshActivation) {
    // §16.5 L3498: re-entry resets the tracker.
    int fire_count = 0;
    ParallelCompletionTracker tracker(2, [&] { ++fire_count; });

    tracker.onLocalRegionComplete("left");
    tracker.onRemoteRegionComplete("right");
    EXPECT_TRUE(tracker.hasFired());
    EXPECT_EQ(fire_count, 1);

    tracker.reset();
    EXPECT_FALSE(tracker.hasFired());
    EXPECT_EQ(tracker.completedCount(), 0u);

    tracker.onLocalRegionComplete("left");
    tracker.onRemoteRegionComplete("right");
    EXPECT_TRUE(tracker.hasFired());
    EXPECT_EQ(fire_count, 2);
}

// ── §16.5 L3500 barrier-timeout hooks ───────────────────────────────

namespace {

struct ArmCall {
    std::uint64_t timeout_ms;
    std::vector<std::string> missing_regions;
};

ParallelCompletionTracker::TimerHooks makeHooks(std::uint64_t timeout_ms, std::vector<std::string> region_ids,
                                                std::vector<ArmCall> *arm_log, int *cancel_count) {
    ParallelCompletionTracker::TimerHooks hooks;
    hooks.timeout_ms = timeout_ms;
    hooks.expected_region_ids.insert(region_ids.begin(), region_ids.end());
    hooks.arm = [arm_log](std::uint64_t ms, std::vector<std::string> missing) {
        arm_log->push_back(ArmCall{ms, std::move(missing)});
    };
    hooks.cancel = [cancel_count] { ++(*cancel_count); };
    return hooks;
}

}  // namespace

TEST(ParallelCompletionTracker, TimerHooksArmOnFirstAndCancelOnThreshold) {
    std::vector<ArmCall> arms;
    int cancels = 0;
    int fires = 0;

    ParallelCompletionTracker tracker(2, [&] { ++fires; }, makeHooks(5000, {"left", "right"}, &arms, &cancels));

    EXPECT_FALSE(tracker.barrierTimerArmed());
    EXPECT_EQ(arms.size(), 0u);
    EXPECT_EQ(cancels, 0);

    tracker.onLocalRegionComplete("left");
    // First completion arms the timer with missing = {right}.
    ASSERT_EQ(arms.size(), 1u);
    EXPECT_EQ(arms[0].timeout_ms, 5000u);
    EXPECT_EQ(arms[0].missing_regions, std::vector<std::string>{"right"});
    EXPECT_TRUE(tracker.barrierTimerArmed());
    EXPECT_EQ(cancels, 0);
    EXPECT_FALSE(tracker.hasFired());

    tracker.onRemoteRegionComplete("right");
    // Threshold reached ⇒ cancel precedes the on_complete fire.
    EXPECT_EQ(cancels, 1);
    EXPECT_FALSE(tracker.barrierTimerArmed());
    EXPECT_TRUE(tracker.hasFired());
    EXPECT_EQ(fires, 1);
}

TEST(ParallelCompletionTracker, TimerHooksRearmWithFreshMissingRegions) {
    std::vector<ArmCall> arms;
    int cancels = 0;
    int fires = 0;

    ParallelCompletionTracker tracker(
        3, [&] { ++fires; }, makeHooks(2500, {"left", "middle", "right"}, &arms, &cancels));

    tracker.onLocalRegionComplete("left");
    ASSERT_EQ(arms.size(), 1u);
    EXPECT_EQ(arms[0].missing_regions, (std::vector<std::string>{"middle", "right"}));
    EXPECT_EQ(cancels, 0);

    tracker.onRemoteRegionComplete("middle");
    // Re-arm: cancel the stale "{middle,right}" arm, arm fresh "{right}".
    EXPECT_EQ(cancels, 1);
    ASSERT_EQ(arms.size(), 2u);
    EXPECT_EQ(arms[1].missing_regions, std::vector<std::string>{"right"});
    EXPECT_EQ(arms[1].timeout_ms, 2500u);
    EXPECT_FALSE(tracker.hasFired());

    tracker.onLocalRegionComplete("right");
    // Threshold ⇒ cancel current arm, fire on_complete.
    EXPECT_EQ(cancels, 2);
    EXPECT_EQ(fires, 1);
    EXPECT_FALSE(tracker.barrierTimerArmed());
}

TEST(ParallelCompletionTracker, TimerHooksResetCancelsArmedTimer) {
    std::vector<ArmCall> arms;
    int cancels = 0;
    ParallelCompletionTracker tracker(2, [] {}, makeHooks(1000, {"left", "right"}, &arms, &cancels));

    tracker.onLocalRegionComplete("left");
    EXPECT_TRUE(tracker.barrierTimerArmed());
    EXPECT_EQ(arms.size(), 1u);

    tracker.reset();
    // reset() must cancel the armed timer, clear completion state, and
    // re-enable firing for the next activation.
    EXPECT_EQ(cancels, 1);
    EXPECT_FALSE(tracker.barrierTimerArmed());
    EXPECT_EQ(tracker.completedCount(), 0u);
    EXPECT_FALSE(tracker.hasFired());
}

TEST(ParallelCompletionTracker, DisabledTimerHooksSkipsArmAndCancel) {
    // Default-constructed TimerHooks ⇒ W3C normative infinity. Tracker
    // must never invoke arm / cancel, and behaviour matches the pre-
    // L3500 threshold-only primitive.
    int arm_calls = 0;
    int cancel_calls = 0;
    ParallelCompletionTracker::TimerHooks disabled;  // arm/cancel are nullptr
    // Populate region ids to prove they're ignored when disabled.
    disabled.expected_region_ids.insert("left");
    disabled.expected_region_ids.insert("right");
    disabled.timeout_ms = 1000;

    int fires = 0;
    ParallelCompletionTracker tracker(2, [&] { ++fires; }, std::move(disabled));

    tracker.onLocalRegionComplete("left");
    tracker.onRemoteRegionComplete("right");
    EXPECT_EQ(arm_calls, 0);
    EXPECT_EQ(cancel_calls, 0);
    EXPECT_EQ(fires, 1);
    EXPECT_FALSE(tracker.barrierTimerArmed());
}
