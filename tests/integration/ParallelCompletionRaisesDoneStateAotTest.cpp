// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.4 + 3.7: a `<parallel>` completing raises `done.state.<id>` —
// C++ AOT path.
//
// The regression is in which events the parser registers. `add_done_state_events`
// walked from each `<final>` to its direct parent, and a `<parallel>` is never
// that parent: its finals sit inside the regions, one level further down. The
// emitter meanwhile raises the parallel's event from the same site it raises
// the region's, guarding on the grandparent, so the generated code named an
// enumerator the model never declared.
//
// That defect is a *compile* failure, which is why registering the fixture is
// itself most of the gate — `check` returns `cpp=ok` for this document either
// way, because acceptance is decided before anything is compiled. This file
// adds the behavioural half: that the event is not merely declared but
// actually carries the machine out of the parallel.
//
// Sibling of `ParallelCompletionRaisesDoneStateTest.cpp` (Interpreter channel).
//
// Fixture:
// integration_resources/parallel_completion_raises_done_state/parallel_completion_raises_done_state.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(parallel_completion_raises_done_state ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "parallel_completion_raises_done_state_sm.h"

#include <algorithm>
#include <gtest/gtest.h>

namespace SCE::Tests {

TEST(ParallelCompletionRaisesDoneStateAotTest, EveryRegionFinalCompletesTheParallel) {
    using SM = SCE::Generated::parallel_completion_raises_done_state::parallel_completion_raises_done_state;

    // Naming the enumerator is the compile-time half of this gate: the model
    // must declare the parallel's own completion event, not only its regions'.
    static_assert(SM::Event::Done_state_run != SM::Event::NONE,
                  "the parallel's done.state event must be declared, not only raised");

    SM sm;
    sm.initialize();

    const auto active = [&sm](SM::State state) {
        const auto states = sm.getPolicy().getActiveStates();
        return std::find(states.begin(), states.end(), state) != states.end();
    };

    ASSERT_TRUE(active(SM::State::A1)) << "the fixture is supposed to start inside the parallel; it did not, "
                                          "so nothing below is testing what it claims";
    ASSERT_TRUE(active(SM::State::B1)) << "the fixture is supposed to start inside the parallel; it did not, "
                                          "so nothing below is testing what it claims";

    sm.processEvent(SM::Event::Go);

    EXPECT_TRUE(active(SM::State::A2)) << "a region did not reach its `<final>` on `go`";
    EXPECT_TRUE(active(SM::State::B2)) << "a region did not reach its `<final>` on `go`";
}

}  // namespace SCE::Tests
