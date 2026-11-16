// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-6.4 (docs/adr/0098): a value an invoke hands its child is the
// initial value of the child's top-level <data> of that name, declared in
// place of its own — C++ AOT.
//
// The parent's generated invoke evaluates every <param> and namelist location
// and hands the child every value (`invoke_seed_child`); the child's generated
// start decides what it takes as it declares its top-level <data>
// (`initializeDataModel`): a <data> whose id was handed is declared with that
// value and its own expr is never evaluated, a state's <data> keeps its own,
// and a name no top-level <data> declares is dropped. The parent keeps what
// each child reported in its own datamodel and judges it, landing a failure in
// a <final> named for the sentence that broke; the test reads the verdict and
// the kept values.
//
// Sibling of `AChildTakesAHandedValueInPlaceOfItsDataTest.cpp` (Interpreter).
//
// Fixture:
// integration_resources/a_child_takes_a_handed_value_in_place_of_its_data/a_child_takes_a_handed_value_in_place_of_its_data.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(a_child_takes_a_handed_value_in_place_of_its_data ...)`.

#include "a_child_takes_a_handed_value_in_place_of_its_data_sm.h"
#include "integration/AHandedValueRefusingEngine.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <gtest/gtest.h>
#include <memory>
#include <optional>
#include <string>
#include <utility>

namespace SCE::Tests {

namespace {

using SM = SCE::Generated::a_child_takes_a_handed_value_in_place_of_its_data::
    a_child_takes_a_handed_value_in_place_of_its_data;

/// A value the parent kept, read in its session, for the failure message.
std::string read(SM &sm, const char *expression) {
    auto result = ::SCE::ScriptEngineProvider::getScriptEngine()
                      .evaluateExpression(sm.getPolicy().sessionIdText(), expression)
                      .get();
    return result.isSuccess() ? result.getValueAsString() : std::string("<unreadable>");
}

/// Whether an expression over what the parent kept holds, compared in the
/// datamodel so a number is not compared as text.
bool holds(SM &sm, const char *expression) {
    return read(sm, expression) == "true";
}

std::string describe(SM &sm) {
    std::string text;
    for (const char *name : {"raising_seen", "effectful_seen", "effects_seen", "refusable_seen", "kept_seen",
                             "stranger_seen", "payload_seen", "script_seen", "entry_seen", "errors_seen", "listed_seen",
                             "stray_seen", "record_seen", "list_errors_seen"}) {
        text += std::string(name) + "=" + read(sm, name) + " ";
    }
    return text;
}

/// Runs the fixture over `engine` until the parent reaches its final state.
void runOver(SM &sm, std::shared_ptr<::SCE::IScriptEngine> engine) {
    sm.setScriptEngine(std::move(engine));
    ASSERT_EQ(sm.initialize(), std::nullopt);
    const bool reachedFinal = sm.runUntilCompletion(std::chrono::seconds(3));
    ASSERT_TRUE(reachedFinal) << "the parent did not reach a final state: a child never reported, or the child "
                                 "of `firstMacrostep` read its own value of `handed` in its first macrostep and "
                                 "stands in `missed`. "
                              << describe(sm);
}

}  // namespace

TEST(AChildTakesAHandedValueInPlaceOfItsDataAotTest, AHandedValueIsDeclaredInPlaceOfTheDatasOwn) {
    SM sm;
    // Aliasing constructor + no-op deleter: the engine is the
    // ScriptEngineProvider singleton's, and the shared_ptr a non-owning view.
    runOver(sm, std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                      [](::SCE::IScriptEngine *) {}));
    if (::testing::Test::HasFatalFailure()) {
        return;
    }
    EXPECT_NE(sm.terminalState(), std::optional<SM::State>(SM::State::FailOwnValueRan))
        << "the own expr of a <data> whose value was handed has an effect it must not have: it was "
           "evaluated, and §scxml-6.4 says any value specified in the <data> element is ignored. "
        << describe(sm);
    EXPECT_NE(sm.terminalState(), std::optional<SM::State>(SM::State::FailStateDataTookTheValue))
        << "a state's <data> took a handed value: only the top-level declarations take one. " << describe(sm);
    EXPECT_NE(sm.terminalState(), std::optional<SM::State>(SM::State::FailUnmatchedNameAdded))
        << "a handed name that matches no top-level <data> became a variable of the child. " << describe(sm);
    EXPECT_NE(sm.terminalState(), std::optional<SM::State>(SM::State::FailChildSharedTheParentsObject))
        << "the child was handed the parent's object, not a copy of it. " << describe(sm);
    EXPECT_NE(sm.terminalState(), std::optional<SM::State>(SM::State::FailScriptReadItsOwnValue))
        << "the top-level <script> did not read the handed values: they were declared after it ran. " << describe(sm);
    EXPECT_NE(sm.terminalState(), std::optional<SM::State>(SM::State::FailEntryReadItsOwnValue))
        << "the initial <onentry> did not read the handed values. " << describe(sm);
    EXPECT_NE(sm.terminalState(), std::optional<SM::State>(SM::State::FailChildRaised))
        << "the child saw error.execution: the own expr of a <data> whose value was handed was evaluated. "
        << describe(sm);
    EXPECT_EQ(sm.terminalState(), std::optional<SM::State>(SM::State::Pass)) << describe(sm);
}

// §scxml-5.3: a handed value the child's session cannot hold is the error of
// that <data>'s initialization, as a failing expr would be: error.execution,
// and the variable left unbound. The parent judges every other value before
// the child's error count, so `failChildRaised` says they all took their
// places.
TEST(AChildTakesAHandedValueInPlaceOfItsDataAotTest, AHandedValueTheSessionCannotHoldFailsItsData) {
    auto refusing = std::make_shared<AHandedValueRefusingEngine>("refusable");
    SM sm;
    runOver(sm, refusing);
    if (::testing::Test::HasFatalFailure()) {
        return;
    }
    EXPECT_EQ(refusing->refusals(), 1u) << "the child declares the handed value once, and nothing else. "
                                        << describe(sm);
    EXPECT_EQ(sm.terminalState(), std::optional<SM::State>(SM::State::FailChildRaised))
        << "the child that could not hold `refusable` raised error.execution with every other value in its "
           "place. "
        << describe(sm);
    EXPECT_TRUE(holds(sm, "refusable_seen === 'unbound'"))
        << "the <data> whose handed value could not be held took a value: it must be left as one whose "
           "initialization failed. "
        << describe(sm);
    EXPECT_TRUE(holds(sm, "errors_seen === 1"))
        << "the failed declaration of a handed value raised no error.execution in the child, or another "
           "<data> raised too. "
        << describe(sm);
}

}  // namespace SCE::Tests
