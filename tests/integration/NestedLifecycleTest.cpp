// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A statechart declared as a transition function answers as its machine does
// (docs/SCE_TRANSITION_FUNCTION.md) — Interpreter channel, the nested lifecycle.
//
// The Interpreter reads the document at run time and generates nothing, so it
// is the one engine whose selection owes nothing to the build. It is held here
// to the function the build computed for the same document: driven to every
// state by the function's shortest path and sent every event there, it must
// reach what the function says and take or discard the event as the function
// says — through transitions held by an ancestor, each way a compound state
// names its default child, an internal and an external transition, a prefix
// and a wildcard descriptor, and a top-level final.
//
// Fixture:
// integration_resources/nested_lifecycle/nested_lifecycle.scxml
// — the canonical source all seven channels compile, so the Interpreter and AOT
// channels are held to the same machine as the Rust / Go / Kotlin / Python / C11
// channels rather than to hand-kept copies.

#include "integration/TransitionFunctionEquivalence.h"
#include "nested_lifecycle_transition_function.h"

#include <fstream>
#include <gtest/gtest.h>
#include <memory>
#include <sstream>

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

class NestedLifecycleTest : public ::testing::Test {
protected:
    void SetUp() override {
        engine_ = &ScriptEngineProvider::getScriptEngine();
        engine_->reset();
    }

    void TearDown() override {
        machine_.reset();
        if (engine_) {
            engine_->shutdown();
        }
    }

    IScriptEngine *engine_ = nullptr;
    std::shared_ptr<StateMachine> machine_;
};

TEST_F(NestedLifecycleTest, TheInterpreterAndTheTransitionFunctionAgreeOnEveryPair) {
    namespace tf = ::SCE::Generated::nested_lifecycle::transition_function;

    const std::string fixture =
        std::string(SCE_PROJECT_ROOT) + "/integration_resources/nested_lifecycle/nested_lifecycle.scxml";
    std::ifstream in(fixture);
    ASSERT_TRUE(in.is_open()) << "canonical fixture not readable: " << fixture;
    std::ostringstream buffer;
    buffer << in.rdbuf();

    const std::size_t pairs =
        expectEveryPairAgrees(SCE_TRANSITION_FUNCTION_VIEW(tf), interpreterProbe(machine_, buffer.str()));
    EXPECT_EQ(pairs, tf::ALL_STATES.size() * tf::ALL_EVENTS.size()) << "the drive did not check every pair";
}

}  // namespace Tests
}  // namespace SCE
