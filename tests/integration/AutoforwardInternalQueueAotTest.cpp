// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4 autoforward skips internal-queue events — C++ AOT path.
//
// Sibling of `AutoforwardInternalQueueTest.cpp` (Interpreter channel).
// Both engines ship in production, so each is held to Appendix D's
// forwarding position independently against one canonical fixture.
//
// Fixture: integration_resources/autoforward_internal_queue/autoforward_internal_queue.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(autoforward_internal_queue ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "autoforward_internal_queue_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <gtest/gtest.h>
#include <memory>

namespace SCE::Tests {

TEST(AutoforwardInternalQueueAotTest, AnInternalQueueEventIsNeverAutoforwarded) {
    using SM = SCE::Generated::autoforward_internal_queue::autoforward_internal_queue;

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
                                 "reported neither verdict, so neither `error.execution` nor `probe` reached it";
    EXPECT_EQ(sm.getCurrentState(), SM::State::Pass)
        << "the watcher saw `error.execution`: an internal-queue event was autoforwarded. "
           "W3C Appendix D `mainEventLoop` forwards only what it dequeues from the external "
           "queue, and §6.2 raises `error.execution` onto the internal one — check that the "
           "event was not routed onto the external queue for some unrelated reason (keeping "
           "it from inline delivery, say), which would leak it past any name-blind forward.";
}

}  // namespace SCE::Tests
