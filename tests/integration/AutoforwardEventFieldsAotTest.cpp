// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4 autoforward field preservation — C++ AOT local-invoke path.
//
// Sibling of `AutoforwardEventFieldsTest.cpp` (Interpreter channel). Both
// engines ship in production — Interpreter for embedded hosting, AOT for
// codegen consumers — so each is held to the §6.4 "exact copy" contract
// independently against one canonical fixture.
//
// Fixture: integration_resources/autoforward_event_fields/autoforward_event_fields.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(autoforward_event_fields ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "autoforward_event_fields_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <gtest/gtest.h>
#include <memory>

namespace SCE::Tests {

TEST(AutoforwardEventFieldsAotTest, ForwardedCopyKeepsDataOriginAndInvokeid) {
    using SM = SCE::Generated::autoforward_event_fields::autoforward_event_fields;

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

    EXPECT_TRUE(reachedFinal) << "parent did not reach a final state within timeout — the child never "
                                 "received the forwarded `childToParent`, so no done.invoke.inv_echo fired";
    EXPECT_EQ(sm.getCurrentState(), SM::State::Pass)
        << "the child reported `stripped`: the autoforwarded copy of `childToParent` lost "
           "`_event.data.value`, `_event.origin` or `_event.invokeid`. W3C §6.4 requires an "
           "exact copy — `forwardToAutoforwardChildren` must carry the source event's metadata, "
           "not just its name.";
}

}  // namespace SCE::Tests
