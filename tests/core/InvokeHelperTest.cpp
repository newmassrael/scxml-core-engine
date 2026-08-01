// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-D-GlobalVariables: `statesToInvoke` is a set.
//
// Appendix D declares it `global statesToInvoke` and `enterStates` schedules
// with `statesToInvoke.add(s)`, so scheduling a state that is already
// scheduled is a no-op. `InvokeHelper` is the Single Source of Truth both
// engines schedule through, which is where that property has to live.
//
// It matters because one state entry can reach the scheduler more than once.
// The Interpreter's `StateHierarchyManager` signals entry through two hooks —
// an onentry callback whose body defers, and a dedicated invoke-defer
// callback — and an atomic state trips both. Without the set property the
// same `<invoke>` reaches `InvokeExecutor::executeInvoke` twice and is only
// stopped there by an "already active" guard, which is a defensive check
// doing a data-structure's job and which would hide a genuine double
// invocation just as effectively.

#include "core/InvokeHelper.h"

#include <gtest/gtest.h>
#include <string>
#include <vector>

namespace {

/// Minimal stand-in for the engines' PendingInvoke: the helper's contract is
/// only that the entry exposes `invokeId` and `state`.
struct PendingInvoke {
    std::string invokeId;
    std::string state;
};

using Pending = std::vector<PendingInvoke>;

TEST(InvokeHelperScheduling, SchedulesAnInvokeThatIsNotYetPending) {
    Pending pending;

    EXPECT_TRUE(SCE::Core::InvokeHelper::deferInvoke(pending, PendingInvoke{"inv_a", "phase"}));
    ASSERT_EQ(pending.size(), 1u);
    EXPECT_EQ(pending[0].invokeId, "inv_a");
    EXPECT_EQ(pending[0].state, "phase");
}

TEST(InvokeHelperScheduling, DoesNotScheduleTheSameInvokeTwiceForOneState) {
    Pending pending;

    ASSERT_TRUE(SCE::Core::InvokeHelper::deferInvoke(pending, PendingInvoke{"inv_a", "phase"}));

    EXPECT_FALSE(SCE::Core::InvokeHelper::deferInvoke(pending, PendingInvoke{"inv_a", "phase"}))
        << "§scxml-D-GlobalVariables makes `statesToInvoke` a set — a second `add` for an "
           "already-scheduled state is a no-op, so a second entry signal for the same state "
           "must not queue the invoke again";
    EXPECT_EQ(pending.size(), 1u) << "a duplicate schedule reaches executeInvoke twice and only "
                                     "survives because that layer refuses an already-active id";
}

TEST(InvokeHelperScheduling, KeepsSiblingInvokesOfOneStateDistinct) {
    Pending pending;

    ASSERT_TRUE(SCE::Core::InvokeHelper::deferInvoke(pending, PendingInvoke{"inv_a", "phase"}));

    EXPECT_TRUE(SCE::Core::InvokeHelper::deferInvoke(pending, PendingInvoke{"inv_b", "phase"}))
        << "W3C SCXML 6.4 allows several `<invoke>` elements in one state; the set is keyed on "
           "(state, invokeId), so distinct ids must both schedule";
    EXPECT_EQ(pending.size(), 2u);
}

TEST(InvokeHelperScheduling, KeepsTheSameInvokeIdInDifferentStatesDistinct) {
    Pending pending;

    ASSERT_TRUE(SCE::Core::InvokeHelper::deferInvoke(pending, PendingInvoke{"inv_a", "phase"}));

    EXPECT_TRUE(SCE::Core::InvokeHelper::deferInvoke(pending, PendingInvoke{"inv_a", "other"}))
        << "the set is over states, not over invoke ids alone — two states each carrying an "
           "invoke of the same id are two separate schedule entries";
    EXPECT_EQ(pending.size(), 2u);
}

TEST(InvokeHelperScheduling, ReschedulesAfterTheInvokesHaveBeenExecuted) {
    Pending pending;

    ASSERT_TRUE(SCE::Core::InvokeHelper::deferInvoke(pending, PendingInvoke{"inv_a", "phase"}));

    int executed = 0;
    SCE::Core::InvokeHelper::executePendingInvokes(pending, [&executed](const PendingInvoke &) { ++executed; });
    EXPECT_EQ(executed, 1);
    ASSERT_TRUE(pending.empty()) << "Appendix D clears `statesToInvoke` right after invoking";

    EXPECT_TRUE(SCE::Core::InvokeHelper::deferInvoke(pending, PendingInvoke{"inv_a", "phase"}))
        << "the set is per-macrostep — re-entering the state in a later macrostep schedules again";
    EXPECT_EQ(pending.size(), 1u);
}

}  // namespace
