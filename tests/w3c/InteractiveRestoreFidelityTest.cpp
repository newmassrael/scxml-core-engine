// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/**
 * @file InteractiveRestoreFidelityTest.cpp
 * @brief W3C SCXML 6.4 across a time-travel restore
 *
 * A branch taken from a restored step must run from the step the recorded run
 * was in: the same queue, and the same invoked sessions, each as running or as
 * ended as it was there.
 *
 * Restoring a step cancels every live invocation and re-creates the ones the
 * step recorded. Cancelling a session that is still running purges what it
 * sent from the parent's queue (W3C test 252); cancelling one that has ended
 * purges nothing, because what it sent before it ended — its done.invoke among
 * it — is still the parent's (W3C test 236). Three defects made a restore
 * disagree with the recorded run:
 *
 *   - the recorded queue was put back before the teardown ran, so tearing down
 *     a running child purged events the recorded step still held;
 *   - every child came back running, so a child that had ended was purged like
 *     a running one when the branch left its invoking state;
 *   - a step that recorded no invocations skipped the teardown, so a child
 *     started after it went on running, reachable as `#_<invokeid>`.
 *
 * Replay from the recorded snapshots cannot show any of them: it restores every
 * step whole. A branch can — injecting an event discards the recorded future
 * and runs from the restored state — so each case below branches, and each
 * isolates one defect: fixing any two leaves the third case red.
 */

#include "InteractiveTestRunner.h"
#include "common/Logger.h"

#include <algorithm>
#include <gtest/gtest.h>
#include <string>

namespace SCE::W3C {
namespace {

/// The child ends at once and sends `childToParent` from its final's exit
/// handler; the parent leaves the invoking state on it, and done.invoke still
/// reaches it there — W3C test 236's shape.
constexpr const char *ENDED_CHILD = R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="invoking">
  <state id="invoking">
    <invoke id="child" type="http://www.w3.org/TR/scxml/">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="end">
          <final id="end">
            <onexit><send target="#_parent" event="childToParent"/></onexit>
          </final>
        </scxml>
      </content>
    </invoke>
    <transition event="childToParent" target="left"/>
  </state>
  <state id="left">
    <transition event="done.invoke.child" target="reached"/>
  </state>
  <final id="reached"/>
</scxml>
)";

/// The child greets the parent and goes on running.
constexpr const char *RUNNING_CHILD = R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="waiting">
  <state id="waiting">
    <invoke id="child" type="http://www.w3.org/TR/scxml/">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="talking">
          <state id="talking">
            <onentry><send target="#_parent" event="hello"/></onentry>
          </state>
        </scxml>
      </content>
    </invoke>
    <transition event="hello" target="reached"/>
  </state>
  <final id="reached"/>
</scxml>
)";

/// The child exists only in `invoking`, which the parent enters on `go`, and
/// answers `ping` with `pong`. From `idle` the parent sends `ping` to it: a
/// session that is gone cannot answer, and one that is still there does.
constexpr const char *LATER_CHILD = R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="idle">
  <state id="idle">
    <transition event="go" target="invoking"/>
    <transition event="probe" target="probed">
      <send target="#_child" event="ping"/>
    </transition>
  </state>
  <state id="invoking">
    <invoke id="child" type="http://www.w3.org/TR/scxml/">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="listening">
          <state id="listening">
            <transition event="ping"><send target="#_parent" event="pong"/></transition>
          </state>
        </scxml>
      </content>
    </invoke>
  </state>
  <state id="probed">
    <transition event="pong" target="answered"/>
  </state>
  <final id="answered"/>
</scxml>
)";

bool isActive(const InteractiveTestRunner &runner, const std::string &state) {
    const auto states = runner.getActiveStates();
    return std::find(states.begin(), states.end(), state) != states.end();
}

/// Step until a step reports anything but a success; returns that report.
StepResult stepToRest(InteractiveTestRunner &runner) {
    StepResult result = StepResult::SUCCESS;
    for (int step = 0; step < 20 && result == StepResult::SUCCESS; ++step) {
        result = runner.stepForward();
    }
    return result;
}

}  // namespace

class InteractiveRestoreFidelityTest : public ::testing::Test {
protected:
    void SetUp() override {
        Logger::setLevel(LogLevel::Warn);
    }
};

/// The running flag: a child that had ended in the recorded run comes back
/// ended, so leaving its invoking state in the branch purges nothing.
TEST_F(InteractiveRestoreFidelityTest, AChildThatHadEndedIsRestoredEnded) {
    InteractiveTestRunner runner;
    ASSERT_TRUE(runner.loadSCXML(ENDED_CHILD, /*isFilePath=*/false));
    ASSERT_TRUE(runner.initialize());

    ASSERT_EQ(stepToRest(runner), StepResult::FINAL_STATE) << "the recorded run reaches `reached`";
    ASSERT_EQ(runner.getTerminalState(), "reached");

    while (runner.stepBackward()) {
    }
    ASSERT_EQ(runner.getCurrentStep(), 0);
    runner.raiseEvent("branch");

    EXPECT_EQ(stepToRest(runner), StepResult::FINAL_STATE)
        << "the child had ended when the recorded run was here, so leaving `invoking` cancels a session with "
           "nothing left to stop and done.invoke survives it; a child revived as running is purged instead, and "
           "the branch is left in `left` with nothing to take";
    EXPECT_EQ(runner.getTerminalState(), "reached");
}

/// The order: the recorded queue goes back after the teardown, so tearing down
/// a running child cannot purge what the recorded step still held.
TEST_F(InteractiveRestoreFidelityTest, ARestoreKeepsWhatARunningChildHadSent) {
    InteractiveTestRunner runner;
    ASSERT_TRUE(runner.loadSCXML(RUNNING_CHILD, /*isFilePath=*/false));
    ASSERT_TRUE(runner.initialize());

    ASSERT_EQ(stepToRest(runner), StepResult::FINAL_STATE) << "the recorded run takes `hello` to `reached`";

    // Leaving `waiting` cancelled the child, so the first restore meets no live
    // session; it re-creates the child, running as it was, and the reset's
    // restore is the one whose teardown meets a running child.
    ASSERT_TRUE(runner.stepBackward());
    runner.reset();
    runner.raiseEvent("branch");

    EXPECT_EQ(stepToRest(runner), StepResult::FINAL_STATE)
        << "`hello` was queued when the recorded run was here; a restore that put the queue back first and "
           "then tore the running child down purged it, and the branch had nothing to take";
    EXPECT_EQ(runner.getTerminalState(), "reached");
}

/// The teardown: a step that recorded no invocations restores to none, so a
/// child started after it is gone and `#_child` reaches nothing.
TEST_F(InteractiveRestoreFidelityTest, ARestoreToAStepBeforeAnInvocationEndsIt) {
    InteractiveTestRunner runner;
    ASSERT_TRUE(runner.loadSCXML(LATER_CHILD, /*isFilePath=*/false));
    ASSERT_TRUE(runner.initialize());

    runner.raiseEvent("go");
    ASSERT_EQ(runner.stepForward(), StepResult::SUCCESS) << "`go` enters `invoking`, which starts the child";
    ASSERT_TRUE(isActive(runner, "invoking"));

    ASSERT_TRUE(runner.stepBackward());
    ASSERT_TRUE(isActive(runner, "idle"));
    ASSERT_TRUE(runner.removeExternalEvent(0)) << "drop `go`, so the branch never enters `invoking` itself";
    runner.raiseEvent("probe");
    stepToRest(runner);

    EXPECT_FALSE(isActive(runner, "answered"))
        << "no child existed at the restored step, so `ping` has no session to reach (W3C SCXML 6.4); a pong "
           "means the child started after that step went on running through the restore";
    EXPECT_TRUE(isActive(runner, "probed")) << "and so the branch rests where `probe` took it";
}

}  // namespace SCE::W3C
