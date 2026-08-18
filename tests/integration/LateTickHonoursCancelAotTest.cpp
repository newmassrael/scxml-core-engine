// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2 + 6.3: a `<cancel>` still lands when the host ticked late —
// C++ AOT path.
//
// The scheduler queue is ordered by (fireTime, sequence) and `tick()` drains
// it. Draining it to exhaustion before running a macrostep is the defect: a
// host that wakes after two fire times have passed holds both entries, and
// raising both onto the external queue makes the second undroppable before the
// first one's transitions have run. The `<cancel>` then executes against a
// queue the event has already left.
//
// The host below sleeps past BOTH fire times before its first tick, because
// that is the only condition under which the two dispatch orders differ. A host
// that wakes between them passes either way, which is why every existing suite
// was blind to this.
//
// Sibling of `LateTickHonoursCancelTest.cpp` (Interpreter channel).
//
// Fixture: integration_resources/late_tick_honours_cancel/late_tick_honours_cancel.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(late_tick_honours_cancel ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "late_tick_honours_cancel_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <gtest/gtest.h>
#include <memory>
#include <thread>

namespace SCE::Tests {
namespace {

using SM = SCE::Generated::late_tick_honours_cancel::late_tick_honours_cancel;

/// Past both `<send delay>`s in `waiting` (100 ms and 200 ms), with margin for
/// a loaded machine.
constexpr auto PAST_BOTH_DEADLINES = std::chrono::milliseconds(400);

std::unique_ptr<SM> started() {
    auto sm = std::make_unique<SM>();
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm->setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                  [](::SCE::IScriptEngine *) {}));
    }
    sm->initialize();
    return sm;
}

}  // namespace

/// The fixture is only meaningful on a scheduler-driven machine, and the policy
/// is where a consumer reads that without running anything.
TEST(LateTickHonoursCancelAotTest, TheFixtureIsSchedulerDriven) {
    EXPECT_TRUE(SM::PolicyType::NEEDS_EVENT_SCHEDULER)
        << "the fixture arms two delayed `<send>`s; a policy that does not declare "
           "NEEDS_EVENT_SCHEDULER means the document lost them, and every assertion "
           "below would then be measuring the wrong machine";
}

/// The axis: one tick, taken after both deadlines passed, must still deliver
/// `poke` first and let `active`'s `<cancel sendid="s1">` drop `settle`.
TEST(LateTickHonoursCancelAotTest, ACancelSurvivesATickThatArrivesAfterBothDeadlines) {
    auto sm = started();
    ASSERT_EQ(sm->getCurrentState(), SM::State::Waiting) << "the machine should be waiting on its two delayed sends";

    std::this_thread::sleep_for(PAST_BOTH_DEADLINES);
    sm->tick();

    EXPECT_NE(sm->getCurrentState(), SM::State::CancelLost)
        << "`settle` was delivered even though `active`'s `<cancel sendid=\"s1\">` ran "
           "first. Both entries were past due when this tick started, so the scheduler "
           "drain raised them together and the cancel found nothing left to drop. W3C "
           "SCXML 6.3 cancels a send that has not been dispatched — dispatch is one "
           "entry per macrostep, not one queue-flush per tick";

    // The verdict is itself scheduler-driven, so a channel whose tick loop
    // stopped working fails here rather than passing by never moving.
    const bool completed = sm->runUntilCompletion(std::chrono::seconds(2));
    EXPECT_TRUE(completed) << "the machine did not complete after the cancel";
    EXPECT_EQ(sm->getCurrentState(), SM::State::Pass) << "the machine did not reach `pass` after the cancel";
}

/// A host that wakes between the two deadlines is the easy case, and it must
/// keep working — the fix is about the late wake-up, not about changing what a
/// punctual one does.
TEST(LateTickHonoursCancelAotTest, APunctualHostReachesTheSameVerdict) {
    auto sm = started();
    const bool completed = sm->runUntilCompletion(std::chrono::seconds(2), std::chrono::milliseconds(10));
    EXPECT_TRUE(completed) << "a 10 ms poll loop did not complete the machine";
    EXPECT_EQ(sm->getCurrentState(), SM::State::Pass)
        << "a 10 ms poll interval, which wakes between the 100 ms and 200 ms deadlines, "
           "must reach `pass`";
}

/// The deadline the host would have to guess is one the engine can state.
/// `runUntilCompletion` uses it, so an interval far coarser than the document's
/// delays no longer decides the outcome.
TEST(LateTickHonoursCancelAotTest, TheEngineSaysWhenItIsNextDue) {
    auto sm = started();

    const auto due = sm->timeUntilNextScheduled();
    ASSERT_TRUE(due.has_value()) << "two delayed sends are armed, so a deadline is owed";
    EXPECT_LE(due->count(), 100) << "the nearer of the two armed sends is 100 ms out; the engine answered "
                                 << due->count() << " ms, which would send a host past the earlier deadline";
    // The lower bound is the half that catches an answer of "due now", which
    // reads as a working query and costs the caller a spin that never sleeps.
    EXPECT_GT(due->count(), 0) << "the nearer send is 100 ms out and nothing is due yet, but the engine "
                                  "answered 0 ms. A host sleeping on that answer does not sleep at all";

    // A poll interval coarser than either delay: with the deadline in hand this
    // is a ceiling on the wait, not the wait itself.
    const auto startedAt = std::chrono::steady_clock::now();
    const bool completed = sm->runUntilCompletion(std::chrono::seconds(3), std::chrono::milliseconds(500));
    const auto took =
        std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::steady_clock::now() - startedAt);
    EXPECT_TRUE(completed) << "the machine did not complete within 3 s";
    EXPECT_EQ(sm->getCurrentState(), SM::State::Pass)
        << "a 500 ms poll interval decided the verdict — the wait must be shortened to "
           "the scheduler's own next deadline, or a coarse interval silently steps over "
           "the deadlines the document distinguishes between";
    // Correctness is not the whole of it: the document's own deadlines are
    // 100 ms + 100 ms, so an engine that sleeps the caller's interval regardless
    // finishes no sooner than 1 s. Timeliness is what the deadline query buys
    // once the dispatch order has made the verdict safe either way.
    EXPECT_LT(took.count(), 450) << "the machine's own deadlines total 200 ms, and it took " << took.count()
                                 << " ms — the poll interval was slept in full rather than shortened to the "
                                    "next deadline, so every delayed event lands as late as the caller's guess";

    EXPECT_FALSE(sm->timeUntilNextScheduled().has_value())
        << "nothing is scheduled once the machine is finished, so no wake-up is owed";
}

}  // namespace SCE::Tests
