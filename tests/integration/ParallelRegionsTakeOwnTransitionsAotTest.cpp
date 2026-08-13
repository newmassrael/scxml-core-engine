// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.4: every region of a `<parallel>` takes its own enabled
// transition in the same microstep — C++ AOT path.
//
// The regression this pins is in the shared hierarchy helper, not in either
// engine's own code: `findLCA` answered a self-transition's domain with the
// state itself, where Appendix D `findLCCA` searches `getProperAncestors` and
// so can only ever answer with an ancestor. `computeExitSet` climbs from the
// source until it reaches the state just below the domain, and with the source
// named as its own domain that walk had no stopping point and ran to the
// document root. The exit set then included the enclosing `<parallel>`, and
// `removeConflictingTransitions` preempted the sibling region's transition on
// that same event: the region was left with no active leaf and its `<assign>`
// never ran.
//
// The observable is `settled`, which the document reaches only when both
// regions' assignments have run — a configuration check alone would still pass
// for a region that moved without executing its transition content.
//
// Sibling of `ParallelRegionsTakeOwnTransitionsTest.cpp` (Interpreter channel).
// Both engines ship in production and both reach the clause through the same
// helper, so each is held to it independently against one canonical fixture.
//
// Fixture:
// integration_resources/parallel_regions_take_own_transitions/parallel_regions_take_own_transitions.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(parallel_regions_take_own_transitions ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "parallel_regions_take_own_transitions_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <algorithm>
#include <gtest/gtest.h>
#include <memory>
#include <vector>

namespace SCE::Tests {

namespace {

using SM = SCE::Generated::parallel_regions_take_own_transitions::parallel_regions_take_own_transitions;

bool isActive(SM &sm, SM::State state) {
    const auto active = sm.getPolicy().getActiveStates();
    return std::find(active.begin(), active.end(), state) != active.end();
}

}  // namespace

TEST(ParallelRegionsTakeOwnTransitionsAotTest, EveryRegionTakesItsOwnTransition) {
    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }

    sm.initialize();

    ASSERT_TRUE(isActive(sm, SM::State::Working))
        << "the fixture is supposed to start with the deeper region in `working`; it did not, "
           "so nothing below is testing what it claims";
    ASSERT_TRUE(isActive(sm, SM::State::Within))
        << "the fixture is supposed to start with the shallower region in `within`; it did not, "
           "so nothing below is testing what it claims";

    sm.processEvent(SM::Event::E);

    EXPECT_TRUE(isActive(sm, SM::State::Judging))
        << "the deeper region lost its leaf. W3C SCXML 3.4 has every region take its own enabled "
           "transition on `e`; the sibling region's external self-transition must not preempt this "
           "one. Appendix D reaches a self-transition's domain through `findLCCA`, whose candidates "
           "come from `getProperAncestors` and therefore never include the state itself — an exit "
           "set that names the enclosing `<parallel>` is the symptom of answering otherwise.";
    EXPECT_TRUE(isActive(sm, SM::State::Within))
        << "the shallower region left `within`, which is both the source and the target of its own "
           "external self-transition";

    sm.processEvent(SM::Event::Check);

    EXPECT_EQ(sm.getCurrentState(), SM::State::Settled)
        << "`check` did not carry the machine to the top-level `settled`, which the document "
           "guards on both regions' assignments having run. Reaching `judging` without "
           "`n == 1 && m == 1` means a region changed state while its transition content was "
           "skipped.";
}

}  // namespace SCE::Tests
