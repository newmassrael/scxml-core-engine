// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.4: a region that took an external self-transition still holds an
// atomic state, so it answers the next event too — C++ AOT path.
//
// This is the channel the axis was found on. A `<parallel>` region that keeps
// its root and loses its leaf reads as present in every structural check while
// nothing inside it can fire again, and the moment it happens produces no
// error, no warning and no failing assertion. What finds it is a later event
// the region was supposed to answer.
//
// Measured 2026-08-14: with
// `sce-build/tests/mutations/parallel_microstep_owns_exit_and_entry.cases`
// restoring the defect, the sibling fixture's driver
// (`ParallelRegionsTakeOwnTransitionsAotTest`) stays entirely green — SURVIVED,
// 0/2 red. That fixture asks its whole question inside one microstep, and this
// one exists because the answer to that question can be right while the
// configuration it left behind is not.
//
// Sibling of `ParallelSelfTransitionKeepsItsLeafTest.cpp` (Interpreter
// channel). Both engines ship and both reach the clause through the same
// microstep contract, so each is held to it independently.
//
// Fixture:
// integration_resources/parallel_self_transition_keeps_its_leaf/parallel_self_transition_keeps_its_leaf.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(parallel_self_transition_keeps_its_leaf ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "parallel_self_transition_keeps_its_leaf_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <algorithm>
#include <gtest/gtest.h>
#include <memory>
#include <string>
#include <vector>

namespace SCE::Tests {

namespace {

using SM = SCE::Generated::parallel_self_transition_keeps_its_leaf::parallel_self_transition_keeps_its_leaf;

bool isActive(SM &sm, SM::State state) {
    const auto active = sm.getPolicy().getActiveStates();
    return std::find(active.begin(), active.end(), state) != active.end();
}

/// Rendered into a failure message; a bare enum tells the reader nothing, and
/// the symptom this test exists for is a configuration that is *almost* right.
std::string describe(SM &sm) {
    std::string out = "[";
    for (const auto state : sm.getPolicy().getActiveStates()) {
        if (out.size() > 1) {
            out += " | ";
        }
        out += sm.getPolicy().getStateName(state);
    }
    return out + "]";
}

}  // namespace

TEST(ParallelSelfTransitionKeepsItsLeafAotTest, TheSelfTransitionedRegionAnswersTheNextEvent) {
    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }

    sm.initialize();

    ASSERT_TRUE(isActive(sm, SM::State::Within))
        << "the fixture is supposed to start with the self-transitioning region in `within`; it "
           "did not, so nothing below is testing what it claims. active: "
        << describe(sm);
    ASSERT_TRUE(isActive(sm, SM::State::Working))
        << "the fixture is supposed to start with the deeper region in `working`; it did not, so "
           "nothing below is testing what it claims. active: "
        << describe(sm);

    sm.processEvent(SM::Event::E);

    // The symptom, named where it happens. A region holding no atomic state is
    // still "in" the parallel by every ancestor test, so this assertion is the
    // only place the loss is visible before it becomes a missing answer.
    EXPECT_TRUE(isActive(sm, SM::State::Within))
        << "the self-transitioning region lost its leaf on the first event. `within` is both the "
           "source and the target of its own external self-transition, so the microstep exits and "
           "re-enters it — anything that exits it a second time takes it back out and does not put "
           "it back, because the engine's current-state pointer has moved to the other region. "
           "active: "
        << describe(sm);
    EXPECT_TRUE(isActive(sm, SM::State::Judging))
        << "the deeper region did not take its own transition on `e`. active: " << describe(sm);

    // The second event is the one this fixture exists for: `judging` has no `e`
    // transition, so nothing but `budget` can answer, and it can only answer
    // from a leaf.
    sm.processEvent(SM::Event::E);

    EXPECT_TRUE(isActive(sm, SM::State::Within))
        << "the self-transitioning region is not in `within` after the second `e`. active: " << describe(sm);

    sm.processEvent(SM::Event::Check);

    EXPECT_EQ(sm.getCurrentState(), SM::State::Settled)
        << "`check` did not carry the machine to the top-level `settled`, which the document "
           "guards on `n == 1 && m == 2`. `m` reaches 2 only if the self-transitioning region "
           "still had a leaf to transition from when the second `e` arrived — a region that kept "
           "its root and lost its leaf answers no event and raises nothing while it fails to. "
           "active: "
        << describe(sm);
}

}  // namespace SCE::Tests
