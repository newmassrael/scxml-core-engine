// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.12.1: `wild`, `wild.` and `wild.*` are one descriptor — C++ AOT path.
//
// The clause calls the three spellings "functionally equivalent since they are
// token prefixes of exactly the same set of event names", and a descriptor
// ending in `.*` matches "zero or more tokens", so a bare `.*` is an empty
// token prefix and matches every event.
//
// No W3C fixture delivers a bare `foo` to a `foo.*` handler, and the four that
// write a bare `.*` write it as a catch-all to `fail` — which an engine that
// matches nothing on `.*` passes by never taking the transition it must not
// take. This fixture puts every case in positive polarity instead.
//
// Sibling of `EventDescriptorSpellingsAgreeTest.cpp` (Interpreter channel).
// Both engines ship in production, so each is held to the clause
// independently against one canonical fixture.
//
// Fixture: integration_resources/event_descriptor_spellings_agree/event_descriptor_spellings_agree.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(event_descriptor_spellings_agree ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "event_descriptor_spellings_agree_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <gtest/gtest.h>
#include <memory>

namespace SCE::Tests {

TEST(EventDescriptorSpellingsAgreeAotTest, EverySpellingOfOneDescriptorCatchesTheSameEvents) {
    using SM = SCE::Generated::event_descriptor_spellings_agree::event_descriptor_spellings_agree;

    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }

    sm.initialize();
    const bool completed = sm.runUntilCompletion(std::chrono::seconds(3));

    EXPECT_TRUE(completed) << "the machine never reached a final state; every case in this fixture has a "
                              "literal fallback, so this means no transition matched an event that two of "
                              "them describe";

    // Each failure final names the case, so this one assertion says which
    // spelling disagreed with the clause:
    //   FailSuffixed   `wild.*` did not catch bare `wild`
    //   FailDotted     `dot.` did not catch bare `dot`
    //   FailBounded    `wild.*` caught `wilder`, across a token boundary
    //   FailUniversal  a bare `.*` did not catch `any.token.sequence`
    EXPECT_EQ(sm.terminalState(), SM::State::Pass)
        << "the machine rested in a failure final: `wild.*` must catch bare `wild` and `dot.` must catch "
           "bare `dot` (the clause calls the spellings functionally equivalent), a bare `.*` must catch "
           "every event, and `wild.*` must NOT catch `wilder` because the prefix is a whole token";
}

}  // namespace SCE::Tests
