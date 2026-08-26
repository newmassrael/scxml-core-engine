// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Appendix D removeConflictingTransitions: a transition that leaves a
// `<parallel>` preempts a sibling region's transition on the same event.
//
// This is the fixture G2 asked for. The registry recorded that C++ and Go carry
// spec-alien heuristics in conflict resolution and that removing them broke
// nothing — "관측된 결함 없음". Measured 2026-08-26: deleting them leaves the
// whole W3C suite green at 404 cases, so the suite cannot see the difference and
// the heuristics' worth could not be judged from it. This document can see it.
//
// Two regions of one `<parallel>`. On `go`:
//
//   - region `a` takes a transition whose target is OUTSIDE the parallel, so its
//     transition domain is the `<scxml>` element and it exits `p` entirely;
//   - region `b` takes a transition that stays inside `b`.
//
// The appendix computes an exit set from the CONFIGURATION — every active state
// that is a descendant of the domain — so `a`'s exit set contains `b1`, the two
// intersect, and `b`'s transition is preempted. Its action must not run.
//
// ⚠ Measured 2026-08-27: this document passes BOTH before and after the exit
// set was made the appendix's, so it does not discriminate that change. The
// parallel-ancestor rule still standing in `ConflictResolutionAlgorithms`
// reaches the same verdict on its own — `a`'s exit set contains `p`, and `b1`
// descends from `p`. What this test holds shut is the REMOVAL of that rule: with
// a chain-shaped exit set (`{a1, a, p}` against `{b1}` — disjoint) and no
// heuristic, both transitions fire. The exit set itself is witnessed by
// `TransitionDomainSelfTransition.ExitSetNamesTheSiblingRegionUnderTheDomain`.
//
// So the assertion below is on the ACTION, not the configuration. Both engines
// end in `done` either way; what separates them is whether region `b`'s
// `<assign>` ran while the region it lives in was being exited.

#include "runtime/EventRaiserImpl.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <gtest/gtest.h>

namespace SCE {
namespace Tests {

namespace {

constexpr const char *DOCUMENT = R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       datamodel="ecmascript" initial="p">

  <datamodel>
    <data id="siblingRan" expr="0"/>
  </datamodel>

  <parallel id="p">
    <state id="a" initial="a1">
      <state id="a1">
        <!-- Target outside the parallel: the domain is <scxml>, so this exits p. -->
        <transition event="go" target="done"/>
      </state>
    </state>
    <state id="b" initial="b1">
      <state id="b1">
        <!-- Stays inside region b. Appendix D: preempted by the transition
             above, because that one exits every state under the domain and
             this one's source is among them. -->
        <transition event="go" target="b2">
          <assign location="siblingRan" expr="1"/>
        </transition>
      </state>
      <state id="b2"/>
    </state>
  </parallel>

  <final id="done"/>
</scxml>
)";

}  // namespace

class ParallelExitPreemptsSiblingRegionTest : public ::testing::Test {
protected:
    void SetUp() override {
        engine_ = &ScriptEngineProvider::getScriptEngine();
        engine_->reset();

        sm_ = std::make_shared<StateMachine>(*engine_);
        sm_->setEventRaiser(std::make_shared<EventRaiserImpl>());
        ASSERT_TRUE(sm_->loadSCXMLFromString(DOCUMENT));
        ASSERT_TRUE(sm_->start());
    }

    void TearDown() override {
        sm_.reset();
        if (engine_) {
            engine_->shutdown();
        }
    }

    std::string read(const std::string &expr) {
        auto result = engine_->evaluateExpression(sm_->getSessionId(), expr).get();
        EXPECT_TRUE(result.isSuccess()) << "`" << expr << "` is readable in this session";
        return result.isSuccess() ? result.getValueAsString() : std::string("<unreadable>");
    }

    IScriptEngine *engine_ = nullptr;
    std::shared_ptr<StateMachine> sm_;
};

/// The axis: the sibling region's transition is preempted, so its action does
/// not run.
TEST_F(ParallelExitPreemptsSiblingRegionTest, LeavingTheParallelPreemptsTheOtherRegion) {
    ASSERT_TRUE(sm_->processEvent("go").success) << "`go` matches a transition in each region";

    EXPECT_EQ(read("siblingRan"), "0")
        << "region `b`'s transition ran its `<assign>` while region `b` was being exited by the "
           "transition leaving `p`. Appendix D removeConflictingTransitions preempts it: the "
           "exit set of a transition whose domain is <scxml> contains every active state under "
           "that domain, `b1` among them, so the two exit sets intersect";
}

}  // namespace Tests
}  // namespace SCE
