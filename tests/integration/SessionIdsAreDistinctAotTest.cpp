// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.10 `_sessionid` is the id of a session - C++ AOT local-invoke path.
//
// Sibling of `SessionIdsAreDistinctTest.cpp` (Interpreter channel). The two
// engines issue session ids in different places, so a fix in one says
// nothing about the other, and a fixture that ran on only one would leave
// half the shipped surface unchecked.
//
// Fixture: integration_resources/session_ids_are_distinct/session_ids_are_distinct.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(session_ids_are_distinct ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "scripting/ScriptEngineProvider.h"
#include "session_ids_are_distinct_sm.h"

#include <chrono>
#include <gtest/gtest.h>
#include <memory>

namespace SCE::Tests {

TEST(SessionIdsAreDistinctAotTest, TwoLiveSessionsAreIssuedDifferentIds) {
    using SM = SCE::Generated::session_ids_are_distinct::session_ids_are_distinct;

    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }

    sm.initialize();
    const bool reachedFinal = sm.runUntilCompletion(std::chrono::seconds(3));

    EXPECT_TRUE(reachedFinal) << "parent did not reach a final state within timeout: only one child "
                                 "reported its `_sessionid`, so the two ids were never compared.";
    EXPECT_EQ(sm.getCurrentState(), SM::State::Pass)
        << "two live sessions reported the same `_sessionid`. The clause binds it to the id "
           "of the current session, and the published `_ioprocessors` location is derived "
           "from it, so one id for two sessions is one address for two sessions.";
}

}  // namespace SCE::Tests
