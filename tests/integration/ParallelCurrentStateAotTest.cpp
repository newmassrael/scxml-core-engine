// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.4: in a parallel machine `getCurrentState()` names an atomic
// state, not whichever state a transition happened to be written against.
//
// The configuration is the truth for a `<parallel>`, and every assertion in
// this repository's parallel suites reads it through `getActiveStates()`. But
// `getCurrentState()` is a single state and 105 files read it, so what it
// answers is a contract too — and it was the one thing `executeMicrostep`
// left unsettled: the microstep sets it to the last transition target it
// processed, and a target may be compound.
//
// Measured 2026-08-13 against the fixture beside this file: the active set was
// `[run | counter | drive | within | outer | a]` — correct — while
// `getCurrentState()` answered `outer`, a state the machine is *within*
// rather than the atomic state it is *in*. `sce_rust_runtime`'s
// `resolve_current_state_to_leaf` and the Go engine's
// `resolveCurrentStateToLeaf` both answer `a`, so C++ was the one backend of
// three disagreeing on a public accessor.
//
// Why this cannot live in the ai_loop suite next door, which is the other
// parallel machine asked per-clause questions here: no transition in
// `examples/ai_loop/ai_loop.scxml` targets a compound state, so that document
// cannot reach the condition. Measured, not assumed — the four compound
// states there are `budget`, `drive`, `running` and `watch`, and none is a
// transition target.
//
// Fixture: tests/integration/parallel_current_state.scxml

#include "parallel_current_state_sm.h"

#include <algorithm>
#include <gtest/gtest.h>
#include <string>
#include <vector>

namespace SCE::Tests {

namespace {

using Machine = SCE::Generated::parallel_current_state::parallel_current_state;

std::vector<Machine::State> active(Machine &sm) {
    return sm.getPolicy().getActiveStates();
}

bool holds(Machine &sm, Machine::State state) {
    const auto states = active(sm);
    return std::find(states.begin(), states.end(), state) != states.end();
}

/// Rendered into a failure message; a bare enum tells the reader nothing.
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

TEST(ParallelCurrentStateAotTest, CurrentStateIsAtomicAfterACompoundTarget) {
    Machine sm;
    sm.initialize();

    sm.processEvent(Machine::Event::Go);

    // The configuration half first: if this is wrong the accessor question is
    // not yet meaningful, and the failure should say which of the two broke.
    ASSERT_TRUE(holds(sm, Machine::State::A))
        << "the compound target's initial child must be entered; active: " << describe(sm);

    const auto current = sm.getCurrentState();
    EXPECT_FALSE(sm.getPolicy().isCompoundState(current))
        << "`getCurrentState()` answered `" << sm.getPolicy().getStateName(current)
        << "`, which is compound — the machine is within it, not in it; active: " << describe(sm);
    EXPECT_EQ(current, Machine::State::A) << "the atomic state the machine is in is `a`; `getCurrentState()` answered `"
                                          << sm.getPolicy().getStateName(current) << "`; active: " << describe(sm);
}

TEST(ParallelCurrentStateAotTest, SettlingCurrentStateDoesNotDisturbTheOtherRegion) {
    Machine sm;
    sm.initialize();

    sm.processEvent(Machine::Event::Go);

    // The descent walks the configuration, so a bug in it could plausibly
    // wander out of `drive` and into the sibling region. `counter` self-
    // transitions on the same event, which is the arrangement that cost a
    // region its leaf once already.
    EXPECT_TRUE(holds(sm, Machine::State::Within))
        << "settling `currentState_` must not touch the sibling region; active: " << describe(sm);
}

TEST(ParallelCurrentStateAotTest, ALeafTargetIsLeftAlone) {
    Machine sm;
    sm.initialize();

    sm.processEvent(Machine::Event::Go);
    sm.processEvent(Machine::Event::Step);

    // `b` is atomic, so the descent has nothing to do. Asserted because a
    // resolver that always descends — or that descends from the wrong state —
    // passes the compound case above and breaks this one.
    EXPECT_EQ(sm.getCurrentState(), Machine::State::B)
        << "an atomic target is already settled; `getCurrentState()` answered `"
        << sm.getPolicy().getStateName(sm.getCurrentState()) << "`; active: " << describe(sm);
}

}  // namespace SCE::Tests
