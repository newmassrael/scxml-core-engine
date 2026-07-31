// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4 autoforward carries `done.invoke.<id>` — C++ AOT path.
//
// Sibling of `AutoforwardDoneInvokeTest.cpp` (Interpreter channel). Both
// engines ship in production — Interpreter for embedded hosting, AOT for
// codegen consumers — so each is held to Appendix D's `mainEventLoop`
// forwarding rule independently against one canonical fixture.
//
// Fixture: integration_resources/autoforward_done_invoke/autoforward_done_invoke.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(autoforward_done_invoke ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "autoforward_done_invoke_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <gtest/gtest.h>
#include <memory>

namespace SCE::Tests {

TEST(AutoforwardDoneInvokeAotTest, DoneInvokeFromASiblingReachesTheAutoforwardChild) {
    using SM = SCE::Generated::autoforward_done_invoke::autoforward_done_invoke;

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

    EXPECT_TRUE(reachedFinal) << "parent did not reach a final state within timeout — the watcher child "
                                 "reported neither verdict, so `done.invoke.inv_short` never reached the "
                                 "parent's external queue at all";
    EXPECT_EQ(sm.getCurrentState(), SM::State::Pass)
        << "the watcher saw only `probe`: `done.invoke.inv_short` was withheld from a live "
           "`autoforward` child. W3C Appendix D `mainEventLoop` forwards every event dequeued "
           "from the external queue and excludes only the cancel event, and §6.4.2 places "
           "`done.invoke.<id>` on that queue — so no name-based platform-event filter belongs "
           "on the forwarding path.";
}

}  // namespace SCE::Tests
