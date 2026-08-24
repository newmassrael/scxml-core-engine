// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix D: a <parallel> is not a transition domain -- Interpreter
// path.
//
// Sibling of `ParallelRegionRootExternalDomainAotTest.cpp`, which pins the same
// clause against the generated machine. The two engines reach the domain by
// different routes and had to be repaired in different places: the AOT engine
// asks `HierarchicalStateHelper::findLCCA` over the State enum, while the
// Interpreter asks `TransitionDomainCalculator` over the parsed model tree by
// state id. A repair to one is not a repair to the other, so the clause is
// asked of both -- the recurring way this repository's backends drift apart is
// a rule that only ever had one channel's witness.
//
// Fixture: tests/integration/parallel_region_root_external_domain.scxml

#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <algorithm>
#include <fstream>
#include <gtest/gtest.h>
#include <memory>
#include <sstream>
#include <string>
#include <vector>

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

namespace {

/// Rendered into every failure message; sorted so the comparison does not
/// depend on the order the engine happens to report its configuration in.
std::string describe(const std::shared_ptr<StateMachine> &sm) {
    auto states = sm->getActiveStates();
    std::sort(states.begin(), states.end());
    std::string out = "[";
    for (const auto &state : states) {
        if (out.size() > 1) {
            out += " | ";
        }
        out += state;
    }
    return out + "]";
}

std::shared_ptr<StateMachine> startFixture() {
    const std::string fixture =
        std::string(SCE_PROJECT_ROOT) + "/tests/integration/parallel_region_root_external_domain.scxml";
    std::ifstream in(fixture);
    if (!in.is_open()) {
        ADD_FAILURE() << "fixture not readable: " << fixture;
        return nullptr;
    }
    std::ostringstream buffer;
    buffer << in.rdbuf();

    auto sm = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());
    if (!sm->loadSCXMLFromString(buffer.str()) || !sm->start()) {
        ADD_FAILURE() << "fixture did not start";
        return nullptr;
    }
    return sm;
}

}  // namespace

TEST(ParallelRegionRootExternalDomainTest, AnExternalRegionRootTransitionExitsEveryRegion) {
    auto sm = startFixture();
    ASSERT_NE(sm, nullptr);

    ASSERT_TRUE(sm->isStateActive("working")) << "precondition; active: " << describe(sm);
    ASSERT_TRUE(sm->isStateActive("alive")) << "precondition; active: " << describe(sm);

    sm->processEvent("restart");

    // The whole configuration, not a handful of membership questions: the way
    // this defect presented in the AOT engine was an ILLEGAL configuration --
    // `alive` and `rebuilding`, two children of the same compound region, both
    // active at once -- which every individual `isStateActive` call answers
    // "true" to.
    EXPECT_EQ(describe(sm), "[alive | drive | restarting | run | watch]")
        << "an external transition on a region root has the DOCUMENT ROOT as its domain "
           "(Appendix D findLCCA filters <parallel> out of the candidate ancestors), so every "
           "region exits and re-enters, `watch` is back at its default, and `watch`'s own "
           "transition on the same event is preempted as conflicting";
}

TEST(ParallelRegionRootExternalDomainTest, AnInternalRegionRootTransitionLeavesTheOtherRegion) {
    auto sm = startFixture();
    ASSERT_NE(sm, nullptr);

    sm->processEvent("hold");

    // The contrast, and the reason the ai_loop document is spelled the way it
    // is. A test that only pinned the external case would pass just as well on
    // an engine that sent EVERY region-root transition to the document root.
    EXPECT_EQ(describe(sm), "[drive | paused | rebuilding | run | watch]")
        << "an internal region-root transition has the region as its domain (source compound, "
           "target its descendant), so the sibling region never exits and answers the event "
           "itself";
}

}  // namespace Tests
}  // namespace SCE
