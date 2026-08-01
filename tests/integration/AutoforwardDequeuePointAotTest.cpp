// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4 autoforward happens at the external dequeue — C++ AOT path.
//
// Sibling of `AutoforwardDequeuePointTest.cpp` (Interpreter channel).
// Both engines ship in production, so each is held to Appendix D's
// forwarding position independently against one canonical fixture.
//
// Fixture: integration_resources/autoforward_dequeue_point/autoforward_dequeue_point.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(autoforward_dequeue_point ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "autoforward_dequeue_point_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <gtest/gtest.h>
#include <memory>

namespace SCE::Tests {

TEST(AutoforwardDequeuePointAotTest, AnExternalEventIsForwardedAtTheDequeueNotTheEnqueue) {
    using SM = SCE::Generated::autoforward_dequeue_point::autoforward_dequeue_point;

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

    EXPECT_TRUE(reachedFinal) << "parent did not reach a final state within timeout — the probe child "
                                 "reported neither verdict, so `second` never reached it";
    EXPECT_EQ(sm.getCurrentState(), SM::State::Pass)
        << "the probe child saw `second` before `mark`, so both events were handed over while "
           "the parent was still executing the transition that queued them. W3C Appendix D "
           "`mainEventLoop` forwards one statement after `externalQueue.dequeue()`, and §6.4.2 "
           "puts it \"at the point at which it removes it from the external event queue\" — "
           "forwarding at the enqueue lets the child run ahead of the parent by a whole event.";
}

}  // namespace SCE::Tests
