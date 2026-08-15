// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.3 + Appendix D: a compound state entered only because the target
// lies inside it does not take its default initial child — C++ AOT path.
//
// This is the channel the axis was found on, through the worked example next
// to it. Answering a dialog in `examples/ai_loop/ai_loop.scxml` targets
// `judging`, whose ancestor `running` is not active at the time; the
// configuration came back holding `priming` as well, and `priming`'s
// `<onentry>` sends the opening prompt. The supervised session was
// re-introduced to itself every time a person answered a question. Nothing
// failed: the machine still converged, and every W3C fixture stayed green.
//
// Sibling of `AncestorEntryIsNotDefaultEntryTest.cpp` (Interpreter channel).
// Both engines ship and each maintains this configuration through its own
// code, so each is held to the clause independently.
//
// Fixture:
// integration_resources/ancestor_entry_is_not_default_entry/ancestor_entry_is_not_default_entry.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(ancestor_entry_is_not_default_entry ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "ancestor_entry_is_not_default_entry_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <algorithm>
#include <gtest/gtest.h>
#include <memory>
#include <string>
#include <vector>

namespace SCE::Tests {

namespace {

using SM = SCE::Generated::ancestor_entry_is_not_default_entry::ancestor_entry_is_not_default_entry;

bool isActive(SM &sm, SM::State state) {
    const auto active = sm.getPolicy().getActiveStates();
    return std::find(active.begin(), active.end(), state) != active.end();
}

/// Rendered into every failure message. The symptom here is a configuration
/// that is *almost* right — the target is in it — so naming only the state
/// that was asserted would hide what actually came back.
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

TEST(AncestorEntryIsNotDefaultEntryAotTest, AnAncestorEnteredOnTheWayToATargetTakesNoDefaultChild) {
    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }

    sm.initialize();

    ASSERT_TRUE(isActive(sm, SM::State::Away))
        << "the run has to start OUTSIDE the `<parallel>` for the first pass to be testing "
           "anything — a source already inside it leaves the ancestors active and the entry "
           "chain never reaches their defaults. active: "
        << describe(sm);

    // Pass one: the parallel is not active, so `run` is entered as a parallel
    // ancestor and `drive` and `outer` as compound ones.
    sm.processEvent(SM::Event::Cross);

    EXPECT_TRUE(isActive(sm, SM::State::Chosen))
        << "the transition named `chosen` and the machine did not enter it. active: " << describe(sm);
    EXPECT_FALSE(isActive(sm, SM::State::By_default))
        << "`outer` has two children active at once. `by_default` is what `initial` names, and "
           "nothing targeted it — it was entered because the engine gave `outer` its default "
           "child while entering `outer` merely as an ancestor of `chosen`. active: "
        << describe(sm);
    EXPECT_TRUE(isActive(sm, SM::State::Idle))
        << "the region no entering state is inside must still be entered with its default — "
           "Appendix D's one exception for a parallel ancestor. active: "
        << describe(sm);

    // Pass two: the parallel is already active now, so `run` and `drive` are
    // skipped and only `outer` is entered. That is a different branch of the
    // entry walk, and it is the one a running machine takes.
    sm.processEvent(SM::Event::Back);
    sm.processEvent(SM::Event::Again);

    EXPECT_FALSE(isActive(sm, SM::State::By_default))
        << "`outer` took its default child on the second pass, where the `<parallel>` was "
           "already active and only `outer` itself was entered — the shape the worked example "
           "hits every time a person answers a dialog. active: "
        << describe(sm);

    sm.processEvent(SM::Event::Check);

    EXPECT_EQ(sm.getCurrentState(), SM::State::Settled)
        << "`check` did not carry the machine to the top-level `settled`. The document checks "
           "its four clauses in document order and lands each in a `<final>` of its own, so the "
           "configuration below names which one broke: `failDefaulted` (a default nobody "
           "targeted), `failLobbied` (`drive`'s default taken while it was only an ancestor), "
           "`failIdled` (the untouched region did not get its default, or got it twice), "
           "`failTargeted` (a pass never reached the target). active: "
        << describe(sm);
}

}  // namespace SCE::Tests
