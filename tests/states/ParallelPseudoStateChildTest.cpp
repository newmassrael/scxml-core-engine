// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// W3C SCXML 3.4 — which children of <parallel> become concurrent regions.
//
// Appendix D defines getChildStates(state) as the <state>, <final>, and
// <parallel> children of a state; <history> and <initial> are pseudo-states
// and are not among them. §3.10 nonetheless makes <history> a legal child of
// <parallel>, so the two rules meet in exactly one place: a <parallel> that
// declares a history child.
//
// ConcurrentStateNode::addChild used to wrap every child in a
// ConcurrentRegion. The extra region can never reach a final state, so the
// parallel's done.state.<id> — which requires every region to be final —
// would never fire. Nothing caught it because the W3C suite has no fixture
// pairing <parallel> with <history>.

#include "model/StateNode.h"
#include "states/ConcurrentStateNode.h"
#include "gtest/gtest.h"
#include <memory>

namespace SCE {
namespace {

TEST(ParallelPseudoStateChild, StateChildBecomesARegion) {
    ConcurrentStateNode parallel("p");
    parallel.addChild(std::make_shared<StateNode>("r1", Type::COMPOUND));

    EXPECT_EQ(parallel.getRegions().size(), 1u);
}

TEST(ParallelPseudoStateChild, HistoryChildIsNotARegion) {
    ConcurrentStateNode parallel("p");
    parallel.addChild(std::make_shared<StateNode>("r1", Type::COMPOUND));
    parallel.addChild(std::make_shared<StateNode>("pHist", Type::HISTORY));

    EXPECT_EQ(parallel.getRegions().size(), 1u);
}

TEST(ParallelPseudoStateChild, InitialChildIsNotARegion) {
    ConcurrentStateNode parallel("p");
    parallel.addChild(std::make_shared<StateNode>("r1", Type::COMPOUND));
    parallel.addChild(std::make_shared<StateNode>("pInit", Type::INITIAL));

    EXPECT_EQ(parallel.getRegions().size(), 1u);
}

TEST(ParallelPseudoStateChild, PseudoStateChildStaysReachableAsAChild) {
    // Transitions target a history state by id and the history registrar walks
    // this same list, so excluding it from the regions must not exclude it
    // from the children.
    ConcurrentStateNode parallel("p");
    parallel.addChild(std::make_shared<StateNode>("r1", Type::COMPOUND));
    parallel.addChild(std::make_shared<StateNode>("pHist", Type::HISTORY));

    ASSERT_EQ(parallel.getChildren().size(), 2u);
    EXPECT_EQ(parallel.getChildren()[1]->getId(), "pHist");
}

TEST(ParallelPseudoStateChild, FinalChildBecomesARegion) {
    // <final> is in getChildStates, so it keeps its region.
    ConcurrentStateNode parallel("p");
    parallel.addChild(std::make_shared<StateNode>("r1", Type::COMPOUND));
    parallel.addChild(std::make_shared<StateNode>("done", Type::FINAL));

    EXPECT_EQ(parallel.getRegions().size(), 2u);
}

}  // namespace
}  // namespace SCE
