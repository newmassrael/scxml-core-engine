// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML C.1 `_event.origin` is an address — C++ AOT local-invoke path.
//
// Sibling of `EventOriginIsALocationTest.cpp` (Interpreter channel). The
// two engines derive this field in different places — the Interpreter in
// `ActionExecutorImpl::ensureCurrentEventSet`, the AOT engine from whatever
// the generated invoke bridge hands to `EventWithMetadata` — so a fix in one
// says nothing about the other, and a fixture that only ran on one would
// leave half the shipped surface unchecked.
//
// Fixture: integration_resources/event_origin_is_a_location/event_origin_is_a_location.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(event_origin_is_a_location ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "event_origin_is_a_location_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <gtest/gtest.h>
#include <memory>

namespace SCE::Tests {

TEST(EventOriginIsALocationAotTest, OriginIsTheSendersPublishedLocationAndRoutesBack) {
    using SM = SCE::Generated::event_origin_is_a_location::event_origin_is_a_location;

    SM sm;
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm.setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                 [](::SCE::IScriptEngine *) {}));
    }

    sm.initialize();
    const bool reachedFinal = sm.runUntilCompletion(std::chrono::seconds(3));

    EXPECT_TRUE(reachedFinal) << "parent did not reach a final state within timeout: it accepted "
                                 "`_event.origin` as an address and sent `reply` to it, and nothing came "
                                 "back. §scxml-C-1 requires the published location to be a usable <send> "
                                 "target.";
    EXPECT_EQ(sm.getCurrentState(), SM::State::Pass)
        << "`_event.origin` did not carry the sender's published `_ioprocessors` location. "
           "§scxml-C-1 requires the origin to match that location; a bare session id matches "
           "nothing the sender published, and cannot be answered.";
}

}  // namespace SCE::Tests
