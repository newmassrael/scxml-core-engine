// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A statechart declared as a transition function answers as its machine does
// (docs/SCE_TRANSITION_FUNCTION.md) — C++ AOT channel, the nested lifecycle.
//
// Sibling of `NestedLifecycleTest.cpp` (Interpreter channel). The document
// declares `sce:transition-function="emit"` and reaches every rule the function
// is computed by: transitions held by an ancestor, a compound state entered by
// its `initial` attribute, by an `<initial>` element and by its first child, an
// internal and an external transition, a prefix and a wildcard descriptor, a
// targetless transition and a top-level final. This drives the machine to
// every state by the function's shortest path, sends every event there, and
// holds what the engine does to what the function says, pair by pair.
//
// Fixture:
// integration_resources/nested_lifecycle/nested_lifecycle.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(nested_lifecycle ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "integration/TransitionFunctionEquivalence.h"
#include "nested_lifecycle_sm.h"
#include "nested_lifecycle_transition_function.h"

#include <gtest/gtest.h>
#include <memory>

namespace SCE::Tests {

TEST(NestedLifecycleAotTest, TheMachineAndItsTransitionFunctionAgreeOnEveryPair) {
    namespace tf = ::SCE::Generated::nested_lifecycle::transition_function;
    using SM = ::SCE::Generated::nested_lifecycle::nested_lifecycle;

    std::unique_ptr<SM> machine;
    const std::size_t pairs = expectEveryPairAgrees(SCE_TRANSITION_FUNCTION_VIEW(tf), aotProbe(machine));
    EXPECT_EQ(pairs, tf::ALL_STATES.size() * tf::ALL_EVENTS.size()) << "the drive did not check every pair";
}

}  // namespace SCE::Tests
