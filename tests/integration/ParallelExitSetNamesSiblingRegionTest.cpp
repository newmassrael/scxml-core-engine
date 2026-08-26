// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Appendix D computeExitSet, in the Interpreter: the exit set of a transition
// is every ACTIVE state that is a proper descendant of its domain — read off
// the configuration, not off one region's own chain.
//
// The document is the one `ParallelExitPreemptsSiblingRegionTest` uses. That
// test asserts the OUTCOME, which a rule about `<parallel>` ancestors in
// `ConflictResolutionAlgorithms` already reaches on its own, so it cannot see
// this. What it cannot see is the set the rule stands in FOR, and that set is
// published: `StateMachine::getLastEnabledTransitions()` hands out each enabled
// transition with the exit set the engine computed for it, and the interactive
// visualizer draws exactly this.
//
// Region `a` takes a transition whose target is outside the `<parallel>`, so
// Appendix D getTransitionDomain answers the `<scxml>` element and every active
// state is below it — region `b`'s `b1` included. An exit set assembled by
// walking region `a`'s own leaf up to the domain names `{a1, a, p}` and stops:
// it can never reach a sibling region, because a sibling region is not on that
// chain. `removeConflictingTransitions` INTERSECTS these sets, so the whole
// question of whether the two transitions conflict is decided on a set that
// cannot mention the state that makes them conflict.

#include "runtime/EventRaiserImpl.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <algorithm>
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
        <!-- Target outside the parallel: the domain is <scxml>. -->
        <transition event="go" target="done"/>
      </state>
    </state>
    <state id="b" initial="b1">
      <state id="b1">
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

class ParallelExitSetNamesSiblingRegionTest : public ::testing::Test {
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

    IScriptEngine *engine_ = nullptr;
    std::shared_ptr<StateMachine> sm_;
};

TEST_F(ParallelExitSetNamesSiblingRegionTest, TheExitSetIsReadOffTheConfiguration) {
    ASSERT_TRUE(sm_->processEvent("go").success) << "`go` matches a transition in each region";

    const auto enabled = sm_->getLastEnabledTransitions();
    ASSERT_FALSE(enabled.empty()) << "both regions reported an enabled transition on `go`";

    const auto leavingTheParallel =
        std::find_if(enabled.begin(), enabled.end(),
                     [](const TransitionDescriptorString &t) { return t.source == "a1" && t.target == "done"; });
    ASSERT_NE(leavingTheParallel, enabled.end())
        << "region `a`'s transition to `done` is not among the enabled transitions the engine published";

    const auto &exitSet = leavingTheParallel->exitSet;
    const auto names = [&exitSet](const std::string &id) {
        return std::find(exitSet.begin(), exitSet.end(), id) != exitSet.end();
    };

    // Appendix D computeExitSet over the domain `<scxml>`: every active state.
    EXPECT_TRUE(names("b1"))
        << "the sibling region's active leaf is absent from the exit set of a transition whose "
           "domain is the <scxml> element. The set was assembled from region `a`'s own leaf-to-domain "
           "chain, which cannot reach region `b`; removeConflictingTransitions then intersects a set "
           "that omits the very state making the two transitions conflict.";
    EXPECT_TRUE(names("b")) << "the sibling region root is a proper descendant of the domain too";
    EXPECT_TRUE(names("p")) << "the <parallel> itself is below the <scxml> domain and exits with it";
    EXPECT_TRUE(names("a1")) << "the source is still in its own exit set";
    EXPECT_TRUE(names("a")) << "the source's region root is below the domain";
}

}  // namespace Tests
}  // namespace SCE
