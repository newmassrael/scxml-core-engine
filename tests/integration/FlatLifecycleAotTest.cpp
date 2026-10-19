// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A statechart declared as a transition function answers as its machine does
// (docs/SCE_TRANSITION_FUNCTION.md) — C++ AOT channel, the flat lifecycle.
//
// Sibling of `FlatLifecycleTest.cpp` (Interpreter channel). The document
// declares `sce:transition-function="emit"`, so its machine is generated with a
// pure function of (state, event) beside it, in the namespace
// `SCE::Generated::flat_lifecycle::transition_function`. The function was
// computed at build time by the selection rule the engine follows at run time;
// this drives the machine to every state by the function's shortest path,
// sends every event there, and holds what the engine does to what the function
// says, pair by pair.
//
// Fixture:
// integration_resources/flat_lifecycle/flat_lifecycle.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(flat_lifecycle ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "flat_lifecycle_sm.h"
#include "flat_lifecycle_transition_function.h"
#include "integration/TransitionFunctionEquivalence.h"

#include <gtest/gtest.h>
#include <memory>

namespace SCE::Tests {

TEST(FlatLifecycleAotTest, TheMachineAndItsTransitionFunctionAgreeOnEveryPair) {
    namespace tf = ::SCE::Generated::flat_lifecycle::transition_function;
    using SM = ::SCE::Generated::flat_lifecycle::flat_lifecycle;

    std::unique_ptr<SM> machine;
    const std::size_t pairs = expectEveryPairAgrees(SCE_TRANSITION_FUNCTION_VIEW(tf), aotProbe(machine));
    EXPECT_EQ(pairs, tf::ALL_STATES.size() * tf::ALL_EVENTS.size()) << "the drive did not check every pair";
}

}  // namespace SCE::Tests
