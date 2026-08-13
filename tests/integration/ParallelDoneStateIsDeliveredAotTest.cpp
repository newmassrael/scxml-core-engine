// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.4 + 3.7: `done.state.<parallel>` is delivered, not merely
// declared — C++ AOT path.
//
// Its sibling `parallel_completion_raises_done_state` proves the enumeration
// declares the event, and proves it by compiling. It cannot prove delivery: it
// has no listener, deliberately, because a transition's `event` attribute is
// itself a registration site and a listener there would make that fixture
// unable to fail for the defect it exists to catch.
//
// So the two documents split one clause. This one listens, and the verdict is
// a top-level `<final>` that only the completion event can reach.
//
// Sibling of `ParallelDoneStateIsDeliveredTest.cpp` (Interpreter channel).
//
// Fixture:
// integration_resources/parallel_done_state_is_delivered/parallel_done_state_is_delivered.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(parallel_done_state_is_delivered ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "parallel_done_state_is_delivered_sm.h"

#include <algorithm>
#include <gtest/gtest.h>
#include <string>

namespace SCE::Tests {

TEST(ParallelDoneStateIsDeliveredAotTest, CompletionCarriesTheMachineToATopLevelFinal) {
    using SM = SCE::Generated::parallel_done_state_is_delivered::parallel_done_state_is_delivered;

    SM sm;
    sm.initialize();

    const auto active = [&sm](SM::State state) {
        const auto states = sm.getPolicy().getActiveStates();
        return std::find(states.begin(), states.end(), state) != states.end();
    };
    const auto describe = [](SM &machine) {
        std::string out = "[";
        for (const auto state : machine.getPolicy().getActiveStates()) {
            if (out.size() > 1) {
                out += " | ";
            }
            out += machine.getPolicy().getStateName(state);
        }
        return out + "]";
    };

    ASSERT_TRUE(active(SM::State::A1) && active(SM::State::B1))
        << "the fixture is supposed to start inside the parallel; it did not, "
           "so nothing below is testing what it claims";

    sm.processEvent(SM::Event::Go);

    // One assertion, and the configuration in its message, because the two
    // ways this can fail are not separately observable here: completion is
    // selected within the SAME macrostep as the regions' finals, so by the
    // time control returns the parallel has been exited and `a2`/`b2` are
    // gone. Measured — asserting them as a precondition failed against an
    // engine that had already done the right thing.
    //
    // The configuration tells the two apart instead. `[run | a | a1 | b | b1]`
    // means `go` moved nothing and the parallel never completed;
    // `[run | a | a2 | b | b2]` means it completed and the event went nowhere.
    EXPECT_TRUE(active(SM::State::Settled))
        << "every region reaching its `<final>` completes the parallel, so `done.state.run` "
           "had to be raised AND selected — `settled` is reachable by nothing else; active: "
        << describe(sm);
}

}  // namespace SCE::Tests
