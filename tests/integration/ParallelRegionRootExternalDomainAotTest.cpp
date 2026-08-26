// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix D: a <parallel> is not a transition domain.
//
// `getTransitionDomain` sends an external transition to `findLCCA`, which
// filters the proper ancestors with `isCompoundStateOrScxmlElement`. A
// <parallel> is neither, so an external transition written on a REGION ROOT
// has the document root as its domain -- every region exits and re-enters,
// and a sibling region's transition on the same event is preempted because
// the two exit sets intersect and the sibling's source is not a descendant of
// this one's.
//
// The engine answered `run` here instead, because the domain rule it then used
// asked for a plain LCA -- the first common ancestor, whatever its kind. That
// rule is now `ExitSetAlgorithms::getTransitionDomain`, written once. That is the
// `findLCA` the appendix distinguishes from `findLCCA`, and the difference is
// invisible until a <parallel> sits between the source and the first compound
// <state> above it, which is exactly a region root.
//
// Measured 2026-08-25 on examples/ai_loop/ai_loop.scxml: the Kotlin engine,
// the only one implementing the filter, ended `session.lost` in
// [alive, restarting] where C++, Rust and Go ended in [rebuilding, restarting].
// That document was then repaired to say `type="internal"`, which is what its
// three region-root transitions meant -- and that repair is why this fixture
// exists rather than the ai_loop suite next door: with the document fixed,
// no committed document reaches the external form any more, so the engines'
// domain calculation had no witness at all.
//
// Fixture: tests/integration/parallel_region_root_external_domain.scxml

#include "parallel_region_root_external_domain_sm.h"

#include <algorithm>
#include <gtest/gtest.h>
#include <string>
#include <vector>

namespace SCE::Tests {

namespace {

using Machine = SCE::Generated::parallel_region_root_external_domain::parallel_region_root_external_domain;

std::vector<Machine::State> active(Machine &sm) {
    return sm.getPolicy().getActiveStates();
}

bool holds(Machine &sm, Machine::State state) {
    const auto states = active(sm);
    return std::find(states.begin(), states.end(), state) != states.end();
}

/// Rendered into every failure message; a bare enum tells the reader nothing.
std::string describe(Machine &sm) {
    std::string out = "[";
    for (const auto state : active(sm)) {
        if (out.size() > 1) {
            out += " | ";
        }
        out += sm.getPolicy().getStateName(state);
    }
    return out + "]";
}

}  // namespace

// The clause itself. Both halves are asserted separately because they fail
// for different reasons: the first says the domain was too narrow, the second
// says the preemption that a document-root domain implies did not happen.
TEST(ParallelRegionRootExternalDomainAotTest, AnExternalRegionRootTransitionExitsEveryRegion) {
    Machine sm;
    sm.initialize();

    ASSERT_TRUE(holds(sm, Machine::State::Working)) << "precondition; active: " << describe(sm);
    ASSERT_TRUE(holds(sm, Machine::State::Alive)) << "precondition; active: " << describe(sm);

    sm.processEvent(Machine::Event::Restart);

    EXPECT_TRUE(holds(sm, Machine::State::Restarting))
        << "the transition's own target must be entered; active: " << describe(sm);

    // The domain is the document root, so `watch` exited with everything else
    // and came back at its default. Reading `rebuilding` here means the domain
    // was resolved to `run` (or to `drive`) and `watch` was left alone.
    EXPECT_TRUE(holds(sm, Machine::State::Alive))
        << "an external transition on a region root has the DOCUMENT ROOT as its domain "
           "(Appendix D findLCCA filters <parallel> out of the candidates), so every region "
           "exits and re-enters and `watch` is back at its default; active: "
        << describe(sm);
    EXPECT_FALSE(holds(sm, Machine::State::Rebuilding))
        << "`watch` answers `restart` too, but its transition conflicts with this one and its "
           "source is not a descendant, so document order preempts it; active: "
        << describe(sm);
}

// The contrast, and the reason the ai_loop document is spelled the way it is.
// A test that only pinned the external case would pass just as well on an
// engine that sent EVERY region-root transition to the document root.
TEST(ParallelRegionRootExternalDomainAotTest, AnInternalRegionRootTransitionLeavesTheOtherRegion) {
    Machine sm;
    sm.initialize();

    sm.processEvent(Machine::Event::Hold);

    EXPECT_TRUE(holds(sm, Machine::State::Paused))
        << "the transition's own target must be entered; active: " << describe(sm);

    // Domain is `drive`: the source is compound and the target is its
    // descendant, so `watch` never exits and answers the event itself.
    EXPECT_TRUE(holds(sm, Machine::State::Rebuilding))
        << "an internal region-root transition has the region as its domain, so the sibling "
           "region keeps its own answer to the same event; active: "
        << describe(sm);
    EXPECT_FALSE(holds(sm, Machine::State::Alive))
        << "`watch` took its own transition, so it is no longer at its default; active: " << describe(sm);
}

}  // namespace SCE::Tests
