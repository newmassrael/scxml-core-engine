// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-5.7.1 under §scxml-6.4: a `<param>` of an `<invoke>` whose expression
// will not evaluate — C++ AOT path.
//
// Two clauses meet here and only one governs. §scxml-6.4.2 terminates the
// element when "the evaluation of its arguments produces an error"; §scxml-5.7.1
// says a failing `<param>` costs `error.execution` and "MUST ignore the name and
// value", then delegates only the SUCCESSFUL name and value to the context —
// "See 5.5 <donedata>, 6.2 <send> and 6.4 <invoke> for details."
//
// 5.7.1 governs. This template kept neither half in the document's reach: the
// failure arm wrote `SCE_LOG_ERROR` and nothing else, and a log is not a queue,
// so a document that miscomputed one `<param>` saw a child come up with a
// `<data>` nothing explained and had no event to act on.
//
// Sibling of `InvokeParamErrorStartsTheChildTest.cpp` (Interpreter).
//
// Fixture: integration_resources/invoke_param_error_starts_the_child/invoke_param_error_starts_the_child.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(invoke_param_error_starts_the_child ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "invoke_param_error_starts_the_child_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <gtest/gtest.h>
#include <memory>

namespace SCE::Tests {

TEST(InvokeParamErrorStartsTheChildAotTest, AnInvokeParamThatWillNotEvaluateCostsItsPairAndNothingElse) {
    using SM = SCE::Generated::invoke_param_error_starts_the_child::invoke_param_error_starts_the_child;

    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        // Aliasing constructor + no-op deleter — engine lifetime is owned by
        // the ScriptEngineProvider singleton; the shared_ptr is a non-owning
        // view. Mirrors SimpleAotTest's W3C-AOT pattern.
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }

    sm.initialize();
    // The fixture's own `timeout` is a 3s delayed `<send>`, so the budget has
    // to outlast it or a never-started child reads as a hang rather than as
    // `FailInvokeNotStarted`.
    const bool reachedFinal = sm.runUntilCompletion(std::chrono::seconds(10));

    EXPECT_TRUE(reachedFinal) << "parent did not reach a final state within timeout — neither the "
                                 "child's `childUp` nor the delayed `timeout` that judges a "
                                 "never-started child arrived";
    EXPECT_NE(sm.getCurrentState(), SM::State::FailNoParamError)
        << "`childUp` arrived with no `error.execution` before it: §scxml-5.7.1 puts that error on "
           "the internal queue while the `<invoke>` is being evaluated, so it is dequeued before "
           "the child's first word. A log line is not a queue.";
    EXPECT_NE(sm.getCurrentState(), SM::State::FailInvokeNotStarted)
        << "the child never started: this engine read §scxml-6.4.2's \"terminate the processing of "
           "the element\" over 5.7.1's per-item rule. One `<param>` that will not evaluate costs "
           "its own pair, not the session.";
    EXPECT_NE(sm.getCurrentState(), SM::State::FailGoodParamLost)
        << "the child's `kept` did not arrive as 'here': §scxml-6.4.3 seeds the child's matching "
           "`<data>` from the param's value, and one sibling that failed does not cost the others.";
    EXPECT_NE(sm.getCurrentState(), SM::State::FailBrokenParamSeeded)
        << "the child found the empty string under `broken`: 5.7.1 says ignore the name AND the "
           "value, so the child must find its own declaration untouched rather than a placeholder "
           "the author never wrote.";
    EXPECT_EQ(sm.getCurrentState(), SM::State::Pass);
}

}  // namespace SCE::Tests
