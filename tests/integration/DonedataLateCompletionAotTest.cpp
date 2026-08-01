// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.5 + 6.3.1: `<donedata>` survives a late completion — C++ AOT path.
//
// Sibling of `DonedataLateCompletionTest.cpp` (Interpreter channel). Both
// engines ship in production, so each is held to the donedata carry
// independently against one canonical fixture.
//
// Fixture: integration_resources/donedata_late_completion/donedata_late_completion.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(donedata_late_completion ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "donedata_late_completion_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <gtest/gtest.h>
#include <memory>

namespace SCE::Tests {

TEST(DonedataLateCompletionAotTest, DonedataRidesACompletionAfterTheInvokeStarted) {
    using SM = SCE::Generated::donedata_late_completion::donedata_late_completion;

    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        // Aliasing constructor + no-op deleter — engine lifetime is owned by
        // the ScriptEngineProvider singleton; the shared_ptr is a non-owning
        // view. Mirrors SimpleAotTest's W3C-AOT pattern.
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }

    sm.initialize();
    const bool reachedFinal = sm.runUntilCompletion(std::chrono::seconds(3));

    EXPECT_TRUE(reachedFinal) << "parent did not reach a final state within timeout — it never saw "
                                 "`done.invoke.inv_late` at all, so the child was not driven to its `<final>`";
    EXPECT_EQ(sm.getCurrentState(), SM::State::Pass)
        << "the parent's `done.invoke.inv_late` guard did not see `_event.data.result === 42`, so the "
           "child's `<donedata>` was dropped on a completion that happened after the invoke was "
           "started. W3C SCXML 6.3.1 raises `done.invoke.<id>` wherever the child reaches its final "
           "state and 5.5 puts that state's donedata on the event; neither is scoped to children that "
           "finalise during start-up.";
}

}  // namespace SCE::Tests
