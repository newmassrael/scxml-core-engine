// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A child session's `<send delay>` is a deadline of the machine that invoked
// it — C++ AOT path.
//
// A host that drives a scheduler-owning machine asks the engine when it next
// needs a tick and sleeps that long. The parent's `tick()` advances every
// running child by the same step, so a moment a child needs is a moment the host
// must not step over. An answer that counts only the parent's own scheduler
// tells a host with nothing of the parent's armed that there is nothing to wait
// for, and the child's timer is never fired.
//
// Driven entirely on `ManualClock`: no case sleeps, and each move lands exactly
// on the deadline the engine reported, which is the use the answer exists for.
//
// Fixture: tests/integration/a_child_timer_is_a_deadline_of_its_parent.scxml.
// It is outside `integration_resources/` for the reason
// `scripts/regen_a_child_timer_is_a_deadline_of_its_parent.sh` states.
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(a_child_timer_is_a_deadline_of_its_parent ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "a_child_timer_is_a_deadline_of_its_parent_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include "common/SceClock.h"

#include <chrono>
#include <gtest/gtest.h>
#include <memory>
#include <vector>

namespace SCE::Tests {
namespace {

using SM = SCE::Generated::a_child_timer_is_a_deadline_of_its_parent::a_child_timer_is_a_deadline_of_its_parent;

/// The machine on host-owned time. The clock is installed BEFORE `initialize()`:
/// the child arms its first timer as it starts, and the engine refuses a clock
/// afterwards because deadlines armed against one do not compare with another.
std::unique_ptr<SM> started() {
    auto sm = std::make_unique<SM>();
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm->setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                  [](::SCE::IScriptEngine *) {}));
    }
    sm->setClock(std::make_shared<SCE::ManualClock>(0));
    sm->initialize();
    return sm;
}

}  // namespace

/// The fixture is only meaningful on a scheduler-driven machine, and the policy
/// is where a consumer reads that without running anything.
TEST(AChildTimerIsADeadlineOfItsParentAotTest, TheFixtureIsSchedulerDriven) {
    EXPECT_TRUE(SM::PolicyType::NEEDS_EVENT_SCHEDULER)
        << "the child arms `<send delay>`s and the parent ticks it; a policy that does not declare "
           "NEEDS_EVENT_SCHEDULER means the document lost them, and every assertion below would "
           "then be measuring the wrong machine";
}

/// The parent arms nothing, so the only deadline in the run is the child's
/// first timer, 200 ms after the child started.
TEST(AChildTimerIsADeadlineOfItsParentAotTest, TheEngineNamesItsChildsFirstTimerAsItsOwnNextDeadline) {
    auto sm = started();
    ASSERT_EQ(sm->getCurrentState(), SM::State::Waiting) << "the parent should be waiting on its child";

    const auto due = sm->timeUntilNextScheduled();
    ASSERT_TRUE(due.has_value()) << "the child armed `<send delay=\"200ms\">` when it started and the parent ticks "
                                    "the child, so that deadline is the parent's. An answer of nullopt tells a host "
                                    "there is nothing to wait for while a running child's timer is pending";
    EXPECT_EQ(*due, std::chrono::milliseconds(200));
}

/// The child arms its second timer when the first fires, so the whole run is two
/// moves of 200 ms and the engine has to name the second one only after the
/// first has been taken.
TEST(AChildTimerIsADeadlineOfItsParentAotTest, AHostWalkingTimeByTheAnswerReachesTheEndOfTheChild) {
    auto sm = started();

    std::vector<std::chrono::milliseconds> walked;
    while (const auto due = sm->timeUntilNextScheduled()) {
        ASSERT_LT(walked.size(), 8u) << "the engine keeps naming deadlines";
        walked.push_back(*due);
        sm->advanceTimeMs(static_cast<uint64_t>(due->count()));
    }

    const std::vector<std::chrono::milliseconds> expected{std::chrono::milliseconds(200),
                                                          std::chrono::milliseconds(200)};
    EXPECT_EQ(walked, expected) << "each move should land on the child's next timer, the second of which is armed "
                                   "by the first firing";
    EXPECT_EQ(sm->terminalState(), SM::State::Finished)
        << "the child's last timer ended it, so the parent should have taken `done.invoke.kid` and finished";
    EXPECT_FALSE(sm->timeUntilNextScheduled().has_value()) << "nothing is armed once the child has ended";
}

}  // namespace SCE::Tests
