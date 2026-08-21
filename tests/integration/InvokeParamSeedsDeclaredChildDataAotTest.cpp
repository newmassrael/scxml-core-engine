// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-6.4.3: an `<invoke>` `<param>` seeds a declared `<data>` of the
// invoked session with the INVOKING session's value — C++ AOT.
//
// The Interpreter sibling (`InvokeParamSeedsDeclaredChildDataTest.cpp`)
// already satisfied this clause: `InvokeExecutor` evaluates each param in
// the invoking session and passes the resulting value. The AOT template
// handed the child the author's expression TEXT and let the child's engine
// evaluate it, which is a different question with the same answer for as
// long as the expression is a literal — and the whole W3C IRP param
// surface is literals. This is the witness that separates the two.
//
// Each phase reaches its own final state so a failure names the sentence
// that broke.
//
// Fixture: integration_resources/invoke_param_seeds_declared_child_data/invoke_param_seeds_declared_child_data.scxml
// (canonical, shared with the Interpreter / Rust / Go / Kotlin / Python /
// C11 channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(invoke_param_seeds_declared_child_data ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "invoke_param_seeds_declared_child_data_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <gtest/gtest.h>
#include <memory>

namespace SCE::Tests {

TEST(InvokeParamSeedsDeclaredChildDataAotTest, InvokeParamCarriesTheInvokingSessionsValue) {
    using SM = SCE::Generated::invoke_param_seeds_declared_child_data::invoke_param_seeds_declared_child_data;

    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        // Aliasing constructor + no-op deleter — engine lifetime is owned by
        // the ScriptEngineProvider singleton; the shared_ptr is a non-owning
        // view. Mirrors SimpleAotTest's W3C-AOT pattern.
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }

    sm.initialize();
    // Four sequential invokes, each answering in its own macrostep.
    const bool reachedFinal = sm.runUntilCompletion(std::chrono::seconds(3));

    EXPECT_TRUE(reachedFinal) << "parent did not reach a final state within timeout — one of the "
                                 "four invokes never produced its `done.invoke.<id>`";
    EXPECT_NE(sm.getCurrentState(), SM::State::FailChildEvaluatedTheExpression)
        << "the child evaluated the author's `<param expr>` text in its own data model and found "
           "its own `token`: §scxml-6.4.3 says the VALUE of the param element, and only the "
           "invoking session can produce it. The Interpreter already does this in "
           "`InvokeExecutor.cpp` — evaluate in the parent, pass the value.";
    EXPECT_NE(sm.getCurrentState(), SM::State::FailParentOnlyExprLost)
        << "a `<param expr>` naming a variable only the parent declares arrived as nothing: the "
           "same defect as above where the child has no shadow to find.";
    EXPECT_NE(sm.getCurrentState(), SM::State::FailUnmatchedParamEnteredTheChild)
        << "a `<param>` naming no top-level `<data>` of the child became a variable there: "
           "§scxml-6.4.3 says the Processor MUST NOT add it to the invoked session's data model. "
           "The namelist arm of the same template already filters on "
           "`DatamodelValidationHelper::isVariableDeclaredInChild`.";
    EXPECT_NE(sm.getCurrentState(), SM::State::FailNamelistValueLost)
        << "the `namelist` value did not arrive: §scxml-6.4.1 says the value stored at the "
           "location is the value, so forwarding the rendered string as an expression turns a "
           "string value into an identifier lookup in the child.";
    EXPECT_NE(sm.getCurrentState(), SM::State::FailShadowSeedLost)
        << "the child saw neither the parent's value nor its own shadow, so its `<data>` default "
           "stood: nothing was seeded at all.";
    EXPECT_NE(sm.getCurrentState(), SM::State::FailDeclaredParamLost)
        << "the param that DOES name a declared `<data>` of the child did not arrive, so the "
           "filter for the unmatched one took the declared one with it.";
    EXPECT_EQ(sm.getCurrentState(), SM::State::Pass);
}

}  // namespace SCE::Tests
