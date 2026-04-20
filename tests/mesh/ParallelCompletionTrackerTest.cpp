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

#include "mesh/ParallelCompletionTracker.h"

#include <gtest/gtest.h>

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
