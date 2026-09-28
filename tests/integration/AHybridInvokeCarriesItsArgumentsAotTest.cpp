// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4.1 + 6.4.3: an <invoke> whose child is named by an expression
// carries its arguments as one whose child is fixed does — C++ AOT.
//
// The value always names `keeper`, and `bare` declares the one name `keeper`
// does not, so a pair seeded by the wrong candidate's declarations is a leak
// `keeper` reports rather than an absence the test has to infer.
//
// Fixture: integration_resources/a_hybrid_invoke_carries_its_arguments/a_hybrid_invoke_carries_its_arguments.scxml
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(a_hybrid_invoke_carries_its_arguments ...)`.

#include "a_hybrid_invoke_carries_its_arguments_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <cstdint>
#include <gtest/gtest.h>
#include <memory>
#include <optional>
#include <string>

namespace SCE::Tests {

namespace {

using SM = SCE::Generated::a_hybrid_invoke_carries_its_arguments::a_hybrid_invoke_carries_its_arguments;

std::string show(const std::optional<int64_t> &value) {
    return value ? std::to_string(*value) : std::string("<unreadable>");
}

}  // namespace

TEST(AHybridInvokeCarriesItsArgumentsAotTest, EachArgumentReachesOnlyTheChildThatDeclaresIt) {
    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }
    sm.initialize();
    const bool completed = sm.runUntilCompletion(std::chrono::seconds(3));

    EXPECT_TRUE(completed) << "the machine never completed; `refusedPhase` parks when its invoke raised nothing";
    EXPECT_EQ(sm.terminalState(), SM::State::Done)
        << "`FailWrongChild` means `bare` ran; `FailRefusedChildStarted` means an unreadable namelist "
           "still started a child";
    EXPECT_EQ(sm.errors(), std::optional<int64_t>(2)) << "errors = " << show(sm.errors());
    EXPECT_EQ(sm.started(), std::optional<int64_t>(2)) << "started = " << show(sm.started());
    EXPECT_EQ(sm.paramsOk(), std::optional<int64_t>(1)) << "paramsOk = " << show(sm.paramsOk());
    EXPECT_EQ(sm.namelistOk(), std::optional<int64_t>(1)) << "namelistOk = " << show(sm.namelistOk());
}

}  // namespace SCE::Tests
