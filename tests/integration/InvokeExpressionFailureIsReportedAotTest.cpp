// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-6.4.3: an `<invoke>` naming its target through an expression
// evaluates that expression at invoke-fire time, and a failure to evaluate
// places `error.execution` on the internal event queue — C++ AOT path.
//
// The clause puts two obligations on the Processor and this fixture is about
// the second only. The evaluated string does not select the child here:
// codegen fixes the child at build time and writes an immediate-`<final>`
// stub for it (docs/SCE_ACCEPTED_SUBSET.md §2.13), so a fixture built on the
// value would pass whatever the expression said. One built on the FAILURE
// measures the obligation every channel shares — and it is the obligation one
// channel was not meeting when this landed.
//
// Sibling of `InvokeExpressionFailureIsReportedTest.cpp` (Interpreter
// channel), which reaches the same raise through a runtime document load.
//
// Fixture: integration_resources/invoke_expression_failure_is_reported/invoke_expression_failure_is_reported.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(invoke_expression_failure_is_reported ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "invoke_expression_failure_is_reported_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <gtest/gtest.h>
#include <memory>

namespace SCE::Tests {

TEST(InvokeExpressionFailureIsReportedAotTest, AnUnevaluatableInvokeExpressionRaisesErrorExecution) {
    using SM = SCE::Generated::invoke_expression_failure_is_reported::invoke_expression_failure_is_reported;

    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }

    sm.initialize();
    const bool completed = sm.runUntilCompletion(std::chrono::seconds(3));

    EXPECT_TRUE(completed) << "the machine never completed. §scxml-6.4.3 requires the expression to be "
                              "evaluated when the `<invoke>` fires; parking means neither the raise nor "
                              "the child arrived.";
    EXPECT_EQ(sm.terminalState(), SM::State::Pass)
        << "reaching `fail` means the child started on an expression that cannot be evaluated, "
           "so nothing evaluated it";
}

}  // namespace SCE::Tests
