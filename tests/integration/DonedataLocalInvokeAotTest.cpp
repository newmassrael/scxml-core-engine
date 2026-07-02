// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.5 + 6.3.1 donedata surfacing — C++ AOT local-invoke path.
//
// Closes the W3C IRP coverage gap by exercising the canonical
// `donedata_local_invoke` fixture
// against the AOT engine. Sibling `DonedataLocalInvokeTest.cpp` covers
// the Interpreter engine — both channels exist in production (Interpreter
// for embedded usage, AOT for codegen consumers), so both are verified
// independently.
//
// Fixture: integration_resources/donedata_local_invoke/donedata_local_invoke.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(donedata_local_invoke ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.
// The build itself is the §6.2.6 freshness invariant — there is no
// committed tree for the cpp backend.

#include "donedata_local_invoke_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <gtest/gtest.h>
#include <memory>

namespace SCE::Tests {

TEST(DonedataLocalInvokeAotTest, ParentObservesDonedataOnDoneInvoke) {
    using SM = SCE::Generated::donedata_local_invoke::donedata_local_invoke;

    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        // Aliasing constructor + no-op deleter — engine lifetime is owned
        // by ScriptEngineProvider singleton; shared_ptr is a non-owning view.
        // Mirrors SimpleAotTest's W3C-AOT pattern.
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }

    sm.initialize();
    // Two sequential invokes (inv_param then inv_content) need more than
    // one macrostep to drive the parent to Pass; `initialize()` runs only
    // the first. `runUntilCompletion` drains the remaining invoke macrosteps.
    const bool reachedFinal = sm.runUntilCompletion(std::chrono::seconds(3));

    EXPECT_TRUE(reachedFinal) << "parent did not reach a final state within timeout — invoke "
                                 "macrostep loop regressed on the AOT engine";
    EXPECT_EQ(sm.getCurrentState(), SM::State::Pass) << "parent reached a final state other than Pass — donedata "
                                                        "envelope round-trip regressed on the AOT engine";
}

}  // namespace SCE::Tests
