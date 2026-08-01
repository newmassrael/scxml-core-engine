// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix D: pending invokes start before the external dequeue — C++ AOT path.
//
// Sibling of `InvokePrecedesExternalDequeueTest.cpp` (Interpreter channel).
// Both engines ship in production, so each is held to Appendix D's invoke
// position independently against one canonical fixture.
//
// Fixture: integration_resources/invoke_precedes_external_dequeue/invoke_precedes_external_dequeue.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(invoke_precedes_external_dequeue ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "invoke_precedes_external_dequeue_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <gtest/gtest.h>
#include <memory>

namespace SCE::Tests {

TEST(InvokePrecedesExternalDequeueAotTest, PendingInvokesStartBeforeTheExternalDequeue) {
    using SM = SCE::Generated::invoke_precedes_external_dequeue::invoke_precedes_external_dequeue;

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

    EXPECT_TRUE(reachedFinal) << "parent did not reach a final state within timeout — the watching child "
                                 "answered neither verdict, so `probe` never reached it";
    EXPECT_EQ(sm.getCurrentState(), SM::State::Pass)
        << "the watching child answered `probe` from `waiting`, so it never saw `kick`. The parent "
           "drained its external queue before starting the invoke, and the event `<onentry>` had "
           "queued for itself was consumed while no child existed. W3C Appendix D `mainEventLoop` "
           "runs `invoke(inv)` for every state entered on the last iteration before it reaches "
           "`externalQueue.dequeue()`, so an autoforward child is live for the whole external queue.";
}

}  // namespace SCE::Tests
