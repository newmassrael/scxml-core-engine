// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.12.1: `wild`, `wild.` and `wild.*` are one descriptor — Interpreter path.
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
// Sibling of `EventDescriptorSpellingsAgreeAotTest.cpp` (C++ AOT channel).
//
// Fixture: integration_resources/event_descriptor_spellings_agree/event_descriptor_spellings_agree.scxml

#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <fstream>
#include <gtest/gtest.h>
#include <sstream>
#include <thread>

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

class EventDescriptorSpellingsAgreeTest : public ::testing::Test {
protected:
    void SetUp() override {
        engine_ = &ScriptEngineProvider::getScriptEngine();
        engine_->reset();
    }

    void TearDown() override {
        if (engine_) {
            engine_->shutdown();
        }
    }

    IScriptEngine *engine_;
};

TEST_F(EventDescriptorSpellingsAgreeTest, EverySpellingOfOneDescriptorCatchesTheSameEvents) {
    const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                "/integration_resources/event_descriptor_spellings_agree/"
                                "event_descriptor_spellings_agree.scxml";
    std::ifstream in(fixture);
    ASSERT_TRUE(in.is_open()) << "canonical fixture not readable: " << fixture;
    std::ostringstream buffer;
    buffer << in.rdbuf();

    auto sm = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());
    ASSERT_TRUE(sm->loadSCXMLFromString(buffer.str()));
    ASSERT_TRUE(sm->start());

    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(3);
    while (std::chrono::steady_clock::now() < deadline && sm->isRunning()) {
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }

    // Each failure final names the case, so this one assertion says which
    // spelling disagreed with the clause:
    //   failSuffixed   `wild.*` did not catch bare `wild`
    //   failDotted     `dot.` did not catch bare `dot`
    //   failBounded    `wild.*` caught `wilder`, across a token boundary
    //   failUniversal  a bare `.*` did not catch `any.token.sequence`
    EXPECT_EQ(sm->terminalState().value_or(""), "pass")
        << "the machine rested in a failure final: `wild.*` must catch bare `wild` and `dot.` must catch "
           "bare `dot` (the clause calls the spellings functionally equivalent), a bare `.*` must catch "
           "every event, and `wild.*` must NOT catch `wilder` because the prefix is a whole token";
}

}  // namespace Tests
}  // namespace SCE
