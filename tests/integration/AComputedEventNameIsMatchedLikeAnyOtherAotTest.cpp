// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A computed event name is matched like any other (§scxml-3.12.1, §scxml-5.10) —
// C++ AOT channel.
//
// Sibling of `AComputedEventNameIsMatchedLikeAnyOtherTest.cpp` (Interpreter
// channel). An AOT machine carries an event as the enum member of a name its
// document writes, so a `<send eventexpr>` whose name it does not write has to be
// resolved to the member of the longest prefix it does (or its wildcard), and has
// to carry the whole name beside that member for `_event.name` to read it, over
// the external and the internal queue, now and after a delay.
//
// Fixture:
// integration_resources/a_computed_event_name_is_matched_like_any_other/a_computed_event_name_is_matched_like_any_other.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(a_computed_event_name_is_matched_like_any_other ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "a_computed_event_name_is_matched_like_any_other_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <algorithm>
#include <chrono>
#include <gtest/gtest.h>
#include <memory>
#include <vector>

namespace SCE::Tests {

TEST(AComputedEventNameIsMatchedLikeAnyOtherAotTest, AComputedNameIsMatchedByTokenAndToldWhole) {
    using SM = SCE::Generated::a_computed_event_name_is_matched_like_any_other::
        a_computed_event_name_is_matched_like_any_other;

    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        // Aliasing constructor + no-op deleter — engine lifetime is owned by the
        // ScriptEngineProvider singleton; the shared_ptr is a non-owning view.
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }

    EXPECT_EQ(sm.initialize(), std::nullopt);
    const bool reachedFinal = sm.runUntilCompletion(std::chrono::seconds(3));

    EXPECT_TRUE(reachedFinal) << "the machine did not reach a final state within timeout — a computed name was "
                                 "dropped (`request.new`, `other.thing`, `request.again`, `last.one`), or matched "
                                 "but told shorter than the name it was sent under";
    EXPECT_EQ(sm.terminalState(), SM::State::Pass);
}

namespace {

using ComputedNameSM =
    SCE::Generated::a_computed_event_name_is_matched_like_any_other::a_computed_event_name_is_matched_like_any_other;

// Runs the machine to its end with a transition observer (docs/adr/0089) that
// keeps each row it is told, cleared again before the run when `clear`.
std::vector<ComputedNameSM::TransitionTaken> runObserved(bool clear, bool &reachedPass) {
    std::vector<ComputedNameSM::TransitionTaken> seen;
    ComputedNameSM sm;
    if constexpr (ComputedNameSM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }
    sm.setTransitionObserver([&seen](const ComputedNameSM::TransitionTaken &taken) { seen.push_back(taken); });
    if (clear) {
        sm.setTransitionObserver({});
    }
    EXPECT_EQ(sm.initialize(), std::nullopt);
    reachedPass = sm.runUntilCompletion(std::chrono::seconds(3)) && sm.terminalState() == ComputedNameSM::State::Pass;
    return seen;
}

}  // namespace

// The document declares its transition table, and a run that reaches `pass`
// takes each of its six transitions once, in document order: the observer is
// told exactly those rows, each by the `(from, index)` its row of the table
// carries.
TEST(AComputedEventNameIsMatchedLikeAnyOtherAotTest, TheObserverIsToldEachRowOfTheTableTheRunTakes) {
    using State = ComputedNameSM::State;
    using Taken = ComputedNameSM::TransitionTaken;

    bool reachedPass = false;
    const std::vector<Taken> seen = runObserved(false, reachedPass);
    ASSERT_TRUE(reachedPass) << "the observed run did not end in `pass`";

    const std::vector<Taken> expected{Taken{State::S1, 0u}, Taken{State::S2, 0u}, Taken{State::S3, 0u},
                                      Taken{State::S4, 0u}, Taken{State::S5, 0u}, Taken{State::S6, 0u}};
    EXPECT_TRUE(seen == expected) << "the observer was told " << seen.size() << " rows, not the six taken in order";

    const auto &rows = ComputedNameSM::PolicyType::transitions;
    for (const Taken &taken : seen) {
        const bool isRow = std::any_of(rows.begin(), rows.end(), [&taken](const auto &row) {
            return row.from == taken.source && row.index == taken.index;
        });
        EXPECT_TRUE(isRow) << "the observer was told a pair that is no row of the table";
    }
    for (const auto &row : rows) {
        const bool wasTaken = std::any_of(seen.begin(), seen.end(), [&row](const Taken &taken) {
            return row.from == taken.source && row.index == taken.index;
        });
        EXPECT_TRUE(wasTaken) << "the run never took row " << row.index << " of a state";
    }
}

// An observer set and then cleared is told nothing, and the run still reaches
// `pass`, as one that never had an observer does.
TEST(AComputedEventNameIsMatchedLikeAnyOtherAotTest, AClearedObserverIsToldNothingAndTheRunIsTheSame) {
    bool reachedPass = false;
    const std::vector<ComputedNameSM::TransitionTaken> seen = runObserved(true, reachedPass);
    EXPECT_TRUE(reachedPass) << "the run did not end in `pass`";
    EXPECT_TRUE(seen.empty()) << "a cleared observer was told " << seen.size() << " rows";
}

}  // namespace SCE::Tests
